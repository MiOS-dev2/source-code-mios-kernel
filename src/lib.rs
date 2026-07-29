#![no_std]
#![allow(dead_code)]
#![feature(abi_x86_interrupt)]
#![allow(static_mut_refs)]

mod vga;
mod vesa;
mod multiboot;
mod keyboard;
mod ata;
mod fs;
mod console;
mod utils;
mod tamzen_font;
mod idt;
mod mouse;
mod bmp;
mod graphics;
mod wm;

use core::panic::PanicInfo;
use core::str;
use multiboot::MultibootInfo;
use graphics::{Graphics, Color};

pub static mut VESA_INFO: Option<vesa::VesaInfo> = None;

static mut GUI_ACTIVE: bool = false;
static mut BOOT_COMPLETE: bool = false;


static mut TERM_INPUT: [u8; 256] = [0; 256];
static mut TERM_LEN: usize = 0;
static mut TERM_HISTORY: [[u8; 80]; 1000] = [[b' '; 80]; 1000];
static mut TERM_HIST_LEN: usize = 0;


static mut GUI_BG_COLOR: Color = Color::rgb(0, 100, 200);
static mut CURSOR_X: usize = 400;
static mut CURSOR_Y: usize = 300;
static mut LEFT_BTN: bool = false;
static mut TERMINAL_WINDOW_OPEN: bool = false;
static mut TERMINAL_WINDOW_MINIMIZED: bool = false;


static mut DRAGGING: bool = false;
static mut DRAG_OFFSET_X: i32 = 0;
static mut DRAG_OFFSET_Y: i32 = 0;
static mut WIN_X: i32 = 100;
static mut WIN_Y: i32 = 100;
static mut WIN_W: i32 = 500;
static mut WIN_H: i32 = 350;


const MOUSE_CURSOR: [[u8; 16]; 16] = [
    [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
    [0,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
    [0,2,1,0,0,0,0,0,0,0,0,0,0,0,0,0],
    [0,2,2,1,0,0,0,0,0,0,0,0,0,0,0,0],
    [0,2,2,2,1,0,0,0,0,0,0,0,0,0,0,0],
    [0,2,2,2,2,1,0,0,0,0,0,0,0,0,0,0],
    [0,2,2,2,2,2,1,0,0,0,0,0,0,0,0,0],
    [0,2,2,2,2,2,2,1,0,0,0,0,0,0,0,0],
    [0,2,2,2,2,2,2,2,1,0,0,0,0,0,0,0],
    [0,2,2,2,2,2,2,2,2,1,0,0,0,0,0,0],
    [0,2,2,2,2,2,2,1,1,1,1,1,0,0,0,0],
    [0,2,2,2,2,1,2,2,1,0,0,0,0,0,0,0],
    [0,2,2,2,1,0,1,2,2,1,0,0,0,0,0,0],
    [0,2,2,1,0,0,1,2,2,1,0,0,0,0,0,0],
    [0,2,1,0,0,0,0,1,2,2,1,0,0,0,0,0],
    [0,0,0,0,0,0,0,0,1,1,1,0,0,0,0,0],
];

fn push_history(line: &str) {
    unsafe {
        if TERM_HIST_LEN < 1000 {
            let bytes = line.as_bytes();
            let len = if bytes.len() > 80 { 80 } else { bytes.len() };
            TERM_HISTORY[TERM_HIST_LEN][..len].copy_from_slice(&bytes[..len]);
            TERM_HIST_LEN += 1;
        }
    }
}

fn clear_history() {
    unsafe {
        TERM_HIST_LEN = 0;
        for i in 0..1000 {
            for j in 0..80 {
                TERM_HISTORY[i][j] = b' ';
            }
        }
    }
}

fn draw_boot_logo(gfx: &mut Graphics) {
    let screen_w = gfx.width;
    
    gfx.clear(Color::BLACK.to_u32());
    
    let title = "MiOS Kernel";
    let title_width = title.len() * tamzen_font::FONT_WIDTH;
    let title_x = (screen_w - title_width) / 2;
    let title_y = 20;
    gfx.draw_text(title_x, title_y, title, Color::rgb(102, 102, 102).to_u32(), Color::BLACK.to_u32());
    
    let boot_msgs = [
        "[  OK  ] Initializing VESA framebuffer...",
        "[  OK  ] Loading kernel modules...",
        "[  OK  ] Initializing ATA driver...",
        "[  OK  ] Mounting root filesystem...",
        "[  OK  ] Starting system services...",
        "[  OK  ] Loading terminal interface...",
        "[  OK  ] Initializing PS/2 mouse...",
        "[  OK  ] Setting up interrupt handlers...",
        "[  OK  ] Loading user database...",
        "[  OK  ] Starting shell...",
        "[  OK  ] System ready.",
    ];
    
    let start_y = 80;
    for (i, msg) in boot_msgs.iter().enumerate() {
        let y = start_y + i * 20;
        gfx.draw_text(20, y, msg, Color::rgb(200, 200, 200).to_u32(), Color::BLACK.to_u32());
        gfx.flush();
        
        for _ in 0..2000000 {
            unsafe { core::arch::asm!("nop") }
        }
    }
    
    for _ in 0..10000000 {
        unsafe { core::arch::asm!("nop") }
    }
}

fn draw_dos_terminal(gfx: &mut Graphics) {
    let screen_w = gfx.width;
    let screen_h = gfx.height;
    

    gfx.clear(Color::rgb(0, 0, 0).to_u32());
    
    unsafe {
        let hist_len = TERM_HIST_LEN;
        let max_lines = (screen_h - 20) / 16;
        let start_row = if hist_len > max_lines { 
            hist_len - max_lines 
        } else { 
            0 
        };
        

        for row in start_row..hist_len {
            let line = core::str::from_utf8(&TERM_HISTORY[row]).unwrap_or("");
            let max_chars = screen_w / 8 - 1;
            let display_line = if line.len() > max_chars { 
                &line[..max_chars] 
            } else { 
                line 
            };
            let y_pos = (row - start_row) * 16;
            

            if line.starts_with("mios-kernel:~#") {
                gfx.draw_text(0, y_pos, display_line, Color::rgb(105, 105, 105).to_u32(), Color::BLACK.to_u32());
            } else if line.starts_with("Error") || line.starts_with("Unknown") {
                gfx.draw_text(0, y_pos, display_line, Color::rgb(129, 129, 129).to_u32(), Color::BLACK.to_u32());
            } else {
                gfx.draw_text(0, y_pos, display_line, Color::WHITE.to_u32(), Color::BLACK.to_u32());
            }
        }
        

        let prompt_y = (hist_len - start_row) * 16;
        let prompt = "mios-kernel:~# ";
        gfx.draw_text(0, prompt_y, prompt, Color::rgb(104, 104, 104).to_u32(), Color::BLACK.to_u32());
        

        let prompt_len = prompt.len();
        if TERM_LEN > 0 {
            let input = core::str::from_utf8(&TERM_INPUT[..TERM_LEN]).unwrap_or("");
            let max_input = screen_w / 8 - prompt_len - 1;
            let display_input = if input.len() > max_input {
                &input[input.len() - max_input..]
            } else {
                input
            };
            gfx.draw_text(prompt_len * 8, prompt_y, display_input, Color::WHITE.to_u32(), Color::BLACK.to_u32());
        }
    }
}

fn draw_window(gfx: &mut Graphics) {
    unsafe {
        let x = WIN_X as usize;
        let y = WIN_Y as usize;
        let w = WIN_W as usize;
        let h = WIN_H as usize;
        

        for dy in 0..5 {
            for dx in 0..5 {
                if dx + dy < 5 {
                    gfx.fill_rect(x + w + dx, y + 5 + dy, 1, 1, Color::rgb(0, 0, 0).to_u32());
                    gfx.fill_rect(x + 5 + dx, y + h + dy, 1, 1, Color::rgb(0, 0, 0).to_u32());
                }
            }
        }
        

        gfx.fill_rect(x, y, w, h, Color::rgb(235, 235, 235).to_u32());
        gfx.draw_rect_border(x, y, w, h, Color::rgb(87, 87, 87).to_u32());
        gfx.draw_rect_border(x + 1, y + 1, w - 2, h - 2, Color::WHITE.to_u32());
        

        let title_bar_h = 30;
        gfx.fill_rect(x + 2, y + 2, w - 4, title_bar_h, Color::rgb(56, 56, 56).to_u32());
        gfx.draw_text(x + 10, y + 8, "Terminal", Color::WHITE.to_u32(), Color::rgb(90, 90, 90).to_u32());
        

        let btn_min_x = x + w - 55;
        let btn_min_y = y + 5;
        gfx.fill_rect(btn_min_x, btn_min_y, 20, 20, Color::rgb(180, 180, 180).to_u32());
        gfx.draw_rect_border(btn_min_x, btn_min_y, 20, 20, Color::rgb(100, 100, 100).to_u32());
        gfx.fill_rect(btn_min_x + 4, btn_min_y + 10, 12, 2, Color::BLACK.to_u32());
        

        let btn_close_x = x + w - 30;
        let btn_close_y = y + 5;
        gfx.fill_rect(btn_close_x, btn_close_y, 20, 20, Color::rgb(200, 80, 80).to_u32());
        gfx.draw_rect_border(btn_close_x, btn_close_y, 20, 20, Color::rgb(100, 100, 100).to_u32());
        gfx.draw_text(btn_close_x + 6, btn_close_y + 4, "X", Color::WHITE.to_u32(), Color::rgb(200, 80, 80).to_u32());
        

        let cx = x + 2;
        let cy = y + title_bar_h + 2;
        let cw = w - 4;
        let ch = h - title_bar_h - 4;
        

        gfx.fill_rect(cx, cy, cw, ch, Color::BLACK.to_u32());
        gfx.draw_rect_border(cx, cy, cw, ch, Color::rgb(128, 128, 128).to_u32());
        

        let hist_len = TERM_HIST_LEN;
        let max_lines = (ch - 20) / 16;
        let start_row = if hist_len > max_lines { 
            hist_len - max_lines 
        } else { 
            0 
        };
        
        for row in start_row..hist_len {
            let line = core::str::from_utf8(&TERM_HISTORY[row]).unwrap_or("");
            let max_chars = (cw - 20) / 8;
            let display_line = if line.len() > max_chars { 
                &line[..max_chars] 
            } else { 
                line 
            };
            let y_pos = cy + 10 + (row - start_row) * 16;
            
            if line.starts_with("mios-kernel:~#") {
                gfx.draw_text(cx + 10, y_pos, display_line, Color::rgb(93, 94, 93).to_u32(), Color::BLACK.to_u32());
            } else if line.starts_with("Error") || line.starts_with("Unknown") {
                gfx.draw_text(cx + 10, y_pos, display_line, Color::rgb(110, 109, 109).to_u32(), Color::BLACK.to_u32());
            } else {
                gfx.draw_text(cx + 10, y_pos, display_line, Color::WHITE.to_u32(), Color::BLACK.to_u32());
            }
        }
        

        let prompt_y = cy + 10 + (hist_len - start_row) * 16;
        let prompt = "mios-kernel:~# ";
        gfx.draw_text(cx + 10, prompt_y, prompt, Color::rgb(133, 133, 133).to_u32(), Color::BLACK.to_u32());
        

        if TERM_LEN > 0 {
            let input = core::str::from_utf8(&TERM_INPUT[..TERM_LEN]).unwrap_or("");
            gfx.draw_text(cx + 10 + prompt.len() * 8, prompt_y, input, Color::WHITE.to_u32(), Color::BLACK.to_u32());
        }
    }
}

fn draw_gui_mode(gfx: &mut Graphics) {
    let screen_w = gfx.width;
    let screen_h = gfx.height;
    
    unsafe {

        gfx.clear(GUI_BG_COLOR.to_u32());
        

        if TERMINAL_WINDOW_OPEN && !TERMINAL_WINDOW_MINIMIZED {
            draw_window(gfx);
        }
        

        if TERMINAL_WINDOW_OPEN && TERMINAL_WINDOW_MINIMIZED {

            let taskbar_y = screen_h - 30;
            gfx.fill_rect(0, taskbar_y, screen_w, 30, Color::rgb(3, 3, 3).to_u32());
            

            let btn_x = 10;
            let btn_y = taskbar_y + 2;
            gfx.fill_rect(btn_x, btn_y, 100, 26, Color::rgb(70, 70, 70).to_u32());
            gfx.draw_rect_border(btn_x, btn_y, 100, 26, Color::rgb(120, 120, 120).to_u32());
            gfx.draw_text(btn_x + 10, btn_y + 6, "Terminal", Color::WHITE.to_u32(), Color::rgb(70, 70, 70).to_u32());
        }
    }
}

fn exec_command(input: &str) -> &'static str {
    let trimmed = input.trim();
    
    if trimmed.is_empty() {
        return "";
    }
    
    let mut parts = [""; 8];
    let mut part_count = 0;
    for word in trimmed.split_whitespace() {
        if part_count < 8 {
            parts[part_count] = word;
            part_count += 1;
        }
    }
    
    match parts[0] {
        "help" => {
            return "Available commands:\n  help     - Show this help\n  tech     - System information\n  gui      - GUI commands: /start, /stop\n  cls      - Clear terminal\n  ver      - Show version\n  mem      - Show memory info\n  uptime   - Show uptime\n  about    - About MiOS\n  kernel   - Kernel info\n  devs     - Developers\n  dice     - Roll a dice\ndir , ls   - OS Dir\n hostname - name host kernel\n  reboot   - Reboot system\n  shutdown - Shutdown system";
        }
        "tech" => {
            return "MiOS Kernel\nArchitecture: x64\nVESA Mode: 800x600\nTerminal: VESA-based\nBuild Date: 2026";
        }
        "gui" => {
            if part_count >= 2 {
                match parts[1] {
                    "/start" => {
                        unsafe { 
                            GUI_ACTIVE = true;
                            TERMINAL_WINDOW_OPEN = true;
                            TERMINAL_WINDOW_MINIMIZED = false;
                            WIN_X = 100;
                            WIN_Y = 100;
                            DRAGGING = false;
                        }
                        return "[KERNEL] starting gui...";
                    }
                    "/stop" => {
                        unsafe { 
                            GUI_ACTIVE = false;
                            TERMINAL_WINDOW_OPEN = false;
                            TERMINAL_WINDOW_MINIMIZED = false;
                            DRAGGING = false;
                        }
                        return "[KERNEL] gui stop... ";
                    }
                    _ => return "Usage: gui /start  or  gui /stop",
                }
            }
            return "Usage: gui /start  or  gui /stop";
        }
        "cls" | "clear" => {
            clear_history();
            return "\x04";
        }
        "ver" => {
            return "MiOS Kernel";
        }
        "mem" => {
            return "Memory: 4096 MB (detected)";
        }
        "uptime" => {
            return "Uptime: 0 ticks";
        }
        "about" => {
            return "MiOS  - Free Kernel\nCreated in 2026\nBy MDEVS";
        }
        "dir" => {
            return "MiOS Dir:\n/mios/\nimage.old\nconfig.cfg\nboot.cfg";
        }
        "ls" => {
            return "MiOS Dir:\n/mios/\nimage.old\nconfig.cfg\nboot.cfg";
        }
        "kernel" => {
            return "MiOS Kernel \nFilesystem: FAT32 (Beta)\nBootloader: Multiboot GNU/GRUB";
        }
        "devs" => {
            return "MiOS Developers: MDEVS Team";
        }
        "hostname" => {
            return "mios-kernel";
        }
        "dice" => {
            let result = (core::hint::black_box(42) % 6) + 1;
            static mut BUF: [u8; 16] = [0; 16];
            unsafe {
                let bytes = b"Roll: ";
                for i in 0..6 { BUF[i] = bytes[i]; }
                let n = result as u8 + b'0';
                BUF[6] = n;
                BUF[7] = 0;
                core::str::from_utf8(&BUF[..7]).unwrap_or("Roll: ?")
            }
        }
        "reboot" => {
            unsafe {
                for _ in 0..3 {
                    core::arch::asm!("mov al, 0xFE", "out 0x64, al", options(nostack));
                }
            }
            return "Rebooting...";
        }
        "shutdown" => {
            unsafe {
                core::arch::asm!("cli", "hlt", options(nostack));
            }
            return "System halted.";
        }
        _ => {
            return "Unknown command. Type 'help' for available commands.";
        }
    }
}

fn draw_mouse_cursor(gfx: &mut Graphics, x: usize, y: usize) {
    let cursor_size = 16;
    for dy in 0..cursor_size {
        for dx in 0..cursor_size {
            if x + dx < gfx.width && y + dy < gfx.height {
                let pixel = MOUSE_CURSOR[dy][dx];
                if pixel == 1 {
                    gfx.put_pixel(x + dx, y + dy, Color::BLACK.to_u32());
                } else if pixel == 2 {
                    gfx.put_pixel(x + dx, y + dy, Color::WHITE.to_u32());
                }
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn rust_main(magic: u32, info_addr: u32) -> ! {
    if magic != 0x36d76289 {
        loop {}
    }

    let mb = unsafe { MultibootInfo::parse(info_addr) };

    if let Some(fb) = mb.framebuffer {
        if fb.addr != 0 && fb.width > 0 && fb.height > 0 {
            let vesa_info = vesa::VesaInfo {
                addr: fb.addr as usize,
                width: fb.width as usize,
                height: fb.height as usize,
                pitch: fb.pitch as usize,
                bpp: fb.bpp,
            };
            unsafe { VESA_INFO = Some(vesa_info); }

            let mut gfx = Graphics::new(fb.addr, fb.width, fb.height, fb.pitch);
            

            unsafe { idt::init_idt(); }
            mouse::init_ps2_mouse();
            unsafe { mouse::init_mouse_interrupts(); }


            draw_boot_logo(&mut gfx);
            

            clear_history();
            push_history("MiOS Kernel");
            push_history("Type 'help' for available commands");
            push_history("");

            unsafe { BOOT_COMPLETE = true; }


            draw_dos_terminal(&mut gfx);
            gfx.flush();

            let mut cursor_x = 400usize;
            let mut cursor_y = 300usize;
            let cursor_size = 16;
            let mut cursor_save = [[0u32; 16]; 16];
            let mut left_prev = false;
            let mut mouse_initialized = false;

            loop {
                let mut need_flush = false;
                let mut mouse_redraw = false;


                if let Some(sc) = mouse::get_key() {
                    unsafe {
                        if sc & 0x80 == 0 {
                            if GUI_ACTIVE {

                                if sc == 0x01 {
                                    TERMINAL_WINDOW_OPEN = false;
                                    TERMINAL_WINDOW_MINIMIZED = false;
                                    DRAGGING = false;
                                    need_flush = true;
                                }
                            }
                            

                            let c = match sc {
                                0x1E => Some('a'), 0x30 => Some('b'), 0x2E => Some('c'),
                                0x20 => Some('d'), 0x12 => Some('e'), 0x21 => Some('f'),
                                0x22 => Some('g'), 0x23 => Some('h'), 0x17 => Some('i'),
                                0x24 => Some('j'), 0x25 => Some('k'), 0x26 => Some('l'),
                                0x32 => Some('m'), 0x31 => Some('n'), 0x18 => Some('o'),
                                0x19 => Some('p'), 0x10 => Some('q'), 0x13 => Some('r'),
                                0x1F => Some('s'), 0x14 => Some('t'), 0x16 => Some('u'),
                                0x2F => Some('v'), 0x11 => Some('w'), 0x2D => Some('x'),
                                0x15 => Some('y'), 0x2C => Some('z'),
                                0x39 => Some(' '),
                                0x1C => Some('\n'),
                                0x0E => Some('\x08'),
                                0x02 => Some('1'), 0x03 => Some('2'), 0x04 => Some('3'),
                                0x05 => Some('4'), 0x06 => Some('5'), 0x07 => Some('6'),
                                0x08 => Some('7'), 0x09 => Some('8'), 0x0A => Some('9'),
                                0x0B => Some('0'),
                                0x33 => Some(','), 0x34 => Some('.'), 0x35 => Some('/'),
                                0x27 => Some(';'), 0x28 => Some('\''), 0x2B => Some('\\'),
                                0x29 => Some('`'), 0x0C => Some('-'), 0x0D => Some('='),
                                0x1A => Some('['), 0x1B => Some(']'),
                                _ => None,
                            };
                            
                            if let Some(ch) = c {
                                if ch == '\n' {
                                    let cmd = core::str::from_utf8(&TERM_INPUT[..TERM_LEN]).unwrap_or("");
                                    let output = exec_command(cmd);
                                    
                                    if output == "\x04" {

                                    } else {
                                        push_history("mios-kernel:~# ");
                                        if !cmd.is_empty() {
                                            push_history(cmd);
                                        }
                                        if !output.is_empty() {
                                            for line in output.split('\n') {
                                                push_history(line);
                                            }
                                        }
                                        push_history("");
                                    }
                                    TERM_LEN = 0;
                                    need_flush = true;
                                } else if ch == '\x08' && TERM_LEN > 0 {
                                    TERM_LEN -= 1;
                                    need_flush = true;
                                } else if (ch as u8) >= 0x20 && TERM_LEN < 255 {
                                    TERM_INPUT[TERM_LEN] = ch as u8;
                                    TERM_LEN += 1;
                                    need_flush = true;
                                }
                            }
                        }
                    }
                }


                if let Some(packet) = mouse::get_mouse_packet() {
                    unsafe {
                        if !mouse_initialized {
                            cursor_x = 400;
                            cursor_y = 300;
                            mouse_initialized = true;
                        }
                        
                        let new_x = (cursor_x as i32 + packet.dx).max(0).min((gfx.width - cursor_size) as i32) as usize;
                        let new_y = (cursor_y as i32 + packet.dy).max(0).min((gfx.height - cursor_size) as i32) as usize;
                        

                        for dy in 0..cursor_size {
                            for dx in 0..cursor_size {
                                if cursor_x + dx < gfx.width && cursor_y + dy < gfx.height {
                                    gfx.put_pixel(cursor_x + dx, cursor_y + dy, cursor_save[dy][dx]);
                                }
                            }
                        }
                        
                        cursor_x = new_x;
                        cursor_y = new_y;
                        

                        for dy in 0..cursor_size {
                            for dx in 0..cursor_size {
                                if cursor_x + dx < gfx.width && cursor_y + dy < gfx.height {
                                    cursor_save[dy][dx] = graphics::BACKBUFFER[(cursor_y + dy) * gfx.width + (cursor_x + dx)];
                                }
                            }
                        }
                        

                        if GUI_ACTIVE {
                            draw_mouse_cursor(&mut gfx, cursor_x, cursor_y);
                            mouse_redraw = true;
                            

                            if packet.left && !left_prev {

                                if TERMINAL_WINDOW_OPEN && TERMINAL_WINDOW_MINIMIZED {
                                    let taskbar_y = gfx.height - 30;
                                    let btn_x = 10;
                                    if cursor_y >= taskbar_y + 2 && cursor_y <= taskbar_y + 28 &&
                                       cursor_x >= btn_x && cursor_x <= btn_x + 100 {
                                        TERMINAL_WINDOW_MINIMIZED = false;
                                        need_flush = true;
                                    }
                                }
                                

                                if TERMINAL_WINDOW_OPEN && !TERMINAL_WINDOW_MINIMIZED {
                                    let x = WIN_X;
                                    let y = WIN_Y;
                                    let w = WIN_W;
                                    

                                    if cursor_x as i32 >= x && cursor_x as i32 <= x + w &&
                                       cursor_y as i32 >= y && cursor_y as i32 <= y + 30 {
                                        DRAGGING = true;
                                        DRAG_OFFSET_X = cursor_x as i32 - x;
                                        DRAG_OFFSET_Y = cursor_y as i32 - y;
                                    }
                                    

                                    let close_x = x + w - 30;
                                    let close_y = y + 5;
                                    if cursor_x as i32 >= close_x && cursor_x as i32 <= close_x + 20 &&
                                       cursor_y as i32 >= close_y && cursor_y as i32 <= close_y + 20 {
                                        TERMINAL_WINDOW_OPEN = false;
                                        TERMINAL_WINDOW_MINIMIZED = false;
                                        DRAGGING = false;
                                        need_flush = true;
                                    }
                                    

                                    let min_x = x + w - 55;
                                    let min_y = y + 5;
                                    if cursor_x as i32 >= min_x && cursor_x as i32 <= min_x + 20 &&
                                       cursor_y as i32 >= min_y && cursor_y as i32 <= min_y + 20 {
                                        TERMINAL_WINDOW_MINIMIZED = true;
                                        DRAGGING = false;
                                        need_flush = true;
                                    }
                                }
                            }
                            

                            if packet.left && DRAGGING && TERMINAL_WINDOW_OPEN && !TERMINAL_WINDOW_MINIMIZED {
                                let new_x = cursor_x as i32 - DRAG_OFFSET_X;
                                let new_y = cursor_y as i32 - DRAG_OFFSET_Y;
                                WIN_X = new_x.max(0).min(gfx.width as i32 - WIN_W);
                                WIN_Y = new_y.max(0).min(gfx.height as i32 - WIN_H - 30);
                                need_flush = true;
                            }
                            
                            if !packet.left {
                                DRAGGING = false;
                            }
                            
                            left_prev = packet.left;
                        }
                    }
                }


                if need_flush {
                    unsafe {
                        if GUI_ACTIVE {
                            draw_gui_mode(&mut gfx);

                            draw_mouse_cursor(&mut gfx, cursor_x, cursor_y);
                        } else {
                            draw_dos_terminal(&mut gfx);
                        }
                        gfx.flush();
                    }
                }

                if mouse_redraw && !need_flush {
                    gfx.flush();
                }


                for _ in 0..1000 {
                    unsafe { core::arch::asm!("nop") }
                }
            }
        }
    }

    loop {
        unsafe { core::arch::asm!("hlt") }
    }
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}