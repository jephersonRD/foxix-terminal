use super::buffer::CellAttributes;

#[derive(Clone, Debug)]
pub struct Cursor {
    row: usize,
    col: usize,
    visible: bool,
    blinking: bool,
    origin: bool,
    attributes: CellAttributes,
    saved_row: usize,
    saved_col: usize,
    max_rows: usize,
    max_cols: usize,
}

impl Cursor {
    pub fn new(max_rows: usize, max_cols: usize) -> Self {
        Self {
            row: 0,
            col: 0,
            visible: true,
            blinking: false,
            origin: false,
            attributes: CellAttributes::default(),
            saved_row: 0,
            saved_col: 0,
            max_rows,
            max_cols,
        }
    }

    pub fn row(&self) -> usize {
        self.row
    }

    pub fn col(&self) -> usize {
        self.col
    }

    pub fn visible(&self) -> bool {
        self.visible
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    pub fn blinking(&self) -> bool {
        self.blinking
    }

    pub fn set_blinking(&mut self, blinking: bool) {
        self.blinking = blinking;
    }

    pub fn attributes(&self) -> CellAttributes {
        self.attributes.clone()
    }

    pub fn set_attributes(&mut self, attrs: CellAttributes) {
        self.attributes = attrs;
    }

    pub fn move_to(&mut self, row: usize, col: usize) {
        self.row = row.min(self.max_rows.saturating_sub(1));
        self.col = col.min(self.max_cols.saturating_sub(1));
    }

    pub fn move_to_row(&mut self, row: usize) {
        self.row = row.min(self.max_rows.saturating_sub(1));
    }

    pub fn move_to_col(&mut self, col: usize) {
        self.col = col.min(self.max_cols.saturating_sub(1));
    }

    pub fn move_up(&mut self, n: usize) {
        if self.row >= n {
            self.row -= n;
        } else {
            self.row = 0;
        }
    }

    /// Mueve el cursor una fila hacia abajo.
    /// Retorna `true` si el cursor ya estaba en la última fila (se necesita scroll).
    pub fn move_down(&mut self) -> bool {
        if self.row < self.max_rows.saturating_sub(1) {
            self.row += 1;
            false
        } else {
            true // cursor en el límite → necesita scroll
        }
    }

    pub fn max_rows(&self) -> usize {
        self.max_rows
    }

    pub fn max_cols(&self) -> usize {
        self.max_cols
    }

    pub fn move_left(&mut self) {
        if self.col > 0 {
            self.col -= 1;
        }
    }

    pub fn move_right(&mut self) {
        if self.col < self.max_cols - 1 {
            self.col += 1;
        }
    }

    pub fn save_position(&mut self) {
        self.saved_row = self.row;
        self.saved_col = self.col;
    }

    pub fn restore_position(&mut self) {
        self.row = self.saved_row.min(self.max_rows.saturating_sub(1));
        self.col = self.saved_col.min(self.max_cols.saturating_sub(1));
    }

    pub fn resize(&mut self, rows: usize, cols: usize) {
        self.max_rows = rows;
        self.max_cols = cols;
        self.row = self.row.min(rows.saturating_sub(1));
        self.col = self.col.min(cols.saturating_sub(1));
    }

    pub fn reset(&mut self) {
        self.row = 0;
        self.col = 0;
        self.visible = true;
        self.blinking = false;
        self.origin = false;
        self.attributes = CellAttributes::default();
    }

    pub fn newline(&mut self) -> bool {
        self.col = 0;
        self.move_down()
    }
}
