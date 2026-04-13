use crate::ui_layout::base::{Layout, Window, WindowId};

pub struct StackLayout;

impl StackLayout {
    pub fn new() -> Self {
        Self
    }
}

impl Default for StackLayout {
    fn default() -> Self {
        Self::new()
    }
}

impl Layout for StackLayout {
    fn name(&self) -> &'static str {
        "stack"
    }

    fn apply(&self, windows: &mut [Window], container_width: f32, container_height: f32) {
        if windows.is_empty() {
            return;
        }

        let n = windows.len();

        for window in windows.iter_mut() {
            window.x = 0.0;
            window.y = 0.0;
            window.width = container_width;
            window.height = container_height;
            window.is_visible = true;
        }

        for (i, window) in windows.iter_mut().enumerate() {
            window.is_visible = i == n - 1;
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

        for (i, window) in windows.iter_mut().enumerate() {
            window.is_visible = i == next_idx;
            window.is_active = i == next_idx;
        }

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

        for (i, window) in windows.iter_mut().enumerate() {
            window.is_visible = i == prev_idx;
            window.is_active = i == prev_idx;
        }

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
