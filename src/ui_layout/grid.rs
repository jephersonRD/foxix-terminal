use crate::ui_layout::base::{Layout, Window, WindowId};

pub struct GridLayout {
    cols: usize,
}

impl GridLayout {
    pub fn new() -> Self {
        Self { cols: 2 }
    }

    pub fn set_cols(&mut self, cols: usize) {
        self.cols = cols.max(1);
    }
}

impl Default for GridLayout {
    fn default() -> Self {
        Self::new()
    }
}

impl Layout for GridLayout {
    fn name(&self) -> &'static str {
        "grid"
    }

    fn apply(&self, windows: &mut [Window], container_width: f32, container_height: f32) {
        if windows.is_empty() {
            return;
        }

        let n = windows.len();
        let cols = self.cols.min(n);
        let rows = (n + cols - 1) / cols;

        let cell_width = container_width / cols as f32;
        let cell_height = container_height / rows as f32;

        for (i, window) in windows.iter_mut().enumerate() {
            let col = i % cols;
            let row = i / cols;
            window.x = col as f32 * cell_width;
            window.y = row as f32 * cell_height;
            window.width = cell_width;
            window.height = cell_height;
            window.is_visible = true;
        }
    }

    fn add_window(&self, windows: &mut Vec<Window>) -> Option<WindowId> {
        None
    }

    fn remove_window(&self, _windows: &mut Vec<Window>, _window_id: WindowId) {}

    fn focus_next(&self, windows: &mut [Window], current: WindowId) -> Option<WindowId> {
        if windows.is_empty() {
            return None;
        }
        let current_idx = windows.iter().position(|w| w.id == current)?;
        let next_idx = (current_idx + 1) % windows.len();
        windows[next_idx].is_active = true;
        windows[current_idx].is_active = false;
        Some(windows[next_idx].id)
    }

    fn focus_prev(&self, windows: &mut [Window], current: WindowId) -> Option<WindowId> {
        if windows.is_empty() {
            return None;
        }
        let current_idx = windows.iter().position(|w| w.id == current)?;
        let prev_idx = if current_idx == 0 {
            windows.len() - 1
        } else {
            current_idx - 1
        };
        windows[prev_idx].is_active = true;
        windows[current_idx].is_active = false;
        Some(windows[prev_idx].id)
    }

    fn swap_next(&self, windows: &mut [Window], current: WindowId) {
        let current_idx = match windows.iter().position(|w| w.id == current) {
            Some(idx) => idx,
            None => return,
        };
        let next_idx = (current_idx + 1) % windows.len();
        windows.swap(current_idx, next_idx);
    }

    fn swap_prev(&self, windows: &mut [Window], current: WindowId) {
        let current_idx = match windows.iter().position(|w| w.id == current) {
            Some(idx) => idx,
            None => return,
        };
        let prev_idx = if current_idx == 0 {
            windows.len() - 1
        } else {
            current_idx - 1
        };
        windows.swap(current_idx, prev_idx);
    }
}
