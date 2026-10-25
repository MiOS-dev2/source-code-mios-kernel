use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format;
use super::dom::*;
use super::style::*;
use crate::font::{web_font, WebFont};

pub enum Cmd {
    Rect { x: i32, y: i32, w: i32, h: i32, color: u32 },
    Border { x: i32, y: i32, w: i32, h: i32, bw: [i32; 4], bc: [u32; 4] },
    Text { x: i32, y: i32, text: String, font: &'static WebFont, color: u32, underline: bool, strike: bool },
    Image { x: i32, y: i32, w: i32, h: i32, img: usize },
    Widget { x: i32, y: i32, w: i32, h: i32, field: usize },
    Marker { x: i32, y: i32, text: String, font: &'static WebFont, color: u32 },
    ClipPush { x: i32, y: i32, w: i32, h: i32 },
    ClipPop,
    Nop,
}

impl Cmd {
    fn shift(&mut self, dy: i32) {
        match self {
            Cmd::Rect { y, .. } | Cmd::Border { y, .. } | Cmd::Text { y, .. } | Cmd::Image { y, .. } |
            Cmd::Widget { y, .. } | Cmd::Marker { y, .. } | Cmd::ClipPush { y, .. } => *y += dy,
            _ => {}
        }
    }
    fn shift_x(&mut self, dx: i32) {
        match self {
            Cmd::Rect { x, .. } | Cmd::Border { x, .. } | Cmd::Text { x, .. } | Cmd::Image { x, .. } |
            Cmd::Widget { x, .. } | Cmd::Marker { x, .. } | Cmd::ClipPush { x, .. } => *x += dx,
            _ => {}
        }
    }
    pub fn y_range(&self) -> (i32, i32) {
        match self {
            Cmd::Rect { y, h, .. } | Cmd::Border { y, h, .. } | Cmd::Image { y, h, .. } | Cmd::Widget { y, h, .. } | Cmd::ClipPush { y, h, .. } => (*y, *y + *h),
            Cmd::Text { y, font, .. } | Cmd::Marker { y, font, .. } => (*y, *y + font.line_height() + 2),
            _ => (i32::MIN, i32::MAX),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum FieldKind { Text, Password, Submit, Button, Checkbox, Radio, Select, TextArea, Reset, Image }

#[derive(Clone)]
pub struct Field {
    pub node: usize,
    pub kind: FieldKind,
    pub name: String,
    pub value: String,
    pub checked: bool,
    pub options: Vec<(String, String)>,
    pub selected: usize,
    pub form: usize,
    pub label: String,
    pub placeholder: String,
}

pub struct LinkBox { pub x: i32, pub y: i32, pub w: i32, pub h: i32, pub link: usize }

pub struct Page {
    pub cmds: Vec<Cmd>,
    pub links: Vec<LinkBox>,
    pub hrefs: Vec<String>,
    pub fields: Vec<Field>,
    pub anchors: Vec<(String, i32)>,
    pub height: i32,
    pub width: i32,
}

pub trait ImageSizes { fn size_of(&self, src: &str) -> Option<(i32, i32, usize)>; }

enum Item {
    Word { text: String, w: f32, node: usize, link: i32 },
    Space { w: f32, node: usize, breakable: bool },
    Break,
    Atom { node: usize, w: f32, h: f32, link: i32, kind: AtomKind },
    Pad { w: f32 },
}

#[derive(Clone, Copy)]
enum AtomKind { Box, Image(usize), Widget(usize), AltBox }

pub struct Layout<'a> {
    pub dom: &'a Dom,
    pub st: &'a [Style],
    pub imgs: &'a dyn ImageSizes,
    pub cmds: Vec<Cmd>,
    pub links: Vec<LinkBox>,
    pub hrefs: Vec<String>,
    pub fields: Vec<Field>,
    pub field_state: &'a BTreeMap<usize, Field>,
    pub anchors: Vec<(String, i32)>,
    measure: u32,
    link: i32,
    cur_form: usize,
    max_cache: BTreeMap<usize, f32>,
    min_cache: BTreeMap<usize, f32>,
    depth: u32,
    abs_q: Vec<usize>,
}

const HUGE: f32 = 100000.0;

fn font_of(s: &Style) -> &'static WebFont { web_font(s.font_size, s.bold, s.mono) }

impl<'a> Layout<'a> {
    pub fn new(dom: &'a Dom, st: &'a [Style], imgs: &'a dyn ImageSizes, field_state: &'a BTreeMap<usize, Field>) -> Self {
        Layout { dom, st, imgs, cmds: Vec::new(), links: Vec::new(), hrefs: Vec::new(), fields: Vec::new(), field_state,
            anchors: Vec::new(), measure: 0, link: -1, cur_form: usize::MAX, max_cache: BTreeMap::new(), min_cache: BTreeMap::new(), depth: 0, abs_q: Vec::new() }
    }

    pub fn run(mut self, width: i32) -> Page {
        let root = self.dom.find_first("body").or_else(|| self.dom.find_first("html")).unwrap_or(ROOT);
        let h = if root == ROOT { self.flow(ROOT, 0.0, 0.0, width as f32).0 } else { self.block(root, 0.0, 0.0, width as f32, None).0 };
        let leftover = core::mem::take(&mut self.abs_q);
        for c in leftover { self.place_abs(c, 0.0, 0.0, width as f32, h); }
        Page { cmds: self.cmds, links: self.links, hrefs: self.hrefs, fields: self.fields, anchors: self.anchors, height: h as i32 + 16, width }
    }

    fn emit(&mut self, c: Cmd) -> usize {
        if self.measure > 0 { return usize::MAX; }
        self.cmds.push(c);
        self.cmds.len() - 1
    }

    fn visible_node(&self, id: usize) -> bool {
        let s = &self.st[id];
        s.display != Display::None && !s.sr_only
    }

    fn is_blockish(&self, id: usize) -> bool {
        match &self.dom.nodes[id].data {
            NodeData::Element(e) => {
                let s = &self.st[id];
                if s.float != 0 && !matches!(e.tag.as_str(), "img") && s.display != Display::None { return false; }
                if s.pos == 2 || s.pos == 3 { if s.is_block_level() { return true; } }
                if s.is_block_level() { return true; }
                if s.display == Display::Inline || s.display == Display::Contents { return self.contains_block(id, 0); }
                false
            }
            _ => false,
        }
    }

    fn contains_block(&self, id: usize, d: u32) -> bool {
        if d > 6 { return false; }
        for &c in self.dom.nodes[id].children.iter() {
            if !self.visible_node(c) { continue; }
            if let NodeData::Element(_) = self.dom.nodes[c].data {
                let s = &self.st[c];
                if s.is_block_level() && s.float == 0 { return true; }
                if (s.display == Display::Inline || s.display == Display::Contents) && self.contains_block(c, d + 1) { return true; }
            }
        }
        false
    }

    fn register_anchor(&mut self, id: usize, y: f32) {
        if self.measure > 0 { return; }
        if let Some(e) = self.dom.elem(id) {
            if !e.id.is_empty() { self.anchors.push((e.id.clone(), y as i32)); }
            if e.tag == "a" { if let Some(n) = e.attr("name") { self.anchors.push((n.to_string(), y as i32)); } }
        }
    }

    fn enter_link(&mut self, id: usize) -> i32 {
        let prev = self.link;
        if let Some(e) = self.dom.elem(id) {
            if e.tag == "a" || e.tag == "area" {
                if let Some(h) = e.attr("href") {
                    if self.measure == 0 {
                        self.hrefs.push(h.to_string());
                        self.link = (self.hrefs.len() - 1) as i32;
                    } else { self.link = 0; }
                }
            }
            if e.tag == "form" { self.cur_form = id; }
        }
        prev
    }

    fn add_link_box(&mut self, x: f32, y: f32, w: f32, h: f32, link: i32) {
        if link >= 0 && self.measure == 0 && w > 0.0 && h > 0.0 {
            self.links.push(LinkBox { x: x as i32, y: y as i32, w: w as i32, h: h as i32, link: link as usize });
        }
    }

    pub fn block(&mut self, id: usize, x: f32, y: f32, cbw: f32, forced_w: Option<f32>) -> (f32, f32) {
        self.depth += 1;
        if self.depth > 120 { self.depth -= 1; return (0.0, 0.0); }
        let s = self.st[id].clone();
        let prev_link = self.enter_link(id);
        let prev_form = if self.dom.tag(id) == "form" { usize::MAX } else { self.cur_form };
        let mut ml = s.margin[3].resolve(cbw);
        let mut mr = s.margin[1].resolve(cbw);
        let mt = s.margin[0].or0(cbw);
        let mb = s.margin[2].or0(cbw);
        let pl = s.padding[3].or0(cbw); let pr = s.padding[1].or0(cbw);
        let pt = s.padding[0].or0(cbw); let pb = s.padding[2].or0(cbw);
        let [bt, br, bb, bl] = s.border_w;
        let hor = pl + pr + bl + br;
        let mut content_w;
        let explicit_w = s.width.resolve(cbw);
        if let Some(fw) = forced_w {
            content_w = fw - hor;
            if ml.is_none() { ml = Some(0.0); }
            if mr.is_none() { mr = Some(0.0); }
        } else if let Some(w) = explicit_w {
            content_w = if s.border_box { w - hor } else { w };
        } else {
            content_w = cbw - ml.unwrap_or(0.0) - mr.unwrap_or(0.0) - hor;
        }
        if let Some(mx) = s.max_w.resolve(cbw) { let mx = if s.border_box { mx - hor } else { mx }; if content_w > mx { content_w = mx; } }
        if let Some(mn) = s.min_w.resolve(cbw) { let mn = if s.border_box { mn - hor } else { mn }; if content_w < mn { content_w = mn; } }
        if content_w < 0.0 { content_w = 0.0; }
        let box_w = content_w + hor;
        let free = cbw - box_w;
        let (ml, _mr) = match (ml, mr) {
            (None, None) => (if free > 0.0 { free / 2.0 } else { 0.0 }, 0.0),
            (None, Some(r)) => (if free - r > 0.0 { free - r } else { 0.0 }, r),
            (Some(l), None) => (l, 0.0),
            (Some(l), Some(r)) => (l, r),
        };
        let bx = x + ml;
        let by = y + mt;
        self.register_anchor(id, by);
        let bg_idx = if s.bg >> 24 != 0 && s.visible { self.emit(Cmd::Nop) } else { usize::MAX };
        let has_border = s.visible && (bt > 0.0 || br > 0.0 || bb > 0.0 || bl > 0.0);
        let cx = bx + bl + pl;
        let cy = by + bt + pt;
        let fixed_h = s.height.resolve(0.0).filter(|_| !matches!(s.height, Len::Pct(_)));
        let clip = s.overflow_hidden && fixed_h.is_some() && self.measure == 0;
        if clip {
            let h = fixed_h.unwrap();
            let ch = if s.border_box { h - pt - pb - bt - bb } else { h };
            self.emit(Cmd::ClipPush { x: cx as i32, y: cy as i32, w: content_w as i32, h: ch.max(0.0) as i32 });
        }
        let start_cmd = self.cmds.len();
        let abs_mark = self.abs_q.len();
        if s.display == Display::ListItem && s.list != 0 && self.measure == 0 {
            let marker = self.list_marker(id, s.list);
            let f = font_of(&s);
            let mw = f.text_width(&marker) as f32;
            let lh = s.line_height();
            let ty = cy + (lh - f.line_height() as f32) / 2.0;
            self.emit(Cmd::Marker { x: (cx - mw - 6.0) as i32, y: ty as i32, text: marker, font: f, color: s.color });
        }
        let (mut ch, used) = if s.display == Display::Table || self.dom.tag(id) == "table" {
            self.table(id, cx, cy, content_w, explicit_w.is_some() || forced_w.is_some())
        } else if s.display == Display::Flex || s.display == Display::InlineFlex {
            self.flex(id, cx, cy, content_w)
        } else {
            self.flow(id, cx, cy, content_w)
        };
        let _ = start_cmd;
        if let Some(h) = fixed_h {
            let want = if s.border_box { h - pt - pb - bt - bb } else { h };
            if clip || ch < want { ch = want.max(0.0); }
        }
        if let Some(mn) = s.min_h.resolve(0.0) { if !matches!(s.min_h, Len::Pct(_)) && ch < mn { ch = mn; } }
        if s.pos != 0 && self.measure == 0 && self.abs_q.len() > abs_mark {
            let mine: Vec<usize> = self.abs_q.split_off(abs_mark);
            let (ox, oy, pw, ph) = (bx + bl, by + bt, box_w - bl - br, bt + pt + ch + pb + bb - bt - bb);
            for c in mine {
                if self.st[c].pos == 3 { self.abs_q.push(c); } else { self.place_abs(c, ox, oy, pw, ph); }
            }
        }
        if clip { self.emit(Cmd::ClipPop); }
        let box_h = bt + pt + ch + pb + bb;
        let is_table = s.display == Display::Table || self.dom.tag(id) == "table";
        let box_w = if is_table && explicit_w.is_none() && forced_w.is_none() && used > 0.0 { (used + hor).min(box_w) } else { box_w };
        if bg_idx != usize::MAX && self.measure == 0 {
            self.cmds[bg_idx] = Cmd::Rect { x: bx as i32, y: by as i32, w: box_w as i32, h: box_h as i32, color: s.bg };
        }
        if has_border {
            self.emit(Cmd::Border { x: bx as i32, y: by as i32, w: box_w as i32, h: box_h as i32,
                bw: [bt as i32, br as i32, bb as i32, bl as i32], bc: s.border_c });
        }
        if self.link >= 0 && prev_link != self.link && self.measure == 0 {
            self.add_link_box(bx, by, box_w, box_h, self.link);
        }
        self.link = prev_link;
        if prev_form != usize::MAX || self.dom.tag(id) == "form" { self.cur_form = prev_form; }
        self.depth -= 1;
        let used_total = if explicit_w.is_some() || forced_w.is_some() { box_w } else { used + hor };
        (mt + box_h + mb, used_total + ml.max(0.0))
    }

    fn place_abs(&mut self, id: usize, ox: f32, oy: f32, pw: f32, ph: f32) {
        let s = self.st[id].clone();
        let [t, r, b, l] = s.off;
        let left = l.resolve(pw);
        let right = r.resolve(pw);
        let top = t.resolve(ph);
        let bottom = b.resolve(ph);
        let ml = s.margin[3].or0(pw);
        let mr = s.margin[1].or0(pw);
        let need_size = (left.is_none() && right.is_some()) || (top.is_none() && bottom.is_some());
        let avail = (pw - left.unwrap_or(0.0) - right.unwrap_or(0.0)).max(0.0);
        if !s.is_block_level() {
            let parent = self.dom.nodes[id].parent;
            let (mut bw, mut bh) = (0.0f32, 0.0f32);
            if need_size {
                self.measure += 1;
                let (hh, uu) = self.inline_run(&[id], 0.0, 0.0, avail.max(1.0), parent);
                self.measure -= 1;
                bw = uu; bh = hh;
            }
            let x = if let Some(lv) = left { ox + lv } else if let Some(rv) = right { ox + pw - rv - bw } else { ox };
            let y = if let Some(tv) = top { oy + tv } else if let Some(bv) = bottom { oy + ph - bv - bh } else { oy };
            self.inline_run(&[id], x, y, avail.max(1.0), parent);
            return;
        }
        let has_w = s.width.resolve(pw).is_some();
        let forced: Option<f32> = if has_w { None }
            else if left.is_some() && right.is_some() { Some((avail - ml - mr).max(0.0)) }
            else { let mc = self.max_content(id); Some(mc.min(avail).max(0.0)) };
        let (mut bw, mut bh) = (0.0f32, 0.0f32);
        if need_size {
            self.measure += 1;
            let (hh, uu) = self.block(id, 0.0, 0.0, pw, forced);
            self.measure -= 1;
            bw = (uu - ml.max(0.0)).max(0.0); bh = hh;
        }
        let x = if let Some(lv) = left { ox + lv } else if let Some(rv) = right { ox + pw - rv - mr - bw - ml } else { ox };
        let y = if let Some(tv) = top { oy + tv } else if let Some(bv) = bottom { oy + ph - bv - bh } else { oy };
        self.block(id, x, y, pw, forced);
    }

    fn list_marker(&self, id: usize, kind: u8) -> String {
        match kind {
            1 => String::from("•"), 2 => String::from("◦"), 3 => String::from("▪"),
            _ => {
                let p = self.dom.nodes[id].parent;
                let mut n = 1i32;
                if p != usize::MAX {
                    if let Some(e) = self.dom.elem(p) { if let Some(st) = e.attr("start") { n = st.trim().parse().unwrap_or(1); } }
                    for &c in self.dom.nodes[p].children.iter() {
                        if c == id { break; }
                        if self.dom.tag(c) == "li" { n += 1; }
                    }
                }
                match kind {
                    5 => format!("{}.", (b'a' + ((n - 1).rem_euclid(26)) as u8) as char),
                    6 => format!("{}.", (b'A' + ((n - 1).rem_euclid(26)) as u8) as char),
                    _ => format!("{}.", n),
                }
            }
        }
    }

    fn flow(&mut self, id: usize, x: f32, y: f32, w: f32) -> (f32, f32) {
        let mut cy = y;
        let mut used = 0.0f32;
        let mut run: Vec<usize> = Vec::new();
        let mut last_mb = 0.0f32;
        let children = self.flat_children(id);
        for c in children {
            if let NodeData::Element(_) = &self.dom.nodes[c].data {
                let sp = self.st[c].pos;
                if sp == 2 || sp == 3 {
                    if self.measure == 0 { self.abs_q.push(c); }
                    continue;
                }
            }
            if self.is_blockish(c) {
                if !run.is_empty() {
                    let (h, u) = self.inline_run(&run, x, cy, w, id);
                    if h > 0.0 { last_mb = 0.0; }
                    cy += h; used = used.max(u); run.clear();
                }
                let s = &self.st[c];
                let mt = s.margin[0].or0(w);
                let collapse = mt.min(last_mb).max(0.0);
                let (h, u) = self.block(c, x, cy - collapse, w, None);
                cy += h - collapse;
                last_mb = self.st[c].margin[2].or0(w);
                used = used.max(u);
            } else {
                run.push(c);
            }
        }
        if !run.is_empty() {
            let (h, u) = self.inline_run(&run, x, cy, w, id);
            cy += h; used = used.max(u);
        }
        (cy - y, used)
    }

    fn flat_children(&self, id: usize) -> Vec<usize> {
        let mut v = Vec::new();
        for &c in self.dom.nodes[id].children.iter() {
            match &self.dom.nodes[c].data {
                NodeData::Text(_) => v.push(c),
                NodeData::Element(_) => {
                    if !self.visible_node(c) { continue; }
                    if self.st[c].display == Display::Contents {
                        v.extend(self.flat_children(c));
                    } else if (self.st[c].display == Display::Inline) && self.contains_block(c, 0) && self.dom.tag(c) != "a" {
                        v.extend(self.flat_children(c));
                    } else { v.push(c); }
                }
                _ => {}
            }
        }
        v
    }

    fn collect(&mut self, id: usize, items: &mut Vec<Item>) {
        match &self.dom.nodes[id].data {
            NodeData::Text(t) => {
                let p = self.dom.nodes[id].parent;
                let s = &self.st[p];
                if !s.visible { 
                }
                let f = font_of(s);
                let text: String = match s.transform {
                    1 => t.to_uppercase(), 2 => t.to_lowercase(),
                    3 => { let mut o = String::new(); let mut up = true; for ch in t.chars() { if up && ch.is_alphabetic() { o.extend(ch.to_uppercase()); up = false; } else { o.push(ch); } if ch.is_whitespace() { up = true; } } o }
                    _ => t.clone(),
                };
                let sw = f.char_width(' ') as f32;
                if s.ws == 1 || s.ws == 3 {
                    for (li, line) in text.split('\n').enumerate() {
                        if li > 0 { items.push(Item::Break); }
                        let line = line.replace('\t', "    ");
                        let mut first = true;
                        for word in line.split(' ') {
                            if !first { items.push(Item::Space { w: sw, node: p, breakable: s.ws == 3 }); }
                            first = false;
                            if !word.is_empty() { items.push(Item::Word { w: f.text_width(word) as f32, text: word.to_string(), node: p, link: self.link }); }
                        }
                    }
                } else {
                    let brk = s.ws != 2;
                    let mut first = true;
                    let space = |items: &mut Vec<Item>| {
                        if !matches!(items.last(), Some(Item::Space { .. })) { items.push(Item::Space { w: sw, node: p, breakable: brk }); }
                    };
                    if text.starts_with(|c: char| c.is_whitespace()) { space(items); }
                    for word in text.split(|c: char| c == ' ' || c == '\n' || c == '\t' || c == '\r' || c == '\u{000C}') {
                        if word.is_empty() { continue; }
                        if !first { space(items); }
                        first = false;
                        items.push(Item::Word { w: f.text_width(word) as f32, text: word.to_string(), node: p, link: self.link });
                    }
                    if !first && text.ends_with(|c: char| c.is_whitespace()) { space(items); }
                }
            }
            NodeData::Element(e) => {
                if !self.visible_node(id) { return; }
                let tag = e.tag.as_str();
                let s = self.st[id].clone();
                match tag {
                    "br" => { items.push(Item::Break); return; }
                    "wbr" => return,
                    "img" => { self.collect_img(id, items); return; }
                    "input" | "button" | "select" | "textarea" => { self.collect_widget(id, items); return; }
                    "iframe" => {
                        let w = s.width.resolve(600.0).unwrap_or(300.0);
                        let h = s.height.resolve(150.0).unwrap_or(150.0).min(400.0);
                        items.push(Item::Atom { node: id, w, h, link: self.link, kind: AtomKind::AltBox });
                        return;
                    }
                    _ => {}
                }
                if s.display == Display::InlineBlock || s.display == Display::InlineFlex || s.float != 0 || s.is_block_level() {
                    let w = self.shrink_width(id, 1200.0);
                    items.push(Item::Atom { node: id, w, h: 0.0, link: self.link, kind: AtomKind::Box });
                    return;
                }
                let prev = self.enter_link(id);
                if self.link >= 0 && self.link != prev && self.measure == 0 { self.register_anchor(id, 0.0); }
                let lpad = s.padding[3].or0(0.0) + s.border_w[3] + s.margin[3].or0(0.0);
                let rpad = s.padding[1].or0(0.0) + s.border_w[1] + s.margin[1].or0(0.0);
                if lpad > 0.5 { items.push(Item::Pad { w: lpad }); }
                for &c in self.dom.nodes[id].children.clone().iter() { self.collect(c, items); }
                if rpad > 0.5 { items.push(Item::Pad { w: rpad }); }
                self.link = prev;
            }
            _ => {}
        }
    }

    fn collect_img(&mut self, id: usize, items: &mut Vec<Item>) {
        let e = self.dom.elem(id).unwrap();
        let s = &self.st[id];
        let src = e.attr("src").or_else(|| e.attr("data-src")).unwrap_or("");
        let alt = e.attr("alt").unwrap_or("");
        let known = if src.is_empty() { None } else { self.imgs.size_of(src) };
        let sw = s.width.resolve(800.0);
        let sh = s.height.resolve(0.0).filter(|_| !matches!(s.height, Len::Pct(_)));
        let (w, h, kind) = match known {
            Some((iw, ih, idx)) => {
                let (iw, ih) = (iw.max(1) as f32, ih.max(1) as f32);
                let (w, h) = match (sw, sh) {
                    (Some(w), Some(h)) => (w, h),
                    (Some(w), None) => (w, w * ih / iw),
                    (None, Some(h)) => (h * iw / ih, h),
                    (None, None) => (iw, ih),
                };
                (w, h, AtomKind::Image(idx))
            }
            None => {
                if let (Some(w), Some(h)) = (sw, sh) {
                    if alt.is_empty() || w < 20.0 { (w, h, AtomKind::AltBox) } else { (w, h, AtomKind::AltBox) }
                } else if !alt.is_empty() {
                    let f = font_of(s);
                    (f.text_width(alt) as f32 + 8.0, f.line_height() as f32 + 4.0, AtomKind::AltBox)
                } else { (0.0, 0.0, AtomKind::AltBox) }
            }
        };
        let (mut w, mut h) = (w, h);
        if let Some(mx) = s.max_w.resolve(1000.0) { if w > mx && mx > 0.0 { h = h * mx / w; w = mx; } }
        if w > 0.0 && h > 0.0 { items.push(Item::Atom { node: id, w, h, link: self.link, kind }); }
    }

    fn field_for(&mut self, id: usize) -> usize {
        if self.measure > 0 { return 0; }
        let e = self.dom.elem(id).unwrap();
        let tag = e.tag.as_str();
        let typ = e.attr("type").unwrap_or("").to_ascii_lowercase();
        let kind = match tag {
            "select" => FieldKind::Select,
            "textarea" => FieldKind::TextArea,
            "button" => if typ == "button" { FieldKind::Button } else if typ == "reset" { FieldKind::Reset } else { FieldKind::Submit },
            _ => match typ.as_str() {
                "submit" => FieldKind::Submit, "button" => FieldKind::Button, "reset" => FieldKind::Reset,
                "checkbox" => FieldKind::Checkbox, "radio" => FieldKind::Radio, "password" => FieldKind::Password,
                "image" => FieldKind::Image, _ => FieldKind::Text,
            },
        };
        let mut f = if let Some(old) = self.field_state.get(&id) { old.clone() } else {
            let mut options = Vec::new();
            let mut selected = 0;
            if kind == FieldKind::Select {
                self.collect_options(id, &mut options, &mut selected);
            }
            let value = match kind {
                FieldKind::TextArea => self.dom.text_content(id).trim_start_matches('\n').to_string(),
                FieldKind::Select => options.get(selected).map(|o: &(String, String)| o.0.clone()).unwrap_or_default(),
                FieldKind::Submit | FieldKind::Button | FieldKind::Reset if tag == "button" => e.attr("value").unwrap_or("").to_string(),
                FieldKind::Checkbox | FieldKind::Radio => e.attr("value").unwrap_or("on").to_string(),
                _ => e.attr("value").unwrap_or("").to_string(),
            };
            let label = match kind {
                FieldKind::Submit if tag == "input" => e.attr("value").unwrap_or("Отправить").to_string(),
                FieldKind::Reset if tag == "input" => e.attr("value").unwrap_or("Сброс").to_string(),
                FieldKind::Button if tag == "input" => e.attr("value").unwrap_or("").to_string(),
                FieldKind::Image => e.attr("alt").unwrap_or("Отправить").to_string(),
                _ if tag == "button" => { let t = self.dom.text_content(id); t.split_whitespace().collect::<Vec<_>>().join(" ") }
                _ => String::new(),
            };
            Field { node: id, kind, name: e.attr("name").unwrap_or("").to_string(), value, checked: e.attr("checked").is_some(),
                options, selected, form: usize::MAX, label, placeholder: e.attr("placeholder").unwrap_or("").to_string() }
        };
        f.form = self.cur_form;
        if f.form == usize::MAX {
            let mut p = self.dom.nodes[id].parent;
            while p != usize::MAX { if self.dom.tag(p) == "form" { f.form = p; break; } p = self.dom.nodes[p].parent; }
        }
        self.fields.push(f);
        self.fields.len() - 1
    }

    fn collect_options(&self, id: usize, out: &mut Vec<(String, String)>, sel: &mut usize) {
        for &c in self.dom.nodes[id].children.iter() {
            match self.dom.tag(c) {
                "option" => {
                    let e = self.dom.elem(c).unwrap();
                    let label = self.dom.text_content(c).trim().to_string();
                    let val = e.attr("value").map(|v| v.to_string()).unwrap_or_else(|| label.clone());
                    if e.attr("selected").is_some() { *sel = out.len(); }
                    out.push((val, label));
                }
                "optgroup" => self.collect_options(c, out, sel),
                _ => {}
            }
        }
    }

    fn collect_widget(&mut self, id: usize, items: &mut Vec<Item>) {
        let fi = self.field_for(id);
        let s = self.st[id].clone();
        let e = self.dom.elem(id).unwrap();
        let f = font_of(&s);
        let lh = f.line_height() as f32;
        let typ = e.attr("type").unwrap_or("").to_ascii_lowercase();
        let tag = e.tag.clone();
        let (mut w, mut h) = match tag.as_str() {
            "textarea" => {
                let cols: f32 = e.attr("cols").and_then(|v| v.parse().ok()).unwrap_or(40.0);
                let rows: f32 = e.attr("rows").and_then(|v| v.parse().ok()).unwrap_or(3.0);
                (cols * 8.0 + 8.0, rows * lh + 8.0)
            }
            "select" => {
                let mut mw = 40.0f32;
                if self.measure == 0 { for (_, l) in self.fields[fi].options.iter() { mw = mw.max(f.text_width(l) as f32); } }
                else { mw = 120.0; }
                (mw + 30.0, lh + 8.0)
            }
            "button" => {
                let t = self.dom.text_content(id);
                let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
                ((f.text_width(&t) as f32 + 20.0).max(24.0), lh + 10.0)
            }
            _ => match typ.as_str() {
                "checkbox" | "radio" => (15.0, 15.0),
                "submit" | "button" | "reset" | "image" => {
                    let t = e.attr("value").or_else(|| e.attr("alt")).unwrap_or(if typ == "reset" { "Сброс" } else { "Отправить" });
                    (f.text_width(t) as f32 + 20.0, lh + 10.0)
                }
                _ => {
                    let size: f32 = e.attr("size").and_then(|v| v.parse().ok()).unwrap_or(24.0);
                    ((size * 7.5).max(60.0).min(600.0), lh + 10.0)
                }
            },
        };
        if let Some(sw) = s.width.resolve(600.0) { if sw > 8.0 { w = if s.border_box { sw } else { sw + 8.0 }; } }
        if let Some(sh) = s.height.resolve(0.0) { if sh > 8.0 && !matches!(s.height, Len::Pct(_)) { h = sh; } }
        if let Some(mx) = s.max_w.resolve(600.0) { if w > mx { w = mx; } }
        items.push(Item::Atom { node: id, w, h, link: -1, kind: AtomKind::Widget(fi) });
    }

    fn shrink_width(&mut self, id: usize, avail: f32) -> f32 {
        let s = &self.st[id];
        let hor = s.padding[1].or0(avail) + s.padding[3].or0(avail) + s.border_w[1] + s.border_w[3];
        if let Some(w) = s.width.resolve(avail) { return if s.border_box { w } else { w + hor }; }
        let mx = self.max_content(id);
        let mut w = mx.min(avail);
        if let Some(m) = s.max_w.resolve(avail) { w = w.min(m + hor); }
        if let Some(m) = s.min_w.resolve(avail) { w = w.max(m); }
        w
    }

    pub fn max_content(&mut self, id: usize) -> f32 {
        if let Some(v) = self.max_cache.get(&id) { return *v; }
        self.measure += 1;
        let saved_link = self.link;
        let (_, used) = self.block(id, 0.0, 0.0, HUGE, None);
        self.link = saved_link;
        self.measure -= 1;
        let v = used.min(HUGE / 2.0);
        self.max_cache.insert(id, v);
        v
    }

    pub fn min_content(&mut self, id: usize) -> f32 {
        if let Some(v) = self.min_cache.get(&id) { return *v; }
        self.measure += 1;
        let saved_link = self.link;
        let (_, used) = self.block(id, 0.0, 0.0, 1.0, None);
        self.link = saved_link;
        self.measure -= 1;
        self.min_cache.insert(id, used);
        used
    }

    fn inline_run(&mut self, nodes: &[usize], x: f32, y: f32, w: f32, container: usize) -> (f32, f32) {
        let mut items: Vec<Item> = Vec::new();
        for &n in nodes { self.collect(n, &mut items); }
        if !items.iter().any(|i| matches!(i, Item::Word { .. } | Item::Atom { .. } | Item::Break)) { return (0.0, 0.0); }
        let cs = self.st[container].clone();
        let align = cs.align;
        let indent = cs.text_indent.max(0.0);
        let mut lines: Vec<(usize, usize, f32)> = Vec::new();
        let mut start = 0usize;
        let mut lw = indent;
        let mut i = 0usize;
        let mut last_break_ok = true;
        while i < items.len() {
            let iw = match &items[i] {
                Item::Word { w, .. } => *w, Item::Space { w, .. } => *w, Item::Atom { w, .. } => *w, Item::Pad { w } => *w, Item::Break => 0.0,
            };
            match &items[i] {
                Item::Break => { lines.push((start, i + 1, lw)); start = i + 1; lw = 0.0; i += 1; continue; }
                Item::Space { breakable, .. } => {
                    last_break_ok = *breakable;
                    if lw == 0.0 || i == start { start = i + 1; i += 1; continue; }
                }
                _ => {}
            }
            let is_space = matches!(items[i], Item::Space { .. });
            if !is_space && lw + iw > w && i > start && last_break_ok {
                lines.push((start, i, lw));
                start = i; lw = 0.0;
                continue;
            }
            if !is_space && iw > w && i == start && self.measure == 0 && w >= 20.0 && matches!(&items[i], Item::Word { text, .. } if text.chars().count() > 1) {
                if let Item::Word { text, node, link, .. } = &items[i] {
                    let (text, node, link) = (text.clone(), *node, *link);
                    let f = font_of(&self.st[node]);
                    let mut acc = String::new();
                    let mut aw = 0.0f32;
                    let mut pieces: Vec<Item> = Vec::new();
                    for ch in text.chars() {
                        let cw = f.char_width(ch) as f32;
                        if aw + cw > w && !acc.is_empty() {
                            pieces.push(Item::Word { text: core::mem::take(&mut acc), w: aw, node, link });
                            pieces.push(Item::Break);
                            aw = 0.0;
                        }
                        acc.push(ch); aw += cw;
                    }
                    if !acc.is_empty() { pieces.push(Item::Word { text: acc, w: aw, node, link }); }
                    items.splice(i..i + 1, pieces);
                    continue;
                }
            }
            if !is_space { last_break_ok = true; }
            if let Item::Word { .. } = items[i] { if !is_space { } }
            lw += iw;
            i += 1;
        }
        if start < items.len() { lines.push((start, items.len(), lw)); }
        let mut cy = y;
        let mut used = 0.0f32;
        for (li, (a, b, _)) in lines.iter().enumerate() {
            let (a, b) = (*a, *b);
            let mut end = b;
            while end > a && matches!(items[end - 1], Item::Space { .. } | Item::Break) { end -= 1; }
            let mut asc = 0.0f32;
            let mut desc = 0.0f32;
            let mut line_w = if li == 0 { indent } else { 0.0 };
            let base_lh = cs.line_height();
            let fcs = font_of(&cs);
            let half = (base_lh - fcs.line_height() as f32) / 2.0;
            let (ca, cd) = (fcs.ascent as f32 + half, fcs.descent as f32 + half);
            if end > a || li == 0 { asc = asc.max(ca); desc = desc.max(cd); }
            for k in a..end {
                match &items[k] {
                    Item::Word { node, w, .. } => {
                        let s = &self.st[*node];
                        let f = font_of(s);
                        let lh = s.line_height();
                        let half = (lh - f.line_height() as f32) / 2.0;
                        asc = asc.max(f.ascent as f32 + half);
                        desc = desc.max(f.descent as f32 + half);
                        line_w += *w;
                    }
                    Item::Space { w, .. } | Item::Pad { w } => line_w += *w,
                    Item::Atom { node, w, h, kind, .. } => {
                        let hh = if let AtomKind::Box = kind { self.atom_height(*node, *w) } else { *h };
                        let va = self.st[*node].valign;
                        if va == 1 { asc = asc.max(hh / 2.0 + 4.0); desc = desc.max(hh / 2.0 - 4.0); }
                        else { asc = asc.max(hh); }
                        line_w += *w;
                    }
                    Item::Break => {}
                }
            }
            let line_h = asc + desc;
            used = used.max(line_w);
            let mut lx = x + if li == 0 { indent } else { 0.0 };
            if self.measure == 0 {
                if align == 1 && line_w < w { lx += (w - line_w) / 2.0; }
                else if align == 2 && line_w < w { lx += w - line_w; }
            }
            let baseline = cy + asc;
            if self.measure == 0 {
                self.emit_line(&items[a..end], lx, baseline, cy, line_h);
            }
            cy += line_h.max(1.0);
        }
        (cy - y, used)
    }

    fn atom_height(&mut self, node: usize, w: f32) -> f32 {
        self.measure += 1;
        let (h, _) = self.block(node, 0.0, 0.0, w, Some(w - self.st[node].margin[1].or0(w) - self.st[node].margin[3].or0(w)));
        self.measure -= 1;
        h
    }

    fn emit_line(&mut self, items: &[Item], x: f32, baseline: f32, top: f32, line_h: f32) {
        let mut cx = x;
        let mut run = String::new();
        let mut run_x = 0.0f32;
        let mut run_node = usize::MAX;
        let mut run_link = -2i32;
        let mut run_w = 0.0f32;
        let mut k = 0;
        while k <= items.len() {
            let flush = k == items.len() || match &items[k] {
                Item::Word { node, link, .. } => *node != run_node || *link != run_link,
                Item::Space { node, .. } => *node != run_node && !run.is_empty() && false,
                _ => true,
            };
            if flush && !run.is_empty() {
                self.emit_text(&run, run_x, baseline, run_node, run_link, run_w, top, line_h);
                run.clear(); run_w = 0.0;
            }
            if k == items.len() { break; }
            match &items[k] {
                Item::Word { text, w, node, link } => {
                    if run.is_empty() { run_x = cx; run_node = *node; run_link = *link; }
                    run.push_str(text); run_w += *w; cx += *w;
                }
                Item::Space { w, node, .. } => {
                    if !run.is_empty() {
                        run.push(' '); run_w += *w;
                    } else if self.st[*node].bg >> 24 != 0 {
                        self.emit(Cmd::Rect { x: cx as i32, y: top as i32, w: *w as i32 + 1, h: line_h as i32, color: self.st[*node].bg });
                    }
                    cx += *w;
                }
                Item::Pad { w } => { cx += *w; }
                Item::Break => {}
                Item::Atom { node, w, h, link, kind } => {
                    let (node, w, h, link, kind) = (*node, *w, *h, *link, *kind);
                    let va = self.st[node].valign;
                    match kind {
                        AtomKind::Box => {
                            let hh = self.atom_height(node, w);
                            let ty = if va == 1 { baseline - hh / 2.0 - 4.0 } else { baseline - hh };
                            let prev = self.link;
                            self.link = link;
                            let mw = w - self.st[node].margin[1].or0(w) - self.st[node].margin[3].or0(w);
                            self.block(node, cx, ty, w, Some(mw));
                            self.link = prev;
                            self.add_link_box(cx, ty, w, hh, link);
                        }
                        AtomKind::Image(idx) => {
                            let ty = if va == 1 { baseline - h / 2.0 - 4.0 } else { baseline - h };
                            self.register_anchor(node, ty);
                            self.emit(Cmd::Image { x: cx as i32, y: ty as i32, w: w as i32, h: h as i32, img: idx });
                            self.add_link_box(cx, ty, w, h, link);
                        }
                        AtomKind::AltBox => {
                            let ty = baseline - h;
                            let alt = self.dom.elem(node).and_then(|e| e.attr("alt").or_else(|| e.attr("title"))).unwrap_or("").to_string();
                            self.emit(Cmd::Border { x: cx as i32, y: ty as i32, w: w as i32, h: h as i32, bw: [1; 4], bc: [0xFFC8C8C8; 4] });
                            if !alt.is_empty() {
                                let s = &self.st[node];
                                let f = font_of(s);
                                self.emit(Cmd::ClipPush { x: cx as i32, y: ty as i32, w: w as i32, h: h as i32 });
                                self.emit(Cmd::Text { x: cx as i32 + 4, y: ty as i32 + 2, text: alt, font: f, color: 0xFF707070, underline: false, strike: false });
                                self.emit(Cmd::ClipPop);
                            }
                            self.add_link_box(cx, ty, w, h, link);
                        }
                        AtomKind::Widget(fi) => {
                            let ty = if va == 1 { baseline - h / 2.0 - 4.0 } else { baseline - h + 4.0 };
                            self.emit(Cmd::Widget { x: cx as i32, y: ty as i32, w: w as i32, h: h as i32, field: fi });
                        }
                    }
                    cx += w;
                }
            }
            k += 1;
        }
    }

    fn emit_text(&mut self, text: &str, x: f32, baseline: f32, node: usize, link: i32, w: f32, top: f32, line_h: f32) {
        let s = &self.st[node];
        if !s.visible { return; }
        let f = font_of(s);
        let mut ty = baseline - f.ascent as f32;
        if s.valign == 4 { ty -= s.font_size * 0.4; }
        if s.valign == 5 { ty += s.font_size * 0.25; }
        if s.bg >> 24 != 0 {
            self.emit(Cmd::Rect { x: x as i32, y: top as i32, w: w as i32 + 1, h: line_h as i32, color: s.bg });
        }
        let (color, underline, strike) = (s.color, s.underline, s.strike);
        self.emit(Cmd::Text { x: x as i32, y: ty as i32, text: text.to_string(), font: f, color, underline, strike });
        self.add_link_box(x, top, w, line_h, link);
    }

    fn table_rows(&self, id: usize, rows: &mut Vec<usize>, depth: u32) {
        if depth > 3 { return; }
        for &c in self.dom.nodes[id].children.iter() {
            if !self.visible_node(c) { continue; }
            let t = self.dom.tag(c);
            let d = self.st[c].display;
            if t == "tr" || d == Display::TableRow { rows.push(c); }
            else if t == "thead" || t == "tbody" || t == "tfoot" || d == Display::TableRowGroup || d == Display::Contents || t == "form" { self.table_rows(c, rows, depth + 1); }
        }
    }

    fn row_cells(&self, row: usize) -> Vec<(usize, usize)> {
        let mut v = Vec::new();
        for &c in self.dom.nodes[row].children.iter() {
            if !self.visible_node(c) { continue; }
            let t = self.dom.tag(c);
            if t == "td" || t == "th" || self.st[c].display == Display::TableCell {
                let span = self.dom.elem(c).and_then(|e| e.attr("colspan")).and_then(|v| v.trim().parse::<usize>().ok()).unwrap_or(1).max(1).min(50);
                v.push((c, span));
            } else if t == "form" || self.st[c].display == Display::Contents {
                v.extend(self.row_cells(c));
            }
        }
        v
    }

    fn table(&mut self, id: usize, x: f32, y: f32, w: f32, fixed_width: bool) -> (f32, f32) {
        let e = self.dom.elem(id);
        let spacing: f32 = e.and_then(|e| e.attr("cellspacing")).and_then(|v| v.trim().parse().ok()).unwrap_or(2.0);
        let cellpad: Option<f32> = e.and_then(|e| e.attr("cellpadding")).and_then(|v| v.trim().parse().ok());
        let border_attr: f32 = e.and_then(|e| e.attr("border")).and_then(|v| v.trim().parse().ok()).unwrap_or(0.0);
        let mut rows = Vec::new();
        self.table_rows(id, &mut rows, 0);
        let mut caption_h = 0.0;
        for &c in self.dom.nodes[id].children.iter() {
            if self.dom.tag(c) == "caption" && self.visible_node(c) {
                let (h, _) = self.block(c, x, y, w, None);
                caption_h += h;
            }
        }
        let grid: Vec<Vec<(usize, usize)>> = rows.iter().map(|&r| self.row_cells(r)).collect();
        let ncols = grid.iter().map(|r| r.iter().map(|c| c.1).sum::<usize>()).max().unwrap_or(0);
        if ncols == 0 { return (caption_h, 0.0); }
        let mut minw = alloc::vec![0.0f32; ncols];
        let mut maxw = alloc::vec![0.0f32; ncols];
        let mut pctw = alloc::vec![0.0f32; ncols];
        let mut fixw = alloc::vec![0.0f32; ncols];
        let extra_pad = cellpad.map(|p| p * 2.0).unwrap_or(0.0) + if border_attr > 0.0 { 2.0 } else { 0.0 };
        for r in grid.iter() {
            let mut col = 0;
            for &(c, span) in r.iter() {
                if span == 1 && col < ncols {
                    let s = &self.st[c];
                    let mn = self.min_content(c) + extra_pad;
                    let mx = self.max_content(c) + extra_pad;
                    minw[col] = minw[col].max(mn);
                    maxw[col] = maxw[col].max(mx);
                    match s.width { Len::Px(p) => fixw[col] = fixw[col].max(p + extra_pad), Len::Pct(p) => pctw[col] = pctw[col].max(p), _ => {} }
                }
                col += span;
            }
        }
        for i in 0..ncols {
            if fixw[i] > 0.0 { maxw[i] = fixw[i].max(minw[i]); minw[i] = minw[i].max(fixw[i].min(maxw[i])); }
            if maxw[i] < minw[i] { maxw[i] = minw[i]; }
        }
        let avail = (w - spacing * (ncols as f32 + 1.0)).max(ncols as f32);
        let mut cols = alloc::vec![0.0f32; ncols];
        for i in 0..ncols { if pctw[i] > 0.0 { let p = avail * pctw[i] / 100.0; maxw[i] = maxw[i].max(p); if fixed_width { minw[i] = minw[i].max(p.min(avail)); } } }
        let smin: f32 = minw.iter().sum();
        let smax: f32 = maxw.iter().sum();
        if smax <= avail {
            for i in 0..ncols { cols[i] = maxw[i]; }
            if fixed_width && smax > 0.0 {
                let extra = avail - smax;
                let flex: f32 = (0..ncols).filter(|&i| fixw[i] == 0.0).map(|i| maxw[i].max(1.0)).sum();
                for i in 0..ncols { if fixw[i] == 0.0 && flex > 0.0 { cols[i] += extra * maxw[i].max(1.0) / flex; } }
                if flex == 0.0 { for i in 0..ncols { cols[i] += extra / ncols as f32; } }
            }
        } else if smin >= avail {
            for i in 0..ncols { cols[i] = minw[i]; }
        } else {
            let k = (avail - smin) / (smax - smin).max(1.0);
            for i in 0..ncols { cols[i] = minw[i] + (maxw[i] - minw[i]) * k; }
        }
        let mut cy = y + caption_h + spacing;
        let mut used_w = spacing;
        for c in cols.iter() { used_w += c + spacing; }
        for (ri, r) in grid.iter().enumerate() {
            let row_node = rows[ri];
            let row_bg = self.st[row_node].bg;
            let row_bg_idx = if row_bg >> 24 != 0 { self.emit(Cmd::Nop) } else { usize::MAX };
            let mut cx = x + spacing;
            let mut col = 0;
            let mut row_h = 0.0f32;
            let mut placed: Vec<(usize, f32, f32, usize, usize, f32)> = Vec::new();
            for &(c, span) in r.iter() {
                if col >= ncols { break; }
                let end = (col + span).min(ncols);
                let cw: f32 = cols[col..end].iter().sum::<f32>() + spacing * (end - col - 1) as f32;
                let bg = self.st[c].bg;
                let bg_idx = if bg >> 24 != 0 { self.emit(Cmd::Nop) } else { usize::MAX };
                let start = self.cmds.len();
                let pad = cellpad.unwrap_or(0.0);
                let (h, _) = self.cell(c, cx, cy, cw, pad, border_attr > 0.0);
                row_h = row_h.max(h);
                placed.push((c, cx, cw, start, bg_idx, h));
                cx += cw + spacing;
                col = end;
            }
            if let Len::Px(p) = self.st[row_node].height { row_h = row_h.max(p); }
            if self.measure == 0 {
                let n_cmds = self.cmds.len();
                for (pi, &(c, px, pw, start, bg_idx, h)) in placed.iter().enumerate() {
                    let va = self.st[c].valign;
                    let off = match va { 2 | 0 if self.st[c].valign == 2 => 0.0, 3 => row_h - h, 2 => 0.0, _ => (row_h - h) / 2.0 };
                    let end = if pi + 1 < placed.len() { placed[pi + 1].3.saturating_sub(if placed[pi + 1].4 != usize::MAX { 1 } else { 0 }) } else { n_cmds };
                    if off > 0.5 {
                        for k in start..end.min(self.cmds.len()) { self.cmds[k].shift(off as i32); }
                        for lb in self.links.iter_mut() { if lb.y >= cy as i32 && lb.x >= px as i32 && lb.x < (px + pw) as i32 { lb.y += off as i32; } }
                    }
                    if bg_idx != usize::MAX {
                        self.cmds[bg_idx] = Cmd::Rect { x: px as i32, y: cy as i32, w: pw as i32, h: row_h as i32, color: self.st[c].bg };
                    }
                    if border_attr > 0.0 && self.st[c].border_w == [0.0; 4] {
                        self.emit(Cmd::Border { x: px as i32, y: cy as i32, w: pw as i32, h: row_h as i32, bw: [1; 4], bc: [0xFFA0A0A0; 4] });
                    } else if self.st[c].border_w != [0.0; 4] {
                        let s = &self.st[c];
                        self.emit(Cmd::Border { x: px as i32, y: cy as i32, w: pw as i32, h: row_h as i32,
                            bw: [s.border_w[0] as i32, s.border_w[1] as i32, s.border_w[2] as i32, s.border_w[3] as i32], bc: s.border_c });
                    }
                }
                if row_bg_idx != usize::MAX {
                    self.cmds[row_bg_idx] = Cmd::Rect { x: (x + spacing) as i32, y: cy as i32, w: (used_w - 2.0 * spacing) as i32, h: row_h as i32, color: row_bg };
                }
            }
            cy += row_h + spacing;
        }
        (cy - y, used_w)
    }

    fn cell(&mut self, c: usize, x: f32, y: f32, w: f32, pad: f32, _border: bool) -> (f32, f32) {
        let s = &self.st[c];
        let has_pad = s.padding.iter().any(|p| p.or0(w) > 1.0);
        let p = if has_pad { 0.0 } else { pad };
        let (h, u) = {
            let bw = s.border_w;
            let saved_bg = s.bg;
            let _ = saved_bg;
            let inner_w = (w - 2.0 * p).max(1.0);
            let prev = self.enter_link(c);
            self.register_anchor(c, y);
            let pl = s.padding[3].or0(w); let pr = s.padding[1].or0(w);
            let pt = s.padding[0].or0(w); let pb = s.padding[2].or0(w);
            let (h, u) = self.flow(c, x + p + pl + bw[3], y + p + pt + bw[0], (inner_w - pl - pr - bw[1] - bw[3]).max(1.0));
            self.link = prev;
            (h + 2.0 * p + pt + pb + bw[0] + bw[2], u + 2.0 * p + pl + pr)
        };
        let mut h = h;
        if let Len::Px(ph) = self.st[c].height { h = h.max(ph); }
        (h, u)
    }

    fn flex(&mut self, id: usize, x: f32, y: f32, w: f32) -> (f32, f32) {
        let s = self.st[id].clone();
        let children: Vec<usize> = self.flat_children(id).into_iter().filter(|&c| {
            match &self.dom.nodes[c].data { NodeData::Text(t) => !t.trim().is_empty(), _ => true }
        }).collect();
        if s.flex_dir == 1 && s.grid_cols == 0 {
            let mut cy = y;
            let mut used = 0.0f32;
            for (i, &c) in children.iter().enumerate() {
                if i > 0 { cy += s.gap_row; }
                if let NodeData::Text(_) = self.dom.nodes[c].data {
                    let (h, u) = self.inline_run(&[c], x, cy, w, id); cy += h; used = used.max(u); continue;
                }
                let (h, u) = if s.align_items == 2 || s.align_items == 1 || s.align_items == 3 {
                    let iw = self.shrink_width(c, w);
                    let ox = match s.align_items { 2 => (w - iw) / 2.0, 3 => w - iw, _ => 0.0 };
                    let m = self.st[c].margin[1].or0(w) + self.st[c].margin[3].or0(w);
                    self.block(c, x + ox.max(0.0), cy, iw, Some((iw - m).max(0.0)))
                } else { self.block(c, x, cy, w, None) };
                cy += h; used = used.max(u);
            }
            return (cy - y, used);
        }
        struct It { node: usize, basis: f32, minw: f32, grow: f32, shrink: f32, text: bool }
        let mut items: Vec<It> = Vec::new();
        let grid = s.grid_cols > 0;
        let gap = if s.gap_col > 0.0 { s.gap_col } else { 0.0 };
        for &c in children.iter() {
            if let NodeData::Text(_) = self.dom.nodes[c].data {
                let (_, u) = { self.measure += 1; let r = self.inline_run(&[c], 0.0, 0.0, HUGE, id); self.measure -= 1; r };
                items.push(It { node: c, basis: u, minw: u.min(w), grow: 0.0, shrink: 1.0, text: true });
                continue;
            }
            let cs = self.st[c].clone();
            let m = cs.margin[1].or0(w) + cs.margin[3].or0(w);
            let basis = if grid {
                let n = s.grid_cols as f32;
                (w - gap * (n - 1.0)) / n
            } else if let Some(b) = cs.basis.resolve(w).filter(|_| !matches!(cs.basis, Len::Auto)) {
                let pb = if cs.border_box { 0.0 } else { cs.padding[1].or0(w) + cs.padding[3].or0(w) + cs.border_w[1] + cs.border_w[3] };
                b + pb + m
            } else { self.shrink_width(c, w) + m };
            let minw = if grid { basis } else { (self.min_content(c) + m).min(basis.max(0.0)) };
            items.push(It { node: c, basis, minw, grow: if grid { 0.0 } else { cs.grow }, shrink: cs.shrink, text: false });
        }
        let mut lines: Vec<(usize, usize)> = Vec::new();
        let mut st = 0;
        let mut acc = 0.0f32;
        for (i, it) in items.iter().enumerate() {
            let add = it.basis + if i > st { gap } else { 0.0 };
            if (s.flex_wrap || grid) && i > st && (acc + add > w + 0.5 || (grid && i - st >= s.grid_cols as usize)) {
                lines.push((st, i)); st = i; acc = it.basis;
            } else { acc += add; }
        }
        if st < items.len() { lines.push((st, items.len())); }
        let mut cy = y;
        let mut used = 0.0f32;
        let row_gap = if s.gap_row > 0.0 { s.gap_row } else { 0.0 };
        for (li, &(a, b)) in lines.iter().enumerate() {
            if li > 0 { cy += row_gap; }
            let n = b - a;
            let mut widths: Vec<f32> = items[a..b].iter().map(|i| i.basis).collect();
            let total: f32 = widths.iter().sum::<f32>() + gap * (n as f32 - 1.0);
            let free = w - total;
            if free > 0.0 {
                let g: f32 = items[a..b].iter().map(|i| i.grow).sum();
                if g > 0.0 { for (k, it) in items[a..b].iter().enumerate() { widths[k] += free * it.grow / g; } }
            } else if free < 0.0 {
                let sh: f32 = items[a..b].iter().map(|i| i.shrink * i.basis).sum();
                if sh > 0.0 {
                    for (k, it) in items[a..b].iter().enumerate() {
                        widths[k] = (widths[k] + free * it.shrink * it.basis / sh).max(it.minw);
                    }
                }
            }
            let tot: f32 = widths.iter().sum::<f32>() + gap * (n as f32 - 1.0);
            let rem = (w - tot).max(0.0);
            let (mut cx, between) = match s.justify {
                1 => (x + rem / 2.0, gap),
                2 => (x + rem, gap),
                3 => (x, if n > 1 { gap + rem / (n as f32 - 1.0) } else { gap }),
                4 => (x + rem / (2.0 * n as f32), gap + rem / n as f32),
                5 => (x + rem / (n as f32 + 1.0), gap + rem / (n as f32 + 1.0)),
                _ => (x, gap),
            };
            let mut line_h = 0.0f32;
            let mut placed: Vec<(usize, usize, f32, usize, usize)> = Vec::new();
            for (k, it) in items[a..b].iter().enumerate() {
                let iw = widths[k];
                let start = self.cmds.len();
                let lstart = self.links.len();
                let h = if it.text {
                    self.inline_run(&[it.node], cx, cy, iw, id).0
                } else {
                    let m = self.st[it.node].margin[1].or0(w) + self.st[it.node].margin[3].or0(w);
                    self.block(it.node, cx, cy, iw, Some((iw - m).max(0.0))).0
                };
                line_h = line_h.max(h);
                placed.push((start, lstart, h, it.node, 0));
                cx += iw + between;
            }
            if s.align_items == 2 || s.align_items == 3 {
                let n_cmds = self.cmds.len();
                let n_links = self.links.len();
                for (pi, &(start, lstart, h, _, _)) in placed.iter().enumerate() {
                    let off = if s.align_items == 2 { (line_h - h) / 2.0 } else { line_h - h };
                    if off < 0.5 || self.measure > 0 { continue; }
                    let end = if pi + 1 < placed.len() { placed[pi + 1].0 } else { n_cmds };
                    let lend = if pi + 1 < placed.len() { placed[pi + 1].1 } else { n_links };
                    for k in start..end { self.cmds[k].shift(off as i32); }
                    for k in lstart..lend { self.links[k].y += off as i32; }
                }
            }
            used = used.max(cx - x - between);
            cy += line_h;
        }
        let _ = Cmd::shift_x;
        (cy - y, used.max(0.0))
    }
}
