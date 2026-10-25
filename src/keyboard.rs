use core::arch::asm;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Enter,
    Backspace,
    Escape,
    Up,
    Down,
    Left,
    Right,
    Tab,
    None,
}

impl Key {
    pub fn from_scancode(scancode: u8, shift: bool, caps: bool) -> Self {
        
        let is_upper = shift || caps;
        
        match scancode {
           
            0x01 => Key::Escape,
            0x0E => Key::Backspace,
            0x1C => Key::Enter,
            0x0F => Key::Tab,
            
            
            0x48 => Key::Up,
            0x50 => Key::Down,
            0x4B => Key::Left,
            0x4D => Key::Right,
            
            
            0x39 => Key::Char(' '),
            
            
            0x10 => if is_upper { Key::Char('Q') } else { Key::Char('q') },
            0x11 => if is_upper { Key::Char('W') } else { Key::Char('w') },
            0x12 => if is_upper { Key::Char('E') } else { Key::Char('e') },
            0x13 => if is_upper { Key::Char('R') } else { Key::Char('r') },
            0x14 => if is_upper { Key::Char('T') } else { Key::Char('t') },
            0x15 => if is_upper { Key::Char('Y') } else { Key::Char('y') },
            0x16 => if is_upper { Key::Char('U') } else { Key::Char('u') },
            0x17 => if is_upper { Key::Char('I') } else { Key::Char('i') },
            0x18 => if is_upper { Key::Char('O') } else { Key::Char('o') },
            0x19 => if is_upper { Key::Char('P') } else { Key::Char('p') },
            0x1E => if is_upper { Key::Char('A') } else { Key::Char('a') },
            0x1F => if is_upper { Key::Char('S') } else { Key::Char('s') },
            0x20 => if is_upper { Key::Char('D') } else { Key::Char('d') },
            0x21 => if is_upper { Key::Char('F') } else { Key::Char('f') },
            0x22 => if is_upper { Key::Char('G') } else { Key::Char('g') },
            0x23 => if is_upper { Key::Char('H') } else { Key::Char('h') },
            0x24 => if is_upper { Key::Char('J') } else { Key::Char('j') },
            0x25 => if is_upper { Key::Char('K') } else { Key::Char('k') },
            0x26 => if is_upper { Key::Char('L') } else { Key::Char('l') },
            0x2C => if is_upper { Key::Char('Z') } else { Key::Char('z') },
            0x2D => if is_upper { Key::Char('X') } else { Key::Char('x') },
            0x2E => if is_upper { Key::Char('C') } else { Key::Char('c') },
            0x2F => if is_upper { Key::Char('V') } else { Key::Char('v') },
            0x30 => if is_upper { Key::Char('B') } else { Key::Char('b') },
            0x31 => if is_upper { Key::Char('N') } else { Key::Char('n') },
            0x32 => if is_upper { Key::Char('M') } else { Key::Char('m') },
            
            
            0x02 => if shift { Key::Char('!') } else { Key::Char('1') },
            0x03 => if shift { Key::Char('@') } else { Key::Char('2') },
            0x04 => if shift { Key::Char('#') } else { Key::Char('3') },
            0x05 => if shift { Key::Char('$') } else { Key::Char('4') },
            0x06 => if shift { Key::Char('%') } else { Key::Char('5') },
            0x07 => if shift { Key::Char('^') } else { Key::Char('6') },
            0x08 => if shift { Key::Char('&') } else { Key::Char('7') },
            0x09 => if shift { Key::Char('*') } else { Key::Char('8') },
            0x0A => if shift { Key::Char('(') } else { Key::Char('9') },
            0x0B => if shift { Key::Char(')') } else { Key::Char('0') },
            
            
            0x0C => if shift { Key::Char('_') } else { Key::Char('-') },
            0x0D => if shift { Key::Char('+') } else { Key::Char('=') },
            0x1A => if shift { Key::Char('{') } else { Key::Char('[') },
            0x1B => if shift { Key::Char('}') } else { Key::Char(']') },
            0x27 => if shift { Key::Char(':') } else { Key::Char(';') },
            0x28 => if shift { Key::Char('"') } else { Key::Char('\'') },
            0x2B => if shift { Key::Char('|') } else { Key::Char('\\') },
            0x29 => if shift { Key::Char('~') } else { Key::Char('`') },
            0x33 => if shift { Key::Char('<') } else { Key::Char(',') },
            0x34 => if shift { Key::Char('>') } else { Key::Char('.') },
            0x35 => if shift { Key::Char('?') } else { Key::Char('/') },
            
            _ => Key::None,
        }
    }
}


static mut LEFT_CTRL: bool = false;
static mut LEFT_ALT: bool = false;
static mut LEFT_SHIFT: bool = false;
static mut RIGHT_SHIFT: bool = false;
static mut CAPS_LOCK: bool = false;

static mut LEFT_WIN: bool = false;
static mut RIGHT_WIN: bool = false;


const SCANCODE_LEFT_CTRL: u8 = 0x1D;
const SCANCODE_LEFT_ALT: u8 = 0x38;
const SCANCODE_LEFT_SHIFT: u8 = 0x2A;
const SCANCODE_RIGHT_SHIFT: u8 = 0x36;
const SCANCODE_CAPS_LOCK: u8 = 0x3A;

const SCANCODE_LEFT_WIN: u8 = 0x5B;
const SCANCODE_RIGHT_WIN: u8 = 0x5C;

const SCANCODE_ALT_RELEASE: u8 = 0xB8;
const SCANCODE_CTRL_RELEASE: u8 = 0x9D;
const SCANCODE_LSHIFT_RELEASE: u8 = 0xAA;
const SCANCODE_RSHIFT_RELEASE: u8 = 0xB6;
const SCANCODE_CAPS_RELEASE: u8 = 0xBA;

const SCANCODE_LEFT_WIN_RELEASE: u8 = 0xDB;
const SCANCODE_RIGHT_WIN_RELEASE: u8 = 0xDC;

const SCANCODE_F4: u8 = 0x3E;
const SCANCODE_ESC: u8 = 0x01;
const SCANCODE_TAB: u8 = 0x0F;


pub fn update_modifiers(scancode: u8) {
    unsafe {
        match scancode {
            SCANCODE_LEFT_CTRL => LEFT_CTRL = true,
            SCANCODE_CTRL_RELEASE => LEFT_CTRL = false,
            SCANCODE_LEFT_ALT => LEFT_ALT = true,
            SCANCODE_ALT_RELEASE => LEFT_ALT = false,
            SCANCODE_LEFT_SHIFT => LEFT_SHIFT = true,
            SCANCODE_LSHIFT_RELEASE => LEFT_SHIFT = false,
            SCANCODE_RIGHT_SHIFT => RIGHT_SHIFT = true,
            SCANCODE_RSHIFT_RELEASE => RIGHT_SHIFT = false,
            SCANCODE_CAPS_LOCK => {
                CAPS_LOCK = !CAPS_LOCK;
            }
            SCANCODE_CAPS_RELEASE => {}
            SCANCODE_LEFT_WIN => LEFT_WIN = true,
            SCANCODE_LEFT_WIN_RELEASE => LEFT_WIN = false,
            SCANCODE_RIGHT_WIN => RIGHT_WIN = true,
            SCANCODE_RIGHT_WIN_RELEASE => RIGHT_WIN = false,
            _ => {}
        }
    }
}


pub fn is_shift_pressed() -> bool {
    unsafe { LEFT_SHIFT || RIGHT_SHIFT }
}


pub fn is_alt_pressed() -> bool {
    unsafe { LEFT_ALT }
}


pub fn is_ctrl_pressed() -> bool {
    unsafe { LEFT_CTRL }
}


pub fn is_caps_lock() -> bool {
    unsafe { CAPS_LOCK }
}

pub fn is_win_pressed() -> bool {
    unsafe { LEFT_WIN || RIGHT_WIN }
}

pub fn is_ctrl_alt() -> bool {
    unsafe { LEFT_CTRL && LEFT_ALT }
}

pub fn is_ctrl_shift() -> bool {
    unsafe { LEFT_CTRL && (LEFT_SHIFT || RIGHT_SHIFT) }
}

pub fn is_ctrl_tab(scancode: u8) -> bool {
    unsafe { LEFT_CTRL && scancode == SCANCODE_TAB }
}

pub fn is_win_key(scancode: u8) -> bool {
    scancode == SCANCODE_LEFT_WIN || scancode == SCANCODE_RIGHT_WIN
}


pub fn is_alt_f4(scancode: u8) -> bool {
    unsafe {
        LEFT_ALT && scancode == SCANCODE_F4
    }
}


pub fn is_ctrl_shift_esc(scancode: u8) -> bool {
    unsafe {
        scancode == SCANCODE_ESC && LEFT_CTRL && (LEFT_SHIFT || RIGHT_SHIFT)
    }
}


pub fn is_alt_shift(scancode: u8) -> bool {
    unsafe {
        (scancode == SCANCODE_LEFT_SHIFT || scancode == SCANCODE_RIGHT_SHIFT) && LEFT_ALT
    }
}


pub fn get_raw_scancode() -> Option<u8> {
    let status: u8;
    unsafe { asm!("in al, 0x64", out("al") status); }
    
    if status & 1 == 0 {
        return None;
    }
    
    let scancode: u8;
    unsafe { asm!("in al, 0x60", out("al") scancode); }
    
    Some(scancode)
}


pub fn get_key() -> Key {
    let status: u8;
    unsafe { asm!("in al, 0x64", out("al") status); }
    
    if status & 1 == 0 {
        return Key::None;
    }
    
    let scancode: u8;
    unsafe { asm!("in al, 0x60", out("al") scancode); }
    
   
    update_modifiers(scancode);
    
    
    if scancode & 0x80 != 0 {
        return Key::None;
    }
    
    
    let shift = is_shift_pressed();
    let caps = is_caps_lock();
    
    Key::from_scancode(scancode, shift, caps)
}


pub fn wait_key() -> Key {
    loop {
        let key = get_key();
        if key != Key::None {
            return key;
        }
        unsafe { asm!("hlt"); }
    }
}


pub fn reset_modifiers() {
    unsafe {
        LEFT_CTRL = false;
        LEFT_ALT = false;
        LEFT_SHIFT = false;
        RIGHT_SHIFT = false;
        CAPS_LOCK = false;
        LEFT_WIN = false;
        RIGHT_WIN = false;
    }
}


pub fn char_from_scancode(scancode: u8) -> Option<char> {
    let shift = is_shift_pressed();
    let caps = is_caps_lock();
    let is_upper = shift || caps;
    
    match scancode {
        
        0x10 => if is_upper { Some('Q') } else { Some('q') },
        0x11 => if is_upper { Some('W') } else { Some('w') },
        0x12 => if is_upper { Some('E') } else { Some('e') },
        0x13 => if is_upper { Some('R') } else { Some('r') },
        0x14 => if is_upper { Some('T') } else { Some('t') },
        0x15 => if is_upper { Some('Y') } else { Some('y') },
        0x16 => if is_upper { Some('U') } else { Some('u') },
        0x17 => if is_upper { Some('I') } else { Some('i') },
        0x18 => if is_upper { Some('O') } else { Some('o') },
        0x19 => if is_upper { Some('P') } else { Some('p') },
        0x1E => if is_upper { Some('A') } else { Some('a') },
        0x1F => if is_upper { Some('S') } else { Some('s') },
        0x20 => if is_upper { Some('D') } else { Some('d') },
        0x21 => if is_upper { Some('F') } else { Some('f') },
        0x22 => if is_upper { Some('G') } else { Some('g') },
        0x23 => if is_upper { Some('H') } else { Some('h') },
        0x24 => if is_upper { Some('J') } else { Some('j') },
        0x25 => if is_upper { Some('K') } else { Some('k') },
        0x26 => if is_upper { Some('L') } else { Some('l') },
        0x2C => if is_upper { Some('Z') } else { Some('z') },
        0x2D => if is_upper { Some('X') } else { Some('x') },
        0x2E => if is_upper { Some('C') } else { Some('c') },
        0x2F => if is_upper { Some('V') } else { Some('v') },
        0x30 => if is_upper { Some('B') } else { Some('b') },
        0x31 => if is_upper { Some('N') } else { Some('n') },
        0x32 => if is_upper { Some('M') } else { Some('m') },
        
        
        0x02 => if shift { Some('!') } else { Some('1') },
        0x03 => if shift { Some('@') } else { Some('2') },
        0x04 => if shift { Some('#') } else { Some('3') },
        0x05 => if shift { Some('$') } else { Some('4') },
        0x06 => if shift { Some('%') } else { Some('5') },
        0x07 => if shift { Some('^') } else { Some('6') },
        0x08 => if shift { Some('&') } else { Some('7') },
        0x09 => if shift { Some('*') } else { Some('8') },
        0x0A => if shift { Some('(') } else { Some('9') },
        0x0B => if shift { Some(')') } else { Some('0') },
        
        
        0x0C => if shift { Some('_') } else { Some('-') },
        0x0D => if shift { Some('+') } else { Some('=') },
        0x1A => if shift { Some('{') } else { Some('[') },
        0x1B => if shift { Some('}') } else { Some(']') },
        0x27 => if shift { Some(':') } else { Some(';') },
        0x28 => if shift { Some('"') } else { Some('\'') },
        0x2B => if shift { Some('|') } else { Some('\\') },
        0x29 => if shift { Some('~') } else { Some('`') },
        0x33 => if shift { Some('<') } else { Some(',') },
        0x34 => if shift { Some('>') } else { Some('.') },
        0x35 => if shift { Some('?') } else { Some('/') },
        
       
        0x39 => Some(' '),
        0x1C => Some('\n'),
        0x0E => Some('\x08'),
        _ => None,
    }
}