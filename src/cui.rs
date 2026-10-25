use crate::graphics::{self, Graphics};

#[repr(C)]
pub struct UiTheme {
    pub window_bg: u32,
    pub button_face: u32,
    pub button_text: u32,
    pub taskbar_bg: u32,
    pub start_menu_bg: u32,
    pub taskbar_text: u32,
    pub desktop_icon_text: u32,
    pub border_light: u32,
    pub border_dark: u32,
    pub id: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct UiWindow {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub title: *const u8,
    pub title_len: usize,
    pub state: i32,
    pub active: i32,
    pub z: u32,
}

impl UiWindow {
    pub const EMPTY: UiWindow = UiWindow { x: 0, y: 0, w: 0, h: 0, title: core::ptr::null(), title_len: 0, state: 1, active: 0, z: 0 };
}

pub const HIT_NONE: i32 = 0;
pub const HIT_CLIENT: i32 = 1;
pub const HIT_TITLE: i32 = 2;
pub const HIT_CLOSE: i32 = 3;
pub const HIT_MAX: i32 = 4;
pub const HIT_MIN: i32 = 5;
pub const HIT_RESIZE: i32 = 6;

pub const TB_NONE: i32 = -1;
pub const TB_START: i32 = -2;
pub const TB_CLOCK: i32 = -3;
pub const TB_EMPTY: i32 = -4;
pub const TB_LANG: i32 = -5;

pub const TITLEBAR_H: usize = 30;
pub const TASKBAR_H: usize = 30;

extern "C" {
    fn ui_init(bb: *mut u32, w: i32, h: i32);
    fn ui_set_theme(t: *const UiTheme);
    fn ui_raised(x: i32, y: i32, w: i32, h: i32);
    fn ui_sunken(x: i32, y: i32, w: i32, h: i32);
    fn ui_draw_window(w: *const UiWindow);
    fn ui_window_hit(w: *const UiWindow, px: i32, py: i32) -> i32;
    fn ui_zorder(wins: *const UiWindow, n: i32, order: *mut i32) -> i32;
    fn ui_top_at(wins: *const UiWindow, n: i32, px: i32, py: i32) -> i32;
    fn ui_draw_taskbar(wins: *const UiWindow, n: i32, menu_open: i32, clock: *const u8, clock_len: usize);
    fn ui_taskbar_hit(wins: *const UiWindow, n: i32, px: i32, py: i32, clock_len: usize) -> i32;
    fn ui_draw_start_menu();
    fn ui_start_menu_contains(px: i32, py: i32) -> i32;
    fn ui_start_menu_item(px: i32, py: i32) -> i32;
}

static mut GFX: *mut Graphics = core::ptr::null_mut();
static mut SCREEN: (usize, usize) = (0, 0);

#[no_mangle]
pub extern "C" fn miux_ui_lang() -> i32 {
    if unsafe { crate::web::keys::RUSSIAN } { 1 } else { 0 }
}

#[no_mangle]
pub extern "C" fn miux_ui_text(x: i32, y: i32, s: *const u8, len: usize, fg: u32, bg: u32) {
    unsafe {
        if GFX.is_null() || s.is_null() || x < 0 || y < 0 { return; }
        let bytes = core::slice::from_raw_parts(s, len);
        let text = match core::str::from_utf8(bytes) { Ok(t) => t, Err(e) => core::str::from_utf8_unchecked(&bytes[..e.valid_up_to()]) };
        crate::font::draw_ui_text(&mut *GFX, x as usize, y as usize, text, fg, bg);
    }
}

#[inline]
fn with<R>(gfx: &mut Graphics, f: impl FnOnce() -> R) -> R {
    unsafe {
        if SCREEN != (gfx.width, gfx.height) {
            SCREEN = (gfx.width, gfx.height);
            ui_init(core::ptr::addr_of_mut!(graphics::BACKBUFFER) as *mut u32, gfx.width as i32, gfx.height as i32);
            sync_theme();
        }
        GFX = gfx as *mut Graphics;
        let r = f();
        GFX = core::ptr::null_mut();
        r
    }
}

fn ready() -> bool { unsafe { SCREEN.0 != 0 } }

pub fn sync_theme() {
    let t = crate::wm::get_theme();
    let th = UiTheme {
        window_bg: t.window_bg.to_u32(),
        button_face: t.button_face.to_u32(),
        button_text: t.button_text.to_u32(),
        taskbar_bg: t.taskbar_bg.to_u32(),
        start_menu_bg: t.start_menu_bg.to_u32(),
        taskbar_text: t.taskbar_text.to_u32(),
        desktop_icon_text: t.desktop_icon_text.to_u32(),
        border_light: t.border_light.to_u32(),
        border_dark: t.border_dark.to_u32(),
        id: crate::wm::theme_index() as i32,
    };
    unsafe { ui_set_theme(&th) }
}

pub fn raised(gfx: &mut Graphics, x: usize, y: usize, w: usize, h: usize) {
    with(gfx, || unsafe { ui_raised(x as i32, y as i32, w as i32, h as i32) })
}
pub fn sunken(gfx: &mut Graphics, x: usize, y: usize, w: usize, h: usize) {
    with(gfx, || unsafe { ui_sunken(x as i32, y as i32, w as i32, h as i32) })
}
pub fn draw_window(gfx: &mut Graphics, w: &UiWindow) {
    with(gfx, || unsafe { ui_draw_window(w) })
}
pub fn window_hit(w: &UiWindow, px: usize, py: usize) -> i32 {
    unsafe { ui_window_hit(w, px as i32, py as i32) }
}
pub fn zorder(wins: &[UiWindow], order: &mut [i32; 8]) -> usize {
    let n = wins.len().min(8);
    unsafe { ui_zorder(wins.as_ptr(), n as i32, order.as_mut_ptr()) as usize }
}
pub fn top_at(wins: &[UiWindow], px: usize, py: usize) -> Option<usize> {
    let r = unsafe { ui_top_at(wins.as_ptr(), wins.len() as i32, px as i32, py as i32) };
    if r < 0 { None } else { Some(r as usize) }
}
pub fn draw_taskbar(gfx: &mut Graphics, wins: &[UiWindow], menu_open: bool, clock: &str) {
    with(gfx, || unsafe { ui_draw_taskbar(wins.as_ptr(), wins.len() as i32, menu_open as i32, clock.as_ptr(), clock.len()) })
}
pub fn taskbar_hit(wins: &[UiWindow], px: usize, py: usize, clock_len: usize) -> i32 {
    if !ready() { return TB_NONE; }
    unsafe { ui_taskbar_hit(wins.as_ptr(), wins.len() as i32, px as i32, py as i32, clock_len) }
}
pub fn draw_start_menu(gfx: &mut Graphics) {
    with(gfx, || unsafe { ui_draw_start_menu() })
}
pub fn start_menu_contains(px: usize, py: usize) -> bool {
    ready() && unsafe { ui_start_menu_contains(px as i32, py as i32) != 0 }
}
pub fn start_menu_item(px: usize, py: usize) -> Option<usize> {
    if !ready() { return None; }
    let r = unsafe { ui_start_menu_item(px as i32, py as i32) };
    if r < 0 { None } else { Some(r as usize) }
}
