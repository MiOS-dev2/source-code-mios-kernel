use crate::mouse;

pub static mut RUSSIAN: bool = false;

const US: &[(u8, char, char)] = &[
    (0x02, '1', '!'), (0x03, '2', '@'), (0x04, '3', '#'), (0x05, '4', '$'), (0x06, '5', '%'),
    (0x07, '6', '^'), (0x08, '7', '&'), (0x09, '8', '*'), (0x0A, '9', '('), (0x0B, '0', ')'),
    (0x0C, '-', '_'), (0x0D, '=', '+'),
    (0x10, 'q', 'Q'), (0x11, 'w', 'W'), (0x12, 'e', 'E'), (0x13, 'r', 'R'), (0x14, 't', 'T'),
    (0x15, 'y', 'Y'), (0x16, 'u', 'U'), (0x17, 'i', 'I'), (0x18, 'o', 'O'), (0x19, 'p', 'P'),
    (0x1A, '[', '{'), (0x1B, ']', '}'),
    (0x1E, 'a', 'A'), (0x1F, 's', 'S'), (0x20, 'd', 'D'), (0x21, 'f', 'F'), (0x22, 'g', 'G'),
    (0x23, 'h', 'H'), (0x24, 'j', 'J'), (0x25, 'k', 'K'), (0x26, 'l', 'L'), (0x27, ';', ':'),
    (0x28, '\'', '"'), (0x29, '`', '~'), (0x2B, '\\', '|'),
    (0x2C, 'z', 'Z'), (0x2D, 'x', 'X'), (0x2E, 'c', 'C'), (0x2F, 'v', 'V'), (0x30, 'b', 'B'),
    (0x31, 'n', 'N'), (0x32, 'm', 'M'), (0x33, ',', '<'), (0x34, '.', '>'), (0x35, '/', '?'),
    (0x39, ' ', ' '),
];

const RU: &[(u8, char, char)] = &[
    (0x02, '1', '!'), (0x03, '2', '"'), (0x04, '3', '№'), (0x05, '4', ';'), (0x06, '5', '%'),
    (0x07, '6', ':'), (0x08, '7', '?'), (0x09, '8', '*'), (0x0A, '9', '('), (0x0B, '0', ')'),
    (0x0C, '-', '_'), (0x0D, '=', '+'),
    (0x10, 'й', 'Й'), (0x11, 'ц', 'Ц'), (0x12, 'у', 'У'), (0x13, 'к', 'К'), (0x14, 'е', 'Е'),
    (0x15, 'н', 'Н'), (0x16, 'г', 'Г'), (0x17, 'ш', 'Ш'), (0x18, 'щ', 'Щ'), (0x19, 'з', 'З'),
    (0x1A, 'х', 'Х'), (0x1B, 'ъ', 'Ъ'),
    (0x1E, 'ф', 'Ф'), (0x1F, 'ы', 'Ы'), (0x20, 'в', 'В'), (0x21, 'а', 'А'), (0x22, 'п', 'П'),
    (0x23, 'р', 'Р'), (0x24, 'о', 'О'), (0x25, 'л', 'Л'), (0x26, 'д', 'Д'), (0x27, 'ж', 'Ж'),
    (0x28, 'э', 'Э'), (0x29, 'ё', 'Ё'), (0x2B, '\\', '/'),
    (0x2C, 'я', 'Я'), (0x2D, 'ч', 'Ч'), (0x2E, 'с', 'С'), (0x2F, 'м', 'М'), (0x30, 'и', 'И'),
    (0x31, 'т', 'Т'), (0x32, 'ь', 'Ь'), (0x33, 'б', 'Б'), (0x34, 'ю', 'Ю'), (0x35, '.', ','),
    (0x39, ' ', ' '),
];

pub fn char_for(sc: u8) -> Option<char> {
    let table = if unsafe { RUSSIAN } { RU } else { US };
    let shift = mouse::shift_pressed();
    table.iter().find(|e| e.0 == sc).map(|e| if shift { e.2 } else { e.1 })
}

pub fn sys_char(sc: u8, c: char) -> char {
    if unsafe { RUSSIAN } {
        if let Some(r) = char_for(sc) { return r; }
    }
    mouse::apply_shift(c)
}

pub fn toggle_layout() { unsafe { RUSSIAN = !RUSSIAN; } }
pub fn layout_name() -> &'static str { if unsafe { RUSSIAN } { "RU" } else { "EN" } }

pub const BACKSPACE: u8 = 0x0E;
pub const ENTER: u8 = 0x1C;
pub const TAB: u8 = 0x0F;
pub const ESC: u8 = 0x01;
pub const F1: u8 = 0x3B;
pub const F5: u8 = 0x3F;
pub const UP: u8 = 0x48;
pub const DOWN: u8 = 0x50;
pub const LEFT: u8 = 0x4B;
pub const RIGHT: u8 = 0x4D;
pub const PGUP: u8 = 0x49;
pub const PGDN: u8 = 0x51;
pub const HOME: u8 = 0x47;
pub const END: u8 = 0x4F;
pub const DELETE: u8 = 0x53;

pub fn ext(sc: u8) -> Option<u8> { if sc == 0 { Some(unsafe { mouse::LAST_EXT_KEY }) } else { None } }
