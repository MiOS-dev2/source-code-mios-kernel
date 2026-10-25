

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::arch::asm;

use crate::ata;
use crate::fs::fat32;
use crate::graphics::Graphics;
use crate::syscfg;

static RY_CFG: &[u8] = include_bytes!("Secure_sys/cfg.mp");
static RY_DIR: &[u8] = include_bytes!("Secure_sys/dir.mp");
static RY_MAS: &[u8] = include_bytes!("Secure_sys/mas.mp");
static RY_USR: &[u8] = include_bytes!("Secure_sys/usr.mp");
static SX_MODE: &[u8] = include_bytes!("systemx64/mode.mp");
static SX_BUILD: &[u8] = include_bytes!("systemx64/build.mp");
static SX_CALC: &[u8] = include_bytes!("systemx64/calc.mp");
static GRUB_BOOT_IMG: &[u8] = include_bytes!("../target/grub_boot.img");
static GRUB_CORE_IMG: &[u8] = include_bytes!("../target/grub_core.img");

const KERNEL_NAME: &str = "MIUX.ELF";

pub fn wanted(info_addr: u32) -> bool {
    unsafe {
        let total = *(info_addr as *const u32) as usize;
        let mut off = 8usize;
        while off + 8 <= total {
            let p = (info_addr as usize + off) as *const u32;
            let ty = *p;
            let sz = *p.add(1) as usize;
            if ty == 0 || sz < 8 {
                break;
            }
            if ty == 1 {
                let s = core::slice::from_raw_parts((p as *const u8).add(8), sz - 8);
                for w in s.split(|&b| b == b' ' || b == 0) {
                    if w == b"install" {
                        return true;
                    }
                }
                return false;
            }
            off += (sz + 7) & !7;
        }
    }
    false
}


unsafe fn find_payload(info_addr: u32) -> Option<&'static [u8]> {
    let total = *(info_addr as *const u32) as usize;
    let mut best: Option<&'static [u8]> = None;
    let mut off = 8usize;
    while off + 8 <= total {
        let p = (info_addr as usize + off) as *const u32;
        let ty = *p;
        let sz = *p.add(1) as usize;
        if ty == 0 || sz < 8 {
            break;
        }
        if ty == 3 {
            let start = *p.add(2) as usize;
            let end = *p.add(3) as usize;
            if end > start + 4 {
                let m = core::slice::from_raw_parts(start as *const u8, end - start);
                let bigger = match best {
                    Some(b) => m.len() > b.len(),
                    None => true,
                };
                if bigger {
                    best = Some(m);
                }
            }
        }
        off += (sz + 7) & !7;
    }
    best
}




const CW: usize = 8;
const CH: usize = 16;

const C_BACK: u32 = 0x0000AA;
const C_BOX: u32 = 0xAAAAAA;
const C_BLACK: u32 = 0x000000;
const C_WHITE: u32 = 0xFFFFFF;
const C_YEL: u32 = 0xFFFF55;
const C_RED: u32 = 0xAA0000;
const C_GRN: u32 = 0x006600;

const P_WELCOME: u8 = 0;
const P_DISK: u8 = 1;
const P_NAME: u8 = 2;
const P_PASS: u8 = 3;
const P_AUTO: u8 = 4;
const P_CONFIRM: u8 = 5;
const P_INSTALL: u8 = 6;
const P_DONE: u8 = 7;
const P_ERROR: u8 = 8;

const LOG_ROWS: usize = 7;

const B_SELECT_CANCEL: &[&str] = &["Выбрать", "Отмена"];
const B_OK_BACK: &[&str] = &["OK", "Назад"];
const B_YES_NO: &[&str] = &["Да", "Нет"];
const B_REBOOT: &[&str] = &["Перезагрузить"];
const B_NONE: &[&str] = &[];

struct DiskItem {
    idx: usize,
    label: String,
}

struct Wiz {
    page: u8,
    disk_pick: usize,
    wsel: usize,
    btn: usize,
    name: String,
    pass: String,
    auto: i8,
    err: String,
    overall: u32,
    cur: u32,
    status: String,
    log: Vec<String>,
    fail_lines: Vec<String>,
}

impl Wiz {
    fn new() -> Wiz {
        Wiz {
            page: P_WELCOME,
            disk_pick: 0,
            wsel: 0,
            btn: 0,
            name: String::new(),
            pass: String::new(),
            auto: -1,
            err: String::new(),
            overall: 0,
            cur: 0,
            status: String::new(),
            log: Vec::new(),
            fail_lines: Vec::new(),
        }
    }
}

fn clip(s: &str, max: usize) -> String {
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i >= max {
            break;
        }
        out.push(ch);
    }
    out
}

fn wrap(s: &str, max: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut n = 0usize;
    for word in s.split(' ') {
        let wl = word.chars().count();
        if n > 0 && n + 1 + wl > max {
            out.push(core::mem::take(&mut cur));
            n = 0;
        }
        if n > 0 {
            cur.push(' ');
            n += 1;
        }
        cur.push_str(word);
        n += wl;
    }
    if n > 0 || out.is_empty() {
        out.push(cur);
    }
    out
}

fn put(g: &mut Graphics, col: usize, row: usize, s: &str, fg: u32, bg: u32) {
    let n = s.chars().count();
    if n == 0 {
        return;
    }
    g.fill_rect(col * CW, row * CH, n * CW, CH, bg);
    g.draw_text(col * CW, row * CH, s, fg, bg);
}

fn draw_box(g: &mut Graphics, c0: usize, r0: usize, bw: usize, bh: usize, title: &str) {
    g.fill_rect((c0 + 2) * CW, (r0 + 1) * CH, bw * CW, bh * CH, C_BLACK);
    g.fill_rect(c0 * CW, r0 * CH, bw * CW, bh * CH, C_BOX);
    let x = c0 * CW + 3;
    let y = r0 * CH + 7;
    let w = bw * CW - 7;
    let h = bh * CH - 15;
    g.draw_rect_border(x, y, w, h, C_WHITE);
    g.draw_rect_border(x + 1, y + 1, w - 2, h - 2, C_BLACK);
    let t = format!(" {} ", title);
    let n = t.chars().count();
    let col = c0 + bw.saturating_sub(n) / 2;
    put(g, col, r0, &t, C_BLACK, C_BOX);
}

enum Item {
    Text(String, u32),
    Blank,
    Menu(Vec<(String, String)>, usize),
    Field(String),
    Bar(u32),
    Log(Vec<String>, bool),
}

fn item_h(it: &Item) -> usize {
    match it {
        Item::Text(..) | Item::Blank | Item::Field(_) | Item::Bar(_) => 1,
        Item::Menu(v, _) => v.len() + 2,
        Item::Log(..) => LOG_ROWS + 2,
    }
}

fn paras(items: &mut Vec<Item>, s: &str, max: usize, color: u32) {
    for line in wrap(s, max) {
        items.push(Item::Text(line, color));
    }
}

fn build_page(w: &Wiz, disks: &[DiskItem], iw: usize) -> (&'static str, Vec<Item>, &'static [&'static str]) {
    let mut it: Vec<Item> = Vec::new();
    match w.page {
        P_WELCOME => {
            paras(&mut it, "Добро пожаловать в установку MiUX 7.0.0!", iw, C_BLACK);
            it.push(Item::Blank);
            paras(&mut it, "Официальный сайт: https://miux.phoneget.top", iw, C_BLACK);
            it.push(Item::Blank);
            let mut m: Vec<(String, String)> = Vec::new();
            m.push((String::from("Установка"), String::from("Установить MiUX на жёсткий диск")));
            m.push((String::from("Перезагрузка"), String::from("Перезагрузить компьютер")));
            it.push(Item::Menu(m, w.wsel));
            ("Установка MiUX", it, B_SELECT_CANCEL)
        }
        P_DISK => {
            paras(&mut it, "Выберите диск, на который будет установлена MiUX:", iw, C_BLACK);
            it.push(Item::Blank);
            let mut m: Vec<(String, String)> = Vec::new();
            for (i, d) in disks.iter().enumerate() {
                m.push((format!("ada{}", i), clip(&d.label, iw.saturating_sub(16))));
            }
            it.push(Item::Menu(m, w.disk_pick));
            it.push(Item::Blank);
            paras(&mut it, "ВНИМАНИЕ: все данные на выбранном диске будут удалены!", iw, C_RED);
            ("Выбор диска", it, B_SELECT_CANCEL)
        }
        P_NAME => {
            paras(&mut it, "Как будут звать вашего пользователя? (латинские буквы, цифры, - и _, до 31 символа)", iw, C_BLACK);
            it.push(Item::Blank);
            it.push(Item::Field(w.name.clone()));
            if !w.err.is_empty() {
                it.push(Item::Blank);
                paras(&mut it, &w.err, iw, C_RED);
            }
            ("Добавление пользователя", it, B_OK_BACK)
        }
        P_PASS => {
            paras(&mut it, "Какой будет пароль? Без пароля нельзя)", iw, C_BLACK);
            it.push(Item::Blank);
            let mut stars = String::new();
            for _ in 0..w.pass.len() {
                stars.push('*');
            }
            it.push(Item::Field(stars));
            if !w.err.is_empty() {
                it.push(Item::Blank);
                paras(&mut it, &w.err, iw, C_RED);
            }
            ("Пароль пользователя", it, B_OK_BACK)
        }
        P_AUTO => {
            paras(&mut it, "Входить ли в ОС автоматически, без запроса пароля?", iw, C_BLACK);
            ("Автоматический вход", it, B_YES_NO)
        }
        P_CONFIRM => {
            let dl = if w.disk_pick < disks.len() { disks[w.disk_pick].label.as_str() } else { "?" };
            paras(&mut it, "Вы уверены, что хотите уничтожить ВСЕ данные на диске:", iw, C_RED);
            it.push(Item::Blank);
            it.push(Item::Text(format!("  ada{}  {}", w.disk_pick, clip(dl, iw.saturating_sub(10))), C_BLACK));
            it.push(Item::Blank);
            it.push(Item::Text(format!("Пользователь: {}", w.name), C_BLACK));
            it.push(Item::Text(String::from("Пароль: задан"), C_BLACK));
            it.push(Item::Text(
                String::from(if w.auto == 1 { "Автоматический вход: да" } else { "Автоматический вход: нет" }),
                C_BLACK,
            ));
            ("Последний шанс!", it, B_YES_NO)
        }
        P_INSTALL => {
            paras(&mut it, "Идёт установка MiUX. Не выключайте компьютер.", iw, C_BLACK);
            it.push(Item::Blank);
            it.push(Item::Text(String::from("Общий прогресс:"), C_BLACK));
            it.push(Item::Bar(w.overall));
            it.push(Item::Blank);
            it.push(Item::Text(clip(&format!("Текущая операция: {}", w.status), iw), C_BLACK));
            it.push(Item::Bar(w.cur));
            it.push(Item::Blank);
            it.push(Item::Log(w.log.clone(), true));
            ("Установка MiUX", it, B_NONE)
        }
        P_DONE => {
            it.push(Item::Text(String::from("MiUX успешно установлена!"), C_GRN));
            it.push(Item::Blank);
            paras(&mut it, "Извлеките CD/ISO из привода, затем нажмите Enter (или любую клавишу) для перезагрузки.", iw, C_BLACK);
            it.push(Item::Blank);
            it.push(Item::Text(format!("Пользователь: {}", w.name), C_BLACK));
            it.push(Item::Text(
                String::from(if w.auto == 1 { "Автоматический вход: включён" } else { "Автоматический вход: выключен" }),
                C_BLACK,
            ));
            ("Установка завершена", it, B_REBOOT)
        }
        _ => {
            paras(&mut it, &w.err, iw, C_RED);
            it.push(Item::Blank);
            paras(&mut it, "Установка не выполнена. Нажмите любую клавишу для перезагрузки.", iw, C_BLACK);
            if !w.fail_lines.is_empty() {
                it.push(Item::Blank);
                it.push(Item::Log(w.fail_lines.clone(), false));
            }
            ("Ошибка установки", it, B_REBOOT)
        }
    }
}

fn draw_screen(g: &mut Graphics, cols: usize, rows: usize, w: &Wiz, disks: &[DiskItem]) {
    let sw = g.width;
    let sh = g.height;
    g.fill_rect(0, 0, sw, sh, C_BACK);
    put(g, 1, 0, "MiUX Installer", C_WHITE, C_BACK);

    let bw = if cols > 76 { 68 } else if cols > 12 { cols - 8 } else { cols };
    let iw = bw.saturating_sub(6);
    let (title, items, btns) = build_page(w, disks, iw);

    let mut sum = 0usize;
    for i in items.iter() {
        sum += item_h(i);
    }
    let btn_rows = if btns.is_empty() { 0 } else { 3 };
    let bh = 2 + sum + 1 + btn_rows + 1;
    let c0 = cols.saturating_sub(bw) / 2;
    let r0 = (rows.saturating_sub(bh) / 2).max(1);
    draw_box(g, c0, r0, bw, bh, title);

    let x = c0 + 3;
    let mut y = r0 + 2;
    for item in items.iter() {
        match item {
            Item::Text(s, c) => {
                put(g, x, y, s, *c, C_BOX);
            }
            Item::Blank => {}
            Item::Menu(v, sel) => {
                let mut tagw = 0usize;
                for (t, _) in v.iter() {
                    tagw = tagw.max(t.chars().count());
                }
                g.draw_rect_border(x * CW + 3, y * CH + 7, iw * CW - 6, (v.len() + 2) * CH - 15, C_BLACK);
                for (i, (tag, desc)) in v.iter().enumerate() {
                    let ry = y + 1 + i;
                    let selected = i == *sel;
                    if selected {
                        g.fill_rect((x + 1) * CW, ry * CH, iw.saturating_sub(2) * CW, CH, C_BACK);
                    }
                    let bg = if selected { C_BACK } else { C_BOX };
                    let tfg = if selected { C_YEL } else { C_BLACK };
                    let dfg = if selected { C_WHITE } else { C_BLACK };
                    put(g, x + 2, ry, tag, tfg, bg);
                    let dmax = iw.saturating_sub(tagw + 6);
                    put(g, x + 2 + tagw + 2, ry, &clip(desc, dmax), dfg, bg);
                }
            }
            Item::Field(s) => {
                let n = s.chars().count();
                let maxc = iw.saturating_sub(3);
                let shown: String = if n > maxc { s.chars().skip(n - maxc).collect() } else { s.clone() };
                let mut line = format!(" {}_", shown);
                while line.chars().count() < iw {
                    line.push(' ');
                }
                put(g, x, y, &line, C_BLACK, C_WHITE);
            }
            Item::Bar(p) => {
                let p = (*p).min(100) as usize;
                let label = format!("{}%", p);
                let lab: Vec<char> = label.chars().collect();
                let start = iw.saturating_sub(lab.len()) / 2;
                let filled = iw * p / 100;
                for i in 0..iw {
                    let ch = if i >= start && i < start + lab.len() { lab[i - start] } else { ' ' };
                    let (fg, bg) = if i < filled { (C_WHITE, C_BACK) } else { (C_BACK, C_WHITE) };
                    let mut buf = String::new();
                    buf.push(ch);
                    put(g, x + i, y, &buf, fg, bg);
                }
            }
            Item::Log(lines, ok_tag) => {
                g.fill_rect(x * CW, y * CH + 4, iw * CW, (LOG_ROWS + 2) * CH - 8, C_WHITE);
                g.draw_rect_border(x * CW, y * CH + 4, iw * CW, (LOG_ROWS + 2) * CH - 8, C_BLACK);
                let start = if lines.len() > LOG_ROWS { lines.len() - LOG_ROWS } else { 0 };
                let mut ry = y + 1;
                for l in &lines[start..] {
                    if *ok_tag {
                        put(g, x + 1, ry, "OK", C_GRN, C_WHITE);
                        put(g, x + 4, ry, &clip(l, iw.saturating_sub(6)), C_BLACK, C_WHITE);
                    } else {
                        put(g, x + 1, ry, &clip(l, iw.saturating_sub(2)), C_BLACK, C_WHITE);
                    }
                    ry += 1;
                }
            }
        }
        y += item_h(item);
    }

    if !btns.is_empty() {
        let rule_row = r0 + 3 + sum;
        let btn_row = r0 + 4 + sum;
        g.fill_rect(c0 * CW + 3, rule_row * CH + 7, bw * CW - 7, 1, C_BLACK);
        let mut total = 0usize;
        for b in btns.iter() {
            total += b.chars().count() + 4;
        }
        total += (btns.len() - 1) * 3;
        let mut bx = c0 + bw.saturating_sub(total) / 2;
        for (i, b) in btns.iter().enumerate() {
            let s = format!("< {} >", b);
            let (fg, bg) = if i == w.btn { (C_WHITE, C_BACK) } else { (C_BLACK, C_BOX) };
            put(g, bx, btn_row, &s, fg, bg);
            bx += s.chars().count() + 3;
        }
    }

    let hint = if w.page == P_INSTALL {
        "Пожалуйста, подождите. Не выключайте компьютер."
    } else {
        "Tab/Влево/Вправо - кнопки | Вверх/Вниз - выбор | Enter - подтвердить | Esc - назад"
    };
    put(g, 1, rows.saturating_sub(1), &clip(hint, cols.saturating_sub(2)), C_WHITE, C_BACK);
}

struct Ui<'a> {
    g: &'a mut Graphics,
    cols: usize,
    rows: usize,
}

impl<'a> Ui<'a> {
    fn present(&mut self, w: &Wiz, disks: &[DiskItem]) {
        draw_screen(self.g, self.cols, self.rows, w, disks);
        self.g.flush();
    }
}


fn inb(port: u16) -> u8 {
    let v: u8;
    unsafe { asm!("in al, dx", in("dx") port, out("al") v) }
    v
}

#[derive(Clone, Copy, PartialEq)]
enum K {
    Ch(char),
    Enter,
    Back,
    Esc,
    Tab,
    Up,
    Down,
    Left,
    Right,
}

fn sc_char(sc: u8, shift: bool) -> Option<char> {
    let c = match sc {
        0x1E => 'a', 0x30 => 'b', 0x2E => 'c', 0x20 => 'd', 0x12 => 'e', 0x21 => 'f',
        0x22 => 'g', 0x23 => 'h', 0x17 => 'i', 0x24 => 'j', 0x25 => 'k', 0x26 => 'l',
        0x32 => 'm', 0x31 => 'n', 0x18 => 'o', 0x19 => 'p', 0x10 => 'q', 0x13 => 'r',
        0x1F => 's', 0x14 => 't', 0x16 => 'u', 0x2F => 'v', 0x11 => 'w', 0x2D => 'x',
        0x15 => 'y', 0x2C => 'z',
        0x39 => ' ',
        0x02 => '1', 0x03 => '2', 0x04 => '3', 0x05 => '4', 0x06 => '5',
        0x07 => '6', 0x08 => '7', 0x09 => '8', 0x0A => '9', 0x0B => '0',
        0x33 => ',', 0x34 => '.', 0x35 => '/', 0x27 => ';', 0x28 => '\'',
        0x2B => '\\', 0x29 => '`', 0x0C => '-', 0x0D => '=', 0x1A => '[', 0x1B => ']',
        _ => return None,
    };
    if !shift {
        return Some(c);
    }
    Some(match c {
        'a'..='z' => ((c as u8) - 32) as char,
        '1' => '!', '2' => '@', '3' => '#', '4' => '$', '5' => '%',
        '6' => '^', '7' => '&', '8' => '*', '9' => '(', '0' => ')',
        '-' => '_', '=' => '+', '[' => '{', ']' => '}', ';' => ':',
        '\'' => '"', '`' => '~', '\\' => '|', ',' => '<', '.' => '>', '/' => '?',
        _ => c,
    })
}

struct Hw {
    shift: bool,
    ext: bool,
}

impl Hw {
    fn poll(&mut self) -> Option<K> {
        loop {
            let st = inb(0x64);
            if st & 1 == 0 {
                return None;
            }
            let b = inb(0x60);
            if st & 0x20 != 0 {
                continue;
            }
            if b == 0xE0 {
                self.ext = true;
                continue;
            }
            if self.ext {
                self.ext = false;
                if b & 0x80 == 0 {
                    match b {
                        0x48 => return Some(K::Up),
                        0x50 => return Some(K::Down),
                        0x4B => return Some(K::Left),
                        0x4D => return Some(K::Right),
                        0x1C => return Some(K::Enter),
                        _ => {}
                    }
                }
                continue;
            }
            match b {
                0x2A | 0x36 => {
                    self.shift = true;
                    continue;
                }
                0xAA | 0xB6 => {
                    self.shift = false;
                    continue;
                }
                _ => {}
            }
            if b & 0x80 != 0 {
                continue;
            }
            let k = match b {
                0x1C => K::Enter,
                0x0E => K::Back,
                0x01 => K::Esc,
                0x0F => K::Tab,
                _ => match sc_char(b, self.shift) {
                    Some(c) => K::Ch(c),
                    None => continue,
                },
            };
            return Some(k);
        }
    }
}



enum Act {
    None,
    Redraw,
    Install,
    Reboot,
}

fn enter_page(w: &mut Wiz, p: u8) {
    w.page = p;
    w.btn = match p {
        P_AUTO => {
            if w.auto == 1 { 0 } else { 1 }
        }
        P_CONFIRM => 1,
        _ => 0,
    };
}

fn forward(w: &mut Wiz) -> Act {
    match w.page {
        P_WELCOME => {
            enter_page(w, P_DISK);
            Act::Redraw
        }
        P_DISK => {
            w.err.clear();
            enter_page(w, P_NAME);
            Act::Redraw
        }
        P_NAME => {
            if w.name.is_empty() {
                w.err = String::from("Введите имя пользователя.");
                return Act::Redraw;
            }
            let low = w.name.to_ascii_lowercase();
            if low == "admin" || low == "system" || low == "user" {
                w.err = String::from("Это имя занято системой, выберите другое.");
                return Act::Redraw;
            }
            w.err.clear();
            enter_page(w, P_PASS);
            Act::Redraw
        }
        P_PASS => {
            if w.pass.is_empty() {
                w.err = String::from("Без пароля нельзя! Введите пароль.");
                return Act::Redraw;
            }
            w.err.clear();
            enter_page(w, P_AUTO);
            Act::Redraw
        }
        P_AUTO => {
            w.err.clear();
            enter_page(w, P_CONFIRM);
            Act::Redraw
        }
        P_CONFIRM => Act::Install,
        _ => Act::Reboot,
    }
}

fn backward(w: &mut Wiz) -> Act {
    if w.page > P_WELCOME && w.page <= P_CONFIRM {
        w.err.clear();
        let p = w.page - 1;
        enter_page(w, p);
        Act::Redraw
    } else {
        Act::None
    }
}

fn activate(w: &mut Wiz) -> Act {
    match w.page {
        P_WELCOME => {
            if w.btn == 1 || w.wsel == 1 {
                Act::Reboot
            } else {
                forward(w)
            }
        }
        P_DISK | P_NAME | P_PASS => {
            if w.btn == 1 {
                backward(w)
            } else {
                forward(w)
            }
        }
        P_AUTO => {
            w.auto = if w.btn == 0 { 1 } else { 0 };
            forward(w)
        }
        P_CONFIRM => {
            if w.btn == 0 {
                Act::Install
            } else {
                backward(w)
            }
        }
        _ => Act::Reboot,
    }
}

fn on_key(w: &mut Wiz, k: K, nd: usize) -> Act {
    match w.page {
        P_INSTALL => return Act::None,
        P_DONE | P_ERROR => return Act::Reboot,
        _ => {}
    }
    match k {
        K::Enter => return activate(w),
        K::Esc => return backward(w),
        K::Tab => {
            w.btn = (w.btn + 1) % 2;
            return Act::Redraw;
        }
        K::Left => {
            if w.btn > 0 {
                w.btn -= 1;
                return Act::Redraw;
            }
            return Act::None;
        }
        K::Right => {
            if w.btn < 1 {
                w.btn += 1;
                return Act::Redraw;
            }
            return Act::None;
        }
        _ => {}
    }
    match w.page {
        P_WELCOME => {
            if k == K::Up && w.wsel > 0 {
                w.wsel -= 1;
                return Act::Redraw;
            }
            if k == K::Down && w.wsel < 1 {
                w.wsel += 1;
                return Act::Redraw;
            }
        }
        P_DISK => {
            if k == K::Up && w.disk_pick > 0 {
                w.disk_pick -= 1;
                return Act::Redraw;
            }
            if k == K::Down && w.disk_pick + 1 < nd {
                w.disk_pick += 1;
                return Act::Redraw;
            }
        }
        P_NAME => {
            if let K::Ch(c) = k {
                if (c.is_ascii_alphanumeric() || c == '-' || c == '_') && w.name.len() < 31 {
                    w.name.push(c);
                    w.err.clear();
                    return Act::Redraw;
                }
            } else if k == K::Back {
                w.name.pop();
                w.err.clear();
                return Act::Redraw;
            }
        }
        P_PASS => {
            if let K::Ch(c) = k {
                if (c as u32) >= 0x20 && (c as u32) < 0x7F && w.pass.len() < 31 {
                    w.pass.push(c);
                    w.err.clear();
                    return Act::Redraw;
                }
            } else if k == K::Back {
                w.pass.pop();
                w.err.clear();
                return Act::Redraw;
            }
        }
        P_AUTO | P_CONFIRM => {
            if let K::Ch(c) = k {
                if c == 'y' || c == 'Y' {
                    w.btn = 0;
                    return activate(w);
                }
                if c == 'n' || c == 'N' {
                    w.btn = 1;
                    return activate(w);
                }
            }
        }
        _ => {}
    }
    Act::None
}


fn reboot() -> ! {
    unsafe {
        let mut i = 0u32;
        while i < 100_000 {
            let st: u8;
            asm!("in al, dx", out("al") st, in("dx") 0x64u16);
            if st & 2 == 0 {
                break;
            }
            i += 1;
        }
        asm!("out dx, al", in("dx") 0x64u16, in("al") 0xFEu8);
        for _ in 0..50_000_000u32 {
            core::hint::spin_loop();
        }
        asm!("out dx, al", in("dx") 0xCF9u16, in("al") 0x06u8);
        for _ in 0..50_000_000u32 {
            core::hint::spin_loop();
        }
    }
    loop {
        unsafe { asm!("hlt") }
    }
}


unsafe fn install_grub(disk: usize, progress: &mut dyn FnMut(usize, usize)) -> Result<(), &'static str> {
    if GRUB_BOOT_IMG.len() < 512 || GRUB_CORE_IMG.len() < 1024 {
        return Err("GRUB images are missing (build with make)");
    }
    let img = GRUB_CORE_IMG;
    let sectors = (img.len() + 511) / 512;
    if sectors > 2000 {
        return Err("GRUB core.img is too big");
    }

    for s in 0..sectors {
        let mut buf = [0u8; 512];
        let from = s * 512;
        let to = if from + 512 > img.len() { img.len() } else { from + 512 };
        buf[..to - from].copy_from_slice(&img[from..to]);
        if s == 0 {
            buf[500..508].copy_from_slice(&2u64.to_le_bytes());
            buf[508..510].copy_from_slice(&((sectors - 1) as u16).to_le_bytes());
            buf[510..512].copy_from_slice(&0x0820u16.to_le_bytes());
        }
        if !ata::write_sector(disk, 1 + s as u32, &buf) {
            return Err("Write error (GRUB core)");
        }
        if s % 8 == 0 || s + 1 == sectors {
            progress(s + 1, sectors);
        }
    }

    let mut mbr = [0u8; 512];
    if !ata::read_sector(disk, 0, &mut mbr) {
        return Err("Read error (MBR)");
    }
    if mbr[510] != 0x55 || mbr[511] != 0xAA {
        return Err("MBR is damaged");
    }
    mbr[0..440].copy_from_slice(&GRUB_BOOT_IMG[0..440]);
    mbr[0x5C..0x64].copy_from_slice(&1u64.to_le_bytes());
    mbr[446] = 0x80;
    if !ata::write_sector(disk, 0, &mbr) {
        return Err("Write error (MBR)");
    }
    let _ = ata::flush(disk);
    Ok(())
}

fn prog(ui: &mut Ui, w: &mut Wiz, disks: &[DiskItem], overall: u32, cur: u32, status: &str) {
    w.overall = overall;
    w.cur = cur;
    w.status = String::from(status);
    ui.present(w, disks);
}

fn step_done(ui: &mut Ui, w: &mut Wiz, disks: &[DiskItem], overall: u32, line: &str) {
    if w.log.len() >= 7 {
        w.log.remove(0);
    }
    w.log.push(String::from(line));
    w.overall = overall;
    w.cur = 100;
    ui.present(w, disks);
}

fn do_install(ui: &mut Ui, w: &mut Wiz, disks: &[DiskItem], disk: usize, payload: &[u8]) -> Result<(), String> {
    let user_text = format!("name={}\npassword={}\n", w.name, w.pass);
    let auto_text = format!("autologin={}\n", if w.auto == 1 { 1 } else { 0 });

    prog(ui, w, disks, 0, 0, "Форматирование диска в FAT32...");
    let r = unsafe { fat32::format(disk, "MIUX") };
    r.map_err(String::from)?;
    step_done(ui, w, disks, 15, "Диск отформатирован (FAT32)");

    prog(ui, w, disks, 15, 0, "Создание папки MiUX...");
    let r = unsafe { fat32::create_dir(fat32::SYS_DIR) };
    r.map_err(String::from)?;
    step_done(ui, w, disks, 18, "Создана дерриктория MiUX");

    prog(ui, w, disks, 18, 0, "Копирование ядра MIUX.ELF...");
    let r = unsafe { fat32::save_file_in_dir(fat32::SYS_DIR, KERNEL_NAME, payload) };
    r.map_err(String::from)?;
    step_done(ui, w, disks, 58, "Ядро MIUX.ELF скопировано");

    prog(ui, w, disks, 58, 0, "Создание system.cfg...");
    let cfg_text = syscfg::defaults_text();
    let r = unsafe { fat32::save_file_in_dir(fat32::SYS_DIR, syscfg::FILE_NAME, cfg_text.as_bytes()) };
    r.map_err(String::from)?;
    step_done(ui, w, disks, 61, "Создан system.cfg");

    prog(ui, w, disks, 61, 0, "Создание пользователя...");
    let r = unsafe { fat32::create_subdir(fat32::SYS_DIR, fat32::USERS_DIR) };
    r.map_err(String::from)?;
    let r = unsafe { fat32::save_file_in_subdir(fat32::SYS_DIR, fat32::USERS_DIR, "user.cfg", user_text.as_bytes()) };
    r.map_err(String::from)?;
    let r = unsafe { fat32::save_file_in_subdir(fat32::SYS_DIR, fat32::USERS_DIR, "auto.cfg", auto_text.as_bytes()) };
    r.map_err(String::from)?;
    step_done(ui, w, disks, 68, "Создан пользователь /MiUX/USERS/");

    prog(ui, w, disks, 68, 0, "Создание папки ryles...");
    let r = unsafe { fat32::create_subdir(fat32::SYS_DIR, "ryles") };
    r.map_err(String::from)?;
    let ryles: [(&str, &[u8]); 4] = [("cfg.mp", RY_CFG), ("dir.mp", RY_DIR), ("mas.mp", RY_MAS), ("usr.mp", RY_USR)];
    for (rn, rd) in ryles.iter() {
        let r = unsafe { fat32::save_file_in_subdir(fat32::SYS_DIR, "ryles", rn, rd) };
        r.map_err(String::from)?;
    }
    step_done(ui, w, disks, 68, "Создана папка ryles (правила защиты системы)");

    prog(ui, w, disks, 68, 0, "Создание папки SYSTEMX64...");
    let r = unsafe { fat32::create_subdir(fat32::SYS_DIR, "SYSTEMX64") };
    r.map_err(String::from)?;
    let sysx: [(&str, &[u8]); 3] = [("mode.mp", SX_MODE), ("build.mp", SX_BUILD), ("calc.mp", SX_CALC)];
    for (sn, sd) in sysx.iter() {
        let r = unsafe { fat32::save_file_in_subdir(fat32::SYS_DIR, "SYSTEMX64", sn, sd) };
        r.map_err(String::from)?;
    }
    step_done(ui, w, disks, 68, "Создана дерриктория SYSTEMX64");

    prog(ui, w, disks, 68, 0, "Проверка записанных файлов...");
    let mut back: Vec<u8> = Vec::new();
    back.resize(payload.len(), 0);
    match unsafe { fat32::load_file_in_dir(fat32::SYS_DIR, KERNEL_NAME, &mut back) } {
        Ok(len) if len == payload.len() && back[..] == payload[..] => {}
        Ok(len) => {
            return Err(format!(
                "Проверка не пройдена: прочитано {} из {} байт или данные отличаются",
                len,
                payload.len()
            ));
        }
        Err(e) => return Err(String::from(e)),
    }
    drop(back);
    let mut ubuf = [0u8; 256];
    match unsafe { fat32::load_file_in_subdir(fat32::SYS_DIR, fat32::USERS_DIR, "user.cfg", &mut ubuf) } {
        Ok(len) if len == user_text.len() && &ubuf[..len] == user_text.as_bytes() => {}
        Ok(_) => return Err(String::from("Проверка не пройдена: user.cfg записан неверно")),
        Err(e) => return Err(String::from(e)),
    }
    let mut abuf = [0u8; 64];
    match unsafe { fat32::load_file_in_subdir(fat32::SYS_DIR, fat32::USERS_DIR, "auto.cfg", &mut abuf) } {
        Ok(len) if len == auto_text.len() && &abuf[..len] == auto_text.as_bytes() => {}
        Ok(_) => return Err(String::from("Проверка не пройдена: auto.cfg записан неверно")),
        Err(e) => return Err(String::from(e)),
    }
    step_done(ui, w, disks, 85, "Проверка пройдена");

    prog(ui, w, disks, 85, 0, "Установка загрузчика GRUB...");
    {
        let mut cb = |done: usize, total: usize| {
            let total = if total == 0 { 1 } else { total };
            w.cur = (done * 100 / total) as u32;
            w.overall = 85 + (done * 15 / total) as u32;
            ui.present(w, disks);
        };
        let r = unsafe { install_grub(disk, &mut cb) };
        r.map_err(String::from)?;
    }
    step_done(ui, w, disks, 100, "Загрузчик GRUB установлен");
    Ok(())
}


pub fn run(g: &mut Graphics, info_addr: u32) -> ! {
    let cols = g.width / CW;
    let rows = g.height / CH;
    let mut ui = Ui { g, cols, rows };
    let mut hw = Hw { shift: false, ext: false };

    while inb(0x64) & 1 != 0 {
        let _ = inb(0x60);
    }

    let mut w = Wiz::new();

    let payload = unsafe { find_payload(info_addr) };
    if payload.is_none() {
        w.page = P_ERROR;
        w.err = String::from("Модуль ядра не загружен GRUB. Запустите установку через пункт MiUX Install.");
        unsafe {
            let total = *(info_addr as *const u32) as usize;
            let mut off = 8usize;
            while off + 8 <= total {
                let p = (info_addr as usize + off) as *const u32;
                let (ty, sz) = (*p, *p.add(1) as usize);
                if ty == 0 || sz < 8 {
                    break;
                }
                w.fail_lines.push(format!("multiboot tag type {} size {}", ty, sz));
                off += (sz + 7) & !7;
            }
        }
    }

    let mut disks: Vec<DiskItem> = Vec::new();
    unsafe {
        let total = ata::disk_count();
        for i in 0..total {
            if disks.len() >= 8 {
                break;
            }
            if let Some(d) = ata::get_disk(i) {
                if d.present && d.dev_type == ata::DeviceType::ATA && d.sectors > 0 {
                    let mut model = String::new();
                    for &b in d.model.iter() {
                        if b >= 0x20 && b < 0x7F {
                            model.push(b as char);
                        }
                    }
                    let label = format!("{}  -  {} MB", model.trim(), d.sectors / 2048);
                    disks.push(DiskItem { idx: i, label });
                }
            }
        }
    }
    if w.page != P_ERROR && disks.is_empty() {
        w.page = P_ERROR;
        w.err = String::from("Жёсткий диск не найден.");
        for line in ata::debug_info().split('\n') {
            if !line.is_empty() {
                w.fail_lines.push(String::from(line));
            }
        }
    }

    ui.present(&w, &disks);

    let nd = disks.len();
    loop {
        let mut act = Act::None;
        let mut redraw = false;
        let mut n = 0;
        while n < 64 {
            n += 1;
            match hw.poll() {
                None => break,
                Some(k) => match on_key(&mut w, k, nd) {
                    Act::None => {}
                    Act::Redraw => redraw = true,
                    a => {
                        act = a;
                        break;
                    }
                },
            }
        }

        match act {
            Act::Reboot => reboot(),
            Act::Install => {
                if let Some(p) = payload {
                    w.page = P_INSTALL;
                    w.overall = 0;
                    w.cur = 0;
                    w.status = String::new();
                    w.log.clear();
                    ui.present(&w, &disks);
                    let disk = disks[w.disk_pick].idx;
                    match do_install(&mut ui, &mut w, &disks, disk, p) {
                        Ok(()) => {
                            w.page = P_DONE;
                            w.btn = 0;
                        }
                        Err(e) => {
                            w.page = P_ERROR;
                            w.err = e;
                            w.btn = 0;
                        }
                    }
                    ui.present(&w, &disks);
                }
            }
            _ => {
                if redraw {
                    ui.present(&w, &disks);
                }
            }
        }
        core::hint::spin_loop();
    }
}
