use crate::config::Config;
use crate::renderer::GpuRenderer;
use crate::ui::tabs::TabManager;
use crate::window::input::InputHandler;
use anyhow::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct Application {
    config: Config,
    tab_manager: TabManager,
    input_handler: InputHandler,
    renderer: Option<GpuRenderer>,
    pub cell_width: f32,
    pub cell_height: f32,
    should_close: Arc<AtomicBool>,
    /// Font size base (del config) para el reset con Ctrl+Shift+0
    base_font_size: u32,
    /// Font size actual (cambia con Ctrl+Shift+=/-)
    current_font_size: u32,
    /// Offset de scroll en historial (0 = presente)
    scroll_offset: usize,
}

impl Application {
    pub fn new(config: Config) -> Result<Self> {
        let base_font_size = config.font_size;
        Ok(Self {
            base_font_size,
            current_font_size: base_font_size,
            config,
            tab_manager: TabManager::new(),
            input_handler: InputHandler::new(),
            renderer: None,
            cell_width: 0.0,
            cell_height: 0.0,
            should_close: Arc::new(AtomicBool::new(false)),
            scroll_offset: 0,
        })
    }

    pub fn init(&mut self, width: u32, height: u32) -> Result<()> {
        let padding = self.config.cell_padding;
        let font_size = self.current_font_size;
        let beam_thickness = self.config.cursor_beam_thickness;

        // Convertir paleta u8 → f32 para el renderer
        let mut palette = [[0.0f32; 3]; 16];
        for (i, rgb) in self.config.color_palette.iter().enumerate() {
            palette[i] = [
                rgb[0] as f32 / 255.0,
                rgb[1] as f32 / 255.0,
                rgb[2] as f32 / 255.0,
            ];
        }
        let fg = self.config.foreground;
        let bg = self.config.background;
        let fg_default = [
            fg[0] as f32 / 255.0,
            fg[1] as f32 / 255.0,
            fg[2] as f32 / 255.0,
            1.0,
        ];
        let bg_default = [
            bg[0] as f32 / 255.0,
            bg[1] as f32 / 255.0,
            bg[2] as f32 / 255.0,
            self.config.background_opacity,
        ];

        let renderer = GpuRenderer::new(
            font_size,
            padding,
            1,
            1,
            width,
            height,
            beam_thickness,
            None,
            palette,
            fg_default,
            bg_default,
        )?;

        self.cell_width = renderer.cell_width;
        self.cell_height = renderer.cell_height;

        let cols = ((width as f32 - padding * 2.0) / self.cell_width).max(1.0) as usize;
        let rows = ((height as f32 - padding * 2.0) / self.cell_height).max(1.0) as usize;

        let mut renderer = renderer;
        renderer.resize(cols, rows, width, height);
        self.renderer = Some(renderer);

        log::info!(
            "App init: ventana {}x{}, celda {:.1}x{:.1}, grid {}x{}",
            width,
            height,
            self.cell_width,
            self.cell_height,
            cols,
            rows
        );

        let shell = self.config.shell.clone();
        self.tab_manager.create_tab(shell.clone(), rows, cols)?;
        log::info!("Tab creado: {} ({}x{})", shell, cols, rows);

        Ok(())
    }

    pub fn handle_input(&mut self, key_event: crate::window::input::KeyEvent) {
        if !key_event.characters.is_empty() {
            if let Some(ref mut tab) = self.tab_manager.active_tab_mut() {
                tab.write_input(key_event.characters.as_bytes()).ok();
            }
        }
    }

    /// Escribe bytes directamente al PTY (teclas especiales, paste, etc.)
    pub fn write_bytes(&mut self, data: &[u8]) {
        if let Some(ref mut tab) = self.tab_manager.active_tab_mut() {
            tab.write_input(data).ok();
        }
    }

    pub fn padding(&self) -> f32 {
        self.config.cell_padding
    }

    // ── Font size en caliente (Kitty: Ctrl+Shift+=/−/0) ─────────────────────

    pub fn increase_font_size(&mut self, delta: u32) {
        self.current_font_size = (self.current_font_size + delta).min(72);
        log::info!(
            "Font size: {}pt (usa Ctrl+Shift+0 para reset)",
            self.current_font_size
        );
    }

    pub fn decrease_font_size(&mut self, delta: u32) {
        self.current_font_size = self.current_font_size.saturating_sub(delta).max(6);
        log::info!("Font size: {}pt", self.current_font_size);
    }

    pub fn reset_font_size(&mut self) {
        self.current_font_size = self.base_font_size;
        log::info!("Font size reset: {}pt", self.current_font_size);
    }

    // ── Scroll del historial (Kitty: Ctrl+Shift+K/J/Home/End) ────────────────

    pub fn scroll_lines(&mut self, delta: i32) {
        if delta < 0 {
            self.scroll_offset = self.scroll_offset.saturating_add((-delta) as usize);
        } else {
            self.scroll_offset = self.scroll_offset.saturating_sub(delta as usize);
        }
        if let Some(ref tab) = self.tab_manager.active_tab() {
            let max = tab.parser.screen().scrollback_len();
            self.scroll_offset = self.scroll_offset.min(max);
        }
    }

    pub fn scroll_to_top(&mut self) {
        if let Some(ref tab) = self.tab_manager.active_tab() {
            self.scroll_offset = tab.parser.screen().scrollback_len();
        }
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
    }

    /// Extrae texto del rango de celdas (row0,col0)..=(row1,col1)
    pub fn get_selected_text(&self, r0: usize, c0: usize, r1: usize, c1: usize) -> String {
        if let Some(ref tab) = self.tab_manager.active_tab() {
            let screen = tab.parser.screen();
            let mut out = String::new();
            for row in r0..=r1.min(screen.rows().saturating_sub(1)) {
                let c_start = if row == r0 { c0 } else { 0 };
                let c_end = if row == r1 {
                    c1
                } else {
                    screen.cols().saturating_sub(1)
                };
                let mut line = String::new();
                for col in c_start..=c_end.min(screen.cols().saturating_sub(1)) {
                    if let Some(cell) = screen.cell(row, col) {
                        line.push(cell.c);
                    }
                }
                let trimmed = line.trim_end();
                if !out.is_empty() {
                    out.push('\n');
                }
                out.push_str(trimmed);
            }
            out
        } else {
            String::new()
        }
    }

    pub fn update(&mut self) -> bool {
        if let Some(ref mut tab) = self.tab_manager.active_tab_mut() {
            tab.read_output().unwrap_or(false)
        } else {
            false
        }
    }

    pub fn render(&mut self) {
        self.render_with_selection(None);
    }

    pub fn render_with_selection(&mut self, sel: Option<(usize, usize, usize, usize)>) {
        if let Some(ref mut renderer) = self.renderer {
            if let Some(ref tab) = self.tab_manager.active_tab() {
                renderer.render(tab.parser.screen(), tab.parser.cursor(), sel);
                // Pass 4: Kitty Graphics Protocol images
                renderer.render_images(&tab.parser.graphics.placements);
            }
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        let padding = self.config.cell_padding;
        let cols = ((width as f32 - padding * 2.0) / self.cell_width).max(1.0) as usize;
        let rows = ((height as f32 - padding * 2.0) / self.cell_height).max(1.0) as usize;

        self.tab_manager.resize_all(rows, cols);

        if let Some(ref mut renderer) = self.renderer {
            renderer.resize(cols, rows, width, height);
        }
    }

    pub fn should_close(&self) -> bool {
        self.should_close.load(Ordering::SeqCst)
    }

    pub fn set_close(&self) {
        self.should_close.store(true, Ordering::SeqCst);
    }
}
