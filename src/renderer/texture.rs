use anyhow::Result;

pub struct TextureAtlas {
    width: u32,
    height: u32,
    texture_id: u32,
    next_x: u32,
    next_y: u32,
    row_height: u32,
}

impl TextureAtlas {
    pub fn new(width: u32, height: u32) -> Result<Self> {
        Ok(Self {
            width,
            height,
            texture_id: 0,
            next_x: 0,
            next_y: 0,
            row_height: 0,
        })
    }

    pub fn allocate(&mut self, w: u32, h: u32) -> Option<(u32, u32)> {
        if self.next_x + w > self.width {
            self.next_x = 0;
            self.next_y += self.row_height;
            self.row_height = 0;
        }

        if self.next_y + h > self.height {
            return None;
        }

        let x = self.next_x;
        let y = self.next_y;

        self.next_x += w;
        self.row_height = self.row_height.max(h);

        Some((x, y))
    }

    pub fn reset(&mut self) {
        self.next_x = 0;
        self.next_y = 0;
        self.row_height = 0;
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn texture_id(&self) -> u32 {
        self.texture_id
    }

    pub fn set_texture_id(&mut self, id: u32) {
        self.texture_id = id;
    }

    pub fn upload_data(&mut self, x: u32, y: u32, w: u32, h: u32, data: &[u8]) {
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, self.texture_id);
            gl::TexSubImage2D(
                gl::TEXTURE_2D,
                0,
                x as i32,
                y as i32,
                w as i32,
                h as i32,
                gl::RED,
                gl::UNSIGNED_BYTE,
                data.as_ptr() as *const _,
            );
        }
    }
}
