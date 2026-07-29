use crate::vesa::VesaDisplay;

pub trait Console {
    fn write_string(&mut self, s: &str);
    fn write_number(&mut self, n: usize);
    fn put_char(&mut self, c: char);
    fn clear(&mut self);
    fn set_color(&mut self, fg: u32, bg: u32);
}

pub struct GfxConsole<'a> {
    pub disp: &'a mut VesaDisplay,
    pub x: usize,
    pub y: usize,
    pub fg: u32,
    pub bg: u32,
    pub rows: usize,
    pub scroll_top: usize,
}

impl<'a> GfxConsole<'a> {
    pub fn new(disp: &'a mut VesaDisplay, x: usize, y: usize) -> Self {
        let rows = disp.height / 16;
        Self { disp, x, y, fg: 0xFFFFFFFF, bg: 0xFF000000, rows, scroll_top: 0 }
    }

    fn newline(&mut self) {
        self.x = 10;
        self.y += 16;
        if self.y + 16 > self.disp.height {
            let line_bytes = self.disp.width * 4;
            let fb = self.disp.fb as *mut u8;
            unsafe {
                core::ptr::copy(
                    fb.add(self.scroll_top * 16 * line_bytes + 16 * line_bytes),
                    fb.add(self.scroll_top * 16 * line_bytes),
                    (self.rows - self.scroll_top - 1) * 16 * line_bytes
                );
                for y in (self.rows - 1)..self.rows {
                    for x in 0..self.disp.width {
                        self.disp.set_pixel(x, y * 16, self.bg);
                    }
                }
            }
            self.y = (self.rows - 1) * 16;
        }
    }

    pub fn clear_line(&mut self, y: usize) {
        self.disp.fill_rect(0, y, self.disp.width, 16, self.bg);
    }
}

impl<'a> Console for GfxConsole<'a> {
    fn write_string(&mut self, s: &str) {
        for c in s.chars() {
            if c == '\n' { self.newline(); } else { self.put_char(c); }
        }
    }

    fn write_number(&mut self, n: usize) {
        let mut buf = [0u8; 20];
        let mut i = 20;
        if n == 0 { buf[i-1] = b'0'; i -= 1; }
        else { let mut num = n; while num > 0 { i -= 1; buf[i] = (num % 10) as u8 + b'0'; num /= 10; } }
        let s = core::str::from_utf8(&buf[i..]).unwrap();
        self.write_string(s);
    }

    fn put_char(&mut self, c: char) {
        if self.x + 8 > self.disp.width { self.newline(); }
        let mut buf = [0u8; 4];
        let s = c.encode_utf8(&mut buf);
        self.disp.draw_text(self.x, self.y, s, self.fg, self.bg);
        self.x += 8;
    }

    fn clear(&mut self) {
        self.disp.clear(crate::vesa::Color(self.bg));
        self.x = 10; self.y = 10;
    }

    fn set_color(&mut self, fg: u32, bg: u32) { self.fg = fg; self.bg = bg; }
}
