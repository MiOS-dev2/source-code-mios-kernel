use alloc::string::String;
use alloc::vec::Vec;
use crate::graphics::{self, Graphics};
use crate::font::{blend, web_font};
use super::layout::{Cmd, Field, FieldKind, Page};
use super::image::Image;

type Clip = (i32, i32, i32, i32);

fn isect(a: Clip, b: Clip) -> Clip { (a.0.max(b.0), a.1.max(b.1), a.2.min(b.2), a.3.min(b.3)) }

#[inline]
fn px_get(gfx: &Graphics, x: i32, y: i32) -> u32 { unsafe { graphics::BACKBUFFER[y as usize * gfx.width + x as usize] } }

pub fn fill(gfx: &mut Graphics, x: i32, y: i32, w: i32, h: i32, color: u32, clip: Clip) {
    let a = color >> 24;
    if a == 0 || w <= 0 || h <= 0 { return; }
    let c = isect(clip, (x, y, x + w, y + h));
    let c = isect(c, (0, 0, gfx.width as i32, gfx.height as i32));
    if c.0 >= c.2 || c.1 >= c.3 { return; }
    let rgb = color & 0xFFFFFF;
    let width = gfx.width;
    unsafe {
        for yy in c.1..c.3 {
            let row = &mut graphics::BACKBUFFER[yy as usize * width + c.0 as usize..yy as usize * width + c.2 as usize];
            if a >= 255 { for p in row.iter_mut() { *p = rgb; } } else { for p in row.iter_mut() { *p = blend(*p, rgb, a); } }
        }
    }
}

fn text(gfx: &mut Graphics, x: i32, y: i32, s: &str, px: f32, bold: bool, color: u32, clip: Clip) -> i32 {
    web_font(px, bold, false).draw(gfx, x, y, s, color & 0xFFFFFF, clip)
}

fn frame(gfx: &mut Graphics, x: i32, y: i32, w: i32, h: i32, c: u32, clip: Clip) {
    fill(gfx, x, y, w, 1, c, clip);
    fill(gfx, x, y + h - 1, w, 1, c, clip);
    fill(gfx, x, y, 1, h, c, clip);
    fill(gfx, x + w - 1, y, 1, h, c, clip);
}

pub fn blit(gfx: &mut Graphics, img: &Image, x: i32, y: i32, w: i32, h: i32, clip: Clip) {
    if w <= 0 || h <= 0 || img.w <= 0 || img.h <= 0 { return; }
    let c = isect(isect(clip, (x, y, x + w, y + h)), (0, 0, gfx.width as i32, gfx.height as i32));
    if c.0 >= c.2 || c.1 >= c.3 { return; }
    let width = gfx.width;
    for yy in c.1..c.3 {
        let sy = ((yy - y) as i64 * img.h as i64 / h as i64) as i32;
        let srow = (sy.min(img.h - 1) * img.w) as usize;
        for xx in c.0..c.2 {
            let sx = ((xx - x) as i64 * img.w as i64 / w as i64) as i32;
            let p = img.px[srow + sx.min(img.w - 1) as usize];
            let a = p >> 24;
            if a == 0 { continue; }
            unsafe {
                let d = &mut graphics::BACKBUFFER[yy as usize * width + xx as usize];
                *d = if a >= 255 { p & 0xFFFFFF } else { blend(*d, p & 0xFFFFFF, a) };
            }
        }
    }
}

pub fn widget(gfx: &mut Graphics, f: &Field, x: i32, y: i32, w: i32, h: i32, focused: bool, blink: bool, clip: Clip) {
    let c = isect(clip, (x, y, x + w, y + h));
    match f.kind {
        FieldKind::Text | FieldKind::Password | FieldKind::TextArea => {
            fill(gfx, x, y, w, h, 0xFFFFFFFF, clip);
            frame(gfx, x, y, w, h, if focused { 0xFF3B82F6 } else { 0xFF9A9A9A }, clip);
            if focused { frame(gfx, x + 1, y + 1, w - 2, h - 2, 0xFF9CC0FA, clip); }
            let inner = isect(c, (x + 3, y + 2, x + w - 3, y + h - 2));
            let fnt = web_font(13.0, false, false);
            let shown: String = if f.kind == FieldKind::Password { f.value.chars().map(|_| '•').collect() } else { f.value.clone() };
            if f.kind == FieldKind::TextArea {
                let mut ty = y + 3;
                let lines: Vec<&str> = shown.split('\n').collect();
                let mut end = x + 5;
                for l in lines.iter() {
                    end = x + 5 + fnt.draw(gfx, x + 5, ty, l, 0x202020, inner);
                    ty += fnt.line_height();
                }
                if focused && blink { fill(gfx, end, ty - fnt.line_height() + 1, 1, fnt.line_height() - 2, 0xFF000000, inner); }
            } else {
                let ty = y + (h - fnt.line_height()) / 2;
                if shown.is_empty() && !f.placeholder.is_empty() && !focused {
                    fnt.draw(gfx, x + 5, ty, &f.placeholder, 0x909090, inner);
                }
                let tw = fnt.text_width(&shown);
                let avail = w - 12;
                let sx = if tw > avail { x + 5 - (tw - avail) } else { x + 5 };
                let end = sx + fnt.draw(gfx, sx, ty, &shown, 0x202020, inner);
                if focused && blink { fill(gfx, end, ty + 1, 1, fnt.line_height() - 2, 0xFF000000, inner); }
            }
        }
        FieldKind::Submit | FieldKind::Button | FieldKind::Reset | FieldKind::Image => {
            for i in 0..h {
                let t = 0xF4 - (i * 0x16 / h.max(1)) as u32;
                fill(gfx, x, y + i, w, 1, 0xFF000000 | t << 16 | t << 8 | t, clip);
            }
            frame(gfx, x, y, w, h, if focused { 0xFF3B82F6 } else { 0xFF8E8E8E }, clip);
            let fnt = web_font(13.0, false, false);
            let label = if f.label.is_empty() { &f.value } else { &f.label };
            let tw = fnt.text_width(label);
            fnt.draw(gfx, x + (w - tw) / 2, y + (h - fnt.line_height()) / 2, label, 0x202020, isect(c, (x + 2, y, x + w - 2, y + h)));
        }
        FieldKind::Checkbox | FieldKind::Radio => {
            let s = w.min(h).min(14);
            let bx = x + (w - s) / 2;
            let by = y + (h - s) / 2;
            fill(gfx, bx, by, s, s, 0xFFFFFFFF, clip);
            frame(gfx, bx, by, s, s, if focused { 0xFF3B82F6 } else { 0xFF767676 }, clip);
            if f.checked {
                if f.kind == FieldKind::Radio {
                    fill(gfx, bx + 4, by + 4, s - 8, s - 8, 0xFF1E6FE0, clip);
                } else {
                    fill(gfx, bx + 1, by + 1, s - 2, s - 2, 0xFF1E6FE0, clip);
                    for i in 0..4 { fill(gfx, bx + 3 + i, by + s / 2 + i - 1, 2, 2, 0xFFFFFFFF, clip); }
                    for i in 0..6 { fill(gfx, bx + 6 + i, by + s / 2 + 2 - i, 2, 2, 0xFFFFFFFF, clip); }
                }
            }
        }
        FieldKind::Select => {
            fill(gfx, x, y, w, h, 0xFFF7F7F7, clip);
            frame(gfx, x, y, w, h, if focused { 0xFF3B82F6 } else { 0xFF8E8E8E }, clip);
            let fnt = web_font(13.0, false, false);
            let label = f.options.get(f.selected).map(|o| o.1.as_str()).unwrap_or("");
            fnt.draw(gfx, x + 5, y + (h - fnt.line_height()) / 2, label, 0x202020, isect(c, (x + 2, y, x + w - 18, y + h)));
            let ax = x + w - 13;
            let ay = y + h / 2 - 2;
            for i in 0..4 { fill(gfx, ax + i, ay + i, 8 - 2 * i, 1, 0xFF404040, clip); }
        }
    }
}

pub fn paint(gfx: &mut Graphics, page: &Page, imgs: &[Option<Image>], view: Clip, scroll: i32, focus: Option<usize>, blink: bool) {
    let ox = view.0;
    let oy = view.1 - scroll;
    let top = scroll;
    let bot = scroll + (view.3 - view.1);
    let mut stack: Vec<Clip> = Vec::new();
    let mut clip = view;
    for c in page.cmds.iter() {
        match c {
            Cmd::ClipPush { x, y, w, h } => {
                stack.push(clip);
                clip = isect(clip, (ox + x, oy + y, ox + x + w, oy + y + h));
                continue;
            }
            Cmd::ClipPop => { if let Some(p) = stack.pop() { clip = p; } continue; }
            Cmd::Nop => continue,
            _ => {}
        }
        let (a, b) = c.y_range();
        if b < top || a > bot || clip.0 >= clip.2 || clip.1 >= clip.3 { continue; }
        match c {
            Cmd::Rect { x, y, w, h, color } => fill(gfx, ox + x, oy + y, *w, *h, *color, clip),
            Cmd::Border { x, y, w, h, bw, bc } => {
                let (x, y) = (ox + x, oy + y);
                fill(gfx, x, y, *w, bw[0], bc[0], clip);
                fill(gfx, x + w - bw[1], y, bw[1], *h, bc[1], clip);
                fill(gfx, x, y + h - bw[2], *w, bw[2], bc[2], clip);
                fill(gfx, x, y, bw[3], *h, bc[3], clip);
            }
            Cmd::Text { x, y, text: t, font, color, underline, strike } => {
                if color >> 24 == 0 { continue; }
                let (x, y) = (ox + x, oy + y);
                let w = font.draw(gfx, x, y, t, color & 0xFFFFFF, clip);
                if *underline { fill(gfx, x, y + font.ascent + 2, w, 1, *color | 0xFF000000, clip); }
                if *strike { fill(gfx, x, y + font.ascent * 2 / 3, w, 1, *color | 0xFF000000, clip); }
            }
            Cmd::Marker { x, y, text: t, font, color } => { font.draw(gfx, ox + x, oy + y, t, color & 0xFFFFFF, clip); }
            Cmd::Image { x, y, w, h, img } => {
                if let Some(Some(im)) = imgs.get(*img) { blit(gfx, im, ox + x, oy + y, *w, *h, clip); }
            }
            Cmd::Widget { x, y, w, h, field } => {
                if let Some(f) = page.fields.get(*field) { widget(gfx, f, ox + x, oy + y, *w, *h, focus == Some(*field), blink, clip); }
            }
            _ => {}
        }
    }
    let _ = text;
    let _ = px_get;
}
