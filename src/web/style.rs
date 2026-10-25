use alloc::collections::BTreeMap;
use alloc::rc::Rc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use super::css::*;
use super::dom::*;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Display { None, Block, Inline, InlineBlock, ListItem, Table, TableRow, TableCell, TableRowGroup, TableCaption, Flex, InlineFlex, Contents }

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Len { Auto, Px(f32), Pct(f32), Calc(f32, f32) }

impl Len {
    pub fn resolve(&self, base: f32) -> Option<f32> {
        match *self { Len::Auto => None, Len::Px(v) => Some(v), Len::Pct(p) => Some(base * p / 100.0), Len::Calc(p, x) => Some(base * p / 100.0 + x) }
    }
    pub fn or0(&self, base: f32) -> f32 { self.resolve(base).unwrap_or(0.0) }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum LineH { Normal, Mul(f32), Px(f32) }

#[derive(Clone)]
pub struct Style {
    pub display: Display,
    pub color: u32,
    pub bg: u32,
    pub font_size: f32,
    pub bold: bool,
    pub italic: bool,
    pub mono: bool,
    pub underline: bool,
    pub strike: bool,
    pub align: u8,
    pub ws: u8,
    pub margin: [Len; 4],
    pub padding: [Len; 4],
    pub border_w: [f32; 4],
    pub border_c: [u32; 4],
    pub width: Len,
    pub height: Len,
    pub min_w: Len,
    pub max_w: Len,
    pub min_h: Len,
    pub max_h: Len,
    pub line_h: LineH,
    pub list: u8,
    pub visible: bool,
    pub pos: u8,
    pub float: u8,
    pub flex_dir: u8,
    pub flex_wrap: bool,
    pub justify: u8,
    pub align_items: u8,
    pub gap_row: f32,
    pub gap_col: f32,
    pub grow: f32,
    pub shrink: f32,
    pub basis: Len,
    pub transform: u8,
    pub border_box: bool,
    pub overflow_hidden: bool,
    pub text_indent: f32,
    pub valign: u8,
    pub sr_only: bool,
    pub vars: Rc<Vec<(String, String)>>,
    pub grid_cols: u8,
    pub radius: f32,
    pub off: [Len; 4],
}

impl Style {
    pub fn initial() -> Style {
        Style {
            display: Display::Inline, color: 0xFF000000, bg: 0, font_size: 16.0, bold: false, italic: false, mono: false,
            underline: false, strike: false, align: 0, ws: 0,
            margin: [Len::Px(0.0); 4], padding: [Len::Px(0.0); 4], border_w: [0.0; 4], border_c: [0xFF000000; 4],
            width: Len::Auto, height: Len::Auto, min_w: Len::Auto, max_w: Len::Auto, min_h: Len::Auto, max_h: Len::Auto,
            line_h: LineH::Normal, list: 1, visible: true, pos: 0, float: 0, flex_dir: 0, flex_wrap: false, justify: 0,
            align_items: 0, gap_row: 0.0, gap_col: 0.0, grow: 0.0, shrink: 1.0, basis: Len::Auto, transform: 0,
            border_box: false, overflow_hidden: false, text_indent: 0.0, valign: 0, sr_only: false, vars: Rc::new(Vec::new()),
            grid_cols: 0, radius: 0.0, off: [Len::Auto; 4],
        }
    }
    pub fn is_block_level(&self) -> bool {
        matches!(self.display, Display::Block | Display::ListItem | Display::Table | Display::Flex | Display::TableRow | Display::TableRowGroup | Display::TableCell | Display::TableCaption)
    }
    pub fn line_height(&self) -> f32 {
        match self.line_h { LineH::Normal => self.font_size * 1.25, LineH::Mul(m) => self.font_size * m, LineH::Px(p) => p }
    }
}

pub fn named_color(n: &str) -> Option<u32> {
    Some(match n {
        "black" => 0x000000, "white" => 0xFFFFFF, "red" => 0xFF0000, "green" => 0x008000, "blue" => 0x0000FF,
        "yellow" => 0xFFFF00, "gray" | "grey" => 0x808080, "silver" => 0xC0C0C0, "maroon" => 0x800000, "purple" => 0x800080,
        "fuchsia" | "magenta" => 0xFF00FF, "lime" => 0x00FF00, "olive" => 0x808000, "navy" => 0x000080, "teal" => 0x008080,
        "aqua" | "cyan" => 0x00FFFF, "orange" => 0xFFA500, "darkgray" | "darkgrey" => 0xA9A9A9, "lightgray" | "lightgrey" => 0xD3D3D3,
        "darkblue" => 0x00008B, "darkred" => 0x8B0000, "darkgreen" => 0x006400, "lightblue" => 0xADD8E6, "lightgreen" => 0x90EE90,
        "pink" => 0xFFC0CB, "brown" => 0xA52A2A, "gold" => 0xFFD700, "beige" => 0xF5F5DC, "whitesmoke" => 0xF5F5F5,
        "gainsboro" => 0xDCDCDC, "dimgray" | "dimgrey" => 0x696969, "lightyellow" => 0xFFFFE0, "ivory" => 0xFFFFF0,
        "steelblue" => 0x4682B4, "royalblue" => 0x4169E1, "dodgerblue" => 0x1E90FF, "skyblue" => 0x87CEEB, "crimson" => 0xDC143C,
        "tomato" => 0xFF6347, "coral" => 0xFF7F50, "salmon" => 0xFA8072, "khaki" => 0xF0E68C, "violet" => 0xEE82EE,
        "indigo" => 0x4B0082, "slategray" | "slategrey" => 0x708090, "lightslategray" => 0x778899, "darkslategray" => 0x2F4F4F,
        "aliceblue" => 0xF0F8FF, "ghostwhite" => 0xF8F8FF, "snow" => 0xFFFAFA, "linen" => 0xFAF0E6, "wheat" => 0xF5DEB3,
        "tan" => 0xD2B48C, "chocolate" => 0xD2691E, "firebrick" => 0xB22222, "orangered" => 0xFF4500, "darkorange" => 0xFF8C00,
        "seagreen" => 0x2E8B57, "forestgreen" => 0x228B22, "limegreen" => 0x32CD32, "midnightblue" => 0x191970,
        "cornflowerblue" => 0x6495ED, "lavender" => 0xE6E6FA, "honeydew" => 0xF0FFF0, "mintcream" => 0xF5FFFA,
        "azure" => 0xF0FFFF, "floralwhite" => 0xFFFAF0, "oldlace" => 0xFDF5E6, "seashell" => 0xFFF5EE, "cadetblue" => 0x5F9EA0,
        "darkcyan" => 0x008B8B, "turquoise" => 0x40E0D0, "hotpink" => 0xFF69B4, "deeppink" => 0xFF1493, "darkviolet" => 0x9400D3,
        _ => return None,
    } | 0xFF000000)
}

fn num(s: &str) -> Option<f32> {
    let s = s.trim();
    let e = s.find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-' || c == '+' || c == 'e')).unwrap_or(s.len());
    let e = if e > 0 && s[..e].ends_with('e') { e - 1 } else { e };
    s[..e].parse::<f32>().ok()
}

pub fn parse_color(v: &str, current: u32) -> Option<u32> {
    let v = v.trim().to_ascii_lowercase();
    if v == "transparent" || v == "none" { return Some(0); }
    if v == "currentcolor" { return Some(current); }
    if v == "inherit" || v == "initial" || v == "unset" { return None; }
    if let Some(h) = v.strip_prefix('#') {
        let h: String = h.chars().take_while(|c| c.is_ascii_hexdigit()).collect();
        let p = |s: &str| u32::from_str_radix(s, 16).ok();
        return match h.len() {
            3 => { let r = p(&h[0..1])?; let g = p(&h[1..2])?; let b = p(&h[2..3])?; Some(0xFF000000 | (r * 17) << 16 | (g * 17) << 8 | b * 17) }
            4 => { let r = p(&h[0..1])?; let g = p(&h[1..2])?; let b = p(&h[2..3])?; let a = p(&h[3..4])?; Some((a * 17) << 24 | (r * 17) << 16 | (g * 17) << 8 | b * 17) }
            6 => Some(0xFF000000 | p(&h)?),
            8 => { let x = p(&h)?; Some((x & 0xFF) << 24 | (x >> 8)) }
            _ => None,
        };
    }
    if v.starts_with("rgb") || v.starts_with("hsl") {
        let a = v.find('(')?;
        let b = v.rfind(')').unwrap_or(v.len());
        let inner = v[a + 1..b].replace('/', " ").replace(',', " ");
        let parts: Vec<&str> = inner.split_whitespace().collect();
        if parts.len() < 3 { return None; }
        let alpha = if parts.len() >= 4 {
            let p = parts[3];
            let x = num(p)?;
            if p.ends_with('%') { x / 100.0 } else { x }
        } else { 1.0 };
        let (r, g, bl) = if v.starts_with("rgb") {
            let c = |p: &str| -> Option<f32> { let x = num(p)?; Some(if p.ends_with('%') { x * 2.55 } else { x }) };
            (c(parts[0])?, c(parts[1])?, c(parts[2])?)
        } else {
            let h = num(parts[0])? / 360.0;
            let s = num(parts[1])? / 100.0;
            let l = num(parts[2])? / 100.0;
            hsl(h, s, l)
        };
        let cl = |x: f32| (if x < 0.0 { 0.0 } else if x > 255.0 { 255.0 } else { x }) as u32;
        let al = cl(alpha * 255.0);
        return Some(al << 24 | cl(r) << 16 | cl(g) << 8 | cl(bl));
    }
    named_color(&v)
}

fn hsl(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    let h = h - (h as i32) as f32;
    let h = if h < 0.0 { h + 1.0 } else { h };
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;
    let f = |mut t: f32| { if t < 0.0 { t += 1.0 } if t > 1.0 { t -= 1.0 } if t < 1.0 / 6.0 { p + (q - p) * 6.0 * t } else if t < 0.5 { q } else if t < 2.0 / 3.0 { p + (q - p) * (2.0 / 3.0 - t) * 6.0 } else { p } };
    (f(h + 1.0 / 3.0) * 255.0, f(h) * 255.0, f(h - 1.0 / 3.0) * 255.0)
}

pub fn parse_len(v: &str, fs: f32) -> Option<Len> {
    let v = v.trim().to_ascii_lowercase();
    if v == "auto" || v == "none" || v == "fit-content" || v == "max-content" || v == "min-content" { return Some(Len::Auto); }
    if v.starts_with("calc(") || v.starts_with("min(") || v.starts_with("max(") || v.starts_with("clamp(") {
        return calc(&v, fs);
    }
    let n = num(&v)?;
    let unit = v.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == '-' || c == '+');
    Some(match unit {
        "px" | "" => Len::Px(n),
        "%" => Len::Pct(n),
        "em" => Len::Px(n * fs),
        "rem" => Len::Px(n * 16.0),
        "pt" => Len::Px(n * 4.0 / 3.0),
        "vw" => Len::Px(n * 10.0),
        "vh" => Len::Px(n * 7.0),
        "vmin" | "vmax" => Len::Px(n * 8.0),
        "ch" | "ex" => Len::Px(n * fs * 0.5),
        "cm" => Len::Px(n * 37.8),
        "mm" => Len::Px(n * 3.78),
        "in" => Len::Px(n * 96.0),
        "pc" => Len::Px(n * 16.0),
        _ => return None,
    })
}

fn calc(v: &str, fs: f32) -> Option<Len> {
    let inner = &v[v.find('(')? + 1..v.rfind(')')?];
    if v.starts_with("clamp(") {
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() == 3 { return parse_len(parts[1].trim(), fs).or_else(|| parse_len(parts[0].trim(), fs)); }
    }
    if v.starts_with("min(") || v.starts_with("max(") {
        let mut best: Option<Len> = None;
        for p in inner.split(',') {
            if let Some(l) = parse_len(p.trim(), fs) {
                if let (Some(Len::Px(a)), Len::Px(b)) = (best, l) {
                    best = Some(Len::Px(if v.starts_with("min(") { a.min(b) } else { a.max(b) }));
                } else if best.is_none() { best = Some(l); }
            }
        }
        return best;
    }
    let toks: Vec<&str> = inner.split_whitespace().collect();
    let mut px = 0.0f32;
    let mut pct: Option<f32> = None;
    let mut sign = 1.0f32;
    for t in toks.iter() {
        match *t {
            "+" => sign = 1.0,
            "-" => sign = -1.0,
            "*" | "/" => {}
            t => match parse_len(t, fs) {
                Some(Len::Px(x)) => px += sign * x,
                Some(Len::Pct(p)) => pct = Some(pct.unwrap_or(0.0) + sign * p),
                _ => {}
            },
        }
    }
    Some(match pct { Some(p) if px == 0.0 => Len::Pct(p), Some(p) => Len::Calc(p, px), None => Len::Px(px) })
}

fn px(v: &str, fs: f32) -> Option<f32> {
    match parse_len(v, fs)? { Len::Px(p) => Some(p), _ => None }
}

fn split_ws(v: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0;
    let mut cur = String::new();
    for ch in v.chars() {
        match ch {
            '(' => { depth += 1; cur.push(ch); }
            ')' => { depth -= 1; cur.push(ch); }
            c if c.is_whitespace() && depth == 0 => { if !cur.is_empty() { out.push(core::mem::take(&mut cur)); } }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() { out.push(cur); }
    out
}

fn four<T: Copy>(vals: &[T]) -> [T; 4] {
    match vals.len() {
        1 => [vals[0]; 4],
        2 => [vals[0], vals[1], vals[0], vals[1]],
        3 => [vals[0], vals[1], vals[2], vals[1]],
        _ => [vals[0], vals[1], vals[2], vals[3]],
    }
}

fn border_width_kw(v: &str, fs: f32) -> Option<f32> {
    match v { "thin" => Some(1.0), "medium" => Some(3.0), "thick" => Some(5.0), _ => px(v, fs) }
}

fn apply_border(s: &mut Style, sides: &[usize], v: &str) {
    let mut w: Option<f32> = None;
    let mut c: Option<u32> = None;
    let mut none = false;
    for t in split_ws(v) {
        let tl = t.to_ascii_lowercase();
        if tl == "none" || tl == "hidden" { none = true; }
        else if matches!(tl.as_str(), "solid" | "dashed" | "dotted" | "double" | "groove" | "ridge" | "inset" | "outset") { if w.is_none() { w = Some(3.0_f32.min(w.unwrap_or(1.0))); } }
        else if let Some(x) = border_width_kw(&tl, s.font_size) { w = Some(x); }
        else if let Some(col) = parse_color(&tl, s.color) { c = Some(col); }
    }
    for &i in sides {
        if none { s.border_w[i] = 0.0; continue; }
        s.border_w[i] = w.unwrap_or(if c.is_some() { 1.0 } else { 3.0 }).min(20.0);
        s.border_c[i] = c.unwrap_or(s.color);
    }
}

fn font_size_kw(v: &str, parent: f32) -> Option<f32> {
    Some(match v {
        "xx-small" => 9.0, "x-small" => 10.0, "small" => 13.0, "medium" => 16.0, "large" => 18.0,
        "x-large" => 24.0, "xx-large" => 32.0, "xxx-large" => 48.0, "smaller" => parent * 0.83, "larger" => parent * 1.2,
        _ => return None,
    })
}

pub fn apply(s: &mut Style, parent: &Style, name: &str, value: &str) {
    let v = value.trim();
    let vl = v.to_ascii_lowercase();
    if vl == "inherit" {
        match name {
            "color" => s.color = parent.color,
            "font-size" => s.font_size = parent.font_size,
            "background-color" | "background" => s.bg = parent.bg,
            "display" => s.display = parent.display,
            _ => {}
        }
        return;
    }
    let fs = s.font_size;
    match name {
        "display" => {
            s.display = match vl.split_whitespace().last().unwrap_or("") {
                "none" => Display::None,
                "block" | "flow-root" | "flow" => if vl.starts_with("inline") { Display::InlineBlock } else { Display::Block },
                "inline" => Display::Inline,
                "inline-block" | "inline-table" => Display::InlineBlock,
                "list-item" => Display::ListItem,
                "table" => Display::Table,
                "table-row" => Display::TableRow,
                "table-cell" => Display::TableCell,
                "table-row-group" | "table-header-group" | "table-footer-group" => Display::TableRowGroup,
                "table-caption" => Display::TableCaption,
                "flex" | "-webkit-box" | "-ms-flexbox" | "-webkit-flex" => if vl.starts_with("inline") { Display::InlineFlex } else { Display::Flex },
                "inline-flex" | "-webkit-inline-box" | "inline-grid" => Display::InlineFlex,
                "grid" => { if s.grid_cols == 0 { s.grid_cols = 1; } Display::Flex }
                "contents" => Display::Contents,
                "table-column" | "table-column-group" => Display::None,
                _ => s.display,
            };
            if vl == "grid" { s.flex_wrap = true; }
        }
        "color" => { if let Some(c) = parse_color(v, parent.color) { if c >> 24 != 0 { s.color = c; } } }
        "background-color" => { if let Some(c) = parse_color(v, s.color) { s.bg = c; } }
        "background" => {
            if vl == "none" || vl == "transparent" { s.bg = 0; return; }
            for t in split_ws(&vl) {
                if t.starts_with("url(") || t.contains("gradient(") {
                    if t.contains("gradient(") {
                        let inner = &t[t.find('(').unwrap() + 1..];
                        for p in inner.split(',') {
                            let p = p.trim().split_whitespace().next().unwrap_or("");
                            if let Some(c) = parse_color(p, s.color) { s.bg = c; break; }
                        }
                    }
                    continue;
                }
                if let Some(c) = parse_color(&t, s.color) { s.bg = c; }
            }
        }
        "font-size" => {
            if let Some(k) = font_size_kw(&vl, parent.font_size) { s.font_size = k; }
            else if let Some(l) = parse_len(&vl, parent.font_size) {
                s.font_size = match l { Len::Px(p) => p, Len::Pct(p) => parent.font_size * p / 100.0, Len::Calc(p, x) => parent.font_size * p / 100.0 + x, Len::Auto => s.font_size };
            }
            s.font_size = s.font_size.max(6.0).min(96.0);
        }
        "font-weight" => { s.bold = matches!(vl.as_str(), "bold" | "bolder" | "600" | "700" | "800" | "900" | "semibold" | "650" | "750"); }
        "font-style" => { s.italic = vl.starts_with("italic") || vl.starts_with("oblique"); }
        "font-family" => { s.mono = vl.contains("mono") || vl.contains("courier") || vl.contains("consolas") || vl.starts_with("monospace") || vl.contains("menlo"); }
        "font" => {
            let parts = split_ws(&vl);
            for (i, p) in parts.iter().enumerate() {
                if p == "bold" || p == "700" || p == "600" { s.bold = true; }
                if p == "italic" { s.italic = true; }
                let sz = p.split('/').next().unwrap_or("");
                if let Some(k) = font_size_kw(sz, parent.font_size) { s.font_size = k; }
                else if sz.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) && i + 1 < parts.len() {
                    if let Some(Len::Px(p)) = parse_len(sz, parent.font_size) { s.font_size = p; }
                    else if let Some(Len::Pct(p)) = parse_len(sz, parent.font_size) { s.font_size = parent.font_size * p / 100.0; }
                    if let Some(lh) = p.split('/').nth(1) { apply(s, parent, "line-height", lh); }
                }
            }
            if vl.contains("mono") || vl.contains("courier") { s.mono = true; }
        }
        "text-decoration" | "text-decoration-line" => {
            if vl.contains("none") { s.underline = false; s.strike = false; }
            if vl.contains("underline") { s.underline = true; }
            if vl.contains("line-through") { s.strike = true; }
        }
        "text-align" => { s.align = match vl.as_str() { "center" | "-webkit-center" | "-moz-center" => 1, "right" | "end" | "-webkit-right" => 2, "justify" => 3, _ => 0 }; }
        "white-space" | "text-wrap" | "white-space-collapse" => {
            s.ws = match vl.as_str() { "pre" => 1, "nowrap" => 2, "pre-wrap" | "pre-line" | "break-spaces" => 3, "normal" | "wrap" => 0, _ => s.ws };
        }
        "margin" => { let vals: Vec<Len> = split_ws(&vl).iter().filter_map(|x| parse_len(x, fs)).collect(); if !vals.is_empty() { s.margin = four(&vals); } }
        "margin-top" => { if let Some(l) = parse_len(&vl, fs) { s.margin[0] = l; } }
        "margin-right" | "margin-inline-end" => { if let Some(l) = parse_len(&vl, fs) { s.margin[1] = l; } }
        "margin-bottom" => { if let Some(l) = parse_len(&vl, fs) { s.margin[2] = l; } }
        "margin-left" | "margin-inline-start" => { if let Some(l) = parse_len(&vl, fs) { s.margin[3] = l; } }
        "margin-inline" => { let vals: Vec<Len> = split_ws(&vl).iter().filter_map(|x| parse_len(x, fs)).collect(); if let Some(a) = vals.first() { s.margin[3] = *a; s.margin[1] = *vals.get(1).unwrap_or(a); } }
        "margin-block" => { let vals: Vec<Len> = split_ws(&vl).iter().filter_map(|x| parse_len(x, fs)).collect(); if let Some(a) = vals.first() { s.margin[0] = *a; s.margin[2] = *vals.get(1).unwrap_or(a); } }
        "padding" => { let vals: Vec<Len> = split_ws(&vl).iter().filter_map(|x| parse_len(x, fs)).collect(); if !vals.is_empty() { s.padding = four(&vals); } }
        "padding-top" => { if let Some(l) = parse_len(&vl, fs) { s.padding[0] = l; } }
        "padding-right" | "padding-inline-end" => { if let Some(l) = parse_len(&vl, fs) { s.padding[1] = l; } }
        "padding-bottom" => { if let Some(l) = parse_len(&vl, fs) { s.padding[2] = l; } }
        "padding-left" | "padding-inline-start" => { if let Some(l) = parse_len(&vl, fs) { s.padding[3] = l; } }
        "padding-inline" => { let vals: Vec<Len> = split_ws(&vl).iter().filter_map(|x| parse_len(x, fs)).collect(); if let Some(a) = vals.first() { s.padding[3] = *a; s.padding[1] = *vals.get(1).unwrap_or(a); } }
        "padding-block" => { let vals: Vec<Len> = split_ws(&vl).iter().filter_map(|x| parse_len(x, fs)).collect(); if let Some(a) = vals.first() { s.padding[0] = *a; s.padding[2] = *vals.get(1).unwrap_or(a); } }
        "border" => apply_border(s, &[0, 1, 2, 3], &vl),
        "border-top" => apply_border(s, &[0], &vl),
        "border-right" => apply_border(s, &[1], &vl),
        "border-bottom" => apply_border(s, &[2], &vl),
        "border-left" => apply_border(s, &[3], &vl),
        "border-width" => { let vals: Vec<f32> = split_ws(&vl).iter().filter_map(|x| border_width_kw(x, fs)).collect(); if !vals.is_empty() { s.border_w = four(&vals); } }
        "border-color" => { let vals: Vec<u32> = split_ws(&vl).iter().filter_map(|x| parse_color(x, s.color)).collect(); if !vals.is_empty() { s.border_c = four(&vals); } }
        "border-style" => { if vl == "none" || vl == "hidden" { s.border_w = [0.0; 4]; } }
        "border-top-color" => { if let Some(c) = parse_color(&vl, s.color) { s.border_c[0] = c; } }
        "border-bottom-color" => { if let Some(c) = parse_color(&vl, s.color) { s.border_c[2] = c; } }
        "border-top-width" => { if let Some(x) = border_width_kw(&vl, fs) { s.border_w[0] = x; } }
        "border-bottom-width" => { if let Some(x) = border_width_kw(&vl, fs) { s.border_w[2] = x; } }
        "border-radius" => { if let Some(x) = px(split_ws(&vl).first().map(|x| x.as_str()).unwrap_or(""), fs) { s.radius = x; } }
        "top" => { if let Some(l) = parse_len(&vl, fs) { s.off[0] = l; } }
        "right" => { if let Some(l) = parse_len(&vl, fs) { s.off[1] = l; } }
        "bottom" => { if let Some(l) = parse_len(&vl, fs) { s.off[2] = l; } }
        "left" => { if let Some(l) = parse_len(&vl, fs) { s.off[3] = l; } }
        "inset" => { let vals: Vec<Len> = split_ws(&vl).iter().filter_map(|x| parse_len(x, fs)).collect(); if !vals.is_empty() { s.off = four(&vals); } }
        "width" => { if let Some(l) = parse_len(&vl, fs) { s.width = l; } }
        "height" => { if let Some(l) = parse_len(&vl, fs) { s.height = l; } }
        "min-width" => { if let Some(l) = parse_len(&vl, fs) { s.min_w = l; } }
        "max-width" => { if let Some(l) = parse_len(&vl, fs) { s.max_w = l; } }
        "min-height" => { if let Some(l) = parse_len(&vl, fs) { s.min_h = l; } }
        "max-height" => { if let Some(l) = parse_len(&vl, fs) { s.max_h = l; } }
        "line-height" => {
            s.line_h = if vl == "normal" { LineH::Normal }
                else if let Some(n) = vl.parse::<f32>().ok() { LineH::Mul(n) }
                else { match parse_len(&vl, fs) { Some(Len::Px(p)) => LineH::Px(p), Some(Len::Pct(p)) => LineH::Mul(p / 100.0), _ => s.line_h } };
        }
        "list-style" | "list-style-type" => {
            for t in split_ws(&vl) {
                s.list = match t.as_str() { "none" => 0, "disc" => 1, "circle" => 2, "square" => 3, "decimal" | "decimal-leading-zero" => 4, "lower-alpha" | "lower-latin" => 5, "upper-alpha" | "upper-latin" => 6, "lower-roman" | "upper-roman" => 4, _ => s.list };
            }
        }
        "visibility" => { s.visible = !(vl == "hidden" || vl == "collapse"); }
        "position" => {
            s.pos = match vl.as_str() { "relative" => 1, "absolute" => 2, "fixed" => 3, "sticky" | "-webkit-sticky" => 4, _ => 0 };
        }
        "float" => { s.float = match vl.as_str() { "left" => 1, "right" => 2, _ => 0 }; }
        "flex-direction" => { s.flex_dir = if vl.starts_with("column") { 1 } else { 0 }; }
        "flex-wrap" => { s.flex_wrap = vl == "wrap" || vl == "wrap-reverse"; }
        "flex-flow" => { if vl.contains("column") { s.flex_dir = 1; } if vl.contains("wrap") && !vl.contains("nowrap") { s.flex_wrap = true; } }
        "justify-content" => { s.justify = match vl.as_str() { "center" => 1, "flex-end" | "end" | "right" => 2, "space-between" => 3, "space-around" => 4, "space-evenly" => 5, _ => 0 }; }
        "align-items" => { s.align_items = match vl.as_str() { "center" => 2, "flex-start" | "start" | "baseline" => 1, "flex-end" | "end" => 3, _ => 0 }; }
        "gap" | "grid-gap" => {
            let vals: Vec<f32> = split_ws(&vl).iter().filter_map(|x| px(x, fs)).collect();
            if let Some(a) = vals.first() { s.gap_row = *a; s.gap_col = *vals.get(1).unwrap_or(a); }
        }
        "row-gap" | "grid-row-gap" => { if let Some(x) = px(&vl, fs) { s.gap_row = x; } }
        "column-gap" | "grid-column-gap" => { if let Some(x) = px(&vl, fs) { s.gap_col = x; } }
        "flex-grow" => { if let Some(x) = num(&vl) { s.grow = x; } }
        "flex-shrink" => { if let Some(x) = num(&vl) { s.shrink = x; } }
        "flex-basis" => { if let Some(l) = parse_len(&vl, fs) { s.basis = l; } }
        "flex" => {
            if vl == "none" { s.grow = 0.0; s.shrink = 0.0; }
            else if vl == "auto" { s.grow = 1.0; s.shrink = 1.0; }
            else {
                let p = split_ws(&vl);
                if let Some(g) = p.first().and_then(|x| x.parse::<f32>().ok()) { s.grow = g; s.basis = Len::Pct(0.0); }
                if let Some(sh) = p.get(1).and_then(|x| x.parse::<f32>().ok()) { s.shrink = sh; }
                if let Some(b) = p.last() { if p.len() > 1 { if let Some(l) = parse_len(b, fs) { s.basis = l; } } }
            }
        }
        "grid-template-columns" => {
            let mut n = 0u8;
            if let Some(i) = vl.find("repeat(") {
                let arg = &vl[i + 7..];
                if let Some(c) = arg.split(',').next().and_then(|x| x.trim().parse::<u8>().ok()) { n = c; }
                else { n = 3; }
            } else {
                n = split_ws(&vl).iter().filter(|t| !t.starts_with('[')).count() as u8;
            }
            s.grid_cols = n.max(1).min(12);
        }
        "text-transform" => { s.transform = match vl.as_str() { "uppercase" => 1, "lowercase" => 2, "capitalize" => 3, _ => 0 }; }
        "box-sizing" => { s.border_box = vl == "border-box"; }
        "overflow" | "overflow-x" | "overflow-y" => { if vl == "hidden" || vl == "clip" { s.overflow_hidden = true; } }
        "text-indent" => { if let Some(x) = px(&vl, fs) { s.text_indent = x; } }
        "vertical-align" => { s.valign = match vl.as_str() { "middle" => 1, "top" | "text-top" => 2, "bottom" | "text-bottom" => 3, "super" => 4, "sub" => 5, _ => 0 }; }
        "clip" | "clip-path" => { if vl.starts_with("rect(0") || vl.starts_with("inset(50%") || vl.contains("rect(1px") || vl.starts_with("inset(100%") { s.sr_only = true; } }
        "opacity" => { if let Some(x) = num(&vl) { if x < 0.05 { s.visible = false; } } }
        "content-visibility" => { if vl == "hidden" { s.display = Display::None; } }
        _ => {}
    }
}

pub const UA_CSS: &str = "
html, body, div, p, h1, h2, h3, h4, h5, h6, ul, ol, dl, dt, dd, pre, form, blockquote, address, center, section, article, header, footer, nav, aside, main, figure, figcaption, fieldset, legend, hr, details, summary, menu, dir, noscript, hgroup, search, frameset, frame { display: block }
head, script, style, link, meta, title, template, base, noembed, param, datalist, source, track, area, map, dialog, svg, canvas, video, audio, object, embed, [hidden] { display: none }
li { display: list-item }
table { display: table; border-spacing: 2px }
tr { display: table-row } td, th { display: table-cell; padding: 1px }
thead, tbody, tfoot { display: table-row-group } caption { display: table-caption; text-align: center }
col, colgroup { display: none }
img, input, button, select, textarea, iframe { display: inline-block }
body { margin: 8px; font-size: 16px; color: #000; line-height: 1.3 }
p, dl, blockquote, figure, pre, fieldset { margin-top: 1em; margin-bottom: 1em }
blockquote, figure { margin-left: 40px; margin-right: 40px }
h1 { font-size: 2em; font-weight: bold; margin: 0.67em 0 }
h2 { font-size: 1.5em; font-weight: bold; margin: 0.83em 0 }
h3 { font-size: 1.17em; font-weight: bold; margin: 1em 0 }
h4 { font-weight: bold; margin: 1.33em 0 }
h5 { font-size: 0.83em; font-weight: bold; margin: 1.67em 0 }
h6 { font-size: 0.67em; font-weight: bold; margin: 2.33em 0 }
ul, ol, menu, dir { margin-top: 1em; margin-bottom: 1em; padding-left: 40px }
ul ul, ol ul, ul ol, ol ol { margin-top: 0; margin-bottom: 0 }
ol { list-style-type: decimal } ul ul { list-style-type: circle }
dd { margin-left: 40px }
b, strong, th, dt { font-weight: bold } th { text-align: center }
i, em, cite, var, dfn, address { font-style: italic }
u, ins { text-decoration: underline } s, strike, del { text-decoration: line-through }
a[href] { color: #1a0dab; text-decoration: underline }
code, kbd, samp, tt, pre, listing, xmp, plaintext { font-family: monospace }
pre, xmp, listing, plaintext { white-space: pre }
small, sub, sup { font-size: 0.83em } big { font-size: 1.17em }
sup { vertical-align: super } sub { vertical-align: sub }
center { text-align: center }
hr { border: 1px solid #aaa; margin: 0.5em 0 }
mark { background-color: yellow }
nobr { white-space: nowrap }
fieldset { border: 1px solid #aaa; padding: 6px }
legend { font-weight: bold }
summary { font-weight: bold }
";
