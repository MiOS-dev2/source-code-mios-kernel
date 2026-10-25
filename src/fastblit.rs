use crate::graphics::{self, Graphics};

pub fn flush_rect(gfx: &mut Graphics, x: usize, y: usize, w: usize, h: usize) {
    if x >= gfx.width || y >= gfx.height || w == 0 || h == 0 { return; }
    let w = w.min(gfx.width - x);
    let h = h.min(gfx.height - y);
    let stride = gfx.pitch / 4;
    for yy in y..y + h {
        let src = yy * gfx.width + x;
        let dst = yy * stride + x;
        let s = unsafe { core::slice::from_raw_parts((&raw const graphics::BACKBUFFER as *const u32).add(src), w) };
        gfx.fb[dst..dst + w].copy_from_slice(s);
    }
}

#[derive(Clone, Copy)]
pub struct Dirty { pub x0: usize, pub y0: usize, pub x1: usize, pub y1: usize, pub any: bool }

impl Dirty {
    pub const fn new() -> Self { Dirty { x0: 0, y0: 0, x1: 0, y1: 0, any: false } }
    pub fn add(&mut self, x: usize, y: usize, w: usize, h: usize) {
        if w == 0 || h == 0 { return; }
        if !self.any { *self = Dirty { x0: x, y0: y, x1: x + w, y1: y + h, any: true }; return; }
        self.x0 = self.x0.min(x);
        self.y0 = self.y0.min(y);
        self.x1 = self.x1.max(x + w);
        self.y1 = self.y1.max(y + h);
    }
    pub fn flush(&mut self, gfx: &mut Graphics) {
        if self.any { flush_rect(gfx, self.x0, self.y0, self.x1 - self.x0, self.y1 - self.y0); }
        self.any = false;
    }
}
