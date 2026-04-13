use crate::terminal::cursor::Cursor;
use std::collections::VecDeque;

#[derive(Clone, Debug)]
pub struct Cell {
    pub c: char,
    pub attrs: CellAttributes,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            c: ' ',
            attrs: CellAttributes::default(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct CellAttributes {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub blink: bool,
    pub inverse: bool,
    pub strikethrough: bool,
    pub fg_color: Option<u8>,
    pub bg_color: Option<u8>,
    pub fg_rgb: Option<[u8; 3]>,
    pub bg_rgb: Option<[u8; 3]>,
}

pub struct ScreenBuffer {
    lines: Vec<Vec<Cell>>,
    scrollback: VecDeque<Vec<Cell>>,
    rows: usize,
    cols: usize,
    max_scrollback: usize,
    dirty: bool,
}

impl ScreenBuffer {
    pub fn new(rows: usize, cols: usize) -> Self {
        let mut buffer = Self {
            lines: Vec::with_capacity(rows),
            scrollback: VecDeque::new(),
            rows,
            cols,
            max_scrollback: 10000,
            dirty: true,
        };
        buffer.init_lines();
        buffer
    }

    fn init_lines(&mut self) {
        self.lines.clear();
        for _ in 0..self.rows {
            self.lines.push(vec![Cell::default(); self.cols]);
        }
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn cell(&self, row: usize, col: usize) -> Option<&Cell> {
        self.lines.get(row).and_then(|line| line.get(col))
    }

    pub fn write_char(&mut self, row: usize, col: usize, c: char, attrs: CellAttributes) {
        if row >= self.rows || col >= self.cols {
            return;
        }

        if let Some(line) = self.lines.get_mut(row) {
            if let Some(cell) = line.get_mut(col) {
                cell.c = c;
                cell.attrs = attrs;
            }
        }
        self.dirty = true;
    }

    pub fn has_changes(&self, _cursor: &Cursor) -> bool {
        self.dirty
    }

    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    pub fn clear_all(&mut self) {
        self.init_lines();
        self.dirty = true;
    }

    pub fn clear_from_cursor(&mut self, row: usize, col: usize) {
        for r in row..self.rows {
            let start = if r == row { col } else { 0 };
            if let Some(line) = self.lines.get_mut(r) {
                for c in line.iter_mut().skip(start) {
                    *c = Cell::default();
                }
            }
        }
        self.dirty = true;
    }

    pub fn clear_to_cursor(&mut self, row: usize, col: usize) {
        for r in 0..=row {
            let end = if r == row { col } else { self.cols };
            if let Some(line) = self.lines.get_mut(r) {
                for c in line.iter_mut().take(end) {
                    *c = Cell::default();
                }
            }
        }
        self.dirty = true;
    }

    pub fn clear_line(&mut self, row: usize) {
        if let Some(line) = self.lines.get_mut(row) {
            for cell in line.iter_mut() {
                *cell = Cell::default();
            }
        }
        self.dirty = true;
    }

    pub fn clear_line_from_cursor(&mut self, row: usize, col: usize) {
        if let Some(line) = self.lines.get_mut(row) {
            for cell in line.iter_mut().skip(col) {
                *cell = Cell::default();
            }
        }
        self.dirty = true;
    }

    pub fn clear_line_to_cursor(&mut self, row: usize, col: usize) {
        if let Some(line) = self.lines.get_mut(row) {
            for cell in line.iter_mut().take(col + 1) {
                *cell = Cell::default();
            }
        }
        self.dirty = true;
    }

    pub fn scroll_up(&mut self, lines: usize) {
        for _ in 0..lines {
            let removed = self.lines.remove(0);
            self.scrollback.push_back(removed);
            if self.scrollback.len() > self.max_scrollback {
                self.scrollback.pop_front();
            }
            self.lines.push(vec![Cell::default(); self.cols]);
        }
        self.dirty = true;
    }

    pub fn scroll_down(&mut self, lines: usize) {
        for _ in 0..lines {
            if let Some(restored) = self.scrollback.pop_back() {
                self.lines.remove(self.lines.len() - 1);
                self.lines.insert(0, restored);
            }
        }
        self.dirty = true;
    }

    /// Scroll solo dentro de la región [top..=bottom] hacia arriba (IND)
    pub fn scroll_region_up(&mut self, top: usize, bottom: usize, n: usize) {
        let top = top.min(self.rows.saturating_sub(1));
        let bottom = bottom.min(self.rows.saturating_sub(1));
        if top >= bottom { return; }
        for _ in 0..n {
            // Guardar la top line en scrollback solo si es la primera fila global
            let removed = self.lines.remove(top);
            if top == 0 {
                self.scrollback.push_back(removed);
                if self.scrollback.len() > self.max_scrollback {
                    self.scrollback.pop_front();
                }
            }
            self.lines.insert(bottom, vec![Cell::default(); self.cols]);
        }
        self.dirty = true;
    }

    /// Scroll solo dentro de la región [top..=bottom] hacia abajo (RI)
    pub fn scroll_region_down(&mut self, top: usize, bottom: usize, n: usize) {
        let top = top.min(self.rows.saturating_sub(1));
        let bottom = bottom.min(self.rows.saturating_sub(1));
        if top >= bottom { return; }
        for _ in 0..n {
            // Eliminar la línea bottom
            if bottom < self.lines.len() {
                self.lines.remove(bottom);
            }
            // Insertar línea en blanco en top
            self.lines.insert(top, vec![Cell::default(); self.cols]);
        }
        self.dirty = true;
    }

    pub fn resize(&mut self, rows: usize, cols: usize) {
        if cols == self.cols && rows == self.rows {
            return;
        }

        // ── REFLOW ALGORITM (Estilo Kitty) ───────────────────────────────────
        // 1. Recolectar todas las líneas (scrollback + visibles) en una sola lista de celdas
        //    pero preservando el concepto de "línea lógica" (unidas por wraps).
        let mut all_cells: Vec<Vec<Cell>> = Vec::new();
        
        // Añadir scrollback
        for line in self.scrollback.drain(..) {
            all_cells.push(line);
        }
        // Añadir líneas visibles
        for line in self.lines.drain(..) {
            all_cells.push(line);
        }

        self.cols = cols;
        self.rows = rows;

        // 2. Redistribuir las celdas en el nuevo ancho
        let mut new_all_lines: Vec<Vec<Cell>> = Vec::new();
        for mut old_line in all_cells {
            // Eliminar espacios al final para un reflow limpio
            while old_line.last().map_or(false, |c| c.c == ' ' && !c.attrs.inverse && c.attrs.bg_color.is_none() && c.attrs.bg_rgb.is_none()) {
                old_line.pop();
            }

            if old_line.is_empty() {
                new_all_lines.push(vec![Cell::default(); cols]);
                continue;
            }

            // Dividir la línea vieja en fragmentos del nuevo ancho 'cols'
            let mut chunks = old_line.chunks(cols);
            while let Some(chunk) = chunks.next() {
                let mut new_line = chunk.to_vec();
                if new_line.len() < cols {
                    new_line.resize(cols, Cell::default());
                }
                new_all_lines.push(new_line);
            }
        }

        // 3. Repartir entre el nuevo scrollback y las nuevas líneas visibles
        if new_all_lines.len() > rows {
            let split_at = new_all_lines.len() - rows;
            for line in new_all_lines.drain(..split_at) {
                self.scrollback.push_back(line);
                if self.scrollback.len() > self.max_scrollback {
                    self.scrollback.pop_front();
                }
            }
        }

        self.lines = new_all_lines;
        // Asegurar que siempre tenemos al menos 'rows' líneas
        while self.lines.len() < rows {
            self.lines.push(vec![Cell::default(); cols]);
        }
        // Si tenemos más de 'rows' por el split (no debería), truncar
        if self.lines.len() > rows {
            self.lines.truncate(rows);
        }

        self.dirty = true;
    }

    pub fn clear(&mut self) {
        self.init_lines();
        self.scrollback.clear();
        self.dirty = true;
    }

    pub fn lines(&self) -> &[Vec<Cell>] {
        &self.lines
    }

    pub fn scrollback(&self) -> &VecDeque<Vec<Cell>> {
        &self.scrollback
    }

    pub fn get_scrollback_line(&self, index: usize) -> Option<&[Cell]> {
        self.scrollback.get(index).map(|v| v.as_slice())
    }

    pub fn scrollback_len(&self) -> usize {
        self.scrollback.len()
    }
}
