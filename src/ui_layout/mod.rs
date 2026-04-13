pub mod base;
pub mod grid;
pub mod stack;
pub mod tall;
pub mod vertical;

pub use base::{Layout, LayoutManager, Window, WindowId, WindowType};
pub use grid::GridLayout;
pub use stack::StackLayout;
pub use tall::TallLayout;
pub use vertical::VerticalLayout;

pub fn create_layout_manager() -> LayoutManager {
    LayoutManager::new()
}
