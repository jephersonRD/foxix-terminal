use crate::terminal::buffer::CellAttributes;
use crate::terminal::cursor::Cursor;
use crate::terminal::graphics::GraphicsManager;
use crate::terminal::ScreenBuffer;
use vte::{Parser, Perform};

pub struct AnsiParser {
    parser: Parser,
    pub screen: ScreenBuffer,
    /// Pantalla alternativa para apps TUI (htop, vim, etc.)
    alt_screen: ScreenBuffer,
    pub cursor: Cursor,
    /// Cursor guardado para pantalla alternativa
    alt_cursor: Cursor,
    tab_stops: Vec<usize>,
    /// true = estamos en la pantalla alternativa
    using_alt: bool,
    /// Región de scroll (top, bottom) — 0-indexed
    scroll_top: usize,
    scroll_bottom: usize,
    /// Respuestas pendientes para enviar al PTY (DA1, DA2, CPR, etc.)
    pub pending_responses: Vec<Vec<u8>>,
    /// Kitty Graphics Protocol — gestiona imágenes APC
    pub graphics: GraphicsManager,
    /// Buffer acumulador para secuencias APC/DCS en curso
    apc_buf: Vec<u8>,
    /// true cuando estamos dentro de un hook APC/DCS
    in_apc: bool,
}

impl AnsiParser {
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            parser: Parser::new(),
            screen: ScreenBuffer::new(rows, cols),
            alt_screen: ScreenBuffer::new(rows, cols),
            cursor: Cursor::new(rows, cols),
            alt_cursor: Cursor::new(rows, cols),
            tab_stops: (0..cols).filter(|&i| i % 8 == 0).collect(),
            using_alt: false,
            scroll_top: 0,
            scroll_bottom: rows.saturating_sub(1),
            pending_responses: Vec::new(),
            graphics: GraphicsManager::new(),
            apc_buf: Vec::new(),
            in_apc: false,
        }
    }

    pub fn parse(&mut self, data: &[u8]) -> bool {
        // ── Pre-procesar APC del Kitty Graphics Protocol ─────────────────────
        // vte 0.14 envía ESC _ (APC, 0x1b 0x5f) al estado SosPmApcString
        // que NO llama hook() — simplemente descarta el contenido.
        // Solución: extraer manualmente ESC_G...ESC\\ antes de pasar a vte.
        let (remainder, remaining_data) = self.extract_apc_sequences(data);

        // Pasar solo el stream limpio (sin APC) a vte
        if !remaining_data.is_empty() {
            let mut parser = std::mem::replace(&mut self.parser, Parser::new());
            parser.advance(self, &remaining_data);
            self.parser = parser;
        }
        true
    }

    /// Extrae y procesa secuencias APC Kitty del buffer de datos.
    /// Retorna los bytes sin APC para pasar a vte.
    fn extract_apc_sequences(&mut self, data: &[u8]) -> ((), Vec<u8>) {
        let mut out = Vec::with_capacity(data.len());
        let mut i = 0;
        let n = data.len();

        while i < n {
            // Buscar inicio de APC: ESC _ (0x1B 0x5F)
            if i + 1 < n && data[i] == 0x1B && data[i + 1] == 0x5F {
                // ESC _ encontrado — ver si el byte siguiente es 'G' (0x47)
                // sin consumir el header del APC para el caso general
                let apc_start = i + 2; // primer byte después de ESC_

                // Buscar el terminador: ESC \ (0x1B 0x5C) o BEL (0x07)
                let mut j = apc_start;
                let mut found_end = false;
                while j < n {
                    if (data[j] == 0x1B && j + 1 < n && data[j + 1] == 0x5C) {
                        // ESC \ encontrado
                        let payload = &data[apc_start..j];
                        self.dispatch_apc(payload);
                        i = j + 2; // saltar todo incluyendo ESC\
                        found_end = true;
                        break;
                    } else if data[j] == 0x07 {
                        // BEL terminado
                        let payload = &data[apc_start..j];
                        self.dispatch_apc(payload);
                        i = j + 1;
                        found_end = true;
                        break;
                    }
                    j += 1;
                }
                if !found_end {
                    // APC incompleto — acumular en buffer para el próximo parse()
                    // Por ahora descartamos (chunk final muy raro)
                    i = n;
                }
            } else {
                out.push(data[i]);
                i += 1;
            }
        }
        ((), out)
    }

    /// Despacha el contenido de una secuencia APC al GraphicsManager si es KGP.
    fn dispatch_apc(&mut self, payload: &[u8]) {
        // KGP puede venir con o sin prefijo 'G'
        // (vte lo remueve, pero algunas implementaciones lo conservan)
        let data = if !payload.is_empty() && payload[0] == b'G' {
            &payload[1..]
        } else {
            payload
        };

        // Convertir a str y pasar al GraphicsManager
        if let Ok(s) = std::str::from_utf8(data) {
            let row = self.cursor.row();
            let col = self.cursor.col();
            log::debug!("KGP APC dispatch: {}B en ({},{})", s.len(), col, row);
            self.graphics.handle_apc(s, row, col);
        }
    }

    pub fn screen(&self) -> &ScreenBuffer {
        if self.using_alt {
            &self.alt_screen
        } else {
            &self.screen
        }
    }

    pub fn screen_mut(&mut self) -> &mut ScreenBuffer {
        if self.using_alt {
            &mut self.alt_screen
        } else {
            &mut self.screen
        }
    }

    pub fn cursor(&self) -> &Cursor {
        &self.cursor
    }

    pub fn resize(&mut self, rows: usize, cols: usize) {
        // Screen normal: preservar contenido (scroll history, lsd, ls output...)
        self.screen.resize(rows, cols);

        // Alt screen: limpiar siempre — htop/vim/mapscii redibujan todo tras SIGWINCH.
        // Conservar contenido viejo generaría artefactos desalineados.
        self.alt_screen.clear();
        self.alt_screen.resize(rows, cols);

        self.cursor.resize(rows, cols);
        self.alt_cursor.resize(rows, cols);
        self.tab_stops = (0..cols).filter(|&i| i % 8 == 0).collect();
        self.scroll_top = 0;
        self.scroll_bottom = rows.saturating_sub(1);
    }

    /// Limpia la pantalla actualmente visible (normal o alternativa)
    pub fn clear_active_screen(&mut self) {
        self.active_screen_mut().clear_all();
        self.cursor.move_to(0, 0);
        self.scroll_top = 0;
        self.scroll_bottom = self.screen.rows().saturating_sub(1);
    }

    pub fn reset(&mut self) {
        self.screen.clear();
        self.alt_screen.clear();
        self.cursor.reset();
        self.alt_cursor.reset();
        self.using_alt = false;
        let rows = self.screen.rows();
        self.scroll_top = 0;
        self.scroll_bottom = rows.saturating_sub(1);
    }

    fn active_screen_mut(&mut self) -> &mut ScreenBuffer {
        if self.using_alt {
            &mut self.alt_screen
        } else {
            &mut self.screen
        }
    }

    /// Avanza una línea respetando el scroll region.
    fn newline(&mut self) {
        let row = self.cursor.row();
        let st = self.scroll_top;
        let sb = self.scroll_bottom;
        if row >= sb {
            self.active_screen_mut().scroll_region_up(st, sb, 1);
        } else {
            let needs_scroll = self.cursor.move_down();
            if needs_scroll {
                self.active_screen_mut().scroll_up(1);
            }
        }
    }

    fn index_up(&mut self) {
        let row = self.cursor.row();
        let st = self.scroll_top;
        let sb = self.scroll_bottom;
        if row <= st {
            self.active_screen_mut().scroll_region_down(st, sb, 1);
        } else {
            self.cursor.move_up(1);
        }
    }

    fn switch_to_alt(&mut self, save_cursor: bool) {
        if !self.using_alt {
            if save_cursor {
                self.cursor.save_position();
            }
            self.using_alt = true;
            self.alt_screen.clear();
            self.alt_cursor = Cursor::new(self.screen.rows(), self.screen.cols());
        }
    }

    fn switch_to_normal(&mut self, restore_cursor: bool) {
        if self.using_alt {
            self.using_alt = false;
            if restore_cursor {
                self.cursor.restore_position();
            }
        }
    }
}

impl Perform for AnsiParser {
    fn print(&mut self, c: char) {
        use unicode_width::UnicodeWidthChar;
        let width = c.width().unwrap_or(1).max(1);

        let _row = self.cursor.row();
        let col = self.cursor.col();
        let attrs = self.cursor.attributes();
        let max_col = self.cursor.max_cols();

        // Si no cabe en la línea actual, wrap
        if col + width > max_col {
            self.cursor.move_to_col(0);
            self.newline();
        }

        let row = self.cursor.row();
        let col = self.cursor.col();

        self.active_screen_mut()
            .write_char(row, col, c, attrs.clone());

        // Si es doble ancho, marcar la siguiente celda como "continuación" (\0)
        if width == 2 && col + 1 < max_col {
            self.active_screen_mut()
                .write_char(row, col + 1, '\0', attrs);
        }

        for _ in 0..width {
            self.cursor.move_right();
        }
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' | b'\x0B' | b'\x0C' => self.newline(),
            b'\r' => self.cursor.move_to_col(0),
            b'\t' => {
                let col = self.cursor.col();
                if let Some(&next) = self.tab_stops.iter().find(|&&x| x > col) {
                    self.cursor.move_to_col(next);
                } else {
                    self.cursor
                        .move_to_col(self.cursor.max_cols().saturating_sub(1));
                }
            }
            b'\x08' => self.cursor.move_left(),
            b'\x07' => {} // Bell
            _ => log::trace!("execute 0x{:02x}", byte),
        }
    }

    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        let mut iter = params.iter();

        match action {
            // ── Cursor movement ──────────────────────────────────────────────
            'H' | 'f' => {
                let row = iter
                    .next()
                    .map(|p| p[0] as usize)
                    .unwrap_or(1)
                    .saturating_sub(1);
                let col = iter
                    .next()
                    .map(|p| p[0] as usize)
                    .unwrap_or(1)
                    .saturating_sub(1);
                self.cursor.move_to(row, col);
            }
            'A' => {
                let n = iter.next().map(|p| p[0] as usize).unwrap_or(1).max(1);
                self.cursor.move_up(n);
            }
            'B' => {
                let n = iter.next().map(|p| p[0] as usize).unwrap_or(1).max(1);
                for _ in 0..n {
                    self.cursor.move_down();
                }
            }
            'C' => {
                let n = iter.next().map(|p| p[0] as usize).unwrap_or(1).max(1);
                for _ in 0..n {
                    self.cursor.move_right();
                }
            }
            'D' => {
                let n = iter.next().map(|p| p[0] as usize).unwrap_or(1).max(1);
                for _ in 0..n {
                    self.cursor.move_left();
                }
            }
            'E' => {
                let n = iter.next().map(|p| p[0] as usize).unwrap_or(1).max(1);
                for _ in 0..n {
                    self.newline();
                }
                self.cursor.move_to_col(0);
            }
            'F' => {
                let n = iter.next().map(|p| p[0] as usize).unwrap_or(1).max(1);
                self.cursor.move_up(n);
                self.cursor.move_to_col(0);
            }
            'G' => {
                let col = iter
                    .next()
                    .map(|p| p[0] as usize)
                    .unwrap_or(1)
                    .saturating_sub(1);
                self.cursor.move_to_col(col);
            }
            'd' => {
                let row = iter
                    .next()
                    .map(|p| p[0] as usize)
                    .unwrap_or(1)
                    .saturating_sub(1);
                self.cursor.move_to_row(row);
            }

            // ── Erase ────────────────────────────────────────────────────────
            'J' => {
                let mode = iter.next().map(|p| p[0]).unwrap_or(0);
                let (row, col) = (self.cursor.row(), self.cursor.col());
                match mode {
                    0 => self.active_screen_mut().clear_from_cursor(row, col),
                    1 => self.active_screen_mut().clear_to_cursor(row, col),
                    2 | 3 => self.active_screen_mut().clear_all(),
                    _ => {}
                }
            }
            'K' => {
                let mode = iter.next().map(|p| p[0]).unwrap_or(0);
                let (row, col) = (self.cursor.row(), self.cursor.col());
                match mode {
                    0 => self.active_screen_mut().clear_line_from_cursor(row, col),
                    1 => self.active_screen_mut().clear_line_to_cursor(row, col),
                    2 => self.active_screen_mut().clear_line(row),
                    _ => {}
                }
            }
            'X' => {
                let n = iter.next().map(|p| p[0] as usize).unwrap_or(1).max(1);
                let (row, col) = (self.cursor.row(), self.cursor.col());
                let cols = self.active_screen_mut().cols();
                for c in col..(col + n).min(cols) {
                    self.active_screen_mut()
                        .write_char(row, c, ' ', CellAttributes::default());
                }
            }

            // ── Scroll ───────────────────────────────────────────────────────
            'S' => {
                let n = iter.next().map(|p| p[0] as usize).unwrap_or(1).max(1);
                let st = self.scroll_top;
                let sb = self.scroll_bottom;
                self.active_screen_mut().scroll_region_up(st, sb, n);
            }
            'T' => {
                let n = iter.next().map(|p| p[0] as usize).unwrap_or(1).max(1);
                let st = self.scroll_top;
                let sb = self.scroll_bottom;
                self.active_screen_mut().scroll_region_down(st, sb, n);
            }

            // ── Scroll Region (DECSTBM) — CRÍTICO para htop/vim ─────────────
            'r' => {
                let rows = self.screen.rows();
                let top = iter
                    .next()
                    .map(|p| p[0] as usize)
                    .unwrap_or(1)
                    .saturating_sub(1);
                let bot = iter
                    .next()
                    .map(|p| p[0] as usize)
                    .unwrap_or(rows)
                    .saturating_sub(1);
                self.scroll_top = top.min(rows.saturating_sub(1));
                self.scroll_bottom = bot.min(rows.saturating_sub(1));
                self.cursor.move_to(0, 0); // DECSTBM siempre mueve cursor a home
            }

            // ── Line insert/delete ───────────────────────────────────────────
            'L' => {
                let n = iter.next().map(|p| p[0] as usize).unwrap_or(1).max(1);
                let st = self.scroll_top;
                let sb = self.scroll_bottom;
                self.active_screen_mut().scroll_region_down(st, sb, n);
            }
            'M' => {
                let n = iter.next().map(|p| p[0] as usize).unwrap_or(1).max(1);
                let st = self.scroll_top;
                let sb = self.scroll_bottom;
                self.active_screen_mut().scroll_region_up(st, sb, n);
            }

            // ── Delete / Insert chars ────────────────────────────────────────
            'P' => {
                let n = iter.next().map(|p| p[0] as usize).unwrap_or(1).max(1);
                let (row, col) = (self.cursor.row(), self.cursor.col());
                let cols = self.active_screen_mut().cols();
                for c in col..(cols.saturating_sub(n)) {
                    if let Some(cell) = self
                        .active_screen_mut()
                        .cell(row, c + n)
                        .map(|c| (c.c, c.attrs.clone()))
                    {
                        self.active_screen_mut().write_char(row, c, cell.0, cell.1);
                    }
                }
                for c in (cols.saturating_sub(n))..cols {
                    self.active_screen_mut()
                        .write_char(row, c, ' ', CellAttributes::default());
                }
            }
            '@' => {
                // Insert blank chars — shift right
                let n = iter.next().map(|p| p[0] as usize).unwrap_or(1).max(1);
                let (row, col) = (self.cursor.row(), self.cursor.col());
                let cols = self.active_screen_mut().cols();
                for c in (col..cols.saturating_sub(n)).rev() {
                    if let Some(cell) = self
                        .active_screen_mut()
                        .cell(row, c)
                        .map(|c| (c.c, c.attrs.clone()))
                    {
                        self.active_screen_mut()
                            .write_char(row, c + n, cell.0, cell.1);
                    }
                }
                for c in col..(col + n).min(cols) {
                    self.active_screen_mut()
                        .write_char(row, c, ' ', CellAttributes::default());
                }
            }

            // ── SGR — Select Graphic Rendition ───────────────────────────────
            'm' => {
                let mut attrs = self.cursor.attributes();
                if params.is_empty() {
                    attrs = CellAttributes::default();
                } else {
                    let mut piter = params.iter();
                    while let Some(param) = piter.next() {
                        let val = param[0];
                        match val {
                            0 => attrs = CellAttributes::default(),
                            1 => attrs.bold = true,
                            2 => attrs.bold = false, // dim/faint
                            3 => attrs.italic = true,
                            4 => attrs.underline = true,
                            5 => attrs.blink = true,
                            7 => attrs.inverse = true,
                            8 => {} // invisible (ignorar visible, no renderizar)
                            9 => attrs.strikethrough = true,
                            21 => attrs.underline = true,
                            22 => attrs.bold = false,
                            23 => attrs.italic = false,
                            24 => attrs.underline = false,
                            25 => attrs.blink = false,
                            27 => attrs.inverse = false,
                            29 => attrs.strikethrough = false,
                            30..=37 => attrs.fg_color = Some((val - 30) as u8),
                            38 => {
                                if let Some(mode_p) = piter.next() {
                                    match mode_p[0] {
                                        5 => {
                                            if let Some(n_p) = piter.next() {
                                                attrs.fg_color = Some(n_p[0] as u8);
                                                attrs.fg_rgb = None;
                                            }
                                        }
                                        2 => {
                                            if let (Some(rp), Some(gp), Some(bp)) =
                                                (piter.next(), piter.next(), piter.next())
                                            {
                                                attrs.fg_rgb =
                                                    Some([rp[0] as u8, gp[0] as u8, bp[0] as u8]);
                                                attrs.fg_color = None;
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            39 => {
                                attrs.fg_color = None;
                                attrs.fg_rgb = None;
                            }
                            40..=47 => attrs.bg_color = Some((val - 40) as u8),
                            48 => {
                                if let Some(mode_p) = piter.next() {
                                    match mode_p[0] {
                                        5 => {
                                            if let Some(n_p) = piter.next() {
                                                attrs.bg_color = Some(n_p[0] as u8);
                                                attrs.bg_rgb = None;
                                            }
                                        }
                                        2 => {
                                            if let (Some(rp), Some(gp), Some(bp)) =
                                                (piter.next(), piter.next(), piter.next())
                                            {
                                                attrs.bg_rgb =
                                                    Some([rp[0] as u8, gp[0] as u8, bp[0] as u8]);
                                                attrs.bg_color = None;
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            49 => {
                                attrs.bg_color = None;
                                attrs.bg_rgb = None;
                            }
                            90..=97 => attrs.fg_color = Some((val - 90 + 8) as u8),
                            100..=107 => attrs.bg_color = Some((val - 100 + 8) as u8),
                            _ => {}
                        }
                    }
                }
                self.cursor.set_attributes(attrs);
            }

            // ── DEC Private Modes (h=set, l=reset) ──────────────────────────
            'h' | 'l' if intermediates == b"?" => {
                let on = action == 'h';
                for param in params.iter() {
                    match param[0] {
                        1 => {}  // DECCKM — application cursor keys (TODO)
                        6 => {}  // DECOM — origin mode
                        7 => {}  // DECAWM — autowrap (ya manejado en print)
                        12 => {} // Cursor blink
                        25 => self.cursor.set_visible(on),
                        47 | 1047 => {
                            // Pantalla alternativa (sin save/restore cursor)
                            if on {
                                self.switch_to_alt(false);
                            } else {
                                self.switch_to_normal(false);
                            }
                        }
                        1049 => {
                            // Pantalla alternativa CON save/restore cursor — CRÍTICO para htop/vim
                            if on {
                                self.switch_to_alt(true);
                            } else {
                                self.switch_to_normal(true);
                            }
                        }
                        2004 => {}                      // Bracketed paste
                        1000 | 1002 | 1006 | 1015 => {} // Mouse
                        _ => log::trace!("DEC ?{} {}", param[0], if on { "set" } else { "reset" }),
                    }
                }
            }

            // ── Normal set/reset modes ───────────────────────────────────────
            'h' | 'l' if intermediates.is_empty() => {}

            // ── Save / Restore cursor ────────────────────────────────────────
            's' if intermediates.is_empty() => self.cursor.save_position(),
            'u' if intermediates.is_empty() => self.cursor.restore_position(),

            // ── Device Status Report (DSR / CPR) ────────────────────────────
            'n' if intermediates.is_empty() => {
                let code = iter.next().map(|p| p[0]).unwrap_or(0);
                match code {
                    5 => {
                        // DSR: terminal status  → OK (ESC[0n)
                        self.pending_responses.push(b"\x1b[0n".to_vec());
                    }
                    6 => {
                        // CPR: cursor position report → ESC[row;colR
                        let row = self.cursor.row() + 1;
                        let col = self.cursor.col() + 1;
                        let resp = format!("\x1b[{};{}R", row, col);
                        self.pending_responses.push(resp.into_bytes());
                    }
                    _ => {}
                }
            }

            // ── Primary / Secondary Device Attributes ────────────────────────
            // ESC[c  o  ESC[0c → DA1: identificarse como VT100 con AVO
            // ESC[>c o  ESC[>0c → DA2: versión del terminal
            'c' => {
                if intermediates == b">" {
                    // DA2 — Secondary: Foxix se identifica como xterm-compatible
                    // Formato: ESC[>Pp;Pv;PcC  (tipo;versión;ROM)
                    self.pending_responses.push(b"\x1b[>61;20;1c".to_vec());
                } else {
                    // DA1 — Primary: VT100 (1) con AVO (2) y sixel (4)
                    self.pending_responses.push(b"\x1b[?1;2c".to_vec());
                }
            }

            // ── Window operations / misc ─────────────────────────────────────
            't' => {} // xterm window ops — ignorar

            _ => log::trace!(
                "Unhandled CSI {:?} {:?} '{}'",
                params,
                intermediates,
                action
            ),
        }
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], _ignore: bool, byte: u8) {
        match (intermediates, byte) {
            (b"", b'7') => self.cursor.save_position(),
            (b"", b'8') => self.cursor.restore_position(),
            (b"", b'c') => self.reset(),
            (b"", b'D') => self.newline(), // IND — index (scroll down at bottom)
            (b"", b'M') => self.index_up(), // RI — reverse index (scroll up at top)
            (b"", b'E') => {
                // NEL — next line
                self.cursor.move_to_col(0);
                self.newline();
            }
            // DCS, OSC, etc — ignorar silenciosamente
            _ => log::trace!("ESC {:?} 0x{:02x}", intermediates, byte),
        }
    }

    // ── Kitty Graphics Protocol (APC): \x1b_G...\x1b\\ ─────────────────────
    //
    // vte dispara hook() al inicio de un DCS/APC, put() por cada byte del
    // payload, y unhook() al final (STRING TERMINATOR \x1b\\ o BEL).
    // Acumulamos en apc_buf y procesamos en unhook.

    fn hook(&mut self, _params: &vte::Params, _intermediates: &[u8], _ignore: bool, action: char) {
        // 'G' es la letra del Kitty Graphics Protocol en APC (\x1b_G...)
        // vte llama hook con action='G' para APC que empiezan con 'G'
        if action == 'G' {
            self.in_apc = true;
            self.apc_buf.clear();
        } else {
            self.in_apc = false;
        }
    }

    fn put(&mut self, byte: u8) {
        if self.in_apc {
            self.apc_buf.push(byte);
        }
    }

    fn unhook(&mut self) {
        if !self.in_apc || self.apc_buf.is_empty() {
            self.in_apc = false;
            return;
        }
        self.in_apc = false;

        if let Ok(s) = std::str::from_utf8(&self.apc_buf) {
            log::debug!("APC received: '{}' ({} bytes)", s, s.len());

            // Detect KGP capability query (a=q) -- yazi sends this to probe support
            let is_query = s.starts_with("a=q") || s.contains(",a=q");
            if is_query {
                let img_id = s
                    .split(',')
                    .find(|kv| kv.starts_with("i="))
                    .and_then(|kv| kv[2..].parse::<u32>().ok())
                    .unwrap_or(1);
                let resp = format!("\x1b_Gi={};OK\x1b\\", img_id);
                self.pending_responses.push(resp.into_bytes());
                log::info!("KGP query response: id={}", img_id);
                self.apc_buf.clear();
                return;
            }
            let row = self.cursor.row();
            let col = self.cursor.col();
            log::debug!("Calling handle_apc with row={}, col={}", row, col);
            self.graphics.handle_apc(s, row, col);
        }
        self.apc_buf.clear();
    }
}
