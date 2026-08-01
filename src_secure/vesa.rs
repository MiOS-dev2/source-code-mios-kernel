use core::ptr;
use crate::tamzen_font;   


#[derive(Clone, Copy)]
pub struct Color(pub u32);
impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Self { Self( ((r as u32) << 16) | ((g as u32) << 8) | (b as u32) ) }
    pub const BLACK: Self = Self(0);
    pub const WHITE: Self = Self(0x00FFFFFF);
    pub const RED:   Self = Self::new(255,0,0);
    pub const BLUE:  Self = Self::new(0,0,255);
    pub const GREEN: Self = Self::new(0,255,0);
    pub const LIGHT_GRAY: Self = Self::new(200,200,200);
    pub const TITLE_BLUE: Self = Self::new(0,80,160);
    pub const DESKTOP_BLUE: Self = Self::new(58,110,165);
    pub const YELLOW: Self = Self::new(255,255,0);
}

#[derive(Clone)]
pub struct VesaInfo {
    pub addr: usize,
    pub width: usize,
    pub height: usize,
    pub pitch: usize,
    pub bpp: u8,
}

pub struct VesaDisplay {
    pub fb: *mut u32,
    pub width: usize,
    pub height: usize,
    pitch: usize,
    bytes_pp: usize,
}

impl VesaDisplay {
    pub unsafe fn from_multiboot(info: &VesaInfo) -> Self {
        let bytes_pp = (info.bpp as usize) / 8;
        let pitch = if info.pitch == 0 || info.pitch < info.width * bytes_pp {
            info.width * bytes_pp
        } else {
            info.pitch
        };
        Self {
            fb: info.addr as *mut u32,
            width: info.width,
            height: info.height,
            pitch,
            bytes_pp,
        }
    }

    pub fn clear(&mut self, color: Color) {
        for y in 0..self.height {
            for x in 0..self.width {
                self.set_pixel(x, y, color.0);
            }
        }
    }

    #[inline]
    pub fn set_pixel(&mut self, x: usize, y: usize, color: u32) {
        if x < self.width && y < self.height {
            let off = y * self.pitch + x * self.bytes_pp;
            unsafe { ptr::write_volatile((self.fb as *mut u8).add(off) as *mut u32, color); }
        }
    }

    pub fn fill_rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: u32) {
        for dy in 0..h {
            for dx in 0..w {
                self.set_pixel(x + dx, y + dy, color);
            }
        }
    }

    pub fn draw_rect_border(&mut self, x: usize, y: usize, w: usize, h: usize, color: u32) {
        for dx in 0..w {
            self.set_pixel(x + dx, y, color);
            self.set_pixel(x + dx, y + h - 1, color);
        }
        for dy in 0..h {
            self.set_pixel(x, y + dy, color);
            self.set_pixel(x + w - 1, y + dy, color);
        }
    }

    pub fn draw_text(&mut self, x: usize, y: usize, text: &str, fg: u32, bg: u32) {
        for (i, ch) in text.chars().enumerate() {
            if ch < ' ' || ch > '~' {
                continue;
            }
            let char_index = (ch as usize) - 0x20;
            let base = char_index * tamzen_font::FONT_HEIGHT;

            for row in 0..tamzen_font::FONT_HEIGHT {
                let byte = tamzen_font::FONT[base + row];
                for col in 0..tamzen_font::FONT_WIDTH {
                    let pixel_x = x + i * tamzen_font::FONT_WIDTH + col;
                    let pixel_y = y + row;
                    if byte & (1 << (7 - col)) != 0 {
                        self.set_pixel(pixel_x, pixel_y, fg);
                    } else {
                        if fg != bg {
                            self.set_pixel(pixel_x, pixel_y, bg);
                        }
                    }
                }
            }
        }
    }

    pub fn draw_number(&mut self, x: usize, y: usize, num: usize, fg: u32, bg: u32) {
        let mut buf = [0u8; 20];
        let mut i = 20;
        if num == 0 { i -= 1; buf[i] = b'0'; }
        else { let mut n = num; while n > 0 { i -= 1; buf[i] = (n % 10) as u8 + b'0'; n /= 10; } }
        let s = core::str::from_utf8(&buf[i..]).unwrap();
        self.draw_text(x, y, s, fg, bg);
    }
}
