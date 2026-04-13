use crate::renderer::box_drawing::fill_box_drawing;
use anyhow::Result;
use gl::types::*;
use std::collections::HashMap;

// ─────────────────────────────────────────────────────────────────────────────
// GlyphInfo — posición en el atlas de texturas
// En el enfoque cell-sprite, cada entrada ocupa exactamente cell_w×cell_h px
// ─────────────────────────────────────────────────────────────────────────────
#[derive(Clone, Debug)]
pub struct GlyphInfo {
    /// UV esquina top-left en el atlas (normalizado 0..1)
    pub u: f32,
    pub v: f32,
    /// Tamaño del sprite en el atlas (siempre cell_width × cell_height)
    pub width: f32,
    pub height: f32,
    // bearing ya no se necesita: el sprite incluye el centrado
    pub bearing_x: f32,
    pub bearing_y: f32,
    pub advance_x: f32,
}

/// Rutas candidatas a fuentes mono en Linux (en orden de preferencia)
const FONT_PATHS: &[&str] = &[
    "/usr/share/fonts/TTF/JetBrainsMonoNerdFont-Regular.ttf",
    "/usr/share/fonts/TTF/JetBrainsMonoNF-Regular.ttf",
    "/usr/share/fonts/TTF/JetBrainsMonoNerdFontMono-Regular.ttf",
    "/usr/share/fonts/TTF/JetBrainsMonoNLNerdFont-Regular.ttf",
    "/usr/share/fonts/TTF/JetBrainsMono-Regular.ttf",
    "/usr/share/fonts/TTF/CaskaydiaCoveNerdFontMono-Regular.ttf",
    "/usr/share/fonts/Adwaita/AdwaitaMono-Regular.ttf",
    "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
    "/usr/share/fonts/TTF/DejaVuSans.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
];

/// Fuentes con iconos Nerd Font (para fallback de símbolos)
const SYMBOL_FONT_PATHS: &[&str] = &[
    "/usr/share/fonts/TTF/SymbolsNerdFont-Regular.ttf",
    "/usr/share/fonts/TTF/Symbols-2048-em Nerd Font Complete.ttf",
];

const ATLAS_SIZE: u32 = 4096;

pub struct GlyphCache {
    texture_id: GLuint,
    glyphs: HashMap<char, GlyphInfo>,
    cell_width: f32,
    cell_height: f32,
    /// Baseline desde la parte superior de la celda
    pub ascender: f32,
    // Posición del próximo sprite en el atlas
    next_x: u32,
    next_y: u32,
    row_height: u32,
    freetype_lib: Option<freetype::Library>,
    freetype_face: Option<freetype::Face>,
    /// Fuente de iconos Nerd Font (fallback para símbolos)
    symbol_face: Option<freetype::Face>,
}

impl GlyphCache {
    pub fn new(font_size: u32) -> Result<Self> {
        // ── FreeType ─────────────────────────────────────────────────────────
        let freetype_lib = freetype::Library::init().ok();
        let mut freetype_face: Option<freetype::Face> = None;

        // Los iconos Nerd Font pueden medir hasta 2x el tamaño de la fuente
        let base_cell_width = font_size as f32 * 0.8;
        let mut cell_width = base_cell_width;
        let mut cell_height = font_size as f32 * 2.0; // 2x para iconos, más compacto
        let mut ascender = font_size as f32 * 1.8;

        if let Some(ref lib) = freetype_lib {
            for &path in FONT_PATHS {
                if let Ok(face) = lib.new_face(path, 0) {
                    // Usar 144 DPI (equivalent a Wayland scale ~1.5x en 1080p)
                    // Kitty usa el DPI real del monitor; 144 da resultados similares
                    // en pantallas Full HD. Para 4K usar 192.
                    let dpi = detect_dpi();
                    if face
                        .set_char_size(0, (font_size * 64) as isize, dpi, dpi)
                        .is_ok()
                    {
                        let metrics = face.size_metrics();
                        let asc = if let Some(m) = metrics {
                            (m.ascender >> 6) as f32
                        } else {
                            font_size as f32 * 0.8
                        };
                        let desc = if let Some(m) = metrics {
                            (m.descender >> 6).unsigned_abs() as f32
                        } else {
                            font_size as f32 * 0.2
                        };

                        // Forzar a 4x para iconos Nerd Font grandes
                        let target_height = font_size as f32 * 4.0;
                        cell_height = target_height;
                        ascender = asc.max(font_size as f32 * 3.0);

                        log::info!(
                            "GlyphCache metrics: ascender={}, descender={}, target={}",
                            asc,
                            desc,
                            target_height
                        );

                        // Ancho de celda: avance del carácter '0' (más representativo que 'M')
                        for measure_ch in ['0', 'M', 'W'] {
                            if face
                                .load_char(measure_ch as usize, freetype::face::LoadFlag::DEFAULT)
                                .is_ok()
                            {
                                let adv = (face.glyph().advance().x >> 6) as f32;
                                if adv > 0.0 {
                                    cell_width = adv;
                                    break;
                                }
                            }
                        }

                        log::info!(
                            "GlyphCache: '{}' — celda {}×{}, ascender {}",
                            path,
                            cell_width,
                            cell_height,
                            ascender
                        );
                        freetype_face = Some(face);
                        break;
                    }
                }
            }
        }

        if freetype_face.is_none() {
            log::warn!("GlyphCache: sin fuente FreeType — usando métricas estimadas");
        }

        // ── Cargar fuente de iconos Nerd Font ────────────────────────────────
        // Los iconos Nerd Font pueden ser más grandes que el texto normal
        // Cargamos con font_size * 4 para asegurar que los iconos no se corten
        let mut symbol_face: Option<freetype::Face> = None;
        let symbol_font_size = font_size * 4; // Fuente de símbolos muy grande
        if let Some(ref lib) = freetype_lib {
            let dpi = detect_dpi();
            for &path in SYMBOL_FONT_PATHS {
                if let Ok(face) = lib.new_face(path, 0) {
                    if face
                        .set_char_size(0, (symbol_font_size * 64) as isize, dpi, dpi)
                        .is_ok()
                    {
                        log::info!(
                            "GlyphCache: Symbols font '{}' at {}pt",
                            path,
                            symbol_font_size
                        );
                        symbol_face = Some(face);
                        break;
                    }
                }
            }
        }

        // ── Atlas de texturas (canal R) ──────────────────────────────────────
        let texture_id = unsafe {
            let mut tex: GLuint = 0;
            gl::GenTextures(1, &mut tex);
            gl::BindTexture(gl::TEXTURE_2D, tex);
            gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);
            // LINEAR: para que los iconos y texto se vean suaves pero nítidos (estilo Kitty)
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
            let zeros = vec![0u8; (ATLAS_SIZE * ATLAS_SIZE) as usize];
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RED as i32,
                ATLAS_SIZE as i32,
                ATLAS_SIZE as i32,
                0,
                gl::RED,
                gl::UNSIGNED_BYTE,
                zeros.as_ptr() as *const _,
            );
            tex
        };

        Ok(Self {
            texture_id,
            glyphs: HashMap::new(),
            cell_width,
            cell_height,
            ascender,
            next_x: 1,
            next_y: 1,
            row_height: 0,
            freetype_lib,
            freetype_face,
            symbol_face,
        })
    }

    pub fn cell_width(&self) -> f32 {
        self.cell_width
    }
    pub fn cell_height(&self) -> f32 {
        self.cell_height
    }
    pub fn ascender(&self) -> f32 {
        self.ascender
    }
    pub fn texture_id(&self) -> GLuint {
        self.texture_id
    }

    pub fn bind(&self) {
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, self.texture_id);
        }
    }

    /// Devuelve info del sprite (cargándolo al atlas si aún no está).
    pub fn get_glyph(&mut self, c: char) -> Option<GlyphInfo> {
        if let Some(info) = self.glyphs.get(&c) {
            return if info.width > 0.0 {
                Some(info.clone())
            } else {
                None
            };
        }

        // Intentar con fuente principal
        if let Some(ref face) = self.freetype_face {
            let glyph_index = face.get_char_index(c as usize);
            let has_in_main = !matches!(glyph_index, None | Some(0));

            if c as u32 >= 0xE000 {
                log::debug!(
                    "Glyph {:04X}: main_font={}, idx={:?}",
                    c as u32,
                    has_in_main,
                    glyph_index
                );
            }

            if has_in_main {
                if let Some(info) = self.rasterize_cell_sprite(c) {
                    return if info.width > 0.0 { Some(info) } else { None };
                }
            }
        }

        // Intentar con fuente de símbolos Nerd Font
        let has_symbol_font = self.symbol_face.is_some();
        if has_symbol_font {
            let glyph_index = unsafe {
                self.symbol_face
                    .as_ref()
                    .unwrap_unchecked()
                    .get_char_index(c as usize)
            };
            let has_in_symbol = !matches!(glyph_index, None | Some(0));

            if c as u32 >= 0xE000 {
                log::debug!(
                    "Glyph {:04X}: symbol_font={}, idx={:?}",
                    c as u32,
                    has_in_symbol,
                    glyph_index
                );
            }

            if has_in_symbol {
                if let Some(info) = self.rasterize_symbol_sprite(c) {
                    return if info.width > 0.0 { Some(info) } else { None };
                }
            }
        }

        // Software rasterizer para caracteres especiales
        let code = c as u32;

        // Box Drawing Lines (U+2500–2580) — usa el módulo dedicado
        if (0x2500..=0x257F).contains(&code) {
            return self.rasterize_box_drawing(c);
        }

        // Block Elements (U+2580–2590F) + Braille (U+2800–28FF) + bloques extra
        let needs_sw_fallback = (0x2580..=0x259F).contains(&code)
            || (0x2800..=0x28FF).contains(&code)
            || matches!(c, '█' | '▀' | '▄' | '▌' | '▐' | '░' | '▒' | '▓');

        if needs_sw_fallback {
            return self.rasterize_fallback(c);
        }

        // Marcar como no disponible
        self.glyphs.insert(
            c,
            GlyphInfo {
                u: 0.0,
                v: 0.0,
                width: 0.0,
                height: 0.0,
                bearing_x: 0.0,
                bearing_y: 0.0,
                advance_x: self.cell_width,
            },
        );
        None
    }
    // ── Cell-sprite rendering (idéntico al de Kitty: render_glyphs_in_cells) ────
    //
    // Pre-compositar el glyph en un canvas de exactamente cell_w × cell_h.
    // El bitmap de FreeType se coloca en el canvas usando bearing_x/y para
    // que la baseline quede exactamente alineada en el grid de celdas.
    // Resultado: el atlas contiene sprites perfectos → GPU dibuja quads simples.
    fn rasterize_cell_sprite(&mut self, c: char) -> Option<GlyphInfo> {
        let cw = self.cell_width as i32;
        let ch = self.cell_height as i32;
        let baseline = self.ascender as i32;

        let face = self.freetype_face.as_mut()?;

        // Usar RENDER para grayscale antialiased
        if let Err(e) = face.load_char(c as usize, freetype::face::LoadFlag::RENDER) {
            log::warn!("Failed to load glyph {:04X}: {:?}", c as u32, e);
            return self.store_empty_sprite(c);
        }

        let glyph_slot = face.glyph();
        let bitmap = glyph_slot.bitmap();
        let bmp_w = bitmap.width() as i32;
        let bmp_h = bitmap.rows() as i32;
        let raw = bitmap.buffer();
        let pitch = bitmap.pitch().unsigned_abs() as i32;
        let pixel_mode = bitmap.pixel_mode().ok()?;

        // Espacio → sprite vacío
        if bmp_w == 0 || bmp_h == 0 {
            return self.store_empty_sprite(c);
        }

        // ── Canvas cell_w × cell_h pre-inicializado en cero ──────────────────
        let mut canvas = vec![0u8; (cw * ch) as usize];

        // Offset en X: bearing_left (puede ser negativo para itálica)
        let x_off = glyph_slot.bitmap_left();
        // Offset en Y: baseline − bitmap_top sitúa la fila 0 del glyph
        let y_off = baseline - glyph_slot.bitmap_top();

        for row in 0..bmp_h {
            let dst_y = y_off + row;
            if dst_y < 0 || dst_y >= ch {
                continue;
            }
            for col in 0..bmp_w {
                let dst_x = x_off + col;
                if dst_x < 0 || dst_x >= cw {
                    continue;
                }
                let alpha = match pixel_mode {
                    freetype::bitmap::PixelMode::Gray => raw[(row * pitch + col) as usize],
                    freetype::bitmap::PixelMode::Mono => {
                        let byte = raw[(row * pitch + col / 8) as usize];
                        if (byte >> (7 - (col % 8))) & 1 != 0 {
                            255
                        } else {
                            0
                        }
                    }
                    _ => raw[(row * pitch + col) as usize],
                };
                canvas[(dst_y * cw + dst_x) as usize] = alpha;
            }
        }

        // ── Subir canvas completo al atlas ────────────────────────────────────
        if self.next_x + cw as u32 + 1 > ATLAS_SIZE {
            self.next_x = 1;
            self.next_y += self.row_height + 1;
            self.row_height = 0;
        }
        if self.next_y + ch as u32 + 1 > ATLAS_SIZE {
            log::warn!("Atlas lleno — glyph {:04X} descartado", c as u32);
            return None;
        }

        unsafe {
            gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);
            gl::BindTexture(gl::TEXTURE_2D, self.texture_id);
            gl::TexSubImage2D(
                gl::TEXTURE_2D,
                0,
                self.next_x as i32,
                self.next_y as i32,
                cw,
                ch,
                gl::RED,
                gl::UNSIGNED_BYTE,
                canvas.as_ptr() as *const _,
            );
        }

        let u = self.next_x as f32 / ATLAS_SIZE as f32;
        let v = self.next_y as f32 / ATLAS_SIZE as f32;

        let info = GlyphInfo {
            u,
            v,
            // ¡Tamaño siempre = celda completa! El bearing ya está en el canvas.
            width: cw as f32,
            height: ch as f32,
            bearing_x: 0.0, // baked en canvas — GPU no necesita offset adicional
            bearing_y: 0.0,
            advance_x: self.cell_width,
        };

        if ch as u32 > self.row_height {
            self.row_height = ch as u32;
        }
        self.next_x += cw as u32 + 1;
        self.glyphs.insert(c, info.clone());
        Some(info)
    }

    fn store_empty_sprite(&mut self, c: char) -> Option<GlyphInfo> {
        let info = GlyphInfo {
            u: 0.0,
            v: 0.0,
            width: 0.0,
            height: 0.0,
            bearing_x: 0.0,
            bearing_y: 0.0,
            advance_x: self.cell_width,
        };
        self.glyphs.insert(c, info.clone());
        Some(info)
    }

    /// Rasteriza un ícono Nerd Font en canvas cell_w×cell_h (idéntico a rasterize_cell_sprite).
    /// Centra el bitmap del ícono en la celda para que nunca se corte o desborde.
    /// Para iconos más anchos que cell_w, usa canvas de 2×cell_w (wide glyph).
    fn rasterize_symbol_sprite(&mut self, c: char) -> Option<GlyphInfo> {
        let cw = self.cell_width as i32;
        let ch = self.cell_height as i32;

        let face = self.symbol_face.as_mut()?;
        face.load_char(c as usize, freetype::face::LoadFlag::RENDER)
            .ok()?;

        let glyph_slot = face.glyph();
        let bitmap = glyph_slot.bitmap();
        let bmp_w = bitmap.width() as i32;
        let bmp_h = bitmap.rows() as i32;

        if bmp_w == 0 || bmp_h == 0 {
            return None;
        }

        // Los iconos Nerd Font suelen ser más anchos que cell_w.
        // Usar un canvas ampliado si el bitmap no cabe en 1 celda.
        let canvas_w = if bmp_w > cw { cw * 2 } else { cw };
        let canvas_h = ch;

        // Canvas fijo canvas_w × canvas_h — igual que rasterize_cell_sprite
        let mut canvas = vec![0u8; (canvas_w * canvas_h) as usize];
        let raw = bitmap.buffer();
        let pitch = bitmap.pitch().unsigned_abs() as i32;

        // Centrar el bitmap del ícono en el canvas
        let x_off = (canvas_w - bmp_w) / 2;
        let y_off = (canvas_h - bmp_h) / 2;

        for row in 0..bmp_h {
            let dst_y = y_off + row;
            if dst_y < 0 || dst_y >= canvas_h {
                continue;
            }
            for col in 0..bmp_w {
                let dst_x = x_off + col;
                if dst_x < 0 || dst_x >= canvas_w {
                    continue;
                }
                let alpha = raw[(row * pitch + col) as usize];
                canvas[(dst_y * canvas_w + dst_x) as usize] = alpha;
            }
        }

        // Subir canvas al atlas
        if self.next_x + canvas_w as u32 + 1 > ATLAS_SIZE {
            self.next_x = 1;
            self.next_y += self.row_height + 1;
            self.row_height = 0;
        }
        if self.next_y + canvas_h as u32 + 1 > ATLAS_SIZE {
            return None;
        }

        unsafe {
            gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);
            gl::BindTexture(gl::TEXTURE_2D, self.texture_id);
            gl::TexSubImage2D(
                gl::TEXTURE_2D,
                0,
                self.next_x as i32,
                self.next_y as i32,
                canvas_w,
                canvas_h,
                gl::RED,
                gl::UNSIGNED_BYTE,
                canvas.as_ptr() as *const _,
            );
        }

        let info = GlyphInfo {
            u: self.next_x as f32 / ATLAS_SIZE as f32,
            v: self.next_y as f32 / ATLAS_SIZE as f32,
            width: canvas_w as f32,
            height: canvas_h as f32,
            bearing_x: 0.0, // baked en canvas
            bearing_y: 0.0,
            advance_x: self.cell_width,
        };

        if canvas_h as u32 > self.row_height {
            self.row_height = canvas_h as u32;
        }
        self.next_x += canvas_w as u32 + 1;
        self.glyphs.insert(c, info.clone());
        Some(info)
    }

    fn rasterize_box_drawing(&mut self, c: char) -> Option<GlyphInfo> {
        let cw = self.cell_width as u32;
        let ch = self.cell_height as u32;
        if cw == 0 || ch == 0 {
            return None;
        }

        let mut cell_buf = vec![0u8; (cw * ch) as usize];
        fill_box_drawing(c, &mut cell_buf, cw, ch);

        if self.next_x + cw + 1 > ATLAS_SIZE {
            self.next_x = 1;
            self.next_y += self.row_height + 1;
            self.row_height = 0;
        }
        if self.next_y + ch + 1 > ATLAS_SIZE {
            return None;
        }

        unsafe {
            gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);
            gl::BindTexture(gl::TEXTURE_2D, self.texture_id);
            gl::TexSubImage2D(
                gl::TEXTURE_2D,
                0,
                self.next_x as i32,
                self.next_y as i32,
                cw as i32,
                ch as i32,
                gl::RED,
                gl::UNSIGNED_BYTE,
                cell_buf.as_ptr() as *const _,
            );
        }

        let u = self.next_x as f32 / ATLAS_SIZE as f32;
        let v = self.next_y as f32 / ATLAS_SIZE as f32;
        let info = GlyphInfo {
            u,
            v,
            width: cw as f32,
            height: ch as f32,
            bearing_x: 0.0,
            bearing_y: 0.0,
            advance_x: self.cell_width,
        };
        if ch > self.row_height {
            self.row_height = ch;
        }
        self.next_x += cw + 1;
        self.glyphs.insert(c, info.clone());
        Some(info)
    }

    /// Fallback sintético: dibuja un rectángulo vacío (outline) en la celda
    fn rasterize_fallback(&mut self, c: char) -> Option<GlyphInfo> {
        let cw = self.cell_width as u32;
        let ch = self.cell_height as u32;
        if cw == 0 || ch == 0 {
            return None;
        }

        // Todos los block elements y braille van a fill_block_element
        // que maneja U+2580-259F, U+2800-28FF y chars de bloque extra
        let mut cell_buf = vec![0u8; (cw * ch) as usize];
        let code = c as u32;

        let is_special = (0x2580..=0x259F).contains(&code)
            || (0x2800..=0x28FF).contains(&code)
            || matches!(c, '█' | '▀' | '▄' | '▌' | '▐' | '░' | '▒' | '▓');

        if is_special {
            fill_block_element(c, &mut cell_buf, cw, ch);
        }
        // Si no es especial, el buffer queda en ceros → celda transparente (vacía)

        if self.next_x + cw + 1 > ATLAS_SIZE {
            self.next_x = 1;
            self.next_y += self.row_height + 1;
            self.row_height = 0;
        }
        if self.next_y + ch + 1 > ATLAS_SIZE {
            return None;
        }

        unsafe {
            gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);
            gl::BindTexture(gl::TEXTURE_2D, self.texture_id);
            gl::TexSubImage2D(
                gl::TEXTURE_2D,
                0,
                self.next_x as i32,
                self.next_y as i32,
                cw as i32,
                ch as i32,
                gl::RED,
                gl::UNSIGNED_BYTE,
                cell_buf.as_ptr() as *const _,
            );
        }

        let u = self.next_x as f32 / ATLAS_SIZE as f32;
        let v = self.next_y as f32 / ATLAS_SIZE as f32;

        let info = GlyphInfo {
            u,
            v,
            width: cw as f32,
            height: ch as f32,
            bearing_x: 0.0,
            bearing_y: 0.0,
            advance_x: self.cell_width,
        };

        if ch > self.row_height {
            self.row_height = ch;
        }
        self.next_x += cw + 1;
        self.glyphs.insert(c, info.clone());
        Some(info)
    }
}

impl Drop for GlyphCache {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteTextures(1, &self.texture_id);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// DPI detection — como Kitty usa el DPI real para renderizado preciso
// ─────────────────────────────────────────────────────────────────────────────

/// Detecta el DPI del sistema. Prioridad:
/// 1. GDK_SCALE (Wayland scale factor) → dpi = 96 * scale
/// 2. xrdb Xft.dpi
/// 3. Fallback: 140 (entre 96 y 192, funciona bien en 1080p y WQHD)
fn detect_dpi() -> u32 {
    // 1. Variable de entorno Wayland/GDK
    if let Ok(scale) = std::env::var("GDK_SCALE") {
        if let Ok(s) = scale.parse::<f32>() {
            if s > 0.0 {
                return (96.0 * s) as u32;
            }
        }
    }
    // También GDK_DPI_SCALE (escala fraccionaria)
    if let Ok(ds) = std::env::var("GDK_DPI_SCALE") {
        if let Ok(s) = ds.parse::<f32>() {
            if s > 0.0 {
                return (96.0 * s) as u32;
            }
        }
    }
    // 2. xrdb Xft.dpi (X11)
    if let Ok(out) = std::process::Command::new("xrdb").arg("-query").output() {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            if line.starts_with("Xft.dpi:") {
                if let Some(val) = line.split(':').nth(1) {
                    if let Ok(dpi) = val.trim().parse::<u32>() {
                        return dpi;
                    }
                }
            }
        }
    }
    // 3. Fallback: 96 DPI — estándar para pantallas 1366×768 y 1920×1080.
    // Produce celdas ~10×22 con font_size=12, igual que Kitty por defecto.
    // Para HiDPI (4K, 2K) usa GDK_SCALE=2 o GDK_DPI_SCALE=1.5 en tu entorno.
    96
}

// ─────────────────────────────────────────────────────────────────────────────
// Elementos de bloque Unicode (U+2580–U+259F) y Braille (U+2800–U+28FF)
// ─────────────────────────────────────────────────────────────────────────────
fn fill_block_element(c: char, buf: &mut [u8], cw: u32, ch: u32) {
    let code = c as u32;

    // Braille: U+2800–U+28FF → 8 dots en grid 2×4
    if (0x2800..=0x28FF).contains(&code) {
        fill_braille_pattern(code, buf, cw, ch);
        return;
    }

    let h1 = ch / 8;
    let h2 = ch * 2 / 8;
    let h3 = ch * 3 / 8;
    let h4 = ch * 4 / 8;
    let h5 = ch * 5 / 8;
    let h6 = ch * 6 / 8;
    let h7 = ch * 7 / 8;
    let w1 = cw / 8;
    let w2 = cw * 2 / 8;
    let w3 = cw * 3 / 8;
    let w4 = cw * 4 / 8;

    // fill un rectángulo en el buffer
    let fill = |buf: &mut [u8], x0: u32, y0: u32, x1: u32, y1: u32| {
        for y in y0..y1.min(ch) {
            for x in x0..x1.min(cw) {
                buf[(y * cw + x) as usize] = 255;
            }
        }
    };

    match code {
        // U+2580 UPPER HALF BLOCK
        0x2580 => fill(buf, 0, 0, cw, h4),
        // U+2581 LOWER ONE EIGHTH
        0x2581 => fill(buf, 0, ch - h1, cw, ch),
        // U+2582 LOWER ONE QUARTER
        0x2582 => fill(buf, 0, ch - h2, cw, ch),
        // U+2583 LOWER THREE EIGHTHS
        0x2583 => fill(buf, 0, ch - h3, cw, ch),
        // U+2584 LOWER HALF BLOCK
        0x2584 => fill(buf, 0, h4, cw, ch),
        // U+2585 LOWER FIVE EIGHTHS
        0x2585 => fill(buf, 0, ch - h5, cw, ch),
        // U+2586 LOWER THREE QUARTERS
        0x2586 => fill(buf, 0, ch - h6, cw, ch),
        // U+2587 LOWER SEVEN EIGHTHS
        0x2587 => fill(buf, 0, ch - h7, cw, ch),
        // U+2588 FULL BLOCK
        0x2588 => fill(buf, 0, 0, cw, ch),
        // U+2589 LEFT SEVEN EIGHTHS
        0x2589 => fill(buf, 0, 0, cw * 7 / 8, ch),
        // U+258A LEFT THREE QUARTERS
        0x258A => fill(buf, 0, 0, cw * 6 / 8, ch),
        // U+258B LEFT FIVE EIGHTHS
        0x258B => fill(buf, 0, 0, cw * 5 / 8, ch),
        // U+258C LEFT HALF BLOCK
        0x258C => fill(buf, 0, 0, w4, ch),
        // U+258D LEFT THREE EIGHTHS
        0x258D => fill(buf, 0, 0, w3, ch),
        // U+258E LEFT ONE QUARTER
        0x258E => fill(buf, 0, 0, w2, ch),
        // U+258F LEFT ONE EIGHTH
        0x258F => fill(buf, 0, 0, w1, ch),
        // U+2590 RIGHT HALF BLOCK
        0x2590 => fill(buf, w4, 0, cw, ch),
        // U+2591 LIGHT SHADE (25% dither)
        0x2591 => {
            for y in 0..ch {
                for x in 0..cw {
                    if (x + y * 2) % 4 == 0 {
                        buf[(y * cw + x) as usize] = 255;
                    }
                }
            }
        }
        // U+2592 MEDIUM SHADE (50% dither)
        0x2592 => {
            for y in 0..ch {
                for x in 0..cw {
                    if (x + y) % 2 == 0 {
                        buf[(y * cw + x) as usize] = 255;
                    }
                }
            }
        }
        // U+2593 DARK SHADE (75% dither)
        0x2593 => {
            for y in 0..ch {
                for x in 0..cw {
                    if (x + y) % 4 != 0 {
                        buf[(y * cw + x) as usize] = 255;
                    }
                }
            }
        }
        // U+2594 UPPER ONE EIGHTH
        0x2594 => fill(buf, 0, 0, cw, h1),
        // U+2595 RIGHT ONE EIGHTH
        0x2595 => fill(buf, cw - w1, 0, cw, ch),
        // U+2596 QUADRANT LOWER LEFT
        0x2596 => fill(buf, 0, h4, w4, ch),
        // U+2597 QUADRANT LOWER RIGHT
        0x2597 => fill(buf, w4, h4, cw, ch),
        // U+2598 QUADRANT UPPER LEFT
        0x2598 => fill(buf, 0, 0, w4, h4),
        // U+2599 QUADRANT UPPER LEFT AND LOWER LEFT AND LOWER RIGHT
        0x2599 => {
            fill(buf, 0, 0, w4, ch);
            fill(buf, w4, h4, cw, ch);
        }
        // U+259A QUADRANT UPPER LEFT AND LOWER RIGHT
        0x259A => {
            fill(buf, 0, 0, w4, h4);
            fill(buf, w4, h4, cw, ch);
        }
        // U+259B QUADRANT UPPER LEFT AND UPPER RIGHT AND LOWER LEFT
        0x259B => {
            fill(buf, 0, 0, cw, h4);
            fill(buf, 0, h4, w4, ch);
        }
        // U+259C QUADRANT UPPER LEFT AND UPPER RIGHT AND LOWER RIGHT
        0x259C => {
            fill(buf, 0, 0, cw, h4);
            fill(buf, w4, h4, cw, ch);
        }
        // U+259D QUADRANT UPPER RIGHT
        0x259D => fill(buf, w4, 0, cw, h4),
        // U+259E QUADRANT UPPER RIGHT AND LOWER LEFT
        0x259E => {
            fill(buf, w4, 0, cw, h4);
            fill(buf, 0, h4, w4, ch);
        }
        // U+259F QUADRANT UPPER RIGHT AND LOWER LEFT AND LOWER RIGHT
        0x259F => {
            fill(buf, w4, 0, cw, h4);
            fill(buf, 0, h4, cw, ch);
        }
        // Fallback: bloque completo para chars no reconocidos en el rango
        _ => fill(buf, 0, 0, cw, ch),
    }
}

fn fill_braille_pattern(code: u32, buf: &mut [u8], cw: u32, ch: u32) {
    // Braille usa una grilla 2 columnas x 4 filas de dots (Unicoe estándar).
    // Cada dot tiene un tamaño proporcional y espaciado uniforme.
    let bit = |n: usize| -> bool { (code & (1 << n)) != 0 };

    // Tamaño de cada punto: ~2px en celda 10x22, escalado con la celda
    let dot_w = ((cw as f32 * 0.28).round() as u32).max(1);
    let dot_h = ((ch as f32 * 0.18).round() as u32).max(1);

    // Espaciado entre centros de dots (horizontal y vertical)
    let hgap = cw / 2; // gap horizontal entre columnas
    let vgap = ch / 4; // gap vertical entre filas

    // Origen de la cuadrícula (centrado en la celda)
    let grid_w = hgap + dot_w;
    let grid_h = vgap * 3 + dot_h;
    let ox = cw.saturating_sub(grid_w) / 2;
    let oy = ch.saturating_sub(grid_h) / 2;

    // Posición de cada columna e fila
    let col_x = [ox, ox + hgap];
    let row_y = [oy, oy + vgap, oy + vgap * 2, oy + vgap * 3];

    // Mapa de bits Braille estándar (Unicode):
    // bits 0-2 = col izq filas 0-2, bit 6 = col izq fila 3
    // bits 3-5 = col der filas 0-2, bit 7 = col der fila 3
    let dots = [
        (0, col_x[0], row_y[0]),
        (1, col_x[0], row_y[1]),
        (2, col_x[0], row_y[2]),
        (3, col_x[1], row_y[0]),
        (4, col_x[1], row_y[1]),
        (5, col_x[1], row_y[2]),
        (6, col_x[0], row_y[3]),
        (7, col_x[1], row_y[3]),
    ];

    for (bit_idx, dx, dy) in dots {
        if !bit(bit_idx) {
            continue;
        }
        let x_end = (dx + dot_w).min(cw);
        let y_end = (dy + dot_h).min(ch);
        for row in dy..y_end {
            for col in dx..x_end {
                buf[(row * cw + col) as usize] = 255;
            }
        }
    }
}
