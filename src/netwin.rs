use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;
use crate::graphics::Graphics;
use crate::net::{self, wifi};
use crate::web::keys;
use crate::wm;

pub const TITLE: &str = "Network";

struct State {
    networks: Vec<wifi::WifiNetwork>,
    selected: Option<usize>,
    password: String,
    show_pw: bool,
    pw_focus: bool,
    message: String,
    msg_ok: bool,
    rect: (i32, i32, i32, i32),
    last_sig: String,
    scanned: bool,
}

static mut STATE: Option<State> = None;

fn st() -> &'static mut State {
    unsafe {
        let s = &mut *(&raw mut STATE);
        if s.is_none() {
            *s = Some(State { networks: Vec::new(), selected: None, password: String::new(), show_pw: false, pw_focus: false,
                message: String::new(), msg_ok: false, rect: (0, 0, 0, 0), last_sig: String::new(), scanned: false });
        }
        s.as_mut().unwrap()
    }
}

type R = (i32, i32, i32, i32);
fn inside(r: R, x: i32, y: i32) -> bool { x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3 }

struct Ui { wired_btn: R, scan_btn: R, list: R, pw: R, show: R, connect: R, lang: R }

fn ui(r: R) -> Ui {
    let (x, y, w, _h) = r;
    let wired_btn = (x + w - 150, y + 36, 136, 26);
    let wy = y + 196;
    let scan_btn = (x + w - 150, wy + 30, 136, 26);
    let list = (x + 14, wy + 62, w - 28, 150);
    let pw = (x + 14, wy + 240, w - 28 - 150, 26);
    let show = (x + w - 150 + 6, wy + 240, 60, 26);
    let lang = (x + w - 78, wy + 240, 64, 26);
    let connect = (x + 14, wy + 276, 150, 28);
    Ui { wired_btn, scan_btn, list, pw, show, connect, lang }
}

fn text(gfx: &mut Graphics, x: i32, y: i32, s: &str, c: u32) {
    if x >= 0 && y >= 0 { gfx.draw_text(x as usize, y as usize, s, c, c); }
}

fn button(gfx: &mut Graphics, r: R, label: &str) {
    wm::draw_raised_rect(gfx, r.0 as usize, r.1 as usize, r.2 as usize, r.3 as usize);
    let tw = label.chars().count() as i32 * 8;
    text(gfx, r.0 + (r.2 - tw) / 2, r.1 + (r.3 - 16) / 2, label, wm::get_theme().button_text.to_u32());
}

fn group(gfx: &mut Graphics, x: i32, y: i32, w: i32, h: i32, title: &str) {
    let t = wm::get_theme();
    let d = t.border_dark.to_u32();
    let l = t.border_light.to_u32();
    gfx.draw_rect_border(x as usize, y as usize + 8, w as usize, h as usize, d);
    gfx.draw_rect_border(x as usize + 1, y as usize + 9, w as usize - 2, h as usize - 2, l);
    let tw = title.chars().count() * 8 + 8;
    gfx.fill_rect(x as usize + 10, y as usize, tw, 16, t.window_bg.to_u32());
    text(gfx, x + 14, y, title, t.button_text.to_u32());
}

fn signature() -> String {
    format!("{}|{}|{}|{}", net::status_text(), net::ip_string(), net::gw_string(), net::dns_string())
}

pub fn draw(gfx: &mut Graphics, x: usize, y: usize, w: usize, h: usize) {
    let s = st();
    s.rect = (x as i32, y as i32, w as i32, h as i32);
    s.last_sig = signature();
    let u = ui(s.rect);
    let t = wm::get_theme();
    let tc = t.button_text.to_u32();
    let (x, y, w) = (x as i32, y as i32, w as i32);
    gfx.fill_rect(x as usize, y as usize, w as usize, h, t.window_bg.to_u32());

    group(gfx, x + 8, y + 6, w - 16, 176, "Проводная сеть (Ethernet)");
    let adapter = net::adapters().iter().find(|a| a.kind == net::Kind::Ethernet).map(|a| a.name.clone());
    let status = net::status();
    let connected = status == net::Status::Connected;
    let sc = match status { net::Status::Connected => 0x107010, net::Status::Dhcp => 0x806000, _ => 0xA02020 };
    let mut ly = y + 32;
    let rows: [(&str, String); 6] = [
        ("Адаптер:", adapter.clone().unwrap_or_else(|| String::from("не найден"))),
        ("Состояние:", String::from(net::status_text())),
        ("IP-адрес:", net::ip_string()),
        ("Шлюз:", net::gw_string()),
        ("DNS:", net::dns_string()),
        ("MAC:", if net::link_speed() > 0 { format!("{}   {} Мбит/с", net::mac_string(), net::link_speed()) } else { net::mac_string() }),
    ];
    for (i, (k, v)) in rows.iter().enumerate() {
        text(gfx, x + 22, ly, k, tc);
        let maxc = ((w - 150 - 130 - 10) / 8).max(4) as usize;
        let v: String = v.chars().take(if i == 0 || i == 1 { maxc } else { 60 }).collect();
        text(gfx, x + 120, ly, &v, if i == 1 { sc } else { tc });
        ly += 22;
    }
    if adapter.is_some() {
        button(gfx, u.wired_btn, if status == net::Status::Disabled { "Подключить" } else { "Отключить" });
    }
    let _ = connected;

    let wy = y + 196;
    let gh = h as i32 - 196 - 12;
    group(gfx, x + 8, wy, w - 16, gh.max(40), "Беспроводная сеть (Wi-Fi)");
    let wname = wifi::adapter_name().unwrap_or_else(|| String::from("не найден"));
    text(gfx, x + 22, wy + 26, "Адаптер:", tc);
    let maxc = ((w - 150 - 120 - 10) / 8).max(4) as usize;
    let wn: String = wname.chars().take(maxc).collect();
    text(gfx, x + 120, wy + 26, &wn, tc);
    button(gfx, u.scan_btn, "Поиск сетей");
    text(gfx, x + 22, wy + 44, "Доступные сети:", tc);
    let l = u.list;
    wm::draw_sunken_rect(gfx, l.0 as usize, l.1 as usize, l.2 as usize, l.3 as usize);
    gfx.fill_rect(l.0 as usize + 2, l.1 as usize + 2, (l.2 - 4) as usize, (l.3 - 4) as usize, 0xFFFFFF);
    if s.networks.is_empty() {
        let msg = if s.scanned { "Сети не найдены" } else { "Нажмите «Поиск сетей»" };
        text(gfx, l.0 + 10, l.1 + 10, msg, 0x808080);
    }
    for (i, n) in s.networks.iter().enumerate() {
        let ry = l.1 + 3 + i as i32 * 24;
        if ry + 24 > l.1 + l.3 { break; }
        let sel = s.selected == Some(i);
        if sel { gfx.fill_rect(l.0 as usize + 3, ry as usize, (l.2 - 6) as usize, 23, 0x3B78D8); }
        let c = if sel { 0xFFFFFF } else { 0x000000 };
        for b in 0..4 {
            let bh = 4 + b * 3;
            let on = n.signal as i32 > b * 25;
            gfx.fill_rect((l.0 + 10 + b * 5) as usize, (ry + 18 - bh) as usize, 3, bh as usize, if on { c } else { 0xB0B0B0 });
        }
        let name: String = n.ssid.chars().take(((l.2 - 140) / 8).max(4) as usize).collect();
        text(gfx, l.0 + 36, ry + 4, &name, c);
        text(gfx, l.0 + l.2 - 100, ry + 4, if n.secure { "WPA2" } else { "Открытая" }, c);
    }
    text(gfx, x + 22, wy + 222, "Пароль:", tc);
    let p = u.pw;
    wm::draw_sunken_rect(gfx, p.0 as usize, p.1 as usize, p.2 as usize, p.3 as usize);
    gfx.fill_rect(p.0 as usize + 2, p.1 as usize + 2, (p.2 - 4) as usize, (p.3 - 4) as usize, 0xFFFFFF);
    let shown: String = if s.show_pw { s.password.clone() } else { s.password.chars().map(|_| '*').collect() };
    let maxc = ((p.2 - 16) / 8).max(1) as usize;
    let cs: Vec<char> = shown.chars().collect();
    let tail: String = cs[cs.len().saturating_sub(maxc)..].iter().collect();
    text(gfx, p.0 + 6, p.1 + 5, &tail, 0x000000);
    if s.pw_focus {
        let cx = p.0 + 6 + tail.chars().count() as i32 * 8;
        gfx.fill_rect(cx as usize, p.1 as usize + 5, 1, 16, 0x000000);
    }
    button(gfx, u.show, if s.show_pw { "Скрыть" } else { "Показ." });
    button(gfx, u.lang, keys::layout_name());
    button(gfx, u.connect, "Подключиться");
    if !s.message.is_empty() {
        let maxc = ((w - 200) / 8).max(4) as usize;
        let m: String = s.message.chars().take(maxc).collect();
        text(gfx, u.connect.0 + u.connect.2 + 12, u.connect.1 + 6, &m, if s.msg_ok { 0x107010 } else { 0xA02020 });
    }
}

pub fn click(mx: usize, my: usize) -> bool {
    let s = st();
    let (mx, my) = (mx as i32, my as i32);
    if !inside(s.rect, mx, my) { return false; }
    let u = ui(s.rect);
    s.pw_focus = false;
    if inside(u.wired_btn, mx, my) {
        if net::status() == net::Status::Disabled { net::set_enabled(true); } else { net::set_enabled(false); }
        return true;
    }
    if inside(u.scan_btn, mx, my) {
        s.scanned = true;
        match wifi::scan() {
            Ok(list) => { s.networks = list; s.selected = None; s.message = format!("Найдено сетей: {}", s.networks.len()); s.msg_ok = true; }
            Err(e) => { s.networks.clear(); s.message = e; s.msg_ok = false; }
        }
        return true;
    }
    if inside(u.list, mx, my) {
        let i = ((my - u.list.1 - 3) / 24) as usize;
        if i < s.networks.len() { s.selected = Some(i); s.pw_focus = s.networks[i].secure; }
        return true;
    }
    if inside(u.pw, mx, my) { s.pw_focus = true; return true; }
    if inside(u.show, mx, my) { s.show_pw = !s.show_pw; s.pw_focus = true; return true; }
    if inside(u.lang, mx, my) { keys::toggle_layout(); s.pw_focus = true; return true; }
    if inside(u.connect, mx, my) { connect(); return true; }
    true
}

fn connect() {
    let s = st();
    let ssid = s.selected.and_then(|i| s.networks.get(i)).map(|n| n.ssid.clone()).unwrap_or_default();
    if ssid.is_empty() && !wifi::has_driver() {
        s.message = wifi::no_driver_msg();
        s.msg_ok = false;
        return;
    }
    match wifi::connect(&ssid, &s.password) {
        Ok(()) => { s.message = format!("Подключено к {}", ssid); s.msg_ok = true; }
        Err(e) => { s.message = e; s.msg_ok = false; }
    }
}

pub fn key(sc: u8) -> bool {
    let s = st();
    if !s.pw_focus { return false; }
    if sc == keys::BACKSPACE { s.password.pop(); }
    else if sc == keys::ENTER { connect(); }
    else if sc == keys::ESC { s.pw_focus = false; }
    else if let Some(c) = keys::char_for(sc) { if s.password.len() < 63 { s.password.push(c); } }
    else { return false; }
    true
}

pub fn changed() -> bool {
    let s = st();
    signature() != s.last_sig
}

fn disc(gfx: &mut Graphics, cx: i32, cy: i32, r: i32, f: &dyn Fn(i32, i32) -> Option<u32>) {
    for dy in -r..=r {
        for dx in -r..=r {
            if dx * dx + dy * dy <= r * r {
                if let Some(c) = f(dx, dy) { gfx.put_pixel((cx + dx) as usize, (cy + dy) as usize, c); }
            }
        }
    }
}

pub fn draw_browser_icon(gfx: &mut Graphics, x: usize, y: usize, size: usize) {
    let r = (size as i32) / 2 - 4;
    let cx = x as i32 + size as i32 / 2;
    let cy = y as i32 + size as i32 / 2;
    disc(gfx, cx, cy, r, &|dx, dy| {
        let d2 = dx * dx + dy * dy;
        if d2 > (r - 2) * (r - 2) { return Some(0x1D4F91); }
        let v = 4 * dx * dx + dy * dy;
        let meridian = dx == 0 || (v - r * r).abs() < 2 * r;
        let parallel = dy == 0 || (dy.abs() - r / 2).abs() == 0;
        if meridian || parallel { return Some(0xE8F2FF); }
        let t = ((dy + r) * 255 / (2 * r)) as u32;
        let red = (0x5A * (255 - t) + 0x1C * t) / 255;
        let grn = (0xA8 * (255 - t) + 0x5F * t) / 255;
        let blu = (0xF0 * (255 - t) + 0xB8 * t) / 255;
        Some(red << 16 | grn << 8 | blu)
    });
}

pub fn draw_network_icon(gfx: &mut Graphics, x: usize, y: usize, size: usize) {
    let s = size as i32;
    let (x, y) = (x as i32, y as i32);
    for k in 0..3 {
        let rr: i32 = 8 + k * 7;
        for dy in -rr..=0 {
            for dx in -rr..=rr {
                let d2 = dx * dx + dy * dy;
                if d2 <= rr * rr && d2 > (rr - 3) * (rr - 3) && dy < -(dx.abs()) / 2 {
                    gfx.put_pixel((x + s / 2 + dx) as usize, (y + 24 + dy) as usize, 0x2A7BD8);
                }
            }
        }
    }
    gfx.fill_rect((x + s / 2 - 2) as usize, (y + 21) as usize, 5, 5, 0x2A7BD8);
    gfx.fill_rect((x + 8) as usize, (y + 28) as usize, (s - 16) as usize, 18, 0x303640);
    gfx.fill_rect((x + 10) as usize, (y + 30) as usize, (s - 20) as usize, 14, 0x5FB0FF);
    gfx.fill_rect((x + s / 2 - 3) as usize, (y + 46) as usize, 6, 4, 0x303640);
    gfx.fill_rect((x + s / 2 - 10) as usize, (y + 50) as usize, 20, 3, 0x303640);
}
