
use core::slice;
use crate::tamzen_font;

pub static mut BACKBUFFER: [u32; 800 * 600] = [0; 800 * 600];

#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: u8, pub g: u8, pub b: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self { Self { r, g, b } }
    pub fn to_u32(&self) -> u32 { ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32) }
    pub const RED: Color = Color::rgb(255, 0, 0);
    pub const GREEN: Color = Color::rgb(0, 255, 0);
    pub const BLUE: Color = Color::rgb(0, 0, 255);
    pub const WHITE: Color = Color::rgb(255, 255, 255);
    pub const BLACK: Color = Color::rgb(0, 0, 0);
    pub const YELLOW: Color = Color::rgb(255, 255, 0);
    pub const DARK_GRAY: Color = Color::rgb(60, 60, 60);
    pub const GRAY: Color = Color::rgb(75, 75, 75);
    pub const LIGHT_GRAY: Color = Color::rgb(200, 200, 200);
    pub const TITLE_BLUE: Color = Color::rgb(89, 0, 255);
    pub const DESKTOP_BLUE: Color = Color::rgb(76, 0, 255);
}

pub struct Graphics {
    pub fb: &'static mut [u32],
    pub width: usize,
    pub height: usize,
    pub pitch: usize,
}

impl Graphics {
    pub fn new(addr: u64, width: u32, height: u32, pitch: u32) -> Self {
        let size = (pitch as usize / 4) * height as usize;
        let fb = unsafe { slice::from_raw_parts_mut(addr as *mut u32, size) };
        Self { fb, width: width as usize, height: height as usize, pitch: pitch as usize }
    }

    #[inline]
    pub fn put_pixel(&mut self, x: usize, y: usize, color: u32) {
        if x < self.width && y < self.height {
            unsafe { BACKBUFFER[y * self.width + x] = color; }
        }
    }

    pub fn flush(&mut self) {
        for y in 0..self.height {
            let src = y * self.width;
            let dst = y * (self.pitch / 4);
            let src_slice = unsafe {
                core::slice::from_raw_parts(
                    BACKBUFFER.as_ptr().add(src),
                    self.width,
                )
            };
            self.fb[dst..dst + self.width].copy_from_slice(src_slice);
        }
    }

    pub fn fill_rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: u32) {
        for dy in 0..h { for dx in 0..w { self.put_pixel(x + dx, y + dy, color); } }
    }

    pub fn draw_rect_border(&mut self, x: usize, y: usize, w: usize, h: usize, color: u32) {
        for dx in 0..w { self.put_pixel(x + dx, y, color); self.put_pixel(x + dx, y + h - 1, color); }
        for dy in 0..h { self.put_pixel(x, y + dy, color); self.put_pixel(x + w - 1, y + dy, color); }
    }


    fn is_cyrillic(ch: char) -> bool {
        let code = ch as u32;
        (code >= 0x0400 && code <= 0x04FF) || code == 0x0451 || code == 0x0401
    }

    fn get_cyrillic_index(ch: char) -> Option<usize> {

        match ch {

            'А' => Some(95), 'Б' => Some(96), 'В' => Some(97), 'Г' => Some(98),
            'Д' => Some(99), 'Е' => Some(100), 'Ё' => Some(101), 'Ж' => Some(102),
            'З' => Some(103), 'И' => Some(104), 'Й' => Some(105), 'К' => Some(106),
            'Л' => Some(107), 'М' => Some(108), 'Н' => Some(109), 'О' => Some(110),
            'П' => Some(111), 'Р' => Some(112), 'С' => Some(113), 'Т' => Some(114),
            'У' => Some(115), 'Ф' => Some(116), 'Х' => Some(117), 'Ц' => Some(118),
            'Ч' => Some(119), 'Ш' => Some(120), 'Щ' => Some(121), 'Ъ' => Some(122),
            'Ы' => Some(123), 'Ь' => Some(124), 'Э' => Some(125), 'Ю' => Some(126),
            'Я' => Some(127),

            _ => None
        }
    }


    fn draw_cyrillic_lowercase(&mut self, x: usize, y: usize, ch: char, fg: u32, bg: u32) {
        use crate::tamzen_font::RUSSIAN_LOWERCASE;
        
        let index = match ch {
            'а' => 0, 'б' => 1, 'в' => 2, 'г' => 3, 'д' => 4, 'е' => 5, 'ё' => 6,
            'ж' => 7, 'з' => 8, 'и' => 9, 'й' => 10, 'к' => 11, 'л' => 12, 'м' => 13,
            'н' => 14, 'о' => 15, 'п' => 16, 'р' => 17, 'с' => 18, 'т' => 19,
            'у' => 20, 'ф' => 21, 'х' => 22, 'ц' => 23, 'ч' => 24, 'ш' => 25,
            'щ' => 26, 'ъ' => 27, 'ы' => 28, 'ь' => 29, 'э' => 30, 'ю' => 31,
            'я' => 32,
            _ => return,
        };
        
        let base = index * tamzen_font::FONT_HEIGHT;
        for row in 0..tamzen_font::FONT_HEIGHT {
            let byte = RUSSIAN_LOWERCASE[base + row];
            for col in 0..tamzen_font::FONT_WIDTH {
                let pixel_x = x + col;
                let pixel_y = y + row;
                if byte & (1 << (7 - col)) != 0 {
                    self.put_pixel(pixel_x, pixel_y, fg);
                } else if fg != bg {
                    self.put_pixel(pixel_x, pixel_y, bg);
                }
            }
        }
    }


    fn draw_cyrillic_uppercase(&mut self, x: usize, y: usize, ch: char, fg: u32, bg: u32) {
        if let Some(index) = Self::get_cyrillic_index(ch) {
            let base = index * tamzen_font::FONT_HEIGHT;
            for row in 0..tamzen_font::FONT_HEIGHT {
                let byte = tamzen_font::FONT[base + row];
                for col in 0..tamzen_font::FONT_WIDTH {
                    let pixel_x = x + col;
                    let pixel_y = y + row;
                    if byte & (1 << (7 - col)) != 0 {
                        self.put_pixel(pixel_x, pixel_y, fg);
                    } else if fg != bg {
                        self.put_pixel(pixel_x, pixel_y, bg);
                    }
                }
            }
        }
    }

    pub fn draw_text(&mut self, x: usize, y: usize, text: &str, fg: u32, bg: u32) {
        let mut current_x = x;
        
        for ch in text.chars() {

            if Self::is_cyrillic(ch) {

                let code = ch as u32;
                if code >= 0x0410 && code <= 0x042F {

                    self.draw_cyrillic_uppercase(current_x, y, ch, fg, bg);
                } else if code >= 0x0430 && code <= 0x044F {

                    self.draw_cyrillic_lowercase(current_x, y, ch, fg, bg);
                } else if ch == 'Ё' {
                    self.draw_cyrillic_uppercase(current_x, y, ch, fg, bg);
                } else if ch == 'ё' {
                    self.draw_cyrillic_lowercase(current_x, y, ch, fg, bg);
                } else {

                    current_x += tamzen_font::FONT_WIDTH;
                    continue;
                }
                current_x += tamzen_font::FONT_WIDTH;
            } else if ch >= ' ' && ch <= '~' {

                let char_index = (ch as usize) - 0x20;
                let base = char_index * tamzen_font::FONT_HEIGHT;
                for row in 0..tamzen_font::FONT_HEIGHT {
                    let byte = tamzen_font::FONT[base + row];
                    for col in 0..tamzen_font::FONT_WIDTH {
                        let pixel_x = current_x + col;
                        let pixel_y = y + row;
                        if byte & (1 << (7 - col)) != 0 {
                            self.put_pixel(pixel_x, pixel_y, fg);
                        } else if fg != bg {
                            self.put_pixel(pixel_x, pixel_y, bg);
                        }
                    }
                }
                current_x += tamzen_font::FONT_WIDTH;
            } else {

                current_x += tamzen_font::FONT_WIDTH;
            }
        }
    }

    pub fn clear(&mut self, color: u32) {
        self.fill_rect(0, 0, self.width, self.height, color);
    }
}