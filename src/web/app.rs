use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format;
use crate::graphics::Graphics;
use crate::net::http::{self, Url};
use crate::wm;
use super::{css, cascade, html, layout, paint, image, keys};
use super::dom::{Dom, NodeData, ROOT};
use super::style::{Style, Display, UA_CSS};
use super::layout::{Cmd, Field, FieldKind, Layout, Page};
use super::image::Image;

pub const HOME_URL: &str = "about:home";

const TOOLBAR_H: i32 = 34;
const STATUS_H: i32 = 20;
const SCROLL_W: i32 = 14;
const POPUP_ROW: i32 = 20;
const POPUP_ROWS: i32 = 12;

#[derive(Clone, Copy, PartialEq)]
enum Focus { Address, Page, Field(usize) }

struct Nav { url: String, post: Option<(Vec<u8>, String)>, push: bool }

struct Sizes<'a> { idx: &'a BTreeMap<String, usize>, imgs: &'a [Option<Image>] }

impl<'a> layout::ImageSizes for Sizes<'a> {
    fn size_of(&self, src: &str) -> Option<(i32, i32, usize)> {
        let i = *self.idx.get(src)?;
        let im = self.imgs.get(i)?.as_ref()?;
        Some((im.w, im.h, i))
    }
}

struct Doc {
    url: Url,
    base: Url,
    dom: Dom,
    styles: Vec<Style>,
    page: Page,
    title: String,
    bg: u32,
    imgs: Vec<Option<Image>>,
    img_idx: BTreeMap<String, usize>,
    img_queue: Vec<String>,
    fields: BTreeMap<usize, Field>,
    lw: i32,
    refresh: Option<String>,
    js: Option<super::js::Session>,
}

impl Doc {
    fn relayout(&mut self, w: i32) {
        let page = {
            let sizes = Sizes { idx: &self.img_idx, imgs: &self.imgs };
            Layout::new(&self.dom, &self.styles, &sizes, &self.fields).run(w)
        };
        self.page = page;
        self.lw = w;
    }
    fn anchor_y(&self, name: &str) -> Option<i32> {
        if name.is_empty() { return None; }
        self.page.anchors.iter().find(|a| a.0 == name).map(|a| a.1)
    }
}

pub struct Browser {
    addr: String,
    addr_fresh: bool,
    history: Vec<String>,
    hpos: usize,
    doc: Option<Doc>,
    scroll: i32,
    focus: Focus,
    status: String,
    pending: Option<Nav>,
    popup: Option<(usize, i32)>,
    rect: (i32, i32, i32, i32),
    drag_scroll: Option<i32>,
    img_timer: u64,
    kiosk: bool,
    local: String,
}

static mut BROWSER: Option<Browser> = None;

pub fn get() -> &'static mut Browser {
    unsafe {
        let b = &mut *(&raw mut BROWSER);
        if b.is_none() {
            *b = Some(Browser::blank(false));
            let br = b.as_mut().unwrap();
            br.navigate(HOME_URL, None, true);
        }
        b.as_mut().unwrap()
    }
}

pub fn open(url: &str) { get().navigate(url, None, true); }

const CP1251: [u16; 128] = [
    0x0402, 0x0403, 0x201A, 0x0453, 0x201E, 0x2026, 0x2020, 0x2021, 0x20AC, 0x2030, 0x0409, 0x2039, 0x040A, 0x040C, 0x040B, 0x040F,
    0x0452, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2013, 0x2014, 0x0020, 0x2122, 0x0459, 0x203A, 0x045A, 0x045C, 0x045B, 0x045F,
    0x00A0, 0x040E, 0x045E, 0x0408, 0x00A4, 0x0490, 0x00A6, 0x00A7, 0x0401, 0x00A9, 0x0404, 0x00AB, 0x00AC, 0x00AD, 0x00AE, 0x0407,
    0x00B0, 0x00B1, 0x0406, 0x0456, 0x0491, 0x00B5, 0x00B6, 0x00B7, 0x0451, 0x2116, 0x0454, 0x00BB, 0x0458, 0x0405, 0x0455, 0x0457,
    0x0410, 0x0411, 0x0412, 0x0413, 0x0414, 0x0415, 0x0416, 0x0417, 0x0418, 0x0419, 0x041A, 0x041B, 0x041C, 0x041D, 0x041E, 0x041F,
    0x0420, 0x0421, 0x0422, 0x0423, 0x0424, 0x0425, 0x0426, 0x0427, 0x0428, 0x0429, 0x042A, 0x042B, 0x042C, 0x042D, 0x042E, 0x042F,
    0x0430, 0x0431, 0x0432, 0x0433, 0x0434, 0x0435, 0x0436, 0x0437, 0x0438, 0x0439, 0x043A, 0x043B, 0x043C, 0x043D, 0x043E, 0x043F,
    0x0440, 0x0441, 0x0442, 0x0443, 0x0444, 0x0445, 0x0446, 0x0447, 0x0448, 0x0449, 0x044A, 0x044B, 0x044C, 0x044D, 0x044E, 0x044F,
];

fn find_charset(s: &str) -> Option<String> {
    let l = s.to_ascii_lowercase();
    let i = l.find("charset")?;
    let rest = l[i + 7..].trim_start_matches(|c: char| c == '=' || c == ' ' || c == '"' || c == '\'');
    let cs: String = rest.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').collect();
    if cs.is_empty() { None } else { Some(cs) }
}

fn decode_text(body: &[u8], content_type: &str) -> String {
    let mut cs = find_charset(content_type);
    if cs.is_none() {
        let head = &body[..body.len().min(4096)];
        let hs: String = head.iter().map(|&b| if b < 128 { b as char } else { ' ' }).collect();
        cs = find_charset(&hs);
    }
    let cs = cs.unwrap_or_else(|| String::from("utf-8"));
    let body = if body.starts_with(&[0xEF, 0xBB, 0xBF]) { &body[3..] } else { body };
    match cs.as_str() {
        "windows-1251" | "cp1251" | "win-1251" => body.iter().map(|&b| if b < 128 { b as char } else { char::from_u32(CP1251[b as usize - 128] as u32).unwrap_or('?') }).collect(),
        "iso-8859-1" | "latin1" | "windows-1252" | "us-ascii" | "ascii" => {
            match core::str::from_utf8(body) { Ok(s) => s.to_string(), Err(_) => body.iter().map(|&b| b as char).collect() }
        }
        _ => String::from_utf8_lossy(body).into_owned(),
    }
}

fn escape(s: &str) -> String { s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;") }

fn b64_decode(s: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let mut acc = 0u32;
    let mut bits = 0;
    for c in s.bytes() {
        let v = match c { b'A'..=b'Z' => c - b'A', b'a'..=b'z' => c - b'a' + 26, b'0'..=b'9' => c - b'0' + 52, b'+' | b'-' => 62, b'/' | b'_' => 63, _ => continue };
        acc = (acc << 6) | v as u32;
        bits += 6;
        if bits >= 8 { bits -= 8; out.push((acc >> bits) as u8); }
    }
    out
}

fn home_html() -> String {
    let st = crate::net::status_text();
    let ip = crate::net::ip_string();
    format!(r#"<html><head><title>Домашняя страница</title><style>
body {{ font-family: sans-serif; background: #f3f5f9; margin: 0; }}
.top {{ background: #2d5fa8; color: white; padding: 18px 28px; }}
.top h1 {{ margin: 0; font-size: 28px; }}
.box {{ background: white; border: 1px solid #d0d7e2; margin: 18px 28px; padding: 14px 20px; }}
.box h2 {{ margin-top: 0; font-size: 18px; color: #2d5fa8; }}
input[type=text] {{ width: 380px; }}
li {{ margin: 4px 0; }}
.note {{ color: #666; font-size: 13px; }}
</style></head><body>
<div class="top"><h1>MiUX Browser</h1>Встроенный браузер MiUX</div>
<div class="box"><h2>Поиск Google</h2>
<form action="https://www.google.com/search" method="get"><input type="text" name="q" placeholder="Введите запрос"> <input type="submit" value="Найти"></form>
<p class="note">Клавиша EN/RU на панели переключает раскладку. Адрес можно вводить без https://</p></div>
<div class="box"><h2>Сайты</h2><ul>
<li><a href="https://www.google.com/">google.com</a></li>
<li><a href="https://example.com/">example.com</a></li>
<li><a href="https://ru.m.wikipedia.org/">Википедия (мобильная)</a></li>
<li><a href="https://lite.duckduckgo.com/lite/">DuckDuckGo Lite</a></li>
<li><a href="http://info.cern.ch/">info.cern.ch - первый сайт в мире</a></li>
<li><a href="https://text.npr.org/">text.npr.org</a></li>
</ul></div>
<div class="box"><h2>Сеть</h2><p>Состояние: <b>{}</b></p><p>IP-адрес: {}</p>
<p class="note">Настройки подключения - в окне «Network» (значок на рабочем столе или команда <code>net</code> в терминале).</p></div>
</body></html>"#, escape(st), escape(&ip))
}

fn error_html(url: &str, msg: &str) -> String {
    format!(r#"<html><head><title>Ошибка</title></head><body style="font-family:sans-serif;margin:40px;background:#fff">
<h2 style="color:#b03030">Не удалось открыть страницу</h2><p><b>{}</b></p><p>{}</p>
<p style="color:#666">Проверьте подключение к сети (окно «Network») и адрес сайта.</p>
<p><a href="about:home">Домашняя страница</a></p></body></html>"#, escape(url), escape(msg))
}

type Prog<'a> = &'a mut dyn FnMut(&str);

fn run_scripts(dom: &mut Dom) -> Option<super::js::Session> {
    let mut ids: Vec<usize> = Vec::new();
    for (i, n) in dom.nodes.iter().enumerate() {
        if let NodeData::Element(e) = &n.data {
            if e.tag == "script" && e.attr("src").is_none() { ids.push(i); }
        }
    }
    if ids.is_empty() { return None; }
    let mut code = String::new();
    for id in ids { code.push_str(&dom.text_content(id)); code.push('\n'); }
    let mut elems: Vec<(String, String, bool)> = Vec::new();
    for i in 0..dom.nodes.len() {
        if let NodeData::Element(e) = &dom.nodes[i].data {
            if e.id.is_empty() || e.tag == "script" { continue; }
            let is_in = e.tag == "input" || e.tag == "textarea" || e.tag == "select";
            let text = if e.tag == "input" { e.attr("value").unwrap_or("").to_string() } else { dom.text_content(i) };
            elems.push((e.id.clone(), text, is_in));
        }
    }
    let (sess, out) = super::js::start(&code, elems);
    if !out.is_empty() {
        if let Some(body) = dom.find_first("body") { dom.add(body, NodeData::Text(out)); }
    }
    Some(sess)
}

fn build_doc(src: &str, url: Url, width: i32, prog: Prog) -> Doc {
    let mut dom = html::parse(src);
    let js = run_scripts(&mut dom);
    let mut base = url.clone();
    let mut refresh = None;
    let mut sheets = Vec::new();
    let env = css::MediaEnv { width };
    let mut dummy = Vec::new();
    sheets.push(css::parse_sheet(UA_CSS, &env, 0, &mut dummy));
    let mut order: u32 = 100000;
    let mut ext_loaded = 0;
    for id in 0..dom.nodes.len() {
        let e = match &dom.nodes[id].data { NodeData::Element(e) => e, _ => continue };
        match e.tag.as_str() {
            "base" => { if let Some(h) = e.attr("href") { if let Some(u) = url.join(h) { base = u; } } }
            "meta" => {
                if e.attr("http-equiv").map(|v| v.eq_ignore_ascii_case("refresh")).unwrap_or(false) {
                    if let Some(c) = e.attr("content") {
                        let delay: u32 = c.split(|ch| ch == ';' || ch == ',').next().unwrap_or("").trim().parse().unwrap_or(99);
                        let lc = c.to_ascii_lowercase();
                        if let Some(i) = lc.find("url=") {
                            let t = c[i + 4..].trim().trim_matches(|ch| ch == '\'' || ch == '"');
                            if delay <= 5 && !t.is_empty() { refresh = base.join(t).map(|u| u.to_string()); }
                        }
                    }
                }
            }
            "style" => {
                if e.attr("media").map(|m| m.contains("print") && !m.contains("screen")).unwrap_or(false) { continue; }
                let text = dom.text_content(id);
                let mut imports = Vec::new();
                sheets.push(css::parse_sheet(&text, &env, order, &mut imports));
                order += 100000;
                for imp in imports { if let Some(s) = load_css(&base, &imp, &env, order, prog, &mut ext_loaded) { sheets.push(s); order += 100000; } }
            }
            "link" => {
                let rel = e.attr("rel").unwrap_or("").to_ascii_lowercase();
                if !rel.split_whitespace().any(|r| r == "stylesheet") || rel.contains("alternate") { continue; }
                if e.attr("media").map(|m| m.contains("print") && !m.contains("screen")).unwrap_or(false) { continue; }
                if let Some(h) = e.attr("href") {
                    if let Some(s) = load_css(&base, h, &env, order, prog, &mut ext_loaded) { sheets.push(s); order += 100000; }
                }
            }
            _ => {}
        }
    }
    prog("Обработка стилей...");
    let styles = cascade::compute(&dom, &sheets);
    let title = dom.find_first("title").map(|t| dom.text_content(t).split_whitespace().collect::<Vec<_>>().join(" ")).unwrap_or_default();
    let mut bg = 0;
    if let Some(h) = dom.find_first("html") { bg = styles[h].bg; }
    if bg >> 24 == 0 { if let Some(b) = dom.find_first("body") { bg = styles[b].bg; } }
    if bg >> 24 == 0 { bg = 0xFFFFFFFF; }
    let mut img_idx = BTreeMap::new();
    let mut img_queue = Vec::new();
    for id in 0..dom.nodes.len() {
        if let NodeData::Element(e) = &dom.nodes[id].data {
            let src = match e.tag.as_str() {
                "img" => e.attr("src").or_else(|| e.attr("data-src")),
                "input" if e.attr("type").map(|t| t.eq_ignore_ascii_case("image")).unwrap_or(false) => e.attr("src"),
                _ => None,
            };
            if let Some(s) = src {
                if s.is_empty() || img_idx.contains_key(s) || img_queue.len() >= 80 { continue; }
                if styles[id].display == super::style::Display::None { continue; }
                img_idx.insert(s.to_string(), img_queue.len());
                img_queue.push(s.to_string());
            }
        }
    }
    let n = img_queue.len();
    let mut imgs = Vec::with_capacity(n);
    for _ in 0..n { imgs.push(None); }
    img_queue.reverse();
    let mut d = Doc { url, base, dom, styles, page: Page { cmds: Vec::new(), links: Vec::new(), hrefs: Vec::new(), fields: Vec::new(), anchors: Vec::new(), height: 0, width: 0 },
        title, bg, imgs, img_idx, img_queue, fields: BTreeMap::new(), lw: 0, refresh, js };
    prog("Вёрстка страницы...");
    d.relayout(width);
    let _ = ROOT;
    d
}

fn load_css(base: &Url, href: &str, env: &css::MediaEnv, order: u32, prog: Prog, count: &mut u32) -> Option<css::Sheet> {
    if *count >= 10 { return None; }
    *count += 1;
    let u = base.join(href)?;
    if u.host == "app.miux" { return None; }
    prog(&format!("Загрузка стилей {}", u.host));
    let r = http::fetch(&u, None, prog).ok()?;
    if r.status != 200 { return None; }
    let text = decode_text(&r.body, &r.content_type());
    let mut imports = Vec::new();
    let sheet = css::parse_sheet(&text, env, order, &mut imports);
    let mut rules = Vec::new();
    for imp in imports.iter().take(4) {
        if *count >= 10 { break; }
        *count += 1;
        if let Some(iu) = r.url.join(imp) {
            if let Ok(ir) = http::fetch(&iu, None, prog) {
                if ir.status == 200 {
                    let t = decode_text(&ir.body, &ir.content_type());
                    let mut dummy = Vec::new();
                    rules.extend(css::parse_sheet(&t, env, order, &mut dummy).rules);
                }
            }
        }
    }
    rules.extend(sheet.rules);
    Some(css::Sheet { rules })
}

fn ui_text(gfx: &mut Graphics, x: i32, y: i32, s: &str, color: u32) {
    if x < 0 || y < 0 { return; }
    gfx.draw_text(x as usize, y as usize, s, color, color);
}

fn button(gfx: &mut Graphics, x: i32, y: i32, w: i32, h: i32, label: &str, enabled: bool) {
    wm::draw_raised_rect(gfx, x as usize, y as usize, w as usize, h as usize);
    let tw = label.chars().count() as i32 * 8;
    let c = if enabled { wm::get_theme().button_text.to_u32() } else { 0x909090 };
    ui_text(gfx, x + (w - tw) / 2, y + (h - 16) / 2, label, c);
}

fn tri(gfx: &mut Graphics, cx: i32, cy: i32, dir: i32, color: u32) {
    let clip = (0, 0, gfx.width as i32, gfx.height as i32);
    for i in 0..6 {
        match dir {
            0 => paint::fill(gfx, cx - 3 + i, cy - i, 1, 2 * i + 1, color | 0xFF000000, clip),
            1 => paint::fill(gfx, cx + 3 - i, cy - i, 1, 2 * i + 1, color | 0xFF000000, clip),
            2 => paint::fill(gfx, cx - i, cy - 3 + i, 2 * i + 1, 1, color | 0xFF000000, clip),
            _ => paint::fill(gfx, cx - i, cy + 3 - i, 2 * i + 1, 1, color | 0xFF000000, clip),
        }
    }
}

struct Ui { back: (i32, i32, i32, i32), fwd: (i32, i32, i32, i32), reload: (i32, i32, i32, i32), home: (i32, i32, i32, i32),
    addr: (i32, i32, i32, i32), go: (i32, i32, i32, i32), lang: (i32, i32, i32, i32), view: (i32, i32, i32, i32), sbar: (i32, i32, i32, i32), status: (i32, i32, i32, i32) }

fn inside(r: (i32, i32, i32, i32), x: i32, y: i32) -> bool { x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3 }

impl Browser {
    fn blank(kiosk: bool) -> Browser {
        Browser {
            addr: if kiosk { String::new() } else { String::from(HOME_URL) }, addr_fresh: true, history: Vec::new(), hpos: 0, doc: None, scroll: 0,
            focus: if kiosk { Focus::Page } else { Focus::Address }, status: String::from("Готово"), pending: None, popup: None, rect: (0, 0, 0, 0),
            drag_scroll: None, img_timer: 0, kiosk, local: String::new(),
        }
    }

    fn ui(&self) -> Ui {
        let (x, y, w, h) = self.rect;
        let by = y + 5;
        let bh = 24;
        let back = (x + 6, by, 28, bh);
        let fwd = (x + 36, by, 28, bh);
        let reload = (x + 66, by, 28, bh);
        let home = (x + 96, by, 28, bh);
        let lang = (x + w - 40, by, 34, bh);
        let go = (x + w - 84, by, 40, bh);
        let addr = (x + 130, by, (w - 130 - 90).max(40), bh);
        let th = if self.kiosk { 0 } else { TOOLBAR_H };
        let sh = if self.kiosk { 0 } else { STATUS_H };
        let view = (x, y + th, w - SCROLL_W, h - th - sh);
        let sbar = (x + w - SCROLL_W, y + th, SCROLL_W, h - th - sh);
        let status = (x, y + h - sh, w, sh);
        Ui { back, fwd, reload, home, addr, go, lang, view, sbar, status }
    }

    fn max_scroll(&self) -> i32 {
        let vh = self.ui().view.3;
        self.doc.as_ref().map(|d| (d.page.height - vh).max(0)).unwrap_or(0)
    }

    fn clamp_scroll(&mut self) {
        let m = self.max_scroll();
        if self.scroll > m { self.scroll = m; }
        if self.scroll < 0 { self.scroll = 0; }
    }

    pub fn draw(&mut self, gfx: &mut Graphics, x: usize, y: usize, w: usize, h: usize) {
        self.rect = (x as i32, y as i32, w as i32, h as i32);
        let ui = self.ui();
        if !self.kiosk {
            let t = wm::get_theme();
            gfx.fill_rect(x, y, w, TOOLBAR_H as usize, t.window_bg.to_u32());
            let can_back = self.hpos > 0;
            let can_fwd = self.hpos + 1 < self.history.len();
            let tc = t.button_text.to_u32();
            button(gfx, ui.back.0, ui.back.1, ui.back.2, ui.back.3, "", can_back);
            tri(gfx, ui.back.0 + 13, ui.back.1 + 12, 0, if can_back { tc } else { 0x909090 });
            button(gfx, ui.fwd.0, ui.fwd.1, ui.fwd.2, ui.fwd.3, "", can_fwd);
            tri(gfx, ui.fwd.0 + 15, ui.fwd.1 + 12, 1, if can_fwd { tc } else { 0x909090 });
            button(gfx, ui.reload.0, ui.reload.1, ui.reload.2, ui.reload.3, if self.pending.is_some() { "x" } else { "R" }, true);
            button(gfx, ui.home.0, ui.home.1, ui.home.2, ui.home.3, "H", true);
            button(gfx, ui.go.0, ui.go.1, ui.go.2, ui.go.3, "Go", true);
            button(gfx, ui.lang.0, ui.lang.1, ui.lang.2, ui.lang.3, keys::layout_name(), true);
            let a = ui.addr;
            wm::draw_sunken_rect(gfx, a.0 as usize, a.1 as usize, a.2 as usize, a.3 as usize);
            gfx.fill_rect(a.0 as usize + 2, a.1 as usize + 2, (a.2 - 4).max(0) as usize, (a.3 - 4).max(0) as usize, 0xFFFFFF);
            let maxc = ((a.2 - 16) / 8).max(1) as usize;
            let chars: Vec<char> = self.addr.chars().collect();
            let start = chars.len().saturating_sub(maxc);
            let shown: String = chars[start..].iter().collect();
            let focused = self.focus == Focus::Address;
            if focused && self.addr_fresh && !shown.is_empty() {
                gfx.fill_rect(a.0 as usize + 5, a.1 as usize + 4, shown.chars().count() * 8, 16, 0x3B78D8);
                ui_text(gfx, a.0 + 6, a.1 + 4, &shown, 0xFFFFFF);
            } else {
                ui_text(gfx, a.0 + 6, a.1 + 4, &shown, 0x000000);
            }
            if focused && !self.addr_fresh {
                let cxp = a.0 + 6 + shown.chars().count() as i32 * 8;
                gfx.fill_rect(cxp as usize, a.1 as usize + 4, 1, 16, 0x000000);
            }
        }
        let v = ui.view;
        let clip = (v.0, v.1, v.0 + v.2, v.1 + v.3);
        self.clamp_scroll();
        let focus_field = match self.focus { Focus::Field(i) => Some(i), _ => None };
        if let Some(d) = self.doc.as_mut() {
            if d.lw != v.2 && v.2 > 50 { d.relayout(v.2); }
        }
        self.clamp_scroll();
        match self.doc.as_ref() {
            Some(d) => {
                paint::fill(gfx, v.0, v.1, v.2, v.3, d.bg | 0xFF000000, clip);
                paint::paint(gfx, &d.page, &d.imgs, clip, self.scroll, focus_field, true);
            }
            None => paint::fill(gfx, v.0, v.1, v.2, v.3, 0xFFFFFFFF, clip),
        }
        if let Some((fi, ps)) = self.popup { self.draw_popup(gfx, fi, ps, clip); }
        let s = ui.sbar;
        gfx.fill_rect(s.0 as usize, s.1 as usize, s.2 as usize, s.3.max(0) as usize, 0xE4E4E4);
        let total = self.doc.as_ref().map(|d| d.page.height).unwrap_or(0).max(1);
        if total > v.3 && s.3 > 20 {
            let th = ((s.3 as i64 * v.3 as i64 / total as i64) as i32).max(24).min(s.3);
            let ty = s.1 + ((s.3 - th) as i64 * self.scroll as i64 / (total - v.3).max(1) as i64) as i32;
            wm::draw_raised_rect(gfx, s.0 as usize + 1, ty as usize, (s.2 - 2) as usize, th as usize);
        }
        self.draw_status(gfx);
    }

    fn draw_status(&self, gfx: &mut Graphics) {
        let st = self.ui().status;
        draw_status_bar(gfx, st, &self.status_line());
    }

    fn status_line(&self) -> String {
        if self.pending.is_some() { return String::from("Загрузка..."); }
        let mut s = self.status.clone();
        if let Some(d) = self.doc.as_ref() {
            if !d.img_queue.is_empty() { s = format!("Загрузка картинок: осталось {}", d.img_queue.len()); }
            else if !d.title.is_empty() && self.status == "Готово" { s = format!("Готово - {}", d.title); }
        }
        s
    }

    fn popup_rect(&self, fi: usize) -> Option<(i32, i32, i32, i32)> {
        let d = self.doc.as_ref()?;
        let v = self.ui().view;
        for c in d.page.cmds.iter() {
            if let Cmd::Widget { x, y, w, h, field } = c {
                if *field == fi {
                    let n = d.page.fields[fi].options.len().max(1) as i32;
                    let rows = n.min(POPUP_ROWS);
                    let ph = rows * POPUP_ROW + 2;
                    let sx = v.0 + x;
                    let mut sy = v.1 + y + h - self.scroll;
                    if sy + ph > v.1 + v.3 { sy = (v.1 + y - self.scroll - ph).max(v.1); }
                    let pw = (*w).max(d.page.fields[fi].options.iter().map(|o| super::super::font::web_font(13.0, false, false).text_width(&o.1)).max().unwrap_or(0) + 16);
                    return Some((sx, sy, pw, ph));
                }
            }
        }
        None
    }

    fn draw_popup(&self, gfx: &mut Graphics, fi: usize, ps: i32, clip: (i32, i32, i32, i32)) {
        let r = match self.popup_rect(fi) { Some(r) => r, None => return };
        let d = self.doc.as_ref().unwrap();
        let f = &d.page.fields[fi];
        let full = (0, 0, gfx.width as i32, gfx.height as i32);
        paint::fill(gfx, r.0, r.1, r.2, r.3, 0xFFFFFFFF, full);
        paint::fill(gfx, r.0, r.1, r.2, 1, 0xFF707070, full);
        paint::fill(gfx, r.0, r.1 + r.3 - 1, r.2, 1, 0xFF707070, full);
        paint::fill(gfx, r.0, r.1, 1, r.3, 0xFF707070, full);
        paint::fill(gfx, r.0 + r.2 - 1, r.1, 1, r.3, 0xFF707070, full);
        let fnt = super::super::font::web_font(13.0, false, false);
        for i in 0..POPUP_ROWS {
            let oi = (ps + i) as usize;
            if oi >= f.options.len() { break; }
            let ry = r.1 + 1 + i * POPUP_ROW;
            if oi == f.selected { paint::fill(gfx, r.0 + 1, ry, r.2 - 2, POPUP_ROW, 0xFF3B78D8, full); }
            let c = if oi == f.selected { 0xFFFFFF } else { 0x202020 };
            fnt.draw(gfx, r.0 + 6, ry + (POPUP_ROW - fnt.line_height()) / 2, &f.options[oi].1, c, (r.0, ry, r.0 + r.2 - 2, ry + POPUP_ROW));
        }
        let _ = clip;
    }

    fn navigate(&mut self, url: &str, post: Option<(Vec<u8>, String)>, push: bool) {
        let url = url.trim();
        if url.is_empty() { return; }
        if post.is_none() {
            if let (Some(d), Some(u)) = (self.doc.as_ref(), Url::parse(url)) {
                let mut a = d.url.clone(); a.fragment.clear();
                let mut b = u.clone(); b.fragment.clear();
                if a.to_string() == b.to_string() && !u.fragment.is_empty() {
                    if let Some(y) = d.anchor_y(&u.fragment) {
                        self.scroll = y;
                        self.clamp_scroll();
                        self.addr = u.to_string();
                        if push { self.push_history(&self.addr.clone()); }
                        return;
                    }
                }
            }
        }
        if self.kiosk { return; }
        self.popup = None;
        self.pending = Some(Nav { url: url.to_string(), post, push });
        self.addr = url.to_string();
        self.addr_fresh = true;
        if self.focus == Focus::Address { self.focus = Focus::Page; }
    }

    fn push_history(&mut self, url: &str) {
        if self.history.get(self.hpos).map(|h| h == url).unwrap_or(false) { return; }
        if !self.history.is_empty() { self.history.truncate(self.hpos + 1); }
        self.history.push(url.to_string());
        self.hpos = self.history.len() - 1;
    }

    fn typed_url(&self, s: &str) -> String {
        let s = s.trim();
        let l = s.to_ascii_lowercase();
        if l.starts_with("http://") || l.starts_with("https://") || l.starts_with("about:") { return s.to_string(); }
        if !s.contains(' ') && (s.contains('.') || s.starts_with("localhost") || s.contains(':')) {
            return format!("https://{}", s);
        }
        format!("https://www.google.com/search?q={}", http::url_encode(s))
    }

    fn do_load(&mut self, gfx: &mut Graphics, nav: Nav) {
        let st_rect = self.ui().status;
        let view_w = self.ui().view.2.max(200);
        let mut last = 0u64;
        let mut prog = |msg: &str| {
            if st_rect.3 <= 0 { return; }
            let now = crate::timer::millis();
            if now - last < 80 { return; }
            last = now;
            draw_status_bar(gfx_ptr(), st_rect, msg);
            flush_rect(st_rect);
        };
        let _ = gfx;
        let url_s = nav.url.clone();
        let (html_src, final_url) = if self.kiosk {
            (self.local.clone(), Url::parse("http://app.miux/").unwrap())
        } else if url_s.eq_ignore_ascii_case(HOME_URL) || url_s.eq_ignore_ascii_case("about:blank") {
            (if url_s.eq_ignore_ascii_case(HOME_URL) { home_html() } else { String::new() }, Url::parse("http://home.miux/").unwrap())
        } else {
            match Url::parse(&url_s) {
                None => (error_html(&url_s, "Неверный адрес"), Url::parse("http://error.miux/").unwrap()),
                Some(u) => {
                    let post = nav.post.as_ref().map(|(b, t)| (&b[..], t.as_str()));
                    let mut res = http::fetch(&u, post, &mut prog);
                    if res.is_err() && u.https && post.is_none() && nav.push && u.port == 443 {
                        let mut hu = u.clone(); hu.https = false; hu.port = 80;
                        if let Ok(r) = http::fetch(&hu, None, &mut prog) { res = Ok(r); }
                    }
                    match res {
                        Err(e) => (error_html(&u.to_string(), &e), u),
                        Ok(r) => {
                            let ct = r.content_type();
                            let fu = r.url.clone();
                            if ct.starts_with("image/") {
                                (format!("<html><body style=\"margin:8px;background:#222\"><img src=\"{}\"></body></html>", escape(&fu.to_string())), fu)
                            } else if ct.contains("html") || ct.contains("xml") {
                                (decode_text(&r.body, &ct), fu)
                            } else if ct.starts_with("text/") || ct.contains("json") || ct.contains("javascript") {
                                (format!("<html><body><pre>{}</pre></body></html>", escape(&decode_text(&r.body, &ct))), fu)
                            } else {
                                (format!("<html><body style=\"font-family:sans-serif;margin:30px\"><h3>Файл нельзя показать</h3><p>Тип: {}</p><p>Размер: {} байт</p></body></html>", escape(&ct), r.body.len()), fu)
                            }
                        }
                    }
                }
            }
        };
        let mut doc = build_doc(&html_src, final_url.clone(), view_w, &mut prog);
        let shown_url = if url_s.to_ascii_lowercase().starts_with("about:") { url_s.clone() } else { final_url.to_string() };
        self.scroll = doc.anchor_y(&final_url.fragment).unwrap_or(0);
        if let Some(r) = doc.refresh.take() {
            if !self.kiosk && r != shown_url { self.pending = Some(Nav { url: r, post: None, push: false }); }
        }
        self.doc = Some(doc);
        self.addr = shown_url.clone();
        self.addr_fresh = true;
        if nav.push { self.push_history(&shown_url); } else if let Some(h) = self.history.get_mut(self.hpos) { *h = shown_url.clone(); }
        self.status = String::from("Готово");
        self.focus = if self.focus == Focus::Address && shown_url != HOME_URL { Focus::Page } else { self.focus };
        if let Focus::Field(_) = self.focus { self.focus = Focus::Page; }
        self.img_timer = crate::timer::millis();
    }

    fn load_next_image(&mut self) -> bool {
        let d = match self.doc.as_mut() { Some(d) => d, None => return false };
        let src = match d.img_queue.pop() { Some(s) => s, None => return false };
        let idx = match d.img_idx.get(&src) { Some(&i) => i, None => return true };
        let data = if src.starts_with("data:") {
            match src.find(',') { Some(c) if src[..c].contains("base64") => Some(b64_decode(&src[c + 1..])), _ => None }
        } else if let Some(u) = d.base.join(&src).filter(|u| u.host != "app.miux") {
            let mut nop = |_: &str| {};
            match http::fetch(&u, None, &mut nop) { Ok(r) if r.status == 200 => Some(r.body), _ => None }
        } else { None };
        if let Some(bytes) = data {
            d.imgs[idx] = image::decode(&bytes);
        }
        if d.imgs[idx].is_some() {
            let w = d.lw;
            d.relayout(w);
        }
        true
    }

    fn submit(&mut self, fi: usize) {
        let d = match self.doc.as_ref() { Some(d) => d, None => return };
        let sub = &d.page.fields[fi];
        let form = sub.form;
        let (action, method) = match d.dom.elem(form) {
            Some(e) if form != usize::MAX => (e.attr("action").unwrap_or("").to_string(), e.attr("method").unwrap_or("get").to_ascii_lowercase()),
            _ => (String::new(), String::from("get")),
        };
        let mut q = String::new();
        for (i, f) in d.page.fields.iter().enumerate() {
            if f.form != form || f.name.is_empty() { continue; }
            let val = match f.kind {
                FieldKind::Text | FieldKind::Password | FieldKind::TextArea | FieldKind::Select => Some(f.value.clone()),
                FieldKind::Checkbox | FieldKind::Radio => if f.checked { Some(f.value.clone()) } else { None },
                FieldKind::Submit => if i == fi { Some(if f.value.is_empty() { f.label.clone() } else { f.value.clone() }) } else { None },
                FieldKind::Image => {
                    if i == fi {
                        if !q.is_empty() { q.push('&'); }
                        q += &format!("{}.x=1&{}.y=1", http::url_encode(&f.name), http::url_encode(&f.name));
                    }
                    None
                }
                _ => None,
            };
            if let Some(v) = val {
                if !q.is_empty() { q.push('&'); }
                q += &http::url_encode(&f.name);
                q.push('=');
                q += &http::url_encode(&v);
            }
        }
        let target = match d.base.join(&action) { Some(u) => u, None => d.url.clone() };
        if method == "post" {
            let u = target.to_string();
            self.navigate(&u, Some((q.into_bytes(), String::from("application/x-www-form-urlencoded"))), true);
        } else {
            let mut u = target;
            u.fragment.clear();
            if let Some(i) = u.path.find('?') { u.path.truncate(i); }
            u.path.push('?');
            u.path += &q;
            let s = u.to_string();
            self.navigate(&s, None, true);
        }
    }

    fn set_field(&mut self, fi: usize, f: Field) {
        if let Some(d) = self.doc.as_mut() {
            d.fields.insert(f.node, f.clone());
            d.page.fields[fi] = f;
        }
    }

    fn go_history(&mut self, delta: i32) {
        let np = self.hpos as i32 + delta;
        if np < 0 || np as usize >= self.history.len() { return; }
        self.hpos = np as usize;
        let u = self.history[self.hpos].clone();
        self.navigate(&u, None, false);
    }

    pub fn key(&mut self, sc: u8) -> bool {
        if self.pending.is_some() { return false; }
        if let Some(e) = keys::ext(sc) {
            if let Some((fi, ps)) = self.popup {
                let n = self.doc.as_ref().map(|d| d.page.fields[fi].options.len()).unwrap_or(0);
                if let Some(d) = self.doc.as_ref() {
                    let mut f = d.page.fields[fi].clone();
                    if e == keys::UP && f.selected > 0 { f.selected -= 1; }
                    if e == keys::DOWN && f.selected + 1 < n { f.selected += 1; }
                    f.value = f.options.get(f.selected).map(|o| o.0.clone()).unwrap_or_default();
                    let sel = f.selected as i32;
                    self.set_field(fi, f);
                    let mut ps = ps;
                    if sel < ps { ps = sel; }
                    if sel >= ps + POPUP_ROWS { ps = sel - POPUP_ROWS + 1; }
                    self.popup = Some((fi, ps));
                }
                return true;
            }
            let vh = self.ui().view.3;
            match e {
                keys::UP => self.scroll -= 40,
                keys::DOWN => self.scroll += 40,
                keys::PGUP => self.scroll -= vh - 40,
                keys::PGDN => self.scroll += vh - 40,
                keys::HOME if self.focus == Focus::Page => self.scroll = 0,
                keys::END if self.focus == Focus::Page => self.scroll = self.max_scroll(),
                keys::LEFT if self.focus == Focus::Page => { self.go_history(-1); }
                _ => return false,
            }
            self.clamp_scroll();
            return true;
        }
        if sc == keys::F5 { self.reload(); return true; }
        match self.focus {
            Focus::Address => {
                if sc == keys::ENTER {
                    let u = self.typed_url(&self.addr.clone());
                    self.navigate(&u, None, true);
                } else if sc == keys::BACKSPACE {
                    if self.addr_fresh { self.addr.clear(); } else { self.addr.pop(); }
                    self.addr_fresh = false;
                } else if sc == keys::ESC {
                    self.addr = self.doc.as_ref().map(|d| d.url.to_string()).unwrap_or_default();
                    self.addr_fresh = true;
                    self.focus = Focus::Page;
                } else if let Some(c) = keys::char_for(sc) {
                    if self.addr_fresh { self.addr.clear(); }
                    self.addr_fresh = false;
                    if self.addr.len() < 2000 { self.addr.push(c); }
                } else { return false; }
                true
            }
            Focus::Field(fi) => {
                let f = match self.doc.as_ref().and_then(|d| d.page.fields.get(fi)) { Some(f) => f.clone(), None => { self.focus = Focus::Page; return true; } };
                let mut f = f;
                match f.kind {
                    FieldKind::Text | FieldKind::Password | FieldKind::TextArea => {
                        if sc == keys::ENTER {
                            if f.kind == FieldKind::TextArea { f.value.push('\n'); self.set_field(fi, f); }
                            else { self.submit(fi); }
                        } else if sc == keys::BACKSPACE { f.value.pop(); self.set_field(fi, f); }
                        else if sc == keys::TAB { self.focus_next(fi); }
                        else if sc == keys::ESC { self.focus = Focus::Page; }
                        else if let Some(c) = keys::char_for(sc) { if f.value.len() < 4000 { f.value.push(c); } self.set_field(fi, f); }
                        else { return false; }
                    }
                    FieldKind::Checkbox | FieldKind::Radio => {
                        if sc == 0x39 { self.activate(fi); } else if sc == keys::TAB { self.focus_next(fi); } else if sc == keys::ENTER { self.submit(fi); } else { return false; }
                    }
                    _ => {
                        if sc == keys::ENTER || sc == 0x39 { self.activate(fi); } else if sc == keys::TAB { self.focus_next(fi); } else { return false; }
                    }
                }
                true
            }
            Focus::Page => {
                let vh = self.ui().view.3;
                if sc == 0x39 { self.scroll += if crate::mouse::shift_pressed() { -(vh - 40) } else { vh - 40 }; self.clamp_scroll(); return true; }
                if sc == keys::BACKSPACE { self.go_history(-1); return true; }
                if sc == keys::TAB {
                    if let Some(d) = self.doc.as_ref() {
                        if let Some(i) = d.page.fields.iter().position(|f| matches!(f.kind, FieldKind::Text | FieldKind::Password | FieldKind::TextArea)) {
                            self.focus = Focus::Field(i);
                            return true;
                        }
                    }
                    if self.kiosk { return true; }
                    self.focus = Focus::Address;
                    self.addr_fresh = true;
                    return true;
                }
                false
            }
        }
    }

    fn focus_next(&mut self, fi: usize) {
        if let Some(d) = self.doc.as_ref() {
            let n = d.page.fields.len();
            for k in 1..=n {
                let j = (fi + k) % n;
                if d.page.fields[j].kind != FieldKind::Button { self.focus = Focus::Field(j); return; }
            }
        }
    }

    fn reload(&mut self) {
        if let Some(u) = self.history.get(self.hpos).cloned() { self.navigate(&u, None, false); }
    }

    fn activate(&mut self, fi: usize) {
        let f = match self.doc.as_ref().and_then(|d| d.page.fields.get(fi)) { Some(f) => f.clone(), None => return };
        self.focus = Focus::Field(fi);
        if self.kiosk && matches!(f.kind, FieldKind::Button | FieldKind::Submit | FieldKind::Image) {
            let name = self.doc.as_ref()
                .and_then(|d| d.dom.elem(f.node))
                .and_then(|e| e.attr("data-launch"))
                .map(|v| v.trim().to_ascii_lowercase());
            if let Some(n) = name {
                if !n.is_empty() {
                    unsafe { *(&raw mut APP_LAUNCH) = n; }
                    return;
                }
            }
        }
        let f_node = f.node;
        let is_toggle = matches!(f.kind, FieldKind::Checkbox | FieldKind::Radio);
        if !is_toggle && matches!(f.kind, FieldKind::Button | FieldKind::Submit | FieldKind::Image | FieldKind::Reset) {
            let code = self.doc.as_ref().and_then(|d| d.dom.elem(f_node)).and_then(|e| e.attr("onclick")).unwrap_or("").to_string();
            self.run_js(&code, f_node);
        }
        match f.kind {
            FieldKind::Checkbox => { let mut f = f; f.checked = !f.checked; self.set_field(fi, f); }
            FieldKind::Radio => {
                let d = self.doc.as_ref().unwrap();
                let group: Vec<usize> = d.page.fields.iter().enumerate()
                    .filter(|(_, g)| g.kind == FieldKind::Radio && g.name == f.name && g.form == f.form).map(|(i, _)| i).collect();
                for i in group {
                    let mut g = self.doc.as_ref().unwrap().page.fields[i].clone();
                    g.checked = i == fi;
                    self.set_field(i, g);
                }
            }
            FieldKind::Submit | FieldKind::Image => self.submit(fi),
            FieldKind::Reset => {
                if let Some(d) = self.doc.as_mut() {
                    let form = f.form;
                    let nodes: Vec<usize> = d.page.fields.iter().filter(|g| g.form == form).map(|g| g.node).collect();
                    for n in nodes { d.fields.remove(&n); }
                    let w = d.lw;
                    d.relayout(w);
                }
            }
            FieldKind::Select => {
                let ps = (f.selected as i32 - POPUP_ROWS / 2).max(0).min((f.options.len() as i32 - POPUP_ROWS).max(0));
                self.popup = Some((fi, ps));
            }
            _ => {}
        }
        if is_toggle {
            let code = self.doc.as_ref().and_then(|d| d.dom.elem(f_node)).and_then(|e| e.attr("onclick")).unwrap_or("").to_string();
            self.run_js(&code, f_node);
        }
    }

    fn run_js(&mut self, code: &str, node: usize) {
        let (id, inputs) = {
            let d = match self.doc.as_ref() { Some(d) => d, None => return };
            if d.js.is_none() { return; }
            let id = if node < d.dom.nodes.len() { d.dom.elem(node).map(|e| e.id.clone()).unwrap_or_default() } else { String::new() };
            let mut inputs: Vec<(String, String)> = Vec::new();
            for f in d.page.fields.iter() {
                if matches!(f.kind, FieldKind::Text | FieldKind::Password | FieldKind::TextArea | FieldKind::Select) {
                    if let Some(en) = d.dom.elem(f.node) {
                        if !en.id.is_empty() { inputs.push((en.id.clone(), f.value.clone())); }
                    }
                }
            }
            (id, inputs)
        };
        if code.trim().is_empty() && id.is_empty() { return; }
        let (out, changes) = {
            let d = self.doc.as_mut().unwrap();
            match d.js.as_mut() { Some(j) => j.dispatch(code, &id, &inputs), None => return }
        };
        if let Some(line) = out.lines().map(|l| l.trim()).find(|l| !l.is_empty()) {
            self.status = String::from(line);
        }
        if changes.is_empty() { return; }
        if let Some(d) = self.doc.as_mut() {
            for c in changes {
                let nid = (0..d.dom.nodes.len()).find(|&i| d.dom.elem(i).map(|e| e.id == c.id).unwrap_or(false));
                let nid = match nid { Some(n) => n, None => continue };
                if c.is_value {
                    if let Some(mut f) = d.page.fields.iter().find(|f| f.node == nid).cloned() {
                        f.value = c.text.clone();
                        d.fields.insert(nid, f);
                    }
                } else {
                    let kids = d.dom.nodes[nid].children.clone();
                    let mut first: Option<usize> = None;
                    for &k in kids.iter() {
                        if let NodeData::Text(t) = &mut d.dom.nodes[k].data {
                            if first.is_none() { *t = c.text.clone(); first = Some(k); } else { t.clear(); }
                        }
                    }
                    if first.is_none() {
                        d.dom.add(nid, NodeData::Text(c.text.clone()));
                        let mut st = Style::inherit_pub(&d.styles[nid]);
                        st.display = Display::Inline;
                        d.styles.push(st);
                    }
                }
            }
            let w = d.lw;
            d.relayout(w);
        }
        self.clamp_scroll();
    }

    pub fn click(&mut self, mx: usize, my: usize) -> bool {
        if self.pending.is_some() { return false; }
        let (mx, my) = (mx as i32, my as i32);
        let ui = self.ui();
        if !inside((self.rect.0, self.rect.1, self.rect.2, self.rect.3), mx, my) { return false; }
        if let Some((fi, ps)) = self.popup {
            if let Some(r) = self.popup_rect(fi) {
                if inside(r, mx, my) {
                    let oi = (ps + (my - r.1 - 1) / POPUP_ROW) as usize;
                    if let Some(mut f) = self.doc.as_ref().map(|d| d.page.fields[fi].clone()) {
                        if oi < f.options.len() {
                            f.selected = oi;
                            f.value = f.options[oi].0.clone();
                            self.set_field(fi, f);
                        }
                    }
                }
            }
            self.popup = None;
            return true;
        }
        if !self.kiosk && inside(ui.back, mx, my) { self.go_history(-1); return true; }
        if !self.kiosk && inside(ui.fwd, mx, my) { self.go_history(1); return true; }
        if !self.kiosk && inside(ui.reload, mx, my) { self.reload(); return true; }
        if !self.kiosk && inside(ui.home, mx, my) { self.navigate(HOME_URL, None, true); return true; }
        if !self.kiosk && inside(ui.lang, mx, my) { keys::toggle_layout(); return true; }
        if !self.kiosk && inside(ui.go, mx, my) { let u = self.typed_url(&self.addr.clone()); self.navigate(&u, None, true); return true; }
        if !self.kiosk && inside(ui.addr, mx, my) { self.addr_fresh = self.focus != Focus::Address; self.focus = Focus::Address; return true; }
        if inside(ui.sbar, mx, my) {
            let total = self.doc.as_ref().map(|d| d.page.height).unwrap_or(0);
            if total > ui.view.3 {
                let rel = (my - ui.sbar.1) as i64 * total as i64 / ui.sbar.3.max(1) as i64;
                self.scroll = rel as i32 - ui.view.3 / 2;
                self.clamp_scroll();
            }
            return true;
        }
        if inside(ui.view, mx, my) {
            let px = mx - ui.view.0;
            let py = my - ui.view.1 + self.scroll;
            let mut hit_field = None;
            let mut href = None;
            if let Some(d) = self.doc.as_ref() {
                for c in d.page.cmds.iter().rev() {
                    if let Cmd::Widget { x, y, w, h, field } = c {
                        if px >= *x && px < x + w && py >= *y && py < y + h { hit_field = Some(*field); break; }
                    }
                }
                if hit_field.is_none() {
                    for l in d.page.links.iter().rev() {
                        if px >= l.x && px < l.x + l.w && py >= l.y && py < l.y + l.h { href = d.page.hrefs.get(l.link).cloned(); break; }
                    }
                }
            }
            if let Some(fi) = hit_field { self.activate(fi); return true; }
            self.focus = Focus::Page;
            if let Some(h) = href {
                let lower = h.trim().to_ascii_lowercase();
                if lower.starts_with("javascript:") {
                    let code = String::from(h.trim()[11..].trim());
                    self.run_js(&code, usize::MAX);
                    return true;
                }
                if lower.starts_with("about:") { self.navigate(h.trim(), None, true); return true; }
                let target = self.doc.as_ref().and_then(|d| d.base.join(&h));
                match target {
                    Some(u) => { let s = u.to_string(); self.navigate(&s, None, true); }
                    None => self.status = format!("Ссылка не поддерживается: {}", h),
                }
            }
            return true;
        }
        true
    }

    pub fn wheel(&mut self, mx: usize, my: usize, delta: i32) -> bool {
        if !inside(self.rect, mx as i32, my as i32) || delta == 0 { return false; }
        if let Some((fi, ps)) = self.popup {
            let n = self.doc.as_ref().map(|d| d.page.fields[fi].options.len() as i32).unwrap_or(0);
            let ps = (ps + delta.signum() * 3).max(0).min((n - POPUP_ROWS).max(0));
            self.popup = Some((fi, ps));
            return true;
        }
        self.scroll += delta * 60;
        self.clamp_scroll();
        true
    }

    pub fn poll(&mut self, gfx: &mut Graphics) -> bool {
        if let Some(nav) = self.pending.take() {
            self.do_load(gfx, nav);
            return true;
        }
        if self.kiosk && self.update_clock() { return true; }
        let now = crate::timer::millis();
        if self.doc.as_ref().map(|d| !d.img_queue.is_empty()).unwrap_or(false) && now - self.img_timer > 30 {
            let r = self.load_next_image();
            self.img_timer = crate::timer::millis();
            return r;
        }
        false
    }

    pub fn is_loading(&self) -> bool { self.pending.is_some() }

    fn update_clock(&mut self) -> bool {
        let now = crate::timer::millis();
        unsafe {
            let last = &mut *(&raw mut CLOCK_LAST_MS);
            if now.wrapping_sub(*last) < 200 { return false; }
            *last = now;
        }
        let d = match self.doc.as_mut() { Some(d) => d, None => return false };
        let mut ids: Vec<usize> = Vec::new();
        for (i, n) in d.dom.nodes.iter().enumerate() {
            if let NodeData::Element(e) = &n.data {
                if e.attr("data-clock").is_some() { ids.push(i); }
            }
        }
        if ids.is_empty() { return false; }
        let (h, m, sec) = crate::get_time();
        let txt = String::from(crate::time_to_str(h, m, sec));
        let mut changed = false;
        for id in ids {
            let children = d.dom.nodes[id].children.clone();
            for c in children {
                if let NodeData::Text(t) = &mut d.dom.nodes[c].data {
                    if *t != txt { *t = txt.clone(); changed = true; }
                    break;
                }
            }
        }
        if changed { let w = d.lw; d.relayout(w); }
        changed
    }
}

static mut GFX_PTR: *mut Graphics = core::ptr::null_mut();
fn gfx_ptr() -> &'static mut Graphics { unsafe { &mut *GFX_PTR } }

fn draw_status_bar(gfx: &mut Graphics, r: (i32, i32, i32, i32), text: &str) {
    if r.2 <= 0 || r.3 <= 0 { return; }
    let t = wm::get_theme();
    gfx.fill_rect(r.0 as usize, r.1 as usize, r.2 as usize, r.3 as usize, t.window_bg.to_u32());
    gfx.fill_rect(r.0 as usize, r.1 as usize, r.2 as usize, 1, t.border_dark.to_u32());
    let maxc = ((r.2 - 12) / 8).max(0) as usize;
    let s: String = text.chars().take(maxc).collect();
    ui_text(gfx, r.0 + 6, r.1 + 3, &s, t.button_text.to_u32());
}

fn flush_rect(r: (i32, i32, i32, i32)) {
    crate::fastblit::flush_rect(gfx_ptr(), r.0.max(0) as usize, r.1.max(0) as usize, r.2.max(0) as usize, r.3.max(0) as usize);
}

pub fn draw(gfx: &mut Graphics, x: usize, y: usize, w: usize, h: usize) {
    unsafe { GFX_PTR = gfx as *mut Graphics; }
    get().draw(gfx, x, y, w, h);
}

pub fn poll(gfx: &mut Graphics) -> bool {
    unsafe {
        if (*(&raw const BROWSER)).is_none() { return false; }
        GFX_PTR = gfx as *mut Graphics;
    }
    get().poll(gfx)
}

pub fn key(sc: u8) -> bool { get().key(sc) }
pub fn click(x: usize, y: usize) -> bool { get().click(x, y) }
pub fn wheel(x: usize, y: usize, d: i32) -> bool { get().wheel(x, y, d) }

static mut APP_BROWSER: Option<Browser> = None;

fn app_get() -> &'static mut Browser {
    unsafe {
        let b = &mut *(&raw mut APP_BROWSER);
        if b.is_none() { *b = Some(Browser::blank(true)); }
        b.as_mut().unwrap()
    }
}

pub fn looks_like_html(data: &[u8]) -> bool {
    let mut i = if data.len() >= 3 && data[0] == 0xEF && data[1] == 0xBB && data[2] == 0xBF { 3 } else { 0 };
    while i < data.len() && (data[i] == b' ' || data[i] == b'\t' || data[i] == b'\r' || data[i] == b'\n') { i += 1; }
    i < data.len() && data[i] == b'<'
}

static mut APP_TITLE: String = String::new();
static mut CLOCK_LAST_MS: u64 = 0;

static mut APP_LAUNCH: String = String::new();

pub fn take_launch() -> Option<String> {
    unsafe {
        let l = &mut *(&raw mut APP_LAUNCH);
        if l.is_empty() { None } else { Some(core::mem::take(l)) }
    }
}

fn extract_title(src: &str) -> String {
    let low = src.to_ascii_lowercase();
    if let Some(a) = low.find("<title>") {
        let st = a + 7;
        if let Some(b) = low[st..].find("</title>") {
            return String::from(src[st..st + b].trim());
        }
    }
    String::new()
}

pub fn app_title() -> &'static str {
    unsafe {
        let t = &*(&raw const APP_TITLE);
        if t.is_empty() { "HTML App" } else { core::mem::transmute::<&str, &'static str>(t.as_str()) }
    }
}

pub fn window_size(bytes: &[u8]) -> Option<(usize, usize)> {
    let src = decode_text(bytes, "");
    let low = src.to_ascii_lowercase();
    let mut pos = 0usize;
    while let Some(a) = low[pos..].find("<meta") {
        let s = pos + a;
        let e = match low[s..].find('>') { Some(e) => s + e, None => break };
        let tag = &low[s..e];
        if tag.contains("window-size") {
            if let Some(c) = tag.find("content=") {
                let rest = tag[c + 8..].trim_start_matches(|ch: char| ch == '"' || ch == '\'');
                let mut it = rest.split(|ch: char| ch == 'x' || ch == '"' || ch == '\'');
                let w = it.next()?.trim().parse::<usize>().ok()?;
                let h = it.next()?.trim().parse::<usize>().ok()?;
                return Some((w, h));
            }
        }
        pos = e;
    }
    None
}

pub fn app_open(bytes: &[u8]) {
    let src = decode_text(bytes, "");
    unsafe { *(&raw mut APP_TITLE) = extract_title(&src); }
    let b = app_get();
    b.local = src;
    b.doc = None;
    b.scroll = 0;
    b.popup = None;
    b.focus = Focus::Page;
    b.pending = Some(Nav { url: String::from("app:"), post: None, push: false });
}

pub fn app_draw(gfx: &mut Graphics, x: usize, y: usize, w: usize, h: usize) {
    unsafe { GFX_PTR = gfx as *mut Graphics; }
    app_get().draw(gfx, x, y, w, h);
}

pub fn app_poll(gfx: &mut Graphics) -> bool {
    unsafe {
        if (*(&raw const APP_BROWSER)).is_none() { return false; }
        GFX_PTR = gfx as *mut Graphics;
    }
    app_get().poll(gfx)
}

pub fn app_key(sc: u8) -> bool { app_get().key(sc) }
pub fn app_click(x: usize, y: usize) -> bool { app_get().click(x, y) }
pub fn app_wheel(x: usize, y: usize, d: i32) -> bool { app_get().wheel(x, y, d) }
