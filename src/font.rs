use crate::graphics::{self, Graphics};

static UI_MONO: &[u8] = include_bytes!("fonts/ui_mono.bin");

pub const UI_W: usize = 8;
pub const UI_H: usize = 16;

static mut UI_FAST: [u16; 256] = [0xFFFF; 256];
static mut UI_READY: bool = false;

fn rd16(d: &[u8], o: usize) -> u16 { u16::from_le_bytes([d[o], d[o + 1]]) }
fn rd32(d: &[u8], o: usize) -> u32 { u32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]]) }
fn rdi16(d: &[u8], o: usize) -> i16 { i16::from_le_bytes([d[o], d[o + 1]]) }

fn ui_count() -> usize { rd16(UI_MONO, 8) as usize }

fn ui_init() {
    unsafe {
        if UI_READY { return; }
        let n = ui_count();
        for i in 0..n {
            let cp = rd32(UI_MONO, 10 + i * 4);
            if cp < 256 { UI_FAST[cp as usize] = i as u16; }
        }
        UI_READY = true;
    }
}

fn ui_index(cp: u32) -> Option<usize> {
    ui_init();
    if cp < 256 {
        let i = unsafe { UI_FAST[cp as usize] };
        return if i == 0xFFFF { None } else { Some(i as usize) };
    }
    let n = ui_count();
    let (mut lo, mut hi) = (0usize, n);
    while lo < hi {
        let mid = (lo + hi) / 2;
        let c = rd32(UI_MONO, 10 + mid * 4);
        if c == cp { return Some(mid); }
        if c < cp { lo = mid + 1 } else { hi = mid }
    }
    None
}

#[inline]
pub fn blend(bg: u32, fg: u32, a: u32) -> u32 {
    if a >= 255 { return fg; }
    if a == 0 { return bg; }
    let ia = 255 - a;
    let r = (((fg >> 16) & 0xFF) * a + ((bg >> 16) & 0xFF) * ia) / 255;
    let g = (((fg >> 8) & 0xFF) * a + ((bg >> 8) & 0xFF) * ia) / 255;
    let b = ((fg & 0xFF) * a + (bg & 0xFF) * ia) / 255;
    (r << 16) | (g << 8) | b
}

#[inline]
fn get_px(gfx: &Graphics, x: usize, y: usize) -> u32 {
    unsafe { graphics::BACKBUFFER[y * gfx.width + x] }
}

pub fn draw_ui_char(gfx: &mut Graphics, x: usize, y: usize, ch: char, fg: u32, bg: u32) {
    let idx = match ui_index(ch as u32).or_else(|| ui_index('?' as u32)) { Some(i) => i, None => return };
    let base = 10 + ui_count() * 4 + idx * (UI_W * UI_H);
    let opaque = fg != bg;
    for row in 0..UI_H {
        let py = y + row;
        if py >= gfx.height { break; }
        let line = &UI_MONO[base + row * UI_W..base + row * UI_W + UI_W];
        for col in 0..UI_W {
            let px = x + col;
            if px >= gfx.width { break; }
            let a = line[col] as u32;
            if opaque {
                gfx.put_pixel(px, py, blend(bg, fg, a));
            } else if a != 0 {
                let under = get_px(gfx, px, py);
                gfx.put_pixel(px, py, blend(under, fg, a));
            }
        }
    }
}

pub fn draw_ui_text(gfx: &mut Graphics, x: usize, y: usize, text: &str, fg: u32, bg: u32) {
    let mut cx = x;
    for ch in text.chars() {
        if ch >= ' ' { draw_ui_char(gfx, cx, y, ch, fg, bg); }
        cx += UI_W;
    }
}

pub struct WebFont {
    data: &'static [u8],
    pub size: u16,
    pub ascent: i32,
    pub descent: i32,
    count: usize,
    fast: [u16; 256],
}

const GLYPH_REC: usize = 18;

impl WebFont {
    fn load(data: &'static [u8]) -> WebFont {
        let size = rd16(data, 4);
        let ascent = rd16(data, 6) as i32;
        let descent = rd16(data, 8) as i32;
        let count = rd16(data, 10) as usize;
        let mut f = WebFont { data, size, ascent, descent, count, fast: [0xFFFF; 256] };
        for i in 0..count {
            let cp = rd32(data, 12 + i * GLYPH_REC);
            if cp < 256 { f.fast[cp as usize] = i as u16; }
        }
        f
    }
    fn index(&self, cp: u32) -> Option<usize> {
        if cp < 256 { let i = self.fast[cp as usize]; return if i == 0xFFFF { None } else { Some(i as usize) }; }
        let (mut lo, mut hi) = (0usize, self.count);
        while lo < hi {
            let mid = (lo + hi) / 2;
            let c = rd32(self.data, 12 + mid * GLYPH_REC);
            if c == cp { return Some(mid); }
            if c < cp { lo = mid + 1 } else { hi = mid }
        }
        None
    }
    fn glyph(&self, ch: char) -> Option<(i32, i32, i32, usize, usize, usize)> {
        let i = self.index(ch as u32).or_else(|| self.index(map_fallback(ch) as u32))?;
        let o = 12 + i * GLYPH_REC;
        let adv = rdi16(self.data, o + 4) as i32;
        let x0 = rdi16(self.data, o + 6) as i32;
        let y0 = rdi16(self.data, o + 8) as i32;
        let w = rd16(self.data, o + 10) as usize;
        let h = rd16(self.data, o + 12) as usize;
        let off = rd32(self.data, o + 14) as usize;
        Some((adv, x0, y0, w, h, off))
    }
    pub fn char_width(&self, ch: char) -> i32 {
        match self.glyph(ch) { Some(g) => g.0, None => (self.size as i32) / 2 }
    }
    pub fn text_width(&self, s: &str) -> i32 {
        s.chars().map(|c| self.char_width(c)).sum()
    }
    pub fn line_height(&self) -> i32 { self.ascent + self.descent }

    pub fn draw(&self, gfx: &mut Graphics, x: i32, y: i32, s: &str, color: u32, clip: (i32, i32, i32, i32)) -> i32 {
        let blob = 12 + self.count * GLYPH_REC;
        let mut cx = x;
        for ch in s.chars() {
            let (adv, x0, y0, w, h, off) = match self.glyph(ch) { Some(g) => g, None => { cx += self.size as i32 / 2; continue; } };
            if cx > clip.2 { break; }
            let gx = cx + x0;
            let gy = y + y0;
            for r in 0..h as i32 {
                let py = gy + r;
                if py < clip.1 || py >= clip.3 || py < 0 || py as usize >= gfx.height { continue; }
                let row = blob + off + (r as usize) * w;
                for c in 0..w as i32 {
                    let px = gx + c;
                    if px < clip.0 || px >= clip.2 || px < 0 || px as usize >= gfx.width { continue; }
                    let a = self.data[row + c as usize] as u32;
                    if a == 0 { continue; }
                    let under = get_px(gfx, px as usize, py as usize);
                    gfx.put_pixel(px as usize, py as usize, blend(under, color, a));
                }
            }
            cx += adv;
        }
        cx - x
    }
}

fn map_fallback(ch: char) -> char {
    match ch {
        '\u{00A0}' => ' ',
        '\u{2010}' | '\u{2011}' | '\u{2012}' | '\u{2212}' => '-',
        '\u{201A}' => ',',
        '\u{2039}' => '<',
        '\u{203A}' => '>',
        '\u{2032}' => '\'',
        '\u{2033}' => '"',
        '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{FEFF}' => ' ',
        _ => '?',
    }
}

static F11: &[u8] = include_bytes!("fonts/web_11.bin");
static F11B: &[u8] = include_bytes!("fonts/web_11b.bin");
static F13: &[u8] = include_bytes!("fonts/web_13.bin");
static F13B: &[u8] = include_bytes!("fonts/web_13b.bin");
static F15: &[u8] = include_bytes!("fonts/web_15.bin");
static F15B: &[u8] = include_bytes!("fonts/web_15b.bin");
static F18: &[u8] = include_bytes!("fonts/web_18.bin");
static F18B: &[u8] = include_bytes!("fonts/web_18b.bin");
static F22: &[u8] = include_bytes!("fonts/web_22.bin");
static F22B: &[u8] = include_bytes!("fonts/web_22b.bin");
static F28: &[u8] = include_bytes!("fonts/web_28.bin");
static F28B: &[u8] = include_bytes!("fonts/web_28b.bin");
static FMONO: &[u8] = include_bytes!("fonts/web_mono13.bin");

static mut FONTS: Option<alloc::vec::Vec<WebFont>> = None;

fn fonts() -> &'static alloc::vec::Vec<WebFont> {
    unsafe {
        if FONTS.is_none() {
            let list = [F11, F11B, F13, F13B, F15, F15B, F18, F18B, F22, F22B, F28, F28B, FMONO];
            FONTS = Some(list.iter().map(|d| WebFont::load(d)).collect());
        }
        FONTS.as_ref().unwrap()
    }
}

pub fn web_font(px: f32, bold: bool, mono: bool) -> &'static WebFont {
    let f = fonts();
    if mono { return &f[12]; }
    let sizes = [11.0f32, 13.0, 15.0, 18.0, 22.0, 28.0];
    let mut best = 0;
    let mut bd = f32::MAX;
    for (i, s) in sizes.iter().enumerate() {
        let d = if px > *s { (px - *s) * 1.2 } else { *s - px };
        if d < bd { bd = d; best = i; }
    }
    &f[best * 2 + if bold { 1 } else { 0 }]
}
