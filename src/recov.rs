
#![allow(dead_code)]

use crate::graphics::{Graphics, Color};
use crate::tamzen_font;


pub struct PanicInfo {
    pub error_code: u32,
    pub res: &'static str,
    pub shell: &'static str,
}


pub static mut PANIC_DATA: PanicInfo = PanicInfo {
    error_code: 0x2c0000,
    res: "System Central kernel",
    shell: "dos-shell-sys 1.0.0.0",
};


pub static mut PANIC_TRIGGERED: bool = false;


pub fn trigger_panic(gfx: &mut Graphics) {
    unsafe {
        PANIC_TRIGGERED = true;
        draw_panic_screen(gfx);
    }
}


pub fn trigger_panic_with_code(gfx: &mut Graphics, code: u32) {
    unsafe {
        PANIC_TRIGGERED = true;
        PANIC_DATA.error_code = code;
        draw_panic_screen(gfx);
    }
}


pub fn draw_panic_screen(gfx: &mut Graphics) {
    let screen_w = gfx.width;
    let screen_h = gfx.height;
    

    let bg_color = Color::rgb(0, 0, 170).to_u32();
    gfx.clear(bg_color);
    

    let text_color = Color::WHITE.to_u32();
    

    let title = "! Central Kernel FATAL ERROR !";
    let title_width = title.len() * tamzen_font::FONT_WIDTH;
    let title_x = (screen_w - title_width) / 2;
    let title_y = 40;
    

    gfx.fill_rect(20, title_y + 20, screen_w - 40, 4, Color::rgb(255, 0, 0).to_u32());
    gfx.draw_text(title_x, title_y, title, Color::rgb(255, 255, 255).to_u32(), bg_color);
    

    let sub_title = "[kernel panic]     [mi-kernel]";
    let sub_width = sub_title.len() * tamzen_font::FONT_WIDTH;
    let sub_x = (screen_w - sub_width) / 2;
    gfx.draw_text(sub_x, title_y + 30, sub_title, Color::WHITE.to_u32(), bg_color);
    

    let msg = "Please reboot your PC!";
    let msg_width = msg.len() * tamzen_font::FONT_WIDTH;
    let msg_x = (screen_w - msg_width) / 2;
    gfx.draw_text(msg_x, title_y + 70, msg, Color::WHITE.to_u32(), bg_color);
    

    let sep_y = title_y + 100;
    gfx.fill_rect(40, sep_y, screen_w - 80, 2, Color::rgb(255, 255, 255).to_u32());
    

    let tech_title = "tech info:";
    gfx.draw_text(40, sep_y + 30, tech_title, Color::WHITE.to_u32(), bg_color);
    

    unsafe {
        let error_code_str = format_hex(PANIC_DATA.error_code);
        let res_str = PANIC_DATA.res;
        let shell_str = PANIC_DATA.shell;
        
        let y_start = sep_y + 60;
        let label_color = Color::rgb(200, 200, 200).to_u32();
        let value_color = Color::WHITE.to_u32();
        

        gfx.draw_text(40, y_start, "[Error code:]", label_color, bg_color);
        gfx.draw_text(200, y_start, error_code_str, value_color, bg_color);
        

        gfx.draw_text(40, y_start + 30, "[res:]", label_color, bg_color);
        gfx.draw_text(200, y_start + 30, res_str, value_color, bg_color);
        

        gfx.draw_text(40, y_start + 60, "[shell:]", label_color, bg_color);
        gfx.draw_text(200, y_start + 60, shell_str, value_color, bg_color);
    }
    

    let footer_y = screen_h - 40;
    let footer = "System halted - Press any key to reboot";
    let footer_width = footer.len() * tamzen_font::FONT_WIDTH;
    let footer_x = (screen_w - footer_width) / 2;
    gfx.draw_text(footer_x, footer_y, footer, Color::rgb(150, 150, 150).to_u32(), bg_color);
    

    let cursor_y = footer_y + 25;
    let cursor_x = footer_x + footer.len() * 8;
    gfx.fill_rect(cursor_x, cursor_y, 4, 4, Color::WHITE.to_u32());
    
    gfx.flush();
}


fn format_hex(num: u32) -> &'static str {
    static mut BUF: [u8; 16] = [0; 16];
    unsafe {
        let hex_chars = b"0123456789ABCDEF";
        let mut pos = 0;
        let mut n = num;
        
        BUF[pos] = b'0'; pos += 1;
        BUF[pos] = b'x'; pos += 1;
        

        let mut started = false;
        for i in (0..8).rev() {
            let digit = ((n >> (i * 4)) & 0xF) as usize;
            if digit != 0 || started || i == 0 {
                BUF[pos] = hex_chars[digit];
                pos += 1;
                started = true;
            }
        }
        
        BUF[pos] = 0;
        core::str::from_utf8(&BUF[..pos]).unwrap_or("0x00000000")
    }
}


pub fn is_panic_active() -> bool {
    unsafe {
        PANIC_TRIGGERED
    }
}


pub fn reset_panic() {
    unsafe {
        PANIC_TRIGGERED = false;
        PANIC_DATA = PanicInfo {
            error_code: 0x2c0000,
            res: "System Central kernel",
            shell: "dos-shell-sys 1.0.0.0",
        };
    }
}