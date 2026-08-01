use crate::graphics::{self, Graphics, Color};
use crate::bmp::BmpImage;

static mut BG_BUF: [u32; 1024 * 768] = [0; 1024 * 768];
static mut TASKBAR_BG_BUF: [u32; 800 * 40] = [0; 800 * 40];

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

pub static THEMES: [Theme; 4] = [

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
}

pub fn get_theme() -> &'static Theme {
    &THEMES[unsafe { CURRENT_THEME }]
}

pub fn is_ubuntu_theme() -> bool {
    unsafe { CURRENT_THEME == 3 }
}

pub fn draw_raised_rect(gfx: &mut Graphics, x: usize, y: usize, w: usize, h: usize) {
    if w == 0 || h == 0 { return; }
    let t = get_theme();
    let face = t.button_face;
    let light = Color::rgb(255, 255, 255);
    let shadow = Color::rgb(80, 80, 90);
    let dark_shadow = Color::rgb(40, 40, 45);

    gfx.fill_rect(x, y, w, h, face.to_u32());

    for dx in 0..w { gfx.put_pixel(x + dx, y, light.to_u32()); }
    for dy in 1..h { gfx.put_pixel(x, y + dy, light.to_u32()); }
    for dy in 1..h { gfx.put_pixel(x + w - 1, y + dy, shadow.to_u32()); }
    for dx in 0..w { gfx.put_pixel(x + dx, y + h - 1, shadow.to_u32()); }

    gfx.put_pixel(x + w - 1, y + h - 1, dark_shadow.to_u32());
    if w >= 2 && h >= 2 {
        gfx.put_pixel(x + w - 2, y + h - 1, dark_shadow.to_u32());
        gfx.put_pixel(x + w - 1, y + h - 2, dark_shadow.to_u32());
    }
}

pub fn draw_sunken_rect(gfx: &mut Graphics, x: usize, y: usize, w: usize, h: usize) {
    if w == 0 || h == 0 { return; }
    let t = get_theme();
    let face = t.button_face;
    let light = Color::rgb(255, 255, 255);
    let shadow = Color::rgb(50, 50, 55);
    let dark_shadow = Color::rgb(30, 30, 35);

    gfx.fill_rect(x, y, w, h, face.to_u32());

    for dx in 0..w { gfx.put_pixel(x + dx, y, shadow.to_u32()); }
    for dy in 1..h { gfx.put_pixel(x, y + dy, shadow.to_u32()); }
    for dy in 1..h { gfx.put_pixel(x + w - 1, y + dy, light.to_u32()); }
    for dx in 0..w { gfx.put_pixel(x + dx, y + h - 1, light.to_u32()); }

    gfx.put_pixel(x, y, dark_shadow.to_u32());
    if w >= 2 { gfx.put_pixel(x + 1, y, dark_shadow.to_u32()); }
    if h >= 2 { gfx.put_pixel(x, y + 1, dark_shadow.to_u32()); }
}

fn draw_window_icon(gfx: &mut Graphics, x: usize, y: usize, size: usize) {
    let icon_size = size * 2 / 3;
    let start_x = x + (size - icon_size) / 2;
    let start_y = y + (size - icon_size) / 2;
    
    for i in 0..icon_size {
        gfx.put_pixel(start_x + i, start_y, Color::rgb(200, 220, 240).to_u32());
        gfx.put_pixel(start_x, start_y + i, Color::rgb(200, 220, 240).to_u32());
        gfx.put_pixel(start_x + icon_size - 1, start_y + i, Color::rgb(100, 130, 170).to_u32());
        gfx.put_pixel(start_x + i, start_y + icon_size - 1, Color::rgb(100, 130, 170).to_u32());
    }
    for dy in 1..icon_size-1 {
        for dx in 1..icon_size-1 {
            if dx < icon_size/2 || dy < icon_size/2 {
                gfx.put_pixel(start_x + dx, start_y + dy, Color::rgb(180, 200, 230).to_u32());
            }
        }
    }
}

fn draw_window_icon_ubuntu(gfx: &mut Graphics, x: usize, y: usize, size: usize) {
    let icon_size = size * 2 / 3;
    let start_x = x + (size - icon_size) / 2;
    let start_y = y + (size - icon_size) / 2;
    
    let r = 3;
    for dy in 0..icon_size {
        for dx in 0..icon_size {
            let edge = dx < r || dx >= icon_size - r || dy < r || dy >= icon_size - r;
            if edge {
                gfx.put_pixel(start_x + dx, start_y + dy, Color::rgb(230, 230, 230).to_u32());
            }
        }
    }
    for dy in r..icon_size - r {
        for dx in r..icon_size - r {
            gfx.put_pixel(start_x + dx, start_y + dy, Color::rgb(200, 200, 200).to_u32());
        }
    }
}

fn draw_win7_button(gfx: &mut Graphics, x: usize, y: usize, size: usize, symbol: &str, hover: bool, pressed: bool) {
    let (bg_color, border_color, symbol_color) = match symbol {
        "X" => {
            if pressed {
                (Color::rgb(180, 60, 60), Color::rgb(140, 40, 40), Color::WHITE)
            } else if hover {
                (Color::rgb(220, 100, 100), Color::rgb(180, 70, 70), Color::WHITE)
            } else {
                (Color::rgb(220, 220, 230), Color::rgb(150, 150, 160), Color::rgb(80, 80, 90))
            }
        },
        _ => {
            if pressed {
                (Color::rgb(180, 180, 190), Color::rgb(100, 100, 110), Color::rgb(100, 100, 110))
            } else if hover {
                (Color::rgb(210, 210, 220), Color::rgb(130, 130, 140), Color::rgb(80, 80, 90))
            } else {
                (Color::rgb(220, 220, 230), Color::rgb(150, 150, 160), Color::rgb(80, 80, 90))
            }
        }
    };
    
    gfx.fill_rect(x, y, size, size, bg_color.to_u32());
    gfx.draw_rect_border(x, y, size, size, border_color.to_u32());
    
    let center_x = x + size / 2;
    let center_y = y + size / 2;
    
    match symbol {
        "_" => {
            let line_y = center_y + 2;
            let line_w = size / 2;
            for dx in 0..line_w {
                let px = center_x - line_w/2 + dx;
                if px < x + size {
                    gfx.put_pixel(px, line_y, symbol_color.to_u32());
                }
            }
        },
        "□" => {
            let box_size = size / 2;
            let bx = center_x - box_size/2;
            let by = center_y - box_size/2;
            for i in 0..box_size {
                gfx.put_pixel(bx + i, by, symbol_color.to_u32());
                gfx.put_pixel(bx + i, by + box_size - 1, symbol_color.to_u32());
                gfx.put_pixel(bx, by + i, symbol_color.to_u32());
                gfx.put_pixel(bx + box_size - 1, by + i, symbol_color.to_u32());
            }
        },
        "X" => {
            let cross_size = size / 3;
            for i in 0..cross_size {
                gfx.put_pixel(center_x - cross_size/2 + i, center_y - cross_size/2 + i, symbol_color.to_u32());
                gfx.put_pixel(center_x - cross_size/2 + i, center_y + cross_size/2 - i, symbol_color.to_u32());
            }
        },
        _ => {}
    }
    
    if !pressed && symbol != "X" {
        for dx in 1..size-1 {
            let highlight = Color::rgb(245, 245, 250);
            gfx.put_pixel(x + dx, y + 1, highlight.to_u32());
        }
    }
}

fn draw_ubuntu_button(gfx: &mut Graphics, x: usize, y: usize, size: usize, symbol: &str, hover: bool, pressed: bool) {
    match symbol {
        "X" => {
            let bg_color = if pressed {
                Color::rgb(200, 50, 50)
            } else if hover {
                Color::rgb(220, 80, 80)
            } else {
                Color::rgb(48, 48, 48)
            };
            gfx.fill_rect(x, y, size, size, bg_color.to_u32());
            
            let cross_size = size / 3;
            let center_x = x + size / 2;
            let center_y = y + size / 2;
            let color = if pressed || hover { Color::WHITE } else { Color::rgb(180, 180, 180) };
            for i in 0..cross_size {
                gfx.put_pixel(center_x - cross_size/2 + i, center_y - cross_size/2 + i, color.to_u32());
                gfx.put_pixel(center_x - cross_size/2 + i, center_y + cross_size/2 - i, color.to_u32());
            }
        },
        "_" => {
            if hover || pressed {
                gfx.fill_rect(x, y, size, size, Color::rgb(60, 60, 60).to_u32());
            }
            let line_y = y + size / 2 + 2;
            for dx in size/4..size*3/4 {
                gfx.put_pixel(x + dx, line_y, Color::rgb(180, 180, 180).to_u32());
            }
        },
        "□" => {
            if hover || pressed {
                gfx.fill_rect(x, y, size, size, Color::rgb(60, 60, 60).to_u32());
            }
            let box_size = size / 3;
            let bx = x + size/2 - box_size/2;
            let by = y + size/2 - box_size/2;
            let color = Color::rgb(180, 180, 180);
            for i in 0..box_size {
                gfx.put_pixel(bx + i, by, color.to_u32());
                gfx.put_pixel(bx + i, by + box_size - 1, color.to_u32());
                gfx.put_pixel(bx, by + i, color.to_u32());
                gfx.put_pixel(bx + box_size - 1, by + i, color.to_u32());
            }
        },
        _ => {}
    }
}

fn get_backbuffer_pixel(gfx: &Graphics, x: usize, y: usize) -> u32 {
    unsafe {
        if y < gfx.height && x < gfx.width {
            graphics::BACKBUFFER[y * gfx.width + x]
        } else {
            0
        }
    }
}

pub fn draw_ubuntu_titlebar(gfx: &mut Graphics, x: usize, y: usize, w: usize, h: usize, active: bool) {
    if w == 0 || h == 0 { return; }
    
    let color = if active {
        Color::rgb(48, 48, 48)
    } else {
        Color::rgb(65, 65, 65)
    };
    
    gfx.fill_rect(x, y, w, h, color.to_u32());
    
    let line_color = if active {
        Color::rgb(60, 60, 60)
    } else {
        Color::rgb(75, 75, 75)
    };
    for dx in 0..w {
        gfx.put_pixel(x + dx, y + h - 1, line_color.to_u32());
    }
}

pub fn draw_glass_titlebar(gfx: &mut Graphics, x: usize, y: usize, w: usize, h: usize, active: bool) {
    if is_ubuntu_theme() {
        draw_ubuntu_titlebar(gfx, x, y, w, h, active);
        return;
    }
    
    if w == 0 || h == 0 { return; }
    
    let (top_color, bottom_color) = if active {
        (Color::rgb(160, 200, 245), Color::rgb(80, 120, 180))
    } else {
        (Color::rgb(190, 195, 205), Color::rgb(140, 145, 155))
    };
    
    for row in 0..h {
        let t = row as f32 / (h as f32 - 1.0).max(1.0);
        let r = (top_color.r as f32 + (bottom_color.r as f32 - top_color.r as f32) * t) as u8;
        let g = (top_color.g as f32 + (bottom_color.g as f32 - top_color.g as f32) * t) as u8;
        let b = (top_color.b as f32 + (bottom_color.b as f32 - top_color.b as f32) * t) as u8;
        
        for dx in 0..w {
            let bg_pixel = get_backbuffer_pixel(gfx, x + dx, y + row);
            let bg_r = (bg_pixel >> 16) & 0xFF;
            let bg_g = (bg_pixel >> 8) & 0xFF;
            let bg_b = bg_pixel & 0xFF;
            
            let alpha = if active { 0.65 } else { 0.50 };
            let final_r = (r as f32 * alpha + bg_r as f32 * (1.0 - alpha)) as u8;
            let final_g = (g as f32 * alpha + bg_g as f32 * (1.0 - alpha)) as u8;
            let final_b = (b as f32 * alpha + bg_b as f32 * (1.0 - alpha)) as u8;
            
            gfx.put_pixel(x + dx, y + row, Color::rgb(final_r, final_g, final_b).to_u32());
        }
    }
    
    let glow_height = 8;
    for row in 0..glow_height {
        let intensity = (120 - (row * 100 / glow_height)) as u8;
        for dx in 3..w-3 {
            let bg_pixel = get_backbuffer_pixel(gfx, x + dx, y + row);
            let bg_r = (bg_pixel >> 16) & 0xFF;
            let bg_g = (bg_pixel >> 8) & 0xFF;
            let bg_b = bg_pixel & 0xFF;
            
            let r = 240;
            let g = 248;
            let b = 255;
            let alpha = 0.25 + (intensity as f32 / 200.0);
            let final_r = (r as f32 * alpha + bg_r as f32 * (1.0 - alpha)) as u8;
            let final_g = (g as f32 * alpha + bg_g as f32 * (1.0 - alpha)) as u8;
            let final_b = (b as f32 * alpha + bg_b as f32 * (1.0 - alpha)) as u8;
            
            gfx.put_pixel(x + dx, y + row, Color::rgb(final_r, final_g, final_b).to_u32());
        }
    }
    
    if h > 2 {
        for dx in 1..w-1 {
            let bg_pixel = get_backbuffer_pixel(gfx, x + dx, y + h - 1);
            let bg_r = (bg_pixel >> 16) & 0xFF;
            let bg_g = (bg_pixel >> 8) & 0xFF;
            let bg_b = bg_pixel & 0xFF;
            let alpha = 0.55;
            let final_r = (255.0 * alpha + bg_r as f32 * (1.0 - alpha)) as u8;
            let final_g = (255.0 * alpha + bg_g as f32 * (1.0 - alpha)) as u8;
            let final_b = (255.0 * alpha + bg_b as f32 * (1.0 - alpha)) as u8;
            gfx.put_pixel(x + dx, y + h - 1, Color::rgb(final_r, final_g, final_b).to_u32());
        }
    }
}

fn draw_text_with_outline(gfx: &mut Graphics, x: usize, y: usize, text: &str, color: u32, outline_color: u32) {
    let offsets = [(-1, -1), (-1, 0), (-1, 1), (0, -1), (0, 1), (1, -1), (1, 0), (1, 1)];
    
    for (ox, oy) in offsets.iter() {
        let nx = (x as isize + ox) as usize;
        let ny = (y as isize + oy) as usize;
        if nx < 800 && ny < 600 {
            gfx.draw_text(nx, ny, text, outline_color, 0);
        }
    }
    
    gfx.draw_text(x, y, text, color, 0);
}

fn save_taskbar_background(gfx: &Graphics, x: usize, y: usize, w: usize, h: usize) {
    for dy in 0..h {
        for dx in 0..w {
            unsafe {
                TASKBAR_BG_BUF[dy * w + dx] = graphics::BACKBUFFER[(y + dy) * gfx.width + (x + dx)];
            }
        }
    }
}

pub fn draw_aero_taskbar(gfx: &mut Graphics, x: usize, y: usize, w: usize, h: usize) {
    if w == 0 || h == 0 { return; }
    
    save_taskbar_background(gfx, x, y, w, h);
    
    for row in 0..h {
        let t = row as f32 / (h as f32 - 1.0).max(1.0);
        let r = (30.0 + (20.0 - 30.0) * t) as u8;
        let g = (40.0 + (25.0 - 40.0) * t) as u8;
        let b = (60.0 + (40.0 - 60.0) * t) as u8;
        
        for dx in 0..w {
            unsafe {
                let bg_pixel = TASKBAR_BG_BUF[row * w + dx];
                let bg_r = (bg_pixel >> 16) & 0xFF;
                let bg_g = (bg_pixel >> 8) & 0xFF;
                let bg_b = bg_pixel & 0xFF;
                
                let alpha = 0.6;
                let final_r = (r as f32 * alpha + bg_r as f32 * (1.0 - alpha)) as u8;
                let final_g = (g as f32 * alpha + bg_g as f32 * (1.0 - alpha)) as u8;
                let final_b = (b as f32 * alpha + bg_b as f32 * (1.0 - alpha)) as u8;
                
                gfx.put_pixel(x + dx, y + row, Color::rgb(final_r, final_g, final_b).to_u32());
            }
        }
    }
    
    for dx in 0..w {
        unsafe {
            let bg_pixel = TASKBAR_BG_BUF[0 * w + dx];
            let bg_r = (bg_pixel >> 16) & 0xFF;
            let bg_g = (bg_pixel >> 8) & 0xFF;
            let bg_b = bg_pixel & 0xFF;
            let r = (255.0 * 0.3 + bg_r as f32 * 0.7) as u8;
            let g = (255.0 * 0.3 + bg_g as f32 * 0.7) as u8;
            let b = (255.0 * 0.3 + bg_b as f32 * 0.7) as u8;
            gfx.put_pixel(x + dx, y, Color::rgb(r, g, b).to_u32());
        }
    }
    
    for dx in 0..w {
        unsafe {
            let bg_pixel = TASKBAR_BG_BUF[(h-1) * w + dx];
            let bg_r = (bg_pixel >> 16) & 0xFF;
            let bg_g = (bg_pixel >> 8) & 0xFF;
            let bg_b = bg_pixel & 0xFF;
            let r = (0.0 * 0.4 + bg_r as f32 * 0.6) as u8;
            let g = (0.0 * 0.4 + bg_g as f32 * 0.6) as u8;
            let b = (0.0 * 0.4 + bg_b as f32 * 0.6) as u8;
            gfx.put_pixel(x + dx, y + h - 1, Color::rgb(r, g, b).to_u32());
        }
    }
}

fn draw_ubuntu_taskbar(gfx: &mut Graphics, x: usize, y: usize, w: usize, h: usize) {
    let color = Color::rgb(48, 48, 48);
    gfx.fill_rect(x, y, w, h, color.to_u32());
    
    for dx in 0..w {
        gfx.put_pixel(x + dx, y, Color::rgb(60, 60, 60).to_u32());
    }
    
    for dy in 4..h-4 {
        for dx in 60..w-90 {
            let alpha = 0.08;
            let base = Color::rgb(60, 60, 60);
            let r = (base.r as f32 * alpha + color.r as f32 * (1.0 - alpha)) as u8;
            let g = (base.g as f32 * alpha + color.g as f32 * (1.0 - alpha)) as u8;
            let b = (base.b as f32 * alpha + color.b as f32 * (1.0 - alpha)) as u8;
            gfx.put_pixel(x + dx, y + dy, Color::rgb(r, g, b).to_u32());
        }
    }
}

fn draw_ubuntu_start_button(gfx: &mut Graphics, x: usize, y: usize, w: usize, h: usize, pressed: bool) {
    let color = if pressed {
        Color::rgb(80, 80, 80)
    } else {
        Color::rgb(55, 55, 55)
    };
    gfx.fill_rect(x, y, w, h, color.to_u32());
    
    for dx in 0..4 {
        gfx.put_pixel(x + dx, y, Color::rgb(48, 48, 48).to_u32());
        gfx.put_pixel(x + w - 1 - dx, y, Color::rgb(48, 48, 48).to_u32());
        gfx.put_pixel(x + dx, y + h - 1, Color::rgb(48, 48, 48).to_u32());
        gfx.put_pixel(x + w - 1 - dx, y + h - 1, Color::rgb(48, 48, 48).to_u32());
    }
}

fn draw_aero_start_button(gfx: &mut Graphics, x: usize, y: usize, w: usize, h: usize, pressed: bool) {
    let mut local_buf = [0u32; 55 * 36];
    unsafe {
        for dy in 0..h {
            for dx in 0..w {
                local_buf[dy * w + dx] = graphics::BACKBUFFER[(y + dy) * gfx.width + (x + dx)];
            }
        }
    }
    
    for row in 0..h {
        let t = row as f32 / (h as f32 - 1.0).max(1.0);
        let r = if pressed {
            50.0 + (30.0 - 50.0) * t
        } else {
            70.0 + (40.0 - 70.0) * t
        };
        let g = if pressed {
            70.0 + (45.0 - 70.0) * t
        } else {
            100.0 + (65.0 - 100.0) * t
        };
        let b = if pressed {
            100.0 + (70.0 - 100.0) * t
        } else {
            140.0 + (100.0 - 140.0) * t
        };
        
        for dx in 0..w {
            let bg_pixel = local_buf[row * w + dx];
            let bg_r = (bg_pixel >> 16) & 0xFF;
            let bg_g = (bg_pixel >> 8) & 0xFF;
            let bg_b = bg_pixel & 0xFF;
            
            let alpha = if pressed { 0.85 } else { 0.7 };
            let final_r = (r as f32 * alpha + bg_r as f32 * (1.0 - alpha)) as u8;
            let final_g = (g as f32 * alpha + bg_g as f32 * (1.0 - alpha)) as u8;
            let final_b = (b as f32 * alpha + bg_b as f32 * (1.0 - alpha)) as u8;
            
            gfx.put_pixel(x + dx, y + row, Color::rgb(final_r, final_g, final_b).to_u32());
        }
    }
    
    let border_color = if pressed { Color::rgb(20, 35, 55) } else { Color::rgb(50, 75, 110) };
    gfx.draw_rect_border(x, y, w, h, border_color.to_u32());
    
    if !pressed {
        for dx in 2..w-2 {
            gfx.put_pixel(x + dx, y + 1, Color::rgb(130, 170, 220).to_u32());
        }
    }
}

pub fn draw_taskbar_background(gfx: &mut Graphics, x: usize, y: usize, w: usize, h: usize) {
    if is_ubuntu_theme() {
        draw_ubuntu_taskbar(gfx, x, y, w, h);
    } else if unsafe { CURRENT_THEME } == 0 {
        draw_aero_taskbar(gfx, x, y, w, h);
    } else {
        let t = get_theme();
        let top_color = t.taskbar_bg;
        let bottom_color = Color::rgb(
            (t.taskbar_bg.r as u16 * 2 / 3) as u8,
            (t.taskbar_bg.g as u16 * 2 / 3) as u8,
            (t.taskbar_bg.b as u16 * 2 / 3) as u8,
        );
        for row in 0..h {
            let t = row as f32 / (h as f32 - 1.0).max(1.0);
            let r = (top_color.r as f32 + (bottom_color.r as f32 - top_color.r as f32) * t) as u8;
            let g = (top_color.g as f32 + (bottom_color.g as f32 - top_color.g as f32) * t) as u8;
            let b = (top_color.b as f32 + (bottom_color.b as f32 - top_color.b as f32) * t) as u8;
            let color = Color::rgb(r, g, b).to_u32();
            for dx in 0..w {
                gfx.put_pixel(x + dx, y + row, color);
            }
        }
    }
}

pub fn draw_windows_logo(gfx: &mut Graphics, x: usize, y: usize) {
    if is_ubuntu_theme() {
        let cx = x + 10;
        let cy = y + 10;
        let r = 8;
        for dy in -r..=r {
            for dx in -r..=r {
                if dx*dx + dy*dy <= r*r {
                    let px = (cx as i32 + dx) as usize;
                    let py = (cy as i32 + dy) as usize;
                    gfx.put_pixel(px, py, Color::rgb(220, 80, 80).to_u32());
                }
            }
        }
        let colors = [Color::rgb(220, 80, 80), Color::rgb(220, 80, 80), Color::rgb(220, 80, 80)];
        for i in 0..3 {
            let idx = i as usize;
            let px = (cx as i32 - 6 + i * 6) as usize;
            let py = (cy as i32 - 2) as usize;
            gfx.put_pixel(px, py, colors[idx].to_u32());
            gfx.put_pixel(px, py + 4, colors[idx].to_u32());
        }
    } else {
        for dy in 0..20 {
            for dx in 0..20 {
                if dx < 14 && dy < 16 {
                    gfx.put_pixel(x + dx, y + dy, Color::rgb(0, 120, 215).to_u32());
                }
            }
        }
        for i in 0..5 {
            for dx in 0..10 {
                gfx.put_pixel(x + 3 + dx + i, y + 4 + i, Color::rgb(240, 200, 0).to_u32());
                gfx.put_pixel(x + 3 + dx + i, y + 5 + i, Color::rgb(240, 200, 0).to_u32());
            }
        }
        for i in 0..4 {
            for dx in 0..8 {
                gfx.put_pixel(x + 5 + dx + i, y + 10 + i, Color::rgb(160, 200, 80).to_u32());
                gfx.put_pixel(x + 5 + dx + i, y + 11 + i, Color::rgb(160, 200, 80).to_u32());
            }
        }
        for i in 0..3 {
            for dx in 0..6 {
                gfx.put_pixel(x + 7 + dx + i, y + 14 + i, Color::rgb(230, 80, 80).to_u32());
                gfx.put_pixel(x + 7 + dx + i, y + 15 + i, Color::rgb(230, 80, 80).to_u32());
            }
        }
    }
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

fn save_background(gfx: &Graphics, x: usize, y: usize, w: usize, h: usize) {
    for dy in 0..h {
        for dx in 0..w {
            unsafe {
                BG_BUF[dy * w + dx] = graphics::BACKBUFFER[(y + dy) * gfx.width + (x + dx)];
            }
        }
    }
}

fn restore_background(gfx: &mut Graphics, x: usize, y: usize, w: usize, h: usize) {
    for dy in 0..h {
        for dx in 0..w {
            gfx.put_pixel(x + dx, y + dy, unsafe { BG_BUF[dy * w + dx] });
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
}

impl Window {
    pub fn new(x: usize, y: usize, w: usize, h: usize, title: &'static str) -> Self {
        Self {
            x, y, width: w, height: h, title,
            is_dragging: false,
            drag_off_x: 0, drag_off_y: 0,
            state: WindowState::Normal,
            restore_rect: (x, y, w, h),
        }
    }

    pub fn draw(&self, gfx: &mut Graphics, active: bool) {
        if self.state == WindowState::Minimized { return; }
        let t = get_theme();
        let (x, y, w, h) = (self.x, self.y, self.width, self.height);
        let titlebar_h = 30;
        let ubuntu = is_ubuntu_theme();

        if !ubuntu {
            for dx in 0..w {
                if y + h + 3 < 600 && x + dx + 5 < 800 {
                    gfx.put_pixel(x + dx + 5, y + h + 3, Color::rgb(0, 0, 0).to_u32());
                }
                if y + h + 2 < 600 && x + dx + 4 < 800 {
                    gfx.put_pixel(x + dx + 4, y + h + 2, Color::rgb(0, 0, 0).to_u32());
                }
                if y + h + 1 < 600 && x + dx + 3 < 800 {
                    gfx.put_pixel(x + dx + 3, y + h + 1, Color::rgb(0, 0, 0).to_u32());
                }
            }
            for dy in 0..h {
                if x + w + 5 < 800 && y + dy + 3 < 600 {
                    gfx.put_pixel(x + w + 5, y + dy + 3, Color::rgb(0, 0, 0).to_u32());
                }
                if x + w + 4 < 800 && y + dy + 2 < 600 {
                    gfx.put_pixel(x + w + 4, y + dy + 2, Color::rgb(0, 0, 0).to_u32());
                }
                if x + w + 3 < 800 && y + dy + 1 < 600 {
                    gfx.put_pixel(x + w + 3, y + dy + 1, Color::rgb(0, 0, 0).to_u32());
                }
            }
        }

        gfx.fill_rect(x, y + titlebar_h, w, h - titlebar_h, t.window_bg.to_u32());
        
        draw_glass_titlebar(gfx, x, y, w, titlebar_h, active);
        
        if ubuntu {
            let icon_size = 16;
            let icon_x = x + 12;
            let icon_y = y + (titlebar_h - icon_size) / 2;
            draw_window_icon_ubuntu(gfx, icon_x, icon_y, icon_size);
            let title_x = icon_x + icon_size + 8;
            let text_color = Color::rgb(230, 230, 230);
            if title_x + self.title.len() * 8 < x + w - 90 {
                gfx.draw_text(title_x, y + 9, self.title, text_color.to_u32(), 0);
            }
        } else {
            let icon_size = 20;
            let icon_x = x + 8;
            let icon_y = y + (titlebar_h - icon_size) / 2;
            draw_window_icon(gfx, icon_x, icon_y, icon_size);
            let title_x = icon_x + icon_size + 8;
            let title_len = self.title.len() * 8;
            if title_x + title_len < x + w - 120 {
                let text_color = if active { 
                    Color::rgb(255, 255, 255) 
                } else { 
                    Color::rgb(200, 200, 210) 
                };
                let outline_color = Color::rgb(0, 0, 0);
                draw_text_with_outline(gfx, title_x, y + 9, self.title, text_color.to_u32(), outline_color.to_u32());
            }
        }
        
        let btn_size = if ubuntu { 28 } else { 20 };
        let btn_y = y + (titlebar_h - btn_size) / 2;
        let btn_spacing = 4;
        
        let close_x = x + w - btn_size - 8;
        let max_x = if close_x >= btn_size + btn_spacing { close_x - btn_size - btn_spacing } else { close_x };
        let min_x = if max_x >= btn_size + btn_spacing { max_x - btn_size - btn_spacing } else { max_x };
        
        if ubuntu {
            if close_x + btn_size <= x + w {
                draw_ubuntu_button(gfx, close_x, btn_y, btn_size, "X", false, false);
            }
            if max_x + btn_size <= x + w {
                draw_ubuntu_button(gfx, max_x, btn_y, btn_size, "□", false, false);
            }
            if min_x + btn_size <= x + w {
                draw_ubuntu_button(gfx, min_x, btn_y, btn_size, "_", false, false);
            }
        } else {
            if min_x + btn_size <= x + w {
                draw_win7_button(gfx, min_x, btn_y, btn_size, "_", false, false);
            }
            if max_x + btn_size <= x + w {
                draw_win7_button(gfx, max_x, btn_y, btn_size, "□", false, false);
            }
            if close_x + btn_size <= x + w {
                draw_win7_button(gfx, close_x, btn_y, btn_size, "X", false, false);
            }
        }
        
        if ubuntu {
            for dx in 0..w {
                gfx.put_pixel(x + dx, y + h - 1, Color::rgb(60, 60, 60).to_u32());
            }
            for dy in 0..h {
                gfx.put_pixel(x + w - 1, y + dy, Color::rgb(60, 60, 60).to_u32());
            }
        } else {
            if w >= 2 && h >= 2 {
                gfx.draw_rect_border(x, y, w, h, Color::rgb(100, 100, 120).to_u32());
                gfx.draw_rect_border(x + 1, y + 1, w - 2, h - 2, Color::rgb(200, 200, 210).to_u32());
            }
        }

        let content_y = y + titlebar_h + 2;
        let content_h = if h > titlebar_h + 4 { h - titlebar_h - 4 } else { 0 };
        if content_h > 0 && w > 4 {
            gfx.fill_rect(x + 2, content_y, w - 4, content_h, t.window_bg.to_u32());
        }
    }

    pub fn contains(&self, x: usize, y: usize) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }
    pub fn is_titlebar(&self, x: usize, y: usize) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + 30
    }
    pub fn is_close_button(&self, x: usize, y: usize) -> bool {
        let ubuntu = is_ubuntu_theme();
        let btn_size = if ubuntu { 28 } else { 20 };
        let btn_y = self.y + (30 - btn_size) / 2;
        let close_x = self.x + self.width - btn_size - 8;
        x >= close_x && x < close_x + btn_size && y >= btn_y && y < btn_y + btn_size
    }
    pub fn is_maximize_button(&self, x: usize, y: usize) -> bool {
        let ubuntu = is_ubuntu_theme();
        let btn_size = if ubuntu { 28 } else { 20 };
        let btn_y = self.y + (30 - btn_size) / 2;
        let btn_spacing = 4;
        let close_x = self.x + self.width - btn_size - 8;
        let max_x = close_x - btn_size - btn_spacing;
        x >= max_x && x < max_x + btn_size && y >= btn_y && y < btn_y + btn_size
    }
    pub fn is_minimize_button(&self, x: usize, y: usize) -> bool {
        let ubuntu = is_ubuntu_theme();
        let btn_size = if ubuntu { 28 } else { 20 };
        let btn_y = self.y + (30 - btn_size) / 2;
        let btn_spacing = 4;
        let close_x = self.x + self.width - btn_size - 8;
        let max_x = close_x - btn_size - btn_spacing;
        let min_x = max_x - btn_size - btn_spacing;
        x >= min_x && x < min_x + btn_size && y >= btn_y && y < btn_y + btn_size
    }
    pub fn is_resize_handle(&self, x: usize, y: usize) -> bool {
        let handle_size = 15;
        let rx = self.x + self.width - handle_size;
        let ry = self.y + self.height - handle_size;
        x >= rx && x <= self.x + self.width && y >= ry && y <= self.y + self.height
    }
}

pub struct Menu {
    pub open: bool,
    items: [&'static str; 4],
}

impl Menu {
    pub const fn new() -> Self {
        Self { open: false, items: ["Terminal", "Files", "Settings", "Shutdown"] }
    }
    pub fn draw(&self, gfx: &mut Graphics) {
        if !self.open { return; }
        let ubuntu = is_ubuntu_theme();
        let x = 4;
        let y = gfx.height - 135;
        let w = 200;
        let h = 115;
        
        if ubuntu {
            gfx.fill_rect(x, y, w, h, Color::rgb(45, 45, 45).to_u32());
            gfx.draw_rect_border(x, y, w, h, Color::rgb(70, 70, 70).to_u32());
            
            gfx.fill_rect(x, y, w, 28, Color::rgb(55, 55, 55).to_u32());
            gfx.draw_text(x + 12, y + 8, "Applications", Color::rgb(200, 200, 200).to_u32(), 0);
            
            for dx in 8..w-8 {
                gfx.put_pixel(x + dx, y + 30, Color::rgb(70, 70, 70).to_u32());
            }
            
            let icons = ["", "", "", ""];
            for (i, item) in self.items.iter().enumerate() {
                let item_y = y + 34 + i * 20;
                let is_last = i == 3;
                let color = if is_last { 
                    Color::rgb(220, 80, 80) 
                } else { 
                    Color::rgb(220, 220, 220) 
                };
                
                gfx.draw_text(x + 8, item_y, icons[i], color.to_u32(), 0);
                gfx.draw_text(x + 28, item_y, item, color.to_u32(), 0);
                
                if i == 1 {
                    for dx in 8..w-8 {
                        gfx.put_pixel(x + dx, y + 34 + i * 20 + 16, Color::rgb(60, 60, 60).to_u32());
                    }
                }
            }
        } else {
            let t = get_theme();
            gfx.fill_rect(x, y, w, h, t.start_menu_bg.to_u32());
            gfx.draw_rect_border(x, y, w, h, Color::rgb(80, 80, 90).to_u32());
            for (i, item) in self.items.iter().enumerate() {
                gfx.draw_text(x + 12, y + 12 + i * 22, item, t.button_text.to_u32(), t.start_menu_bg.to_u32());
            }
        }
    }
    pub fn contains(&self, x: usize, y: usize, gfx: &Graphics) -> bool {
        if !self.open { return false; }
        let sx = 4;
        let sy = gfx.height - 135;
        x >= sx && x < sx + 200 && y >= sy && y < sy + 115
    }
    pub fn handle_click(&self, gfx: &Graphics, cx: usize, cy: usize) -> Option<&'static str> {
        if !self.open { return None; }
        let x = 4;
        let y = gfx.height - 135;
        if cx < x || cx > x + 200 || cy < y || cy > y + 115 { return None; }
        let idx = (cy - y - 34) / 20;
        if idx < self.items.len() {
            Some(self.items[idx])
        } else { None }
    }
}

pub struct WindowManager {
    pub windows: [Option<Window>; 8],
    pub window_count: usize,
    pub dragged_window: Option<usize>,
    pub active_window: Option<usize>,
    saved_bg_w: usize,
    saved_bg_h: usize,
    pub start_menu: Menu,
}

impl WindowManager {
    pub const fn new() -> Self {
        const NONE: Option<Window> = None;
        Self {
            windows: [NONE; 8],
            window_count: 0,
            dragged_window: None,
            active_window: None,
            saved_bg_w: 0,
            saved_bg_h: 0,
            start_menu: Menu::new(),
        }
    }


    pub fn create_window(&mut self, title: &'static str, x: usize, y: usize, w: usize, h: usize) -> usize {
        if self.window_count < 8 {
            self.windows[self.window_count] = Some(Window::new(x, y, w, h, title));
            let idx = self.window_count;
            self.active_window = Some(idx);
            self.window_count += 1;
            return idx; 
        }
        0 
    }

    pub fn resize_window(&mut self, idx: usize, new_width: usize, new_height: usize) {
        if idx < self.window_count {
            if let Some(ref mut win) = self.windows[idx] {
                win.width = new_width;
                win.height = new_height;
                if win.x + win.width > 800 { win.x = 800 - win.width; }
                if win.y + win.height > 600 { win.y = 600 - win.height; }
            }
        }
    }

    pub fn draw_all_with_content(&self, gfx: &mut Graphics, draw_content: &dyn Fn(&mut Graphics, &Window, usize)) {
        for i in 0..self.window_count {
            if let Some(ref win) = self.windows[i] {
                let active = self.active_window == Some(i);
                win.draw(gfx, active);
                draw_content(gfx, win, i);
            }
        }
    }

    pub fn draw_taskbar(&self, gfx: &mut Graphics) {
        let t = get_theme();
        let taskbar_h = 40;
        let y = gfx.height - taskbar_h;
        let ubuntu = is_ubuntu_theme();
        
        draw_taskbar_background(gfx, 0, y, gfx.width, taskbar_h);
        
        if ubuntu {
            draw_ubuntu_start_button(gfx, 4, y + 4, 55, taskbar_h - 8, self.start_menu.open);
        } else if unsafe { CURRENT_THEME } == 0 {
            draw_aero_start_button(gfx, 4, y + 4, 55, taskbar_h - 8, self.start_menu.open);
        } else {
            draw_raised_rect(gfx, 4, y + 4, 55, taskbar_h - 8);
        }
        
        draw_windows_logo(gfx, 14, y + 10);
        let text_color = if ubuntu { Color::rgb(230, 230, 230) } else { t.taskbar_text };
        gfx.draw_text(32, y + 12, if ubuntu { "Activities" } else { "Пуск" }, text_color.to_u32(), t.taskbar_bg.to_u32());
        
        let mut btn_x = 67;
        for i in 0..self.window_count {
            if let Some(ref win) = self.windows[i] {
                if win.state != WindowState::Minimized {
                    let btn_w = if ubuntu { 160 } else { 150 };
                    if btn_x + btn_w < gfx.width - 100 {
                        let pressed = self.active_window == Some(i);
                        if ubuntu {
                            let bg_color = if pressed {
                                Color::rgb(60, 60, 60)
                            } else {
                                Color::rgb(50, 50, 50)
                            };
                            gfx.fill_rect(btn_x, y + 4, btn_w, taskbar_h - 8, bg_color.to_u32());
                            if pressed {
                                for dx in 8..btn_w-8 {
                                    gfx.put_pixel(btn_x + dx, y + 4, Color::rgb(220, 80, 80).to_u32());
                                }
                            }
                            draw_window_icon_ubuntu(gfx, btn_x + 5, y + 10, 16);
                            if btn_x + 26 < gfx.width {
                                let title_color = if pressed { Color::rgb(255, 255, 255) } else { Color::rgb(200, 200, 200) };
                                gfx.draw_text(btn_x + 26, y + 14, win.title, title_color.to_u32(), 0);
                            }
                        } else {
                            if pressed {
                                draw_sunken_rect(gfx, btn_x, y + 4, btn_w, taskbar_h - 8);
                            } else {
                                draw_raised_rect(gfx, btn_x, y + 4, btn_w, taskbar_h - 8);
                            }
                            draw_window_icon(gfx, btn_x + 5, y + 10, 16);
                            if btn_x + 26 < gfx.width {
                                gfx.draw_text(btn_x + 26, y + 14, win.title, t.taskbar_text.to_u32(), t.taskbar_bg.to_u32());
                            }
                        }
                        btn_x += btn_w + 4;
                    }
                }
            }
        }
        
        if gfx.width >= 90 {
            if ubuntu {
                gfx.fill_rect(gfx.width - 90, y, 90, taskbar_h, Color::rgb(48, 48, 48).to_u32());
                gfx.draw_text(gfx.width - 80, y + 12, "16:20", Color::rgb(230, 230, 230).to_u32(), 0);
            } else if unsafe { CURRENT_THEME } == 0 {
                unsafe {
                    for dx in 0..90 {
                        let bg_pixel = graphics::BACKBUFFER[(y + 10) * gfx.width + (gfx.width - 90 + dx)];
                        let bg_r = (bg_pixel >> 16) & 0xFF;
                        let bg_g = (bg_pixel >> 8) & 0xFF;
                        let bg_b = bg_pixel & 0xFF;
                        let alpha = 0.6;
                        let final_r = (30.0 * alpha + bg_r as f32 * (1.0 - alpha)) as u8;
                        let final_g = (40.0 * alpha + bg_g as f32 * (1.0 - alpha)) as u8;
                        let final_b = (60.0 * alpha + bg_b as f32 * (1.0 - alpha)) as u8;
                        gfx.fill_rect(gfx.width - 90 + dx, y, 1, taskbar_h, Color::rgb(final_r, final_g, final_b).to_u32());
                    }
                }
            } else {
                gfx.fill_rect(gfx.width - 90, y, 90, taskbar_h, Color::rgb(220, 220, 230).to_u32());
            }
            if !ubuntu {
                gfx.draw_text(gfx.width - 80, y + 12, "16:20", t.taskbar_text.to_u32(), t.taskbar_bg.to_u32());
            }
        }
    }

    pub fn handle_mouse_press(&mut self, gfx: &mut Graphics, cx: usize, cy: usize) -> bool {
        let taskbar_y = gfx.height - 40;
        if cy >= taskbar_y {
            if cx >= 4 && cx < 59 {
                self.start_menu.open = !self.start_menu.open;
                return true;
            }
            let mut btn_x = 67;
            let ubuntu = is_ubuntu_theme();
            let btn_w = if ubuntu { 160 } else { 150 };
            for i in 0..self.window_count {
                if let Some(ref win) = self.windows[i] {
                    if win.state != WindowState::Minimized {
                        if cx >= btn_x && cx < btn_x + btn_w {
                            if win.state == WindowState::Minimized {
                                self.restore_window(i);
                            } else {
                                self.active_window = Some(i);
                            }
                            return true;
                        }
                        btn_x += btn_w + 4;
                    }
                }
            }
            return false;
        }

        for i in (0..self.window_count).rev() {
            if let Some(ref mut win) = self.windows[i] {
                if win.is_close_button(cx, cy) {
                    for j in i..self.window_count - 1 {
                        self.windows[j] = self.windows[j + 1].take();
                    }
                    self.windows[self.window_count - 1] = None;
                    self.window_count -= 1;
                    self.dragged_window = None;
                    self.active_window = if self.window_count > 0 { Some(self.window_count - 1) } else { None };
                    return true;
                }
                if win.is_maximize_button(cx, cy) {
                    if win.state == WindowState::Maximized {
                        let (rx, ry, rw, rh) = win.restore_rect;
                        win.x = rx; win.y = ry; win.width = rw; win.height = rh;
                        win.state = WindowState::Normal;
                    } else {
                        win.restore_rect = (win.x, win.y, win.width, win.height);
                        win.x = 0; win.y = 0;
                        win.width = gfx.width;
                        win.height = gfx.height - 40;
                        win.state = WindowState::Maximized;
                    }
                    self.active_window = Some(i);
                    return true;
                }
                if win.is_minimize_button(cx, cy) {
                    win.state = WindowState::Minimized;
                    self.dragged_window = None;
                    self.active_window = if i == 0 && self.window_count > 1 { Some(1) } else { Some(0) };
                    return true;
                }
                if win.is_titlebar(cx, cy) {
                    self.active_window = Some(i);
                    win.is_dragging = true;
                    win.drag_off_x = cx - win.x;
                    win.drag_off_y = cy - win.y;
                    self.dragged_window = Some(i);
                    save_background(gfx, win.x, win.y, win.width, win.height);
                    self.saved_bg_w = win.width;
                    self.saved_bg_h = win.height;
                    return true;
                }
            }
        }
        false
    }

    pub fn handle_mouse_release(&mut self, gfx: &mut Graphics) -> bool {
        if let Some(i) = self.dragged_window {
            if let Some(ref mut win) = self.windows[i] {
                win.is_dragging = false;
                win.draw(gfx, true);
            }
            self.dragged_window = None;
            return true;
        }
        false
    }

    pub fn handle_mouse_move(&mut self, gfx: &mut Graphics, cx: usize, cy: usize) -> bool {
        if let Some(i) = self.dragged_window {
            if let Some(ref mut win) = self.windows[i] {
                restore_background(gfx, win.x, win.y, self.saved_bg_w, self.saved_bg_h);
                win.x = if cx >= win.drag_off_x { cx - win.drag_off_x } else { 0 };
                win.y = if cy >= win.drag_off_y { cy - win.drag_off_y } else { 0 };
                if win.x + win.width > gfx.width { win.x = gfx.width - win.width; }
                if win.y + win.height > gfx.height { win.y = gfx.height - win.height; }
                save_background(gfx, win.x, win.y, win.width, win.height);
                self.saved_bg_w = win.width;
                self.saved_bg_h = win.height;
                win.draw(gfx, true);
            }
            return true;
        }
        false
    }

    pub fn save_dragged_background(&mut self, gfx: &Graphics) {
        if let Some(i) = self.dragged_window {
            if let Some(ref win) = self.windows[i] {
                save_background(gfx, win.x, win.y, win.width, win.height);
                self.saved_bg_w = win.width;
                self.saved_bg_h = win.height;
            }
        }
    }

    pub fn restore_window(&mut self, idx: usize) {
        if idx < self.window_count {
            if let Some(ref mut win) = self.windows[idx] {
                if win.state == WindowState::Minimized {
                    win.state = WindowState::Normal;
                    self.active_window = Some(idx);
                }
            }
        }
    }

    pub fn get_window_title(&self, idx: usize) -> Option<&'static str> {
        self.windows[idx].as_ref().map(|w| w.title)
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
        let start_x = cx + (w - 8 - total_width) / 2;
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
            let text_x = icon_x + (icon_size - text_width) / 2;
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