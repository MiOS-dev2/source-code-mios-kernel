// src/vga.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black      = 0,
    Blue       = 1,
    Green      = 2,
    Cyan       = 3,
    Red        = 4,
    Magenta    = 5,
    Brown      = 6,
    LightGray  = 7,
    DarkGray   = 8,
    LightBlue  = 9,
    LightGreen = 10,
    LightCyan  = 11,
    LightRed   = 12,
    Pink       = 13,
    Yellow     = 14,
    White      = 15,
}

pub struct VGA {
    buffer: *mut u16,
    column: usize,
    row: usize,
    color: u8,
    pub cursor_pos: *mut u16,
}

impl VGA {
    pub const VGA_BUFFER: usize = 0xB8000;
    const WIDTH: usize = 80;
    const HEIGHT: usize = 25;
    
    pub const fn new() -> Self {
        Self {
            buffer: Self::VGA_BUFFER as *mut u16,
            column: 0,
            row: 0,
            color: 0x0A,
            cursor_pos: Self::VGA_BUFFER as *mut u16,
        }
    }
    
    pub fn set_color(&mut self, fg: Color, bg: Color) {
        self.color = ((bg as u8) << 4) | (fg as u8);
    }
    
    pub fn clear(&mut self) {
        for y in 0..Self::HEIGHT {
            for x in 0..Self::WIDTH {
                unsafe {
                    let offset = y * Self::WIDTH + x;
                    *self.buffer.add(offset) = (self.color as u16) << 8 | 0x20;
                }
            }
        }
        self.column = 0;
        self.row = 0;
        self.cursor_pos = self.buffer;
    }
    
    fn scroll(&mut self) {
        for y in 1..Self::HEIGHT {
            for x in 0..Self::WIDTH {
                unsafe {
                    let src = y * Self::WIDTH + x;
                    let dst = (y - 1) * Self::WIDTH + x;
                    *self.buffer.add(dst) = *self.buffer.add(src);
                }
            }
        }
        
        let last_row = (Self::HEIGHT - 1) * Self::WIDTH;
        for x in 0..Self::WIDTH {
            unsafe {
                *self.buffer.add(last_row + x) = (self.color as u16) << 8 | 0x20;
            }
        }
    }
    
    pub fn put_char(&mut self, c: char) {
        if c == '\n' {
            self.column = 0;
            self.row += 1;
        } else if c == '\r' {
            self.column = 0;
        } else {
            if self.column >= Self::WIDTH {
                self.column = 0;
                self.row += 1;
            }
            
            if self.row >= Self::HEIGHT {
                self.scroll();
                self.row = Self::HEIGHT - 1;
            }
            
            let offset = self.row * Self::WIDTH + self.column;
            unsafe {
                *self.buffer.add(offset) = (self.color as u16) << 8 | (c as u16);
            }
            self.column += 1;
        }
        self.cursor_pos = unsafe { self.buffer.add(self.row * Self::WIDTH + self.column) };
    }
    
    pub fn write_string(&mut self, s: &str) {
        for c in s.chars() {
            self.put_char(c);
        }
    }
    
    pub fn write_number(&mut self, mut num: usize) {
        if num == 0 {
            self.put_char('0');
            return;
        }
        
        let mut buf = [0u8; 20];
        let mut i = 20;
        
        while num > 0 {
            i -= 1;
            buf[i] = (num % 10) as u8 + b'0';
            num /= 10;
        }
        
        for j in i..20 {
            self.put_char(buf[j] as char);
        }
    }
    
    pub fn write_hex(&mut self, num: usize) {
        self.write_string("0x");
        let mut wrote = false;
        for i in (0..16).rev() {
            let digit = ((num >> (i * 4)) & 0xF) as u8;
            if !wrote && digit == 0 && i != 0 {
                continue;
            }
            wrote = true;
            let c = match digit {
                0..=9 => (b'0' + digit) as char,
                10..=15 => (b'A' + (digit - 10)) as char,
                _ => '?',
            };
            self.put_char(c);
        }
        if !wrote {
            self.put_char('0');
        }
    }
    
    pub fn get_cursor_pos(&self) -> *mut u16 {
        self.cursor_pos
    }
    
    pub fn set_cursor_pos(&mut self, pos: *mut u16) {
        self.cursor_pos = pos;
        let offset = (pos as usize - Self::VGA_BUFFER) / 2;
        self.column = offset % Self::WIDTH;
        self.row = offset / Self::WIDTH;
    }
    
    pub fn backspace(&mut self) {
        if self.column > 0 {
            self.column -= 1;
        } else if self.row > 0 {
            self.row -= 1;
            self.column = Self::WIDTH - 1;
        } else {
            return;
        }
        
        let offset = self.row * Self::WIDTH + self.column;
        unsafe {
            *self.buffer.add(offset) = (self.color as u16) << 8 | 0x20;
        }
        self.cursor_pos = unsafe { self.buffer.add(offset) };
    }
}
pub fn outb(port: u16, val: u8) { unsafe { core::arch::asm!("out dx, al", in("dx") port, in("al") val); } }
pub fn inb(port: u16) -> u8 { let v: u8; unsafe { core::arch::asm!("in al, dx", in("dx") port, out("al") v); } v }
