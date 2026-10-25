
use core::arch::naked_asm;

const MOUSE_DATA: u16 = 0x60;
const MOUSE_STATUS: u16 = 0x64;
const MOUSE_CMD: u16 = 0x64;

const PIC1_CMD: u16 = 0x20;
const PIC1_DATA: u16 = 0x21;
const PIC2_CMD: u16 = 0xA0;
const PIC2_DATA: u16 = 0xA1;
const PIC_EOI: u8 = 0x20;




#[derive(Debug, Clone, Copy)]
pub struct MousePacket {
    pub dx: i32,
    pub dy: i32,
    pub left: bool,
    pub right: bool,
    pub middle: bool,
    pub wheel: i32,
}

fn inb(port: u16) -> u8 {
    let v: u8;
    unsafe { core::arch::asm!("in al, dx", in("dx") port, out("al") v) }
    v
}

fn outb(port: u16, val: u8) {
    unsafe { core::arch::asm!("out dx, al", in("dx") port, in("al") val) }
}

fn mouse_wait(read: bool) {
    let timeout = 100_000;
    if read {
        for _ in 0..timeout {
            if inb(MOUSE_STATUS) & 1 == 1 { return; }
        }
    } else {
        for _ in 0..timeout {
            if inb(MOUSE_STATUS) & 2 == 0 { return; }
        }
    }
}

fn mouse_write(cmd: u8) {
    mouse_wait(false);
    outb(MOUSE_CMD, 0xD4);
    mouse_wait(false);
    outb(MOUSE_DATA, cmd);
}

fn mouse_read() -> u8 {
    mouse_wait(true);
    inb(MOUSE_DATA)
}

pub fn init_ps2_mouse() {
    
    mouse_wait(false);
    outb(MOUSE_CMD, 0xA8);

    
    mouse_wait(false);
    outb(MOUSE_CMD, 0x20);
    let mut config = mouse_read();

    
    config |= 0x03;  
    config &= !0x20; 
    config &= !0x10; 


    mouse_wait(false);
    outb(MOUSE_CMD, 0x60);
    mouse_wait(false);
    outb(MOUSE_DATA, config);


    mouse_write(0xFF);
    let ack = mouse_read();
    
    if ack == 0xFA {
        mouse_read(); 
        let _id = mouse_read();
        for rate in [200u8, 100, 80] {
            mouse_write(0xF3);
            mouse_read();
            mouse_write(rate);
            mouse_read();
        }
        mouse_write(0xF2);
        mouse_read();
        let id = mouse_read();
        unsafe { PKT_SIZE = if id == 3 || id == 4 { 4 } else { 3 }; }
        mouse_write(0xF3);
        mouse_read();
        mouse_write(100);
        mouse_read();
    }


    mouse_write(0xF4);
    mouse_read(); 
}


static mut MS_RING: [u8; 256] = [0; 256];
static mut MS_HEAD: u8 = 0;
static mut MS_TAIL: u8 = 0;
static mut KB_RING: [u8; 256] = [0; 256];
static mut KB_HEAD: u8 = 0;
static mut KB_TAIL: u8 = 0;

#[unsafe(naked)]
extern "x86-interrupt" fn mouse_handler(_sf: &crate::idt::InterruptStackFrame, _error_code: u64) {
    naked_asm!(
        "push rax",
        "push rcx",
        "push rdx",
        "mov dx, 0x64",
        "in al, dx",
        "test al, 0x01",
        "jz 2f",
        "mov dx, 0x60",
        "in al, dx",
        "movzx ecx, byte ptr [rip + {head}]",
        "lea rdx, [rip + {ring}]",
        "mov [rdx + rcx], al",
        "inc cl",
        "mov byte ptr [rip + {head}], cl",
        "2:",
        "mov al, 0x20",
        "out 0xA0, al",
        "out 0x20, al",
        "pop rdx",
        "pop rcx",
        "pop rax",
        "iretq",
        ring = sym MS_RING,
        head = sym MS_HEAD,
    );
}

#[unsafe(naked)]
extern "x86-interrupt" fn keyboard_handler(_sf: &crate::idt::InterruptStackFrame, _error_code: u64) {
    naked_asm!(
        "push rax",
        "push rcx",
        "push rdx",
        "mov dx, 0x60",
        "in al, dx",
        "movzx ecx, byte ptr [rip + {head}]",
        "lea rdx, [rip + {ring}]",
        "mov [rdx + rcx], al",
        "inc cl",
        "mov byte ptr [rip + {head}], cl",
        "mov al, 0x20",
        "out 0x20, al",
        "pop rdx",
        "pop rcx",
        "pop rax",
        "iretq",
        ring = sym KB_RING,
        head = sym KB_HEAD,
    );
}

pub unsafe fn init_mouse_interrupts() {
    
    outb(PIC1_CMD, 0x11);
    outb(PIC1_DATA, 0x20);  
    outb(PIC1_DATA, 0x04);  
    outb(PIC1_DATA, 0x01);

    outb(PIC2_CMD, 0x11);
    outb(PIC2_DATA, 0x28);  
    outb(PIC2_DATA, 0x02);
    outb(PIC2_DATA, 0x01);

    
    outb(PIC1_DATA, 0xF9);  
    
    outb(PIC2_DATA, 0xEF);  

    crate::idt::set_handler(0x21, keyboard_handler); 
    crate::idt::set_handler(0x2C, mouse_handler);    

    core::arch::asm!("sti");
}

static mut PKT_SIZE: usize = 3;
static mut PKT: [u8; 4] = [0; 4];
static mut PKT_IDX: usize = 0;
static mut PENDING: Option<MousePacket> = None;
static mut SHIFT: bool = false;
static mut ALT: bool = false;
static mut LAYOUT_CHANGED: bool = false;
static mut EXT: bool = false;
pub static mut LAST_EXT_KEY: u8 = 0;

fn ring_pop(ring: *const [u8; 256], head: *const u8, tail: *mut u8) -> Option<u8> {
    unsafe {
        let h = core::ptr::read_volatile(head);
        let t = core::ptr::read_volatile(tail);
        if h == t { return None; }
        let b = core::ptr::read_volatile(&(*ring)[t as usize]);
        core::ptr::write_volatile(tail, t.wrapping_add(1));
        Some(b)
    }
}

fn next_raw_packet() -> Option<MousePacket> {
    unsafe {
        loop {
            let b = ring_pop(&raw const MS_RING, &raw const MS_HEAD, &raw mut MS_TAIL)?;
            if PKT_IDX == 0 && b & 0x08 == 0 { continue; }
            PKT[PKT_IDX] = b;
            PKT_IDX += 1;
            if PKT_IDX < PKT_SIZE { continue; }
            PKT_IDX = 0;
            let b0 = PKT[0];
            if b0 & 0xC0 != 0 { continue; }
            let mut dx = PKT[1] as i32;
            let mut dy = PKT[2] as i32;
            if b0 & 0x10 != 0 { dx -= 256; }
            if b0 & 0x20 != 0 { dy -= 256; }
            let wheel = if PKT_SIZE == 4 { ((PKT[3] & 0x0F) as i8).wrapping_shl(4) as i32 >> 4 } else { 0 };
            return Some(MousePacket {
                dx, dy: -dy,
                left: b0 & 0x01 != 0, right: b0 & 0x02 != 0, middle: b0 & 0x04 != 0,
                wheel,
            });
        }
    }
}

pub fn get_mouse_packet() -> Option<MousePacket> {
    unsafe {
        let mut acc = match PENDING.take() { Some(p) => p, None => next_raw_packet()? };
        while let Some(p) = next_raw_packet() {
            if p.left == acc.left && p.right == acc.right && p.middle == acc.middle {
                acc.dx += p.dx;
                acc.dy += p.dy;
                acc.wheel += p.wheel;
            } else {
                PENDING = Some(p);
                break;
            }
        }
        Some(acc)
    }
}

pub fn get_key() -> Option<u8> {
    unsafe {
        loop {
            let sc = ring_pop(&raw const KB_RING, &raw const KB_HEAD, &raw mut KB_TAIL)?;
            if sc == 0xE0 { EXT = true; continue; }
            if EXT {
                EXT = false;
                if sc & 0x80 == 0 { LAST_EXT_KEY = sc; }
                if sc == 0x38 { alt_down(); } else if sc == 0xB8 { ALT = false; }
                return Some(if sc & 0x80 == 0 { 0x00 } else { 0x80 });
            }
            match sc {
                0x2A | 0x36 => {
                    let was = SHIFT;
                    SHIFT = true;
                    if !was && ALT { switch_layout(); }
                }
                0xAA | 0xB6 => SHIFT = false,
                0x38 => alt_down(),
                0xB8 => ALT = false,
                _ => {}
            }
            return Some(sc);
        }
    }
}

pub fn shift_pressed() -> bool { unsafe { SHIFT } }

unsafe fn switch_layout() {
    crate::web::keys::toggle_layout();
    LAYOUT_CHANGED = true;
}

unsafe fn alt_down() {
    let was = ALT;
    ALT = true;
    if !was && SHIFT { switch_layout(); }
}

pub fn take_layout_changed() -> bool {
    unsafe { let c = LAYOUT_CHANGED; LAYOUT_CHANGED = false; c }
}

pub fn apply_shift(c: char) -> char {
    if !shift_pressed() { return c; }
    match c {
        'a'..='z' => ((c as u8) - 32) as char,
        '1' => '!', '2' => '@', '3' => '#', '4' => '$', '5' => '%',
        '6' => '^', '7' => '&', '8' => '*', '9' => '(', '0' => ')',
        '-' => '_', '=' => '+', '[' => '{', ']' => '}', ';' => ':',
        '\'' => '"', '`' => '~', '\\' => '|', ',' => '<', '.' => '>', '/' => '?',
        _ => c,
    }
}
