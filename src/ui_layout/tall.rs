use crate::ui_layout::base::{Layout, Window, WindowId};

pub struct TallLayout {
    bias: f32,
}

impl TallLayout {
    pub fn new() -> Self {
        Self { bias: 0.5 }
    }

    pub fn set_bias(&mut self, bias: f32) {
        self.bias = bias.clamp(0.2, 0.8);
    }

    pub fn bias(&self) -> f32 {
        self.bias
    }
}

impl Default for TallLayout {
    fn default() -> Self {
        Self::new()
    }
}

impl Layout for TallLayout {
    fn name(&self) -> &'static str {
        "tall"
    }

    fn apply(&self, windows: &mut [Window], container_width: f32, container_height: f32) {
        if windows.is_empty() {
            return;
        }

        let n = windows.len();

        if n == 1 {
            windows[0].x = 0.0;
            windows[0].y = 0.0;
            windows[0].width = container_width;
            windows[0].height = container_height;
            windows[0].is_visible = true;
            return;
        }

        let first_height = container_height * self.bias;
        let second_height = container_height - first_height;

        windows[0].x = 0.0;
        windows[0].y = 0.0;
        windows[0].width = container_width;
        windows[0].height = first_height;
        windows[0].is_visible = true;

        for (i, window) in windows.iter_mut().enumerate().skip(1) {
            window.x = 0.0;
            window.y = first_height + (i - 1) as f32 * second_height / (n - 1) as f32;
            window.width = container_width;
            window.height = second_height / (n - 1) as f32;
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
        if current_idx < windows.len() - 1 {
            windows.swap(current_idx, current_idx + 1);
        }
    }

    fn swap_prev(&self, windows: &mut [Window], current: WindowId) {
        let current_idx = match windows.iter().position(|w| w.id == current) {
            Some(idx) => idx,
            None => return,
        };
        if current_idx > 0 {
            windows.swap(current_idx, current_idx - 1);
        }
    }
}
