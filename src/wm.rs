use crate::graphics::{Graphics, Color};
use crate::cui;
use crate::bmp::BmpImage;


pub struct Theme {
    pub titlebar_active: Color,
    pub titlebar_inactive: Color,
    pub window_bg: Color,
    pub button_face: Color,
    pub button_text: Color,
    pub taskbar_bg: Color,
    pub start_menu_bg: Color,
    pub taskbar_text: Color,
    pub desktop_icon_text: Color,
    pub border_light: Color,
    pub border_dark: Color,
    pub titlebar_gradient_top_active: Color,
    pub titlebar_gradient_bottom_active: Color,
    pub titlebar_gradient_top_inactive: Color,
    pub titlebar_gradient_bottom_inactive: Color,
}

pub static THEMES: [Theme; 5] = [

    Theme {
        titlebar_active: Color::rgb(220, 220, 230),
        titlebar_inactive: Color::rgb(235, 235, 240),
        window_bg: Color::rgb(248, 248, 252),
        button_face: Color::rgb(210, 210, 220),
        button_text: Color::BLACK,
        taskbar_bg: Color::rgb(200, 200, 210),
        start_menu_bg: Color::rgb(245, 245, 250),
        taskbar_text: Color::BLACK,
        desktop_icon_text: Color::WHITE,
        border_light: Color::rgb(255, 255, 255),
        border_dark: Color::rgb(120, 120, 130),
        titlebar_gradient_top_active: Color::rgb(150, 180, 210),
        titlebar_gradient_bottom_active: Color::rgb(100, 130, 170),
        titlebar_gradient_top_inactive: Color::rgb(200, 200, 210),
        titlebar_gradient_bottom_inactive: Color::rgb(160, 160, 170),
    },
    Theme {
        titlebar_active: Color::rgb(60, 60, 65),
        titlebar_inactive: Color::rgb(75, 75, 80),
        window_bg: Color::rgb(35, 35, 40),
        button_face: Color::rgb(80, 80, 90),
        button_text: Color::WHITE,
        taskbar_bg: Color::rgb(50, 50, 55),
        start_menu_bg: Color::rgb(45, 45, 50),
        taskbar_text: Color::WHITE,
        desktop_icon_text: Color::WHITE,
        border_light: Color::rgb(100, 100, 110),
        border_dark: Color::rgb(30, 30, 35),
        titlebar_gradient_top_active: Color::rgb(80, 90, 110),
        titlebar_gradient_bottom_active: Color::rgb(50, 60, 80),
        titlebar_gradient_top_inactive: Color::rgb(90, 90, 95),
        titlebar_gradient_bottom_inactive: Color::rgb(65, 65, 75),
    },
    Theme {
        titlebar_active: Color::rgb(140, 140, 145),
        titlebar_inactive: Color::rgb(160, 160, 165),
        window_bg: Color::rgb(240, 240, 245),
        button_face: Color::rgb(180, 180, 190),
        button_text: Color::BLACK,
        taskbar_bg: Color::rgb(170, 170, 180),
        start_menu_bg: Color::rgb(235, 235, 240),
        taskbar_text: Color::BLACK,
        desktop_icon_text: Color::BLACK,
        border_light: Color::rgb(220, 220, 230),
        border_dark: Color::rgb(100, 100, 110),
        titlebar_gradient_top_active: Color::rgb(160, 170, 185),
        titlebar_gradient_bottom_active: Color::rgb(120, 130, 145),
        titlebar_gradient_top_inactive: Color::rgb(190, 195, 205),
        titlebar_gradient_bottom_inactive: Color::rgb(160, 165, 175),
    },

    Theme {
        titlebar_active: Color::rgb(48, 48, 48),
        titlebar_inactive: Color::rgb(65, 65, 65),
        window_bg: Color::rgb(246, 246, 246),
        button_face: Color::rgb(220, 220, 220),
        button_text: Color::rgb(50, 50, 50),
        taskbar_bg: Color::rgb(48, 48, 48),
        start_menu_bg: Color::rgb(50, 50, 50),
        taskbar_text: Color::rgb(230, 230, 230),
        desktop_icon_text: Color::rgb(255, 255, 255),
        border_light: Color::rgb(80, 80, 80),
        border_dark: Color::rgb(30, 30, 30),
        titlebar_gradient_top_active: Color::rgb(48, 48, 48),
        titlebar_gradient_bottom_active: Color::rgb(48, 48, 48),
        titlebar_gradient_top_inactive: Color::rgb(65, 65, 65),
        titlebar_gradient_bottom_inactive: Color::rgb(65, 65, 65),
    },

    Theme {
        titlebar_active: Color::rgb(10, 36, 106),
        titlebar_inactive: Color::rgb(128, 128, 128),
        window_bg: Color::rgb(212, 208, 200),
        button_face: Color::rgb(212, 208, 200),
        button_text: Color::BLACK,
        taskbar_bg: Color::rgb(212, 208, 200),
        start_menu_bg: Color::rgb(212, 208, 200),
        taskbar_text: Color::BLACK,
        desktop_icon_text: Color::WHITE,
        border_light: Color::rgb(255, 255, 255),
        border_dark: Color::rgb(128, 128, 128),
        titlebar_gradient_top_active: Color::rgb(10, 36, 106),
        titlebar_gradient_bottom_active: Color::rgb(166, 202, 240),
        titlebar_gradient_top_inactive: Color::rgb(128, 128, 128),
        titlebar_gradient_bottom_inactive: Color::rgb(192, 192, 192),
    },
];

static mut CURRENT_THEME: usize = 3;

#[link_section = ".rodata"]
#[used]
static T1_BMP: &[u8] = include_bytes!("t1.bmp");
#[link_section = ".rodata"]
#[used]
static T2_BMP: &[u8] = include_bytes!("t2.bmp");
#[link_section = ".rodata"]
#[used]
static T3_BMP: &[u8] = include_bytes!("t3.bmp");

static mut THEME_IMAGES: [Option<BmpImage>; 4] = [None, None, None, None];

pub fn init_theme_images() {
    unsafe {
        THEME_IMAGES[0] = BmpImage::from_bytes(T1_BMP);
        THEME_IMAGES[1] = BmpImage::from_bytes(T2_BMP);
        THEME_IMAGES[2] = BmpImage::from_bytes(T3_BMP);
        THEME_IMAGES[3] = BmpImage::from_bytes(T1_BMP);
    }
}

pub fn set_theme(idx: usize) {
    unsafe { 
        CURRENT_THEME = idx % THEMES.len();
    }
    cui::sync_theme();
}

pub fn get_theme() -> &'static Theme {
    &THEMES[unsafe { CURRENT_THEME }]
}

pub fn is_ubuntu_theme() -> bool {
    unsafe { CURRENT_THEME == 3 }
}


pub fn theme_index() -> usize {
    unsafe { CURRENT_THEME }
}

pub fn draw_raised_rect(gfx: &mut Graphics, x: usize, y: usize, w: usize, h: usize) {
    cui::raised(gfx, x, y, w, h);
}

pub fn draw_sunken_rect(gfx: &mut Graphics, x: usize, y: usize, w: usize, h: usize) {
    cui::sunken(gfx, x, y, w, h);
}

fn draw_theme_icon(gfx: &mut Graphics, x: usize, y: usize, size: usize, theme_idx: usize) {
    unsafe {
        if let Some(ref bmp) = THEME_IMAGES[theme_idx] {
            let row_size = ((bmp.width * 3 + 3) / 4) * 4;
            for dy in 0..size.min(bmp.height) {
                let bmp_y = (bmp.height - 1).saturating_sub(dy);
                let bmp_row_start = bmp_y * row_size;
                for dx in 0..size.min(bmp.width) {
                    let pixel_offset = bmp_row_start + dx * 3;
                    if pixel_offset + 2 < bmp.data.len() {
                        let b = bmp.data[pixel_offset] as u32;
                        let g = bmp.data[pixel_offset + 1] as u32;
                        let r = bmp.data[pixel_offset + 2] as u32;
                        let color = (r << 16) | (g << 8) | b;
                        if color != 0xFF00FF {
                            gfx.put_pixel(x + dx, y + dy, color);
                        }
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum WindowState {
    Normal,
    Minimized,
    Maximized,
}

pub struct Window {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub title: &'static str,
    pub is_dragging: bool,
    pub drag_off_x: usize,
    pub drag_off_y: usize,
    pub state: WindowState,
    pub restore_rect: (usize, usize, usize, usize),
    pub z: u32,
}

impl Window {
    pub fn new(x: usize, y: usize, w: usize, h: usize, title: &'static str) -> Self {
        Self {
            x, y, width: w, height: h, title,
            is_dragging: false,
            drag_off_x: 0, drag_off_y: 0,
            state: WindowState::Normal,
            restore_rect: (x, y, w, h),
            z: 0,
        }
    }

    pub fn ui(&self, active: bool) -> cui::UiWindow {
        let shown: &str = if self.title == "HTML App" { crate::web::app::app_title() } else { self.title };
        cui::UiWindow {
            x: self.x as i32,
            y: self.y as i32,
            w: self.width as i32,
            h: self.height as i32,
            title: shown.as_ptr(),
            title_len: shown.len(),
            state: match self.state { WindowState::Normal => 0, WindowState::Minimized => 1, WindowState::Maximized => 2 },
            active: active as i32,
            z: self.z,
        }
    }

    pub fn draw(&self, gfx: &mut Graphics, active: bool) {
        cui::draw_window(gfx, &self.ui(active));
    }

    fn hit(&self, x: usize, y: usize) -> i32 { cui::window_hit(&self.ui(false), x, y) }

    pub fn contains(&self, x: usize, y: usize) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }
    pub fn is_titlebar(&self, x: usize, y: usize) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + cui::TITLEBAR_H
    }
    pub fn is_close_button(&self, x: usize, y: usize) -> bool { self.hit(x, y) == cui::HIT_CLOSE }
    pub fn is_maximize_button(&self, x: usize, y: usize) -> bool { self.hit(x, y) == cui::HIT_MAX }
    pub fn is_minimize_button(&self, x: usize, y: usize) -> bool { self.hit(x, y) == cui::HIT_MIN }
    pub fn is_resize_handle(&self, x: usize, y: usize) -> bool { self.hit(x, y) == cui::HIT_RESIZE }
}

pub struct Menu {
    pub open: bool,
    items: [&'static str; 5],
}

impl Menu {
    pub const fn new() -> Self {
        Self { open: false, items: ["Terminal", "Files", "Settings", "Calculator", "Shutdown"] }
    }
    pub fn draw(&self, gfx: &mut Graphics) {
        if self.open { cui::draw_start_menu(gfx); }
    }
    pub fn contains(&self, x: usize, y: usize, _gfx: &Graphics) -> bool {
        self.open && cui::start_menu_contains(x, y)
    }
    pub fn handle_click(&self, _gfx: &Graphics, cx: usize, cy: usize) -> Option<&'static str> {
        if !self.open { return None; }
        cui::start_menu_item(cx, cy).map(|i| self.items[i])
    }
}

pub static mut ON_WINDOW_CLOSED: Option<fn(usize)> = None;

pub struct WindowManager {
    pub windows: [Option<Window>; 8],
    pub window_count: usize,
    pub dragged_window: Option<usize>,
    pub resizing_window: Option<usize>,
    pub active_window: Option<usize>,
    next_z: u32,
    pub start_menu: Menu,
    pub screen_w: usize,
    pub screen_h: usize,
}

impl WindowManager {
    pub const fn new() -> Self {
        const NONE: Option<Window> = None;
        Self {
            windows: [NONE; 8],
            window_count: 0,
            dragged_window: None,
            resizing_window: None,
            active_window: None,
            next_z: 1,
            start_menu: Menu::new(),
            screen_w: 800,
            screen_h: 600,
        }
    }

    pub fn set_screen_size(&mut self, w: usize, h: usize) {
        self.screen_w = w;
        self.screen_h = h;
    }

    fn scale_size(&self, w: usize, h: usize) -> (usize, usize) {
        let (sw, sh) = (self.screen_w, self.screen_h);
        let (num, den) = if sw * 600 <= sh * 800 { (sw, 800) } else { (sh, 600) };
        let nw = (w * num / den).min(sw * 9 / 10);
        let nh = (h * num / den).min(sh * 9 / 10);
        (nw.max(300).min(sw), nh.max(200).min(sh.saturating_sub(cui::TASKBAR_H)))
    }

    pub fn ui_windows(&self, out: &mut [cui::UiWindow; 8]) -> usize {
        let n = self.window_count.min(8);
        for i in 0..n {
            out[i] = match self.windows[i] {
                Some(ref w) => w.ui(self.active_window == Some(i)),
                None => cui::UiWindow::EMPTY,
            };
        }
        n
    }

    pub fn find_window(&self, title: &str) -> Option<usize> {
        (0..self.window_count).find(|&i| self.windows[i].as_ref().map(|w| w.title == title).unwrap_or(false))
    }

    pub fn create_window(&mut self, title: &'static str, x: usize, y: usize, w: usize, h: usize) -> usize {
        if let Some(i) = self.find_window(title) {
            self.restore_window(i);
            return i;
        }
        let (sw, sh) = self.scale_size(w, h);
        if self.window_count >= 8 { return usize::MAX; }
        let x = x.min(self.screen_w.saturating_sub(sw));
        let y = y.min(self.screen_h.saturating_sub(sh + cui::TASKBAR_H));
        let mut win = Window::new(x, y, sw, sh, title);
        win.z = self.take_z();
        let idx = self.window_count;
        self.windows[idx] = Some(win);
        self.active_window = Some(idx);
        self.window_count += 1;
        idx
    }

    pub fn set_window_size(&mut self, idx: usize, w: usize, h: usize) {
        let (sw, sh) = self.scale_size(w, h);
        let (scr_w, scr_h) = (self.screen_w, self.screen_h);
        if let Some(Some(win)) = self.windows.get_mut(idx) {
            if win.state != WindowState::Normal { return; }
            win.width = sw;
            win.height = sh;
            win.x = win.x.min(scr_w.saturating_sub(sw));
            win.y = win.y.min(scr_h.saturating_sub(sh + cui::TASKBAR_H));
            win.restore_rect = (win.x, win.y, sw, sh);
        }
    }

    fn take_z(&mut self) -> u32 {
        let z = self.next_z;
        self.next_z = self.next_z.wrapping_add(1);
        z
    }

    pub fn raise(&mut self, idx: usize) {
        if idx >= self.window_count { return; }
        let top = self.windows[..self.window_count].iter().flatten().map(|w| w.z).max().unwrap_or(0);
        if let Some(ref mut w) = self.windows[idx] {
            if w.z != top || top == 0 { w.z = self.next_z; self.next_z += 1; }
        }
        self.active_window = Some(idx);
    }

    fn topmost_visible(&self, except: Option<usize>) -> Option<usize> {
        let mut best: Option<(usize, u32)> = None;
        for i in 0..self.window_count {
            if Some(i) == except { continue; }
            if let Some(ref w) = self.windows[i] {
                if w.state == WindowState::Minimized { continue; }
                if best.map(|(_, z)| w.z > z).unwrap_or(true) { best = Some((i, w.z)); }
            }
        }
        best.map(|(i, _)| i)
    }

    pub fn top_at(&self, x: usize, y: usize) -> Option<usize> {
        let mut uw = [cui::UiWindow::EMPTY; 8];
        let n = self.ui_windows(&mut uw);
        cui::top_at(&uw[..n], x, y)
    }

    pub fn close_window(&mut self, idx: usize) {
        if idx >= self.window_count { return; }
        for j in idx..self.window_count - 1 {
            self.windows[j] = self.windows[j + 1].take();
        }
        self.windows[self.window_count - 1] = None;
        self.window_count -= 1;
        let fix = |o: Option<usize>| match o { Some(i) if i == idx => None, Some(i) if i > idx => Some(i - 1), x => x };
        self.dragged_window = fix(self.dragged_window);
        self.resizing_window = fix(self.resizing_window);
        self.active_window = self.topmost_visible(None);
        unsafe { if let Some(f) = ON_WINDOW_CLOSED { f(idx); } }
    }

    pub fn resize_window(&mut self, idx: usize, new_width: usize, new_height: usize) {
        if idx < self.window_count {
            let (sw, sh) = (self.screen_w, self.screen_h);
            if let Some(ref mut win) = self.windows[idx] {
                win.width = new_width.min(sw);
                win.height = new_height.min(sh);
                if win.x + win.width > sw { win.x = sw - win.width; }
                if win.y + win.height > sh { win.y = sh - win.height; }
            }
        }
    }

    pub fn draw_all_with_content(&self, gfx: &mut Graphics, draw_content: &dyn Fn(&mut Graphics, &Window, usize)) {
        let mut uw = [cui::UiWindow::EMPTY; 8];
        let n = self.ui_windows(&mut uw);
        let mut order = [0i32; 8];
        let n = cui::zorder(&uw[..n], &mut order);
        for k in 0..n {
            let i = order[k] as usize;
            if let Some(ref win) = self.windows[i] {
                if win.state == WindowState::Minimized { continue; }
                cui::draw_window(gfx, &uw[i]);
                draw_content(gfx, win, i);
            }
        }
    }

    pub fn draw_taskbar(&self, gfx: &mut Graphics, clock: &str) {
        let mut uw = [cui::UiWindow::EMPTY; 8];
        let n = self.ui_windows(&mut uw);
        cui::draw_taskbar(gfx, &uw[..n], self.start_menu.open, clock);
        self.start_menu.draw(gfx);
    }

    pub fn taskbar_hit(&self, x: usize, y: usize, clock_len: usize) -> i32 {
        let mut uw = [cui::UiWindow::EMPTY; 8];
        let n = self.ui_windows(&mut uw);
        cui::taskbar_hit(&uw[..n], x, y, clock_len)
    }

    pub fn taskbar_click(&mut self, idx: usize) {
        if idx >= self.window_count { return; }
        let minimized = self.windows[idx].as_ref().map(|w| w.state == WindowState::Minimized).unwrap_or(true);
        if minimized {
            self.restore_window(idx);
        } else if self.active_window == Some(idx) && self.topmost_visible(None) == Some(idx) {
            self.minimize_window(idx);
        } else {
            self.raise(idx);
        }
    }

    pub fn minimize_window(&mut self, idx: usize) {
        if let Some(ref mut win) = self.windows[idx] {
            win.state = WindowState::Minimized;
            win.is_dragging = false;
        }
        if self.dragged_window == Some(idx) { self.dragged_window = None; }
        if self.resizing_window == Some(idx) { self.resizing_window = None; }
        self.active_window = self.topmost_visible(Some(idx));
    }

    pub fn handle_mouse_press(&mut self, gfx: &mut Graphics, cx: usize, cy: usize) -> bool {
        if cy >= gfx.height.saturating_sub(cui::TASKBAR_H) { return false; }
        let i = match self.top_at(cx, cy) { Some(i) => i, None => return false };
        self.raise(i);
        let hit = match self.windows[i] { Some(ref w) => w.hit(cx, cy), None => return false };
        match hit {
            cui::HIT_CLOSE => { self.close_window(i); true }
            cui::HIT_MIN => { self.minimize_window(i); true }
            cui::HIT_MAX => {
                let (sw, sh) = (gfx.width, gfx.height);
                if let Some(ref mut win) = self.windows[i] {
                    if win.state == WindowState::Maximized {
                        let (rx, ry, rw, rh) = win.restore_rect;
                        win.x = rx; win.y = ry; win.width = rw; win.height = rh;
                        win.state = WindowState::Normal;
                    } else {
                        win.restore_rect = (win.x, win.y, win.width, win.height);
                        win.x = 0; win.y = 0;
                        win.width = sw;
                        win.height = sh - cui::TASKBAR_H;
                        win.state = WindowState::Maximized;
                    }
                }
                true
            }
            cui::HIT_TITLE => {
                if let Some(ref mut win) = self.windows[i] {
                    if win.state == WindowState::Maximized { return true; }
                    win.is_dragging = true;
                    win.drag_off_x = cx - win.x;
                    win.drag_off_y = cy - win.y;
                    self.dragged_window = Some(i);
                }
                true
            }
            cui::HIT_RESIZE => {
                if self.windows[i].as_ref().map(|w| w.state == WindowState::Normal).unwrap_or(false) {
                    self.resizing_window = Some(i);
                    return true;
                }
                false
            }
            _ => false,
        }
    }

    pub fn handle_mouse_release(&mut self, _gfx: &mut Graphics) -> bool {
        self.resizing_window = None;
        if let Some(i) = self.dragged_window.take() {
            if let Some(ref mut win) = self.windows[i] { win.is_dragging = false; }
            return true;
        }
        false
    }

    pub fn handle_mouse_move(&mut self, gfx: &mut Graphics, cx: usize, cy: usize) -> bool {
        let (sw, sh) = (gfx.width, gfx.height.saturating_sub(cui::TASKBAR_H));
        if let Some(i) = self.resizing_window {
            if let Some(ref mut win) = self.windows[i] {
                let nw = (cx + 5).saturating_sub(win.x).max(200).min(sw - win.x);
                let nh = (cy + 5).saturating_sub(win.y).max(150).min(sh.saturating_sub(win.y).max(150));
                win.width = nw;
                win.height = nh;
                win.restore_rect = (win.x, win.y, nw, nh);
            }
            return true;
        }
        if let Some(i) = self.dragged_window {
            if let Some(ref mut win) = self.windows[i] {
                win.x = cx.saturating_sub(win.drag_off_x);
                win.y = cy.saturating_sub(win.drag_off_y);
                if win.x + win.width > sw { win.x = sw.saturating_sub(win.width); }
                let max_y = sh.saturating_sub(cui::TITLEBAR_H);
                if win.y > max_y { win.y = max_y; }
                win.restore_rect.0 = win.x;
                win.restore_rect.1 = win.y;
            }
            return true;
        }
        false
    }

    pub fn restore_window(&mut self, idx: usize) {
        if idx < self.window_count {
            if let Some(ref mut win) = self.windows[idx] {
                if win.state == WindowState::Minimized {
                    win.state = if win.width >= self.screen_w && win.x == 0 && win.y == 0 { WindowState::Maximized } else { WindowState::Normal };
                }
            }
            self.raise(idx);
        }
    }

    pub fn get_window_title(&self, idx: usize) -> Option<&'static str> {
        self.windows.get(idx).and_then(|w| w.as_ref()).map(|w| w.title)
    }
    
    pub fn draw_theme_selector(&mut self, gfx: &mut Graphics, x: usize, y: usize, w: usize, h: usize) {
        let t = get_theme();
        let cx = x + 4;
        let cy = y + 34;
        
        if w > 8 && h > 38 {
            gfx.fill_rect(cx, cy, w - 8, h - 38, t.window_bg.to_u32());
            gfx.draw_text(cx + 10, cy + 12, "Select Theme:", Color::BLACK.to_u32(), t.window_bg.to_u32());
        }
        
        let themes = [("Aero", 0usize), ("Basic", 1), ("Classic", 2), ("Ubuntu", 3)];
        let icon_size = 72;
        let spacing = 16;
        let total_width = icon_size * 4 + spacing * 3;
        let start_x = cx + w.saturating_sub(8 + total_width) / 2;
        let icons_y = cy + 45;
        
        for (i, (name, theme_idx)) in themes.iter().enumerate() {
            let icon_x = start_x + i * (icon_size + spacing);
            let btn_y = icons_y + icon_size + 12;
            
            if unsafe { CURRENT_THEME } == *theme_idx {
                draw_sunken_rect(gfx, icon_x - 4, icons_y - 4, icon_size + 8, icon_size + 8);
            } else {
                draw_raised_rect(gfx, icon_x - 4, icons_y - 4, icon_size + 8, icon_size + 8);
            }
            
            draw_theme_icon(gfx, icon_x, icons_y, icon_size, *theme_idx);
            
            let text_width = name.len() * 8;
            let text_x = icon_x + icon_size.saturating_sub(text_width) / 2;
            gfx.draw_text(text_x, btn_y, name, Color::BLACK.to_u32(), t.window_bg.to_u32());
            
            let clickable_x = icon_x - 4;
            let clickable_y = icons_y - 4;
            let clickable_w = icon_size + 8;
            let clickable_h = icon_size + 8 + 24;
            
            unsafe {
                THEME_SELECTOR_RECTS[i] = (clickable_x, clickable_y, clickable_w, clickable_h, *theme_idx);
                THEME_SELECTOR_OPEN = true;
                THEME_SELECTOR_X = x;
                THEME_SELECTOR_Y = y;
                THEME_SELECTOR_W = w;
                THEME_SELECTOR_H = h;
            }
        }
    }
}

pub static mut THEME_SELECTOR_OPEN: bool = false;
pub static mut THEME_SELECTOR_X: usize = 0;
pub static mut THEME_SELECTOR_Y: usize = 0;
pub static mut THEME_SELECTOR_W: usize = 0;
pub static mut THEME_SELECTOR_H: usize = 0;
pub static mut THEME_SELECTOR_RECTS: [(usize, usize, usize, usize, usize); 4] = [(0, 0, 0, 0, 0); 4];

pub fn handle_theme_selector_click(cx: usize, cy: usize) -> Option<usize> {
    unsafe {
        if !THEME_SELECTOR_OPEN { return None; }
        for (x, y, w, h, theme_idx) in THEME_SELECTOR_RECTS.iter() {
            if cx >= *x && cx <= *x + *w && cy >= *y && cy <= *y + *h {
                return Some(*theme_idx);
            }
        }
        None
    }
}

pub fn close_theme_selector() {
    unsafe {
        THEME_SELECTOR_OPEN = false;
    }
}