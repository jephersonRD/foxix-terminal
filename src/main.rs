#![allow(unused)]

use std::error::Error;
use std::ffi::CString;
use std::num::NonZeroU32;

use foxix::config::Config as AppConfig;
use foxix::ui::Application;
use foxix::window::input::InputHandler;
use glutin::config::{Api, Config, ConfigTemplateBuilder, GetGlConfig};
use glutin::context::{
    ContextApi, ContextAttributesBuilder, NotCurrentContext, PossiblyCurrentContext, Version,
};
use glutin::display::GetGlDisplay;
use glutin::prelude::*;
use glutin::surface::{Surface, SwapInterval, WindowSurface};
use glutin_winit::{DisplayBuilder, GlWindow};
use raw_window_handle::HasWindowHandle;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{ElementState, KeyEvent as WinitKeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{Window, WindowAttributes, WindowId};

/// Estado de selección de texto con el ratón
#[derive(Clone, Debug, Default)]
struct Selection {
    active: bool,
    start_px: (f64, f64),
    end_px: (f64, f64),
    /// Texto seleccionado (ya calculado)
    text: String,
}

struct App {
    config: AppConfig,
    template: ConfigTemplateBuilder,
    display_builder: DisplayBuilder,
    input_handler: InputHandler,
    app: Option<Application>,
    gl_context: Option<PossiblyCurrentContext>,
    state: Option<AppState>,
    frame_count: u64,
    start: std::time::Instant,
    exit_state: Result<(), Box<dyn Error>>,
    mouse_pos: PhysicalPosition<f64>,
    selection: Selection,
    modifiers: u32,
    /// Resize pendiente — se aplica al PTY/shell tras RESIZE_DEBOUNCE_MS
    pending_resize: Option<(u32, u32)>,
    resize_deadline: Option<std::time::Instant>,
}

struct AppState {
    gl_surface: Surface<WindowSurface>,
    window: Window,
}

impl App {
    fn new(config: AppConfig) -> Self {
        let template = ConfigTemplateBuilder::new().with_alpha_size(8);
        let display_builder =
            DisplayBuilder::new().with_window_attributes(Some(window_attributes(&config)));

        Self {
            config: config.clone(),
            template,
            display_builder,
            input_handler: InputHandler::new(),
            app: None,
            gl_context: None,
            state: None,
            frame_count: 0,
            start: std::time::Instant::now(),
            exit_state: Ok(()),
            mouse_pos: PhysicalPosition::new(0.0, 0.0),
            selection: Selection::default(),
            modifiers: 0,
            pending_resize: None,
            resize_deadline: None,
        }
    }

    /// Convierte pixels a (col, row) de la grid de celdas
    fn px_to_cell(&self, x: f64, y: f64) -> Option<(usize, usize)> {
        if let Some(ref app) = self.app {
            let (cw, ch, pad) = (
                app.cell_width as f64,
                app.cell_height as f64,
                app.padding() as f64,
            );
            let col = ((x - pad) / cw) as usize;
            let row = ((y - pad) / ch) as usize;
            Some((col, row))
        } else {
            None
        }
    }

    /// Extrae el texto seleccionado del buffer
    fn selected_text(&self) -> String {
        if let Some(ref app) = self.app {
            let (cw, ch, pad) = (
                self.app.as_ref().map(|a| a.cell_width).unwrap_or(8.0) as f64,
                self.app.as_ref().map(|a| a.cell_height).unwrap_or(16.0) as f64,
                self.app.as_ref().map(|a| a.padding()).unwrap_or(0.0) as f64,
            );
            let (sx, sy) = self.selection.start_px;
            let (ex, ey) = self.selection.end_px;

            let (r0, c0) = (
                ((sy.min(ey) - pad) / ch) as usize,
                ((sx.min(ex) - pad) / cw) as usize,
            );
            let (r1, c1) = (
                ((sy.max(ey) - pad) / ch) as usize,
                ((sx.max(ex) - pad) / cw) as usize,
            );
            app.get_selected_text(r0, c0, r1, c1)
        } else {
            String::new()
        }
    }

    /// Copia `text` al clipboard de Wayland
    fn copy_to_clipboard(&self, text: &str) {
        if text.is_empty() {
            return;
        }
        let _ = std::process::Command::new("wl-copy").arg(text).spawn();
        log::info!("Copied {} chars to clipboard", text.len());
    }

    /// Pega desde el clipboard de Wayland al PTY
    fn paste_from_clipboard(&mut self) {
        let out = std::process::Command::new("wl-paste")
            .arg("--no-newline")
            .output();
        if let Ok(out) = out {
            if !out.stdout.is_empty() {
                if let Some(ref mut app) = self.app {
                    app.write_bytes(&out.stdout);
                }
                log::info!("Pasted {} bytes from clipboard", out.stdout.len());
            }
        }
    }
}

fn window_attributes(config: &AppConfig) -> WindowAttributes {
    // Transparencia si opacity < 1.0
    let transparent = config.background_opacity < 0.999;
    Window::default_attributes()
        .with_title("Foxix")
        .with_inner_size(LogicalSize::new(800, 600))
        .with_transparent(transparent)
}

fn gl_config_picker(configs: Box<dyn Iterator<Item = Config> + '_>) -> Config {
    configs
        .reduce(|accum, config| {
            // Priorizar configs con soporte a transparencia (alpha buffer)
            let has_alpha = config.supports_transparency().unwrap_or(false);
            let accum_has = accum.supports_transparency().unwrap_or(false);
            if has_alpha && !accum_has {
                config
            } else {
                accum
            }
        })
        .unwrap()
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        log::info!("RESUMED called - creating window");

        let (mut window, gl_config) = match self.display_builder.clone().build(
            event_loop,
            self.template.clone(),
            gl_config_picker,
        ) {
            Ok((window, gl_config)) => {
                let win = window.unwrap();
                // Solicitar foco explícito para Wayland
                win.request_redraw();
                (win, gl_config)
            }
            Err(err) => {
                self.exit_state = Err(err);
                event_loop.exit();
                return;
            }
        };

        log::info!("Picked a config with {} samples", gl_config.num_samples());

        let raw_window_handle = window.window_handle().ok().map(|wh| wh.as_raw());

        let context_attributes = ContextAttributesBuilder::new().build(raw_window_handle);
        let fallback_context_attributes = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::Gles(None))
            .build(raw_window_handle);
        let legacy_context_attributes = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::OpenGl(Some(Version::new(2, 1))))
            .build(raw_window_handle);

        let gl_display = gl_config.display();

        let gl_context = unsafe {
            gl_display
                .create_context(&gl_config, &context_attributes)
                .unwrap_or_else(|_| {
                    gl_display
                        .create_context(&gl_config, &fallback_context_attributes)
                        .unwrap_or_else(|_| {
                            gl_display
                                .create_context(&gl_config, &legacy_context_attributes)
                                .expect("Failed to create GL context")
                        })
                })
        };

        self.gl_context = Some(gl_context.treat_as_possibly_current());

        let attrs = window
            .build_surface_attributes(Default::default())
            .expect("Failed to build surface attributes");
        let gl_surface = unsafe {
            gl_config
                .display()
                .create_window_surface(&gl_config, &attrs)
                .unwrap()
        };

        let gl_context = self.gl_context.as_ref().unwrap();
        gl_context.make_current(&gl_surface).unwrap();
        log::info!("OpenGL context made current");

        unsafe {
            let cstr = CString::new("".as_bytes()).unwrap();
            gl::load_with(|symbol| {
                let symbol_cstr = CString::new(symbol).unwrap();
                gl_config.display().get_proc_address(symbol_cstr.as_c_str()) as *const _
            });

            let version = gl::GetString(gl::VERSION);
            if !version.is_null() {
                let version_str = std::ffi::CStr::from_ptr(version as *const _)
                    .to_string_lossy()
                    .into_owned();
                log::info!("OpenGL version: {}", version_str);
            }
        }

        if let Err(res) = gl_surface
            .set_swap_interval(gl_context, SwapInterval::Wait(NonZeroU32::new(1).unwrap()))
        {
            log::warn!("Error setting vsync: {:?}", res);
        }

        let size = window.inner_size();
        log::info!(
            "Creating application with window size {}x{}",
            size.width,
            size.height
        );
        let mut application = Application::new(self.config.clone());
        if let Err(e) = application {
            log::error!("Application::new failed: {:?}", e);
            event_loop.exit();
            return;
        }
        let mut application = application.unwrap();
        if let Err(e) = application.init(size.width, size.height) {
            log::error!("Application::init failed: {:?}", e);
            event_loop.exit();
            return;
        }
        self.app = Some(application);

        log::info!(
            "Application initialized: cell {:.1}x{:.1}",
            self.app.as_ref().unwrap().cell_width,
            self.app.as_ref().unwrap().cell_height
        );

        log::info!("Setting state and exiting resumed");
        self.state = Some(AppState { gl_surface, window });
        log::info!("resumed complete, doing initial render");

        // Initial render
        if let (Some(ref mut app), Some(ref gl_context), Some(ref state)) =
            (&mut self.app, &mut self.gl_context, &self.state)
        {
            log::info!("Calling app.update()");
            app.update();
            log::info!("app.update() done, calling app.render()");
            app.render();
            log::info!("Render done, trying swap...");
            if let Err(e) = state.gl_surface.swap_buffers(gl_context) {
                log::error!("Initial swap error: {:?}", e);
            }
            log::info!("Initial swap done");
        }

        // Request subsequent redraws
        if let Some(ref state) = self.state {
            state.window.request_redraw();
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        log::info!("Suspended");
        if let Some(state) = self.state.take() {
            drop(state);
        }
        self.gl_context = Some(
            self.gl_context
                .take()
                .unwrap()
                .make_not_current()
                .unwrap()
                .treat_as_possibly_current(),
        );
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                log::info!("Close requested");
                event_loop.exit();
            }
            WindowEvent::Resized(size) if size.width != 0 && size.height != 0 => {
                log::info!("Window resized: {:?}", size);
                // 1. Resize del surface GL — inmediato para evitar artefactos visuales
                if let (Some(ref gl_ctx), Some(ref state)) =
                    (self.gl_context.as_ref(), self.state.as_ref())
                {
                    state.gl_surface.resize(
                        gl_ctx,
                        NonZeroU32::new(size.width).unwrap(),
                        NonZeroU32::new(size.height).unwrap(),
                    );
                    state.window.request_redraw();
                }
                // 2. Render inmediato con nuevo viewport (sin cambiar PTY todavía)
                if let (Some(ref mut app), Some(ref gl_ctx), Some(ref state)) = (
                    self.app.as_mut(),
                    self.gl_context.as_ref(),
                    self.state.as_ref(),
                ) {
                    unsafe {
                        gl::Clear(gl::COLOR_BUFFER_BIT);
                    }
                    app.render();
                    let _ = state.gl_surface.swap_buffers(gl_ctx);
                }
                // 3. Guardar resize pendiente — se aplica al PTY después de DEBOUNCE
                // Esto evita que zsh reciba múltiples SIGWINCH al hacer Super+F
                const DEBOUNCE_MS: u64 = 80;
                self.pending_resize = Some((size.width, size.height));
                self.resize_deadline =
                    Some(std::time::Instant::now() + std::time::Duration::from_millis(DEBOUNCE_MS));
            }
            WindowEvent::RedrawRequested => {
                // Rendering now happens in about_to_wait
            }
            // ── Mouse: seguimiento de posición ─────────────────────────────
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_pos = position;
                // Si hay selección activa, actualizar end y provocar redraw
                if self.selection.active {
                    self.selection.end_px = (position.x, position.y);
                    if let Some(ref state) = self.state {
                        state.window.request_redraw();
                    }
                }
            }
            // Mouse pressed — inicio de selección
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                self.selection = Selection {
                    active: true,
                    start_px: (self.mouse_pos.x, self.mouse_pos.y),
                    end_px: (self.mouse_pos.x, self.mouse_pos.y),
                    text: String::new(),
                };
                // Forzar redibujado al iniciar selección
                if let Some(ref state) = self.state {
                    state.window.request_redraw();
                }
            }
            // Mouse released — fin de selección
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {
                if self.selection.active {
                    self.selection.active = false;
                    let text = self.selected_text();
                    if !text.is_empty() {
                        // Auto-copy al soltar (como Kitty)
                        self.copy_to_clipboard(&text);
                        self.selection.text = text;
                    }
                }
            }
            // ── Teclado ────────────────────────────────────────────────────
            WindowEvent::ModifiersChanged(modifiers) => {
                let s = modifiers.state();
                self.modifiers = 0;
                if s.super_key() {
                    self.modifiers |= 0x08;
                }
                if s.control_key() {
                    self.modifiers |= 0x01;
                }
                if s.alt_key() {
                    self.modifiers |= 0x02;
                }
                if s.shift_key() {
                    self.modifiers |= 0x04;
                }
                self.input_handler.set_modifiers(self.modifiers);
            }
            WindowEvent::KeyboardInput {
                event:
                    WinitKeyEvent {
                        state: ElementState::Pressed,
                        logical_key,
                        text,
                        ..
                    },
                ..
            } => {
                let is_ctrl = (self.modifiers & 0x01) != 0;
                let is_shift = (self.modifiers & 0x04) != 0;
                let is_super = (self.modifiers & 0x08) != 0;

                match logical_key {
                    // ── Shortcuts ────────────────────────────────────────────
                    Key::Character(ref c)
                        if (is_ctrl && is_shift && c.eq_ignore_ascii_case("q"))
                            || (is_super && c.eq_ignore_ascii_case("q")) =>
                    {
                        event_loop.exit();
                        return;
                    }
                    // Ctrl+Shift+C → copiar selección al clipboard
                    Key::Character(ref c) if is_ctrl && is_shift && c.eq_ignore_ascii_case("c") => {
                        let text = self.selection.text.clone();
                        if !text.is_empty() {
                            self.copy_to_clipboard(&text);
                        }
                        return;
                    }
                    // Ctrl+Shift+V → pegar desde clipboard
                    Key::Character(ref c) if is_ctrl && is_shift && c.eq_ignore_ascii_case("v") => {
                        self.paste_from_clipboard();
                        return;
                    }
                    // Ctrl+V también pega (como Kitty)
                    Key::Character(ref c)
                        if is_ctrl && c.eq_ignore_ascii_case("v") && !is_shift =>
                    {
                        self.paste_from_clipboard();
                        return;
                    }
                    // ── Kitty: Cambiar tamaño de fuente en caliente ───────────
                    // Ctrl+Shift+= / Ctrl+Shift++ → aumentar font_size +1pt
                    Key::Character(ref c)
                        if is_ctrl && is_shift && (c.as_str() == "=" || c.as_str() == "+") =>
                    {
                        if let Some(ref mut app) = self.app {
                            app.increase_font_size(1);
                        }
                        return;
                    }
                    // Ctrl+Shift+- → disminuir font_size -1pt
                    Key::Character(ref c) if is_ctrl && is_shift && c.as_str() == "-" => {
                        if let Some(ref mut app) = self.app {
                            app.decrease_font_size(1);
                        }
                        return;
                    }
                    // Ctrl+Shift+0 → resetear font_size al valor de foxix.conf
                    Key::Character(ref c) if is_ctrl && is_shift && c.as_str() == "0" => {
                        if let Some(ref mut app) = self.app {
                            app.reset_font_size();
                        }
                        return;
                    }
                    // ── Kitty: Scroll con teclado (Ctrl+Shift+Up/Down/K/J) ────
                    Key::Named(NamedKey::ArrowUp) if is_ctrl && is_shift => {
                        if let Some(ref mut app) = self.app {
                            app.scroll_lines(-1);
                        }
                        return;
                    }
                    Key::Named(NamedKey::ArrowDown) if is_ctrl && is_shift => {
                        if let Some(ref mut app) = self.app {
                            app.scroll_lines(1);
                        }
                        return;
                    }
                    Key::Character(ref c) if is_ctrl && is_shift && c.eq_ignore_ascii_case("k") => {
                        if let Some(ref mut app) = self.app {
                            app.scroll_lines(-1);
                        }
                        return;
                    }
                    Key::Character(ref c) if is_ctrl && is_shift && c.eq_ignore_ascii_case("j") => {
                        if let Some(ref mut app) = self.app {
                            app.scroll_lines(1);
                        }
                        return;
                    }
                    // Ctrl+Shift+Home → scroll al inicio del historial
                    Key::Named(NamedKey::Home) if is_ctrl && is_shift => {
                        if let Some(ref mut app) = self.app {
                            app.scroll_to_top();
                        }
                        return;
                    }
                    // Ctrl+Shift+End → scroll al final
                    Key::Named(NamedKey::End) if is_ctrl && is_shift => {
                        if let Some(ref mut app) = self.app {
                            app.scroll_to_bottom();
                        }
                        return;
                    }
                    // ── Teclas de control → enviar al shell ──────────────────
                    Key::Named(NamedKey::Escape) => {
                        // ESC va al shell, NO cierra la ventana
                        if let Some(ref mut app) = self.app {
                            app.write_bytes(b"\x1b");
                        }
                    }
                    Key::Character(c) => {
                        // Ctrl+letra → enviar código de control ASCII
                        if is_ctrl && !is_shift {
                            let ch = c.chars().next().unwrap_or('\0');
                            let code = (ch.to_ascii_uppercase() as u8).wrapping_sub(b'@');
                            if code < 32 {
                                if let Some(ref mut app) = self.app {
                                    app.write_bytes(&[code]);
                                }
                                return;
                            }
                        }
                        for ch in c.chars() {
                            let ke = self.input_handler.handle_char(ch);
                            if let Some(ref mut app) = self.app {
                                app.handle_input(ke);
                            }
                        }
                    }
                    Key::Named(NamedKey::Enter) => {
                        if let Some(ref mut app) = self.app {
                            app.write_bytes(b"\r");
                        }
                    }
                    Key::Named(NamedKey::Tab) => {
                        if let Some(ref mut app) = self.app {
                            app.write_bytes(b"\t");
                        }
                    }
                    Key::Named(NamedKey::Backspace) => {
                        if let Some(ref mut app) = self.app {
                            app.write_bytes(b"\x7f");
                        }
                    }
                    Key::Named(NamedKey::Delete) => {
                        if let Some(ref mut app) = self.app {
                            app.write_bytes(b"\x1b[3~");
                        }
                    }
                    Key::Named(NamedKey::Home) => {
                        if let Some(ref mut app) = self.app {
                            app.write_bytes(b"\x1b[H");
                        }
                    }
                    Key::Named(NamedKey::End) => {
                        if let Some(ref mut app) = self.app {
                            app.write_bytes(b"\x1b[F");
                        }
                    }
                    Key::Named(NamedKey::PageUp) => {
                        if let Some(ref mut app) = self.app {
                            app.write_bytes(b"\x1b[5~");
                        }
                    }
                    Key::Named(NamedKey::PageDown) => {
                        if let Some(ref mut app) = self.app {
                            app.write_bytes(b"\x1b[6~");
                        }
                    }
                    Key::Named(NamedKey::ArrowUp) => {
                        if let Some(ref mut app) = self.app {
                            app.write_bytes(b"\x1b[A");
                        }
                    }
                    Key::Named(NamedKey::ArrowDown) => {
                        if let Some(ref mut app) = self.app {
                            app.write_bytes(b"\x1b[B");
                        }
                    }
                    Key::Named(NamedKey::ArrowRight) => {
                        if let Some(ref mut app) = self.app {
                            app.write_bytes(b"\x1b[C");
                        }
                    }
                    Key::Named(NamedKey::ArrowLeft) => {
                        if let Some(ref mut app) = self.app {
                            app.write_bytes(b"\x1b[D");
                        }
                    }
                    // ── Teclas de Función (F1-F12) ──
                    Key::Named(NamedKey::F1) => {
                        if let Some(ref mut app) = self.app {
                            app.write_bytes(b"\x1bOP");
                        }
                    }
                    Key::Named(NamedKey::F2) => {
                        if let Some(ref mut app) = self.app {
                            app.write_bytes(b"\x1bOQ");
                        }
                    }
                    Key::Named(NamedKey::F3) => {
                        if let Some(ref mut app) = self.app {
                            app.write_bytes(b"\x1bOR");
                        }
                    }
                    Key::Named(NamedKey::F4) => {
                        if let Some(ref mut app) = self.app {
                            app.write_bytes(b"\x1bOS");
                        }
                    }
                    Key::Named(NamedKey::F5) => {
                        if let Some(ref mut app) = self.app {
                            app.write_bytes(b"\x1b[15~");
                        }
                    }
                    Key::Named(NamedKey::F6) => {
                        if let Some(ref mut app) = self.app {
                            app.write_bytes(b"\x1b[17~");
                        }
                    }
                    Key::Named(NamedKey::F7) => {
                        if let Some(ref mut app) = self.app {
                            app.write_bytes(b"\x1b[18~");
                        }
                    }
                    Key::Named(NamedKey::F8) => {
                        if let Some(ref mut app) = self.app {
                            app.write_bytes(b"\x1b[19~");
                        }
                    }
                    Key::Named(NamedKey::F9) => {
                        if let Some(ref mut app) = self.app {
                            app.write_bytes(b"\x1b[20~");
                        }
                    }
                    Key::Named(NamedKey::F10) => {
                        if let Some(ref mut app) = self.app {
                            app.write_bytes(b"\x1b[21~");
                        }
                    }
                    Key::Named(NamedKey::F11) => {
                        if let Some(ref mut app) = self.app {
                            app.write_bytes(b"\x1b[23~");
                        }
                    }
                    Key::Named(NamedKey::F12) => {
                        if let Some(ref mut app) = self.app {
                            app.write_bytes(b"\x1b[24~");
                        }
                    }
                    _ => {
                        if let Some(txt) = text {
                            for ch in txt.chars() {
                                let ke = self.input_handler.handle_char(ch);
                                if let Some(ref mut app) = self.app {
                                    app.handle_input(ke);
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.frame_count += 1;

        // ── Aplicar resize debounced al PTY/shell ────────────────────────────
        // Solo enviamos SIGWINCH cuando la ventana lleva ≥80ms sin cambiar.
        // Evita que zsh cree una nueva línea por cada frame del resize animation.
        if let Some(deadline) = self.resize_deadline {
            if std::time::Instant::now() >= deadline {
                if let (Some((w, h)), Some(ref mut app)) =
                    (self.pending_resize.take(), self.app.as_mut())
                {
                    app.resize(w, h);
                }
                self.resize_deadline = None;
            }
        }

        // Calcular rect de selección en coords de celda para pasarlo al renderer
        let sel_rect: Option<(usize, usize, usize, usize)> =
            if self.selection.active || !self.selection.text.is_empty() {
                if let Some(ref app) = self.app {
                    let pad = app.padding() as f64;
                    let cw = app.cell_width as f64;
                    let ch = app.cell_height as f64;
                    let (sx, sy) = self.selection.start_px;
                    let (ex, ey) = self.selection.end_px;
                    let r0 = ((sy.min(ey) - pad) / ch).max(0.0) as usize;
                    let c0 = ((sx.min(ex) - pad) / cw).max(0.0) as usize;
                    let r1 = ((sy.max(ey) - pad) / ch).max(0.0) as usize;
                    let c1 = ((sx.max(ex) - pad) / cw).max(0.0) as usize;
                    Some((r0, c0, r1, c1))
                } else {
                    None
                }
            } else {
                None
            };

        if let (Some(ref mut app), Some(ref gl_context), Some(ref state)) =
            (&mut self.app, &mut self.gl_context, &self.state)
        {
            let had_data = app.update();
            let elapsed = self.start.elapsed().as_millis();
            // Renderizar si: hay datos PTY, hay selección activa, o tick del cursor (60fps)
            let sel_active = self.selection.active;
            let cursor_tick = elapsed % 16 < 2;

            if had_data || sel_active || cursor_tick || self.resize_deadline.is_some() {
                app.render_with_selection(sel_rect);
                if let Err(e) = state.gl_surface.swap_buffers(gl_context) {
                    log::error!("Swap buffers: {:?}", e);
                }
            }
        }

        // WaitUntil al próximo frame, o antes si hay resize pendiente
        let next = if self.resize_deadline.is_some() {
            std::time::Instant::now() + std::time::Duration::from_millis(8)
        } else {
            std::time::Instant::now() + std::time::Duration::from_millis(16)
        };
        event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(next));
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        log::info!("Exiting");
        self.state = None;
    }
}

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Starting Foxix...");

    let app_config = AppConfig::load()?;
    let mut app = App::new(app_config);

    let event_loop: EventLoop<()> = EventLoop::new()?;

    log::info!("Event loop created");

    // Empezar con Poll para el primer frame, luego about_to_wait cambiará a WaitUntil
    event_loop.set_control_flow(ControlFlow::Poll);

    event_loop.run_app(&mut app)?;

    Ok(())
}
