
use crate::graphics::Graphics;

pub static TEST_HTML: &[u8] = include_bytes!("html/test.html");


#[derive(Clone, Copy, PartialEq)]
pub enum NodeKind {
    Text,
    H1, H2, H3, H4, H5, H6,
    P, Br, Hr,
    B, I, U, A,
    Ul, Ol, Li,
    Input, Textarea, Button,
    Form, Center, Div, Span,
    Table, Tr, Td, Th, Font,
}

pub struct HtmlNode {
    pub kind: NodeKind,
    pub text: [u8; 256],
    pub text_len: usize,
    pub attr_value: [u8; 128],
    pub attr_len: usize,
    pub input_type: u8,
    pub color: u32,
    pub has_color: bool,
    pub center: bool,
}

impl HtmlNode {
    pub const fn new() -> Self {
        Self {
            kind: NodeKind::Text,
            text: [0; 256],
            text_len: 0,
            attr_value: [0; 128],
            attr_len: 0,
            input_type: 0,
            color: 0,
            has_color: false,
            center: false,
        }
    }
}

pub const MAX_NODES: usize = 1024;
pub const MAX_INPUTS: usize = 32;


pub struct HtmlApp {
    pub nodes: [HtmlNode; MAX_NODES],
    pub node_count: usize,

    pub input_bufs: [[u8; 128]; MAX_INPUTS],
    pub input_lens: [usize; MAX_INPUTS],
    pub input_focus: Option<usize>,

    pub input_x: [usize; MAX_INPUTS],
    pub input_y: [usize; MAX_INPUTS],
    pub input_w: [usize; MAX_INPUTS],
    pub input_h: [usize; MAX_INPUTS],

    pub scroll_y: usize,
    pub content_height: usize,

    pub btn_x: [usize; MAX_INPUTS],
    pub btn_y: [usize; MAX_INPUTS],
    pub btn_w: [usize; MAX_INPUTS],
    pub btn_h: [usize; MAX_INPUTS],
    pub btn_label: [[u8; 64]; MAX_INPUTS],
    pub btn_label_len: [usize; MAX_INPUTS],
    pub btn_count: usize,

    pub status: [u8; 128],
    pub status_len: usize,

    pub loaded: bool,

    pub arrow_up_x: usize,
    pub arrow_up_y: usize,
    pub arrow_up_w: usize,
    pub arrow_up_h: usize,
    pub arrow_dn_x: usize,
    pub arrow_dn_y: usize,
    pub arrow_dn_w: usize,
    pub arrow_dn_h: usize,
}

impl HtmlApp {
    pub const fn new() -> Self {
        Self {
            nodes: [const { HtmlNode::new() }; MAX_NODES],
            node_count: 0,
            input_bufs: [[0; 128]; MAX_INPUTS],
            input_lens: [0; MAX_INPUTS],
            input_focus: None,
            input_x: [0; MAX_INPUTS],
            input_y: [0; MAX_INPUTS],
            input_w: [0; MAX_INPUTS],
            input_h: [0; MAX_INPUTS],
            scroll_y: 0,
            content_height: 0,
            btn_x: [0; MAX_INPUTS],
            btn_y: [0; MAX_INPUTS],
            btn_w: [0; MAX_INPUTS],
            btn_h: [0; MAX_INPUTS],
            btn_label: [[0; 64]; MAX_INPUTS],
            btn_label_len: [0; MAX_INPUTS],
            btn_count: 0,
            status: [0; 128],
            status_len: 0,
            loaded: false,
            arrow_up_x: 0, arrow_up_y: 0, arrow_up_w: 0, arrow_up_h: 0,
            arrow_dn_x: 0, arrow_dn_y: 0, arrow_dn_w: 0, arrow_dn_h: 0,
        }
    }

    pub fn load(&mut self, data: &[u8]) {
        self.node_count = 0;
        self.scroll_y = 0;
        self.input_focus = None;
        self.btn_count = 0;
        self.status_len = 0;
        for i in 0..MAX_INPUTS {
            self.input_lens[i] = 0;
        }
        parse_html(data, self);
        self.loaded = true;
    }
}


fn push_text_node(app: &mut HtmlApp, kind: NodeKind, text: &[u8], color: u32, has_color: bool, center: bool) {
    if app.node_count >= MAX_NODES { return; }
    if text.is_empty() { return; }
    let mut node = HtmlNode::new();
    node.kind = kind;
    let n = text.len().min(255);
    node.text[..n].copy_from_slice(&text[..n]);
    node.text_len = n;
    node.color = color;
    node.has_color = has_color;
    node.center = center;
    app.nodes[app.node_count] = node;
    app.node_count += 1;
}

fn push_simple_node(app: &mut HtmlApp, kind: NodeKind) {
    if app.node_count >= MAX_NODES { return; }
    let mut node = HtmlNode::new();
    node.kind = kind;
    app.nodes[app.node_count] = node;
    app.node_count += 1;
}

fn decode_entities(src: &[u8]) -> ([u8; 256], usize) {
    let mut out = [0u8; 256];
    let mut o = 0;
    let mut i = 0;
    while i < src.len() && o < 255 {
        if src[i] == b'&' {
            if src[i..].starts_with(b"&amp;") { out[o] = b'&'; o += 1; i += 5; continue; }
            if src[i..].starts_with(b"&lt;")  { out[o] = b'<'; o += 1; i += 4; continue; }
            if src[i..].starts_with(b"&gt;")  { out[o] = b'>'; o += 1; i += 4; continue; }
            if src[i..].starts_with(b"&quot;"){ out[o] = b'"'; o += 1; i += 6; continue; }
            if src[i..].starts_with(b"&#39;") { out[o] = b'\''; o += 1; i += 5; continue; }
            if src[i..].starts_with(b"&nbsp;"){ out[o] = b' '; o += 1; i += 6; continue; }
        }
        out[o] = src[i];
        o += 1;
        i += 1;
    }
    (out, o)
}

fn tag_name_of(tag: &[u8]) -> &[u8] {
    let mut end = 0;
    while end < tag.len() {
        let c = tag[end];
        if c == b' ' || c == b'\t' || c == b'\r' || c == b'\n' || c == b'/' { break; }
        end += 1;
    }
    &tag[..end]
}

fn get_attr(tag: &[u8], name: &[u8]) -> Option<([u8; 128], usize)> {
    let mut i = 0;
    while i < tag.len() {
        if tag[i..].starts_with(name) {
            let after = i + name.len();
            if after < tag.len() {
                let mut j = after;
                while j < tag.len() && (tag[j] == b' ' || tag[j] == b'\t') { j += 1; }
                if j < tag.len() && tag[j] == b'=' {
                    j += 1;
                    while j < tag.len() && (tag[j] == b' ' || tag[j] == b'\t') { j += 1; }
                    if j < tag.len() {
                        let quote = tag[j];
                        let mut val = [0u8; 128];
                        let mut vlen = 0;
                        if quote == b'"' || quote == b'\'' {
                            j += 1;
                            while j < tag.len() && tag[j] != quote && vlen < 127 {
                                val[vlen] = tag[j]; vlen += 1; j += 1;
                            }
                        } else {
                            while j < tag.len() && tag[j] != b' ' && tag[j] != b'\t' && tag[j] != b'>' && vlen < 127 {
                                val[vlen] = tag[j]; vlen += 1; j += 1;
                            }
                        }
                        return Some((val, vlen));
                    }
                }
            }
        }
        i += 1;
    }
    None
}

fn parse_color(name: &[u8]) -> u32 {
    match name {
        b"red" => 0xFF0000,
        b"green" => 0x008000,
        b"blue" => 0x0000FF,
        b"black" => 0x000000,
        b"white" => 0xFFFFFF,
        b"yellow" => 0xFFFF00,
        b"purple" => 0x800080,
        b"gray" | b"grey" => 0x808080,
        b"orange" => 0xFFA500,
        b"pink" => 0xFFC0CB,
        b"cyan" | b"aqua" => 0x00FFFF,
        b"magenta" | b"fuchsia" => 0xFF00FF,
        b"lime" => 0x00FF00,
        b"navy" => 0x000080,
        b"teal" => 0x008080,
        b"maroon" => 0x800000,
        b"olive" => 0x808000,
        b"silver" => 0xC0C0C0,
        _ => 0x000000,
    }
}

fn parse_html(data: &[u8], app: &mut HtmlApp) {
    let mut i = 0;
    let mut in_script = false;
    let mut in_style = false;
    let mut list_depth: i32 = 0;
    let mut ol_counters: [usize; 8] = [0; 8];
    let mut center_mode = false;

    while i < data.len() {
        let c = data[i];
        if c == b'<' {
            if data[i..].starts_with(b"<!--") {
                let mut j = i + 4;
                while j + 2 < data.len() && !data[j..].starts_with(b"-->") { j += 1; }
                i = if j + 2 < data.len() { j + 3 } else { data.len() };
                continue;
            }

            let mut j = i + 1;
            while j < data.len() && data[j] != b'>' { j += 1; }
            if j >= data.len() { break; }

            let tag = &data[i + 1..j];
            let is_closing = !tag.is_empty() && tag[0] == b'/';
            let tag_inner = if is_closing { &tag[1..] } else { tag };
            let name = tag_name_of(tag_inner);

            if !is_closing && name == b"script" { in_script = true; i = j + 1; continue; }
            if is_closing && name == b"script" { in_script = false; i = j + 1; continue; }
            if !is_closing && name == b"style" { in_style = true; i = j + 1; continue; }
            if is_closing && name == b"style" { in_style = false; i = j + 1; continue; }
            if in_script || in_style { i = j + 1; continue; }

            if is_closing {
                match name {
                    b"h1" | b"h2" | b"h3" | b"h4" | b"h5" | b"h6" => push_simple_node(app, NodeKind::Br),
                    b"p" | b"div" => push_simple_node(app, NodeKind::Br),
                    b"ul" | b"ol" => { if list_depth > 0 { list_depth -= 1; } }
                    b"li" => push_simple_node(app, NodeKind::Br),
                    b"form" | b"table" | b"tr" => push_simple_node(app, NodeKind::Br),
                    b"center" => { center_mode = false; push_simple_node(app, NodeKind::Br); }
                    _ => {}
                }
            } else {
                match name {
                    b"h1" => push_simple_node(app, NodeKind::H1),
                    b"h2" => push_simple_node(app, NodeKind::H2),
                    b"h3" => push_simple_node(app, NodeKind::H3),
                    b"h4" => push_simple_node(app, NodeKind::H4),
                    b"h5" => push_simple_node(app, NodeKind::H5),
                    b"h6" => push_simple_node(app, NodeKind::H6),
                    b"p" => push_simple_node(app, NodeKind::P),
                    b"br" => push_simple_node(app, NodeKind::Br),
                    b"hr" => push_simple_node(app, NodeKind::Hr),
                    b"b" | b"strong" => push_simple_node(app, NodeKind::B),
                    b"i" | b"em" => push_simple_node(app, NodeKind::I),
                    b"u" => push_simple_node(app, NodeKind::U),
                    b"a" => push_simple_node(app, NodeKind::A),
                    b"ul" => { push_simple_node(app, NodeKind::Ul); list_depth += 1; }
                    b"ol" => {
                        push_simple_node(app, NodeKind::Ol);
                        if (list_depth as usize) < ol_counters.len() {
                            ol_counters[list_depth as usize] = 0;
                        }
                        list_depth += 1;
                    }
                    b"li" => {
                        if list_depth > 0 && (list_depth as usize - 1) < ol_counters.len() {
                            ol_counters[list_depth as usize - 1] += 1;
                        }
                        push_simple_node(app, NodeKind::Li);
                    }
                    b"center" => { center_mode = true; }
                    b"div" => push_simple_node(app, NodeKind::Div),
                    b"span" => push_simple_node(app, NodeKind::Span),
                    b"font" => {
                        let mut node = HtmlNode::new();
                        node.kind = NodeKind::Font;
                        if let Some((val, vlen)) = get_attr(tag, b"color") {
                            node.color = parse_color(&val[..vlen]);
                            node.has_color = true;
                        }
                        if app.node_count < MAX_NODES {
                            app.nodes[app.node_count] = node;
                            app.node_count += 1;
                        }
                    }
                    b"input" => {
                        let mut node = HtmlNode::new();
                        node.kind = NodeKind::Input;
                        let mut itype: u8 = 0;
                        if let Some((val, vlen)) = get_attr(tag, b"type") {
                            match &val[..vlen] {
                                b"password" => itype = 1,
                                b"submit" => itype = 2,
                                _ => itype = 0,
                            }
                        }
                        node.input_type = itype;
                        if let Some((val, vlen)) = get_attr(tag, b"value") {
                            let n = vlen.min(127);
                            node.attr_value[..n].copy_from_slice(&val[..n]);
                            node.attr_len = n;
                        }
                        if app.node_count < MAX_NODES {
                            app.nodes[app.node_count] = node;
                            app.node_count += 1;
                        }
                    }
                    b"textarea" => {
                        let mut node = HtmlNode::new();
                        node.kind = NodeKind::Textarea;
                        if app.node_count < MAX_NODES {
                            app.nodes[app.node_count] = node;
                            app.node_count += 1;
                        }
                    }
                    b"button" => {
                        let mut node = HtmlNode::new();
                        node.kind = NodeKind::Button;
                        if let Some((val, vlen)) = get_attr(tag, b"value") {
                            let n = vlen.min(127);
                            node.attr_value[..n].copy_from_slice(&val[..n]);
                            node.attr_len = n;
                        }
                        if app.node_count < MAX_NODES {
                            app.nodes[app.node_count] = node;
                            app.node_count += 1;
                        }
                    }
                    b"form" => push_simple_node(app, NodeKind::Form),
                    b"table" => push_simple_node(app, NodeKind::Table),
                    b"tr" => push_simple_node(app, NodeKind::Tr),
                    b"td" => push_simple_node(app, NodeKind::Td),
                    b"th" => push_simple_node(app, NodeKind::Th),
                    b"title" | b"head" | b"meta" | b"link" | b"body" | b"html" => {}
                    _ => {}
                }
            }
            i = j + 1;
        } else {
            let start = i;
            while i < data.len() && data[i] != b'<' { i += 1; }
            let raw = &data[start..i];
            let mut s = 0;
            let mut e = raw.len();
            while s < e && (raw[s] == b'\n' || raw[s] == b'\r' || raw[s] == b'\t' || raw[s] == b' ') { s += 1; }
            while e > s && (raw[e - 1] == b'\n' || raw[e - 1] == b'\r' || raw[e - 1] == b'\t' || raw[e - 1] == b' ') { e -= 1; }
            if e > s {
                let (decoded, dlen) = decode_entities(&raw[s..e]);
                push_text_node(app, NodeKind::Text, &decoded[..dlen], 0, false, center_mode);
            }
        }
    }
}


const LINE_H: usize = 16;
const FONT_W: usize = 8;

struct RenderCtx {
    cx: usize,
    cy: usize,
    w: usize,
    h: usize,
    world_y: usize,
    line_x: usize,
    line_start_x: usize,
    bold: bool,
    italic: bool,
    underline: bool,
    color: u32,
    has_color: bool,
    input_idx: usize,
    btn_idx: usize,
    list_depth: i32,
    ol_counters: [usize; 8],
    view_top: usize,
    view_bottom: usize,
    scroll: usize,
}

impl RenderCtx {
    fn new(cx: usize, cy: usize, w: usize, h: usize, scroll: usize) -> Self {
        Self {
            cx, cy, w, h,
            world_y: 10,
            line_x: cx + 10,
            line_start_x: cx + 10,
            bold: false,
            italic: false,
            underline: false,
            color: 0x000000,
            has_color: false,
            input_idx: 0,
            btn_idx: 0,
            list_depth: 0,
            ol_counters: [0; 8],
            view_top: scroll,
            view_bottom: scroll + h,
            scroll,
        }
    }

    fn newline(&mut self) {
        self.world_y += LINE_H;
        self.line_x = self.line_start_x;
    }

    fn screen_y_of(&self, world_y: usize) -> Option<usize> {
        if world_y + LINE_H <= self.view_top { return None; }
        if world_y >= self.view_bottom { return None; }
        Some(self.cy + (world_y - self.scroll))
    }
}

fn put_char(
    gfx: &mut Graphics,
    app: &mut HtmlApp,
    ctx: &mut RenderCtx,
    ch: u8,
) {
    if ctx.line_x + FONT_W > ctx.cx + ctx.w - 10 {
        ctx.newline();
    }

    if let Some(sy) = ctx.screen_y_of(ctx.world_y) {
        if ch != b' ' {
            let color = if ctx.has_color { ctx.color } else { 0x000000 };
            let bg = 0xFFFFFF;
            let ch_str = [ch];
            let s = core::str::from_utf8(&ch_str).unwrap_or(" ");
            gfx.draw_text(ctx.line_x, sy, s, color, bg);
            if ctx.bold {
                gfx.draw_text(ctx.line_x + 1, sy, s, color, bg);
            }
            if ctx.italic {
                gfx.draw_text(ctx.line_x + 1, sy + 1, s, color, bg);
            }
            if ctx.underline {
                gfx.fill_rect(ctx.line_x, sy + 12, FONT_W, 1, color);
            }
        }
    }

    ctx.line_x += FONT_W;
    if ctx.world_y + LINE_H > app.content_height {
        app.content_height = ctx.world_y + LINE_H;
    }
}

fn draw_text_run(
    gfx: &mut Graphics,
    app: &mut HtmlApp,
    ctx: &mut RenderCtx,
    text: &[u8],
    centered: bool,
) {
    if centered && !text.is_empty() {
        let total_w = text.len() * FONT_W;
        if total_w < ctx.w {
            ctx.line_start_x = ctx.cx + (ctx.w - total_w) / 2;
            ctx.line_x = ctx.line_start_x;
        }
    }
    for &ch in text {
        if ch == b'\n' {
            ctx.newline();
        } else if ch == b'\r' {
        } else if ch == b'\t' {
            for _ in 0..4 { put_char(gfx, app, ctx, b' '); }
        } else {
            put_char(gfx, app, ctx, ch);
        }
    }
    if centered {
        ctx.line_start_x = ctx.cx + 10;
        ctx.line_x = ctx.line_start_x;
    }
}

pub fn draw_html_content(gfx: &mut Graphics, cx: usize, cy: usize, w: usize, h: usize, app: &mut HtmlApp) {
    gfx.fill_rect(cx, cy, w, h, 0xFFFFFF);
    gfx.draw_rect_border(cx, cy, w, h, 0x787878);

    if !app.loaded {
        gfx.draw_text(cx + 10, cy + 10, "No HTML loaded.", 0x000000, 0xFFFFFF);
        return;
    }

    let view_h = if h > 40 { h - 40 } else { h };

    if app.content_height > view_h {
        let max_scroll = app.content_height - view_h;
        if app.scroll_y > max_scroll { app.scroll_y = max_scroll; }
    } else {
        app.scroll_y = 0;
    }

    let mut ctx = RenderCtx::new(cx, cy, w, view_h, app.scroll_y);
    app.content_height = 0;

    let mut i = 0;
    while i < app.node_count {
        let kind = app.nodes[i].kind;

        match kind {
            NodeKind::Text => {
                let mut tmp = [0u8; 256];
                let n = app.nodes[i].text_len.min(256);
                tmp[..n].copy_from_slice(&app.nodes[i].text[..n]);
                let centered = app.nodes[i].center;
                draw_text_run(gfx, app, &mut ctx, &tmp[..n], centered);
            }
            NodeKind::H1 | NodeKind::H2 | NodeKind::H3 | NodeKind::H4 | NodeKind::H5 | NodeKind::H6 => {
                let scale = match kind {
                    NodeKind::H1 => 3,
                    NodeKind::H2 => 2,
                    _ => 1,
                };
                ctx.world_y += LINE_H * (scale as usize);
                ctx.line_x = ctx.line_start_x;
                ctx.bold = true;
                if i + 1 < app.node_count && app.nodes[i + 1].kind == NodeKind::Text {
                    let mut tmp = [0u8; 256];
                    let n = app.nodes[i + 1].text_len.min(256);
                    tmp[..n].copy_from_slice(&app.nodes[i + 1].text[..n]);
                    let centered = app.nodes[i + 1].center;
                    draw_text_run(gfx, app, &mut ctx, &tmp[..n], centered);
                    i += 1;
                }
                ctx.bold = false;
                ctx.newline();
            }
            NodeKind::P => { ctx.newline(); }
            NodeKind::Br => { ctx.newline(); }
            NodeKind::Hr => {
                ctx.newline();
                if let Some(sy) = ctx.screen_y_of(ctx.world_y + 6) {
                    gfx.fill_rect(ctx.cx + 10, sy, ctx.w - 20, 1, 0x808080);
                }
                ctx.newline();
            }
            NodeKind::B => { ctx.bold = true; }
            NodeKind::I => { ctx.italic = true; }
            NodeKind::U => { ctx.underline = true; }
            NodeKind::A => {
                ctx.underline = true;
                ctx.color = 0x0000EE;
                ctx.has_color = true;
            }
            NodeKind::Font => {
                if app.nodes[i].has_color {
                    ctx.color = app.nodes[i].color;
                    ctx.has_color = true;
                }
            }
            NodeKind::Ul => {
                ctx.list_depth += 1;
                ctx.newline();
            }
            NodeKind::Ol => {
                if (ctx.list_depth as usize) < ctx.ol_counters.len() {
                    ctx.ol_counters[ctx.list_depth as usize] = 0;
                }
                ctx.list_depth += 1;
                ctx.newline();
            }
            NodeKind::Li => {
                ctx.newline();
                let indent = (ctx.list_depth.max(1) as usize) * 16;
                ctx.line_start_x = ctx.cx + 10 + indent;
                ctx.line_x = ctx.line_start_x;
                if ctx.list_depth > 0 {
                    let depth = (ctx.list_depth - 1) as usize;
                    let marker = if depth < ctx.ol_counters.len() && ctx.ol_counters[depth] > 0 {
                        num_to_str_local(ctx.ol_counters[depth])
                    } else {
                        "*"
                    };
                    if let Some(sy) = ctx.screen_y_of(ctx.world_y) {
                        gfx.draw_text(ctx.line_x, sy, marker, 0x000000, 0xFFFFFF);
                    }
                    ctx.line_start_x += 16;
                    ctx.line_x = ctx.line_start_x;
                }
            }
            NodeKind::Input => {
                let iw = 160usize;
                let ih = 20usize;
                if ctx.line_x + iw > ctx.cx + ctx.w - 10 {
                    ctx.newline();
                }
                let ix = ctx.line_x;
                let input_type = app.nodes[i].input_type;
                let world_y = ctx.world_y;

                if ctx.input_idx < MAX_INPUTS {
                    let idx = ctx.input_idx;
                    if let Some(sy) = ctx.screen_y_of(world_y) {
                        app.input_x[idx] = ix;
                        app.input_y[idx] = sy;
                        app.input_w[idx] = iw;
                        app.input_h[idx] = ih;

                        gfx.fill_rect(ix, sy, iw, ih, 0xFFFFFF);
                        gfx.draw_rect_border(ix, sy, iw, ih, 0x808080);

                        let mut shown = [0u8; 128];
                        let mut slen = 0;
                        if input_type == 1 {
                            for _ in 0..app.input_lens[idx] {
                                if slen < 127 { shown[slen] = b'*'; slen += 1; }
                            }
                        } else {
                            let n = app.input_lens[idx].min(127);
                            shown[..n].copy_from_slice(&app.input_bufs[idx][..n]);
                            slen = n;
                        }
                        let s = core::str::from_utf8(&shown[..slen]).unwrap_or("");
                        let max_chars = (iw - 8) / FONT_W;
                        let disp = if s.len() > max_chars { &s[s.len() - max_chars..] } else { s };
                        gfx.draw_text(ix + 4, sy + 6, disp, 0x000000, 0xFFFFFF);

                        if app.input_focus == Some(idx) {
                            let cur_x = ix + 4 + disp.len() * FONT_W;
                            if cur_x + 2 < ix + iw {
                                gfx.fill_rect(cur_x, sy + 4, 1, ih - 8, 0x000000);
                            }
                        }
                    } else {
                        app.input_w[idx] = 0;
                        app.input_h[idx] = 0;
                    }
                    ctx.input_idx += 1;
                }
                ctx.world_y += ih;
                ctx.line_x = ctx.line_start_x;
                ctx.newline();
            }
            NodeKind::Textarea => {
                let iw = 240usize;
                let ih = 60usize;
                if ctx.line_x + iw > ctx.cx + ctx.w - 10 {
                    ctx.newline();
                }
                let ix = ctx.line_x;
                let world_y = ctx.world_y;

                if ctx.input_idx < MAX_INPUTS {
                    let idx = ctx.input_idx;
                    if let Some(sy) = ctx.screen_y_of(world_y) {
                        app.input_x[idx] = ix;
                        app.input_y[idx] = sy;
                        app.input_w[idx] = iw;
                        app.input_h[idx] = ih;

                        gfx.fill_rect(ix, sy, iw, ih, 0xFFFFFF);
                        gfx.draw_rect_border(ix, sy, iw, ih, 0x808080);

                        let s = core::str::from_utf8(&app.input_bufs[idx][..app.input_lens[idx]]).unwrap_or("");
                        gfx.draw_text(ix + 4, sy + 6, s, 0x000000, 0xFFFFFF);

                        if app.input_focus == Some(idx) {
                            gfx.fill_rect(ix + 4 + s.len() * FONT_W, sy + 4, 1, 12, 0x000000);
                        }
                    } else {
                        app.input_w[idx] = 0;
                        app.input_h[idx] = 0;
                    }
                    ctx.input_idx += 1;
                }
                ctx.world_y += ih;
                ctx.line_x = ctx.line_start_x;
                ctx.newline();
                ctx.newline();
            }
            NodeKind::Button => {
                let mut label_tmp = [0u8; 64];
                let llen;
                if app.nodes[i].attr_len > 0 {
                    llen = app.nodes[i].attr_len.min(63);
                    label_tmp[..llen].copy_from_slice(&app.nodes[i].attr_value[..llen]);
                } else {
                    let b = b"Button";
                    llen = b.len();
                    label_tmp[..llen].copy_from_slice(b);
                }
                let label = core::str::from_utf8(&label_tmp[..llen]).unwrap_or("Button");
                let bw = label.len() * FONT_W + 20;
                let bh = 24usize;
                if ctx.line_x + bw > ctx.cx + ctx.w - 10 {
                    ctx.newline();
                }
                let bx = ctx.line_x;
                let world_y = ctx.world_y;

                if ctx.btn_idx < MAX_INPUTS {
                    let idx = ctx.btn_idx;
                    if let Some(sy) = ctx.screen_y_of(world_y) {
                        app.btn_x[idx] = bx;
                        app.btn_y[idx] = sy;
                        app.btn_w[idx] = bw;
                        app.btn_h[idx] = bh;

                        gfx.fill_rect(bx, sy, bw, bh, 0xD4D0C8);
                        gfx.draw_rect_border(bx, sy, bw, bh, 0x000000);
                        gfx.draw_text(bx + 10, sy + 6, label, 0x000000, 0xD4D0C8);
                    } else {
                        app.btn_x[idx] = 0;
                        app.btn_y[idx] = 0;
                        app.btn_w[idx] = 0;
                        app.btn_h[idx] = 0;
                    }
                    let n = label.len().min(63);
                    app.btn_label[idx][..n].copy_from_slice(&label.as_bytes()[..n]);
                    app.btn_label_len[idx] = n;
                    ctx.btn_idx += 1;
                }
                ctx.line_x += bw + 6;
            }
            NodeKind::Form => { ctx.newline(); }
            NodeKind::Center => {}
            NodeKind::Div | NodeKind::Span => {}
            NodeKind::Table => { ctx.newline(); }
            NodeKind::Tr => { ctx.newline(); }
            NodeKind::Td | NodeKind::Th => {
                if ctx.line_x > ctx.line_start_x {
                    ctx.line_x += 16;
                }
            }
        }
        i += 1;
    }

    app.btn_count = ctx.btn_idx;

    let arrow_size = 28usize;
    let arrow_y = cy + h - arrow_size - 4;
    let arrow_up_x = cx + w - arrow_size * 2 - 12;
    let arrow_dn_x = cx + w - arrow_size - 8;

    app.arrow_up_x = arrow_up_x;
    app.arrow_up_y = arrow_y;
    app.arrow_up_w = arrow_size;
    app.arrow_up_h = arrow_size;
    app.arrow_dn_x = arrow_dn_x;
    app.arrow_dn_y = arrow_y;
    app.arrow_dn_w = arrow_size;
    app.arrow_dn_h = arrow_size;

    gfx.fill_rect(arrow_up_x, arrow_y, arrow_size, arrow_size, 0xD4D0C8);
    gfx.draw_rect_border(arrow_up_x, arrow_y, arrow_size, arrow_size, 0x000000);
    for r in 0..10 {
        gfx.fill_rect(arrow_up_x + 14 - r, arrow_y + 9 + r, 1 + r * 2, 1, 0x000000);
    }

    gfx.fill_rect(arrow_dn_x, arrow_y, arrow_size, arrow_size, 0xD4D0C8);
    gfx.draw_rect_border(arrow_dn_x, arrow_y, arrow_size, arrow_size, 0x000000);
    for r in 0..10 {
        gfx.fill_rect(arrow_dn_x + 4 + r, arrow_y + 9 + r, 19 - r * 2, 1, 0x000000);
    }

    let mut info = [0u8; 64];
    let prefix = b"Scroll: ";
    let mut p = 0;
    for &b in prefix { if p < 63 { info[p] = b; p += 1; } }
    let s = num_to_str_local(app.scroll_y);
    for b in s.bytes() { if p < 63 { info[p] = b; p += 1; } }
    let mid = b" / ";
    for &b in mid { if p < 63 { info[p] = b; p += 1; } }
    let max_sc = if app.content_height > view_h { app.content_height - view_h } else { 0 };
    let s2 = num_to_str_local(max_sc);
    for b in s2.bytes() { if p < 63 { info[p] = b; p += 1; } }
    let info_str = core::str::from_utf8(&info[..p]).unwrap_or("Scroll");
    gfx.draw_text(cx + 6, arrow_y + 8, info_str, 0x606060, 0xFFFFFF);

    if app.status_len > 0 {
        let status_str = core::str::from_utf8(&app.status[..app.status_len]).unwrap_or("");
        let sx = cx + 6;
        gfx.draw_text(sx, arrow_y + 8, status_str, 0x007000, 0xFFFFFF);
    }
}

fn num_to_str_local(n: usize) -> &'static str {
    static mut BUF: [u8; 12] = [0; 12];
    unsafe {
        let mut temp = n;
        if temp == 0 {
            BUF[0] = b'0';
            BUF[1] = 0;
            return "0";
        }
        let mut digits = [0u8; 12];
        let mut i = 0;
        while temp > 0 && i < 12 {
            digits[i] = b'0' + (temp % 10) as u8;
            temp /= 10;
            i += 1;
        }
        let mut j = 0;
        while i > 0 {
            i -= 1;
            BUF[j] = digits[i];
            j += 1;
        }
        BUF[j] = 0;
        core::str::from_utf8(&BUF[..j]).unwrap_or("?")
    }
}


pub fn handle_html_click(
    app: &mut HtmlApp,
    cx: usize,
    cy: usize,
    w: usize,
    h: usize,
    mx: usize,
    my: usize,
) -> bool {
    let _ = (cx, cy, w, h);

    if app.arrow_up_w > 0
        && mx >= app.arrow_up_x && mx < app.arrow_up_x + app.arrow_up_w
        && my >= app.arrow_up_y && my < app.arrow_up_y + app.arrow_up_h
    {
        if app.scroll_y >= LINE_H * 3 {
            app.scroll_y -= LINE_H * 3;
        } else {
            app.scroll_y = 0;
        }
        return true;
    }
    if app.arrow_dn_w > 0
        && mx >= app.arrow_dn_x && mx < app.arrow_dn_x + app.arrow_dn_w
        && my >= app.arrow_dn_y && my < app.arrow_dn_y + app.arrow_dn_h
    {
        app.scroll_y = app.scroll_y.saturating_add(LINE_H * 3);
        return true;
    }

    for i in 0..MAX_INPUTS {
        let ix = app.input_x[i];
        let iy = app.input_y[i];
        let iw = app.input_w[i];
        let ih = app.input_h[i];
        if iw == 0 || ih == 0 { continue; }
        if mx >= ix && mx < ix + iw && my >= iy && my < iy + ih {
            app.input_focus = Some(i);
            return true;
        }
    }

    for i in 0..app.btn_count {
        let bx = app.btn_x[i];
        let by = app.btn_y[i];
        let bw = app.btn_w[i];
        let bh = app.btn_h[i];
        if bw == 0 || bh == 0 { continue; }
        if mx >= bx && mx < bx + bw && my >= by && my < by + bh {
            let mut label_tmp = [0u8; 64];
            let n = app.btn_label_len[i].min(63);
            label_tmp[..n].copy_from_slice(&app.btn_label[i][..n]);
            let label = core::str::from_utf8(&label_tmp[..n]).unwrap_or("Button");

            if label.eq_ignore_ascii_case("Clear") {
                for k in 0..MAX_INPUTS {
                    app.input_lens[k] = 0;
                }
                app.input_focus = None;
                set_status(app, "Form cleared");
            } else if label.eq_ignore_ascii_case("Submit") {
                let mut buf = [0u8; 128];
                let mut p = 0;
                let prefix = b"Submitted: ";
                for &b in prefix { if p < 127 { buf[p] = b; p += 1; } }
                let mut any = false;
                for k in 0..MAX_INPUTS {
                    if app.input_lens[k] == 0 { continue; }
                    if any { if p < 127 { buf[p] = b' '; p += 1; } }
                    for j in 0..app.input_lens[k].min(20) {
                        let c = app.input_bufs[k][j];
                        if c >= 0x20 && c < 0x7F {
                            if p < 127 { buf[p] = c; p += 1; }
                        }
                    }
                    any = true;
                }
                if !any {
                    let nb = b"(empty)";
                    for &b in nb { if p < 127 { buf[p] = b; p += 1; } }
                }
                app.status[..p].copy_from_slice(&buf[..p]);
                app.status_len = p;
            } else if label.eq_ignore_ascii_case("Hello") || label.eq_ignore_ascii_case("Say Hello") {
                set_status(app, "Hello from MiUX!");
            } else {
                let mut buf = [0u8; 128];
                let mut p = 0;
                let prefix = b"Clicked: ";
                for &b in prefix { if p < 127 { buf[p] = b; p += 1; } }
                for &b in label.as_bytes() { if p < 127 { buf[p] = b; p += 1; } }
                app.status[..p].copy_from_slice(&buf[..p]);
                app.status_len = p;
            }
            return true;
        }
    }

    let was_focused = app.input_focus.is_some();
    app.input_focus = None;
    was_focused
}

fn set_status(app: &mut HtmlApp, s: &str) {
    let b = s.as_bytes();
    let n = b.len().min(127);
    app.status[..n].copy_from_slice(&b[..n]);
    app.status_len = n;
}


pub fn handle_html_keyboard(app: &mut HtmlApp, scancode: u8) -> bool {
    if scancode & 0x80 != 0 {
        return false;
    }

    if scancode == 0x48 {
        if app.scroll_y >= LINE_H * 3 {
            app.scroll_y -= LINE_H * 3;
        } else {
            app.scroll_y = 0;
        }
        return true;
    }
    if scancode == 0x50 {
        app.scroll_y = app.scroll_y.saturating_add(LINE_H * 3);
        return true;
    }

    let idx = match app.input_focus {
        Some(i) => i,
        None => return false,
    };
    if idx >= MAX_INPUTS { return false; }

    let ch = match scancode {
        0x1E => Some(b'a'), 0x30 => Some(b'b'), 0x2E => Some(b'c'),
        0x20 => Some(b'd'), 0x12 => Some(b'e'), 0x21 => Some(b'f'),
        0x22 => Some(b'g'), 0x23 => Some(b'h'), 0x17 => Some(b'i'),
        0x24 => Some(b'j'), 0x25 => Some(b'k'), 0x26 => Some(b'l'),
        0x32 => Some(b'm'), 0x31 => Some(b'n'), 0x18 => Some(b'o'),
        0x19 => Some(b'p'), 0x10 => Some(b'q'), 0x13 => Some(b'r'),
        0x1F => Some(b's'), 0x14 => Some(b't'), 0x16 => Some(b'u'),
        0x2F => Some(b'v'), 0x11 => Some(b'w'), 0x2D => Some(b'x'),
        0x15 => Some(b'y'), 0x2C => Some(b'z'),
        0x39 => Some(b' '),
        0x02 => Some(b'1'), 0x03 => Some(b'2'), 0x04 => Some(b'3'),
        0x05 => Some(b'4'), 0x06 => Some(b'5'), 0x07 => Some(b'6'),
        0x08 => Some(b'7'), 0x09 => Some(b'8'), 0x0A => Some(b'9'),
        0x0B => Some(b'0'),
        0x33 => Some(b','), 0x34 => Some(b'.'), 0x35 => Some(b'/'),
        0x27 => Some(b';'), 0x28 => Some(b'\''), 0x2B => Some(b'\\'),
        0x29 => Some(b'`'), 0x0C => Some(b'-'), 0x0D => Some(b'='),
        0x1A => Some(b'['), 0x1B => Some(b']'),
        _ => None,
    };
    let ch = ch.map(|b| crate::mouse::apply_shift(b as char) as u8);

    if scancode == 0x0E {
        if app.input_lens[idx] > 0 {
            app.input_lens[idx] -= 1;
            return true;
        }
        return false;
    }

    if let Some(c) = ch {
        if app.input_lens[idx] < 127 {
            app.input_bufs[idx][app.input_lens[idx]] = c;
            app.input_lens[idx] += 1;
            return true;
        }
    }
    false
}