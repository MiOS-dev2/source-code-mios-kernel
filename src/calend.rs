#![allow(dead_code)]

use crate::graphics::{Graphics, Color};
use crate::wm;

pub static mut CALENDAR_WINDOW_REQUESTED: bool = false;
pub static mut CALENDAR_WINDOW_INDEX: Option<usize> = None;
pub static mut CALENDAR_WINDOW_CLOSE_REQUESTED: bool = false;


unsafe fn cmos_read(reg: u8) -> u8 {
    let val: u8;
    core::arch::asm!(
        "out 0x70, al",
        "in al, 0x71",
        inout("al") reg => val,
        options(nomem, nostack, preserves_flags)
    );
    val
}

fn bcd_to_bin(b: u8) -> u8 {
    (b & 0x0F) + ((b >> 4) * 10)
}


pub fn get_datetime() -> (u8, u8, u8, u8, u8, u16) {
    unsafe {
        loop {
            let status = cmos_read(0x0A);
            if status & 0x80 == 0 { break; }
        }

        let mut sec   = cmos_read(0x00);
        let mut min   = cmos_read(0x02);
        let mut hour  = cmos_read(0x04);
        let mut day   = cmos_read(0x07);
        let mut month = cmos_read(0x08);
        let mut year  = cmos_read(0x09);

        let reg_b = cmos_read(0x0B);
        let bcd = reg_b & 0x04 == 0;

        if bcd {
            sec   = bcd_to_bin(sec);
            min   = bcd_to_bin(min);
            hour  = bcd_to_bin(hour);
            day   = bcd_to_bin(day);
            month = bcd_to_bin(month);
            year  = bcd_to_bin(year);
        }

        if reg_b & 0x02 == 0 {
            let pm = cmos_read(0x04) & 0x80 != 0;
            if pm && hour < 12 { hour += 12; }
            if !pm && hour == 12 { hour = 0; }
        }

        let full_year = 2000 + year as u16;

        (hour, min, sec, day, month, full_year)
    }
}

fn u8_to_2digits(v: u8) -> [u8; 2] {
    [b'0' + (v / 10), b'0' + (v % 10)]
}

fn u16_to_4digits(v: u16) -> [u8; 4] {
    [
        b'0' + ((v / 1000) % 10) as u8,
        b'0' + ((v / 100) % 10) as u8,
        b'0' + ((v / 10) % 10) as u8,
        b'0' + (v % 10) as u8,
    ]
}



pub fn draw_calendar(gfx: &mut Graphics, cx: usize, cy: usize, win_w: usize, win_h: usize) {
    let bg          = Color::rgb(243, 243, 243).to_u32();
    let header_bg   = Color::rgb(0, 120, 212).to_u32();
    let header_sub  = Color::rgb(0, 99, 177).to_u32();
    let text_dark   = Color::rgb(32, 32, 32).to_u32();
    let text_light  = Color::WHITE.to_u32();
    let label_gray  = Color::rgb(96, 96, 96).to_u32();
    let accent      = Color::rgb(0, 120, 212).to_u32();
    let accent_soft = Color::rgb(199, 224, 244).to_u32();
    let card_bg     = Color::WHITE.to_u32();
    let card_border = Color::rgb(225, 225, 225).to_u32();
    let divider     = Color::rgb(232, 232, 232).to_u32();

    gfx.fill_rect(cx, cy, win_w, win_h, bg);

    let header_h = 56usize;
    gfx.fill_rect(cx, cy, win_w, header_h, header_bg);
    gfx.draw_text(cx + 16, cy + 12, "Calendar", text_light, header_bg);

    let (h, m, s, day, month, year) = get_datetime();

    let day_d = u8_to_2digits(day);
    let mon_d = u8_to_2digits(month);
    let yr_d  = u16_to_4digits(year);

    let mut date_buf = [0u8; 10];
    date_buf[0] = day_d[0]; date_buf[1] = day_d[1];
    date_buf[2] = b'.';
    date_buf[3] = mon_d[0]; date_buf[4] = mon_d[1];
    date_buf[5] = b'.';
    date_buf[6] = yr_d[0]; date_buf[7] = yr_d[1];
    date_buf[8] = yr_d[2]; date_buf[9] = yr_d[3];
    let date_str = core::str::from_utf8(&date_buf).unwrap_or("00.00.0000");

    let weekday = zeller_weekday(day, month, year);
    let wd_str = match weekday {
        0 => "Sunday",
        1 => "Monday",
        2 => "Tuesday",
        3 => "Wednesday",
        4 => "Thursday",
        5 => "Friday",
        6 => "Saturday",
        _ => "?",
    };

    let mut sub_buf = [0u8; 32];
    let mut idx = 0usize;
    for &b in wd_str.as_bytes() {
        if idx >= sub_buf.len() - 2 { break; }
        sub_buf[idx] = b; idx += 1;
    }
    if idx < sub_buf.len() - 1 {
        sub_buf[idx] = b','; idx += 1;
        sub_buf[idx] = b' '; idx += 1;
    }
    for &b in date_str.as_bytes() {
        if idx >= sub_buf.len() { break; }
        sub_buf[idx] = b; idx += 1;
    }
    let sub_str = core::str::from_utf8(&sub_buf[..idx]).unwrap_or("");

    gfx.draw_text(cx + 16, cy + 32, sub_str,
                  Color::rgb(224, 238, 250).to_u32(), header_bg);

    gfx.fill_rect(cx, cy + header_h, win_w, 2, header_sub);

    let pad = 16usize;
    let card_x = cx + pad;
    let card_y = cy + header_h + pad;
    let card_w = win_w - pad * 2;
    let card_h = 130usize;

    gfx.fill_rect(card_x + 2, card_y + 2, card_w, card_h,
                  Color::rgb(220, 220, 220).to_u32());
    gfx.fill_rect(card_x, card_y, card_w, card_h, card_bg);
    gfx.fill_rect(card_x, card_y, card_w, 1, card_border);
    gfx.fill_rect(card_x, card_y + card_h - 1, card_w, 1, card_border);
    gfx.fill_rect(card_x, card_y, 1, card_h, card_border);
    gfx.fill_rect(card_x + card_w - 1, card_y, 1, card_h, card_border);

    let mut time_buf = [0u8; 8];
    let hh = u8_to_2digits(h);
    let mm = u8_to_2digits(m);
    let ss = u8_to_2digits(s);
    time_buf[0] = hh[0]; time_buf[1] = hh[1];
    time_buf[2] = b':';
    time_buf[3] = mm[0]; time_buf[4] = mm[1];
    time_buf[5] = b':';
    time_buf[6] = ss[0]; time_buf[7] = ss[1];
    let time_str = core::str::from_utf8(&time_buf).unwrap_or("00:00:00");

    let scale = 5usize;
    let char_w = 6 * scale;
    let total_w = time_str.len() * char_w;
    let start_x = card_x + if card_w > total_w { (card_w - total_w) / 2 } else { 6 };
    let start_y = card_y + (card_h - 7 * scale) / 2;

    for (i, ch) in time_str.bytes().enumerate() {
        draw_big_char_cal(gfx, start_x + i * char_w, start_y, ch, scale,
                          text_dark, card_bg);
    }

    let accent_w = 60usize;
    let accent_x = card_x + (card_w - accent_w) / 2;
    let accent_y = card_y + card_h - 18;
    gfx.fill_rect(accent_x, accent_y, accent_w, 3, accent);

    let date_card_y = card_y + card_h + pad;
    let footer_h = 28usize;
    let footer_y = cy + win_h - footer_h;
    let date_card_h = if footer_y > date_card_y + 40 {
        footer_y - date_card_y - pad
    } else {
        40
    };

    let dc_x = card_x;
    let dc_y = date_card_y;
    let dc_w = card_w;
    let dc_h = date_card_h;

    gfx.fill_rect(dc_x + 2, dc_y + 2, dc_w, dc_h,
                  Color::rgb(220, 220, 220).to_u32());
    gfx.fill_rect(dc_x, dc_y, dc_w, dc_h, card_bg);
    gfx.fill_rect(dc_x, dc_y, dc_w, 1, card_border);
    gfx.fill_rect(dc_x, dc_y + dc_h - 1, dc_w, 1, card_border);
    gfx.fill_rect(dc_x, dc_y, 1, dc_h, card_border);
    gfx.fill_rect(dc_x + dc_w - 1, dc_y, 1, dc_h, card_border);

    gfx.fill_rect(dc_x, dc_y, 4, dc_h, accent);

    let wd_pad_x = 20usize;
    let wd_y = dc_y + 18;
    gfx.draw_text(dc_x + wd_pad_x, wd_y, wd_str, text_dark, card_bg);

    let date_y2 = wd_y + 22;
    gfx.draw_text(dc_x + wd_pad_x, date_y2, date_str, label_gray, card_bg);

    let div_y = date_y2 + 26;
    if div_y < dc_y + dc_h {
        gfx.fill_rect(dc_x + wd_pad_x, div_y, dc_w - wd_pad_x * 2, 1, divider);

        let row_y = div_y + 14;
        let row_gap = 20usize;

        if row_y + row_gap * 2 + 10 < dc_y + dc_h {
            gfx.draw_text(dc_x + wd_pad_x, row_y, "Time zone", label_gray, card_bg);
            gfx.draw_text(dc_x + wd_pad_x + 100, row_y, "Local (BIOS RTC)", text_dark, card_bg);

            gfx.draw_text(dc_x + wd_pad_x, row_y + row_gap, "Source", label_gray, card_bg);
            gfx.draw_text(dc_x + wd_pad_x + 100, row_y + row_gap, "CMOS Real-Time Clock", text_dark, card_bg);

            gfx.draw_text(dc_x + wd_pad_x, row_y + row_gap * 2, "Format", label_gray, card_bg);
            gfx.draw_text(dc_x + wd_pad_x + 100, row_y + row_gap * 2, "24-hour", text_dark, card_bg);
        }
    }

    let chip_w = 90usize;
    let chip_h = 22usize;
    if dc_w > chip_w + 32 && dc_h > chip_h + 32 {
        let chip_x = dc_x + dc_w - chip_w - 16;
        let chip_y = dc_y + dc_h - chip_h - 16;
        gfx.fill_rect(chip_x, chip_y, chip_w, chip_h, accent_soft);
        gfx.draw_text(chip_x + 10, chip_y + 6, "LIVE", accent, accent_soft);
    }

    gfx.fill_rect(cx, footer_y, win_w, footer_h,
                  Color::rgb(240, 240, 240).to_u32());
    gfx.fill_rect(cx, footer_y, win_w, 1, divider);
    gfx.draw_text(cx + 12, footer_y + 8, "Time & date from BIOS (CMOS RTC)",
                  label_gray, Color::rgb(240, 240, 240).to_u32());
}


fn zeller_weekday(day: u8, month: u8, year: u16) -> u8 {
    let mut m = month as i32;
    let mut y = year as i32;
    if m < 3 {
        m += 12;
        y -= 1;
    }
    let k = y % 100;
    let j = y / 100;
    let h = (day as i32 + (13 * (m + 1)) / 5 + k + k / 4 + j / 4 + 5 * j) % 7;

    ((h + 6) % 7) as u8
}

fn draw_big_char_cal(
    gfx: &mut Graphics,
    x: usize,
    y: usize,
    ch: u8,
    scale: usize,
    color: u32,
    bg: u32,
) {
    let glyph: [u8; 7] = match ch {
        b'0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
        b'1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        b'2' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111],
        b'3' => [0b11111, 0b00010, 0b00100, 0b00010, 0b00001, 0b10001, 0b01110],
        b'4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        b'5' => [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
        b'6' => [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
        b'7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        b'8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        b'9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100],
        b':' => [0b00000, 0b00100, 0b00100, 0b00000, 0b00100, 0b00100, 0b00000],
        _ => [0u8; 7],
    };

    let cw = 5 * scale;
    let chh = 7 * scale;
    gfx.fill_rect(x, y, cw, chh, bg);

    for (row, bits) in glyph.iter().enumerate() {
        for col in 0..5 {
            if (bits >> (4 - col)) & 1 == 1 {
                gfx.fill_rect(x + col * scale, y + row * scale, scale, scale, color);
            }
        }
    }
}


pub fn calendar_start() -> &'static str {
    unsafe {
        CALENDAR_WINDOW_REQUESTED = true;
    }
    "Calendar opened!"
}

pub fn calendar_stop() -> &'static str {
    unsafe {
        CALENDAR_WINDOW_CLOSE_REQUESTED = true;
    }
    "Calendar closed!"
}