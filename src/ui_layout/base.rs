use std::collections::HashMap;
use std::fmt;

use crate::ui_layout::grid::GridLayout;
use crate::ui_layout::stack::StackLayout;
use crate::ui_layout::tall::TallLayout;
use crate::ui_layout::vertical::VerticalLayout;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowType {
    Terminal,
    Floating,
}

#[derive(Debug, Clone)]
pub struct Window {
    pub id: WindowId,
    pub window_type: WindowType,
    pub tab_id: usize,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub is_active: bool,
    pub is_visible: bool,
}

impl Window {
    pub fn new(id: WindowId, tab_id: usize) -> Self {
        Self {
            id,
            window_type: WindowType::Terminal,
            tab_id,
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
            is_active: true,
            is_visible: true,
        }
    }
}

pub trait Layout: Send + Sync {
    fn name(&self) -> &'static str;
    fn apply(&self, windows: &mut [Window], container_width: f32, container_height: f32);
    fn add_window(&self, windows: &mut Vec<Window>) -> Option<WindowId>;
    fn remove_window(&self, windows: &mut Vec<Window>, window_id: WindowId);
    fn focus_next(&self, windows: &mut [Window], current: WindowId) -> Option<WindowId>;
    fn focus_prev(&self, windows: &mut [Window], current: WindowId) -> Option<WindowId>;
    fn swap_next(&self, windows: &mut [Window], current: WindowId);
    fn swap_prev(&self, windows: &mut [Window], current: WindowId);
}

pub struct LayoutManager {
    layouts: HashMap<&'static str, Box<dyn Layout>>,
    active_layout: &'static str,
    next_window_id: usize,
}

impl LayoutManager {
    pub fn new() -> Self {
        let mut layouts: HashMap<&'static str, Box<dyn Layout>> = HashMap::new();

        layouts.insert("grid", Box::new(GridLayout::new()));
        layouts.insert("tall", Box::new(TallLayout::new()));
        layouts.insert("vertical", Box::new(VerticalLayout::new()));
        layouts.insert("stack", Box::new(StackLayout::new()));

        Self {
            layouts,
            active_layout: "tall",
            next_window_id: 1,
        }
    }

    pub fn register_layout(&mut self, name: &'static str, layout: Box<dyn Layout>) {
        self.layouts.insert(name, layout);
    }

    pub fn set_active_layout(&mut self, name: &'static str) {
        if self.layouts.contains_key(name) {
            self.active_layout = name;
        }
    }

    pub fn get_active_layout(&self) -> Option<&dyn Layout> {
        self.layouts.get(self.active_layout).map(|l| l.as_ref())
    }

    pub fn get_layout_names(&self) -> Vec<&'static str> {
        self.layouts.keys().copied().collect()
    }

    pub fn next_layout(&mut self) {
        let names: Vec<&'static str> = self.layouts.keys().copied().collect();
        if let Some(idx) = names.iter().position(|&n| n == self.active_layout) {
            let next_idx = (idx + 1) % names.len();
            self.active_layout = names[next_idx];
        }
    }

    pub fn prev_layout(&mut self) {
        let names: Vec<&'static str> = self.layouts.keys().copied().collect();
        if let Some(idx) = names.iter().position(|&n| n == self.active_layout) {
            let prev_idx = if idx == 0 { names.len() - 1 } else { idx - 1 };
            self.active_layout = names[prev_idx];
        }
    }

    pub fn create_window(&mut self, tab_id: usize) -> WindowId {
        let id = WindowId(self.next_window_id);
        self.next_window_id += 1;
        id
    }

    pub fn apply_layout(&self, windows: &mut [Window], width: f32, height: f32) {
        if let Some(layout) = self.get_active_layout() {
            layout.apply(windows, width, height);
        }
    }
}

impl Default for LayoutManager {
    fn default() -> Self {
        Self::new()
    }
}
