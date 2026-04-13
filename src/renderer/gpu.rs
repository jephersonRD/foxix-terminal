use crate::renderer::GlyphCache;
use crate::terminal::buffer::{CellAttributes, ScreenBuffer};
use crate::terminal::cursor::Cursor;
use anyhow::Result;
use gl::types::*;

// ─────────────────────────────────────────────────────────────────────────────
// Vertex shader — pixel coords (0,0 = top-left) → NDC
// ─────────────────────────────────────────────────────────────────────────────
const VERTEX_SHADER: &str = r#"#version 330 core
in vec2 aPos;
in vec2 aUV;
in vec4 aColor;

out vec2 vUV;
out vec4 vColor;

uniform vec2 uScreenSize;

void main() {
    float x = (aPos.x / uScreenSize.x) * 2.0 - 1.0;
    float y = 1.0 - (aPos.y / uScreenSize.y) * 2.0;
    gl_Position = vec4(x, y, 0.0, 1.0);
    vUV    = aUV;
    vColor = aColor;
}
"#;

// ─────────────────────────────────────────────────────────────────────────────
// Fragment shader — Quality text rendering (gamma-correct, Kitty-inspired)
// ─────────────────────────────────────────────────────────────────────────────
const FRAGMENT_SHADER: &str = r#"#version 330 core
in vec2 vUV;
in vec4 vColor;

out vec4 FragColor;

uniform sampler2D uAtlas;
uniform bool      uUseAtlas;
uniform float     uTextContrast;

void main() {
    if (uUseAtlas) {
        float coverage = texture(uAtlas, vUV).r;
        float contrast = clamp(uTextContrast, 0.5, 3.0);
        float alpha = pow(coverage, 1.0 / contrast);
        FragColor = vec4(vColor.rgb, vColor.a * clamp(alpha, 0.0, 1.0));
    } else {
        FragColor = vColor;
    }
}
"#;

// ─────────────────────────────────────────────────────────────────────────────
// Image shader — RGBA textures (Kitty Graphics Protocol)
// ─────────────────────────────────────────────────────────────────────────────
const IMG_VERTEX_SHADER: &str = r#"#version 330 core
layout (location = 0) in vec2 aPos;
layout (location = 1) in vec2 aUV;

out vec2 vUV;

uniform vec2 uScreen; // window width, height in pixels

void main() {
    // Convert pixel coords to NDC [-1, 1]
    vec2 ndc = (aPos / uScreen) * 2.0 - 1.0;
    ndc.y = -ndc.y; // flip Y (screen Y down, NDC Y up)
    gl_Position = vec4(ndc, 0.0, 1.0);
    vUV = aUV;
}
"#;

const IMG_FRAGMENT_SHADER: &str = r#"#version 330 core
in vec2 vUV;
out vec4 FragColor;
uniform sampler2D uTex;
void main() {
    FragColor = texture(uTex, vUV);
}
"#;

// Stride: 2 (pos) + 2 (uv) + 4 (color) = 8 floats
const STRIDE: usize = 8;

/// Colores estilo Kitty/oh-my-zsh (Dracula-ish)
const BG_DEFAULT: [f32; 4] = [0.118, 0.118, 0.180, 1.0]; // #1e1e2e Dracula bg
const FG_DEFAULT: [f32; 4] = [0.854, 0.854, 0.886, 1.0]; // #dadaf8 texto principal
const BOLD_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0]; // blanco para bold
const CURSOR_COLOR: [f32; 4] = [0.855, 0.510, 0.960, 1.0]; // #da81f5 rosa kitty
/// Color de fondo de la selección (azul kitty)
const SEL_COLOR: [f32; 4] = [0.310, 0.475, 0.820, 0.65]; // #507dd1 semi-transparent

/// Devuelve true si (row,col) está dentro del rectángulo de selección
#[inline]
fn is_in_selection(row: usize, col: usize, r0: usize, c0: usize, r1: usize, c1: usize) -> bool {
    if r0 == r1 {
        // Selección en una sola fila
        row == r0 && col >= c0 && col <= c1
    } else if row == r0 {
        col >= c0
    } else if row == r1 {
        col <= c1
    } else {
        row > r0 && row < r1
    }
}

const ANSI_COLORS: [[f32; 3]; 16] = [
    [0.145, 0.145, 0.180], // 0  black — Dracula
    [0.855, 0.349, 0.396], // 1  red — Dracula
    [0.396, 0.882, 0.510], // 2  green — Dracula
    [0.937, 0.855, 0.302], // 3  yellow — Dracula
    [0.376, 0.678, 0.945], // 4  blue — Dracula
    [0.855, 0.510, 0.961], // 5  magenta — Dracula
    [0.302, 0.882, 0.953], // 6  cyan — Dracula
    [0.855, 0.855, 0.886], // 7  white — Dracula
    [0.302, 0.302, 0.373], // 8  bright black — Dracula
    [0.961, 0.475, 0.518], // 9  bright red
    [0.565, 0.937, 0.647], // 10 bright green
    [0.980, 0.937, 0.478], // 11 bright yellow
    [0.541, 0.773, 0.984], // 12 bright blue
    [0.941, 0.702, 0.980], // 13 bright magenta
    [0.498, 0.937, 0.984], // 14 bright cyan
    [0.988, 0.988, 1.000], // 15 bright white — Dracula
];

pub struct GpuRenderer {
    program: GLuint,
    vao: GLuint,
    vbo: GLuint,
    ebo: GLuint,
    loc_screen: GLint,
    loc_atlas: GLint,
    loc_use_atlas: GLint,
    loc_contrast: GLint,
    /// Image shader (RGBA, Kitty Graphics Protocol)
    img_program: GLuint,
    img_vao: GLuint,
    img_vbo: GLuint,
    img_loc_screen: GLint,
    img_loc_tex: GLint,
    pub cell_width: f32,
    pub cell_height: f32,
    ascender: f32,
    padding_x: f32,
    padding_y: f32,
    window_w: f32,
    window_h: f32,
    cols: usize,
    rows: usize,
    glyph_cache: Option<GlyphCache>,
    blink_on: bool,
    last_blink: std::time::Instant,
    cursor_beam_thickness: f32,
    text_contrast: f32,
    background_color: [f32; 4],
    /// Paleta ANSI color0–color15 leen de foxix.conf
    pub color_palette: [[f32; 3]; 16],
    /// Color de texto por defecto (foreground de foxix.conf)
    pub fg_default: [f32; 4],
    /// Color de fondo por defecto (background de foxix.conf)
    pub bg_default: [f32; 4],
}

impl GpuRenderer {
    pub fn new(
        font_size: u32,
        padding: f32,
        cols: usize,
        rows: usize,
        window_w: u32,
        window_h: u32,
        cursor_beam_thickness: f32,
        background_color: Option<[f32; 4]>,
        color_palette: [[f32; 3]; 16],
        fg_default: [f32; 4],
        bg_default: [f32; 4],
    ) -> Result<Self> {
        unsafe {
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

            // Shaders
            let program = compile_program(VERTEX_SHADER, FRAGMENT_SHADER)?;

            // VAO / VBO / EBO
            let mut vao = 0u32;
            let mut vbo = 0u32;
            let mut ebo = 0u32;
            gl::GenVertexArrays(1, &mut vao);
            gl::GenBuffers(1, &mut vbo);
            gl::GenBuffers(1, &mut ebo);

            gl::BindVertexArray(vao);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo);

            let stride = (STRIDE * std::mem::size_of::<f32>()) as GLsizei;
            let off = |i: usize| (i * std::mem::size_of::<f32>()) as *const _;
            // aPos   location 0
            gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, stride, off(0));
            gl::EnableVertexAttribArray(0);
            // aUV    location 1
            gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE, stride, off(2));
            gl::EnableVertexAttribArray(1);
            // aColor location 2
            gl::VertexAttribPointer(2, 4, gl::FLOAT, gl::FALSE, stride, off(4));
            gl::EnableVertexAttribArray(2);

            gl::BindVertexArray(0);

            // Uniform locations
            let u = |name: &str| {
                let cs = std::ffi::CString::new(name).unwrap();
                gl::GetUniformLocation(program, cs.as_ptr())
            };
            let loc_screen = u("uScreenSize");
            let loc_atlas = u("uAtlas");
            let loc_use_atlas = u("uUseAtlas");
            let loc_contrast = u("uTextContrast");

            // Glyph cache (FreeType)
            log::info!("Creating GlyphCache with font_size {}...", font_size);
            let glyph_cache = match GlyphCache::new(font_size) {
                Ok(gc) => {
                    log::info!("GlyphCache created successfully");
                    Some(gc)
                }
                Err(e) => {
                    log::error!("GlyphCache::new failed: {:?}", e);
                    None
                }
            };

            let (cell_width, cell_height, ascender) = if let Some(ref gc) = glyph_cache {
                (gc.cell_width(), gc.cell_height(), gc.ascender())
            } else {
                let h = font_size as f32 * 1.4;
                (font_size as f32 * 0.6, h, font_size as f32 * 1.1)
            };

            log::info!(
                "Renderer: celda {:.1}×{:.1}, ascender {:.1}, ventana {}×{}",
                cell_width,
                cell_height,
                ascender,
                window_w,
                window_h
            );

            let background_color = background_color.unwrap_or(bg_default);

            // ── Image shader init (Kitty Graphics Protocol) ─────────────────────────
            let img_program = compile_program(IMG_VERTEX_SHADER, IMG_FRAGMENT_SHADER).unwrap_or(0);

            let mut img_vao = 0u32;
            let mut img_vbo = 0u32;
            gl::GenVertexArrays(1, &mut img_vao);
            gl::GenBuffers(1, &mut img_vbo);
            gl::BindVertexArray(img_vao);
            gl::BindBuffer(gl::ARRAY_BUFFER, img_vbo);
            // Layout: vec2 aPos + vec2 aUV = 4 floats per vertex
            let img_stride = (4 * std::mem::size_of::<f32>()) as i32;
            gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, img_stride, std::ptr::null());
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(
                1,
                2,
                gl::FLOAT,
                gl::FALSE,
                img_stride,
                (2 * std::mem::size_of::<f32>()) as *const _,
            );
            gl::EnableVertexAttribArray(1);
            gl::BindVertexArray(0);

            let iu = |name: &str| {
                let cs = std::ffi::CString::new(name).unwrap();
                gl::GetUniformLocation(img_program, cs.as_ptr())
            };
            let img_loc_screen = iu("uScreen");
            let img_loc_tex = iu("uTex");

            Ok(Self {
                program,
                vao,
                vbo,
                ebo,
                loc_screen,
                loc_atlas,
                loc_use_atlas,
                loc_contrast,
                img_program,
                img_vao,
                img_vbo,
                img_loc_screen,
                img_loc_tex,
                cell_width,
                cell_height,
                ascender,
                padding_x: padding,
                padding_y: padding,
                window_w: window_w as f32,
                window_h: window_h as f32,
                cols,
                rows,
                glyph_cache,
                blink_on: true,
                last_blink: std::time::Instant::now(),
                cursor_beam_thickness,
                text_contrast: 1.8,
                background_color,
                color_palette,
                fg_default,
                bg_default,
            })
        }
    }

    pub fn resize(&mut self, cols: usize, rows: usize, w: u32, h: u32) {
        self.cols = cols;
        self.rows = rows;
        self.window_w = w as f32;
        self.window_h = h as f32;
        let used_w = cols as f32 * self.cell_width;
        let used_h = rows as f32 * self.cell_height;
        self.padding_x = ((w as f32 - used_w) / 2.0).max(0.0);
        self.padding_y = ((h as f32 - used_h) / 2.0).max(0.0);
    }

    /// Renderiza las imágenes del Kitty Graphics Protocol sobre el terminal.
    /// Debe llamarse después de render() (Pass 4).
    pub fn render_images(&self, placements: &[crate::terminal::graphics::ImagePlacement]) {
        if placements.is_empty() || self.img_program == 0 {
            return;
        }

        // Ordenar por z_index (menor primero)
        let mut sorted: Vec<_> = placements.iter().collect();
        sorted.sort_by_key(|p| p.z_index);

        unsafe {
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::UseProgram(self.img_program);
            gl::Uniform2f(self.img_loc_screen, self.window_w, self.window_h);
            gl::Uniform1i(self.img_loc_tex, 0);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindVertexArray(self.img_vao);
            gl::BindBuffer(gl::ARRAY_BUFFER, self.img_vbo);

            for p in sorted {
                if p.texture_id == 0 {
                    continue;
                }

                // Calcular posición en píxeles a partir de la celda
                let x0 = self.padding_x + p.col as f32 * self.cell_width;
                let y0 = self.padding_y + p.row as f32 * self.cell_height;

                // Si se especifica número de celdas (c y r), usar ese tamaño
                // De lo contrario, usar tamaño natural de la imagen
                let (pw, ph) = if p.cols > 0 && p.rows > 0 {
                    (
                        p.cols as f32 * self.cell_width,
                        p.rows as f32 * self.cell_height,
                    )
                } else {
                    // Usar tamaño natural de la imagen
                    (p.img_width as f32, p.img_height as f32)
                };

                // Quad: 6 vertices (2 triángulos), cada uno con vec2 pos + vec2 uv
                #[rustfmt::skip]
                let quad: [f32; 24] = [
                    // pos XY        UV
                    x0,      y0,      0.0, 0.0,
                    x0 + pw, y0,      1.0, 0.0,
                    x0 + pw, y0 + ph, 1.0, 1.0,
                    x0,      y0,      0.0, 0.0,
                    x0 + pw, y0 + ph, 1.0, 1.0,
                    x0,      y0 + ph, 0.0, 1.0,
                ];

                gl::BindTexture(gl::TEXTURE_2D, p.texture_id);
                gl::BufferData(
                    gl::ARRAY_BUFFER,
                    (quad.len() * 4) as isize,
                    quad.as_ptr() as *const _,
                    gl::STREAM_DRAW,
                );
                gl::DrawArrays(gl::TRIANGLES, 0, 6);
            }

            gl::BindVertexArray(0);
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }
    }
    pub fn render(
        &mut self,
        screen: &ScreenBuffer,
        cursor: &Cursor,
        sel: Option<(usize, usize, usize, usize)>,
    ) {
        // Blink cursor cada 600ms
        let now = std::time::Instant::now();
        if now.duration_since(self.last_blink).as_millis() > 600 {
            self.blink_on = !self.blink_on;
            self.last_blink = now;
        }

        // ── Clear ────────────────────────────────────────────────────────────
        unsafe {
            gl::Viewport(0, 0, self.window_w as i32, self.window_h as i32);
            gl::ClearColor(
                self.background_color[0],
                self.background_color[1],
                self.background_color[2],
                self.background_color[3],
            );
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }

        let mut verts: Vec<f32> = Vec::with_capacity(8192);
        let mut idxs: Vec<u32> = Vec::with_capacity(8192);
        let mut quad_i = 0u32;

        // ── Pass 1: Fondos de celdas con color personalizado ─────────────────
        // Celdas con background_color se saltan — el ClearColor ya pone ese fondo.
        for row in 0..screen.rows().min(self.rows) {
            for col in 0..screen.cols().min(self.cols) {
                let cell = match screen.cell(row, col) {
                    Some(c) => c,
                    None => continue,
                };

                let in_sel = sel.map_or(false, |(r0, c0, r1, c1)| {
                    is_in_selection(row, col, r0, c0, r1, c1)
                });

                let bg = if in_sel {
                    SEL_COLOR
                } else {
                    // Omitir si el fondo es exactamente el color por defecto
                    let has_custom = cell.attrs.bg_color.is_some()
                        || cell.attrs.bg_rgb.is_some()
                        || cell.attrs.inverse;
                    if !has_custom {
                        continue;
                    }
                    self.resolve_bg(&cell.attrs)
                };

                let x = self.padding_x + col as f32 * self.cell_width;
                let y = self.padding_y + row as f32 * self.cell_height;
                self.push_rect(
                    &mut verts,
                    &mut idxs,
                    &mut quad_i,
                    x,
                    y,
                    self.cell_width,
                    self.cell_height,
                    bg,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                );
            }
        }
        self.flush(&verts, &idxs, false);
        verts.clear();
        idxs.clear();
        quad_i = 0;

        // -- Pass 2: Text sprites (with correct cell_cols for wide/icon glyphs) --
        if let Some(ref mut cache) = self.glyph_cache {
            cache.bind();
        }

        for row in 0..screen.rows().min(self.rows) {
            for col in 0..screen.cols().min(self.cols) {
                let cell = match screen.cell(row, col) {
                    Some(c) => c,
                    None => continue,
                };
                if cell.c == ' ' || cell.c == '\0' {
                    continue;
                }

                let glyph = if let Some(ref mut cache) = self.glyph_cache {
                    cache.get_glyph(cell.c)
                } else {
                    None
                };

                let g = match glyph {
                    Some(g) if g.width > 0.5 => g,
                    _ => continue,
                };

                // Text color
                let fg = if let Some((r0, c0, r1, c1)) = sel {
                    if is_in_selection(row, col, r0, c0, r1, c1) {
                        self.background_color
                    } else {
                        self.resolve_fg(&cell.attrs)
                    }
                } else {
                    self.resolve_fg(&cell.attrs)
                };

                // Position: bearing already baked into canvas
                let x0 = self.padding_x + col as f32 * self.cell_width;
                let y0 = self.padding_y + row as f32 * self.cell_height;

                // Quad = sprite size (cell_cols * cell_w x cell_h)
                let quad_w = g.width;
                let quad_h = g.height;

                const AS: f32 = 4096.0;
                let u1 = g.u + g.width / AS;
                let v1 = g.v + g.height / AS;
                self.push_rect(
                    &mut verts,
                    &mut idxs,
                    &mut quad_i,
                    x0,
                    y0,
                    quad_w,
                    quad_h,
                    fg,
                    g.u,
                    g.v,
                    u1,
                    v1,
                );
            }
        }
        self.flush(&verts, &idxs, true);

        // ── Pass 3: Cursor beam ──────────────────────────────────────────────
        if cursor.visible() && self.blink_on {
            verts.clear();
            idxs.clear();
            quad_i = 0;
            let cx = self.padding_x + cursor.col() as f32 * self.cell_width;
            let cy = self.padding_y + cursor.row() as f32 * self.cell_height;
            self.push_rect(
                &mut verts,
                &mut idxs,
                &mut quad_i,
                cx,
                cy,
                self.cursor_beam_thickness,
                self.cell_height,
                CURSOR_COLOR,
                0.0,
                0.0,
                0.0,
                0.0,
            );
            self.flush(&verts, &idxs, false);
        }
    }

    fn flush(&self, verts: &[f32], idxs: &[u32], use_atlas: bool) {
        if idxs.is_empty() {
            return;
        }
        unsafe {
            gl::Viewport(0, 0, self.window_w as i32, self.window_h as i32);
            gl::UseProgram(self.program);
            gl::Uniform2f(self.loc_screen, self.window_w, self.window_h);
            gl::Uniform1i(self.loc_atlas, 0);
            gl::Uniform1i(self.loc_use_atlas, use_atlas as i32);
            // Contraste para texto más nítido
            gl::Uniform1f(
                self.loc_contrast,
                if use_atlas { self.text_contrast } else { 1.0 },
            );

            gl::BindVertexArray(self.vao);
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (verts.len() * 4) as GLsizeiptr,
                verts.as_ptr() as *const _,
                gl::STREAM_DRAW,
            );
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.ebo);
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (idxs.len() * 4) as GLsizeiptr,
                idxs.as_ptr() as *const _,
                gl::STREAM_DRAW,
            );
            gl::DrawElements(
                gl::TRIANGLES,
                idxs.len() as i32,
                gl::UNSIGNED_INT,
                std::ptr::null(),
            );
            gl::BindVertexArray(0);
        }
    }

    fn push_rect(
        &self,
        verts: &mut Vec<f32>,
        idxs: &mut Vec<u32>,
        qi: &mut u32,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color: [f32; 4],
        u0: f32,
        v0: f32,
        u1: f32,
        v1: f32,
    ) {
        let [r, g, b, a] = color;
        // 4 vértices en orden: TL TR BR BL
        let quad = [
            x,
            y,
            u0,
            v0,
            r,
            g,
            b,
            a,
            x + w,
            y,
            u1,
            v0,
            r,
            g,
            b,
            a,
            x + w,
            y + h,
            u1,
            v1,
            r,
            g,
            b,
            a,
            x,
            y + h,
            u0,
            v1,
            r,
            g,
            b,
            a,
        ];
        verts.extend_from_slice(&quad);
        let i = *qi;
        idxs.extend_from_slice(&[i, i + 1, i + 2, i, i + 2, i + 3]);
        *qi += 4;
    }

    fn resolve_fg(&self, attrs: &CellAttributes) -> [f32; 4] {
        // Inverse: intercambiar fg/bg
        if attrs.inverse {
            return self.resolve_bg_raw(attrs);
        }
        self.resolve_fg_raw(attrs)
    }

    fn resolve_bg(&self, attrs: &CellAttributes) -> [f32; 4] {
        if attrs.inverse {
            return self.resolve_fg_raw(attrs);
        }
        self.resolve_bg_raw(attrs)
    }

    #[inline]
    fn resolve_fg_raw(&self, attrs: &CellAttributes) -> [f32; 4] {
        if let Some(rgb) = attrs.fg_rgb {
            return [
                rgb[0] as f32 / 255.0,
                rgb[1] as f32 / 255.0,
                rgb[2] as f32 / 255.0,
                1.0,
            ];
        }
        if let Some(idx) = attrs.fg_color {
            return self.ansi256(idx);
        }
        if attrs.bold {
            // Bold: usar brighter version del fg si bold_is_bright (Kitty lo hace así)
            return [
                self.fg_default[0].min(1.0) * 1.1,
                self.fg_default[1].min(1.0) * 1.1,
                self.fg_default[2].min(1.0) * 1.1,
                1.0,
            ];
        }
        self.fg_default
    }

    #[inline]
    fn resolve_bg_raw(&self, attrs: &CellAttributes) -> [f32; 4] {
        if let Some(rgb) = attrs.bg_rgb {
            return [
                rgb[0] as f32 / 255.0,
                rgb[1] as f32 / 255.0,
                rgb[2] as f32 / 255.0,
                1.0,
            ];
        }
        if let Some(idx) = attrs.bg_color {
            return self.ansi256(idx);
        }
        self.bg_default
    }

    /// Paleta completa de 256 colores ANSI — los primeros 16 vienen de foxix.conf
    fn ansi256(&self, idx: u8) -> [f32; 4] {
        let (r, g, b) = match idx {
            // 0-15: paleta configurable desde foxix.conf (color0..color15)
            0..=15 => {
                let c = self.color_palette[idx as usize];
                return [c[0], c[1], c[2], 1.0];
            }
            // 16-231: cubo de color 6x6x6
            16..=231 => {
                let n = idx - 16;
                let bi = n % 6;
                let gi = (n / 6) % 6;
                let ri = (n / 36) % 6;
                let lut = [0u8, 95, 135, 175, 215, 255];
                (lut[ri as usize], lut[gi as usize], lut[bi as usize])
            }
            // 232-255: escala de grises
            232..=255 => {
                let v = 8 + (idx - 232) * 10;
                (v, v, v)
            }
        };
        [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0]
    }
}

impl Drop for GpuRenderer {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.program);
            gl::DeleteBuffers(1, &self.vbo);
            gl::DeleteBuffers(1, &self.ebo);
            gl::DeleteVertexArrays(1, &self.vao);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers de compilación de shaders
// ─────────────────────────────────────────────────────────────────────────────
unsafe fn compile_shader(kind: GLenum, src: &str) -> Result<GLuint> {
    let sh = gl::CreateShader(kind);
    let cs = std::ffi::CString::new(src).unwrap();
    gl::ShaderSource(sh, 1, &cs.as_ptr(), std::ptr::null());
    gl::CompileShader(sh);

    let mut ok: GLint = 0;
    gl::GetShaderiv(sh, gl::COMPILE_STATUS, &mut ok);
    if ok == 0 {
        let mut len: GLint = 0;
        gl::GetShaderiv(sh, gl::INFO_LOG_LENGTH, &mut len);
        let mut log = vec![0u8; len as usize];
        gl::GetShaderInfoLog(sh, len, std::ptr::null_mut(), log.as_mut_ptr() as *mut _);
        anyhow::bail!("Shader error: {}", String::from_utf8_lossy(&log));
    }
    Ok(sh)
}

unsafe fn compile_program(vs: &str, fs: &str) -> Result<GLuint> {
    let v = compile_shader(gl::VERTEX_SHADER, vs)?;
    let f = compile_shader(gl::FRAGMENT_SHADER, fs)?;
    let p = gl::CreateProgram();
    gl::AttachShader(p, v);
    gl::AttachShader(p, f);
    gl::LinkProgram(p);

    let mut ok: GLint = 0;
    gl::GetProgramiv(p, gl::LINK_STATUS, &mut ok);
    if ok == 0 {
        let mut len: GLint = 0;
        gl::GetProgramiv(p, gl::INFO_LOG_LENGTH, &mut len);
        let mut log = vec![0u8; len as usize];
        gl::GetProgramInfoLog(p, len, std::ptr::null_mut(), log.as_mut_ptr() as *mut _);
        anyhow::bail!("Program link error: {}", String::from_utf8_lossy(&log));
    }
    gl::DeleteShader(v);
    gl::DeleteShader(f);
    Ok(p)
}
