pub mod input;
pub mod wayland;
pub mod x11;

pub use input::InputHandler;
pub use wayland::WaylandWindow;
pub use x11::X11Window;

#[derive(Clone, Debug)]
pub enum WindowEvent {
    Close,
    Resize { width: u32, height: u32 },
    KeyPress { keycode: u32, modifiers: u32 },
    KeyRelease { keycode: u32, modifiers: u32 },
    MouseMotion { x: i32, y: i32 },
    MouseButton { button: u32, pressed: bool },
    Focus { gained: bool },
}

pub trait WindowBackend: Send {
    fn create_window(&mut self, title: &str, width: u32, height: u32) -> anyhow::Result<()>;
    fn poll_events(&mut self) -> Option<WindowEvent>;
    fn swap_buffers(&mut self);
    fn set_title(&mut self, title: &str);
    fn resize(&mut self, width: u32, height: u32);
    fn should_close(&self) -> bool;
    fn get_size(&self) -> (u32, u32);
    fn make_current(&mut self);
}
