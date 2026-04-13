use std::collections::VecDeque;
use winit::event::ElementState;
use winit::keyboard::PhysicalKey;

pub struct InputHandler {
    key_buffer: VecDeque<KeyEvent>,
    mouse_x: i32,
    mouse_y: i32,
    mouse_buttons: u32,
    modifiers: u32,
    mouse_reporting: bool,
    has_focus: bool,
}

#[derive(Clone, Debug)]
pub struct KeyEvent {
    pub keycode: u32,
    pub characters: String,
    pub modifiers: u32,
    pub pressed: bool,
}

impl InputHandler {
    pub fn new() -> Self {
        Self {
            key_buffer: VecDeque::new(),
            mouse_x: 0,
            mouse_y: 0,
            mouse_buttons: 0,
            modifiers: 0,
            mouse_reporting: false,
            has_focus: true,
        }
    }

    pub fn set_focus(&mut self, focus: bool) {
        self.has_focus = focus;
    }

    pub fn has_focus(&self) -> bool {
        self.has_focus
    }

    pub fn set_modifiers(&mut self, modifiers: u32) {
        self.modifiers = modifiers;
    }

    pub fn get_modifiers(&self) -> u32 {
        self.modifiers
    }

    pub fn is_super_pressed(&self) -> bool {
        (self.modifiers & 0x08) != 0 // SUPER key
    }

    pub fn handle_key_code(
        &mut self,
        physical_key: PhysicalKey,
        state: ElementState,
    ) -> Option<KeyEvent> {
        // Solo procesar si tenemos foco
        if !self.has_focus {
            return None;
        }

        if state == ElementState::Pressed {
            // En Wayland, algunos eventos necesitan ser capturados aquí
            if let PhysicalKey::Code(key_code) = physical_key {
                return Some(KeyEvent {
                    keycode: key_code as u32,
                    characters: String::new(),
                    modifiers: self.modifiers,
                    pressed: true,
                });
            }
        }
        None
    }

    pub fn handle_char(&mut self, ch: char) -> KeyEvent {
        KeyEvent {
            keycode: 0,
            characters: ch.to_string(),
            modifiers: self.modifiers,
            pressed: true,
        }
    }

    pub fn handle_physical_char(&mut self, ch: char) -> KeyEvent {
        // Método alternativo que fuerza la captura del caracter
        KeyEvent {
            keycode: 0,
            characters: ch.to_string(),
            modifiers: self.modifiers,
            pressed: true,
        }
    }

    pub fn pop_event(&mut self) -> Option<KeyEvent> {
        self.key_buffer.pop_front()
    }

    pub fn clear(&mut self) {
        self.key_buffer.clear();
    }

    pub fn set_mouse_reporting(&mut self, enabled: bool) {
        self.mouse_reporting = enabled;
    }

    pub fn mouse_position(&self) -> (i32, i32) {
        (self.mouse_x, self.mouse_y)
    }
}
