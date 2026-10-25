

use alloc::format;
use alloc::string::String;

use crate::fs::fat32;
use crate::graphics::Color;


pub const FILE_NAME: &str = "system.cfg";


#[derive(Clone, Copy, PartialEq)]
struct Settings {
    wallpaper: u32,                   
    use_bmp: bool,                   
    bmp_index: usize,          
    live: bool,
    live_interval: u32,    
    theme: usize,               
    theme_selector: usize,          
    wm2: bool,                        
    russian: bool,                    
    icons: [u32; 9],
}

impl Settings {
    
    const DEFAULT: Settings = Settings {
        wallpaper: 0x059400,
        use_bmp: true,
        bmp_index: 0,
        live: false,
        live_interval: 15,
        theme: 3,
        theme_selector: 0,
        wm2: false,
        russian: false,
        icons: [u32::MAX; 9],
    };
}

static mut INSTALLED: bool = false;
static mut LAST: Settings = Settings::DEFAULT;


pub fn defaults_text() -> String {
    render(&Settings::DEFAULT)
}


pub const MAX_SHORTCUTS: usize = 24;
pub const SHORTCUT_PATH_MAX: usize = 64;

#[derive(Clone, Copy)]
pub struct Shortcut {
    pub disk: usize,
    pub path: [u8; SHORTCUT_PATH_MAX],
    pub len: usize,
    pub pos: u32,
    pub is_dir: bool,
}

impl Shortcut {
    const EMPTY: Shortcut = Shortcut { disk: 0, path: [0; SHORTCUT_PATH_MAX], len: 0, pos: u32::MAX, is_dir: false };

    pub fn path_str(&self) -> &str {
        core::str::from_utf8(&self.path[..self.len]).unwrap_or("")
    }

    pub fn label(&self) -> &str {
        let p = self.path_str();
        match p.rfind('/') {
            Some(i) => &p[i + 1..],
            None => p,
        }
    }
}

static mut SHORTCUTS: [Shortcut; MAX_SHORTCUTS] = [Shortcut::EMPTY; MAX_SHORTCUTS];
static mut SHORTCUT_COUNT: usize = 0;

pub fn shortcut_count() -> usize {
    unsafe { SHORTCUT_COUNT }
}

pub fn shortcut(i: usize) -> Option<Shortcut> {
    unsafe {
        if i < SHORTCUT_COUNT { Some(SHORTCUTS[i]) } else { None }
    }
}

fn valid_path(path: &[u8]) -> bool {
    if path.len() < 2 || path.len() > SHORTCUT_PATH_MAX || path[0] != b'/' {
        return false;
    }
    path.iter().all(|&b| b >= 0x20 && b < 0x7F)
}

pub fn add_shortcut(disk: usize, path: &str) -> Result<(), &'static str> {
    add_shortcut_kind(disk, path, false)
}

pub fn add_folder_shortcut(disk: usize, path: &str) -> Result<(), &'static str> {
    add_shortcut_kind(disk, path, true)
}

fn add_shortcut_kind(disk: usize, path: &str, is_dir: bool) -> Result<(), &'static str> {
    let pb = path.as_bytes();
    if pb.len() > SHORTCUT_PATH_MAX {
        return Err("Path too long");
    }
    if !valid_path(pb) {
        return Err("Bad path");
    }
    unsafe {
        for i in 0..SHORTCUT_COUNT {
            if SHORTCUTS[i].disk == disk && SHORTCUTS[i].path_str().eq_ignore_ascii_case(path) {
                return Err("Shortcut already exists");
            }
        }
        if SHORTCUT_COUNT >= MAX_SHORTCUTS {
            return Err("Too many shortcuts");
        }
        let mut sc = Shortcut::EMPTY;
        sc.disk = disk;
        sc.path[..pb.len()].copy_from_slice(pb);
        sc.len = pb.len();
        sc.is_dir = is_dir;
        SHORTCUTS[SHORTCUT_COUNT] = sc;
        SHORTCUT_COUNT += 1;
        if INSTALLED {
            if !save(&capture()) {
                SHORTCUT_COUNT -= 1;
                return Err("Cannot write SYSTEM.CFG");
            }
        }
    }
    Ok(())
}

pub fn remove_shortcuts_for(disk: usize, path: &str) {
    unsafe {
        let mut removed = false;
        let mut i = 0usize;
        while i < SHORTCUT_COUNT {
            if SHORTCUTS[i].disk == disk && SHORTCUTS[i].path_str().eq_ignore_ascii_case(path) {
                let mut j = i;
                while j + 1 < SHORTCUT_COUNT {
                    SHORTCUTS[j] = SHORTCUTS[j + 1];
                    j += 1;
                }
                SHORTCUT_COUNT -= 1;
                removed = true;
            } else {
                i += 1;
            }
        }
        if removed && INSTALLED {
            let _ = save(&capture());
        }
    }
}

pub fn remove_shortcut_at(idx: usize) -> bool {
    unsafe {
        if idx >= SHORTCUT_COUNT {
            return false;
        }
        let mut j = idx;
        while j + 1 < SHORTCUT_COUNT {
            SHORTCUTS[j] = SHORTCUTS[j + 1];
            j += 1;
        }
        SHORTCUT_COUNT -= 1;
        if INSTALLED {
            let _ = save(&capture());
        }
        true
    }
}

pub fn set_shortcut_pos(idx: usize, x: usize, y: usize) {
    unsafe {
        if idx >= SHORTCUT_COUNT {
            return;
        }
        SHORTCUTS[idx].pos = (((x & 0xFFFF) as u32) << 16) | ((y & 0xFFFF) as u32);
        if INSTALLED {
            let _ = save(&capture());
        }
    }
}

fn parse_xy(v: &str) -> Option<(usize, u32)> {
    let colon = v.find(':')?;
    let n = v[..colon].trim().parse::<usize>().ok()?;
    let rest = &v[colon + 1..];
    let comma = rest.find(',')?;
    let x = rest[..comma].trim().parse::<u32>().ok()?;
    let y = rest[comma + 1..].trim().parse::<u32>().ok()?;
    if x > 0xFFFE || y > 0xFFFF {
        return None;
    }
    Some((n, (x << 16) | y))
}

fn load_shortcuts(text: &str) {
    unsafe {
        SHORTCUT_COUNT = 0;
        for line in text.lines() {
            let line = line.trim();
            if !line.starts_with("shortcut=") {
                continue;
            }
            let val = line["shortcut=".len()..].trim();
            let colon = match val.find(':') {
                Some(p) => p,
                None => continue,
            };
            let disk = match val[..colon].trim().parse::<usize>() {
                Ok(d) => d,
                Err(_) => continue,
            };
            if disk >= 8 {
                continue;
            }
            let path = val[colon + 1..].trim().as_bytes();
            if !valid_path(path) {
                continue;
            }
            if SHORTCUT_COUNT >= MAX_SHORTCUTS {
                break;
            }
            let mut sc = Shortcut::EMPTY;
            sc.disk = disk;
            sc.path[..path.len()].copy_from_slice(path);
            sc.len = path.len();
            SHORTCUTS[SHORTCUT_COUNT] = sc;
            SHORTCUT_COUNT += 1;
        }
        for line in text.lines() {
            let line = line.trim();
            if !line.starts_with("shortcut_xy=") {
                continue;
            }
            if let Some((n, p)) = parse_xy(line["shortcut_xy=".len()..].trim()) {
                if n < SHORTCUT_COUNT {
                    SHORTCUTS[n].pos = p;
                }
            }
        }
        for line in text.lines() {
            let line = line.trim();
            if !line.starts_with("shortcut_dir=") {
                continue;
            }
            if let Ok(n) = line["shortcut_dir=".len()..].trim().parse::<usize>() {
                if n < SHORTCUT_COUNT {
                    SHORTCUTS[n].is_dir = true;
                }
            }
        }
    }
}

fn render(s: &Settings) -> String {
    let mut out = format!(
        "# MiUX system settings.\n\
         wallpaper_color={:06X}\n\
         use_bmp_wallpaper={}\n\
         bmp_wallpaper={}\n\
         live_wallpaper={}\n\
         live_wallpaper_interval={}\n\
         theme={}\n\
         theme_selector={}\n\
         wm2_theme={}\n\
         language={}\n",
        s.wallpaper & 0xFFFFFF,
        s.use_bmp,
        s.bmp_index,
        s.live,
        s.live_interval,
        s.theme,
        s.theme_selector,
        s.wm2,
        if s.russian { "ru" } else { "en" },
    );
    for n in 0..9 {
        if s.icons[n] != u32::MAX {
            out.push_str(&format!("icon={}:{},{}\n", n, s.icons[n] >> 16, s.icons[n] & 0xFFFF));
        }
    }
    unsafe {
        for i in 0..SHORTCUT_COUNT {
            let sc = SHORTCUTS[i];
            out.push_str(&format!("shortcut={}:{}\n", sc.disk, sc.path_str()));
        }
        for i in 0..SHORTCUT_COUNT {
            let sc = SHORTCUTS[i];
            if sc.pos != u32::MAX {
                out.push_str(&format!("shortcut_xy={}:{},{}\n", i, sc.pos >> 16, sc.pos & 0xFFFF));
            }
        }
        for i in 0..SHORTCUT_COUNT {
            if SHORTCUTS[i].is_dir {
                out.push_str(&format!("shortcut_dir={}\n", i));
            }
        }
    }
    out
}

fn parse_bool(v: &str) -> Option<bool> {
    match v {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}


fn parse(text: &str, base: Settings) -> Settings {
    let mut s = base;
    s.icons = [u32::MAX; 9];
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let eq = match line.find('=') {
            Some(p) => p,
            None => continue,
        };
        let key = line[..eq].trim();
        let val = line[eq + 1..].trim();
        match key {
            "wallpaper_color" => {
                if let Ok(v) = u32::from_str_radix(val, 16) {
                    s.wallpaper = v & 0xFFFFFF;
                }
            }
            "use_bmp_wallpaper" => {
                if let Some(b) = parse_bool(val) { s.use_bmp = b; }
            }
            "bmp_wallpaper" => {
                if let Ok(v) = val.parse::<usize>() {
                    if v < 3 { s.bmp_index = v; }
                }
            }
            "live_wallpaper" => {
                if let Some(b) = parse_bool(val) { s.live = b; }
            }
            "live_wallpaper_interval" => {
                if let Ok(v) = val.parse::<u32>() {
                    s.live_interval = v.max(crate::LIVE_WALLPAPER_MIN_SEC).min(crate::LIVE_WALLPAPER_MAX_SEC);
                }
            }
            "theme" => {
                if let Ok(v) = val.parse::<usize>() {
                    if v < 5 { s.theme = v; }
                }
            }
            "theme_selector" => {
                if let Ok(v) = val.parse::<usize>() {
                    if v < 5 { s.theme_selector = v; }
                }
            }
            "wm2_theme" => {
                if let Some(b) = parse_bool(val) { s.wm2 = b; }
            }
            "icon" => {
                if let Some((n, p)) = parse_xy(val) {
                    if n < 9 { s.icons[n] = p; }
                }
            }
            "language" => {
                match val {
                    "ru" => s.russian = true,
                    "en" => s.russian = false,
                    _ => {}
                }
            }
            _ => {}
        }
    }
    s
}


fn capture() -> Settings {
    unsafe {
        let wp = crate::WALLPAPER;
        Settings {
            wallpaper: wp.to_u32(),
            use_bmp: crate::USE_BMP_WALLPAPER,
            bmp_index: crate::CURRENT_BMP_INDEX,
            live: crate::LIVE_WALLPAPER_ENABLED,
            live_interval: crate::LIVE_WALLPAPER_INTERVAL_SEC,
            theme: crate::wm::theme_index(),
            theme_selector: crate::THEME_IDX,
            wm2: crate::USE_WM2_THEME,
            russian: crate::web::keys::RUSSIAN,
            icons: crate::SYS_ICON_POS,
        }
    }
}


fn apply_settings(s: &Settings) {
    unsafe {
        crate::WALLPAPER = Color::rgb((s.wallpaper >> 16) as u8, (s.wallpaper >> 8) as u8, s.wallpaper as u8);
        crate::USE_BMP_WALLPAPER = s.use_bmp;
        crate::CURRENT_BMP_INDEX = s.bmp_index;
        crate::LIVE_WALLPAPER_ENABLED = s.live;
        crate::LIVE_WALLPAPER_INTERVAL_SEC = s.live_interval;
        crate::LIVE_WALLPAPER_ELAPSED_SEC = 0;
        crate::THEME_IDX = s.theme_selector;
        crate::USE_WM2_THEME = s.wm2;
        crate::web::keys::RUSSIAN = s.russian;
        crate::SYS_ICON_POS = s.icons;
    }
    crate::wm::set_theme(s.theme);
}


fn same(a: &Settings, b: &Settings) -> bool {
    let mut x = *a;
    let mut y = *b;
    if x.live && y.live {
        x.bmp_index = 0;
        y.bmp_index = 0;
    }
    x == y
}

fn read_file() -> Option<String> {
    let mut buf = [0u8; 4096];
    let n = match unsafe { fat32::load_file_in_dir(fat32::SYS_DIR, FILE_NAME, &mut buf) } {
        Ok(n) => n,
        Err(_) => match unsafe { fat32::load_file(FILE_NAME, &mut buf) } {
            Ok(n) => n,
            Err(_) => return None,
        },
    };
    core::str::from_utf8(&buf[..n]).ok().map(String::from)
}

fn save(s: &Settings) -> bool {
    let text = render(s);
    unsafe { fat32::save_file_in_dir(fat32::SYS_DIR, FILE_NAME, text.as_bytes()).is_ok() }
}


pub fn booted_from_disk(info_addr: u32) -> bool {
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
            if ty == 3 {
                return false;
            }
            off += (sz + 7) & !7;
        }
    }
    true
}


pub fn init(info_addr: u32) {
    unsafe {
        INSTALLED = booted_from_disk(info_addr);
        if !INSTALLED {
            return;
        }
    }
    match read_file() {
        Some(text) => {
            let s = parse(&text, capture());
            apply_settings(&s);
            load_shortcuts(&text);
        }
        
        None => { save(&capture()); }
    }
    unsafe { LAST = capture(); }
}


pub fn apply() {
    unsafe {
        if !INSTALLED {
            return;
        }
    }
    if let Some(text) = read_file() {
        let s = parse(&text, capture());
        apply_settings(&s);
        load_shortcuts(&text);
    }
    unsafe { LAST = capture(); }
}


pub fn poll() {
    unsafe {
        if !INSTALLED {
            return;
        }
        let cur = capture();
        if same(&LAST, &cur) {
            return;
        }
        LAST = cur;
        let _ = save(&cur);
    }
}
