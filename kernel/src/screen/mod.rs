use crate::UEFIBootInfo;
use crate::screen::font::{KERNEL_FONT, PSFFont};
use core::fmt::Write;

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
        let framebuffer =
            unsafe { core::slice::from_raw_parts_mut(value.framebuffer, value.framebuffer_size) };

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
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for c in s.chars() {
            match c {
                '\n' => {
                    self.cursor_x = 0;
                    self.cursor_y += self.curr_font.height as usize;
                }
                _ => self.write_char(c)?,
            }
        }

        Ok(())
    }

    fn write_char(&mut self, c: char) -> core::fmt::Result {
        let glyph = self.curr_font.get_char(c);
        let glyph_width = self.curr_font.width as usize;
        let glyph_height = self.curr_font.height as usize;

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

        if self.cursor_y >= self.height {
            self.clear();
            self.cursor_y = 0;
            self.cursor_x = 0;
        }

        Ok(())
    }
}

static mut FRAMEBUFFER_WRITER: Option<FramebufferWriter> = None;

pub fn init_writer(writer: FramebufferWriter) {
    // SAFETY: for now, our os is single-threaded, so using a global writer is fine
    unsafe { FRAMEBUFFER_WRITER = Some(writer) }
}

pub fn framebuffer_writer() -> &'static mut FramebufferWriter {
    // SAFETY: for now, our os is single-threaded, so using a global writer is fine
    unsafe {
        #[allow(static_mut_refs)]
        FRAMEBUFFER_WRITER.as_mut().unwrap()
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        $crate::screen::framebuffer_writer().write_fmt(format_args!($($arg)*)).unwrap();
    });
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($fmt:expr) => ($crate::print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::print!(concat!($fmt, "\n"), $($arg)*));
}
