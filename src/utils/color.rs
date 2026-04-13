#[derive(Clone, Copy, Debug)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 && hex.len() != 8 {
            return None;
        }

        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        let a = if hex.len() == 8 {
            u8::from_str_radix(&hex[6..8], 16).ok()?
        } else {
            255
        };

        Some(Self { r, g, b, a })
    }

    pub fn to_rgba(&self) -> [f32; 4] {
        [
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
            self.a as f32 / 255.0,
        ]
    }

    pub fn to_rgb(&self) -> [f32; 3] {
        [
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
        ]
    }
}

impl Default for Color {
    fn default() -> Self {
        Self {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        }
    }
}

pub struct ColorPalette;

impl ColorPalette {
    pub const ANSI_COLORS: [Color; 16] = [
        Color {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        },
        Color {
            r: 205,
            g: 0,
            b: 0,
            a: 255,
        },
        Color {
            r: 0,
            g: 205,
            b: 0,
            a: 255,
        },
        Color {
            r: 205,
            g: 205,
            b: 0,
            a: 255,
        },
        Color {
            r: 0,
            g: 0,
            b: 238,
            a: 255,
        },
        Color {
            r: 205,
            g: 0,
            b: 205,
            a: 255,
        },
        Color {
            r: 0,
            g: 205,
            b: 205,
            a: 255,
        },
        Color {
            r: 229,
            g: 229,
            b: 229,
            a: 255,
        },
        Color {
            r: 127,
            g: 127,
            b: 127,
            a: 255,
        },
        Color {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        },
        Color {
            r: 0,
            g: 255,
            b: 0,
            a: 255,
        },
        Color {
            r: 255,
            g: 255,
            b: 0,
            a: 255,
        },
        Color {
            r: 0,
            g: 0,
            b: 255,
            a: 255,
        },
        Color {
            r: 255,
            g: 0,
            b: 255,
            a: 255,
        },
        Color {
            r: 0,
            g: 255,
            b: 255,
            a: 255,
        },
        Color {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        },
    ];

    pub fn ansi_color(index: u8) -> Color {
        Self::ANSI_COLORS[index as usize % 16]
    }
}
