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
mod atapi;
mod recov;

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


static mut HOSTNAME: [u8; 64] = [0; 64];
static mut CURRENT_DIR: [u8; 64] = [0; 64];
static mut BOOT_TICKS: usize = 0;


static mut CONFIG_COLOR: u8 = 0; 
static mut CONFIG_MOUSE_VISIBLE: bool = false;
static mut CONFIG_MOUSE_X: usize = 400;
static mut CONFIG_MOUSE_Y: usize = 300;


fn get_config_colors() -> (u32, u32) {
    unsafe {
        match CONFIG_COLOR {
            1 => (Color::WHITE.to_u32(), Color::rgb(200, 0, 0).to_u32()), // red
            2 => (Color::WHITE.to_u32(), Color::rgb(0, 0, 200).to_u32()), // blue
            3 => (Color::rgb(0, 255, 0).to_u32(), Color::BLACK.to_u32()), // green
            _ => (Color::WHITE.to_u32(), Color::BLACK.to_u32()), // def
        }
    }
}

fn set_config_defaults() {
    unsafe {
        CONFIG_COLOR = 0;
        CONFIG_MOUSE_VISIBLE = false;
        CONFIG_MOUSE_X = 400;
        CONFIG_MOUSE_Y = 300;
    }
}

fn is_config_default() -> bool {
    unsafe {
        CONFIG_COLOR == 0 && CONFIG_MOUSE_VISIBLE == false
    }
}

#[derive(Copy, Clone)]
struct FileEntry {
    name: [u8; 32],
    is_dir: bool,
    size: u32,
}

static mut FILE_LIST: [FileEntry; 64] = [FileEntry {
    name: [0; 32],
    is_dir: false,
    size: 0,
}; 64];
static mut FILE_COUNT: usize = 0;


static mut TED_ACTIVE: bool = false;
static mut TED_TEXT: [[u8; 80]; 100] = [[b' '; 80]; 100];
static mut TED_ROW: usize = 0;
static mut TED_COL: usize = 0;
static mut TED_LINE: usize = 0;


const MAX_USERS: usize = 10;
const MAX_NAME_LEN: usize = 32;

#[derive(Copy, Clone)]
struct User {
    id: u32,
    name: [u8; MAX_NAME_LEN],
    is_temp: bool,
    is_active: bool,
}

static mut USERS: [User; MAX_USERS] = [User {
    id: 0,
    name: [0; MAX_NAME_LEN],
    is_temp: false,
    is_active: false,
}; MAX_USERS];

static mut USER_COUNT: usize = 0;
static mut CURRENT_USER_ID: u32 = 0;
static mut IS_LOGGED_IN: bool = false;
static mut NEED_LOGIN: bool = true;


fn init_users() {
    unsafe {
        let main_name = b"Main User";
        let name_len = if main_name.len() < MAX_NAME_LEN { main_name.len() } else { MAX_NAME_LEN - 1 };
        USERS[0].id = 4456;
        USERS[0].name[..name_len].copy_from_slice(&main_name[..name_len]);
        USERS[0].name[name_len] = 0;
        USERS[0].is_temp = false;
        USERS[0].is_active = true;
        
        USER_COUNT = 1;
        CURRENT_USER_ID = 4456;
        IS_LOGGED_IN = false;
        NEED_LOGIN = true;
    }
}

fn get_user_name(id: u32) -> &'static str {
    unsafe {
        for i in 0..USER_COUNT {
            if USERS[i].id == id && USERS[i].is_active {
                let name = core::str::from_utf8(&USERS[i].name).unwrap_or("Unknown");
                return name.trim_end_matches('\0');
            }
        }
        "Unknown"
    }
}

fn user_exists(id: u32) -> bool {
    unsafe {
        for i in 0..USER_COUNT {
            if USERS[i].id == id && USERS[i].is_active {
                return true;
            }
        }
        false
    }
}

fn login(user_id: u32) -> bool {
    unsafe {
        if user_exists(user_id) {
            CURRENT_USER_ID = user_id;
            IS_LOGGED_IN = true;
            NEED_LOGIN = false;
            true
        } else {
            false
        }
    }
}

fn logout() -> bool {
    unsafe {
        if IS_LOGGED_IN {
            IS_LOGGED_IN = false;
            CURRENT_USER_ID = 0;
            NEED_LOGIN = true;
            true
        } else {
            false
        }
    }
}

fn is_current_user_main() -> bool {
    unsafe {
        IS_LOGGED_IN && CURRENT_USER_ID == 4456
    }
}

fn create_temp_user() -> Result<u32, &'static str> {
    unsafe {
        for i in 0..USER_COUNT {
            if USERS[i].is_temp && USERS[i].is_active {
                return Err("Temporary user already exists!");
            }
        }
        
        if USER_COUNT >= MAX_USERS {
            return Err("Maximum users reached!");
        }
        
        let temp_id = 4356;
        let temp_name = b"Temp User";
        let name_len = if temp_name.len() < MAX_NAME_LEN { temp_name.len() } else { MAX_NAME_LEN - 1 };
        
        USERS[USER_COUNT].id = temp_id;
        USERS[USER_COUNT].name[..name_len].copy_from_slice(&temp_name[..name_len]);
        USERS[USER_COUNT].name[name_len] = 0;
        USERS[USER_COUNT].is_temp = true;
        USERS[USER_COUNT].is_active = true;
        
        USER_COUNT += 1;
        
        Ok(temp_id)
    }
}

fn list_users() -> &'static str {
    unsafe {
        static mut BUFFER: [u8; 512] = [0; 512];
        let mut pos = 0;
        
        for i in 0..USER_COUNT {
            if USERS[i].is_active {
                let name = core::str::from_utf8(&USERS[i].name).unwrap_or("Unknown");
                let trimmed = name.trim_end_matches('\0');
                let status = if USERS[i].is_temp { " [TEMP]" } else { " [MAIN]" };
                let active = if CURRENT_USER_ID == USERS[i].id && IS_LOGGED_IN { " <--" } else { "" };
                
                let id_str = num_to_str(USERS[i].id as usize);
                
                for b in id_str.as_bytes() {
                    if pos < 511 { BUFFER[pos] = *b; pos += 1; }
                }
                BUFFER[pos] = b':'; pos += 1;
                BUFFER[pos] = b' '; pos += 1;
                
                for b in trimmed.as_bytes() {
                    if pos < 511 { BUFFER[pos] = *b; pos += 1; }
                }
                
                for b in status.as_bytes() {
                    if pos < 511 { BUFFER[pos] = *b; pos += 1; }
                }
                
                for b in active.as_bytes() {
                    if pos < 511 { BUFFER[pos] = *b; pos += 1; }
                }
                
                BUFFER[pos] = b'\n'; pos += 1;
            }
        }
        
        BUFFER[pos] = 0;
        core::str::from_utf8(&BUFFER[..pos]).unwrap_or("")
    }
}


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

fn num_to_str(num: usize) -> &'static str {
    static mut BUF: [u8; 10] = [0; 10];
    unsafe {
        let mut temp = num;
        let mut digits = [0u8; 10];
        let mut i = 0;
        if temp == 0 {
            BUF[0] = b'0';
            BUF[1] = 0;
            return "0";
        }
        while temp > 0 {
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
        core::str::from_utf8(&BUF[..j]).unwrap_or("0")
    }
}


fn draw_corner_char(gfx: &mut Graphics, x: usize, y: usize, color: u32, bg: u32, corner_type: u8) {
    let chars = match corner_type {
        0 => "┌",
        1 => "┐",
        2 => "└",
        3 => "┘",
        4 => "─",
        5 => "│",
        _ => " ",
    };
    gfx.draw_text(x, y, chars, color, bg);
}


fn read_iso_sector(sector: u32, buf: &mut [u8; 2048]) -> bool {
    if atapi::read_cd_sector(sector, buf) {
        return true;
    }
    
    unsafe {
        let iso_addr = 0x100000 as *const u8;
        let offset = (sector * 2048) as usize;
        let src = iso_addr.add(offset);
        for i in 0..2048 {
            buf[i] = *src.add(i);
        }
        return true;
    }
}

fn init_filesystem() {
    unsafe {
        let hostname_bytes = b"mios-kernel";
        for i in 0..hostname_bytes.len() {
            HOSTNAME[i] = hostname_bytes[i];
        }
        HOSTNAME[hostname_bytes.len()] = 0;
        
        let dir_bytes = b"/mios";
        for i in 0..dir_bytes.len() {
            CURRENT_DIR[i] = dir_bytes[i];
        }
        CURRENT_DIR[dir_bytes.len()] = 0;
        
        let mut sector = [0u8; 2048];
        if !read_iso_sector(16, &mut sector) {
            push_history("[ISO] Failed to read ISO sectors");
            create_test_filesystem();
            return;
        }
        
        let is_iso = &sector[1..6] == b"CD001";
        
        if is_iso {
            push_history("[ISO] ISO 9660 filesystem detected");
            
            let root_dir_lba = u32::from_le_bytes([
                sector[158], sector[159], sector[160], sector[161]
            ]);
            
            let mut root_sector = [0u8; 2048];
            if !read_iso_sector(root_dir_lba, &mut root_sector) {
                push_history("[ISO] Failed to read root directory");
                create_test_filesystem();
                return;
            }
            
            let mut idx = 0;
            let mut offset = 0;
            
            while offset < 2048 && idx < 64 {
                let entry_len = root_sector[offset] as usize;
                if entry_len == 0 {
                    break;
                }
                
                let name_len = root_sector[offset + 32] as usize;
                let name_start = offset + 33;
                
                if name_len > 0 && name_len <= 31 {
                    let mut is_dir = false;
                    let mut size = 0u32;
                    
                    if (root_sector[offset + 25] & 0x02) != 0 {
                        is_dir = true;
                    }
                    
                    if !is_dir {
                        let size_bytes = &root_sector[offset + 10..offset + 14];
                        size = u32::from_le_bytes([
                            size_bytes[0], size_bytes[1], 
                            size_bytes[2], size_bytes[3]
                        ]);
                    }
                    
                    let name = &root_sector[name_start..name_start + name_len];
                    let name_str = core::str::from_utf8(name).unwrap_or("");
                    
                    if name_str != "." && name_str != ".." && name_str.len() > 0 {
                        FILE_LIST[idx].name[..name_len].copy_from_slice(name);
                        FILE_LIST[idx].name[name_len] = 0;
                        FILE_LIST[idx].is_dir = is_dir;
                        FILE_LIST[idx].size = size;
                        idx += 1;
                    }
                }
                
                offset += entry_len;
            }
            
            FILE_COUNT = idx;
            push_history("[ISO] Loaded files from ISO");
        } else {
            push_history("[ISO] Not a valid ISO 9660 filesystem");
            create_test_filesystem();
        }
    }
}

fn create_test_filesystem() {
    unsafe {
        let dirs = ["/mios", "/boot", "/system"];
        let files = ["kernel.bin", "initrd.img", "boot.cfg", "config.cfg", "grub.cfg"];
        
        let mut idx = 0;
        for dir in dirs.iter() {
            if idx < 64 {
                let bytes = dir.as_bytes();
                let len = bytes.len().min(31);
                FILE_LIST[idx].name[..len].copy_from_slice(&bytes[..len]);
                FILE_LIST[idx].name[len] = 0;
                FILE_LIST[idx].is_dir = true;
                FILE_LIST[idx].size = 0;
                idx += 1;
            }
        }
        
        for file in files.iter() {
            if idx < 64 {
                let bytes = file.as_bytes();
                let len = bytes.len().min(31);
                FILE_LIST[idx].name[..len].copy_from_slice(&bytes[..len]);
                FILE_LIST[idx].name[len] = 0;
                FILE_LIST[idx].is_dir = false;
                FILE_LIST[idx].size = 1024 + (idx * 512) as u32;
                idx += 1;
            }
        }
        
        FILE_COUNT = idx;
    }
}

fn list_directory(path: &str) -> &'static str {
    unsafe {
        static mut RESULT: [u8; 1024] = [0; 1024];
        let mut pos = 0;
        
        let header = b"Directory: ";
        for b in header {
            if pos < 1023 { RESULT[pos] = *b; pos += 1; }
        }
        for b in path.as_bytes() {
            if pos < 1023 { RESULT[pos] = *b; pos += 1; }
        }
        RESULT[pos] = b'\n'; pos += 1;
        for _ in 0..20 {
            RESULT[pos] = b'='; pos += 1;
        }
        RESULT[pos] = b'\n'; pos += 1;
        
        let cd_status = if atapi::CD_PRESENT { 
            atapi::get_cd_info()
        } else { 
            "From memory (GRUB)"
        };
        for b in cd_status.as_bytes() {
            if pos < 1023 { RESULT[pos] = *b; pos += 1; }
        }
        RESULT[pos] = b'\n'; pos += 1;
        for _ in 0..20 {
            RESULT[pos] = b'='; pos += 1;
        }
        RESULT[pos] = b'\n'; pos += 1;
        
        for i in 0..FILE_COUNT {
            let name = core::str::from_utf8(&FILE_LIST[i].name).unwrap_or("");
            let name_trimmed = name.trim_end_matches('\0');
            
            if name_trimmed.len() > 0 {
                if FILE_LIST[i].is_dir {
                    RESULT[pos] = b'['; pos += 1;
                    RESULT[pos] = b'D'; pos += 1;
                    RESULT[pos] = b']'; pos += 1;
                    RESULT[pos] = b' '; pos += 1;
                } else {
                    RESULT[pos] = b'['; pos += 1;
                    RESULT[pos] = b'F'; pos += 1;
                    RESULT[pos] = b']'; pos += 1;
                    RESULT[pos] = b' '; pos += 1;
                }
                
                for b in name_trimmed.as_bytes() {
                    if pos < 1023 { RESULT[pos] = *b; pos += 1; }
                }
                
                if !FILE_LIST[i].is_dir {
                    let size_str = num_to_str(FILE_LIST[i].size as usize);
                    RESULT[pos] = b' '; pos += 1;
                    RESULT[pos] = b'('; pos += 1;
                    for b in size_str.as_bytes() {
                        if pos < 1023 { RESULT[pos] = *b; pos += 1; }
                    }
                    RESULT[pos] = b'b'; pos += 1;
                    RESULT[pos] = b')'; pos += 1;
                }
                
                RESULT[pos] = b'\n'; pos += 1;
            }
        }
        
        RESULT[pos] = 0;
        core::str::from_utf8(&RESULT[..pos]).unwrap_or("")
    }
}

fn change_directory(path: &str) -> &'static str {
    unsafe {
        if path == "/" || path == "" {
            let dir_bytes = b"/mios";
            for i in 0..dir_bytes.len() {
                CURRENT_DIR[i] = dir_bytes[i];
            }
            CURRENT_DIR[dir_bytes.len()] = 0;
            return "Changed to /mios";
        }
        
        if path == ".." {
            return "Already at root";
        }
        
        let mut found = false;
        for i in 0..FILE_COUNT {
            let name = core::str::from_utf8(&FILE_LIST[i].name).unwrap_or("");
            let name_trimmed = name.trim_end_matches('\0');
            if name_trimmed == path && FILE_LIST[i].is_dir {
                found = true;
                break;
            }
        }
        
        if found {
            let bytes = path.as_bytes();
            let len = bytes.len().min(63);
            CURRENT_DIR[..len].copy_from_slice(&bytes[..len]);
            CURRENT_DIR[len] = 0;
            
            static mut MSG: [u8; 64] = [0; 64];
            let msg_bytes = b"Changed to: ";
            let mut pos = 0;
            for b in msg_bytes {
                if pos < 63 { MSG[pos] = *b; pos += 1; }
            }
            for b in path.as_bytes() {
                if pos < 63 { MSG[pos] = *b; pos += 1; }
            }
            MSG[pos] = 0;
            return core::str::from_utf8(&MSG[..pos]).unwrap_or("");
        } else {
            return "Directory not found!";
        }
    }
}

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
        "[SYSTEM BOOT MGR] Initializing VESA framebuffer...",
        "[SYSTEM BOOT MGR] Loading kernel modules...",
        "[SYSTEM BOOT MGR] Initializing IDE controller...",
        "[SYSTEM BOOT MGR] Scanning for ATAPI devices...",
        "[SYSTEM BOOT MGR] Mounting ISO 9660 filesystem...",
        "[SYSTEM BOOT MGR] Starting system services...",
        "[SYSTEM BOOT MGR] Loading terminal interface...",
        "[SYSTEM BOOT MGR] Initializing PS/2 mouse...",
        "[SYSTEM BOOT MGR] Setting up interrupt handlers...",
        "[SYSTEM BOOT MGR] Loading user database...",
        "[SYSTEM BOOT MGR] Starting shell...",
        "[SYSTEM BOOT MGR] config load...",
        "[SYSTEM BOOT MGR] load lib",
        "[SYSTEM BOOT MGR] /mios/usr",
        "[SYSTEM BOOT MGR] load RECOV",
        "[SYSTEM BOOT MGR] load sys backup [/mios :  SYS1.000000001]",
        "[SYSTEM BOOT MGR] load sys backup [/mios :  SYS1.000000001]",
        "[SYSTEM BOOT MGR] load sys backup [/mios :  SYS1.000000002]",
        "[SYSTEM BOOT MGR] i386 cheking...",
        "[SYSTEM BOOT MGR] ATAPI...",
        "[SYSTEM BOOT MGR] hostname check",
        "[SYSTEM BOOT MGR] sh...",
        "[SYSTEM BOOT MGR] secure load",
        "[SYSTEM BOOT MGR] System ready.",
        "[SYSTEM BOOT MGR] Welcome to system!",
    ];
    
    let start_y = 80;
    for (i, msg) in boot_msgs.iter().enumerate() {
        let y = start_y + i * 20;
        gfx.draw_text(20, y, msg, Color::rgb(200, 200, 200).to_u32(), Color::BLACK.to_u32());
        gfx.flush();
        
        for _ in 0..2000000 {
            unsafe { core::arch::asm!("nop", options(nostack)); }
        }
    }
    
    for _ in 0..10000000 {
        unsafe { core::arch::asm!("nop", options(nostack)); }
    }
}

fn draw_login_screen(gfx: &mut Graphics) {
    let (fg, bg) = get_config_colors();
    gfx.clear(bg);
    
    let users_list = list_users();
    let mut y = 40;
    for line in users_list.split('\n') {
        if !line.is_empty() {
            gfx.draw_text(20, y, line, fg, bg);
            y += 20;
        }
    }
    
    let prompt_y = y + 20;
    gfx.draw_text(20, prompt_y, "/mios/users:~# ", Color::rgb(104, 104, 104).to_u32(), bg);
    gfx.flush();
}

fn draw_dos_terminal(gfx: &mut Graphics) {
    let screen_w = gfx.width;
    let screen_h = gfx.height;
    
    let (fg, bg) = get_config_colors();
    gfx.clear(bg);
    
    unsafe {
        if TED_ACTIVE {
            draw_ted_editor(gfx);
            return;
        }
        
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
                gfx.draw_text(0, y_pos, display_line, Color::rgb(148, 148, 148).to_u32(), bg);
            } else if line.starts_with("Error") || line.starts_with("Unknown") {
                gfx.draw_text(0, y_pos, display_line, Color::rgb(129, 129, 129).to_u32(), bg);
            } else {
                gfx.draw_text(0, y_pos, display_line, fg, bg);
            }
        }
        
        let prompt_y = (hist_len - start_row) * 16;
        let host = core::str::from_utf8(&HOSTNAME).unwrap_or("mios-kernel");
        let host_trimmed = host.trim_end_matches('\0');
        let prompt_str = format_prompt(host_trimmed);
        gfx.draw_text(0, prompt_y, prompt_str, Color::rgb(104, 104, 104).to_u32(), bg);
        
        let prompt_len = prompt_str.len();
        if TERM_LEN > 0 {
            let input = core::str::from_utf8(&TERM_INPUT[..TERM_LEN]).unwrap_or("");
            let max_input = screen_w / 8 - prompt_len - 1;
            let display_input = if input.len() > max_input {
                &input[input.len() - max_input..]
            } else {
                input
            };
            gfx.draw_text(prompt_len * 8, prompt_y, display_input, fg, bg);
        }
    }
}

fn format_prompt(hostname: &str) -> &'static str {
    static mut BUF: [u8; 64] = [0; 64];
    unsafe {
        let mut pos = 0;
        for b in hostname.as_bytes() {
            if pos < 63 { BUF[pos] = *b; pos += 1; }
        }
        BUF[pos] = b':'; pos += 1;
        BUF[pos] = b'~'; pos += 1;
        BUF[pos] = b'#'; pos += 1;
        BUF[pos] = b' '; pos += 1;
        BUF[pos] = 0;
        core::str::from_utf8(&BUF[..pos]).unwrap_or("mios-kernel:~# ")
    }
}


fn draw_ted_editor(gfx: &mut Graphics) {
    let screen_w = gfx.width;
    let screen_h = gfx.height;
    let (fg, bg) = get_config_colors();
    
    unsafe {
        let border_color = Color::rgb(0, 150, 255).to_u32();
        let border_bg = bg;
        let text_color = fg;
        let text_bg = bg;
        
        let margin = 4;
        let right = screen_w - margin;
        let bottom = screen_h - margin;
        
        draw_corner_char(gfx, margin, margin, border_color, border_bg, 0);
        for x in (margin + 8)..(right - 8) {
            draw_corner_char(gfx, x, margin, border_color, border_bg, 4);
        }
        draw_corner_char(gfx, right - 8, margin, border_color, border_bg, 1);
        
        for y in (margin + 16)..(bottom - 16) {
            draw_corner_char(gfx, margin, y, border_color, border_bg, 5);
            draw_corner_char(gfx, right - 8, y, border_color, border_bg, 5);
        }
        
        draw_corner_char(gfx, margin, bottom - 16, border_color, border_bg, 2);
        for x in (margin + 8)..(right - 8) {
            draw_corner_char(gfx, x, bottom - 16, border_color, border_bg, 4);
        }
        draw_corner_char(gfx, right - 8, bottom - 16, border_color, border_bg, 3);
        
        let start_x = margin + 12;
        let start_y = margin + 16;
        let max_lines = (bottom - start_y - 30) / 16;
        
        for i in 0..max_lines {
            if i < TED_LINE {
                let line = core::str::from_utf8(&TED_TEXT[i]).unwrap_or("");
                let max_chars = (right - start_x - 16) / 8;
                let display_line = if line.len() > max_chars {
                    &line[..max_chars]
                } else {
                    line
                };
                let y_pos = start_y + i * 16;
                gfx.draw_text(start_x, y_pos, display_line, text_color, text_bg);
            }
        }
        
        let info_y = bottom - 24;
        let info_line = "ted (text editor) version 1.0.0.0 By Mikhail";
        gfx.draw_text(start_x, info_y, info_line, Color::rgb(100, 200, 100).to_u32(), text_bg);
        
        let cursor_x = start_x + TED_COL * 8;
        let cursor_y = start_y + TED_ROW * 16;
        gfx.fill_rect(cursor_x, cursor_y, 1, 16, Color::WHITE.to_u32());
    }
}


fn ted_command(args: &str) -> &'static str {
    unsafe {
        if args == "--start" || args == "" {
            TED_ACTIVE = true;
            TED_ROW = 0;
            TED_COL = 0;
            TED_LINE = 0;
            for i in 0..100 {
                for j in 0..80 {
                    TED_TEXT[i][j] = b' ';
                }
            }
            let welcome = "Welcome to TED text editor!";
            for (i, c) in welcome.as_bytes().iter().enumerate() {
                if i < 80 {
                    TED_TEXT[0][i] = *c;
                }
            }
            TED_LINE = 1;
            return "TED started. Type text to edit.";
        } else if args == "--stop" || args == "/stop" {
            TED_ACTIVE = false;
            return "TED closed.";
        }
        "Usage: /ted --start or /ted --stop"
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
                gfx.draw_text(cx + 10, y_pos, display_line, Color::rgb(148, 148, 148).to_u32(), Color::BLACK.to_u32());
            } else if line.starts_with("Error") || line.starts_with("Unknown") {
                gfx.draw_text(cx + 10, y_pos, display_line, Color::rgb(110, 109, 109).to_u32(), Color::BLACK.to_u32());
            } else {
                gfx.draw_text(cx + 10, y_pos, display_line, Color::WHITE.to_u32(), Color::BLACK.to_u32());
            }
        }
        
        let prompt_y = cy + 10 + (hist_len - start_row) * 16;
        let host = core::str::from_utf8(&HOSTNAME).unwrap_or("mios-kernel");
        let host_trimmed = host.trim_end_matches('\0');
        let prompt_str = format_prompt(host_trimmed);
        gfx.draw_text(cx + 10, prompt_y, prompt_str, Color::rgb(133, 133, 133).to_u32(), Color::BLACK.to_u32());
        
        if TERM_LEN > 0 {
            let input = core::str::from_utf8(&TERM_INPUT[..TERM_LEN]).unwrap_or("");
            gfx.draw_text(cx + 10 + prompt_str.len() * 8, prompt_y, input, Color::WHITE.to_u32(), Color::BLACK.to_u32());
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
            gfx.fill_rect(0, taskbar_y, screen_w, 30, Color::rgb(0, 100, 200).to_u32());
            
            let btn_x = 10;
            let btn_y = taskbar_y + 2;
            gfx.fill_rect(btn_x, btn_y, 100, 26, Color::rgb(104, 104, 104).to_u32());
            gfx.draw_rect_border(btn_x, btn_y, 100, 26, Color::rgb(120, 120, 120).to_u32());
            gfx.draw_text(btn_x + 10, btn_y + 6, "Terminal", Color::WHITE.to_u32(), Color::rgb(19, 19, 19).to_u32());
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
    return "MiOS Kernel Commands:
  tech          - System info
  ver           - Kernel version
  mem           - Memory info
  uptime        - System uptime
  about         - About MiOS
  kernel        - Kernel details
  devs          - Developers
  dir, ls       - List directory
  cd <dir>      - Change directory
  pwd           - Show current dir
  cd-inf        - Directory info
  gui /start    - Start GUI
  gui /stop     - Stop GUI
  hostname      - Show hostname
  hostname edit - Change hostname
  cls, clear    - Clear terminal
  reboot        - Reboot system
  shutdown      - Shutdown system
  task          - View task keys
  /ted          - Start TED editor
  /ted --stop   - Stop TED editor
  /acc --info   - Show all users
  /acc --new    - Create temp user
  /acc --logout - Logout current user
  /config --color 'def/red/blue/green'
  /config --mouse/on
  /config --mouse/off
  /config --hostname 'new name'
  /config --cfg/def
  sys /rf stop  - Kernel panic
  dice          - Roll a dice";
}
        "/config" => {
            if part_count >= 2 {
                match parts[1] {
                    "--color" => {
                        if part_count >= 3 {
                            let color = parts[2];
                            unsafe {
                                match color {
                                    "def" => {
                                        CONFIG_COLOR = 0;
                                        return "Color set to default (white on black)";
                                    }
                                    "red" => {
                                        CONFIG_COLOR = 1;
                                        return "Color set to red (white on red)";
                                    }
                                    "blue" => {
                                        CONFIG_COLOR = 2;
                                        return "Color set to blue (white on blue)";
                                    }
                                    "green" => {
                                        CONFIG_COLOR = 3;
                                        return "Color set to green (green on black)";
                                    }
                                    _ => return "Invalid color! Use: def, red, blue, green",
                                }
                            }
                        }
                        return "***STOP KEY! Usage: /config --color 'def/red/blue/green'";
                    }
                    "--mouse/on" => {
                        unsafe {
                            CONFIG_MOUSE_VISIBLE = true;
                            return "Mouse cursor enabled in DOS mode!";
                        }
                    }
                    "--mouse/off" => {
                        unsafe {
                            CONFIG_MOUSE_VISIBLE = false;
                            return "Mouse cursor disabled in DOS mode!";
                        }
                    }
                    "--hostname" => {
                        if part_count >= 3 {
                            let new_name = parts[2];
                            unsafe {
                                let bytes = new_name.as_bytes();
                                let len = bytes.len().min(63);
                                HOSTNAME[..len].copy_from_slice(&bytes[..len]);
                                HOSTNAME[len] = 0;
                                return "Hostname changed successfully!";
                            }
                        }
                        return "Usage: /config --hostname 'new name'";
                    }
                    "--cfg/def" => {
                        unsafe {
                            if is_config_default() {
                                return "All settings are already default!";
                            }
                            set_config_defaults();
                            return "All settings reset to default!";
                        }
                    }
                    _ => return "Usage: /config --color --mouse/on --mouse/off --hostname --cfg/def",
                }
            }
            return "Usage: /config --color --mouse/on --mouse/off --hostname --cfg/def";
        }
        "sys" => {
            if part_count >= 3 && parts[1] == "/rf" && parts[2] == "stop" {
                unsafe {
                    use recov;
                    let vesa_info = VESA_INFO.as_ref().unwrap();
                    let mut panic_gfx = Graphics::new(
                        vesa_info.addr as u64,
                        vesa_info.width as u32,
                        vesa_info.height as u32,
                        vesa_info.pitch as u32
                    );
                    recov::trigger_panic_with_code(&mut panic_gfx, 0x2c0000);
                    loop {
                        recov::draw_panic_screen(&mut panic_gfx);
                        for _ in 0..1000000 {
                            unsafe { core::arch::asm!("nop", options(nostack)); }
                        }
                    }
                }
            }
            return "Usage: sys /rf stop";
        }
        "tech" => {
            return "MiOS Kernel\nArchitecture: x64\nVESA Mode: 800x600\nTerminal: VESA-based\nBuild Date: 2026";
        }
        "cfg /hlp" => {
            return "Config keys:\n --color 'def/red/blue/green'\n --mouse/on\n --mouse/off\n --hostname 'new name'\n --cfg/def";
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
        "/ted" => {
            if part_count >= 2 {
                return ted_command(parts[1]);
            }
            return ted_command("");
        }
        "/acc" => {
            if part_count >= 2 {
                match parts[1] {
                    "--info" => {
                        return list_users();
                    }
                    "--new" => {
                        if is_current_user_main() {
                            match create_temp_user() {
                                Ok(_id) => {
                                    return "Temporary user created! ID: 4356";
                                }
                                Err(e) => return e,
                            }
                        } else {
                            return "Permission denied! Only Main User can create temporary users.";
                        }
                    }
                    "--logout" => {
                        if logout() {
                            return "Logged out successfully!";
                        } else {
                            return "No user logged in!";
                        }
                    }
                    _ => return "Usage: /acc --info | --new | --logout",
                }
            }
            return "Usage: /acc --info | --new | --logout";
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
        "task" => {
        return "task keys:
        ===============================
        set keys:             settings:
        lib.rs                kernel
        mouse.rs              mouse
        keyboard.rs           keyboard";
        }
        "task key" => {
        return "task keys:
        ===============================
        set keys:             settings:
        lib.rs                kernel
        mouse.rs              mouse
        keyboard.rs           keyboard";
        }
        "uptime" => {
            unsafe {
                let seconds = BOOT_TICKS / 100;
                let minutes = seconds / 60;
                let hours = minutes / 60;
                let days = hours / 24;
                
                static mut BUF: [u8; 64] = [0; 64];
                let mut pos = 0;
                if days > 0 {
                    let days_str = num_to_str(days);
                    for b in days_str.as_bytes() { BUF[pos] = *b; pos += 1; }
                    BUF[pos] = b'd'; pos += 1;
                    BUF[pos] = b' '; pos += 1;
                }
                let hours_rem = hours % 24;
                let hours_str = num_to_str(hours_rem);
                for b in hours_str.as_bytes() { BUF[pos] = *b; pos += 1; }
                BUF[pos] = b'h'; pos += 1;
                BUF[pos] = b' '; pos += 1;
                let minutes_rem = minutes % 60;
                let minutes_str = num_to_str(minutes_rem);
                for b in minutes_str.as_bytes() { BUF[pos] = *b; pos += 1; }
                BUF[pos] = b'm'; pos += 1;
                BUF[pos] = 0;
                return core::str::from_utf8(&BUF[..pos]).unwrap_or("");
            }
        }
        "about" => {
            return "MiOS  - Free Kernel\nCreated in 2026\nBy MDEVS";
        }
        "pwd" => {
            unsafe {
                let dir = core::str::from_utf8(&CURRENT_DIR).unwrap_or("/mios/");
                return dir.trim_end_matches('\0');
            }
        }
        "cd" => {
            if part_count >= 2 {
                return change_directory(parts[1]);
            }
            unsafe {
                let dir_bytes = b"/mios";
                for i in 0..dir_bytes.len() {
                    CURRENT_DIR[i] = dir_bytes[i];
                }
                CURRENT_DIR[dir_bytes.len()] = 0;
                return "Changed to /mios";
            }
        }
        "cd-inf" | "cdinfo" => {
            unsafe {
                let dir = core::str::from_utf8(&CURRENT_DIR).unwrap_or("/mios");
                let dir_trimmed = dir.trim_end_matches('\0');
                let file_count = FILE_COUNT;
                let cd_status = if atapi::CD_PRESENT { 
                    atapi::get_cd_info()
                } else { 
                    "None (using memory image)"
                };
                static mut MSG: [u8; 128] = [0; 128];
                let msg1 = b"Current directory: ";
                let mut pos = 0;
                for b in msg1 {
                    if pos < 127 { MSG[pos] = *b; pos += 1; }
                }
                for b in dir_trimmed.as_bytes() {
                    if pos < 127 { MSG[pos] = *b; pos += 1; }
                }
                MSG[pos] = b'\n'; pos += 1;
                let msg2 = b"CD-ROM: ";
                for b in msg2 {
                    if pos < 127 { MSG[pos] = *b; pos += 1; }
                }
                for b in cd_status.as_bytes() {
                    if pos < 127 { MSG[pos] = *b; pos += 1; }
                }
                MSG[pos] = b'\n'; pos += 1;
                let msg3 = b"Files: ";
                for b in msg3 {
                    if pos < 127 { MSG[pos] = *b; pos += 1; }
                }
                let count_str = num_to_str(file_count);
                for b in count_str.as_bytes() {
                    if pos < 127 { MSG[pos] = *b; pos += 1; }
                }
                MSG[pos] = 0;
                return core::str::from_utf8(&MSG[..pos]).unwrap_or("");
            }
        }
        "dir" | "ls" => {
            unsafe {
                let dir = core::str::from_utf8(&CURRENT_DIR).unwrap_or("/mios");
                return list_directory(dir.trim_end_matches('\0'));
            }
        }
        "hostname" => {
            if part_count >= 3 && parts[1] == "edit" {
                unsafe {
                    let name = parts[2];
                    let bytes = name.as_bytes();
                    let len = bytes.len().min(63);
                    HOSTNAME[..len].copy_from_slice(&bytes[..len]);
                    HOSTNAME[len] = 0;
                    static mut MSG: [u8; 64] = [0; 64];
                    let msg_bytes = b"Hostname changed to: ";
                    let mut pos = 0;
                    for b in msg_bytes {
                        if pos < 63 { MSG[pos] = *b; pos += 1; }
                    }
                    for b in name.as_bytes() {
                        if pos < 63 { MSG[pos] = *b; pos += 1; }
                    }
                    MSG[pos] = 0;
                    return core::str::from_utf8(&MSG[..pos]).unwrap_or("");
                }
            }
            unsafe {
                let host = core::str::from_utf8(&HOSTNAME).unwrap_or("mios-kernel");
                return host.trim_end_matches('\0');
            }
        }
        "kernel" => {
            return "MiOS Kernel \nFilesystem: ISO 9660\nCD-ROM: ATAPI/IDE (Real hardware access)\nBootloader: Multiboot GNU/GRUB";
        }
        "devs" => {
            return "MiOS Developers: MDEVS Team";
        }
        "rus [set]" => {
            return "Привет! Иди на х^й";
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
            
            unsafe {
                atapi::init_cdrom();
                init_filesystem();
                init_users();
                set_config_defaults();
            }
            
            clear_history();

            unsafe {
                if atapi::CD_PRESENT {
                    let info = atapi::get_cd_info();
                    
                }
            }

            let mut cursor_x = 400usize;
            let mut cursor_y = 300usize;
            let cursor_size = 16;
            let mut cursor_save = [[0u32; 16]; 16];
            let mut left_prev = false;
            let mut mouse_initialized = false;

           
            loop {
                
                if recov::is_panic_active() {
                    if let Some(_sc) = mouse::get_key() {
                        for _ in 0..3 {
                            unsafe { core::arch::asm!("mov al, 0xFE", "out 0x64, al", options(nostack)); }
                        }
                    }
                    for _ in 0..10000 {
                        unsafe { core::arch::asm!("nop", options(nostack)); }
                    }
                    continue;
                }

                let mut need_flush = false;
                let mut mouse_redraw = false;

                unsafe { BOOT_TICKS += 1; }

                
                unsafe {
                    if NEED_LOGIN {
                        draw_login_screen(&mut gfx);
                        
                        let mut input: [u8; 16] = [0; 16];
                        let mut len = 0;
                        let mut enter_pressed = false;
                        
                        let users_lines = list_users().split('\n').count();
                        let prompt_y = 40 + (users_lines * 20) + 20;
                        
                        while !enter_pressed {
                            if let Some(sc) = mouse::get_key() {
                                if sc & 0x80 == 0 {
                                    let c = match sc {
                                        0x1C => Some('\n'),
                                        0x0E => Some('\x08'),
                                        0x02 => Some('1'), 0x03 => Some('2'), 0x04 => Some('3'),
                                        0x05 => Some('4'), 0x06 => Some('5'), 0x07 => Some('6'),
                                        0x08 => Some('7'), 0x09 => Some('8'), 0x0A => Some('9'),
                                        0x0B => Some('0'),
                                        _ => None,
                                    };
                                    
                                    if let Some(ch) = c {
                                        if ch == '\n' {
                                            enter_pressed = true;
                                            break;
                                        } else if ch == '\x08' && len > 0 {
                                            len -= 1;
                                            draw_login_screen(&mut gfx);
                                            let input_str = core::str::from_utf8(&input[..len]).unwrap_or("");
                                            gfx.draw_text(20 + 16 * 8, prompt_y, input_str, Color::WHITE.to_u32(), Color::BLACK.to_u32());
                                            gfx.flush();
                                        } else if len < 15 && ch.is_ascii_digit() {
                                            input[len] = ch as u8;
                                            len += 1;
                                            let input_str = core::str::from_utf8(&input[..len]).unwrap_or("");
                                            gfx.draw_text(20 + 16 * 8, prompt_y, input_str, Color::WHITE.to_u32(), Color::BLACK.to_u32());
                                            gfx.flush();
                                        }
                                    }
                                }
                            }
                            for _ in 0..1000 {
                                unsafe { core::arch::asm!("nop", options(nostack)); }
                            }
                        }
                        
                        let input_str = core::str::from_utf8(&input[..len]).unwrap_or("");
                        if let Ok(id) = input_str.parse::<u32>() {
                            if login(id) {
                                unsafe {
                                    push_history("Dos-Shell-sys 1.0.0.0");
                                    push_history("Welcome to KRNL 6.0.0");
                                    push_history("");
                                    push_history("# Official Site: https://mios-system.vercel.app");
                                    push_history("# Support: https://mios-system.vercel.app/faq_main");
                                    push_history("# Update center: https://mios-system.vercel.app/dowl_mgr");
                                    push_history("");
                                    push_history("");
                                    push_history("0 configurations, bootloader GNU GRUB");
                                    push_history("Type 'help' to get a list of all available commands.");
                                    push_history("");
                                    NEED_LOGIN = false;
                                    gfx.clear(Color::BLACK.to_u32());
                                    draw_dos_terminal(&mut gfx);
                                    gfx.flush();
                                }
                            } else {
                                unsafe { 
                                    push_history("[ERROR] Invalid user ID!");
                                    for _ in 0..5000000 {
                                        core::arch::asm!("nop", options(nostack));
                                    }
                                }
                            }
                        } else {
                            unsafe { 
                                push_history("[ERROR] Invalid input!");
                                for _ in 0..5000000 {
                                    core::arch::asm!("nop", options(nostack));
                                }
                            }
                        }
                        continue;
                    }
                }


                if let Some(sc) = mouse::get_key() {
                    unsafe {
                        if !IS_LOGGED_IN {
                            continue;
                        }
                    }
                    
                    unsafe {
                        if sc & 0x80 == 0 {
                            if TED_ACTIVE {
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
                                        TED_ROW += 1;
                                        TED_COL = 0;
                                        if TED_ROW >= 100 {
                                            TED_ROW = 99;
                                        }
                                        if TED_ROW >= TED_LINE {
                                            TED_LINE = TED_ROW + 1;
                                        }
                                        need_flush = true;
                                    } else if ch == '\x08' {
                                        if TED_COL > 0 {
                                            TED_COL -= 1;
                                            for i in TED_COL..80 {
                                                if i + 1 < 80 {
                                                    TED_TEXT[TED_ROW][i] = TED_TEXT[TED_ROW][i + 1];
                                                } else {
                                                    TED_TEXT[TED_ROW][i] = b' ';
                                                }
                                            }
                                            need_flush = true;
                                        } else if TED_ROW > 0 {
                                            let prev_len = TED_TEXT[TED_ROW - 1].iter().position(|&c| c == 0 || c == b' ').unwrap_or(80);
                                            let curr_len = TED_TEXT[TED_ROW].iter().position(|&c| c == 0 || c == b' ').unwrap_or(80);
                                            let total_len = prev_len + curr_len;
                                            if total_len < 80 {
                                                for i in prev_len..(prev_len + curr_len) {
                                                    if i - prev_len < curr_len {
                                                        TED_TEXT[TED_ROW - 1][i] = TED_TEXT[TED_ROW][i - prev_len];
                                                    }
                                                }
                                                TED_ROW -= 1;
                                                TED_COL = prev_len;
                                                for i in (TED_ROW + 1)..TED_LINE {
                                                    if i + 1 < 100 {
                                                        TED_TEXT[i] = TED_TEXT[i + 1];
                                                    }
                                                }
                                                TED_LINE -= 1;
                                                need_flush = true;
                                            }
                                        }
                                    } else {
                                        if TED_COL < 79 && TED_ROW < 100 {
                                            for i in (TED_COL..79).rev() {
                                                TED_TEXT[TED_ROW][i + 1] = TED_TEXT[TED_ROW][i];
                                            }
                                            TED_TEXT[TED_ROW][TED_COL] = ch as u8;
                                            TED_COL += 1;
                                            if TED_ROW >= TED_LINE {
                                                TED_LINE = TED_ROW + 1;
                                            }
                                            need_flush = true;
                                        }
                                    }
                                }
                            } else {
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
                                            let host = core::str::from_utf8(&HOSTNAME).unwrap_or("mios-kernel");
                                            let host_trimmed = host.trim_end_matches('\0');
                                            let prompt = format_prompt(host_trimmed);
                                            push_history(prompt);
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
                        

                        if CONFIG_MOUSE_VISIBLE && !GUI_ACTIVE {
                            draw_mouse_cursor(&mut gfx, cursor_x, cursor_y);
                            mouse_redraw = true;
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

                            if CONFIG_MOUSE_VISIBLE {
                                draw_mouse_cursor(&mut gfx, cursor_x, cursor_y);
                            }
                        }
                        gfx.flush();
                    }
                }

                if mouse_redraw && !need_flush {
                    gfx.flush();
                }

                for _ in 0..1000 {
                    unsafe { core::arch::asm!("nop", options(nostack)); }
                }
            }
        }
    }

    loop {
        unsafe { core::arch::asm!("hlt", options(nostack)); }
    }
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}