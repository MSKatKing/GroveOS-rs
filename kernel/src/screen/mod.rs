use core::fmt::Write;
use crate::screen::font::{PSFFont, KERNEL_FONT};
use crate::UEFIBootInfo;

mod font;

pub struct FramebufferWriter {
    framebuffer: &'static mut [u32],
    width: usize,
    height: usize,

    cursor_x: usize,
    cursor_y: usize,

    curr_font: &'static PSFFont,

    fg_color: u32,
    bg_color: u32,
}

impl FramebufferWriter {
    pub fn clear(&mut self) {
        self.cursor_x = 0;
        self.cursor_y = 0;

        self.framebuffer.fill(self.bg_color);
    }
}

impl From<&UEFIBootInfo> for FramebufferWriter {
    fn from(value: &UEFIBootInfo) -> Self {
        // SAFETY: this is okay because we know the base framebuffer pointer and the framebuffer size
        let framebuffer = unsafe {
            core::slice::from_raw_parts_mut(value.framebuffer, value.framebuffer_size)
        };

        Self {
            framebuffer,
            width: value.framebuffer_width,
            height: value.framebuffer_height,

            cursor_x: 0,
            cursor_y: 0,
            curr_font: &KERNEL_FONT,

            fg_color: 0xFFFFFFFF,
            bg_color: 0x00000000,
        }
    }
}

impl Write for FramebufferWriter {
    fn write_char(&mut self, c: char) -> core::fmt::Result {
        let glyph = self.curr_font.get_char(c);
        let glyph_width = self.curr_font.width as usize;
        let glyph_height = self.curr_font.height as usize;

        let x_offset = self.cursor_x;
        let y_offset = self.cursor_y;

        let bytes_per_row = (self.curr_font.width as usize + 7) / 8;

        for row in 0..glyph_height {
            let row_start = row * bytes_per_row;

            for col in 0..glyph_width {
                let byte_index = row_start + col / 8;
                let bit_index = 7 - (col % 8);

                if byte_index >= glyph.len() {
                    continue;
                }

                let byte = glyph[byte_index];
                let pixel_on = (byte >> bit_index) & 1;

                let x = self.cursor_x + col;
                let y = self.cursor_y + row;

                if x < self.width && y < self.height {
                    let pixel_index = y * self.width + x;
                    self.framebuffer[pixel_index] = if pixel_on != 0 {
                        self.fg_color
                    } else {
                        self.bg_color
                    };
                }
            }
        }

        self.cursor_x += glyph_width;
        if self.cursor_x + glyph_width >= self.width {
            self.cursor_x = 0;
            self.cursor_y += glyph_height;
        }

        Ok(())
    }

    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for c in s.chars() {
            match c {
                '\n' => {
                    self.cursor_x = 0;
                    self.cursor_y += self.curr_font.height as usize;
                }
                _ => self.write_char(c)?
            }
        }

        Ok(())
    }
}