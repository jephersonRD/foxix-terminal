use crate::window::{WindowBackend, WindowEvent};

pub struct X11Window {
    width: u32,
    height: u32,
    should_close: bool,
}

impl X11Window {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            width: 800,
            height: 600,
            should_close: false,
        })
    }
}

impl WindowBackend for X11Window {
    fn create_window(&mut self, _title: &str, width: u32, height: u32) -> anyhow::Result<()> {
        self.width = width;
        self.height = height;
        Ok(())
    }

    fn poll_events(&mut self) -> Option<WindowEvent> {
        None
    }

    fn swap_buffers(&mut self) {}

    fn set_title(&mut self, _title: &str) {}

    fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    fn should_close(&self) -> bool {
        self.should_close
    }

    fn get_size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn make_current(&mut self) {}
}
