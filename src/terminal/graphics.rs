/// Kitty Graphics Protocol (KGP) — Foxix implementation
/// ─────────────────────────────────────────────────────────────────────────────
/// Docs: https://sw.kovidgoyal.net/kitty/graphics-protocol.html
///
/// Escape: ESC _ G <control-data> ; <payload> ESC \
///   - control-data: key=value pairs separated by comma
///   - payload: base64-encoded image data (chunked with m=1/m=0)
///
/// Foxix renders images as full-color GL_RGBA textures in a separate pass,
/// positioned at the terminal cell where the APC sequence was received.
use gl::types::GLuint;
use image::GenericImageView;

// ─────────────────────────────────────────────────────────────────────────────
// Data types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GraphicsFormat {
    Rgba = 32,
    Rgb = 24,
    Png = 100,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GraphicsAction {
    Transmit,        // a=T
    TransmitDisplay, // a=t
    Display,         // a=p
    Delete,          // a=d
    Query,           // a=q
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransmissionType {
    Direct,    // t=d (default) — data in payload
    File,      // t=f — path in payload
    TempFile,  // t=t
    SharedMem, // t=s
}

/// Image placement: maps image_id → position in terminal grid
#[derive(Debug, Clone)]
pub struct ImagePlacement {
    pub image_id: u32,
    pub row: usize,
    pub col: usize,
    pub rows: u32,
    pub cols: u32,
    /// OpenGL texture ID (0 = not yet uploaded)
    pub texture_id: u32,
    pub img_width: u32,
    pub img_height: u32,
    pub z_index: i32,
}

/// Accumulated image data (chunks via m=1)
#[derive(Debug, Default)]
pub struct PendingImage {
    pub id: u32,
    pub action: Option<GraphicsAction>,
    pub format: GraphicsFormat,
    pub trans: TransmissionType,
    pub width: u32,       // image width in pixels
    pub height: u32,      // image height in pixels
    pub payload: Vec<u8>, // decoded base64
    pub row: usize,       // cell row position
    pub col: usize,       // cell column position
    pub rows: u32,        // number of cell rows to occupy
    pub cols: u32,        // number of cell cols to occupy
    pub quiet: u8,
    pub data_width: u32,  // s - data width (pixels)
    pub data_height: u32, // v - data height (pixels)
    pub z_index: i32,     // z - layer order
}

impl Default for GraphicsFormat {
    fn default() -> Self {
        GraphicsFormat::Png
    }
}

impl Default for TransmissionType {
    fn default() -> Self {
        TransmissionType::Direct
    }
}

/// Kitty Graphics Protocol manager
pub struct GraphicsManager {
    /// id → placement (for rendering)
    pub placements: Vec<ImagePlacement>,
    /// Currently accumulating image (for chunked transfers)
    pending: Option<PendingImage>,
    /// Next auto-assigned image id
    next_id: u32,
}

impl GraphicsManager {
    pub fn new() -> Self {
        Self {
            placements: Vec::new(),
            pending: None,
            next_id: 1,
        }
    }

    /// Parse and handle one APC graphics command.
    /// `apc_data` is the raw content between `\x1b_G` and `\x1b\\` (no prefix).
    /// `cursor_row/col` is where the APC was received.
    pub fn handle_apc(&mut self, apc_data: &str, cursor_row: usize, cursor_col: usize) {
        // Split at first ';' → control data | payload
        let (ctrl, b64) = if let Some(pos) = apc_data.find(';') {
            (&apc_data[..pos], &apc_data[pos + 1..])
        } else {
            (apc_data, "")
        };

        log::info!(
            "KGP handle_apc: ctrl='{}', payload_len={}, row={}, col={}",
            ctrl,
            b64.len(),
            cursor_row,
            cursor_col
        );

        // Parse key=value pairs
        let mut action = GraphicsAction::Transmit;
        let mut format = GraphicsFormat::Png;
        let mut trans_type = TransmissionType::Direct;
        let mut img_id: u32 = 0;
        let mut data_width: u32 = 0;
        let mut data_height: u32 = 0;
        let mut img_width: u32 = 0;
        let mut img_height: u32 = 0;
        let mut cols: u32 = 0;
        let mut rows: u32 = 0;
        let mut more = false;
        let mut quiet: u8 = 0;
        let mut z_index: i32 = 0;

        for kv in ctrl.split(',') {
            let mut it = kv.splitn(2, '=');
            let k = it.next().unwrap_or("").trim();
            let v = it.next().unwrap_or("").trim();
            match k {
                "a" => {
                    action = match v {
                        "t" => GraphicsAction::TransmitDisplay,
                        "T" => GraphicsAction::Transmit,
                        "p" => GraphicsAction::Display,
                        "d" => GraphicsAction::Delete,
                        "q" => GraphicsAction::Query,
                        _ => GraphicsAction::Transmit,
                    }
                }
                "f" => {
                    format = match v {
                        "32" => GraphicsFormat::Rgba,
                        "24" => GraphicsFormat::Rgb,
                        "100" => GraphicsFormat::Png,
                        _ => GraphicsFormat::Png,
                    }
                }
                "t" => {
                    trans_type = match v {
                        "d" => TransmissionType::Direct,
                        "f" => TransmissionType::File,
                        "t" => TransmissionType::TempFile,
                        "s" => TransmissionType::SharedMem,
                        _ => TransmissionType::Direct,
                    }
                }
                "i" => img_id = v.parse().unwrap_or(0),
                "s" => data_width = v.parse().unwrap_or(0),
                "v" => data_height = v.parse().unwrap_or(0),
                "w" => img_width = v.parse().unwrap_or(0),
                "h" => img_height = v.parse().unwrap_or(0),
                "c" => cols = v.parse().unwrap_or(0),
                "r" => rows = v.parse().unwrap_or(0),
                "m" => more = v.trim() == "1",
                "q" => quiet = v.parse().unwrap_or(0),
                "z" => z_index = v.parse().unwrap_or(0),
                _ => {}
            }
        }

        log::info!(
            "KGP: id={}, fmt={:?}, trans={:?}, s={}, v={}, c={}, r={}, m={}",
            img_id,
            format,
            trans_type,
            data_width,
            data_height,
            cols,
            rows,
            more
        );

        // Asignar id automático si no viene
        if img_id == 0 {
            img_id = self.next_id;
            self.next_id += 1;
        }

        // Handle chunked transfer (m=1)
        if let Some(ref mut p) = self.pending {
            // Accumulate more data
            if !b64.is_empty() {
                p.payload.extend_from_slice(b64.as_bytes());
            }
            if !more {
                // Last chunk - finalize
                log::info!(
                    "KGP: completing chunked transfer, total {} bytes",
                    p.payload.len()
                );
                let finished = self.pending.take().unwrap();
                self.finalize_image(finished);
            }
            return;
        }

        // Handle t=f (file path in payload, not base64)
        if trans_type == TransmissionType::File || trans_type == TransmissionType::TempFile {
            // Payload is a file path, not base64 encoded
            let path = std::str::from_utf8(b64.as_bytes())
                .unwrap_or("")
                .trim()
                .to_string();
            if path.is_empty() {
                log::warn!("KGP: empty file path");
                return;
            }
            let path = if path.starts_with('/') {
                path
            } else if path.starts_with('~') {
                path.replace('~', &std::env::var("HOME").unwrap_or_default())
            } else {
                path
            };
            log::info!("KGP (file): loading {:?}", path);
            self.load_image_from_file(
                img_id, action, format, cursor_row, cursor_col, cols, rows, z_index, path,
            );
            return;
        }

        // Handle t=d (direct/base64 data)
        let decoded = if b64.is_empty() {
            Vec::new()
        } else {
            decode_base64(b64)
        };

        log::debug!("KGP decoded payload: {} bytes", decoded.len());

        let p = PendingImage {
            id: img_id,
            action: Some(action),
            format,
            trans: trans_type,
            width: data_width,
            height: data_height,
            payload: decoded,
            row: cursor_row,
            col: cursor_col,
            rows,
            cols,
            quiet,
            data_width,
            data_height,
            z_index,
        };

        if more {
            self.pending = Some(p);
        } else {
            self.finalize_image(p);
        }
    }

    fn load_image_from_file(
        &mut self,
        img_id: u32,
        action: GraphicsAction,
        format: GraphicsFormat,
        row: usize,
        col: usize,
        cols: u32,
        rows: u32,
        z_index: i32,
        path: String,
    ) {
        // Read file and decode
        match std::fs::read(&path) {
            Ok(file_bytes) => {
                let img = PendingImage {
                    id: img_id,
                    action: Some(action),
                    format: GraphicsFormat::Png, // auto-detect from file
                    trans: TransmissionType::Direct,
                    width: 0,
                    height: 0,
                    payload: file_bytes,
                    row,
                    col,
                    rows,
                    cols,
                    quiet: 0,
                    data_width: 0,
                    data_height: 0,
                    z_index,
                };
                self.finalize_image(img);
            }
            Err(e) => {
                log::warn!("KGP: failed to read file {}: {}", path, e);
            }
        }
    }

    fn finalize_image(&mut self, img: PendingImage) {
        log::info!(
            "KGP finalize: id={}, fmt={:?}, trans={:?}, payload={} bytes, data_w={}, data_h={}",
            img.id,
            img.format,
            img.trans,
            img.payload.len(),
            img.data_width,
            img.data_height
        );

        // Obtener los bytes RGBA según el tipo de transmisión
        let rgba_data = match img.trans {
            TransmissionType::File | TransmissionType::TempFile => {
                // Payload = ruta del archivo (bytes UTF-8)
                let path_str = match std::str::from_utf8(&img.payload) {
                    Ok(s) => s.trim().to_string(),
                    Err(_) => {
                        log::warn!("KGP: ruta de archivo no es UTF-8 válida");
                        return;
                    }
                };
                // Puede venir en base64 si el payload se acumuló codificado
                let path = if path_str.starts_with('/') || path_str.starts_with('~') {
                    path_str.replace('~', &std::env::var("HOME").unwrap_or_default())
                } else {
                    // Intentar decodificar base64
                    let decoded_bytes = decode_base64(&path_str);
                    match std::str::from_utf8(&decoded_bytes) {
                        Ok(s) => s.trim().to_string(),
                        Err(_) => path_str,
                    }
                };
                log::info!("KGP (file): cargando {:?}", path);
                match std::fs::read(&path) {
                    Ok(file_bytes) => {
                        let file_img = PendingImage {
                            payload: file_bytes,
                            format: GraphicsFormat::Png, // image crate auto-detecta
                            ..img
                        };
                        decode_to_rgba(&file_img)
                    }
                    Err(e) => {
                        log::warn!("KGP: no se pudo leer archivo {}: {}", path, e);
                        return;
                    }
                }
            }
            TransmissionType::SharedMem => {
                // TODO: soporte shm — por ahora ignorar
                log::debug!("KGP: SharedMem no soportado");
                return;
            }
            TransmissionType::Direct => decode_to_rgba(&img),
        };

        let rgba_data = match rgba_data {
            Some(d) => d,
            None => {
                log::warn!("KGP: no se pudo decodificar imagen id={}", img.id);
                return;
            }
        };
        let (rgba, img_w, img_h) = rgba_data;

        // Upload to OpenGL
        let texture_id = unsafe { upload_rgba_texture(&rgba, img_w, img_h) };
        if texture_id == 0 {
            return;
        }

        // Remove old placement with same id
        self.placements.retain(|p| {
            if p.image_id == img.id {
                unsafe {
                    gl::DeleteTextures(1, &p.texture_id);
                }
                false
            } else {
                true
            }
        });

        self.placements.push(ImagePlacement {
            image_id: img.id,
            row: img.row,
            col: img.col,
            rows: img.rows,
            cols: img.cols,
            texture_id,
            img_width: img_w,
            img_height: img_h,
            z_index: img.z_index,
        });

        log::info!(
            "KGP: imagen id={} subida {}×{} px en ({},{})",
            img.id,
            img_w,
            img_h,
            img.col,
            img.row
        );
    }

    pub fn clear(&mut self) {
        for p in &self.placements {
            unsafe {
                gl::DeleteTextures(1, &p.texture_id);
            }
        }
        self.placements.clear();
    }
}

/// Decode base64 (inline, sin crate extra — sólo caracteres A-Za-z0-9+/=)
fn decode_base64(s: &str) -> Vec<u8> {
    const TABLE: [u8; 128] = {
        let mut t = [0xFFu8; 128];
        let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut i = 0usize;
        while i < 64 {
            t[chars[i] as usize] = i as u8;
            i += 1;
        }
        t
    };

    let bytes = s.as_bytes();
    let n = bytes.len();
    let mut out = Vec::with_capacity(n / 4 * 3);
    let mut i = 0;
    while i + 3 < n {
        let b0 = bytes[i];
        let b1 = bytes[i + 1];
        let b2 = bytes[i + 2];
        let b3 = bytes[i + 3];
        if b0 == b'=' {
            break;
        }
        let v0 = TABLE.get(b0 as usize).copied().unwrap_or(0xFF);
        let v1 = TABLE.get(b1 as usize).copied().unwrap_or(0xFF);
        let v2 = TABLE.get(b2 as usize).copied().unwrap_or(0xFF);
        let v3 = TABLE.get(b3 as usize).copied().unwrap_or(0xFF);
        if v0 == 0xFF || v1 == 0xFF {
            break;
        }
        let combined = ((v0 as u32) << 18) | ((v1 as u32) << 12) | ((v2 as u32) << 6) | (v3 as u32);
        out.push((combined >> 16) as u8);
        if b2 != b'=' {
            out.push((combined >> 8) as u8);
        }
        if b3 != b'=' {
            out.push(combined as u8);
        }
        i += 4;
    }
    out
}

/// Decode payload to RGBA bytes + (width, height)
fn decode_to_rgba(img: &PendingImage) -> Option<(Vec<u8>, u32, u32)> {
    if img.payload.is_empty() {
        return None;
    }

    match img.format {
        GraphicsFormat::Png => {
            // Decode via image crate - it auto-detects dimensions from PNG
            let dyn_img = image::load_from_memory(&img.payload).ok()?;
            let (w, h) = dyn_img.dimensions();
            log::debug!("KGP decode PNG: {}x{}", w, h);
            let rgba = dyn_img.to_rgba8().into_raw();
            Some((rgba, w, h))
        }
        GraphicsFormat::Rgb => {
            // Use data_width/data_height (s and v parameters)
            let w = if img.data_width > 0 {
                img.data_width
            } else {
                img.width
            };
            let h = if img.data_height > 0 {
                img.data_height
            } else {
                img.height
            };

            if w == 0 || h == 0 {
                log::warn!("KGP RGB: zero dimensions w={}, h={}", w, h);
                return None;
            }

            let expected = (w * h * 3) as usize;
            if img.payload.len() < expected {
                log::warn!(
                    "KGP RGB: payload too small: {} < {}",
                    img.payload.len(),
                    expected
                );
                return None;
            }

            log::debug!("KGP decode RGB: {}x{}", w, h);
            let mut rgba = Vec::with_capacity((w * h * 4) as usize);
            for chunk in img.payload.chunks(3) {
                rgba.extend_from_slice(chunk);
                rgba.push(255);
            }
            Some((rgba, w, h))
        }
        GraphicsFormat::Rgba => {
            // Use data_width/data_height (s and v parameters)
            let w = if img.data_width > 0 {
                img.data_width
            } else {
                img.width
            };
            let h = if img.data_height > 0 {
                img.data_height
            } else {
                img.height
            };

            if w == 0 || h == 0 {
                log::warn!("KGP RGBA: zero dimensions w={}, h={}", w, h);
                return None;
            }

            let expected = (w * h * 4) as usize;
            if img.payload.len() < expected {
                log::warn!(
                    "KGP RGBA: payload too small: {} < {}",
                    img.payload.len(),
                    expected
                );
                return None;
            }

            log::debug!("KGP decode RGBA: {}x{}", w, h);
            Some((img.payload.clone(), w, h))
        }
    }
}

/// Upload RGBA data as GL_TEXTURE_2D, return texture ID (0 on error)
unsafe fn upload_rgba_texture(rgba: &[u8], w: u32, h: u32) -> u32 {
    let mut tex: u32 = 0;
    gl::GenTextures(1, &mut tex);
    gl::BindTexture(gl::TEXTURE_2D, tex);
    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
    gl::PixelStorei(gl::UNPACK_ALIGNMENT, 4);
    gl::TexImage2D(
        gl::TEXTURE_2D,
        0,
        gl::RGBA8 as i32,
        w as i32,
        h as i32,
        0,
        gl::RGBA,
        gl::UNSIGNED_BYTE,
        rgba.as_ptr() as *const _,
    );
    gl::BindTexture(gl::TEXTURE_2D, 0);
    tex
}
