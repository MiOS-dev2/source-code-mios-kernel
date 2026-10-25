#![no_std]
#![allow(dead_code)]
#![feature(abi_x86_interrupt)]
#![allow(static_mut_refs)]
#![allow(dangerous_implicit_autorefs)]

mod vga;
mod vesa;
mod multiboot;
mod keyboard;
mod shell;
mod ata;
mod fs;
mod console;
mod vesa_gui;
mod utils;
mod graphics;
mod gui;
mod wm;
mod tamzen_font;
mod idt;
mod mouse;
mod bmp;
mod users;
mod apps;
mod gl;
mod task_manager;
mod cre_tool;
mod calend;
mod html;
mod support;
mod pcinf;

extern crate alloc;
mod heap;
mod timer;
mod pci;
mod net;
mod netcmd;
mod font;
mod web;
mod fastblit;
mod netwin;
mod cui;
mod installer;
mod syscfg;
mod termfs;

pub static mut NET_APP_REQUEST: u8 = 0;

static mut PKG_URL: [u8; 256] = [0; 256];
static mut PKG_URL_LEN: usize = 0;
static mut PKG_PENDING: bool = false;

struct PkgAnim {
    active: bool,
    busy: bool,
    frame: usize,
    last: u64,
    gfx: *mut Graphics,
    wm: *const WindowManager,
    ti: *const [u8; 256],
    tl: usize,
    cx: usize,
    cy: usize,
    cb: *mut [[u32; 16]; 16],
    cs: *const [[u32; 16]; 16],
}
static mut PKG: PkgAnim = PkgAnim {
    active: false, busy: false, frame: 0, last: 0,
    gfx: core::ptr::null_mut(), wm: core::ptr::null(), ti: core::ptr::null(), tl: 0,
    cx: 0, cy: 0, cb: core::ptr::null_mut(), cs: core::ptr::null(),
};
const PKG_FRAMES: [&str; 5] = [
    "Dowloading... [#    ]",
    "Dowloading... [ #   ]",
    "Dowloading... [  #  ]",
    "Dowloading... [   # ]",
    "Dowloading... [    #]",
];


use core::panic::PanicInfo;
use core::str;
use multiboot::MultibootInfo;
use graphics::{Graphics, Color};
use wm::{WindowManager, WindowState};
use bmp::BmpImage;
use ata::AtaDrive;

pub static mut VESA_INFO: Option<vesa::VesaInfo> = None;

static mut LAST_CLOCK_SEC: u8 = 255;

static mut EXT4_WINDOW_INDEX: Option<usize> = None;

fn num_to_str(num: usize) -> &'static str {
    static mut BUF: [u8; 20] = [0; 20];
    unsafe {
        let mut temp = num;
        let mut digits = [0u8; 20];
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

fn boot_log(gfx: &mut Graphics, prefix: &str, message: &str, status: &str) {
    let prefix_color = Color::rgb(43, 44, 43).to_u32();
    let message_color = Color::rgb(114, 112, 112).to_u32();
    let ok_color = Color::rgb(0, 51, 0).to_u32();
    let info_color = Color::rgb(8, 22, 83).to_u32();
    let fail_color = Color::rgb(220, 0, 0).to_u32();
    let bg = Color::rgb(0, 0, 0).to_u32();

    static mut BOOT_LINE: usize = 0;
    unsafe {
        let max_lines = (gfx.height - 10) / 16;
        if BOOT_LINE >= max_lines {
            let w = gfx.width;
            let src_start = (10 + 16) * w;
            let src_end = (10 + max_lines * 16) * w;
            graphics::BACKBUFFER.copy_within(src_start..src_end, 10 * w);
            BOOT_LINE = max_lines - 1;
        }
        let y = 10 + BOOT_LINE * 16;
        BOOT_LINE += 1;
        gfx.fill_rect(0, y, gfx.width, 16, bg);
        gfx.draw_text(10, y, prefix, prefix_color, bg);
        let msg_x = 10 + prefix.len() * 8 + 8;
        gfx.draw_text(msg_x, y, message, message_color, bg);
        if !status.is_empty() {
            let status_x = gfx.width - 120;
            let (status_text, status_color) = match status {
                "OK" => ("[ OK ]", ok_color),
                "info" => ("[ info ]", info_color),
                "FAIL" => ("[ FAIL ]", fail_color),
                _ => (status, message_color),
            };
            gfx.draw_text(status_x, y, status_text, status_color, bg);
        }
        gfx.flush();
    }
}

fn boot_delay() {
    for _ in 0..800_000 {
        unsafe { core::arch::asm!("nop") }
    }
}

fn boot_log_reset() {
    unsafe {}
}

fn shutdown_log(gfx: &mut Graphics, message: &str, status: &str) {
    let prefix = "[ HALT ]";
    let prefix_color = Color::rgb(43, 44, 43).to_u32();
    let message_color = Color::rgb(114, 112, 112).to_u32();
    let ok_color = Color::rgb(0, 51, 0).to_u32();
    let info_color = Color::rgb(8, 22, 83).to_u32();
    let bg = Color::rgb(0, 0, 0).to_u32();

    static mut SHUT_LINE: usize = 0;
    unsafe {
        let max_lines = (gfx.height - 10) / 16;
        if SHUT_LINE >= max_lines {
            let w = gfx.width;
            let src_start = (10 + 16) * w;
            let src_end = (10 + max_lines * 16) * w;
            graphics::BACKBUFFER.copy_within(src_start..src_end, 10 * w);
            SHUT_LINE = max_lines - 1;
        }
        let y = 10 + SHUT_LINE * 16;
        SHUT_LINE += 1;
        gfx.fill_rect(0, y, gfx.width, 16, bg);
        gfx.draw_text(10, y, prefix, prefix_color, bg);
        let msg_x = 10 + prefix.len() * 8 + 8;
        gfx.draw_text(msg_x, y, message, message_color, bg);
        if !status.is_empty() {
            let status_x = gfx.width - 120;
            let (status_text, status_color) = match status {
                "OK" => ("[ OK ]", ok_color),
                "info" => ("[ info ]", info_color),
                _ => (status, message_color),
            };
            gfx.draw_text(status_x, y, status_text, status_color, bg);
        }
        gfx.flush();
    }
}

fn shutdown_sequence(gfx: &mut Graphics) {
    gfx.clear(Color::BLACK.to_u32());

    let steps: [(&str, &str); 32] = [
        ("Shutdown requested by user...", "info"),
        ("Closing all windows...", "OK"),
        ("Stopping window manager...", "OK"),
        ("Stopping desktop shell...", "OK"),
        ("Closing terminal session...", "OK"),
        ("Stopping live wallpaper...", "OK"),
        ("Closing media player...", "OK"),
        ("Closing Paint...", "OK"),
        ("Closing Notepad...", "OK"),
        ("Closing Cre Tool...", "OK"),
        ("Saving user settings...", ""),
        ("Writing system.cfg...", "OK"),
        ("Saving desktop icon positions...", "OK"),
        ("Saving shortcuts...", "OK"),
        ("Releasing bmp image buffers...", "OK"),
        ("Stopping package manager...", "OK"),
        ("Closing network connections...", "OK"),
        ("Stopping HTTP client...", "OK"),
        ("Network stack shutdown...", "OK"),
        ("Stopping network driver...", "OK"),
        ("Syncing file systems...", "info"),
        ("Flushing fat32 cache...", "OK"),
        ("Unmounting fat32 volumes...", "OK"),
        ("Flushing ATA write cache...", "OK"),
        ("Stopping SATA controller...", "OK"),
        ("Stopping ATAPI...", "OK"),
        ("Stopping mouse driver...", "OK"),
        ("Stopping keyboard driver...", "OK"),
        ("Stopping timer...", "OK"),
        ("Releasing heap memory...", "OK"),
        ("Stopping VESA graphics...", "OK"),
        ("Power down...", "info"),
    ];

    for (i, (msg, status)) in steps.iter().enumerate() {
        if i == 11 {
            syscfg::poll();
        }
        shutdown_log(gfx, msg, status);
        boot_delay();
        if i == 20 || i == 23 {
            boot_delay();
            boot_delay();
        }
    }
    boot_delay();
    boot_delay();
}

#[cfg(target_arch = "x86")]
use core::arch::x86::{__cpuid, _rdtsc};
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::{__cpuid, _rdtsc};

struct Txt {
    b: [u8; 96],
    n: usize,
}

impl Txt {
    const fn new() -> Self {
        Txt { b: [0; 96], n: 0 }
    }
    fn push(&mut self, c: u8) {
        if self.n < self.b.len() {
            self.b[self.n] = c;
            self.n += 1;
        }
    }
    fn as_str(&self) -> &str {
        unsafe { core::str::from_utf8_unchecked(&self.b[..self.n]) }
    }
    fn is_empty(&self) -> bool {
        self.as_str().trim().is_empty()
    }
}

impl core::fmt::Write for Txt {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for c in s.bytes() {
            self.push(if c >= 0x20 && c < 0x7F { c } else { b'?' });
        }
        Ok(())
    }
}

fn bl_line(gfx: &mut Graphics, prefix: &str, status: &str, args: core::fmt::Arguments) {
    let mut t = Txt::new();
    let _ = core::fmt::write(&mut t, args);
    boot_log(gfx, prefix, t.as_str(), status);
    boot_delay();
}

macro_rules! blog {
    ($g:expr, $p:expr, $s:expr, $($a:tt)*) => {
        bl_line($g, $p, $s, format_args!($($a)*))
    };
}

unsafe fn bl_rd8(a: usize) -> u8 { core::ptr::read_volatile(a as *const u8) }
unsafe fn bl_rd16(a: usize) -> u16 { core::ptr::read_unaligned(a as *const u16) }
unsafe fn bl_rd32(a: usize) -> u32 { core::ptr::read_unaligned(a as *const u32) }
unsafe fn bl_rd64(a: usize) -> u64 { core::ptr::read_unaligned(a as *const u64) }

unsafe fn bl_outb(port: u16, val: u8) {
    core::arch::asm!("out dx, al", in("dx") port, in("al") val, options(nomem, nostack, preserves_flags));
}
unsafe fn bl_inb(port: u16) -> u8 {
    let v: u8;
    core::arch::asm!("in al, dx", out("al") v, in("dx") port, options(nomem, nostack, preserves_flags));
    v
}

fn bl_tsc_hz() -> u64 {
    unsafe {
        let old = bl_inb(0x61);
        bl_outb(0x61, (old & 0xFC) | 0x01);
        bl_outb(0x43, 0xB0);
        bl_outb(0x42, (59659u16 & 0xFF) as u8);
        bl_outb(0x42, (59659u16 >> 8) as u8);
        let g = bl_inb(0x61) & 0xFE;
        bl_outb(0x61, g);
        bl_outb(0x61, g | 1);

        let t0 = _rdtsc();
        let mut guard: u32 = 0;
        while bl_inb(0x61) & 0x20 == 0 {
            guard += 1;
            if guard > 20_000_000 {
                bl_outb(0x61, old);
                return 0;
            }
        }
        let t1 = _rdtsc();
        bl_outb(0x61, old);
        t1.wrapping_sub(t0) * 20
    }
}

fn boot_wait_ms(hz: u64, ms: u64) {
    if hz == 0 {
        for _ in 0..(ms * 1_000_000) {
            unsafe { core::arch::asm!("nop") }
        }
        return;
    }
    let ticks = (hz / 1000) * ms;
    unsafe {
        let start = _rdtsc();
        while _rdtsc().wrapping_sub(start) < ticks {
            core::hint::spin_loop();
        }
    }
}

struct BootHw {
    mem_lower: u32,
    mem_upper: u32,
    have_mem: bool,
    usable_kb: u64,
    have_mmap: bool,
    bios_dev: u32,
    have_bdev: bool,
    acpi_oem: [u8; 6],
    acpi_rev: u8,
    have_acpi: bool,
}

unsafe fn bl_parse_mb2(info_addr: u32) -> BootHw {
    let mut hw = BootHw {
        mem_lower: 0, mem_upper: 0, have_mem: false,
        usable_kb: 0, have_mmap: false,
        bios_dev: 0, have_bdev: false,
        acpi_oem: [0; 6], acpi_rev: 0, have_acpi: false,
    };
    let base = info_addr as usize;
    let total = bl_rd32(base) as usize;
    let mut off = 8usize;
    while off + 8 <= total {
        let p = base + off;
        let t = bl_rd32(p);
        let sz = bl_rd32(p + 4) as usize;
        if t == 0 || sz < 8 { break; }
        match t {
            4 => {
                hw.mem_lower = bl_rd32(p + 8);
                hw.mem_upper = bl_rd32(p + 12);
                hw.have_mem = true;
            }
            5 => {
                hw.bios_dev = bl_rd32(p + 8);
                hw.have_bdev = true;
            }
            6 => {
                let es = bl_rd32(p + 8) as usize;
                if es >= 24 {
                    let mut e = p + 16;
                    while e + es <= p + sz {
                        let len = bl_rd64(e + 8);
                        let ty = bl_rd32(e + 16);
                        if ty == 1 { hw.usable_kb += len / 1024; }
                        e += es;
                    }
                    hw.have_mmap = true;
                }
            }
            14 | 15 => {
                if sz >= 24 {
                    for i in 0..6 { hw.acpi_oem[i] = bl_rd8(p + 8 + 9 + i); }
                    hw.acpi_rev = bl_rd8(p + 8 + 15);
                    hw.have_acpi = true;
                }
            }
            _ => {}
        }
        off += (sz + 7) & !7;
    }
    hw
}

struct SmbIos {
    found: bool,
    bios_vendor: Txt,
    bios_ver: Txt,
    bios_date: Txt,
    bios_rom_kb: u32,
    sys_manuf: Txt,
    sys_prod: Txt,
    sys_ver: Txt,
    board_manuf: Txt,
    board_prod: Txt,
    dimm_count: usize,
    dimm_mb: [u32; 8],
    dimm_speed: [u32; 8],
}

unsafe fn bl_sum(a: usize, len: usize) -> u8 {
    let mut s = 0u8;
    for i in 0..len { s = s.wrapping_add(bl_rd8(a + i)); }
    s
}

unsafe fn bl_smb_str(p: usize, l: usize, idx: u8, end: usize) -> Txt {
    let mut t = Txt::new();
    if idx == 0 { return t; }
    let mut q = p + l;
    let mut cur = 1u8;
    while q < end {
        if bl_rd8(q) == 0 { break; }
        let mut e = q;
        while e < end && bl_rd8(e) != 0 { e += 1; }
        if cur == idx {
            let mut i = q;
            while i < e && t.n < 90 {
                let c = bl_rd8(i);
                t.push(if c >= 0x20 && c < 0x7F { c } else { b'?' });
                i += 1;
            }
            return t;
        }
        q = e + 1;
        cur = cur.wrapping_add(1);
    }
    t
}

unsafe fn bl_smbios_read() -> SmbIos {
    let mut s = SmbIos {
        found: false,
        bios_vendor: Txt::new(), bios_ver: Txt::new(), bios_date: Txt::new(), bios_rom_kb: 0,
        sys_manuf: Txt::new(), sys_prod: Txt::new(), sys_ver: Txt::new(),
        board_manuf: Txt::new(), board_prod: Txt::new(),
        dimm_count: 0, dimm_mb: [0; 8], dimm_speed: [0; 8],
    };

    let mut tab = 0usize;
    let mut tlen = 0usize;
    let mut a = 0xF0000usize;
    while a < 0x100000 {
        if bl_rd8(a) == b'_' && bl_rd8(a + 1) == b'S' && bl_rd8(a + 2) == b'M' {
            if bl_rd8(a + 3) == b'_' {
                let l = bl_rd8(a + 5) as usize;
                if l >= 0x1F && bl_sum(a, l) == 0 {
                    tlen = bl_rd16(a + 0x16) as usize;
                    tab = bl_rd32(a + 0x18) as usize;
                    if tab != 0 && tlen != 0 { break; }
                }
            } else if bl_rd8(a + 3) == b'3' && bl_rd8(a + 4) == b'_' {
                let l = bl_rd8(a + 6) as usize;
                if l >= 0x18 && bl_sum(a, l) == 0 {
                    tlen = bl_rd32(a + 0x0C) as usize;
                    let addr = bl_rd64(a + 0x10);
                    if addr != 0 && addr < 0xFFFF_F000 && tlen != 0 {
                        tab = addr as usize;
                        break;
                    }
                }
            }
        }
        a += 16;
    }
    if tab == 0 || tlen == 0 { return s; }
    s.found = true;

    let end = tab + tlen;
    let mut p = tab;
    let mut n = 0;
    while p + 4 <= end && n < 512 {
        let ty = bl_rd8(p);
        let l = bl_rd8(p + 1) as usize;
        if l < 4 { break; }
        match ty {
            0 => {
                s.bios_vendor = bl_smb_str(p, l, bl_rd8(p + 4), end);
                s.bios_ver = bl_smb_str(p, l, bl_rd8(p + 5), end);
                s.bios_date = bl_smb_str(p, l, bl_rd8(p + 8), end);
                if l > 9 { s.bios_rom_kb = (bl_rd8(p + 9) as u32 + 1) * 64; }
            }
            1 => {
                s.sys_manuf = bl_smb_str(p, l, bl_rd8(p + 4), end);
                s.sys_prod = bl_smb_str(p, l, bl_rd8(p + 5), end);
                s.sys_ver = bl_smb_str(p, l, bl_rd8(p + 6), end);
            }
            2 => {
                s.board_manuf = bl_smb_str(p, l, bl_rd8(p + 4), end);
                s.board_prod = bl_smb_str(p, l, bl_rd8(p + 5), end);
            }
            17 => {
                if l >= 0x10 && s.dimm_count < 8 {
                    let raw = bl_rd16(p + 0x0C) as u32;
                    let mut mb = 0u32;
                    if raw == 0x7FFF && l >= 0x20 {
                        mb = bl_rd32(p + 0x1C) & 0x7FFF_FFFF;
                    } else if raw != 0 && raw != 0xFFFF {
                        mb = if raw & 0x8000 != 0 { (raw & 0x7FFF) / 1024 } else { raw };
                    }
                    if mb > 0 {
                        let speed = if l >= 0x17 { bl_rd16(p + 0x15) as u32 } else { 0 };
                        s.dimm_mb[s.dimm_count] = mb;
                        s.dimm_speed[s.dimm_count] = speed;
                        s.dimm_count += 1;
                    }
                }
            }
            _ => {}
        }
        if ty == 127 { break; }
        let mut q = p + l;
        loop {
            if q + 1 >= end { q = end; break; }
            if bl_rd8(q) == 0 && bl_rd8(q + 1) == 0 { q += 2; break; }
            q += 1;
        }
        p = q;
        n += 1;
    }
    s
}

struct CpuInfo {
    vendor: Txt,
    brand: Txt,
    family: u32,
    model: u32,
    stepping: u32,
    logical: u32,
    feat: Txt,
    has_hv: bool,
    hv: Txt,
}

unsafe fn bl_push_u32(t: &mut Txt, v: u32) {
    for b in v.to_le_bytes() {
        t.push(if b >= 0x20 && b < 0x7F { b } else { b' ' });
    }
}

unsafe fn bl_cpu() -> CpuInfo {
    let mut c = CpuInfo {
        vendor: Txt::new(), brand: Txt::new(),
        family: 0, model: 0, stepping: 0, logical: 1,
        feat: Txt::new(), has_hv: false, hv: Txt::new(),
    };
    let l0 = __cpuid(0);
    bl_push_u32(&mut c.vendor, l0.ebx);
    bl_push_u32(&mut c.vendor, l0.edx);
    bl_push_u32(&mut c.vendor, l0.ecx);

    let ext = __cpuid(0x8000_0000).eax;
    if ext >= 0x8000_0004 {
        for leaf in 0x8000_0002u32..=0x8000_0004u32 {
            let r = __cpuid(leaf);
            bl_push_u32(&mut c.brand, r.eax);
            bl_push_u32(&mut c.brand, r.ebx);
            bl_push_u32(&mut c.brand, r.ecx);
            bl_push_u32(&mut c.brand, r.edx);
        }
    }

    if l0.eax >= 1 {
        let r = __cpuid(1);
        let fam = (r.eax >> 8) & 0xF;
        let efam = (r.eax >> 20) & 0xFF;
        let mdl = (r.eax >> 4) & 0xF;
        let emdl = (r.eax >> 16) & 0xF;
        c.family = if fam == 0xF { fam + efam } else { fam };
        c.model = if fam == 6 || fam == 0xF { (emdl << 4) + mdl } else { mdl };
        c.stepping = r.eax & 0xF;
        c.logical = (r.ebx >> 16) & 0xFF;
        if c.logical == 0 { c.logical = 1; }

        let mut f = |name: &str, on: bool| {
            if on {
                for b in name.bytes() { c.feat.push(b); }
                c.feat.push(b' ');
            }
        };
        f("SSE", r.edx & (1 << 25) != 0);
        f("SSE2", r.edx & (1 << 26) != 0);
        f("SSE3", r.ecx & 1 != 0);
        f("SSSE3", r.ecx & (1 << 9) != 0);
        f("SSE4.1", r.ecx & (1 << 19) != 0);
        f("SSE4.2", r.ecx & (1 << 20) != 0);
        f("AES", r.ecx & (1 << 25) != 0);
        f("AVX", r.ecx & (1 << 28) != 0);

        if r.ecx & (1 << 31) != 0 {
            c.has_hv = true;
            let h = __cpuid(0x4000_0000);
            bl_push_u32(&mut c.hv, h.ebx);
            bl_push_u32(&mut c.hv, h.ecx);
            bl_push_u32(&mut c.hv, h.edx);
        }
    }
    c
}

fn boot_stage_logs(gfx: &mut Graphics) {
    blog!(gfx, "[ FONT ]", "", "Starting font subsystem...");
    blog!(gfx, "[ FONT ]", "", "Loading Tamzen bitmap font...");
    blog!(gfx, "[ FONT ]", "OK", "Glyph cell width: {} px", tamzen_font::FONT_WIDTH);
    blog!(gfx, "[ FONT ]", "", "Loading font module...");
    blog!(gfx, "[ FONT ]", "OK", "Built glyph lookup table.");
    blog!(gfx, "[ FONT ]", "OK", "Started text renderer.");
    blog!(gfx, "[ FONT ]", "OK", "Started input layout switcher.");
    blog!(gfx, "[ FONT ]", "OK", "Reached stage: Fonts.");

    blog!(gfx, "[ FAT32]", "", "Starting FAT32 filesystem driver...");
    blog!(gfx, "[ FAT32]", "", "Probing storage controllers...");
    blog!(gfx, "[ FAT32]", "OK", "Found storage controller.");
    blog!(gfx, "[ FAT32]", "", "Reading boot sector...");
    blog!(gfx, "[ FAT32]", "OK", "Boot sector signature is valid.");
    blog!(gfx, "[ FAT32]", "", "Reading file allocation table...");
    blog!(gfx, "[ FAT32]", "OK", "Allocation table loaded.");
    blog!(gfx, "[ FAT32]", "", "Checking directory structure...");
    blog!(gfx, "[ FAT32]", "OK", "Mounted root volume.");
    blog!(gfx, "[ FAT32]", "OK", "Reached stage: Local File Systems.");

    blog!(gfx, "[ TERM ]", "", "Starting terminal subsystem...");
    blog!(gfx, "[ TERM ]", "OK", "Allocated input buffer (256 bytes).");
    blog!(gfx, "[ TERM ]", "", "Loading command shell...");
    blog!(gfx, "[ TERM ]", "OK", "Registered built-in commands.");
    blog!(gfx, "[ TERM ]", "OK", "Started user session manager.");
    blog!(gfx, "[ TERM ]", "OK", "Reached stage: Terminal.");

    let net_up = net::net().is_some();
    blog!(gfx, "[ NET  ]", "", "Starting network subsystem...");
    blog!(gfx, "[ NET  ]", "", "Scanning PCI bus for network adapters...");
    if net_up {
        blog!(gfx, "[ NET  ]", "OK", "Network adapter found.");
        blog!(gfx, "[ NET  ]", "", "Initializing packet queues...");
        blog!(gfx, "[ NET  ]", "OK", "Started network stack.");
        blog!(gfx, "[ NET  ]", "OK", "Reached stage: Network.");
    } else {
        blog!(gfx, "[ NET  ]", "info", "No network adapter found.");
        blog!(gfx, "[ NET  ]", "info", "Network stack disabled.");
        blog!(gfx, "[ NET  ]", "info", "Skipped stage: Network.");
    }

    blog!(gfx, "[ WEB  ]", "", "Starting web view...");
    blog!(gfx, "[ WEB  ]", "", "Loading HTML parser...");
    blog!(gfx, "[ WEB  ]", "OK", "HTML parser ready.");
    blog!(gfx, "[ WEB  ]", "", "Loading page layout engine...");
    blog!(gfx, "[ WEB  ]", "OK", "Layout engine ready.");
    blog!(gfx, "[ WEB  ]", "OK", "Started browser window manager.");
    blog!(gfx, "[ WEB  ]", "OK", "Reached stage: Web View.");
    blog!(gfx, "[ BOOT ]", "OK", "Reached stage: Graphical Interface.");
}

fn boot_hw_info(gfx: &mut Graphics, info_addr: u32, fb_bpp: u32, fb_addr: usize) {
    let hw = unsafe { bl_parse_mb2(info_addr) };
    let smb = unsafe { bl_smbios_read() };
    let cpu = unsafe { bl_cpu() };
    let hz = bl_tsc_hz();

    unsafe {
        TECH_CPU = Txt::new();
        for b in cpu.brand.as_str().trim().bytes() { TECH_CPU.push(b); }
        TECH_HOST = Txt::new();
        if smb.found {
            for b in smb.sys_manuf.as_str().trim().bytes() { TECH_HOST.push(b); }
            TECH_HOST.push(b' ');
            for b in smb.sys_prod.as_str().trim().bytes() { TECH_HOST.push(b); }
        }
        TECH_CORES = cpu.logical;
        TECH_RAM_MB = if hw.have_mmap {
            hw.usable_kb / 1024
        } else if hw.have_mem {
            ((hw.mem_lower as u64) + (hw.mem_upper as u64) + 1024) / 1024
        } else { 0 };
    }

    blog!(gfx, "[ HW   ]", "", "Detecting system hardware...");

    if smb.found {
        blog!(gfx, "[ BIOS ]", "OK", "Vendor: {}", smb.bios_vendor.as_str().trim());
        blog!(gfx, "[ BIOS ]", "", "Version: {}   Date: {}", smb.bios_ver.as_str().trim(), smb.bios_date.as_str().trim());
        if smb.bios_rom_kb > 0 {
            blog!(gfx, "[ BIOS ]", "", "ROM size: {} KB", smb.bios_rom_kb);
        }
        blog!(gfx, "[ SYS  ]", "OK", "Manufacturer: {}", smb.sys_manuf.as_str().trim());
        blog!(gfx, "[ SYS  ]", "", "Model: {}", smb.sys_prod.as_str().trim());
        if !smb.sys_ver.is_empty() {
            blog!(gfx, "[ SYS  ]", "", "Version: {}", smb.sys_ver.as_str().trim());
        }
        blog!(gfx, "[ BOARD]", "OK", "Board: {} {}", smb.board_manuf.as_str().trim(), smb.board_prod.as_str().trim());
    } else {
        blog!(gfx, "[ BIOS ]", "info", "BIOS tables not found (UEFI firmware?).");
    }

    if !cpu.brand.is_empty() {
        blog!(gfx, "[ CPU  ]", "OK", "{}", cpu.brand.as_str().trim());
    }
    blog!(gfx, "[ CPU  ]", "", "Vendor: {}   Family {} Model {} Stepping {}",
          cpu.vendor.as_str().trim(), cpu.family, cpu.model, cpu.stepping);
    blog!(gfx, "[ CPU  ]", "", "Logical processors: {}", cpu.logical);
    if hz > 0 {
        blog!(gfx, "[ CPU  ]", "", "Clock: ~{} MHz", hz / 1_000_000);
    }
    if !cpu.feat.is_empty() {
        blog!(gfx, "[ CPU  ]", "", "Features: {}", cpu.feat.as_str().trim());
    }
    if cpu.has_hv {
        blog!(gfx, "[ CPU  ]", "info", "Virtual machine: {}", cpu.hv.as_str().trim());
    }

    if hw.have_mmap {
        blog!(gfx, "[ MEM  ]", "OK", "Usable RAM: {} MB", hw.usable_kb / 1024);
    }
    if hw.have_mem {
        blog!(gfx, "[ MEM  ]", "", "BIOS memory: {} KB low, {} KB high", hw.mem_lower, hw.mem_upper);
    }
    if smb.found {
        for i in 0..smb.dimm_count {
            if smb.dimm_speed[i] > 0 {
                blog!(gfx, "[ MEM  ]", "", "Module {}: {} MB @ {} MT/s", i + 1, smb.dimm_mb[i], smb.dimm_speed[i]);
            } else {
                blog!(gfx, "[ MEM  ]", "", "Module {}: {} MB", i + 1, smb.dimm_mb[i]);
            }
        }
    }

    let (sw, sh) = (gfx.width, gfx.height);
    blog!(gfx, "[ DISP ]", "OK", "Framebuffer: {}x{} {} bpp at {:#x}", sw, sh, fb_bpp, fb_addr);

    if hw.have_acpi {
        let mut oem = Txt::new();
        for i in 0..6 {
            let c = hw.acpi_oem[i];
            if c >= 0x20 && c < 0x7F { oem.push(c); }
        }
        blog!(gfx, "[ ACPI ]", "OK", "OEM: {}   Revision: {}", oem.as_str().trim(), hw.acpi_rev);
    }
    if hw.have_bdev {
        blog!(gfx, "[ BIOS ]", "", "Boot device: {:#x}", hw.bios_dev);
    }

    blog!(gfx, "[ HW   ]", "OK", "Hardware detection complete.");

    boot_wait_ms(hz, 3500);
}

static mut ATA_DRIVE: AtaDrive = AtaDrive::new();
static mut ATA_MOUNTED: bool = false;
static mut ATA_CURRENT_DIR: [u8; 256] = [0; 256];
static mut ATA_DIR_ENTRIES: [[u8; 32]; 64] = [[0; 32]; 64];
static mut ATA_DIR_COUNT: usize = 0;
static mut ATA_FILE_DATA: [[u8; 512]; 16] = [[0; 512]; 16];
static mut ATA_FILE_SIZES: [usize; 16] = [0; 16];
static mut ATA_FILE_COUNT: usize = 0;

static mut GL_WINDOW_REQUESTED: bool = false;
static mut GL_WINDOW_CLOSE_REQUESTED: bool = false;
static mut GL_APP_INSTANCE: Option<gl::GlApp> = None;
static mut GL_WINDOW_INDEX: Option<usize> = None;

static mut LAST_GL_UPDATE: usize = 0;
static mut GL_UPDATE_COUNTER: usize = 0;

static mut TASK_WINDOW_REQUESTED: bool = false;
static mut TASK_WINDOW_CLOSE_REQUESTED: bool = false;
static mut TASK_WINDOW_INDEX: Option<usize> = None;

static mut CRE_TOOL_WINDOW_REQUESTED: bool = false;
static mut CRE_TOOL_WINDOW_CLOSE_REQUESTED: bool = false;
static mut CRE_TOOL_WINDOW_INDEX: Option<usize> = None;

static mut CLOCK_WINDOW_REQUESTED: bool = false;
static mut CLOCK_WINDOW_INDEX: Option<usize> = None;

static mut HTML_WINDOW_REQUESTED: bool = false;
static mut HTML_WINDOW_CLOSE_REQUESTED: bool = false;
static mut HTML_WINDOW_INDEX: Option<usize> = None;
static mut HTML_APP_INSTANCE: Option<html::HtmlApp> = None;

static mut SUPPORT_WINDOW_REQUESTED: bool = false;
static mut SUPPORT_WINDOW_CLOSE_REQUESTED: bool = false;
static mut SUPPORT_WINDOW_INDEX: Option<usize> = None;
static mut SUPPORT_APP_INSTANCE: Option<support::SupportApp> = None;

static mut PCINF_WINDOW_REQUESTED: bool = false;
static mut PCINF_WINDOW_CLOSE_REQUESTED: bool = false;
static mut PCINF_WINDOW_INDEX: Option<usize> = None;
static mut PCINF_APP_INSTANCE: Option<pcinf::PcInfoApp> = None;

static mut MEDIA_WINDOW_REQUESTED: bool = false;
static mut MEDIA_WINDOW_INDEX: Option<usize> = None;
static mut MEDIA_BROWSER_OPEN: bool = false;
static mut MEDIA_IMAGE_INDEX: usize = 0;
static mut MEDIA_LOADED_IMAGE: Option<BmpImage> = None;
static mut MEDIA_NEEDS_REDRAW: bool = false;

const MEDIA_BMP_COUNT: usize = 15;

#[link_section = ".rodata"]
#[used]
static IMG_BMP: &[u8] = include_bytes!("img.bmp");
#[link_section = ".rodata"]
#[used]
static IMG2_BMP: &[u8] = include_bytes!("img2.bmp");
#[link_section = ".rodata"]
#[used]
static IMG3_BMP: &[u8] = include_bytes!("img3.bmp");
#[link_section = ".rodata"]
#[used]
static IC_BMP: &[u8] = include_bytes!("ic.bmp");
#[link_section = ".rodata"]
#[used]
static IC0_BMP: &[u8] = include_bytes!("ic0.bmp");
#[link_section = ".rodata"]
#[used]
static IC1_BMP: &[u8] = include_bytes!("ic1.bmp");
#[link_section = ".rodata"]
#[used]
static IC2_BMP: &[u8] = include_bytes!("ic2.bmp");
#[link_section = ".rodata"]
#[used]
static IC3_BMP: &[u8] = include_bytes!("ic3.bmp");
#[link_section = ".rodata"]
#[used]
static IC4_BMP: &[u8] = include_bytes!("ic4.bmp");
#[link_section = ".rodata"]
#[used]
static IC5_BMP: &[u8] = include_bytes!("ic5.bmp");
#[link_section = ".rodata"]
#[used]
static ABOUT_BMP: &[u8] = include_bytes!("about.bmp");
#[link_section = ".rodata"]
#[used]
static HDD_BMP: &[u8] = include_bytes!("harddrive.bmp");
#[link_section = ".rodata"]
#[used]
static FOLDER_BMP: &[u8] = include_bytes!("folder.bmp");
#[link_section = ".rodata"]
#[used]
static FILE_BMP: &[u8] = include_bytes!("file.bmp");
#[link_section = ".rodata"]
#[used]
static TOOL_BMP: &[u8] = include_bytes!("tool.bmp");
#[link_section = ".rodata"]
#[used]
static SK_BMP: &[u8] = include_bytes!("sk.bmp");
#[link_section = ".rodata"]
#[used]
static LOGIN_BMP: &[u8] = include_bytes!("login.bmp");

fn media_bmp_data(idx: usize) -> (&'static str, &'static [u8]) {
    match idx {
        0 => ("img.bmp", IMG_BMP),
        1 => ("img2.bmp", IMG2_BMP),
        2 => ("img3.bmp", IMG3_BMP),
        3 => ("ic.bmp", IC_BMP),
        4 => ("ic0.bmp", IC0_BMP),
        5 => ("ic1.bmp", IC1_BMP),
        6 => ("ic2.bmp", IC2_BMP),
        7 => ("ic3.bmp", IC3_BMP),
        8 => ("ic4.bmp", IC4_BMP),
        9 => ("ic5.bmp", IC5_BMP),
        10 => ("about.bmp", ABOUT_BMP),
        11 => ("folder.bmp", FOLDER_BMP),
        12 => ("file.bmp", FILE_BMP),
        13 => ("tool.bmp", TOOL_BMP),
        14 => ("login.bmp", LOGIN_BMP),
        _ => ("?", IMG_BMP),
    }
}

static mut WALLPAPER: Color = Color::rgb(5, 148, 0);
static mut USE_BMP_WALLPAPER: bool = true;
static mut BMP_IMAGE: Option<BmpImage> = None;
static mut BMP_IMAGE2: Option<BmpImage> = None;
static mut BMP_IMAGE3: Option<BmpImage> = None;
static mut CURRENT_BMP_INDEX: usize = 0;

const DWP_MAX: usize = 64;
const DWP_ROWS: usize = 8;
const DWP_MAX_BYTES: u32 = 16 * 1024 * 1024;
static mut DISK_WP: Option<BmpImage> = None;
static mut DISK_WP_BUF: Option<alloc::vec::Vec<u8>> = None;
static mut DWP_LIST: [(u32, fs::fat32::FsEntry); DWP_MAX] = [(0, fs::fat32::FsEntry::EMPTY); DWP_MAX];
static mut DWP_COUNT: usize = 0;
static mut DWP_PAGE: usize = 0;
static mut DWP_SYS: u32 = 0;
static mut DWP_STATUS: &str = "";
static mut DWP_STATUS_OK: bool = true;

fn disk_wp_scan() {
    unsafe {
        DWP_COUNT = 0;
        DWP_PAGE = 0;
        DWP_SYS = 0;
        DWP_STATUS = "";
        DWP_STATUS_OK = true;
        if !fs::fat32::ensure_mounted() {
            DWP_STATUS = "No FAT32 disk";
            DWP_STATUS_OK = false;
            return;
        }
        let fsr = match fs::fat32::fs_ref() {
            Some(f) => f,
            None => {
                DWP_STATUS = "No FAT32 disk";
                DWP_STATUS_OK = false;
                return;
            }
        };
        let mut dirs = [fsr.root(), 0u32];
        let mut nd = 1usize;
        if let Some(c) = fs::fat32::sys_dir_cluster(fs::fat32::SYS_DIR) {
            if c != dirs[0] {
                dirs[1] = c;
                DWP_SYS = c;
                nd = 2;
            }
        }
        let mut tmp = [fs::fat32::FsEntry::EMPTY; 64];
        for k in 0..nd {
            let n = fsr.list_dir(dirs[k], &mut tmp);
            for i in 0..n {
                let e = tmp[i];
                if e.is_dir || e.size < 54 || e.size > DWP_MAX_BYTES { continue; }
                if e.name_len < 5 { continue; }
                let ext = &e.name[e.name_len - 4..e.name_len];
                if !ext.eq_ignore_ascii_case(b".BMP") { continue; }
                if DWP_COUNT >= DWP_MAX { break; }
                DWP_LIST[DWP_COUNT] = (dirs[k], e);
                DWP_COUNT += 1;
            }
        }
        if DWP_COUNT == 0 {
            DWP_STATUS = "No .BMP files found";
            DWP_STATUS_OK = false;
        }
    }
}

fn disk_wp_apply(idx: usize) -> bool {
    unsafe {
        if idx >= DWP_COUNT { return false; }
        let (dir, e) = DWP_LIST[idx];
        let fsr = match fs::fat32::fs_ref() {
            Some(f) => f,
            None => {
                DWP_STATUS = "No FAT32 disk";
                DWP_STATUS_OK = false;
                return false;
            }
        };
        let name = core::str::from_utf8(&e.name[..e.name_len]).unwrap_or("");
        let short = match fs::fat32::make_short_name(name) {
            Some(n) => n,
            None => {
                DWP_STATUS = "Bad file name";
                DWP_STATUS_OK = false;
                return false;
            }
        };
        let size = e.size as usize;
        let mut v: alloc::vec::Vec<u8> = alloc::vec::Vec::new();
        if v.try_reserve_exact(size).is_err() {
            DWP_STATUS = "Not enough memory";
            DWP_STATUS_OK = false;
            return false;
        }
        v.resize(size, 0);
        match fsr.read_file_bytes_in(dir, &short, &mut v[..]) {
            Ok(n) => v.truncate(n),
            Err(er) => {
                DWP_STATUS = er;
                DWP_STATUS_OK = false;
                return false;
            }
        }
        if v.len() < 54 || v[0] != b'B' || v[1] != b'M' {
            DWP_STATUS = "Not a BMP file";
            DWP_STATUS_OK = false;
            return false;
        }
        let pix_off = u32::from_le_bytes([v[10], v[11], v[12], v[13]]) as usize;
        let w = u32::from_le_bytes([v[18], v[19], v[20], v[21]]) as usize;
        let h = u32::from_le_bytes([v[22], v[23], v[24], v[25]]) as usize;
        let bpp = u16::from_le_bytes([v[28], v[29]]);
        let compr = u32::from_le_bytes([v[30], v[31], v[32], v[33]]);
        if bpp != 24 || compr != 0 {
            DWP_STATUS = "Only 24-bit BMP";
            DWP_STATUS_OK = false;
            return false;
        }
        if w == 0 || h == 0 || w > 8192 || h > 8192 || pix_off < 54 || pix_off > v.len() {
            DWP_STATUS = "Bad BMP size";
            DWP_STATUS_OK = false;
            return false;
        }
        let row = ((w * 3 + 3) / 4) * 4;
        if v.len() - pix_off < row * h {
            DWP_STATUS = "BMP is damaged";
            DWP_STATUS_OK = false;
            return false;
        }
        let sl: &'static [u8] = core::slice::from_raw_parts(v.as_ptr(), v.len());
        let img = match BmpImage::from_bytes(sl) {
            Some(i) => i,
            None => {
                DWP_STATUS = "Only 24-bit BMP";
                DWP_STATUS_OK = false;
                return false;
            }
        };
        DISK_WP = None;
        DISK_WP_BUF = Some(v);
        DISK_WP = Some(img);
        CURRENT_BMP_INDEX = 3;
        USE_BMP_WALLPAPER = true;
        DWP_STATUS = "";
        DWP_STATUS_OK = true;
        true
    }
}

static mut LIVE_WALLPAPER_ENABLED: bool = false;
static mut LIVE_WALLPAPER_INTERVAL_SEC: u32 = 15;
static mut LIVE_WALLPAPER_ELAPSED_SEC: u32 = 0;
static mut RNG_STATE: u32 = 0x2545F491;
const LIVE_WALLPAPER_MIN_SEC: u32 = 7;
const LIVE_WALLPAPER_MAX_SEC: u32 = 60;

static mut LOGIN_IMAGE: Option<BmpImage> = None;

static mut ICON_TERMINAL: Option<BmpImage> = None;
static mut ICON_FILES: Option<BmpImage> = None;
static mut ICON_SETTINGS: Option<BmpImage> = None;
static mut ICON_ABOUT: Option<BmpImage> = None;
static mut ICON_NOTEPAD: Option<BmpImage> = None;
static mut ICON_SNAKE: Option<BmpImage> = None;
static mut ICON_PAINT: Option<BmpImage> = None;
static mut ICON_FOLDER: Option<BmpImage> = None;
static mut ICON_HDD: Option<BmpImage> = None;
static mut ICON_FILE: Option<BmpImage> = None;
static mut ICON_TOOL: Option<BmpImage> = None;
static mut ICON_SHORTCUT: Option<BmpImage> = None;
static mut ABOUT_IMAGE: Option<BmpImage> = None;

static mut THEME_IDX: usize = 0;
static mut SYS_ICON_POS: [u32; 9] = [u32::MAX; 9];
static mut FOLDER_ICON_POS: [u32; 10] = [u32::MAX; 10];
static mut ICON_LIM: (usize, usize) = (4096, 4096);
static mut ICON_DRAG: usize = usize::MAX;
static mut ICON_DRAG_MOVED: bool = false;
static mut ICON_DRAG_START: (usize, usize) = (0, 0);
static mut ICON_DRAG_OFF: (usize, usize) = (0, 0);
static mut ICON_DRAG_POS: (usize, usize) = (0, 0);
static mut USE_WM2_THEME: bool = false;
static mut TERM_HISTORY: [[u8; 80]; 1000] = [[b' '; 80]; 1000];
static mut TERM_HIST_LEN: usize = 0;
static mut TERM_SCROLL: usize = 0;
static mut NOTEPAD_TEXT: [u8; 1024] = [0; 1024];
static mut NOTEPAD_LEN: usize = 0;
static mut NOTEPAD_STATUS: [u8; 48] = [0; 48];
static mut NOTEPAD_STATUS_LEN: usize = 0;
static mut NOTEPAD_STATUS_OK: bool = true;
static mut SAVED_DOCUMENTS: [[u8; 512]; 5] = [[0; 512]; 5];
static mut SAVED_DOCUMENTS_LEN: [usize; 5] = [0; 5];
static mut SAVED_DOCUMENTS_COUNT: usize = 0;
static mut WALLPAPER_SELECTOR_OPEN: bool = false;
static mut THEME_SELECTOR_OPEN: bool = false;
static mut DOCUMENT_MANAGER_OPEN: bool = false;
static mut DOCUMENT_MANAGER_MODE: u8 = 0;
static mut GRADIENT_TOP_ACTIVE: Color = Color::rgb(90, 90, 90);
static mut GRADIENT_BOTTOM_ACTIVE: Color = Color::rgb(20, 20, 20);
static mut GRADIENT_TOP_INACTIVE: Color = Color::rgb(105, 105, 105);
static mut GRADIENT_BOTTOM_INACTIVE: Color = Color::rgb(50, 50, 50);
static mut LOGGED_IN: bool = false;

static mut GUI_MODE: bool = false;
static mut GUI_START_REQUESTED: bool = false;
static mut GUI_STOP_REQUESTED: bool = false;
static mut NOTEPAD_WINDOW_REQUESTED: bool = false;
static mut POWER_REQUEST: bool = false;
const BARE_DESKTOP: bool = true;
const OFF_SCREEN: usize = 100_000;

fn draw_text_console(gfx: &mut Graphics) {
    let bg = Color::rgb(0, 0, 0).to_u32();
    let fg = Color::rgb(204, 204, 204).to_u32();
    gfx.clear(bg);

    let rows = gfx.height.saturating_sub(16) / 16;
    if rows == 0 { return; }
    let max_chars = gfx.width.saturating_sub(16) / 8;

    let mut live: alloc::vec::Vec<alloc::string::String> = alloc::vec::Vec::new();
    unsafe {
        if !LOGGED_IN {
            let u = core::str::from_utf8(&LOGIN_USERNAME_INPUT[..LOGIN_USERNAME_LEN]).unwrap_or("");
            live.push(alloc::format!("miux login: {}", u));
            if LOGIN_IS_PASSWORD {
                live.push(alloc::string::String::from("Password: "));
            }
        } else {
            let t = core::str::from_utf8(&TERM_INPUT_GLOBAL[..TERM_LEN_GLOBAL]).unwrap_or("");
            live.push(alloc::format!("{}{}", termfs::prompt(), t));
        }
    }

    let hist_len = unsafe { TERM_HIST_LEN };
    let scroll = unsafe {
        let max_scroll = hist_len.saturating_sub(rows);
        if TERM_SCROLL > max_scroll { TERM_SCROLL = max_scroll; }
        TERM_SCROLL
    };
    let show_live = scroll == 0;
    let live_n = if show_live { live.len() } else { 0 };
    let hist_rows = rows.saturating_sub(live_n);
    let end_row = hist_len - scroll;
    let start_row = end_row.saturating_sub(hist_rows);

    let mut y = 8usize;
    unsafe {
        for row in start_row..end_row {
            let h = &TERM_HISTORY[row];
            if h[0] == 0x0E {
                let k = h[1].wrapping_sub(b'0') as usize;
                tech_draw_slice(gfx, 8, y, k);
                let txt = core::str::from_utf8(&h[2..]).unwrap_or("").trim_end_matches('\0');
                let tmax = max_chars.saturating_sub((TECH_SZ + 16) / 8);
                let tshow = match txt.char_indices().nth(tmax) {
                    Some((b, _)) => &txt[..b],
                    None => txt,
                };
                gfx.draw_text(8 + TECH_SZ + 16, y, tshow, fg, bg);
                y += 16;
                continue;
            }
            let line = core::str::from_utf8(&h[..]).unwrap_or("").trim_end_matches('\0');
            let shown = match line.char_indices().nth(max_chars) {
                Some((b, _)) => &line[..b],
                None => line,
            };
            gfx.draw_text(8, y, shown, fg, bg);
            y += 16;
        }
    }
    if show_live {
        let n = live.len();
        for (i, l) in live.iter().enumerate() {
            let mut t = l.clone();
            if i + 1 == n { t.push('_'); }
            let shown = match t.char_indices().nth(max_chars) {
                Some((b, _)) => &t[..b],
                None => &t[..],
            };
            gfx.draw_text(8, y, shown, fg, bg);
            y += 16;
        }
    }
}

fn on_window_closed(removed: usize) {
    fn fix(slot: &mut Option<usize>, removed: usize) {
        match *slot {
            Some(i) if i == removed => *slot = None,
            Some(i) if i > removed => *slot = Some(i - 1),
            _ => {}
        }
    }
    unsafe {
        for slot in [
            &mut *core::ptr::addr_of_mut!(EXT4_WINDOW_INDEX),
            &mut *core::ptr::addr_of_mut!(GL_WINDOW_INDEX),
            &mut *core::ptr::addr_of_mut!(TASK_WINDOW_INDEX),
            &mut *core::ptr::addr_of_mut!(CRE_TOOL_WINDOW_INDEX),
            &mut *core::ptr::addr_of_mut!(CLOCK_WINDOW_INDEX),
            &mut *core::ptr::addr_of_mut!(HTML_WINDOW_INDEX),
            &mut *core::ptr::addr_of_mut!(SUPPORT_WINDOW_INDEX),
            &mut *core::ptr::addr_of_mut!(PCINF_WINDOW_INDEX),
            &mut *core::ptr::addr_of_mut!(MEDIA_WINDOW_INDEX),
            &mut *core::ptr::addr_of_mut!(calend::CALENDAR_WINDOW_INDEX),
        ] {
            fix(slot, removed);
        }
        let mut p = Some(cre_tool::get_preview_window());
        fix(&mut p, removed);
        cre_tool::set_preview_window(p.unwrap_or(usize::MAX));
        let mut p = Some(PREVIEW_WINDOW_INDEX);
        fix(&mut p, removed);
        PREVIEW_WINDOW_INDEX = p.unwrap_or(usize::MAX);
    }
}

static mut USER_LOGIN_ATTEMPTS: usize = 0;
static mut USER_LOGIN_LOCKED: bool = false;

static mut LOGIN_USERNAME_INPUT: [u8; 32] = [0; 32];
static mut LOGIN_USERNAME_LEN: usize = 0;
static mut LOGIN_PASSWORD_INPUT: [u8; 32] = [0; 32];
static mut LOGIN_PASSWORD_LEN: usize = 0;
static mut LOGIN_IS_PASSWORD: bool = false;
static mut LOGIN_ERROR_MSG: [u8; 64] = [0; 64];
static mut LOGIN_ERROR_LEN: usize = 0;

static mut PAINT_CANVAS: [[u32; 400]; 300] = [[0; 400]; 300];
static mut PAINT_COLOR: u32 = 0x000000;
static mut PAINT_PEN_SIZE: usize = 1;
static mut PAINT_TOOL: u8 = 0;
static mut PAINT_PREV_X: usize = 0;
static mut PAINT_PREV_Y: usize = 0;
static mut PAINT_IS_DRAWING: bool = false;
static mut PAINT_RECT_START_X: usize = 0;
static mut PAINT_RECT_START_Y: usize = 0;
static mut PAINT_RECT_END_X: usize = 0;
static mut PAINT_RECT_END_Y: usize = 0;
static mut PAINT_CANVAS_BACKUP: [[u32; 400]; 300] = [[0; 400]; 300];

static mut SAVED_PAINTINGS: [[[u32; 400]; 300]; 5] = [[[0; 400]; 300]; 5];
static mut SAVED_PAINTINGS_COUNT: usize = 0;

static mut CURRENT_FS_SECTION: usize = 0;
static mut CONTEXT_MENU_OPEN: bool = false;
static mut CONTEXT_MENU_X: usize = 0;
static mut CONTEXT_MENU_Y: usize = 0;

static mut NAME_INPUT_OPEN: bool = false;
static mut NAME_INPUT_BUFFER: [u8; 64] = [0; 64];
static mut NAME_INPUT_LEN: usize = 0;
static mut NAME_INPUT_CALLBACK: Option<fn(&str)> = None;
static mut NAME_INPUT_X: usize = 0;
static mut NAME_INPUT_Y: usize = 0;
static mut NAME_INPUT_MODE: u8 = 0;
static mut NAME_INPUT_FOLDER_INDEX: usize = 0;

static mut DESKTOP_FOLDERS: [[u8; 64]; 10] = [[0; 64]; 10];
static mut DESKTOP_FOLDERS_COUNT: usize = 0;
static mut FOLDER_FILES: [[[u8; 64]; 20]; 10] = [[[0; 64]; 20]; 10];
static mut FOLDER_FILES_COUNT: [usize; 10] = [0; 10];
static mut FOLDER_FILE_CONTENTS: [[[u8; 1024]; 20]; 10] = [[[0; 1024]; 20]; 10];
static mut FOLDER_FILE_LENS: [[usize; 20]; 10] = [[0; 20]; 10];

static mut TUTORIAL_OPEN: bool = false;
static mut TUTORIAL_STEP: usize = 0;

static mut APP_RESULT: [u8; 256] = [0; 256];

static mut GUI_ELEMENTS: [apps::GuiElement; 16] = [apps::GuiElement {
    x: 0, y: 0, w: 0, h: 0,
    kind: 0,
    text: [0; 64],
    text_len: 0,
    visible: false,
}; 16];
static mut GUI_ELEMENTS_COUNT: usize = 0;
static mut SELECTED_ELEMENT: usize = 0;
static mut DRAG_STATE: u8 = 0;
static mut DRAG_OFFSET_X: i32 = 0;
static mut DRAG_OFFSET_Y: i32 = 0;
static mut APP_PREVIEW_RUNNING: bool = false;

static mut PREVIEW_ELEMENTS: [apps::GuiElement; 16] = [apps::GuiElement {
    x: 0, y: 0, w: 0, h: 0,
    kind: 0,
    text: [0; 64],
    text_len: 0,
    visible: false,
}; 16];
static mut PREVIEW_ELEMENTS_COUNT: usize = 0;
static mut PREVIEW_WINDOW_INDEX: usize = 0;
static mut PREVIEW_RUNNING: bool = false;

static mut FAT32_MOUNTED: [bool; 8] = [false; 8];
static mut FAT32_FILESYSTEMS: [Option<fs::fat32::FAT32FS>; 8] = [None, None, None, None, None, None, None, None];
static mut DISK_COUNT: usize = 0;
static mut FILE_MANAGER_DISK_LIST: [u8; 2048] = [0; 2048];
static mut FILE_MANAGER_DISK_LIST_LEN: usize = 0;
static mut FILE_MANAGER_DIALOG_OPEN: bool = false;
static mut FILE_MANAGER_DIALOG_DISK: usize = 0;
static mut FILE_MANAGER_MOUNTED: bool = false;
static mut FILE_MANAGER_FILES: [u8; 8192] = [0; 8192];
static mut FILE_MANAGER_FILES_LEN: usize = 0;

static mut TERM_INPUT_GLOBAL: [u8; 256] = [0; 256];
static mut TERM_LEN_GLOBAL: usize = 0;


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

fn get_time() -> (u8, u8, u8) {
    unsafe {
        loop {
            let status = cmos_read(0x0A);
            if status & 0x80 == 0 { break; }
        }

        let mut sec = cmos_read(0x00);
        let mut min = cmos_read(0x02);
        let mut hour = cmos_read(0x04);

        let reg_b = cmos_read(0x0B);
        let bcd = reg_b & 0x04 == 0;

        if bcd {
            sec = bcd_to_bin(sec);
            min = bcd_to_bin(min);
            hour = bcd_to_bin(hour);
        }

        (hour, min, sec)
    }
}

fn next_random_u32() -> u32 {
    unsafe {
        let (h, m, s) = get_time();
        RNG_STATE ^= ((h as u32) << 16) | ((m as u32) << 8) | (s as u32) | 1;
        RNG_STATE ^= RNG_STATE << 13;
        RNG_STATE ^= RNG_STATE >> 17;
        RNG_STATE ^= RNG_STATE << 5;
        if RNG_STATE == 0 { RNG_STATE = 0x2545F491; }
        RNG_STATE
    }
}

fn pick_random_bmp_index(current: usize) -> usize {
    let mut idx = (next_random_u32() % 3) as usize;
    if idx == current {
        idx = (idx + 1) % 3;
    }
    idx
}

fn time_to_str(h: u8, m: u8, s: u8) -> &'static str {
    static mut BUF: [u8; 9] = [0; 9];
    unsafe {
        BUF[0] = b'0' + (h / 10);
        BUF[1] = b'0' + (h % 10);
        BUF[2] = b':';
        BUF[3] = b'0' + (m / 10);
        BUF[4] = b'0' + (m % 10);
        BUF[5] = b':';
        BUF[6] = b'0' + (s / 10);
        BUF[7] = b'0' + (s % 10);
        BUF[8] = 0;
        core::str::from_utf8(&BUF[..8]).unwrap_or("00:00:00")
    }
}

fn draw_big_char(
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
                let px = x + col * scale;
                let py = y + row * scale;
                gfx.fill_rect(px, py, scale, scale, color);
            }
        }
    }
}


fn draw_line_on_canvas(x1: usize, y1: usize, x2: usize, y2: usize, color: u32) {
    unsafe {
        let mut x = x1 as i32;
        let mut y = y1 as i32;
        let dx = (x2 as i32 - x1 as i32).abs();
        let dy = -(y2 as i32 - y1 as i32).abs();
        let sx = if x1 < x2 { 1 } else { -1 };
        let sy = if y1 < y2 { 1 } else { -1 };
        let mut err = dx + dy;
        loop {
            if x >= 0 && x < 400 && y >= 0 && y < 300 {
                PAINT_CANVAS[y as usize][x as usize] = color;
            }
            if x == x2 as i32 && y == y2 as i32 { break; }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }
}

fn draw_rect_on_canvas(x1: usize, y1: usize, x2: usize, y2: usize, color: u32) {
    unsafe {
        let start_x = if x1 < x2 { x1 } else { x2 };
        let end_x = if x1 > x2 { x1 } else { x2 };
        let start_y = if y1 < y2 { y1 } else { y2 };
        let end_y = if y1 > y2 { y1 } else { y2 };

        for y in start_y..=end_y {
            for x in start_x..=end_x {
                if x < 400 && y < 300 {
                    PAINT_CANVAS[y][x] = color;
                }
            }
        }
    }
}

fn mount_ata_drive(vga: &mut crate::vga::VGA) {
    unsafe {
        let mut drive = AtaDrive::new();
        drive.init(vga);

        if drive.exists {
            ATA_DRIVE = drive;
            ATA_MOUNTED = true;
            vga.write_string("[ATA] Drive mounted successfully!\n");
            vga.write_string("[ATA] Model: ");
            for i in 0..40 {
                if ATA_DRIVE.model[i] != 0 && ATA_DRIVE.model[i] != 0x20 {
                    vga.put_char(ATA_DRIVE.model[i] as char);
                }
            }
            vga.write_string("\n[ATA] Sectors: ");
            vga.write_number(ATA_DRIVE.sectors as usize);
            vga.write_string("\n");

            let mut buffer = [0u8; 512];
            if ATA_DRIVE.read_sector(1, &mut buffer) {
                parse_ata_directory(&buffer);
            }
        } else {
            vga.write_string("[ATA] No ATA drive detected!\n");
        }
    }
}

fn parse_ata_directory(data: &[u8; 512]) {
    unsafe {
        ATA_DIR_COUNT = 0;
        let mut offset = 0;

        while offset < 512 && ATA_DIR_COUNT < 64 {
            if data[offset] != 0 {
                let name_len = data[offset] as usize;
                if name_len > 0 && name_len <= 31 {
                    let start = offset + 1;
                    for i in 0..name_len {
                        if start + i < 512 {
                            ATA_DIR_ENTRIES[ATA_DIR_COUNT][i] = data[start + i];
                        }
                    }
                    ATA_DIR_COUNT += 1;
                    offset += 1 + name_len + 4;
                } else {
                    offset += 1;
                }
            } else {
                offset += 1;
            }
        }
    }
}

fn read_ata_file(filename: &str, buffer: &mut [u8]) -> usize {
    unsafe {
        if !ATA_MOUNTED { return 0; }

        for i in 0..ATA_DIR_COUNT {
            let entry_name = str::from_utf8(&ATA_DIR_ENTRIES[i]).unwrap_or("");
            let entry_name_trimmed = entry_name.trim_end_matches('\0');

            if entry_name_trimmed == filename {
                let mut sector_buf = [0u8; 512];
                if ATA_DRIVE.read_sector(2 + i as u32, &mut sector_buf) {
                    let size = sector_buf[0] as usize;
                    let copy_len = if size > buffer.len() { buffer.len() } else { size };
                    buffer[..copy_len].copy_from_slice(&sector_buf[1..1 + copy_len]);
                    return copy_len;
                }
                return 0;
            }
        }
        0
    }
}

fn write_ata_file(filename: &str, data: &[u8]) -> bool {
    unsafe {
        if !ATA_MOUNTED { return false; }

        for i in 0..ATA_DIR_COUNT {
            let entry_name = str::from_utf8(&ATA_DIR_ENTRIES[i]).unwrap_or("");
            if entry_name.trim_end_matches('\0') == filename {
                let mut sector_buf = [0u8; 512];
                let size = if data.len() > 511 { 511 } else { data.len() };
                sector_buf[0] = size as u8;
                sector_buf[1..1 + size].copy_from_slice(&data[..size]);
                return ATA_DRIVE.write_sector(2 + i as u32, &sector_buf);
            }
        }

        if ATA_DIR_COUNT < 64 {
            let name_bytes = filename.as_bytes();
            let name_len = if name_bytes.len() > 31 { 31 } else { name_bytes.len() };
            ATA_DIR_ENTRIES[ATA_DIR_COUNT][..name_len].copy_from_slice(&name_bytes[..name_len]);
            ATA_DIR_COUNT += 1;

            let mut dir_buf = [0u8; 512];
            let mut offset = 0;
            for i in 0..ATA_DIR_COUNT {
                let name_len = str::from_utf8(&ATA_DIR_ENTRIES[i]).unwrap_or("").trim_end_matches('\0').len();
                dir_buf[offset] = name_len as u8;
                offset += 1;
                for j in 0..name_len {
                    dir_buf[offset + j] = ATA_DIR_ENTRIES[i][j];
                }
                offset += name_len;
                dir_buf[offset] = 0;
                dir_buf[offset + 1] = 0;
                dir_buf[offset + 2] = 0;
                dir_buf[offset + 3] = 0;
                offset += 4;
            }
            ATA_DRIVE.write_sector(1, &dir_buf);

            let mut file_buf = [0u8; 512];
            let size = if data.len() > 511 { 511 } else { data.len() };
            file_buf[0] = size as u8;
            file_buf[1..1 + size].copy_from_slice(&data[..size]);
            return ATA_DRIVE.write_sector(2 + ATA_DIR_COUNT as u32 - 1, &file_buf);
        }
        false
    }
}

fn list_ata_files() -> &'static str {
    unsafe {
        if !ATA_MOUNTED {
            return "ATA drive not mounted!";
        }

        static mut RESULT: [u8; 512] = [0; 512];
        let mut pos = 0;

        let header = b"Files on SATA drive:\n";
        for i in 0..header.len() {
            if pos < 511 { RESULT[pos] = header[i]; pos += 1; }
        }

        for i in 0..ATA_DIR_COUNT {
            let name = core::str::from_utf8(&ATA_DIR_ENTRIES[i]).unwrap_or("");
            let name_trimmed = name.trim_end_matches('\0');
            for b in name_trimmed.bytes() {
                if pos < 511 { RESULT[pos] = b; pos += 1; }
            }
            if pos < 511 { RESULT[pos] = b'\n'; pos += 1; }
        }

        RESULT[pos] = 0;
        core::str::from_utf8(&RESULT[..pos]).unwrap_or("")
    }
}

fn draw_icon(gfx: &mut Graphics, x: usize, y: usize, icon: &Option<BmpImage>, size: usize) {
    if let Some(ref bmp) = icon {
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

fn clip_str(s: &str, max: usize) -> &str {
    if s.len() <= max { return s; }
    let mut n = max;
    while n > 0 && !s.is_char_boundary(n) { n -= 1; }
    &s[..n]
}

static TECH_PNG: &[u8] = include_bytes!("systemx64/tech_logo.png");
const TECH_SZ: usize = 192;
static mut TECH_IMG: Option<alloc::vec::Vec<u32>> = None;
static mut TECH_CPU: Txt = Txt::new();
static mut TECH_HOST: Txt = Txt::new();
static mut TECH_RAM_MB: u64 = 0;
static mut TECH_CORES: u32 = 0;

fn tech_build_img() -> Option<alloc::vec::Vec<u32>> {
    let src = web::image::decode(TECH_PNG)?;
    let sw = src.w as usize;
    let sh = src.h as usize;
    if sw == 0 || sh == 0 || src.px.len() < sw * sh { return None; }
    let mut out = alloc::vec::Vec::with_capacity(TECH_SZ * TECH_SZ);
    for dy in 0..TECH_SZ {
        let y0 = dy * sh / TECH_SZ;
        let mut y1 = (dy + 1) * sh / TECH_SZ;
        if y1 <= y0 { y1 = y0 + 1; }
        for dx in 0..TECH_SZ {
            let x0 = dx * sw / TECH_SZ;
            let mut x1 = (dx + 1) * sw / TECH_SZ;
            if x1 <= x0 { x1 = x0 + 1; }
            let mut r = 0u32;
            let mut g = 0u32;
            let mut b = 0u32;
            let mut n = 0u32;
            for sy in y0..y1.min(sh) {
                for sx in x0..x1.min(sw) {
                    let p = src.px[sy * sw + sx];
                    let a = p >> 24;
                    let pr = (p >> 16) & 255;
                    let pg = (p >> 8) & 255;
                    let pb = p & 255;
                    r += (pr * a + 255 * (255 - a)) / 255;
                    g += (pg * a + 255 * (255 - a)) / 255;
                    b += (pb * a + 255 * (255 - a)) / 255;
                    n += 1;
                }
            }
            if n == 0 { n = 1; }
            out.push(((r / n) << 16) | ((g / n) << 8) | (b / n));
        }
    }
    Some(out)
}

fn tech_draw_slice(gfx: &mut Graphics, x: usize, y: usize, k: usize) {
    unsafe {
        if TECH_IMG.is_none() {
            TECH_IMG = tech_build_img();
        }
        if let Some(ref img) = TECH_IMG {
            for dy in 0..16 {
                let sy = k * 16 + dy;
                if sy >= TECH_SZ { break; }
                for dx in 0..TECH_SZ {
                    gfx.put_pixel(x + dx, y + dy, img[sy * TECH_SZ + dx]);
                }
            }
        }
    }
}

fn tech_info_lines() -> alloc::vec::Vec<alloc::string::String> {
    use alloc::format;
    use alloc::string::String;
    let mut v: alloc::vec::Vec<String> = alloc::vec::Vec::new();
    unsafe {
        let user = users::get_current_username();
        v.push(format!("{}@miux", user));
        v.push(String::from("---------------------"));
        v.push(String::from("OS: MiUX 7.0.0"));
        v.push(String::from("Kernel: MiUX 7.0.0 x86_64"));
        v.push(String::from("Desktop: MiDE v5.3"));

        let host = TECH_HOST.as_str().trim();
        v.push(if host.is_empty() { String::from("Host: unknown") } else { format!("Host: {}", host) });

        let cpu = TECH_CPU.as_str().trim();
        let cpu_name = if cpu.is_empty() { "unknown" } else { cpu };
        if TECH_CORES > 0 {
            v.push(format!("CPU: {} ({})", cpu_name, TECH_CORES));
        } else {
            v.push(format!("CPU: {}", cpu_name));
        }

        let heap_used = heap::used() / (1024 * 1024);
        v.push(format!("Memory: {} MB total, heap {} MB used", TECH_RAM_MB, heap_used));

        if let Some(ref vi) = VESA_INFO {
            v.push(format!("Resolution: {}x{} {}bpp", vi.width, vi.height, vi.bpp));
        } else {
            v.push(String::from("Resolution: unknown"));
        }

        let mut disks = 0usize;
        let mut bytes: u64 = 0;
        for i in 0..ata::disk_count() {
            if let Some(d) = ata::get_disk(i) {
                if d.present && d.dev_type == ata::DeviceType::ATA {
                    disks += 1;
                    bytes += d.sectors * (if d.sector_size == 0 { 512 } else { d.sector_size as u64 });
                }
            }
        }
        let gb10 = bytes * 10 / (1024 * 1024 * 1024);
        v.push(format!("Disks: {} ({}.{} GB)", disks, gb10 / 10, gb10 % 10));

        let secs = timer::millis() / 1000;
        v.push(format!("Uptime: {}h {}m {}s", secs / 3600, (secs / 60) % 60, secs % 60));

        match net::status() {
            net::Status::Connected => v.push(format!("Network: connected, {}", net::ip_string())),
            net::Status::Dhcp => v.push(String::from("Network: getting IP (DHCP)")),
            net::Status::Disabled => v.push(String::from("Network: disabled")),
            net::Status::NoCable => v.push(String::from("Network: cable unplugged")),
            net::Status::NoAdapter => v.push(String::from("Network: no adapter")),
        }
    }
    v
}

fn draw_folder_icon(gfx: &mut Graphics, x: usize, y: usize, name: &str) {
    let icon_size = 64;
    let folder_icon = unsafe { &ICON_FOLDER };
    draw_icon(gfx, x, y, folder_icon, icon_size);

    let display_name = clip_str(name, 10);
    let text_x = x + icon_size.saturating_sub(display_name.chars().count() * 8) / 2;
    gfx.draw_text(text_x, y + icon_size + 5, display_name, Color::WHITE.to_u32(), Color::rgb(0, 0, 0).to_u32());
}

fn draw_file_icon(gfx: &mut Graphics, x: usize, y: usize, name: &str) {
    let icon_size = 32;
    let file_icon = unsafe { &ICON_FILE };
    draw_icon(gfx, x, y, file_icon, icon_size);

    let display_name = clip_str(name, 12);
    let text_x = x + icon_size.saturating_sub(display_name.chars().count() * 8) / 2;
    gfx.draw_text(text_x, y + icon_size + 3, display_name, Color::BLACK.to_u32(), Color::rgb(235, 235, 235).to_u32());
}

fn draw_line(gfx: &mut Graphics, x1: usize, y1: usize, x2: usize, y2: usize, color: u32) {
    let mut x = x1 as i32;
    let mut y = y1 as i32;
    let dx = (x2 as i32 - x1 as i32).abs();
    let dy = -(y2 as i32 - y1 as i32).abs();
    let sx = if x1 < x2 { 1 } else { -1 };
    let sy = if y1 < y2 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        if x >= 0 && x < gfx.width as i32 && y >= 0 && y < gfx.height as i32 {
            gfx.put_pixel(x as usize, y as usize, color);
        }
        if x == x2 as i32 && y == y2 as i32 { break; }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

fn draw_context_menu(gfx: &mut Graphics, x: usize, y: usize) {
    let menu_w = 160;
    let menu_h = context_menu_h();
    let desktop_items = ["Terminal", "AboutOS", "Settings", "Create Folder"];
    let fm_items = ["Create Folder", "Open Terminal"];
    let file_items = ["Delete file", "Create shortcut"];
    let shortcut_items = ["Delete"];
    let menu_kind = unsafe { CONTEXT_MENU_KIND };
    let menu_items: &[&str] = if menu_kind == 3 {
        &shortcut_items[..]
    } else if menu_kind == 2 {
        &file_items[..]
    } else if menu_kind == 1 {
        &fm_items[..]
    } else {
        &desktop_items[..]
    };

    let mut menu_x = x;
    let mut menu_y = y;
    if menu_x + menu_w > gfx.width {
        menu_x = gfx.width - menu_w;
    }
    if menu_y + menu_h > gfx.height {
        menu_y = gfx.height - menu_h;
    }

    gfx.fill_rect(menu_x, menu_y, menu_w, menu_h, Color::rgb(240, 240, 240).to_u32());
    gfx.draw_rect_border(menu_x, menu_y, menu_w, menu_h, Color::rgb(87, 87, 87).to_u32());
    gfx.draw_rect_border(menu_x + 1, menu_y + 1, menu_w - 2, menu_h - 2, Color::WHITE.to_u32());

    for (i, item) in menu_items.iter().enumerate() {
        let item_y = menu_y + 4 + i * 24;
        gfx.draw_text(menu_x + 8, item_y + 4, item, Color::BLACK.to_u32(), Color::rgb(240, 240, 240).to_u32());

        if i < menu_items.len() - 1 {
            gfx.fill_rect(menu_x + 4, item_y + 22, menu_w - 8, 1, Color::rgb(200, 200, 200).to_u32());
        }
    }

    unsafe {
        CONTEXT_MENU_OPEN = true;
        CONTEXT_MENU_X = menu_x;
        CONTEXT_MENU_Y = menu_y;
    }
}

fn show_name_input_dialog(gfx: &mut Graphics, callback: fn(&str), mode: u8, folder_index: usize) {
    unsafe {
        NAME_INPUT_OPEN = true;
        NAME_INPUT_BUFFER = [0; 64];
        NAME_INPUT_LEN = 0;
        NAME_INPUT_CALLBACK = Some(callback);
        NAME_INPUT_MODE = mode;
        NAME_INPUT_FOLDER_INDEX = folder_index;

        let screen_w = gfx.width;
        let screen_h = gfx.height;
        NAME_INPUT_X = (screen_w - 300) / 2;
        NAME_INPUT_Y = (screen_h - 150) / 2;
    }
}

fn draw_name_input_dialog(gfx: &mut Graphics) {
    unsafe {
        if !NAME_INPUT_OPEN { return; }
        if NAME_INPUT_MODE == 3 {
            draw_save_dialog(gfx);
            return;
        }

        let dialog_w = 300;
        let dialog_h = 150;
        let dialog_x = NAME_INPUT_X;
        let dialog_y = NAME_INPUT_Y;
        let title = if NAME_INPUT_MODE == 0 { "Create Folder" } else if NAME_INPUT_MODE == 2 { "Load File" } else { "Save File" };

        gfx.fill_rect(dialog_x, dialog_y, dialog_w, dialog_h, Color::rgb(235, 235, 235).to_u32());
        gfx.draw_rect_border(dialog_x, dialog_y, dialog_w, dialog_h, Color::rgb(87, 87, 87).to_u32());
        gfx.draw_rect_border(dialog_x + 1, dialog_y + 1, dialog_w - 2, dialog_h - 2, Color::WHITE.to_u32());

        let title_bar_h = 30;
        gfx.fill_rect(dialog_x + 2, dialog_y + 2, dialog_w - 4, title_bar_h, Color::rgb(90, 90, 90).to_u32());
        gfx.draw_text(dialog_x + 10, dialog_y + 6, title, Color::WHITE.to_u32(), Color::rgb(90, 90, 90).to_u32());

        let prompt = if NAME_INPUT_MODE == 0 { "Folder name:" } else { "File name:" };
        gfx.draw_text(dialog_x + 20, dialog_y + 50, prompt, Color::BLACK.to_u32(), Color::rgb(235, 235, 235).to_u32());

        gfx.fill_rect(dialog_x + 20, dialog_y + 70, 260, 24, Color::WHITE.to_u32());
        gfx.draw_rect_border(dialog_x + 20, dialog_y + 70, 260, 24, Color::rgb(128, 128, 128).to_u32());

        let name_str = core::str::from_utf8(&NAME_INPUT_BUFFER[..NAME_INPUT_LEN]).unwrap_or("");
        gfx.draw_text(dialog_x + 25, dialog_y + 74, name_str, Color::BLACK.to_u32(), Color::WHITE.to_u32());

        wm::draw_raised_rect(gfx, dialog_x + 60, dialog_y + 110, 80, 25);
        gfx.draw_text(dialog_x + 85, dialog_y + 115, "OK", Color::BLACK.to_u32(), Color::rgb(212, 208, 200).to_u32());

        wm::draw_raised_rect(gfx, dialog_x + 160, dialog_y + 110, 80, 25);
        gfx.draw_text(dialog_x + 182, dialog_y + 115, "Cancel", Color::BLACK.to_u32(), Color::rgb(212, 208, 200).to_u32());
    }
}

fn handle_name_input_click(cursor_x: usize, cursor_y: usize) -> bool {
    unsafe {
        if !NAME_INPUT_OPEN { return false; }
        if NAME_INPUT_MODE == 3 {
            return handle_save_dialog_click(cursor_x, cursor_y);
        }

        let dialog_x = NAME_INPUT_X;
        let dialog_y = NAME_INPUT_Y;

        if cursor_x >= dialog_x + 60 && cursor_x <= dialog_x + 140 &&
           cursor_y >= dialog_y + 110 && cursor_y <= dialog_y + 135 {

            if let Some(callback) = NAME_INPUT_CALLBACK {
                let name_str = core::str::from_utf8(&NAME_INPUT_BUFFER[..NAME_INPUT_LEN]).unwrap_or("");
                if name_str.len() > 0 {
                    callback(name_str);
                }
            }
            NAME_INPUT_OPEN = false;
            return true;
        }

        if cursor_x >= dialog_x + 160 && cursor_x <= dialog_x + 240 &&
           cursor_y >= dialog_y + 110 && cursor_y <= dialog_y + 135 {
            NAME_INPUT_OPEN = false;
            return true;
        }

        false
    }
}

fn create_folder_on_desktop(name: &str) {
    let name = name.trim();
    if name.is_empty() { return; }

    unsafe {
        if syscfg::shortcut_count() >= syscfg::MAX_SHORTCUTS { return; }
        let mut shown = [0u8; 13];
        let (disk, len) = match fs::fat32::create_root_dir_unique(name, &mut shown) {
            Ok(x) => x,
            Err(_) => return,
        };
        let mut path = [0u8; syscfg::SHORTCUT_PATH_MAX];
        path[0] = b'/';
        path[1..1 + len].copy_from_slice(&shown[..len]);
        let p = core::str::from_utf8(&path[..1 + len]).unwrap_or("");
        let _ = syscfg::add_folder_shortcut(disk, p);
        if FILE_MANAGER_MOUNTED && FM_VIEW.disk == disk {
            FM_VIEW.reload();
        }
    }
}

fn save_file_to_folder(name: &str) {
    unsafe {
        let folder_index = NAME_INPUT_FOLDER_INDEX;
        if folder_index < DESKTOP_FOLDERS_COUNT {
            let file_count = FOLDER_FILES_COUNT[folder_index];
            if file_count < 20 {
                let name_bytes = name.as_bytes();
                let copy_len = if name_bytes.len() > 63 { 63 } else { name_bytes.len() };
                FOLDER_FILES[folder_index][file_count][..copy_len].copy_from_slice(&name_bytes[..copy_len]);
                FOLDER_FILES[folder_index][file_count][copy_len] = 0;

                let content_len = if NOTEPAD_LEN > 1024 { 1024 } else { NOTEPAD_LEN };
                FOLDER_FILE_CONTENTS[folder_index][file_count][..content_len].copy_from_slice(&NOTEPAD_TEXT[..content_len]);
                FOLDER_FILE_LENS[folder_index][file_count] = content_len;

                FOLDER_FILES_COUNT[folder_index] += 1;
            }
        }
    }
}

fn open_folder_window(wm: &mut WindowManager, folder_name: &str, folder_index: usize) {
    static mut FOLDER_TITLES: [[u8; 64]; 10] = [[0; 64]; 10];
    unsafe {
        if folder_index >= 10 { return; }
        let buf = &mut FOLDER_TITLES[folder_index];
        let name_bytes = folder_name.as_bytes();
        let mut copy_len = if name_bytes.len() > 63 { 63 } else { name_bytes.len() };
        while copy_len > 0 && !folder_name.is_char_boundary(copy_len) { copy_len -= 1; }
        buf[..copy_len].copy_from_slice(&name_bytes[..copy_len]);
        buf[copy_len] = 0;
        let title = core::str::from_utf8(&FOLDER_TITLES[folder_index][..copy_len]).unwrap_or("Folder");
        wm.create_window(title, 150, 120, 400, 300);
        CURRENT_OPEN_FOLDER = folder_index;
    }
}

static mut CURRENT_OPEN_FOLDER: usize = 0;

fn open_file_from_folder(wm: &mut WindowManager, folder_index: usize, file_index: usize) {
    unsafe {
        let content_len = FOLDER_FILE_LENS[folder_index][file_index];
        let content = &FOLDER_FILE_CONTENTS[folder_index][file_index][..content_len];
        NOTEPAD_TEXT[..content_len].copy_from_slice(content);
        NOTEPAD_LEN = content_len;
        wm.create_window("Notepad", 100, 100, 500, 400);
    }
}

fn notepad_set_status(ok: bool, parts: &[&str]) {
    unsafe {
        NOTEPAD_STATUS_OK = ok;
        let mut n = 0usize;
        for p in parts {
            for &b in p.as_bytes() {
                if n < NOTEPAD_STATUS.len() {
                    NOTEPAD_STATUS[n] = b;
                    n += 1;
                }
            }
        }
        NOTEPAD_STATUS_LEN = n;
    }
}

fn notepad_file_name(name: &str, out: &mut [u8; 80]) -> usize {
    let mut len = if name.len() > 60 { 60 } else { name.len() };
    while len > 0 && !name.is_char_boundary(len) { len -= 1; }
    out[..len].copy_from_slice(&name.as_bytes()[..len]);
    if !name[..len].contains('.') {
        out[len..len + 4].copy_from_slice(b".TXT");
        len += 4;
    }
    len
}

fn notepad_save_fat32(name: &str) {
    unsafe {
        let mut nb = [0u8; 80];
        let nl = notepad_file_name(name, &mut nb);
        let fname = core::str::from_utf8(&nb[..nl]).unwrap_or("");

        let mut out = [0u8; 2048];
        let mut o = 0usize;
        for i in 0..NOTEPAD_LEN {
            let b = NOTEPAD_TEXT[i];
            if b == b'\n' {
                out[o] = b'\r';
                out[o + 1] = b'\n';
                o += 2;
            } else {
                out[o] = b;
                o += 1;
            }
        }

        match save_bytes_fat32(fname, &out[..o]) {
            Ok(()) => {
                let mut d = [0u8; 13];
                let dl = fs::fat32::display_name(fname, &mut d);
                let ds = core::str::from_utf8(&d[..dl]).unwrap_or("");
                notepad_set_status(true, &["Saved: ", ds]);
            }
            Err(e) => notepad_set_status(false, &[e]),
        }
    }
}

fn notepad_load_fat32(name: &str) {
    unsafe {
        let mut nb = [0u8; 80];
        let nl = notepad_file_name(name, &mut nb);
        let fname = core::str::from_utf8(&nb[..nl]).unwrap_or("");

        let mut raw = [0u8; 4096];
        match fs::fat32::load_file(fname, &mut raw) {
            Ok(size) => {
                let mut txt = [0u8; 1024];
                let mut n = 0usize;
                let mut too_big = false;
                let start = if size >= 3 && raw[0] == 0xEF && raw[1] == 0xBB && raw[2] == 0xBF { 3 } else { 0 };
                for i in start..size {
                    let mut b = raw[i];
                    if b == b'\r' { continue; }
                    if b == b'\t' { b = b' '; }
                    if b < 0x20 && b != b'\n' { continue; }
                    if n >= 1023 { too_big = true; break; }
                    txt[n] = b;
                    n += 1;
                }
                if too_big {
                    notepad_set_status(false, &["File too big (max 1023 bytes)"]);
                } else {
                    NOTEPAD_TEXT[..n].copy_from_slice(&txt[..n]);
                    NOTEPAD_LEN = n;
                    let mut d = [0u8; 13];
                    let dl = fs::fat32::display_name(fname, &mut d);
                    let ds = core::str::from_utf8(&d[..dl]).unwrap_or("");
                    notepad_set_status(true, &["Loaded: ", ds]);
                }
            }
            Err(e) => notepad_set_status(false, &[e]),
        }
    }
}

fn save_bytes_fat32(fname: &str, data: &[u8]) -> Result<(), &'static str> {
    unsafe {
        if SAVE_DISK < 8 {
            let d = SAVE_DISK;
            return match (fs::fat32::make_short_name(fname), &FAT32_FILESYSTEMS[d]) {
                (Some(short), Some(f)) => {
                    let r = f.write_file_in(SAVE_DIR, &short, data);
                    let _ = ata::flush(d);
                    r
                }
                _ => Err("Bad file name"),
            };
        }
        if FILE_MANAGER_MOUNTED && FILE_MANAGER_DIALOG_DISK < 8 {
            let d = FILE_MANAGER_DIALOG_DISK;
            match (fs::fat32::make_short_name(fname), &FAT32_FILESYSTEMS[d]) {
                (Some(short), Some(f)) => {
                    let r = f.write_file(&short, data);
                    let _ = ata::flush(d);
                    r
                }
                _ => Err("Bad file name"),
            }
        } else {
            fs::fat32::save_file(fname, data)
        }
    }
}

fn cretool_save_fat32(name: &str) {
    unsafe {
        let mut nb = [0u8; 80];
        let mut len = if name.len() > 60 { 60 } else { name.len() };
        while len > 0 && !name.is_char_boundary(len) { len -= 1; }
        nb[..len].copy_from_slice(&name.as_bytes()[..len]);
        if !name[..len].contains('.') {
            nb[len..len + 3].copy_from_slice(b".PY");
            len += 3;
        }
        let fname = core::str::from_utf8(&nb[..len]).unwrap_or("");
        let data = cre_tool::code_bytes();

        let res = save_bytes_fat32(fname, data);

        match res {
            Ok(()) => {
                let mut d = [0u8; 13];
                let dl = fs::fat32::display_name(fname, &mut d);
                let ds = core::str::from_utf8(&d[..dl]).unwrap_or("");
                let mut msg = [0u8; 40];
                let pre = b"Saved: ";
                msg[..pre.len()].copy_from_slice(pre);
                msg[pre.len()..pre.len() + dl].copy_from_slice(ds.as_bytes());
                cre_tool::console_note(core::str::from_utf8(&msg[..pre.len() + dl]).unwrap_or("Saved"));
            }
            Err(e) => cre_tool::console_note(e),
        }
    }
}

fn open_ata_file_in_notepad(wm: &mut WindowManager, filename: &str) {
    unsafe {
        let mut buffer = [0u8; 512];
        let size = read_ata_file(filename, &mut buffer);
        if size > 0 {
            NOTEPAD_TEXT[..size].copy_from_slice(&buffer[..size]);
            NOTEPAD_LEN = size;
            wm.create_window("Notepad", 100, 100, 500, 400);
        }
    }
}

const BMP_IO_MAX: usize = 1_572_864;
static mut BMP_IO_BUF: [u8; BMP_IO_MAX] = [0; BMP_IO_MAX];
static mut PAINT_STATUS: [u8; 25] = [0; 25];
static mut PAINT_STATUS_LEN: usize = 0;
static mut PAINT_STATUS_OK: bool = true;

fn paint_set_status(ok: bool, parts: &[&str]) {
    unsafe {
        PAINT_STATUS_OK = ok;
        let mut n = 0usize;
        for p in parts {
            for &b in p.as_bytes() {
                if n < PAINT_STATUS.len() {
                    PAINT_STATUS[n] = b;
                    n += 1;
                }
            }
        }
        PAINT_STATUS_LEN = n;
    }
}

fn paint_err_text(e: &'static str) -> &'static str {
    if e.starts_with("Not FAT32") { "No FAT32 disk" } else { e }
}

fn paint_file_name(name: &str, out: &mut [u8; 80]) -> usize {
    let mut len = if name.len() > 60 { 60 } else { name.len() };
    while len > 0 && !name.is_char_boundary(len) { len -= 1; }
    if let Some(p) = name[..len].rfind('.') {
        if p > 0 { len = p; }
    }
    out[..len].copy_from_slice(&name.as_bytes()[..len]);
    out[len..len + 4].copy_from_slice(b".BMP");
    len + 4
}

fn paint_save_bmp(name: &str) {
    unsafe {
        const W: usize = 400;
        const H: usize = 300;
        let mut nb = [0u8; 80];
        let nl = paint_file_name(name, &mut nb);
        let fname = core::str::from_utf8(&nb[..nl]).unwrap_or("");

        let img_size = W * 3 * H;
        let total = 54 + img_size;
        let buf = &mut BMP_IO_BUF;
        for b in buf[..54].iter_mut() { *b = 0; }
        buf[0] = b'B';
        buf[1] = b'M';
        buf[2..6].copy_from_slice(&(total as u32).to_le_bytes());
        buf[10..14].copy_from_slice(&54u32.to_le_bytes());
        buf[14..18].copy_from_slice(&40u32.to_le_bytes());
        buf[18..22].copy_from_slice(&(W as u32).to_le_bytes());
        buf[22..26].copy_from_slice(&(H as u32).to_le_bytes());
        buf[26..28].copy_from_slice(&1u16.to_le_bytes());
        buf[28..30].copy_from_slice(&24u16.to_le_bytes());
        buf[34..38].copy_from_slice(&(img_size as u32).to_le_bytes());
        buf[38..42].copy_from_slice(&2835u32.to_le_bytes());
        buf[42..46].copy_from_slice(&2835u32.to_le_bytes());
        let mut o = 54usize;
        for y in (0..H).rev() {
            for x in 0..W {
                let c = PAINT_CANVAS[y][x];
                buf[o] = (c & 0xFF) as u8;
                buf[o + 1] = ((c >> 8) & 0xFF) as u8;
                buf[o + 2] = ((c >> 16) & 0xFF) as u8;
                o += 3;
            }
        }

        match save_bytes_fat32(fname, &buf[..total]) {
            Ok(()) => {
                let mut d = [0u8; 13];
                let dl = fs::fat32::display_name(fname, &mut d);
                let ds = core::str::from_utf8(&d[..dl]).unwrap_or("");
                paint_set_status(true, &["Saved: ", ds]);
            }
            Err(e) => paint_set_status(false, &[paint_err_text(e)]),
        }
    }
}

fn paint_import_bmp(d: &[u8]) -> Result<bool, &'static str> {
    if d.len() < 54 || d[0] != b'B' || d[1] != b'M' {
        return Err("Not a BMP file");
    }
    let rd32 = |o: usize| u32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]]);
    let off = rd32(10) as usize;
    if rd32(14) < 40 { return Err("Unsupported BMP"); }
    let w = rd32(18) as i32;
    let h = rd32(22) as i32;
    let bpp = u16::from_le_bytes([d[28], d[29]]) as usize;
    if rd32(30) != 0 { return Err("Compressed BMP"); }
    if bpp != 24 && bpp != 32 { return Err("Need 24/32-bit BMP"); }
    if w <= 0 || h == 0 || w > 8192 || h > 8192 || h < -8192 { return Err("Bad BMP size"); }
    let top_down = h < 0;
    let ww = w as usize;
    let hh = if h < 0 { (-h) as usize } else { h as usize };
    let bytes_pp = bpp / 8;
    let row = ((ww * bpp + 31) / 32) * 4;
    if off > d.len() || off + row * hh > d.len() { return Err("BMP data truncated"); }
    unsafe {
        for y in 0..300 {
            for x in 0..400 {
                let c = if x < ww && y < hh {
                    let sy = if top_down { y } else { hh - 1 - y };
                    let q = off + sy * row + x * bytes_pp;
                    ((d[q + 2] as u32) << 16) | ((d[q + 1] as u32) << 8) | (d[q] as u32)
                } else {
                    0xFFFFFF
                };
                PAINT_CANVAS[y][x] = c;
                PAINT_CANVAS_BACKUP[y][x] = c;
            }
        }
        PAINT_IS_DRAWING = false;
    }
    Ok(ww > 400 || hh > 300)
}

const DV_MAX_ENTRIES: usize = 128;
const DV_MAX_DEPTH: usize = 12;
const SD_W: usize = 480;
const SD_H: usize = 352;
const SD_ROWS: usize = 11;

#[derive(Clone, Copy)]
enum ViewItem {
    Up,
    Entry(usize),
}

struct DirView {
    disk: usize,
    depth: usize,
    cl: [u32; DV_MAX_DEPTH + 1],
    names: [[u8; 13]; DV_MAX_DEPTH + 1],
    name_len: [usize; DV_MAX_DEPTH + 1],
    entries: [fs::fat32::FsEntry; DV_MAX_ENTRIES],
    count: usize,
    scroll: usize,
}

fn put_bytes(buf: &mut [u8], mut p: usize, s: &[u8]) -> usize {
    for &b in s {
        if p < buf.len() {
            buf[p] = b;
            p += 1;
        }
    }
    p
}

impl DirView {
    const fn new() -> DirView {
        DirView {
            disk: 0,
            depth: 0,
            cl: [0; DV_MAX_DEPTH + 1],
            names: [[0; 13]; DV_MAX_DEPTH + 1],
            name_len: [0; DV_MAX_DEPTH + 1],
            entries: [fs::fat32::FsEntry::EMPTY; DV_MAX_ENTRIES],
            count: 0,
            scroll: 0,
        }
    }

    fn open_root(&mut self, disk: usize) -> bool {
        if disk >= 8 {
            return false;
        }
        let root = unsafe {
            match &FAT32_FILESYSTEMS[disk] {
                Some(f) => f.root(),
                None => return false,
            }
        };
        self.disk = disk;
        self.depth = 0;
        self.cl[0] = root;
        self.name_len[0] = 0;
        self.scroll = 0;
        self.reload();
        true
    }

    fn cur(&self) -> u32 {
        self.cl[self.depth]
    }

    fn total_items(&self) -> usize {
        self.count + if self.depth > 0 { 1 } else { 0 }
    }

    fn item(&self, k: usize) -> Option<ViewItem> {
        let off = if self.depth > 0 { 1 } else { 0 };
        if k >= self.count + off {
            return None;
        }
        if off == 1 && k == 0 {
            Some(ViewItem::Up)
        } else {
            Some(ViewItem::Entry(k - off))
        }
    }

    fn reload(&mut self) {
        self.count = 0;
        if self.disk < 8 {
            unsafe {
                if let Some(f) = &FAT32_FILESYSTEMS[self.disk] {
                    let c = self.cl[self.depth];
                    self.count = f.list_dir(c, &mut self.entries);
                }
            }
        }
        let mut i = 1usize;
        while i < self.count {
            let mut j = i;
            while j > 0 && !self.entries[j - 1].is_dir && self.entries[j].is_dir {
                self.entries.swap(j - 1, j);
                j -= 1;
            }
            i += 1;
        }
        let max = self.total_items().saturating_sub(1);
        if self.scroll > max {
            self.scroll = max;
        }
    }

    fn enter(&mut self, idx: usize) {
        if idx >= self.count || !self.entries[idx].is_dir || self.depth >= DV_MAX_DEPTH {
            return;
        }
        let e = self.entries[idx];
        let d = self.depth + 1;
        self.cl[d] = if e.cluster < 2 { self.cl[0] } else { e.cluster };
        self.names[d] = e.name;
        self.name_len[d] = e.name_len;
        self.depth = d;
        self.scroll = 0;
        self.reload();
    }

    fn up(&mut self) {
        if self.depth > 0 {
            self.depth -= 1;
            self.scroll = 0;
            self.reload();
        }
    }

    fn scroll_by(&mut self, wheel: i32, rows: usize) {
        let max = self.total_items().saturating_sub(rows);
        let step = (wheel.unsigned_abs() as usize) * 3;
        if wheel > 0 {
            self.scroll = (self.scroll + step).min(max);
        } else {
            self.scroll = self.scroll.saturating_sub(step);
        }
    }

    fn path(&self, out: &mut [u8; 160]) -> usize {
        let mut p = put_bytes(out, 0, b"Disk ");
        p = put_bytes(out, p, num_to_str(self.disk + 1).as_bytes());
        p = put_bytes(out, p, b":\\");
        for d in 1..=self.depth {
            if d > 1 {
                p = put_bytes(out, p, b"\\");
            }
            p = put_bytes(out, p, &self.names[d][..self.name_len[d]]);
        }
        p
    }

    fn path_slash(&self, out: &mut [u8; 160]) -> usize {
        let mut p = put_bytes(out, 0, b"Disk ");
        p = put_bytes(out, p, num_to_str(self.disk + 1).as_bytes());
        p = put_bytes(out, p, b":/");
        for d in 1..=self.depth {
            if d > 1 {
                p = put_bytes(out, p, b"/");
            }
            p = put_bytes(out, p, &self.names[d][..self.name_len[d]]);
        }
        p
    }
}

static mut FM_VIEW: DirView = DirView::new();
static mut SD_VIEW: DirView = DirView::new();
static mut SD_OK: bool = false;
static mut SAVE_DISK: usize = usize::MAX;
static mut SAVE_DIR: u32 = 0;
static mut CONTEXT_MENU_KIND: u8 = 0;
static mut SC_CTX_IDX: usize = 0;
static mut FM_CTX_NAME: [u8; 13] = [0; 13];
static mut FM_CTX_LEN: usize = 0;

static mut FM_MSG: [u8; 48] = [0; 48];
static mut FM_MSG_LEN: usize = 0;
static mut FM_MSG_OK: bool = true;

fn fm_set_msg(ok: bool, parts: &[&str]) {
    unsafe {
        FM_MSG_OK = ok;
        let mut n = 0usize;
        for p in parts {
            for &b in p.as_bytes() {
                if n < FM_MSG.len() {
                    FM_MSG[n] = b;
                    n += 1;
                }
            }
        }
        FM_MSG_LEN = n;
    }
}

fn fm_visible_rows(win_h: usize) -> usize {
    win_h.saturating_sub(145) / 18
}

fn context_menu_h() -> usize {
    let kind = unsafe { CONTEXT_MENU_KIND };
    if kind == 3 { 32 } else if kind == 1 || kind == 2 { 56 } else { 112 }
}

fn fm_item_path(name: &str, out: &mut [u8; syscfg::SHORTCUT_PATH_MAX]) -> Option<usize> {
    unsafe {
        let mut p = 0usize;
        for d in 1..=FM_VIEW.depth {
            let l = FM_VIEW.name_len[d];
            if p + 1 + l > out.len() {
                return None;
            }
            out[p] = b'/';
            p += 1;
            out[p..p + l].copy_from_slice(&FM_VIEW.names[d][..l]);
            p += l;
        }
        let nb = name.as_bytes();
        if p + 1 + nb.len() > out.len() {
            return None;
        }
        out[p] = b'/';
        p += 1;
        out[p..p + nb.len()].copy_from_slice(nb);
        p += nb.len();
        Some(p)
    }
}

fn fm_is_sys_folder(e: &fs::fat32::FsEntry) -> bool {
    unsafe {
        e.is_dir && FM_VIEW.depth == 0 && e.name[..e.name_len].eq_ignore_ascii_case(b"MiUX")
    }
}

fn fm_try_protect(wm: &mut WindowManager) -> bool {
    unsafe {
        if !FILE_MANAGER_MOUNTED || FM_CTX_LEN == 0 {
            return false;
        }
        let d = FM_VIEW.disk;
        if d >= 8 {
            return false;
        }
        let name = &FM_CTX_NAME[..FM_CTX_LEN];
        let depth = FM_VIEW.depth;
        let in_sys = depth >= 1 && FM_VIEW.names[1][..FM_VIEW.name_len[1]].eq_ignore_ascii_case(b"MiUX");
        let rule = if depth == 0 && name.eq_ignore_ascii_case(b"MiUX") {
            "dir.mp"
        } else if depth == 1 && in_sys && name.eq_ignore_ascii_case(b"MIUX.ELF") {
            "mas.mp"
        } else if depth == 1 && in_sys && name.eq_ignore_ascii_case(syscfg::FILE_NAME.as_bytes()) {
            "cfg.mp"
        } else if depth == 2 && in_sys
            && FM_VIEW.names[2][..FM_VIEW.name_len[2]].eq_ignore_ascii_case(b"users")
            && name.eq_ignore_ascii_case(b"user.cfg") {
            "usr.mp"
        } else {
            return false;
        };
        let mut path = [0u8; 32];
        let pre = b"/MiUX/ryles/";
        path[..pre.len()].copy_from_slice(pre);
        path[pre.len()..pre.len() + rule.len()].copy_from_slice(rule.as_bytes());
        let p = core::str::from_utf8(&path[..pre.len() + rule.len()]).unwrap_or("");
        let target = match &FAT32_FILESYSTEMS[d] {
            Some(f) => f.resolve_file(p),
            None => Err("No filesystem"),
        };
        match target {
            Ok((dir, _)) => {
                fm_open_file_at(wm, d, dir, rule);
                fm_set_msg(false, &["Access denied"]);
            }
            Err(_) => fm_set_msg(false, &["Protected system file"]),
        }
        true
    }
}

fn fm_delete_selected() {
    unsafe {
        if !FILE_MANAGER_MOUNTED {
            return;
        }
        let d = FM_VIEW.disk;
        if d >= 8 || FM_CTX_LEN == 0 {
            return;
        }
        let name = core::str::from_utf8(&FM_CTX_NAME[..FM_CTX_LEN]).unwrap_or("");
        let res = match (fs::fat32::make_short_name(name), &FAT32_FILESYSTEMS[d]) {
            (Some(short), Some(f)) => {
                let r = f.delete_file_in(FM_VIEW.cur(), &short);
                let _ = ata::flush(d);
                r
            }
            _ => Err("Bad file name"),
        };
        if res.is_ok() {
            let mut pb = [0u8; syscfg::SHORTCUT_PATH_MAX];
            if let Some(pl) = fm_item_path(name, &mut pb) {
                syscfg::remove_shortcuts_for(d, core::str::from_utf8(&pb[..pl]).unwrap_or(""));
            }
        }
        FM_VIEW.reload();
        match res {
            Ok(()) => fm_set_msg(true, &["File deleted"]),
            Err(e) => fm_set_msg(false, &[e]),
        }
    }
}

fn fm_make_shortcut() {
    unsafe {
        if !FILE_MANAGER_MOUNTED {
            return;
        }
        let d = FM_VIEW.disk;
        if d >= 8 || FM_CTX_LEN == 0 {
            return;
        }
        let name = core::str::from_utf8(&FM_CTX_NAME[..FM_CTX_LEN]).unwrap_or("");
        let mut pb = [0u8; syscfg::SHORTCUT_PATH_MAX];
        let res = match fm_item_path(name, &mut pb) {
            Some(pl) => syscfg::add_shortcut(d, core::str::from_utf8(&pb[..pl]).unwrap_or("")),
            None => Err("Path too long"),
        };
        match res {
            Ok(()) => fm_set_msg(true, &["Shortcut created"]),
            Err(e) => fm_set_msg(false, &[e]),
        }
    }
}

fn icon_dec(p: u32) -> (usize, usize) {
    ((p >> 16) as usize, (p & 0xFFFF) as usize)
}

fn icon_enc(x: usize, y: usize) -> u32 {
    (((x & 0xFFFF) as u32) << 16) | ((y & 0xFFFF) as u32)
}

fn icon_clamp(x: usize, y: usize) -> (usize, usize) {
    unsafe { (x.min(ICON_LIM.0.saturating_sub(64)), y.min(ICON_LIM.1.saturating_sub(84))) }
}

fn icon_drag_override(id: usize) -> Option<(usize, usize)> {
    unsafe {
        if ICON_DRAG == id && ICON_DRAG_MOVED { Some(ICON_DRAG_POS) } else { None }
    }
}

fn icon_drag_begin(id: usize, ix: usize, iy: usize, cx: usize, cy: usize) {
    unsafe {
        ICON_DRAG = id;
        ICON_DRAG_MOVED = false;
        ICON_DRAG_START = (cx, cy);
        ICON_DRAG_OFF = (cx.saturating_sub(ix), cy.saturating_sub(iy));
        ICON_DRAG_POS = (ix, iy);
    }
}

fn sys_icon_pos(n: usize) -> (usize, usize) {
    if BARE_DESKTOP { return (OFF_SCREEN, OFF_SCREEN); }
    if let Some(p) = icon_drag_override(n) { return p; }
    unsafe {
        if n < 9 && SYS_ICON_POS[n] != u32::MAX {
            let (x, y) = icon_dec(SYS_ICON_POS[n]);
            return icon_clamp(x, y);
        }
    }
    (50 + n * 100, 50)
}

fn folder_pos_or(i: usize, dx: usize, dy: usize) -> (usize, usize) {
    if BARE_DESKTOP { return (OFF_SCREEN, OFF_SCREEN); }
    if let Some(p) = icon_drag_override(100 + i) { return p; }
    unsafe {
        if i < 10 && FOLDER_ICON_POS[i] != u32::MAX {
            let (x, y) = icon_dec(FOLDER_ICON_POS[i]);
            return icon_clamp(x, y);
        }
    }
    (dx, dy)
}

fn icon_snap(id: usize, x: usize, y: usize) -> (usize, usize) {
    unsafe {
        let (lw, lh) = ICON_LIM;
        let max_x = lw.saturating_sub(64);
        let max_y = lh.saturating_sub(84);
        let cols = if max_x >= 50 { (max_x - 50) / 100 + 1 } else { 1 };
        let rows = if max_y >= 50 { (max_y - 50) / 100 + 1 } else { 1 };

        let mut others = [(0usize, 0usize); 48];
        let mut n = 0usize;
        for k in 0..9 {
            if k != id && n < 48 { others[n] = sys_icon_pos(k); n += 1; }
        }
        for i in 0..DESKTOP_FOLDERS_COUNT {
            if 100 + i != id && n < 48 {
                others[n] = folder_pos_or(i, 50 + (i % 6) * 100, 154 + (i / 6) * 94);
                n += 1;
            }
        }
        for i in 0..syscfg::shortcut_count() {
            if 200 + i != id && n < 48 { others[n] = shortcut_pos(i); n += 1; }
        }

        let mut best = (x, y);
        let mut best_d = usize::MAX;
        for r in 0..rows {
            for c in 0..cols {
                let cx = 50 + c * 100;
                let cy = 50 + r * 100;
                let mut free = true;
                for k in 0..n {
                    if others[k].0.abs_diff(cx) < 90 && others[k].1.abs_diff(cy) < 84 {
                        free = false;
                        break;
                    }
                }
                if !free { continue; }
                let dx = cx.abs_diff(x);
                let dy = cy.abs_diff(y);
                let d = dx * dx + dy * dy;
                if d < best_d {
                    best_d = d;
                    best = (cx, cy);
                }
            }
        }
        best
    }
}

fn shortcut_pos(i: usize) -> (usize, usize) {
    if BARE_DESKTOP { return (OFF_SCREEN, OFF_SCREEN); }
    if let Some(p) = icon_drag_override(200 + i) { return p; }
    if let Some(sc) = syscfg::shortcut(i) {
        if sc.pos != u32::MAX {
            let (x, y) = icon_dec(sc.pos);
            return icon_clamp(x, y);
        }
    }
    let slot = unsafe { DESKTOP_FOLDERS_COUNT } + i;
    (50 + (slot % 6) * 100, 50 + 64 + 40 + (slot / 6) * (64 + 30))
}

fn draw_shortcut_icon(gfx: &mut Graphics, x: usize, y: usize, name: &str) {
    let icon_size = 64;
    let icon = unsafe { &ICON_SHORTCUT };
    draw_icon(gfx, x, y, icon, icon_size);

    let display_name = clip_str(name, 12);
    let tw = display_name.chars().count() * 8;
    let text_x = if tw >= icon_size { x.saturating_sub((tw - icon_size) / 2) } else { x + (icon_size - tw) / 2 };
    gfx.draw_text(text_x, y + icon_size + 5, display_name, Color::WHITE.to_u32(), Color::rgb(0, 0, 0).to_u32());
}

fn fm_create_folder(name: &str) {
    unsafe {
        if !FILE_MANAGER_MOUNTED {
            return;
        }
        let d = FM_VIEW.disk;
        if d >= 8 {
            return;
        }
        let res = match (fs::fat32::make_dir_name(name), &FAT32_FILESYSTEMS[d]) {
            (Some(short), Some(f)) => {
                let r = f.mkdir_in(FM_VIEW.cur(), &short);
                let _ = ata::flush(d);
                r
            }
            _ => Err("Bad folder name"),
        };
        FM_VIEW.reload();
        match res {
            Ok(()) => fm_set_msg(true, &["Folder created"]),
            Err(e) => fm_set_msg(false, &[e]),
        }
    }
}

fn draw_dir_rows(gfx: &mut Graphics, v: &DirView, x: usize, top: usize, rows: usize, bg: u32) {
    let total = v.total_items();
    for r in 0..rows {
        let k = v.scroll + r;
        if k >= total {
            break;
        }
        let y = top + r * 18;
        let mut line = [0u8; 80];
        let mut p = 0usize;
        let mut is_dir = true;
        match v.item(k) {
            Some(ViewItem::Up) => {
                p = put_bytes(&mut line, p, b"[DIR]  ..");
            }
            Some(ViewItem::Entry(e)) => {
                let en = &v.entries[e];
                is_dir = en.is_dir;
                if en.is_dir {
                    p = put_bytes(&mut line, p, b"[DIR]  ");
                } else {
                    p = put_bytes(&mut line, p, b"[FILE] ");
                }
                p = put_bytes(&mut line, p, &en.name[..en.name_len]);
                if !en.is_dir {
                    p = put_bytes(&mut line, p, b" (");
                    p = put_bytes(&mut line, p, num_to_str(en.size as usize).as_bytes());
                    p = put_bytes(&mut line, p, b" bytes)");
                }
            }
            None => {}
        }
        let s = core::str::from_utf8(&line[..p]).unwrap_or("");
        let fg = if is_dir { Color::rgb(0, 70, 170).to_u32() } else { Color::BLACK.to_u32() };
        gfx.draw_text(x, y, s, fg, bg);
    }
}


fn sd_disk_ok(i: usize) -> bool {
    unsafe {
        match ata::get_disk(i) {
            Some(info) => {
                if !info.present || info.dev_type != ata::DeviceType::ATA {
                    return false;
                }
            }
            None => return false,
        }
        mount_fat32_disk(i)
    }
}

fn sd_init() {
    unsafe {
        SD_OK = false;
        let count = ata::disk_count();
        DISK_COUNT = count;
        let mut chosen = usize::MAX;
        if FILE_MANAGER_MOUNTED && FILE_MANAGER_DIALOG_DISK < 8 && FAT32_FILESYSTEMS[FILE_MANAGER_DIALOG_DISK].is_some() {
            chosen = FILE_MANAGER_DIALOG_DISK;
        }
        if chosen == usize::MAX {
            let lim = if count < 8 { count } else { 8 };
            for i in 0..lim {
                if sd_disk_ok(i) {
                    chosen = i;
                    break;
                }
            }
        }
        if chosen != usize::MAX && SD_VIEW.open_root(chosen) {
            SD_OK = true;
        }
    }
}

fn sd_next_disk() {
    unsafe {
        let count = if DISK_COUNT < 8 { DISK_COUNT } else { 8 };
        if count == 0 {
            return;
        }
        let start = if SD_OK { SD_VIEW.disk } else { 0 };
        for step in 1..=count {
            let i = (start + step) % count;
            if sd_disk_ok(i) {
                if SD_VIEW.open_root(i) {
                    SD_OK = true;
                }
                return;
            }
        }
    }
}

fn show_save_dialog(gfx: &mut Graphics, callback: fn(&str)) {
    show_name_input_dialog(gfx, callback, 3, 0);
    unsafe {
        NAME_INPUT_X = gfx.width.saturating_sub(SD_W) / 2;
        NAME_INPUT_Y = gfx.height.saturating_sub(SD_H) / 2;
    }
    sd_init();
}

fn draw_save_dialog(gfx: &mut Graphics) {
    unsafe {
        let x = NAME_INPUT_X;
        let y = NAME_INPUT_Y;
        let w = SD_W;
        let h = SD_H;
        let panel = Color::rgb(235, 235, 235).to_u32();
        let btn_bg = Color::rgb(212, 208, 200).to_u32();
        let white = Color::WHITE.to_u32();
        let black = Color::BLACK.to_u32();
        let grey = Color::rgb(128, 128, 128).to_u32();
        let bar = Color::rgb(90, 90, 90).to_u32();

        gfx.fill_rect(x, y, w, h, panel);
        gfx.draw_rect_border(x, y, w, h, Color::rgb(87, 87, 87).to_u32());
        gfx.draw_rect_border(x + 1, y + 1, w - 2, h - 2, white);
        gfx.fill_rect(x + 2, y + 2, w - 4, 30, bar);
        gfx.draw_text(x + 10, y + 6, "Save As", white, bar);

        wm::draw_raised_rect(gfx, x + 10, y + 38, 50, 22);
        gfx.draw_text(x + 24, y + 41, "Up", black, btn_bg);

        let path_w = w - 66 - 90;
        gfx.fill_rect(x + 66, y + 38, path_w, 22, white);
        gfx.draw_rect_border(x + 66, y + 38, path_w, 22, grey);
        let mut pb = [0u8; 160];
        let pl = if SD_OK { SD_VIEW.path(&mut pb) } else { 0 };
        let ps = core::str::from_utf8(&pb[..pl]).unwrap_or("");
        let maxc = (path_w - 8) / 8;
        let ps = if ps.len() > maxc { &ps[ps.len() - maxc..] } else { ps };
        gfx.draw_text(x + 70, y + 41, ps, black, white);

        wm::draw_raised_rect(gfx, x + w - 80, y + 38, 70, 22);
        if SD_OK {
            let mut db = [0u8; 12];
            let mut dp = put_bytes(&mut db, 0, b"Disk ");
            dp = put_bytes(&mut db, dp, num_to_str(SD_VIEW.disk + 1).as_bytes());
            let ds = core::str::from_utf8(&db[..dp]).unwrap_or("Disk");
            gfx.draw_text(x + w - 74, y + 41, ds, black, btn_bg);
        }

        let lx = x + 10;
        let ly = y + 68;
        let lw = w - 20;
        let lh = SD_ROWS * 18 + 4;
        gfx.fill_rect(lx, ly, lw, lh, white);
        gfx.draw_rect_border(lx, ly, lw, lh, grey);
        if !SD_OK {
            gfx.draw_text(lx + 8, ly + 6, "No FAT32 disk found", Color::rgb(180, 0, 0).to_u32(), white);
        } else if SD_VIEW.total_items() == 0 {
            gfx.draw_text(lx + 8, ly + 6, "(empty folder)", grey, white);
        } else {
            draw_dir_rows(gfx, &SD_VIEW, lx + 6, ly + 2, SD_ROWS, white);
        }

        gfx.draw_text(x + 10, y + 284, "File name:", black, panel);
        gfx.fill_rect(x + 94, y + 280, w - 104, 24, white);
        gfx.draw_rect_border(x + 94, y + 280, w - 104, 24, grey);
        let ns = core::str::from_utf8(&NAME_INPUT_BUFFER[..NAME_INPUT_LEN]).unwrap_or("");
        let maxn = (w - 104 - 14) / 8;
        let mut start = if ns.len() > maxn { ns.len() - maxn } else { 0 };
        while start < ns.len() && !ns.is_char_boundary(start) {
            start += 1;
        }
        gfx.draw_text(x + 100, y + 284, &ns[start..], black, white);

        gfx.draw_text(x + 10, y + 322, "Click a folder to open it", grey, panel);
        wm::draw_raised_rect(gfx, x + w - 190, y + 314, 80, 28);
        gfx.draw_text(x + w - 190 + 22, y + 314 + 6, "Save", black, btn_bg);
        wm::draw_raised_rect(gfx, x + w - 100, y + 314, 80, 28);
        gfx.draw_text(x + w - 100 + 16, y + 314 + 6, "Cancel", black, btn_bg);
    }
}

fn handle_save_dialog_click(mx: usize, my: usize) -> bool {
    unsafe {
        let x = NAME_INPUT_X;
        let y = NAME_INPUT_Y;
        let w = SD_W;
        let inside = |ax: usize, ay: usize, aw: usize, ah: usize| -> bool {
            mx >= ax && mx < ax + aw && my >= ay && my < ay + ah
        };

        if inside(x + w - 190, y + 314, 80, 28) {
            if name_input_confirm() {
                NAME_INPUT_OPEN = false;
            }
            return true;
        }
        if inside(x + w - 100, y + 314, 80, 28) {
            NAME_INPUT_OPEN = false;
            return true;
        }
        if SD_OK {
            if inside(x + 10, y + 38, 50, 22) {
                SD_VIEW.up();
                return true;
            }
            if inside(x + w - 80, y + 38, 70, 22) {
                sd_next_disk();
                return true;
            }
            if inside(x + 10, y + 70, w - 20, SD_ROWS * 18) {
                let k = SD_VIEW.scroll + (my - (y + 70)) / 18;
                match SD_VIEW.item(k) {
                    Some(ViewItem::Up) => {
                        SD_VIEW.up();
                    }
                    Some(ViewItem::Entry(e)) => {
                        if SD_VIEW.entries[e].is_dir {
                            SD_VIEW.enter(e);
                        } else {
                            let nm = SD_VIEW.entries[e].name;
                            let nl = SD_VIEW.entries[e].name_len;
                            NAME_INPUT_BUFFER = [0; 64];
                            NAME_INPUT_BUFFER[..nl].copy_from_slice(&nm[..nl]);
                            NAME_INPUT_LEN = nl;
                        }
                    }
                    None => {}
                }
                return true;
            }
        }
        false
    }
}

fn name_input_confirm() -> bool {
    unsafe {
        let mode3 = NAME_INPUT_MODE == 3;
        if NAME_INPUT_LEN == 0 {
            return !mode3;
        }
        if let Some(callback) = NAME_INPUT_CALLBACK {
            let name_str = core::str::from_utf8(&NAME_INPUT_BUFFER[..NAME_INPUT_LEN]).unwrap_or("");
            if name_str.len() > 0 {
                if mode3 {
                    if SD_OK {
                        SAVE_DISK = SD_VIEW.disk;
                        SAVE_DIR = SD_VIEW.cur();
                    }
                    callback(name_str);
                    SAVE_DISK = usize::MAX;
                    if FILE_MANAGER_MOUNTED {
                        FM_VIEW.reload();
                    }
                } else {
                    callback(name_str);
                }
            }
        }
        true
    }
}

fn fm_open_file(wm: &mut WindowManager, disk: usize, name: &str) {
    unsafe { fm_open_file_at(wm, disk, FM_VIEW.cur(), name) }
}

fn open_shortcut(wm: &mut WindowManager, idx: usize) {
    let sc = match syscfg::shortcut(idx) {
        Some(s) => s,
        None => return,
    };
    unsafe {
        DISK_COUNT = ata::disk_count();
        let d = sc.disk;
        if d >= 8 || !mount_fat32_disk(d) {
            return;
        }
        if sc.is_dir {
            let p = alloc::format!("Disk {}:{}", d + 1, sc.path_str());
            if wm.find_window("File Manager").is_none() {
                wm.create_window("File Manager", 150, 120, 500, 350);
            }
            let _ = fm_goto_path(&p);
            return;
        }
        let target = match &FAT32_FILESYSTEMS[d] {
            Some(f) => f.resolve_file(sc.path_str()),
            None => return,
        };
        if let Ok((dir, _)) = target {
            fm_open_file_at(wm, d, dir, sc.label());
        }
    }
}

fn open_system_app(wm: &mut WindowManager, name: &str) {
    unsafe {
        match fs::fat32::load_file_in_subdir(fs::fat32::SYS_DIR, "SYSTEMX64", name, &mut BMP_IO_BUF) {
            Ok(n) if web::app::looks_like_html(&BMP_IO_BUF[..n]) => {
                web::app::app_open(&BMP_IO_BUF[..n]);
                let (aw, ah) = web::app::window_size(&BMP_IO_BUF[..n]).unwrap_or((800, 600));
                let wi = wm.create_window("HTML App", 120, 80, aw, ah);
                if wi != usize::MAX { wm.set_window_size(wi, aw, ah); }
            }
            _ => {}
        }
    }
}

fn fm_open_file_at(wm: &mut WindowManager, disk: usize, dir: u32, name: &str) {
    unsafe {
        let ext = match name.rfind('.') {
            Some(p) => &name[p + 1..],
            None => return,
        };
        let is_bmp = ext.eq_ignore_ascii_case("bmp");
        let is_text = ext.eq_ignore_ascii_case("txt") || ext.eq_ignore_ascii_case("cfg")
            || ext.eq_ignore_ascii_case("ini") || ext.eq_ignore_ascii_case("log")
            || ext.eq_ignore_ascii_case("md");
        let is_py = ext.eq_ignore_ascii_case("py") || ext.eq_ignore_ascii_case("mp");
        if !is_bmp && !is_text && !is_py { return; }
        if disk >= 8 { return; }
        let short = match fs::fat32::make_short_name(name) {
            Some(n) => n,
            None => return,
        };
        let fsr = match &FAT32_FILESYSTEMS[disk] {
            Some(f) => f,
            None => return,
        };
        let mut d = [0u8; 13];
        let dl = fs::fat32::display_name(name, &mut d);
        let ds = core::str::from_utf8(&d[..dl]).unwrap_or("");

        let res = fsr.read_file_bytes_in(dir, &short, &mut BMP_IO_BUF);
        if is_py {
            if ext.eq_ignore_ascii_case("mp") {
                if let Ok(hsize) = &res {
                    let hsize = *hsize;
                    if web::app::looks_like_html(&BMP_IO_BUF[..hsize]) {
                        web::app::app_open(&BMP_IO_BUF[..hsize]);
                        let (aw, ah) = web::app::window_size(&BMP_IO_BUF[..hsize]).unwrap_or((800, 600));
                        let wi = wm.create_window("HTML App", 120, 80, aw, ah);
                        if wi != usize::MAX { wm.set_window_size(wi, aw, ah); }
                        return;
                    }
                }
            }
            match res {
                Ok(size) => { cre_tool::run_file_bytes(&BMP_IO_BUF[..size]); }
                Err(e) => cre_tool::console_message(e),
            }
            let pw = wm.create_window("Preview", 150, 120, 400, 300);
            cre_tool::set_preview_window(pw);
            return;
        }
        if is_bmp {
            let r = match res {
                Ok(n) => paint_import_bmp(&BMP_IO_BUF[..n]),
                Err(e) => Err(e),
            };
            match r {
                Ok(cropped) => paint_set_status(true, &[if cropped { "Cropped: " } else { "Opened: " }, ds]),
                Err(e) => paint_set_status(false, &[e]),
            }
            wm.create_window("Paint", 100, 100, 420, 400);
        } else {
            match res {
                Ok(size) => {
                    let start = if size >= 3 && BMP_IO_BUF[0] == 0xEF && BMP_IO_BUF[1] == 0xBB && BMP_IO_BUF[2] == 0xBF { 3 } else { 0 };
                    let mut txt = [0u8; 1024];
                    let mut n = 0usize;
                    let mut too_big = false;
                    for i in start..size {
                        let mut b = BMP_IO_BUF[i];
                        if b == b'\r' { continue; }
                        if b == b'\t' { b = b' '; }
                        if b < 0x20 && b != b'\n' { continue; }
                        if n >= 1023 { too_big = true; break; }
                        txt[n] = b;
                        n += 1;
                    }
                    if too_big {
                        notepad_set_status(false, &["File too big (max 1023 bytes)"]);
                    } else {
                        NOTEPAD_TEXT[..n].copy_from_slice(&txt[..n]);
                        NOTEPAD_LEN = n;
                        notepad_set_status(true, &["Loaded: ", ds]);
                    }
                }
                Err(e) => notepad_set_status(false, &[e]),
            }
            wm.create_window("Notepad", 100, 100, 500, 400);
        }
    }
}

fn show_blue_screen_of_death(gfx: &mut Graphics) -> ! {
    gfx.clear(Color::rgb(0, 0, 128).to_u32());

    let screen_w = gfx.width;
    let screen_h = gfx.height;

    let title = "Setop MiUX!";
    let title_width = title.len() * tamzen_font::FONT_WIDTH;
    let title_x = (screen_w - title_width) / 2;
    let title_y = screen_h / 2 - 60;
    gfx.draw_text(title_x, title_y, title, Color::WHITE.to_u32(), Color::rgb(0, 0, 128).to_u32());

    let error_msg = "Error code: 00000x3";
    let error_width = error_msg.len() * tamzen_font::FONT_WIDTH;
    let error_x = (screen_w - error_width) / 2;
    let error_y = screen_h / 2 - 20;
    gfx.draw_text(error_x, error_y, error_msg, Color::WHITE.to_u32(), Color::rgb(0, 0, 128).to_u32());

    let restart_msg = "Please restart you PC!";
    let restart_width = restart_msg.len() * tamzen_font::FONT_WIDTH;
    let restart_x = (screen_w - restart_width) / 2;
    let restart_y = screen_h / 2 + 20;
    gfx.draw_text(restart_x, restart_y, restart_msg, Color::WHITE.to_u32(), Color::rgb(0, 0, 128).to_u32());

    gfx.flush();

    loop {
        unsafe { core::arch::asm!("hlt") }
    }
}

fn show_blue_screen_stop_gui(gfx: &mut Graphics) -> ! {
    gfx.clear(Color::rgb(0, 0, 128).to_u32());

    let screen_w = gfx.width;
    let screen_h = gfx.height;

    let title = "GUI STOP!";
    let title_width = title.len() * tamzen_font::FONT_WIDTH;
    let title_x = (screen_w - title_width) / 2;
    let title_y = screen_h / 2 - 60;
    gfx.draw_text(title_x, title_y, title, Color::WHITE.to_u32(), Color::rgb(0, 0, 128).to_u32());

    let error_msg = "Error code: 00000x4c";
    let error_width = error_msg.len() * tamzen_font::FONT_WIDTH;
    let error_x = (screen_w - error_width) / 2;
    let error_y = screen_h / 2 - 20;
    gfx.draw_text(error_x, error_y, error_msg, Color::WHITE.to_u32(), Color::rgb(0, 0, 128).to_u32());

    let restart_msg = "GUI system halted. System stopped.";
    let restart_width = restart_msg.len() * tamzen_font::FONT_WIDTH;
    let restart_x = (screen_w - restart_width) / 2;
    let restart_y = screen_h / 2 + 20;
    gfx.draw_text(restart_x, restart_y, restart_msg, Color::WHITE.to_u32(), Color::rgb(0, 0, 128).to_u32());

    gfx.flush();

    loop {
        unsafe { core::arch::asm!("hlt") }
    }
}

fn draw_tutorial_window(gfx: &mut Graphics) {
    unsafe {
        if !TUTORIAL_OPEN { return; }

        let screen_w = gfx.width;
        let screen_h = gfx.height;

        let win_w = 500;
        let win_h = 370;
        let win_x = (screen_w - win_w) / 2;
        let win_y = (screen_h - win_h) / 2;

        gfx.fill_rect(win_x, win_y, win_w, win_h, Color::rgb(235, 235, 235).to_u32());
        gfx.draw_rect_border(win_x, win_y, win_w, win_h, Color::rgb(87, 87, 87).to_u32());
        gfx.draw_rect_border(win_x + 1, win_y + 1, win_w - 2, win_h - 2, Color::WHITE.to_u32());

        let title_bar_h = 30;
        gfx.fill_rect(win_x + 2, win_y + 2, win_w - 4, title_bar_h, Color::rgb(90, 90, 90).to_u32());
        gfx.draw_text(win_x + 10, win_y + 8, "Welcome to MiUX!", Color::WHITE.to_u32(), Color::rgb(90, 90, 90).to_u32());

        wm::draw_raised_rect(gfx, win_x + win_w - 25, win_y + 5, 20, 20);
        gfx.draw_text(win_x + win_w - 19, win_y + 9, "X", Color::BLACK.to_u32(), Color::rgb(212, 208, 200).to_u32());

        match TUTORIAL_STEP {
            0 => {
                gfx.draw_text(win_x + 20, win_y + 60, "Welcome!", Color::BLACK.to_u32(), Color::rgb(235, 235, 235).to_u32());
                gfx.draw_text(win_x + 20, win_y + 90, "Click the next button to get started.", Color::rgb(80, 80, 80).to_u32(), Color::rgb(235, 235, 235).to_u32());

                wm::draw_raised_rect(gfx, win_x + win_w - 90, win_y + win_h - 45, 80, 25);
                gfx.draw_text(win_x + win_w - 65, win_y + win_h - 39, "Next", Color::BLACK.to_u32(), Color::rgb(212, 208, 200).to_u32());
            }
            1 => {
                gfx.draw_text(win_x + 20, win_y + 60, "1 - New Aero-style interface", Color::rgb(0, 80, 0).to_u32(), Color::rgb(235, 235, 235).to_u32());
                gfx.draw_text(win_x + 30, win_y + 90, "A new system interface! Those same Aero-style", Color::rgb(80, 80, 80).to_u32(), Color::rgb(235, 235, 235).to_u32());
                gfx.draw_text(win_x + 30, win_y + 110, "windows with that same transparent interface.", Color::rgb(80, 80, 80).to_u32(), Color::rgb(235, 235, 235).to_u32());

                wm::draw_raised_rect(gfx, win_x + 20, win_y + win_h - 45, 80, 25);
                gfx.draw_text(win_x + 45, win_y + win_h - 39, "Prev", Color::BLACK.to_u32(), Color::rgb(212, 208, 200).to_u32());

                wm::draw_raised_rect(gfx, win_x + win_w - 90, win_y + win_h - 45, 80, 25);
                gfx.draw_text(win_x + win_w - 65, win_y + win_h - 39, "Next", Color::BLACK.to_u32(), Color::rgb(212, 208, 200).to_u32());
            }
            2 => {
                gfx.draw_text(win_x + 20, win_y + 60, "2 - Customizing the OS", Color::rgb(0, 80, 0).to_u32(), Color::rgb(235, 235, 235).to_u32());
                gfx.draw_text(win_x + 30, win_y + 90, "Go to settings, to change the wallpaper,", Color::rgb(80, 80, 80).to_u32(), Color::rgb(235, 235, 235).to_u32());
                gfx.draw_text(win_x + 30, win_y + 110, "click the corresponding button.", Color::rgb(80, 80, 80).to_u32(), Color::rgb(235, 235, 235).to_u32());

                wm::draw_raised_rect(gfx, win_x + 20, win_y + win_h - 45, 80, 25);
                gfx.draw_text(win_x + 45, win_y + win_h - 39, "Prev", Color::BLACK.to_u32(), Color::rgb(212, 208, 200).to_u32());

                wm::draw_raised_rect(gfx, win_x + win_w - 90, win_y + win_h - 45, 80, 25);
                gfx.draw_text(win_x + win_w - 65, win_y + win_h - 39, "Next", Color::BLACK.to_u32(), Color::rgb(212, 208, 200).to_u32());
            }
            3 => {
                gfx.draw_text(win_x + 20, win_y + 60, "3 - System terminal", Color::rgb(0, 80, 0).to_u32(), Color::rgb(235, 235, 235).to_u32());
                gfx.draw_text(win_x + 30, win_y + 90, "The system terminal is a tool that allows you", Color::rgb(80, 80, 80).to_u32(), Color::rgb(235, 235, 235).to_u32());
                gfx.draw_text(win_x + 30, win_y + 110, "to manage the system! For full command", Color::rgb(80, 80, 80).to_u32(), Color::rgb(235, 235, 235).to_u32());
                gfx.draw_text(win_x + 30, win_y + 130, "access, enter the help command.", Color::rgb(80, 80, 80).to_u32(), Color::rgb(235, 235, 235).to_u32());

                wm::draw_raised_rect(gfx, win_x + 20, win_y + win_h - 45, 80, 25);
                gfx.draw_text(win_x + 45, win_y + win_h - 39, "Prev", Color::BLACK.to_u32(), Color::rgb(212, 208, 200).to_u32());

                wm::draw_raised_rect(gfx, win_x + win_w - 90, win_y + win_h - 45, 80, 25);
                gfx.draw_text(win_x + win_w - 65, win_y + win_h - 39, "Next", Color::BLACK.to_u32(), Color::rgb(212, 208, 200).to_u32());
            }
            4 => {
                gfx.draw_text(win_x + 20, win_y + 60, "4 - Creating folders on the desktop", Color::rgb(0, 80, 0).to_u32(), Color::rgb(235, 235, 235).to_u32());
                gfx.draw_text(win_x + 30, win_y + 90, "There are two ways to create a folder.", Color::rgb(80, 80, 80).to_u32(), Color::rgb(235, 235, 235).to_u32());
                gfx.draw_text(win_x + 30, win_y + 120, "1 - Right-click on the desktop and select", Color::rgb(80, 80, 80).to_u32(), Color::rgb(235, 235, 235).to_u32());
                gfx.draw_text(win_x + 30, win_y + 140, "    Create new folder. Enter the folder's name", Color::rgb(80, 80, 80).to_u32(), Color::rgb(235, 235, 235).to_u32());
                gfx.draw_text(win_x + 30, win_y + 160, "    and click OK.", Color::rgb(80, 80, 80).to_u32(), Color::rgb(235, 235, 235).to_u32());
                gfx.draw_text(win_x + 30, win_y + 190, "2 - Open the terminal and enter the command", Color::rgb(80, 80, 80).to_u32(), Color::rgb(235, 235, 235).to_u32());
                gfx.draw_text(win_x + 30, win_y + 210, "    new folder.", Color::rgb(80, 80, 80).to_u32(), Color::rgb(235, 235, 235).to_u32());

                wm::draw_raised_rect(gfx, win_x + 20, win_y + win_h - 45, 80, 25);
                gfx.draw_text(win_x + 45, win_y + win_h - 39, "Prev", Color::BLACK.to_u32(), Color::rgb(212, 208, 200).to_u32());

                wm::draw_raised_rect(gfx, win_x + win_w - 90, win_y + win_h - 45, 80, 25);
                gfx.draw_text(win_x + win_w - 70, win_y + win_h - 39, "Finish", Color::BLACK.to_u32(), Color::rgb(212, 208, 200).to_u32());
            }
            _ => {}
        }
    }
}

fn handle_tutorial_click(gfx: &Graphics, cursor_x: usize, cursor_y: usize) -> bool {
    unsafe {
        if !TUTORIAL_OPEN { return false; }

        let screen_w = gfx.width;
        let screen_h = gfx.height;
        let win_w = 500;
        let win_h = 370;
        let win_x = (screen_w - win_w) / 2;
        let win_y = (screen_h - win_h) / 2;

        let close_x = win_x + win_w - 25;
        if cursor_x >= close_x && cursor_x <= close_x + 20 &&
           cursor_y >= win_y + 5 && cursor_y <= win_y + 25 {
            TUTORIAL_OPEN = false;
            return true;
        }

        match TUTORIAL_STEP {
            0 => {
                let next_x = win_x + win_w - 90;
                if cursor_x >= next_x && cursor_x <= next_x + 80 &&
                   cursor_y >= win_y + win_h - 45 && cursor_y <= win_y + win_h - 20 {
                    TUTORIAL_STEP = 1;
                    return true;
                }
            }
            1 | 2 | 3 => {
                let prev_x = win_x + 20;
                let next_x = win_x + win_w - 90;

                if cursor_x >= prev_x && cursor_x <= prev_x + 80 &&
                   cursor_y >= win_y + win_h - 45 && cursor_y <= win_y + win_h - 20 {
                    TUTORIAL_STEP -= 1;
                    return true;
                }

                if cursor_x >= next_x && cursor_x <= next_x + 80 &&
                   cursor_y >= win_y + win_h - 45 && cursor_y <= win_y + win_h - 20 {
                    TUTORIAL_STEP += 1;
                    return true;
                }
            }
            4 => {
                let prev_x = win_x + 20;
                let finish_x = win_x + win_w - 90;

                if cursor_x >= prev_x && cursor_x <= prev_x + 80 &&
                   cursor_y >= win_y + win_h - 45 && cursor_y <= win_y + win_h - 20 {
                    TUTORIAL_STEP = 3;
                    return true;
                }

                if cursor_x >= finish_x && cursor_x <= finish_x + 80 &&
                   cursor_y >= win_y + win_h - 45 && cursor_y <= win_y + win_h - 20 {
                    TUTORIAL_OPEN = false;
                    return true;
                }
            }
            _ => {}
        }

        false
    }
}

fn open_tutorial() -> &'static str {
    unsafe {
        TUTORIAL_OPEN = true;
        TUTORIAL_STEP = 0;
    }
    "Opening tutorial window..."
}

fn switch_to_wm2_theme() -> &'static str {
    unsafe {
        USE_WM2_THEME = true;
        "Theme switched to wm2.rs! New Aero-style interface activated!"
    }
}

fn call_app(app_name: &str, args: &str) -> &'static str {
    unsafe {
        APP_RESULT = [0; 256];

        match app_name {
            "calc" | "calculator" => apps::calculator(args),
            "echo" => apps::echo(args),
            "strlen" | "len" => apps::strlen(args),
            "reverse" | "rev" => apps::reverse(args),
            "to-upper" | "upper" => apps::to_upper(args),
            "to-lower" | "lower" => apps::to_lower(args),
            "greet" => apps::greet(args),
            "cre-tool" | "cretool" => apps::cre_tool(args),
            "list-apps" | "apps" => apps::list_apps(),
            "gl" => {
                let result = gl::gl_start();
                GL_WINDOW_REQUESTED = true;
                result
            }
            "gl-stop" => gl::gl_stop(),
            _ => {
                let msg = "App not found! Use 'list-apps' to see available apps.";
                let bytes = msg.as_bytes();
                let len = bytes.len().min(255);
                APP_RESULT[..len].copy_from_slice(&bytes[..len]);
                APP_RESULT[len] = 0;
                core::str::from_utf8(&APP_RESULT[..len]).unwrap_or("")
            }
        }
    }
}

fn draw_login_window(gfx: &mut Graphics) {
    let screen_w = gfx.width;
    let screen_h = gfx.height;

    unsafe {
        if let Some(ref bmp) = LOGIN_IMAGE {
            bmp.draw_as_wallpaper(gfx);
        } else {
            let wallpaper = WALLPAPER;
            gfx.clear(wallpaper.to_u32());
        }
    }

    gfx.fill_rect(0, 0, screen_w, screen_h, Color::rgb(0, 0, 0).to_u32());

    let login_w = 380;
    let login_h = 300;
    let login_x = (screen_w - login_w) / 2;
    let login_y = (screen_h - login_h) / 2;

    gfx.fill_rect(login_x, login_y, login_w, login_h, Color::rgb(235, 235, 235).to_u32());
    gfx.draw_rect_border(login_x, login_y, login_w, login_h, Color::rgb(49, 49, 49).to_u32());
    gfx.draw_rect_border(login_x + 1, login_y + 1, login_w - 2, login_h - 2, Color::WHITE.to_u32());

    let title_bar_h = 40;
    gfx.fill_rect(login_x + 2, login_y + 2, login_w - 4, title_bar_h, Color::rgb(70, 70, 70).to_u32());
    gfx.draw_text(login_x + 20, login_y + 12, "Login to MiUX beta", Color::WHITE.to_u32(), Color::rgb(70, 70, 70).to_u32());

    let fields_y = login_y + 60;

    gfx.draw_text(login_x + 30, fields_y + 10, "Username:", Color::BLACK.to_u32(), Color::rgb(235, 235, 235).to_u32());
    gfx.fill_rect(login_x + 30, fields_y + 30, 320, 30, Color::WHITE.to_u32());
    gfx.draw_rect_border(login_x + 30, fields_y + 30, 320, 30, Color::rgb(150, 150, 150).to_u32());

    unsafe {
        let username_str = core::str::from_utf8(&LOGIN_USERNAME_INPUT[..LOGIN_USERNAME_LEN]).unwrap_or("");
        gfx.draw_text(login_x + 35, fields_y + 37, username_str, Color::BLACK.to_u32(), Color::WHITE.to_u32());
        if !LOGIN_IS_PASSWORD {
            gfx.draw_text(login_x + 35 + username_str.len() * 8, fields_y + 37, "|", Color::rgb(41, 41, 41).to_u32(), Color::WHITE.to_u32());
        }
    }

    gfx.draw_text(login_x + 30, fields_y + 80, "Password:", Color::BLACK.to_u32(), Color::rgb(235, 235, 235).to_u32());
    gfx.fill_rect(login_x + 30, fields_y + 100, 320, 30, Color::WHITE.to_u32());
    gfx.draw_rect_border(login_x + 30, fields_y + 100, 320, 30, Color::rgb(150, 150, 150).to_u32());

    unsafe {
        let pass_len = LOGIN_PASSWORD_LEN;
        let mut stars = [b'*'; 32];
        for i in 0..pass_len.min(31) {
            stars[i] = b'*';
        }
        let stars_str = core::str::from_utf8(&stars[..pass_len.min(31)]).unwrap_or("");
        gfx.draw_text(login_x + 35, fields_y + 107, stars_str, Color::BLACK.to_u32(), Color::WHITE.to_u32());
        if LOGIN_IS_PASSWORD {
            let pos = if pass_len < 31 { pass_len } else { 31 };
            gfx.draw_text(login_x + 35 + pos * 8, fields_y + 107, "|", Color::rgb(71, 71, 71).to_u32(), Color::WHITE.to_u32());
        }
    }

    unsafe {
        if LOGIN_ERROR_LEN > 0 {
            let err_str = core::str::from_utf8(&LOGIN_ERROR_MSG[..LOGIN_ERROR_LEN]).unwrap_or("");
            gfx.draw_text(login_x + 30, fields_y + 150, err_str, Color::rgb(255, 0, 0).to_u32(), Color::rgb(235, 235, 235).to_u32());
        }
    }

    let btn_w = 120;
    let btn_h = 36;
    let btn_x = login_x + (login_w - btn_w) / 2;
    let btn_y = fields_y + 180;

    wm::draw_raised_rect(gfx, btn_x, btn_y, btn_w, btn_h);
    gfx.fill_rect(btn_x + 2, btn_y + 2, btn_w - 4, btn_h - 4, Color::rgb(36, 36, 36).to_u32());
    gfx.draw_text(btn_x + (btn_w - 5 * 8) / 2, btn_y + 10, "Login", Color::WHITE.to_u32(), Color::rgb(0, 120, 215).to_u32());

    gfx.draw_text(login_x + 30, login_y + login_h - 35, "Users: admin, system, user",
                  Color::rgb(100, 100, 100).to_u32(), Color::rgb(235, 235, 235).to_u32());
}

fn utf8_push(buf: &mut [u8], len: &mut usize, cap: usize, ch: char) -> bool {
    let mut tmp = [0u8; 4];
    let bytes = ch.encode_utf8(&mut tmp).as_bytes();
    if *len + bytes.len() > cap { return false; }
    buf[*len..*len + bytes.len()].copy_from_slice(bytes);
    *len += bytes.len();
    true
}

fn utf8_pop(buf: &[u8], len: &mut usize) {
    if *len == 0 { return; }
    *len -= 1;
    while *len > 0 && (buf[*len] & 0xC0) == 0x80 { *len -= 1; }
}

static mut NP_CUR: usize = usize::MAX;
static mut NP_ANCHOR: usize = usize::MAX;
static mut NP_DRAG: bool = false;

fn np_clen(b0: u8) -> usize {
    if b0 >= 0xF0 { 4 } else if b0 >= 0xE0 { 3 } else if b0 >= 0xC0 { 2 } else { 1 }
}

fn np_layout<F: FnMut(usize, usize, usize, usize)>(text: &[u8], len: usize, mut f: F) -> (usize, usize) {
    let mut row = 0usize;
    let mut col = 0usize;
    let mut i = 0usize;
    while i < len {
        let b0 = text[i];
        let end = if i + np_clen(b0) > len { len } else { i + np_clen(b0) };
        if b0 == b'\n' {
            f(i, end, row, col);
            row += 1;
            col = 0;
        } else {
            if col >= 40 { row += 1; col = 0; }
            f(i, end, row, col);
            col += 1;
        }
        i = end;
    }
    if col >= 40 { row += 1; col = 0; }
    (row, col)
}

unsafe fn np_fix() {
    unsafe {
        let len = NOTEPAD_LEN;
        if NP_CUR > len { NP_CUR = len; }
        while NP_CUR > 0 && NP_CUR < len && (NOTEPAD_TEXT[NP_CUR] & 0xC0) == 0x80 { NP_CUR -= 1; }
        if NP_ANCHOR != usize::MAX {
            if NP_ANCHOR > len { NP_ANCHOR = len; }
            while NP_ANCHOR > 0 && NP_ANCHOR < len && (NOTEPAD_TEXT[NP_ANCHOR] & 0xC0) == 0x80 { NP_ANCHOR -= 1; }
        }
    }
}

unsafe fn np_sel() -> Option<(usize, usize)> {
    unsafe {
        if NP_ANCHOR == usize::MAX || NP_ANCHOR == NP_CUR { return None; }
        if NP_ANCHOR < NP_CUR { Some((NP_ANCHOR, NP_CUR)) } else { Some((NP_CUR, NP_ANCHOR)) }
    }
}

unsafe fn np_delete_sel() -> bool {
    unsafe {
        np_fix();
        if let Some((a, b)) = np_sel() {
            let len = NOTEPAD_LEN;
            NOTEPAD_TEXT.copy_within(b..len, a);
            NOTEPAD_LEN = len - (b - a);
            NP_CUR = a;
            NP_ANCHOR = usize::MAX;
            true
        } else {
            NP_ANCHOR = usize::MAX;
            false
        }
    }
}

unsafe fn np_insert(ch: char) {
    unsafe {
        np_delete_sel();
        np_fix();
        let mut tmp = [0u8; 4];
        let bytes = ch.encode_utf8(&mut tmp).as_bytes();
        let n = bytes.len();
        let len = NOTEPAD_LEN;
        if len + n > 1023 { return; }
        let cur = NP_CUR;
        NOTEPAD_TEXT.copy_within(cur..len, cur + n);
        NOTEPAD_TEXT[cur..cur + n].copy_from_slice(bytes);
        NOTEPAD_LEN = len + n;
        NP_CUR = cur + n;
    }
}

unsafe fn np_backspace() {
    unsafe {
        if np_delete_sel() { return; }
        np_fix();
        let cur = NP_CUR;
        if cur == 0 { return; }
        let mut p = cur - 1;
        while p > 0 && (NOTEPAD_TEXT[p] & 0xC0) == 0x80 { p -= 1; }
        let len = NOTEPAD_LEN;
        NOTEPAD_TEXT.copy_within(cur..len, p);
        NOTEPAD_LEN = len - (cur - p);
        NP_CUR = p;
    }
}

unsafe fn np_hit_test(mx: usize, my: usize, ncx: usize, ncy: usize) -> usize {
    unsafe {
        let len = NOTEPAD_LEN;
        let text = &NOTEPAD_TEXT;
        let ox = ncx + 10;
        let oy = ncy + 32 + 50;
        let trow = if my > oy { (my - oy) / 16 } else { 0 };
        let tcol = if mx > ox { (mx - ox + 4) / 8 } else { 0 };
        let mut result = len;
        let mut found = false;
        np_layout(text, len, |s, _e, row, col| {
            if found { return; }
            if row > trow || (row == trow && (col >= tcol || text[s] == b'\n')) {
                result = s;
                found = true;
            }
        });
        result
    }
}

fn handle_login_input(sc: u8, username_input: &mut [u8; 32], username_len: &mut usize,
                      password_input: &mut [u8; 32], password_len: &mut usize,
                      is_password: &mut bool) -> bool {
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
    let c = c.map(mouse::apply_shift);

    if let Some(ch) = c {
        if ch == '\x08' {
            if *is_password {
                if *password_len > 0 { *password_len -= 1; }
            } else {
                if *username_len > 0 { *username_len -= 1; }
            }
        } else if ch == '\n' {
            return true;
        } else if ch == '\t' {
            *is_password = !*is_password;
        } else if (ch as u8) >= 0x20 {
            if *is_password {
                if *password_len < 31 {
                    password_input[*password_len] = ch as u8;
                    *password_len += 1;
                }
            } else {
                if *username_len < 31 {
                    username_input[*username_len] = ch as u8;
                    *username_len += 1;
                }
            }
        }
    }
    false
}

fn user_add(username: &str, password: &str, is_admin: bool) -> &'static str {
    if users::create_user(username, password, is_admin) {
        if is_admin {
            "Admin created successfully!"
        } else {
            "User created successfully!"
        }
    } else {
        "Error: User already exists or limit reached!"
    }
}

fn user_del(username: &str) -> &'static str {
    if users::delete_user(username) {
        "User deleted successfully!"
    } else {
        "Error: User not found or is admin!"
    }
}

fn user_pass(username: &str, new_password: &str) -> &'static str {
    if users::change_password(username, new_password) {
        "Password changed successfully!"
    } else {
        "Error: User not found!"
    }
}

fn user_whoami() -> &'static str {
    unsafe {
        let name = users::get_current_username();
        let home = users::get_current_user_home();
        let is_admin = users::is_admin();
        let role = if is_admin { "Admin" } else { "User" };

        static mut OUTPUT: [u8; 256] = [0; 256];
        let mut pos = 0;
        let lines = [
            "Name: ",
            "\nRole: ",
            "\nHome: ",
            "\n",
        ];
        let values = [name.as_bytes(), role.as_bytes(), home.as_bytes()];

        for i in 0..3 {
            for b in lines[i].as_bytes() {
                if pos < 255 { OUTPUT[pos] = *b; pos += 1; }
            }
            for b in values[i] {
                if pos < 255 { OUTPUT[pos] = *b; pos += 1; }
            }
        }
        OUTPUT[pos] = 0;
        core::str::from_utf8(&OUTPUT[..pos]).unwrap_or("")
    }
}

fn update_fat32_disk_list() -> &'static str {
    unsafe {
        let count = ata::disk_count();
        DISK_COUNT = count;

        if count == 0 {
            return "No disks found!";
        }

        let mut pos = 0;
        let header = b"=== DISKS ===\n";
        for b in header { if pos < 2047 { FILE_MANAGER_DISK_LIST[pos] = *b; pos += 1; } }

        for i in 0..count {
            if let Some(disk) = ata::get_disk(i) {
                if disk.present {
                    let model = core::str::from_utf8(&disk.model).unwrap_or("Unknown");
                    let model_trimmed = model.trim_end_matches('\0');

                    let type_str = match disk.dev_type {
                        ata::DeviceType::ATA => "ATA HDD",
                        ata::DeviceType::ATAPI => "ATAPI CD-ROM",
                        _ => "Unknown",
                    };

                    let mounted = if FAT32_MOUNTED[i] { " [FAT32]" } else { "" };

                    let num_str = num_to_str(i + 1);
                    for b in num_str.as_bytes() { if pos < 2047 { FILE_MANAGER_DISK_LIST[pos] = *b; pos += 1; } }
                    if pos < 2047 { FILE_MANAGER_DISK_LIST[pos] = b'.'; pos += 1; }
                    if pos < 2047 { FILE_MANAGER_DISK_LIST[pos] = b' '; pos += 1; }
                    for b in model_trimmed.as_bytes() { if pos < 2047 { FILE_MANAGER_DISK_LIST[pos] = *b; pos += 1; } }
                    if pos < 2047 { FILE_MANAGER_DISK_LIST[pos] = b' '; pos += 1; }
                    for b in type_str.as_bytes() { if pos < 2047 { FILE_MANAGER_DISK_LIST[pos] = *b; pos += 1; } }
                    for b in mounted.as_bytes() { if pos < 2047 { FILE_MANAGER_DISK_LIST[pos] = *b; pos += 1; } }
                    if pos < 2047 { FILE_MANAGER_DISK_LIST[pos] = b'\n'; pos += 1; }
                }
            }
        }

        FILE_MANAGER_DISK_LIST[pos] = 0;
        FILE_MANAGER_DISK_LIST_LEN = pos;
        core::str::from_utf8(&FILE_MANAGER_DISK_LIST[..pos]).unwrap_or("")
    }
}

fn mount_fat32_disk(disk_index: usize) -> bool {
    unsafe {
        if disk_index >= DISK_COUNT {
            return false;
        }

        if FAT32_MOUNTED[disk_index] {
            return true;
        }

        if let Some(fs) = fs::fat32::FAT32FS::new(disk_index) {
            FAT32_FILESYSTEMS[disk_index] = Some(fs);
            FAT32_MOUNTED[disk_index] = true;
            return true;
        }

        false
    }
}

fn read_fat32_disk(disk_index: usize) -> &'static str {
    unsafe {
        if disk_index >= DISK_COUNT {
            return "Invalid disk!";
        }

        if !FAT32_MOUNTED[disk_index] {
            return "Disk not mounted!";
        }

        if let Some(ref fs) = FAT32_FILESYSTEMS[disk_index] {
            return fs.list_files();
        }

        "Error reading disk!"
    }
}


static mut EXT4_TRIED: bool = false;
static mut EXT4_OK: bool = false;

fn try_mount_ext4() -> bool {
    unsafe {
        if EXT4_TRIED {
            return EXT4_OK;
        }
        EXT4_TRIED = true;
        EXT4_OK = fs::ext4::mount(0);
        EXT4_OK
    }
}

static mut FM_BOOTED_FROM_DISK: bool = false;
static mut FM_PATH_EDIT: bool = false;
static mut FM_PATH_BUF: [u8; 160] = [0; 160];
static mut FM_PATH_LEN: usize = 0;
static mut FM_PATH_ERR: &str = "";

const FM_BTN_Y: usize = 5;
const FM_BTN_W: usize = 120;
const FM_BTN_H: usize = 25;
const FM_BTN1_X: usize = 8;
const FM_BTN2_X: usize = 136;
const FM_PATH_Y: usize = 42;
const FM_PATH_H: usize = 24;
const FM_TILE_W: usize = 225;
const FM_TILE_H: usize = 62;
const FM_TILE_GAP: usize = 8;
const FM_TILES_Y: usize = 90;

fn fm_hit(px: usize, py: usize, x: usize, y: usize, w: usize, h: usize) -> bool {
    px >= x && px < x + w && py >= y && py < y + h
}

fn fm_tile_layout(win_w: usize, win_h: usize) -> (usize, usize) {
    let cols = ((win_w.saturating_sub(30) + FM_TILE_GAP) / (FM_TILE_W + FM_TILE_GAP)).max(1);
    let rows = (win_h.saturating_sub(FM_TILES_Y + 25) + FM_TILE_GAP) / (FM_TILE_H + FM_TILE_GAP);
    (cols, rows)
}

fn fm_tile_pos(i: usize, cols: usize) -> (usize, usize) {
    let col = i % cols;
    let row = i / cols;
    (15 + col * (FM_TILE_W + FM_TILE_GAP), FM_TILES_Y + row * (FM_TILE_H + FM_TILE_GAP))
}

fn fm_os_disk() -> Option<usize> {
    unsafe {
        let want = if FM_BOOTED_FROM_DISK { ata::DeviceType::ATA } else { ata::DeviceType::ATAPI };
        for i in 0..ata::disk_count() {
            if let Some(d) = ata::get_disk(i) {
                if d.present && d.dev_type == want {
                    return Some(i);
                }
            }
        }
        None
    }
}

fn fm_tail(s: &str, maxc: usize) -> &str {
    let n = s.chars().count();
    if n <= maxc {
        return s;
    }
    match s.char_indices().nth(n - maxc) {
        Some((i, _)) => &s[i..],
        None => s,
    }
}

fn fm_fmt_size(bytes: u64, out: &mut [u8; 40]) -> usize {
    let mut p = put_bytes(out, 0, b"Size: ");
    let gb: u64 = 1 << 30;
    let mb: u64 = 1 << 20;
    if bytes >= gb {
        p = put_bytes(out, p, num_to_str((bytes / gb) as usize).as_bytes());
        p = put_bytes(out, p, b".");
        let tenth = ((bytes % gb) * 10 / gb) as usize;
        p = put_bytes(out, p, num_to_str(tenth).as_bytes());
        p = put_bytes(out, p, b" GB");
    } else if bytes >= mb {
        p = put_bytes(out, p, num_to_str((bytes / mb) as usize).as_bytes());
        p = put_bytes(out, p, b" MB");
    } else if bytes >= 1024 {
        p = put_bytes(out, p, num_to_str((bytes / 1024) as usize).as_bytes());
        p = put_bytes(out, p, b" KB");
    } else {
        p = put_bytes(out, p, b"n/a");
    }
    p
}

fn fm_draw_disk_tile(gfx: &mut Graphics, x: usize, y: usize, i: usize, is_sys: bool) {
    unsafe {
        let disk = match ata::get_disk(i) {
            Some(d) => d,
            None => return,
        };
        let tile_bg = Color::rgb(240, 240, 240).to_u32();
        let border = if is_sys { Color::rgb(0, 120, 215).to_u32() } else { Color::rgb(173, 173, 173).to_u32() };
        gfx.fill_rect(x, y, FM_TILE_W, FM_TILE_H, tile_bg);
        gfx.draw_rect_border(x, y, FM_TILE_W, FM_TILE_H, border);
        gfx.draw_rect_border(x + 1, y + 1, FM_TILE_W - 2, FM_TILE_H - 2, Color::WHITE.to_u32());
        draw_icon(gfx, x + 4, y + 3, &ICON_HDD, 56);

        let tx = x + 68;
        let mut nb = [0u8; 40];
        let mut p = 0;
        if is_sys {
            p = put_bytes(&mut nb, p, b"System ");
        }
        p = put_bytes(&mut nb, p, b"Disk ");
        p = put_bytes(&mut nb, p, num_to_str(i + 1).as_bytes());
        p = put_bytes(&mut nb, p, b":/");
        let name = core::str::from_utf8(&nb[..p]).unwrap_or("Disk");
        gfx.draw_text(tx, y + 8, name, Color::BLACK.to_u32(), tile_bg);

        let mut sb = [0u8; 40];
        let sl = fm_fmt_size(disk.sectors.saturating_mul(disk.sector_size as u64), &mut sb);
        let size = core::str::from_utf8(&sb[..sl]).unwrap_or("");
        gfx.draw_text(tx, y + 26, size, Color::rgb(80, 80, 80).to_u32(), tile_bg);

        if FAT32_MOUNTED[i] {
            gfx.draw_text(tx, y + 43, "FAT32", Color::rgb(0, 130, 0).to_u32(), tile_bg);
        } else if disk.dev_type == ata::DeviceType::ATAPI {
            gfx.draw_text(tx, y + 43, "CD-ROM", Color::rgb(110, 110, 110).to_u32(), tile_bg);
        } else {
            gfx.draw_text(tx, y + 43, "Click to open", Color::rgb(110, 110, 110).to_u32(), tile_bg);
        }
    }
}

fn fm_path_begin_edit() {
    unsafe {
        FM_PATH_ERR = "";
        FM_PATH_EDIT = true;
        if FILE_MANAGER_MOUNTED {
            let mut pb = [0u8; 160];
            let pl = FM_VIEW.path_slash(&mut pb);
            FM_PATH_BUF[..pl].copy_from_slice(&pb[..pl]);
            FM_PATH_LEN = pl;
        } else {
            let t = b"Disk 1:/";
            FM_PATH_BUF[..t.len()].copy_from_slice(t);
            FM_PATH_LEN = t.len();
        }
    }
}

fn fm_sc_char(sc: u8) -> Option<char> {
    match sc {
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
    }
}

fn fm_starts_ci(s: &str, pre: &str) -> bool {
    s.len() >= pre.len() && s.as_bytes()[..pre.len()].eq_ignore_ascii_case(pre.as_bytes())
}

unsafe fn fm_view_restore(disk: usize, depth: usize, cl: [u32; DV_MAX_DEPTH + 1], names: [[u8; 13]; DV_MAX_DEPTH + 1], nl: [usize; DV_MAX_DEPTH + 1]) {
    FM_VIEW.disk = disk;
    FM_VIEW.depth = depth;
    FM_VIEW.cl = cl;
    FM_VIEW.names = names;
    FM_VIEW.name_len = nl;
    FM_VIEW.scroll = 0;
    FM_VIEW.reload();
}

fn fm_goto_path(input: &str) -> Result<(), &'static str> {
    unsafe {
        let p = input.trim();
        let colon = match p.find(':') {
            Some(c) => c,
            None => return Err("Use: Disk 1:/folder"),
        };
        let mut head = p[..colon].trim();
        let rest = &p[colon + 1..];
        if fm_starts_ci(head, "system") {
            head = head[6..].trim_start();
        }
        if !fm_starts_ci(head, "disk") {
            return Err("Path: Disk 1:/folder");
        }
        let num = head[4..].trim();
        if num.is_empty() {
            return Err("Disk number missing");
        }
        let mut n = 0usize;
        for b in num.bytes() {
            if !b.is_ascii_digit() {
                return Err("Bad disk number");
            }
            n = n * 10 + (b - b'0') as usize;
            if n > 99 {
                return Err("No such disk");
            }
        }
        DISK_COUNT = ata::disk_count();
        if n == 0 || n > DISK_COUNT {
            return Err("No such disk");
        }
        let idx = n - 1;
        match ata::get_disk(idx) {
            Some(d) if d.present => {}
            _ => return Err("No such disk"),
        }
        if !mount_fat32_disk(idx) {
            return Err("Not a FAT32 disk");
        }

        let o_disk = FM_VIEW.disk;
        let o_depth = FM_VIEW.depth;
        let o_cl = FM_VIEW.cl;
        let o_names = FM_VIEW.names;
        let o_nl = FM_VIEW.name_len;

        if !FM_VIEW.open_root(idx) {
            fm_view_restore(o_disk, o_depth, o_cl, o_names, o_nl);
            return Err("Not a FAT32 disk");
        }
        for comp in rest.split(|c| c == '/' || c == '\\') {
            let comp = comp.trim();
            if comp.is_empty() || comp == "." {
                continue;
            }
            if comp == ".." {
                FM_VIEW.up();
                continue;
            }
            let mut found: Option<usize> = None;
            for k in 0..FM_VIEW.count {
                let e = &FM_VIEW.entries[k];
                if e.is_dir && e.name_len == comp.len()
                    && e.name[..e.name_len].eq_ignore_ascii_case(comp.as_bytes()) {
                    found = Some(k);
                    break;
                }
            }
            match found {
                Some(k) => {
                    let before = FM_VIEW.depth;
                    FM_VIEW.enter(k);
                    if FM_VIEW.depth == before {
                        fm_view_restore(o_disk, o_depth, o_cl, o_names, o_nl);
                        return Err("Path is too deep");
                    }
                }
                None => {
                    fm_view_restore(o_disk, o_depth, o_cl, o_names, o_nl);
                    return Err("Folder not found");
                }
            }
        }
        FILE_MANAGER_MOUNTED = true;
        FILE_MANAGER_DIALOG_OPEN = false;
        FM_MSG_LEN = 0;
        Ok(())
    }
}

fn fm_path_confirm() {
    unsafe {
        let s = core::str::from_utf8(&FM_PATH_BUF[..FM_PATH_LEN]).unwrap_or("");
        match fm_goto_path(s) {
            Ok(()) => {
                FM_PATH_EDIT = false;
                FM_PATH_ERR = "";
            }
            Err(e) => {
                FM_PATH_ERR = e;
            }
        }
    }
}

fn draw_file_manager_content(gfx: &mut Graphics, cx: usize, cy: usize, win_w: usize, win_h: usize) {
    unsafe {
        let bg = Color::rgb(245, 245, 245).to_u32();
        gfx.fill_rect(cx, cy, win_w, win_h, bg);

        let hdr = Color::rgb(200, 200, 200).to_u32();
        let face = Color::rgb(212, 208, 200).to_u32();
        gfx.fill_rect(cx, cy, win_w, 35, hdr);
        wm::draw_raised_rect(gfx, cx + FM_BTN1_X, cy + FM_BTN_Y, FM_BTN_W, FM_BTN_H);
        gfx.draw_text(cx + FM_BTN1_X + 8, cy + FM_BTN_Y + 5, "Open Terminal", Color::BLACK.to_u32(), face);
        wm::draw_raised_rect(gfx, cx + FM_BTN2_X, cy + FM_BTN_Y, FM_BTN_W, FM_BTN_H);
        gfx.draw_text(cx + FM_BTN2_X + 8, cy + FM_BTN_Y + 5, "About FileMGR", Color::BLACK.to_u32(), face);
        if FM_PATH_EDIT && !FM_PATH_ERR.is_empty() {
            let ex = cx + FM_BTN2_X + FM_BTN_W + 12;
            let maxc = (cx + win_w).saturating_sub(ex + 6) / 8;
            gfx.draw_text(ex, cy + 10, fm_tail(FM_PATH_ERR, maxc), Color::rgb(180, 0, 0).to_u32(), hdr);
        }
        gfx.fill_rect(cx, cy + 35, win_w, 1, Color::rgb(180, 180, 180).to_u32());

        let content_y = cy + 45;

        let px = cx + 10;
        let py = cy + FM_PATH_Y;
        let pw = win_w.saturating_sub(20);
        gfx.fill_rect(px, py, pw, FM_PATH_H, Color::WHITE.to_u32());
        let pborder = if FM_PATH_EDIT { Color::rgb(0, 120, 215).to_u32() } else { Color::rgb(122, 122, 122).to_u32() };
        gfx.draw_rect_border(px, py, pw, FM_PATH_H, pborder);
        let pmax = pw.saturating_sub(14) / 8;
        if FM_PATH_EDIT {
            let full = core::str::from_utf8(&FM_PATH_BUF[..FM_PATH_LEN]).unwrap_or("");
            let shown = fm_tail(full, pmax.saturating_sub(1));
            gfx.draw_text(px + 6, py + 4, shown, Color::BLACK.to_u32(), Color::WHITE.to_u32());
            gfx.draw_text(px + 6 + shown.chars().count() * 8, py + 4, "|", Color::BLACK.to_u32(), Color::WHITE.to_u32());
        } else if FILE_MANAGER_MOUNTED {
            let mut pb = [0u8; 160];
            let pl = FM_VIEW.path_slash(&mut pb);
            let full = core::str::from_utf8(&pb[..pl]).unwrap_or("Disk");
            gfx.draw_text(px + 6, py + 4, fm_tail(full, pmax), Color::BLACK.to_u32(), Color::WHITE.to_u32());
        } else {
            gfx.draw_text(px + 6, py + 4, "Computer", Color::BLACK.to_u32(), Color::WHITE.to_u32());
        }

        if FILE_MANAGER_DIALOG_OPEN {
            let dialog_w = 300;
            let dialog_h = 130;
            let dialog_x = cx + (win_w - dialog_w) / 2;
            let dialog_y = cy + (win_h - dialog_h) / 2;

            gfx.fill_rect(dialog_x, dialog_y, dialog_w, dialog_h, Color::rgb(235, 235, 235).to_u32());
            gfx.draw_rect_border(dialog_x, dialog_y, dialog_w, dialog_h, Color::rgb(87, 87, 87).to_u32());
            gfx.draw_rect_border(dialog_x + 1, dialog_y + 1, dialog_w - 2, dialog_h - 2, Color::WHITE.to_u32());

            gfx.draw_text(dialog_x + 20, dialog_y + 20, "Mount this disk?", Color::BLACK.to_u32(), Color::rgb(235, 235, 235).to_u32());

            let disk_num = FILE_MANAGER_DIALOG_DISK + 1;
            let disk_str = num_to_str(disk_num);
            let mut msg = [0u8; 64];
            let mut p = 0;
            let prefix = b"Disk ";
            for b in prefix { if p < 63 { msg[p] = *b; p += 1; } }
            for b in disk_str.as_bytes() { if p < 63 { msg[p] = *b; p += 1; } }
            msg[p] = 0;
            let msg_str = core::str::from_utf8(&msg[..p]).unwrap_or("Disk ?");
            gfx.draw_text(dialog_x + 20, dialog_y + 45, msg_str, Color::rgb(80, 80, 80).to_u32(), Color::rgb(235, 235, 235).to_u32());

            wm::draw_raised_rect(gfx, dialog_x + 40, dialog_y + 75, 80, 28);
            gfx.draw_text(dialog_x + 70, dialog_y + 81, "OK", Color::BLACK.to_u32(), Color::rgb(212, 208, 200).to_u32());

            wm::draw_raised_rect(gfx, dialog_x + 170, dialog_y + 75, 80, 28);
            gfx.draw_text(dialog_x + 195, dialog_y + 81, "NO", Color::BLACK.to_u32(), Color::rgb(212, 208, 200).to_u32());

            return;
        }

        if FILE_MANAGER_MOUNTED {
            gfx.fill_rect(cx + 10, content_y + 30, win_w - 20, 1, Color::rgb(200, 200, 200).to_u32());

            draw_dir_rows(gfx, &FM_VIEW, cx + 15, content_y + 45, fm_visible_rows(win_h), bg);
            if FM_VIEW.total_items() == 0 {
                gfx.draw_text(cx + 15, content_y + 45, "(empty folder) - right click for menu", Color::rgb(120, 120, 120).to_u32(), bg);
            }

            wm::draw_raised_rect(gfx, cx + 10, cy + win_h - 35, 80, 25);
            gfx.draw_text(cx + 30, cy + win_h - 29, "Back", Color::BLACK.to_u32(), Color::rgb(212, 208, 200).to_u32());
            wm::draw_raised_rect(gfx, cx + 100, cy + win_h - 35, 80, 25);
            gfx.draw_text(cx + 128, cy + win_h - 29, "Up", Color::BLACK.to_u32(), Color::rgb(212, 208, 200).to_u32());
            if FM_MSG_LEN > 0 {
                let ms = core::str::from_utf8(&FM_MSG[..FM_MSG_LEN]).unwrap_or("");
                let mc = if FM_MSG_OK { Color::rgb(0, 130, 0).to_u32() } else { Color::rgb(180, 0, 0).to_u32() };
                gfx.draw_text(cx + 195, cy + win_h - 29, ms, mc, bg);
            }

            return;
        }

        gfx.fill_rect(cx + 10, content_y + 30, win_w - 20, 1, Color::rgb(200, 200, 200).to_u32());

        let _disk_info = update_fat32_disk_list();
        let (cols, rows) = fm_tile_layout(win_w, win_h);
        let os_disk = fm_os_disk();

        for i in 0..DISK_COUNT {
            if i >= cols * rows { break; }
            if let Some(disk) = ata::get_disk(i) {
                if disk.present {
                    let (dx, dy) = fm_tile_pos(i, cols);
                    fm_draw_disk_tile(gfx, cx + dx, cy + dy, i, os_disk == Some(i));
                }
            }
        }

        let info_y = cy + win_h - 25;
        gfx.fill_rect(cx, info_y, win_w, 25, Color::rgb(235, 235, 235).to_u32());
        gfx.fill_rect(cx, info_y, win_w, 1, Color::rgb(200, 200, 200).to_u32());
        let count_str = num_to_str(DISK_COUNT);
        let mut info = [0u8; 128];
        let mut p = 0;
        for b in count_str.as_bytes() { if p < 127 { info[p] = *b; p += 1; } }
        let suffix = b" disk(s). Click a disk to open it.";
        for b in suffix { if p < 127 { info[p] = *b; p += 1; } }
        info[p] = 0;
        let info_str = core::str::from_utf8(&info[..p]).unwrap_or("Click a disk.");
        gfx.draw_text(cx + 10, info_y + 6, info_str, Color::rgb(100, 100, 100).to_u32(), Color::rgb(235, 235, 235).to_u32());
    }
}

fn draw_ext4_window_content(gfx: &mut Graphics, cx: usize, cy: usize, win_w: usize, win_h: usize) {
    unsafe {
        let bg = Color::rgb(245, 245, 245).to_u32();
        gfx.fill_rect(cx, cy, win_w, win_h, bg);

        gfx.fill_rect(cx, cy, win_w, 35, Color::rgb(60, 60, 120).to_u32());
        gfx.draw_text(cx + 10, cy + 10, "EXT4 Root Filesystem", Color::WHITE.to_u32(), Color::rgb(60, 60, 120).to_u32());
        gfx.fill_rect(cx, cy + 35, win_w, 1, Color::rgb(180, 180, 180).to_u32());

        let content_y = cy + 45;

        if !EXT4_OK {
            let msg = "EXT4 not mounted. Use terminal: ext4";
            gfx.draw_text(cx + 10, content_y + 10, msg, Color::rgb(200, 0, 0).to_u32(), bg);
            return;
        }

        let content = fs::ext4::list_files();
        let mut y_pos = content_y + 10;
        for line in content.split('\n') {
            if y_pos > cy + win_h - 30 { break; }
            if !line.is_empty() {
                let color = if line.starts_with("[DIR]") {
                    Color::rgb(0, 80, 200).to_u32()
                } else {
                    Color::BLACK.to_u32()
                };
                gfx.draw_text(cx + 10, y_pos, line, color, bg);
                y_pos += 18;
            }
        }
    }
}

fn media_open_image(idx: usize) {
    unsafe {
        if idx >= MEDIA_BMP_COUNT { return; }
        MEDIA_IMAGE_INDEX = idx;
        let (_name, data) = media_bmp_data(idx);
        MEDIA_LOADED_IMAGE = BmpImage::from_bytes(data);
        MEDIA_BROWSER_OPEN = false;
        MEDIA_NEEDS_REDRAW = true;
    }
}

fn draw_media_content(gfx: &mut Graphics, cx: usize, cy: usize, win_w: usize, win_h: usize) {
    unsafe {
        let bg = Color::rgb(28, 28, 32).to_u32();
        gfx.fill_rect(cx, cy, win_w, win_h, bg);

        gfx.fill_rect(cx, cy, win_w, 34, Color::rgb(45, 45, 55).to_u32());
        gfx.draw_text(cx + 10, cy + 9, "Media - BMP Viewer",
                      Color::WHITE.to_u32(),
                      Color::rgb(45, 45, 55).to_u32());

        wm::draw_raised_rect(gfx, cx + win_w - 100, cy + 5, 90, 24);
        gfx.draw_text(cx + win_w - 88, cy + 10, "Browse",
                      Color::BLACK.to_u32(),
                      Color::rgb(212, 208, 200).to_u32());

        if MEDIA_BROWSER_OPEN {
            let list_x = cx + 20;
            let list_y = cy + 60;
            let list_w = win_w - 44;
            let item_h = 30;
            let visible_count = ((win_h - 100) / item_h).min(MEDIA_BMP_COUNT);

            gfx.fill_rect(list_x, list_y, list_w, (item_h * visible_count + 10) as usize,
                          Color::rgb(40, 40, 50).to_u32());
            gfx.draw_rect_border(list_x, list_y, list_w, (item_h * visible_count + 10) as usize,
                                 Color::rgb(120, 120, 140).to_u32());

            for i in 0..visible_count {
                let iy = list_y + 5 + i * item_h;
                let is_sel = i == MEDIA_IMAGE_INDEX;
                let item_bg = if is_sel {
                    Color::rgb(0, 120, 215).to_u32()
                } else {
                    Color::rgb(55, 55, 70).to_u32()
                };
                gfx.fill_rect(list_x + 5, iy, list_w - 10, item_h - 3, item_bg);
                let txt_color = if is_sel { Color::WHITE.to_u32() } else { Color::rgb(220, 220, 220).to_u32() };
                let (name, _data) = media_bmp_data(i);
                gfx.draw_text(list_x + 15, iy + 8, name, txt_color, item_bg);
            }
        } else {
            let view_x = cx + 10;
            let view_y = cy + 45;
            let view_w = win_w - 24;
            let view_h = win_h - 28 - 60;

            if let Some(ref bmp) = MEDIA_LOADED_IMAGE {
                let bw = bmp.width;
                let bh = bmp.height;
                let off_x = view_x + if view_w > bw { (view_w - bw) / 2 } else { 0 };
                let off_y = view_y + if view_h > bh { (view_h - bh) / 2 } else { 0 };

                let row_size = ((bw * 3 + 3) / 4) * 4;
                for dy in 0..bh.min(view_h) {
                    if off_y + dy >= cy + win_h - 60 { break; }
                    let bmp_y = (bh - 1).saturating_sub(dy);
                    let bmp_row_start = bmp_y * row_size;
                    for dx in 0..bw.min(view_w) {
                        let pixel_offset = bmp_row_start + dx * 3;
                        if pixel_offset + 2 < bmp.data.len() {
                            let b = bmp.data[pixel_offset] as u32;
                            let g = bmp.data[pixel_offset + 1] as u32;
                            let r = bmp.data[pixel_offset + 2] as u32;
                            let color = (r << 16) | (g << 8) | b;
                            gfx.put_pixel(off_x + dx, off_y + dy, color);
                        }
                    }
                }

                let info_y = cy + win_h - 55;
                gfx.fill_rect(cx + 5, info_y, win_w - 14, 30,
                              Color::rgb(35, 35, 45).to_u32());

                let (name, _data) = media_bmp_data(MEDIA_IMAGE_INDEX);
                let mut info_buf = [0u8; 128];
                let mut p = 0usize;
                for b in name.bytes() { if p < 127 { info_buf[p] = b; p += 1; } }
                if p < 127 { info_buf[p] = b' '; p += 1; }
                if p < 127 { info_buf[p] = b'|'; p += 1; }
                if p < 127 { info_buf[p] = b' '; p += 1; }
                let w_s = num_to_str(bw);
                for b in w_s.bytes() { if p < 127 { info_buf[p] = b; p += 1; } }
                if p < 127 { info_buf[p] = b'x'; p += 1; }
                let h_s = num_to_str(bh);
                for b in h_s.bytes() { if p < 127 { info_buf[p] = b; p += 1; } }

                let info_str = core::str::from_utf8(&info_buf[..p]).unwrap_or("");
                gfx.draw_text(cx + 12, info_y + 8, info_str,
                              Color::rgb(200, 220, 255).to_u32(),
                              Color::rgb(35, 35, 45).to_u32());

                wm::draw_raised_rect(gfx, cx + win_w - 4 - 110, info_y + 2, 100, 26);
                gfx.draw_text(cx + win_w - 4 - 100, info_y + 8, "Close Image",
                              Color::BLACK.to_u32(),
                              Color::rgb(212, 208, 200).to_u32());
            } else {
                gfx.fill_rect(view_x, view_y, view_w, view_h,
                              Color::rgb(15, 15, 20).to_u32());
                gfx.draw_text(view_x + 20, view_y + 20,
                              "Click 'Browse' to choose a BMP image",
                              Color::rgb(180, 180, 180).to_u32(),
                              Color::rgb(15, 15, 20).to_u32());
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
    heap::init();

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

            if installer::wanted(info_addr) {
                installer::run(&mut gfx, info_addr);
            }

            let mut vga = crate::vga::VGA::new();
            vga.clear();
            vga.write_string("MiUX booting...\n");
            vga.write_string("Initializing VESA framebuffer...\n");

            gfx.clear(Color::rgb(0, 0, 0).to_u32());

            boot_log(&mut gfx, "[ BOOT ]", "Vesa init...", "OK");
            boot_delay();

            boot_log(&mut gfx, "[ BOOT ]", "Load bmp img module...", "info");
            boot_delay();
            boot_log(&mut gfx, "[ BOOT ]", "bmp init...", "OK");
            boot_delay();

            boot_log(&mut gfx, "[ BOOT ]", "Keyboard init...", "OK");
            boot_delay();

            boot_log(&mut gfx, "[ BOOT ]", "Mouse init...", "OK");
            boot_delay();

            boot_log(&mut gfx, "[ BOOT ]", "ATA load...", "");
            boot_delay();
            boot_log(&mut gfx, "[ BOOT ]", "ATA init...", "OK");
            boot_delay();

            boot_log(&mut gfx, "[ BOOT ]", "SATA init...", "OK");
            boot_delay();

            boot_log(&mut gfx, "[ BOOT ]", "Load atapi...", "");
            boot_delay();
            boot_log(&mut gfx, "[ BOOT ]", "ATAPi...", "OK");
            boot_delay();

            boot_log(&mut gfx, "[ BOOT ]", "fat32 load...", "OK");
            boot_delay();
            boot_log(&mut gfx, "[ BOOT ]", "fat32 init...", "OK");
            boot_delay();

            timer::init();
            net::init();
            boot_log(&mut gfx, "[ BOOT ]", "Network init...", if net::net().is_some() { "OK" } else { "info" });
            boot_delay();

            boot_stage_logs(&mut gfx);
            boot_hw_info(&mut gfx, info_addr, fb.bpp as u32, fb.addr as usize);

            unsafe {
                BMP_IMAGE = BmpImage::from_bytes(IMG_BMP);
                BMP_IMAGE2 = BmpImage::from_bytes(IMG2_BMP);
                BMP_IMAGE3 = BmpImage::from_bytes(IMG3_BMP);
                ICON_TERMINAL = BmpImage::from_bytes(IC_BMP);
                ICON_FILES = BmpImage::from_bytes(IC0_BMP);
                ICON_SETTINGS = BmpImage::from_bytes(IC1_BMP);
                ICON_ABOUT = BmpImage::from_bytes(IC2_BMP);
                ICON_NOTEPAD = BmpImage::from_bytes(IC3_BMP);
                ICON_SNAKE = BmpImage::from_bytes(IC4_BMP);
                ICON_PAINT = BmpImage::from_bytes(IC5_BMP);
                ICON_FOLDER = BmpImage::from_bytes(FOLDER_BMP);
                ICON_HDD = BmpImage::from_bytes(HDD_BMP);
                ICON_FILE = BmpImage::from_bytes(FILE_BMP);
                ICON_TOOL = BmpImage::from_bytes(TOOL_BMP);
                ICON_SHORTCUT = BmpImage::from_bytes(SK_BMP);
                ABOUT_IMAGE = BmpImage::from_bytes(ABOUT_BMP);
                LOGIN_IMAGE = BmpImage::from_bytes(LOGIN_BMP);

                for y in 0..300 {
                    for x in 0..400 {
                        PAINT_CANVAS[y][x] = 0xFFFFFF;
                        PAINT_CANVAS_BACKUP[y][x] = 0xFFFFFF;
                    }
                }
                PAINT_COLOR = 0x000000;
                PAINT_PEN_SIZE = 1;
                PAINT_TOOL = 0;
                PAINT_IS_DRAWING = false;
                PAINT_PREV_X = 0;
                PAINT_PREV_Y = 0;
                CURRENT_FS_SECTION = 0;
                DESKTOP_FOLDERS_COUNT = 0;
                for i in 0..10 {
                    FOLDER_FILES_COUNT[i] = 0;
                }
                SAVED_PAINTINGS_COUNT = 0;

                users::init_user_system();
                LOGGED_IN = false;
                USER_LOGIN_ATTEMPTS = 0;
                USER_LOGIN_LOCKED = false;
                LOGIN_USERNAME_LEN = 0;
                LOGIN_PASSWORD_LEN = 0;
                LOGIN_IS_PASSWORD = false;
                LOGIN_ERROR_LEN = 0;

                TERM_INPUT_GLOBAL = [0; 256];
                TERM_LEN_GLOBAL = 0;
            }

            vga.write_string("Loading ATA driver...\n");
            mount_ata_drive(&mut vga);
            syscfg::init(info_addr);
            unsafe { FM_BOOTED_FROM_DISK = syscfg::booted_from_disk(info_addr); }
            users::load_installed(info_addr);

            vga.write_string("Initializing Window Manager...\n");

            let mut wm = WindowManager::new();
            wm.set_screen_size(gfx.width, gfx.height);
            unsafe { wm::ON_WINDOW_CLOSED = Some(on_window_closed); }

            let screen_w = gfx.width;
            let _screen_h = gfx.height;

            gfx.clear(Color::rgb(0, 0, 0).to_u32());

            let logo_y = 60;

            gfx.draw_text((screen_w - 100) / 2 + 15, logo_y + 90, "MiUX", Color::rgb(252, 255, 252).to_u32(), Color::BLACK.to_u32());

            let bar_width = 400;
            let bar_height = 20;
            let bar_x = (screen_w - bar_width) / 2;
            let bar_y = logo_y + 130;

            gfx.draw_rect_border(bar_x - 1, bar_y - 1, bar_width + 2, bar_height + 2, Color::rgb(100, 100, 100).to_u32());
            gfx.flush();

            for tick in 0..60 {
                let progress = (tick * bar_width) / 60;
                gfx.fill_rect(bar_x, bar_y, progress, bar_height, Color::rgb(255, 255, 255).to_u32());
                gfx.flush();
                for _ in 0..20000 {
                    unsafe { core::arch::asm!("nop") }
                }
            }

            gfx.fill_rect(bar_x, bar_y, bar_width, bar_height, Color::rgb(255, 255, 255).to_u32());
            gfx.flush();

            for _ in 0..30000 {
                unsafe { core::arch::asm!("nop") }
            }

            let mut term_input: [u8; 256] = [0; 256];
            let mut term_len = 0;

            fn push_history(line: &str) {
                unsafe {
                    TERM_SCROLL = 0;
                    if TERM_HIST_LEN >= 1000 {
                        TERM_HISTORY.copy_within(1..1000, 0);
                        TERM_HIST_LEN = 999;
                    }
                    if TERM_HIST_LEN < 1000 {
                        TERM_HISTORY[TERM_HIST_LEN] = [0; 80];
                        let bytes = clip_str(line, 80).as_bytes();
                        let len = bytes.len();
                        TERM_HISTORY[TERM_HIST_LEN][..len].copy_from_slice(bytes);
                        TERM_HIST_LEN += 1;
                    }
                }
            }

            fn clear_history() {
                unsafe {
                    TERM_SCROLL = 0;
                    TERM_HIST_LEN = 0;
                    for i in 0..1000 {
                        for j in 0..80 {
                            TERM_HISTORY[i][j] = b' ';
                        }
                    }
                }
            }

            fn exec_command(input: &str) -> &'static str {
                let trimmed = input.trim();

                let mut parts = [""; 8];
                let mut part_count = 0;
                for word in trimmed.split_whitespace() {
                    if part_count < 8 {
                        parts[part_count] = word;
                        part_count += 1;
                    }
                }

                if part_count >= 1 {
                    if parts[0] == "gui" {
                        unsafe {
                            if part_count >= 3 && parts[1] == "/start" && parts[2] == "--all" {
                                if GUI_MODE {
                                    return "GUI is already running.";
                                }
                                GUI_START_REQUESTED = true;
                                return "Starting GUI...";
                            }
                            if part_count >= 2 && parts[1] == "/stop" {
                                if !GUI_MODE {
                                    return "GUI is not running.";
                                }
                                GUI_STOP_REQUESTED = true;
                                return "Stopping GUI...";
                            }
                        }
                        return "Use: gui /start --all  or  gui /stop";
                    }

                    if parts[0] == "notepad" {
                        unsafe {
                            if part_count >= 2 && parts[1] == "/start" {
                                if !GUI_MODE {
                                    return "Notepad needs the GUI. Run: gui /start --all";
                                }
                                NOTEPAD_WINDOW_REQUESTED = true;
                                return "Notepad started!";
                            }
                        }
                        return "Use: notepad /start  (works only inside GUI: gui /start --all)";
                    }

                    if parts[0] == "shutdown" || parts[0] == "poweroff" {
                        unsafe { POWER_REQUEST = true; }
                        return "System is going down...";
                    }

                    if unsafe { !GUI_MODE } {
                        let needs_gui = match parts[0] {
                            "gl" | "calendar" | "task" | "taskmgr" | "html" => part_count >= 2 && parts[1] == "/start",
                            "pc" => part_count >= 2 && parts[1] == "--info",
                            "call" => part_count >= 2 && parts[1] == "gl",
                            "new" => part_count >= 2 && parts[1] == "folder",
                            "support" | "help-center" | "support-center" | "media" | "cre-tool"
                            | "tutorial" | "browser" | "web" | "net" | "network" | "wifi"
                            | "miux/grap/wm2.rs" => true,
                            _ => false,
                        };
                        if needs_gui {
                            return "This command needs the GUI. First run: gui /start --all";
                        }
                    }

                    match parts[0] {
                        "ls" | "dir" => return termfs::cmd_ls(),
                        "cd" => return termfs::cmd_cd(trimmed[2..].trim()),
                        "cat" => return termfs::cmd_cat(trimmed[3..].trim()),
                        "disks" => return termfs::cmd_disks(),
                        "mount" if part_count == 1 => return termfs::cmd_disks(),
                        _ => {}
                    }
                }

                if part_count >= 2 && parts[0] == "call" {
                    let app_name = parts[1];
                    static mut ARGS_BUF: [u8; 256] = [0; 256];
                    if part_count > 2 {
                        unsafe {
                            let mut pos = 0;
                            for i in 2..part_count {
                                let word = parts[i].as_bytes();
                                if pos > 0 {
                                    ARGS_BUF[pos] = b' ';
                                    pos += 1;
                                }
                                for &b in word {
                                    if pos < 255 {
                                        ARGS_BUF[pos] = b;
                                        pos += 1;
                                    }
                                }
                            }
                            ARGS_BUF[pos] = 0;
                            let args_str = core::str::from_utf8(&ARGS_BUF[..pos]).unwrap_or("");
                            return call_app(app_name, args_str);
                        }
                    } else {
                        return call_app(app_name, "");
                    }
                }

                if trimmed.starts_with("call") && part_count == 1 {
                    return "Usage: call <app_name> [args...]";
                }

                if part_count >= 2 && parts[0] == "display" && parts[1] == "/scr" {
                    if part_count >= 3 {
                        let res = parts[2];
                        if res == "default" {
                            return "Resolution reset to default!";
                        } else if let Some(pos) = res.find('x') {
                            let w_str = &res[..pos];
                            let h_str = &res[pos+1..];
                            if let (Ok(w), Ok(h)) = (w_str.parse::<usize>(), h_str.parse::<usize>()) {
                                if w >= 640 && w <= 1920 && h >= 480 && h <= 1080 {
                                    let mut msg = [0u8; 64];
                                    let mut p = 0;
                                    let prefix = b"Resolution changed to ";
                                    for &b in prefix {
                                        if p < 63 { msg[p] = b; p += 1; }
                                    }
                                    let w_str2 = num_to_str(w);
                                    for b in w_str2.bytes() {
                                        if p < 63 { msg[p] = b; p += 1; }
                                    }
                                    if p < 63 { msg[p] = b'x'; p += 1; }
                                    let h_str2 = num_to_str(h);
                                    for b in h_str2.bytes() {
                                        if p < 63 { msg[p] = b; p += 1; }
                                    }
                                    if p < 63 { msg[p] = b'!'; p += 1; }
                                    msg[p] = 0;
                                    let msg_str = core::str::from_utf8(&msg[..p]).unwrap_or("Resolution changed!");

                                    unsafe {
                                        if TERM_HIST_LEN < 1000 {
                                            let bytes = msg_str.as_bytes();
                                            let len = if bytes.len() > 80 { 80 } else { bytes.len() };
                                            TERM_HISTORY[TERM_HIST_LEN][..len].copy_from_slice(&bytes[..len]);
                                            TERM_HIST_LEN += 1;
                                        }
                                    }
                                    return "Resolution changed!";
                                } else {
                                    return "Invalid resolution! Use: 640x480 to 1920x1080";
                                }
                            }
                        }
                    }
                    return "Usage: display /scr <WxH> (e.g. display /scr 1920x1080) or display /scr default";
                }

                if trimmed == "display" {
                    return "Usage: display /scr <WxH>  or  display /scr default";
                }

                if part_count >= 2 && parts[0] == "gl" {
                    match parts[1] {
                        "/start" => {
                            unsafe {
                                GL_WINDOW_REQUESTED = true;
                                if GL_APP_INSTANCE.is_none() {
                                    GL_APP_INSTANCE = Some(gl::GlApp::new());
                                }
                            }
                            return "GL started! Opening 3D gear visualization...";
                        }
                        "/stop" => {
                            unsafe {
                                if let Some(_) = GL_APP_INSTANCE {
                                    GL_WINDOW_CLOSE_REQUESTED = true;
                                    gl::gl_stop();
                                }
                            }
                            return "GL stopped!";
                        }
                        _ => {
                            return "Use: gl /start  or  gl /stop";
                        }
                    }
                }

                if part_count >= 2 && parts[0] == "calendar" {
                    match parts[1] {
                        "/start" => {
                            unsafe { calend::CALENDAR_WINDOW_REQUESTED = true; }
                            return "Calendar started!";
                        }
                        "/stop" => {
                            unsafe { calend::CALENDAR_WINDOW_CLOSE_REQUESTED = true; }
                            return "Calendar stopped!";
                        }
                        _ => {
                            return "Use: calendar /start  or  calendar /stop";
                        }
                    }
                }

                if trimmed == "calendar" {
                    return "Use: calendar /start  or  calendar /stop";
                }

                if part_count >= 2 && parts[0] == "/start" && parts[1] == "--inf" {
                    return "Available apps for /start:\n  gl /start        - 3D gear visualization\n  task /start      - Task Manager\n  calendar /start  - Calendar with BIOS date/time\n  html /start      - HTML4 Viewer\n  support          - MiUX Support Center\n  media            - BMP image viewer\n  cre-tool         - Graphical app builder\n  notepad /start   - Notepad (only inside GUI: gui /start --all)\n  call <app>       - Run any registered app (list-apps)";
                }

                if part_count >= 2 && (parts[0] == "task" || parts[0] == "taskmgr") {
                    match parts[1] {
                        "/start" => {
                            unsafe {
                                TASK_WINDOW_REQUESTED = true;
                                task_manager::task_start();
                            }
                            return "Task Manager started!";
                        }
                        "/stop" => {
                            unsafe {
                                TASK_WINDOW_CLOSE_REQUESTED = true;
                                task_manager::task_stop();
                            }
                            return "Task Manager stopped!";
                        }
                        _ => {
                            return "Use: task /start  or  task /stop";
                        }
                    }
                }

                if trimmed == "task" || trimmed == "taskmgr" {
                    return "Use: task /start  or  task /stop";
                }

                if trimmed == "gl" {
                    return "Use: gl /start  or  gl /stop";
                }

                if part_count >= 2 && parts[0] == "html" {
                    match parts[1] {
                        "/start" => {
                            unsafe { HTML_WINDOW_REQUESTED = true; }
                            return "Opening HTML4 Viewer...";
                        }
                        "/stop" => {
                            unsafe { HTML_WINDOW_CLOSE_REQUESTED = true; }
                            return "HTML Viewer closed!";
                        }
                        _ => {
                            return "Use: html /start  or  html /stop";
                        }
                    }
                }

                if trimmed == "html" {
                    return "Use: html /start  or  html /stop";
                }

                if trimmed == "support" || trimmed == "help-center" || trimmed == "support-center" {
                    unsafe { SUPPORT_WINDOW_REQUESTED = true; }
                    return "Opening MiUX Support Center...";
                }

                if part_count >= 2 && parts[0] == "pc" && parts[1] == "--info" {
                    unsafe { PCINF_WINDOW_REQUESTED = true; }
                    return "Opening PC Information...";
                }
                
                if trimmed == "pc" {
                    return "Use: pc --info";
                }

                if parts[0] == "format" {
                    unsafe {
                        if part_count >= 2 {
                            if let Ok(disk_num) = parts[1].parse::<usize>() {
                                if disk_num == 0 || disk_num > ata::disk_count() {
                                    return "Invalid disk number! Use 'disks' to see available disks.";
                                }
                                if part_count < 3 || (parts[2] != "/y" && parts[2] != "/Y") {
                                    return "WARNING: this will ERASE ALL DATA on the disk!\nTo confirm type: format <disk_number> /y";
                                }
                                let idx = disk_num - 1;
                                return match fs::fat32::format(idx, "MIUX") {
                                    Ok(()) => {
                                        FAT32_MOUNTED[idx] = false;
                                        FAT32_FILESYSTEMS[idx] = None;
                                        if let Some(newfs) = fs::fat32::FAT32FS::new(idx) {
                                            FAT32_FILESYSTEMS[idx] = Some(newfs);
                                            FAT32_MOUNTED[idx] = true;
                                        }
                                        "Disk formatted to FAT32 successfully!\nNotepad: Save / Load now work with this disk."
                                    }
                                    Err(e) => e,
                                };
                            }
                        }
                        return "Usage: format <disk_number> /y   (formats the disk to FAT32, ERASES ALL DATA)";
                    }
                }

                if part_count >= 2 && parts[0] == "mount" {
                    unsafe {
                        if part_count >= 2 {
                            if let Ok(disk_num) = parts[1].parse::<usize>() {
                                if disk_num > 0 && disk_num <= ata::disk_count() {
                                    let idx = disk_num - 1;
                                    if fs::fat32::mount(idx) {
                                        return "FAT32 mounted successfully!";
                                    } else {
                                        return "Failed to mount FAT32 on disk! Not a FAT32 filesystem.";
                                    }
                                } else {
                                    return "Invalid disk number! Use 'disks' to see available disks.";
                                }
                            }
                        }
                        return "Usage: mount <disk_number>"
                    }
                }

                if part_count >= 1 && parts[0] == "pkg" {
                    if part_count >= 3 && parts[1] == "install" {
                        let u = parts[2].as_bytes();
                        if u.len() > 255 { return "URL too long"; }
                        unsafe {
                            PKG_URL[..u.len()].copy_from_slice(u);
                            PKG_URL_LEN = u.len();
                            PKG_PENDING = true;
                        }
                        return "";
                    }
                    return "Usage: pkg install <url>";
                }

                if let Some(r) = netcmd::exec(trimmed) {
                    return r;
                }

                match trimmed {
                    "help" => "  System:        ver, uptime, mem, about, tech, kernel\n  Users:         login, logout, whoami, users, useradd, userdel, passwd\n  Files:         dir, ls, cd, cat <file>, <filename>, disks, mount <N>, fs, new folder, ata, tals\n  EXT4:          ext4, ext4ls, ext4cat <file>, ext4info\n  Terminal:      cls, clear \n  Dev:           devs, developers, mide\n  Fun:           dice\n  GUI:           gui /start --all (graphical mode), gui /stop\n  Notepad:       notepad /start (only inside GUI)\n  Power:         shutdown\n  Advanced:      stop gui, del ram\n  Other:         tutorial\n  Media:         media - BMP image viewer\n  HTML:          html /start - HTML4 viewer (test.html)\n  Support:       support - MiUX Support Center\n  MiDE Settings: miux/grap/wm2.rs\n  Apps:          call <app_name> [args], list-apps\n  Cre Tool:      cre-tool - graphical app builder\n  GL:            gl /start, gl /stop - 3D gears\n  Display:       display /scr 1920x1080, display /scr default\n start modules:   /start --inf\n PC INFO:        pc --info\n  Network:       ifconfig, nslookup <host>, wget [-s] <url>, net (окно Network)\n  Browser:       browser [адрес] - веб-браузер (HTML, CSS, HTTPS)",
                    "ver" => "MiUX 7.0.0",
                    "mem" => "Memory: 4096 MB",
                    "uptime" => "Uptime: 0 ticks",
                    "dice" => "Dice: 4",
                    "fs" => "FAT32",
                    "tutorial" => open_tutorial(),
                    "miux/grap/wm2.rs" => switch_to_wm2_theme(),
                    "ata" => {
                        if unsafe { ATA_MOUNTED } {
                            "ATA drive is mounted!"
                        } else {
                            "No ATA drive detected!"
                        }
                    },
                    "atals" => list_ata_files(),
                    "devs" => "MiUX Developers: MDEVS",
                    "developers" => "MiUX Developers: MDEVS",
                    "kernel" => "MiUX KERNEL:\nFILES KERNEL: lib.rs , wm.rs , mide.rs\nKERNEL BILDER: Mikhail\nKERNEL VER: MiUX 7.0.0\nMade in 2026",
                    "mide" => "\nmmmmmmmmmmmmmmmmmmmmmmm          MiDE 5.4\nm MiDE         _ O X  m          MiDE FULL version: 5.3 \nmmmmmmmmmmmmmmmmmmmmmmm          MiUX Version: 7.0.0\nm                     m          Kernel MiDE: ver1.0\nm                     m          made in 2026\nm                     m          MiDE by MDEVS (C)\nmmmmmmmmmmmmmmmmmmmmmmm",
                    "cls" => "\x04",
                    "clear" => "\x04",
                    "about" => "MiUX 7.0.0 Made in 2026 :)",
                    "tech" => "\x0E",
                    "dir" => "\nC:/MiUX/\nkernel.bin - lib.rs\nmouse.rs - mouse USB\nvm.rs - MiDE 5.4\nvga.rs - vga mode\nbmp.rs - bmp img system\nfs>\n   ext4.rs\n   mod.rs\n   tar.rs",
                    "stop gui" => "Access error",
                    "del ram" => "Access error",
                    "new folder" => "\x07",
                    "media" => {
                        unsafe {
                            MEDIA_WINDOW_REQUESTED = true;
                        }
                        "Opening Media (BMP Viewer)..."
                    },
                    "list-apps" => call_app("list-apps", ""),
                    "cre-tool" => {
                        unsafe { CRE_TOOL_WINDOW_REQUESTED = true; }
                        cre_tool::cre_tool_start()
                    },
                    "login" => {
                        unsafe {
                            if USER_LOGIN_ATTEMPTS >= 3 {
                                USER_LOGIN_LOCKED = true;
                                return "System locked! Too many attempts.";
                            }
                            if USER_LOGIN_LOCKED {
                                return "System locked! Contact admin.";
                            }
                            "Use: login <username> <password>"
                        }
                    },
                    "logout" => {
                        unsafe {
                            users::logout();
                            LOGGED_IN = false;
                            "Logged out successfully!"
                        }
                    },
                    "whoami" => user_whoami(),
                    "users" => users::list_users(),
                    "useradd" => {
                        unsafe {
                            if !users::is_admin() {
                                return "Error: Admin rights required!";
                            }
                            "Use: useradd <username> <password> [admin]"
                        }
                    },
                    "userdel" => {
                        unsafe {
                            if !users::is_admin() {
                                return "Error: Admin rights required!";
                            }
                            "Use: userdel <username>"
                        }
                    },
                    "passwd" => "Use: passwd <username> <new_password>",
                    "ext4" => {
                        unsafe {
                            if fs::ext4::mount(0) {
                                EXT4_OK = true;
                                EXT4_TRIED = true;
                                "EXT4 filesystem detected on disk 0! Use 'ext4ls' or open File Manager."
                            } else {
                                EXT4_TRIED = true;
                                EXT4_OK = false;
                                "No EXT4 filesystem found at start of disk 0 (maybe GPT/partitioned)."
                            }
                        }
                    },
                    "ext4ls" => {
                        unsafe {
                            if fs::ext4::is_mounted() {
                                fs::ext4::list_files()
                            } else {
                                "EXT4 not mounted. Run 'ext4' first."
                            }
                        }
                    },
                    "ext4info" => {
                        unsafe {
                            if fs::ext4::is_mounted() {
                                fs::ext4::info()
                            } else {
                                "EXT4 not mounted. Run 'ext4' first."
                            }
                        }
                    },
                    "" => "",
                    _ => {
                        if part_count >= 2 {
                            match parts[0] {
                                "login" => {
                                    unsafe {
                                        if USER_LOGIN_LOCKED {
                                            return "System locked!";
                                        }
                                        if part_count >= 3 {
                                            if users::login(parts[1], parts[2]) {
                                                LOGGED_IN = true;
                                                USER_LOGIN_ATTEMPTS = 0;
                                                return "Welcome!";
                                            } else {
                                                USER_LOGIN_ATTEMPTS += 1;
                                                if USER_LOGIN_ATTEMPTS >= 3 {
                                                    USER_LOGIN_LOCKED = true;
                                                    return "System locked! Too many attempts.";
                                                }
                                                return "Error: Invalid username or password!";
                                            }
                                        } else {
                                            return "Use: login <username> <password>";
                                        }
                                    }
                                }
                                "useradd" => {
                                    unsafe {
                                        if !users::is_admin() {
                                            return "Error: Admin rights required!";
                                        }
                                        if part_count >= 3 {
                                            let is_admin = part_count >= 4 && parts[3] == "admin";
                                            return user_add(parts[1], parts[2], is_admin);
                                        } else {
                                            return "Use: useradd <username> <password> [admin]";
                                        }
                                    }
                                }
                                "userdel" => {
                                    unsafe {
                                        if !users::is_admin() {
                                            return "Error: Admin rights required!";
                                        }
                                        if part_count >= 2 {
                                            return user_del(parts[1]);
                                        } else {
                                            return "Use: userdel <username>";
                                        }
                                    }
                                }
                                "passwd" => {
                                    if part_count >= 3 {
                                        return user_pass(parts[1], parts[2]);
                                    } else {
                                        return "Use: passwd <username> <new_password>";
                                    }
                                }
                                "ext4cat" => {
                                    unsafe {
                                        if !fs::ext4::is_mounted() {
                                            return "EXT4 not mounted. Run 'ext4' first.";
                                        }
                                        if part_count < 2 {
                                            return "Usage: ext4cat <filename>";
                                        }
                                        return fs::ext4::read_file(parts[1]);
                                    }
                                }
                                _ => {}
                            }
                        }
                        if let Some(r) = termfs::open_by_name(trimmed) {
                            return r;
                        }
                        "Unknown command, error: 00000x5"
                    }
                }
            }

            fn draw_window_content(gfx: &mut Graphics, win: &wm::Window, idx: usize) {
                if win.state == WindowState::Minimized {
                    return;
                }

                let is_preview = unsafe {
                    PREVIEW_RUNNING && idx == PREVIEW_WINDOW_INDEX
                };

                if is_preview {
                    let t = wm::get_theme();
                    let cx = win.x + 2;
                    let cy = win.y + 26;

                    gfx.fill_rect(cx, cy, win.width - 4, win.height - 28, Color::WHITE.to_u32());

                    unsafe {
                        for i in 0..PREVIEW_ELEMENTS_COUNT {
                            let el = &PREVIEW_ELEMENTS[i];
                            if !el.visible { continue; }

                            let el_x = cx + el.x;
                            let el_y = cy + el.y;

                            if el.kind == 0 {
                            } else if el.kind == 1 {
                                wm::draw_raised_rect(gfx, el_x, el_y, el.w, el.h);
                                let text = core::str::from_utf8(&el.text[..el.text_len]).unwrap_or("Button");
                                let text_x = el_x + (el.w - text.len() * 8) / 2;
                                gfx.draw_text(text_x, el_y + 6, text, Color::BLACK.to_u32(), Color::rgb(212, 208, 200).to_u32());
                            } else if el.kind == 2 {
                                let text = core::str::from_utf8(&el.text[..el.text_len]).unwrap_or("Label");
                                gfx.draw_text(el_x, el_y, text, Color::BLACK.to_u32(), Color::WHITE.to_u32());
                            }
                        }
                    }
                    return;
                }

                let t = wm::get_theme();
                let use_wm2 = unsafe { USE_WM2_THEME };
                if use_wm2 {
                }

                let cx = win.x + 2;
                let cy = win.y + 26;

                let mut is_folder_window = false;
                let mut current_folder_index = 0;
                unsafe {
                    for i in 0..DESKTOP_FOLDERS_COUNT {
                        let folder_name = core::str::from_utf8(&DESKTOP_FOLDERS[i]).unwrap_or("");
                        if folder_name.trim_end_matches('\0') == win.title {
                            is_folder_window = true;
                            current_folder_index = i;
                            break;
                        }
                    }
                }

                if is_folder_window {
                    gfx.fill_rect(cx, cy, win.width - 4, win.height - 28, Color::rgb(235, 235, 235).to_u32());
                    gfx.draw_text(cx + 10, cy + 10, "Folder: ", Color::BLACK.to_u32(), Color::rgb(235, 235, 235).to_u32());
                    gfx.draw_text(cx + 10 + 8*8, cy + 10, win.title, Color::BLACK.to_u32(), Color::rgb(235, 235, 235).to_u32());
                    gfx.draw_text(cx + 10, cy + 30, "", Color::BLACK.to_u32(), Color::rgb(235, 235, 235).to_u32());

                    wm::draw_raised_rect(gfx, cx + 10, cy + 40, 150, 24);
                    gfx.draw_text(cx + 18, cy + 45, "Save Current File", Color::BLACK.to_u32(), Color::rgb(212, 208, 200).to_u32());

                    unsafe {
                        let file_count = FOLDER_FILES_COUNT[current_folder_index];
                        if file_count == 0 {
                            gfx.draw_text(cx + 10, cy + 80, "No files", Color::rgb(128, 128, 128).to_u32(), Color::rgb(235, 235, 235).to_u32());
                        } else {
                            let icon_size = 32;
                            let start_y = cy + 80;
                            let mut x_pos = cx + 10;
                            let mut y_pos = start_y;
                            let items_per_row = 8;

                            for i in 0..file_count {
                                let filename = core::str::from_utf8(&FOLDER_FILES[current_folder_index][i]).unwrap_or("");
                                let filename_trimmed = filename.trim_end_matches('\0');

                                draw_file_icon(gfx, x_pos, y_pos, filename_trimmed);

                                x_pos += icon_size + 10;
                                if (i + 1) % items_per_row == 0 {
                                    x_pos = cx + 10;
                                    y_pos += icon_size + 20;
                                }
                            }
                        }
                    }
                } else {
                    match win.title {
                        "Media" => {
                            draw_media_content(gfx, cx, cy, win.width - 4, win.height - 28);
                        }
                        "Browser" => {
                            web::app::draw(gfx, win.x + 2, win.y + 30, win.width - 4, win.height - 32);
                        }
                        "HTML App" => {
                            web::app::app_draw(gfx, win.x + 2, win.y + 30, win.width - 4, win.height - 32);
                        }
                        "Network" => {
                            netwin::draw(gfx, win.x + 2, win.y + 30, win.width - 4, win.height - 32);
                        }
                        "HTML Viewer" => {
                            unsafe {
                                if HTML_APP_INSTANCE.is_none() {
                                    let mut app = html::HtmlApp::new();
                                    app.load(html::TEST_HTML);
                                    HTML_APP_INSTANCE = Some(app);
                                }
                                if let Some(ref mut app) = HTML_APP_INSTANCE {
                                    html::draw_html_content(gfx, cx, cy, win.width - 4, win.height - 28, app);
                                }
                            }
                        }
                        "Support Center" => {
                            unsafe {
                                if SUPPORT_APP_INSTANCE.is_none() {
                                    let mut app = support::SupportApp::new();
                                    app.load(support::SUPPORT_HTML);
                                    SUPPORT_APP_INSTANCE = Some(app);
                                }
                                if let Some(ref mut app) = SUPPORT_APP_INSTANCE {
                                    support::draw_support_content(gfx, cx, cy, win.width - 4, win.height - 28, app);
                                }
                            }
                        }
                        "PC Info" => {
                            unsafe {
                                if PCINF_APP_INSTANCE.is_none() {
                                    let mut app = pcinf::PcInfoApp::new();
                                    app.load(640, 4 * 1024 * 1024);
                                    PCINF_APP_INSTANCE = Some(app);
                                }
                                if let Some(ref mut app) = PCINF_APP_INSTANCE {
                                    pcinf::draw_pcinf_content(gfx, cx, cy, win.width - 4, win.height - 28, app);
                                }
                            }
                        }
                        "Terminal" => {
                            let history = unsafe { &TERM_HISTORY };
                            let hist_len = unsafe { TERM_HIST_LEN };

                            let ubuntu_purple = Color::rgb(0, 0, 0).to_u32();

                            gfx.fill_rect(cx, cy, win.width - 4, win.height - 28, ubuntu_purple);

                            let max_lines = (win.height - 28 - 30) / 16;
                            let base_start = if hist_len > max_lines { hist_len - max_lines } else { 0 };
                            let scroll = unsafe {
                                if TERM_SCROLL > base_start { TERM_SCROLL = base_start; }
                                TERM_SCROLL
                            };
                            let start_row = base_start - scroll;
                            let end_row = if scroll > 0 { (start_row + max_lines).min(hist_len) } else { hist_len };

                            for row in start_row..end_row {
                                if history[row][0] == 0x0E {
                                    let k = history[row][1].wrapping_sub(b'0') as usize;
                                    let ty = cy + 10 + (row - start_row) * 16;
                                    tech_draw_slice(gfx, cx + 10, ty, k);
                                    let txt = core::str::from_utf8(&history[row][2..]).unwrap_or("").trim_end_matches('\0');
                                    let tmax = win.width.saturating_sub(24 + TECH_SZ + 16) / 8;
                                    let tshow = match txt.char_indices().nth(tmax) {
                                        Some((b, _)) => &txt[..b],
                                        None => txt,
                                    };
                                    gfx.draw_text(cx + 10 + TECH_SZ + 16, ty, tshow, Color::rgb(255, 255, 255).to_u32(), ubuntu_purple);
                                    continue;
                                }
                                let line = core::str::from_utf8(&history[row]).unwrap_or("");
                                let max_chars = win.width.saturating_sub(24) / 8;
                                let display_line = match line.char_indices().nth(max_chars) {
                                    Some((b, _)) => &line[..b],
                                    None => line,
                                };
                                let y_pos = cy + 10 + (row - start_row) * 16;
                                gfx.draw_text(cx + 10, y_pos, display_line, Color::rgb(255, 255, 255).to_u32(), ubuntu_purple);
                            }

                            let prompt_y = cy + 10 + (hist_len - start_row) * 16;
                            let terminal_bottom = cy + win.height - 28;
                            let final_prompt_y = if prompt_y + 20 > terminal_bottom {
                                cy + 10 + max_lines * 16
                            } else {
                                prompt_y
                            };

                            if scroll == 0 {
                            gfx.draw_text(cx + 10, final_prompt_y, termfs::prompt(), Color::rgb(250, 250, 250).to_u32(), ubuntu_purple);
                            }

                            unsafe {
                                let global_len = TERM_LEN_GLOBAL;
                                if global_len > 0 && scroll == 0 {
                                    let text = core::str::from_utf8(&TERM_INPUT_GLOBAL[..global_len]).unwrap_or("");
                                    gfx.draw_text(cx + 10 + termfs::prompt().chars().count() * 8, final_prompt_y, text, Color::rgb(0, 255, 0).to_u32(), ubuntu_purple);
                                }
                            }
                        }
                        "AboutOS" => {
                            let about_img = unsafe { &ABOUT_IMAGE };
                            if let Some(ref bmp) = about_img {
                                let img_x = cx + (win.width - 4 - bmp.width) / 2;
                                let img_y = cy + 10;
                                let row_size = ((bmp.width * 3 + 3) / 4) * 4;
                                for dy in 0..bmp.height {
                                    let bmp_y = (bmp.height - 1).saturating_sub(dy);
                                    let bmp_row_start = bmp_y * row_size;
                                    for dx in 0..bmp.width {
                                        let pixel_offset = bmp_row_start + dx * 3;
                                        if pixel_offset + 2 < bmp.data.len() {
                                            let b = bmp.data[pixel_offset] as u32;
                                            let g = bmp.data[pixel_offset + 1] as u32;
                                            let r = bmp.data[pixel_offset + 2] as u32;
                                            let color = (r << 16) | (g << 8) | b;
                                            gfx.put_pixel(img_x + dx, img_y + dy, color);
                                        }
                                    }
                                }
                                let info_y = img_y + bmp.height + 20;
                                gfx.draw_text(cx + 10, info_y, "", Color::BLACK.to_u32(), t.window_bg.to_u32());
                                gfx.draw_text(cx + 10, info_y + 20, "MiUX 7.0.0 system", Color::BLACK.to_u32(), t.window_bg.to_u32());
                                gfx.draw_text(cx + 10, info_y + 40, "Created in 2026", Color::BLACK.to_u32(), t.window_bg.to_u32());
                                gfx.draw_text(cx + 10, info_y + 60, "Site: https://p.miux.phoneget.top", Color::BLACK.to_u32(), t.window_bg.to_u32());
                                gfx.draw_text(cx + 10, info_y + 80, "Creators: MDEVS (c)", Color::BLACK.to_u32(), t.window_bg.to_u32());
                                gfx.draw_text(cx + 10, info_y + 100, "This version is copyrighted", Color::BLACK.to_u32(), t.window_bg.to_u32());
                                gfx.draw_text(cx + 10, info_y + 140, "", Color::BLACK.to_u32(), t.window_bg.to_u32());
                            } else {
                                gfx.draw_text(cx + 10, cy + 10, "About MiUX system", Color::BLACK.to_u32(), t.window_bg.to_u32());
                                gfx.draw_text(cx + 10, cy + 30, "", Color::BLACK.to_u32(), t.window_bg.to_u32());
                                gfx.draw_text(cx + 10, cy + 50, "MiUX 7.0.0 system", Color::BLACK.to_u32(), t.window_bg.to_u32());
                                gfx.draw_text(cx + 10, cy + 70, "Created in 2026", Color::BLACK.to_u32(), t.window_bg.to_u32());
                                gfx.draw_text(cx + 10, cy + 90, "Site: https://p.miux.phoneget.top", Color::BLACK.to_u32(), t.window_bg.to_u32());
                                gfx.draw_text(cx + 10, cy + 110, "Creators: MDEVS (c)", Color::BLACK.to_u32(), t.window_bg.to_u32());
                                gfx.draw_text(cx + 10, cy + 130, "This version is copyrighted", Color::BLACK.to_u32(), t.window_bg.to_u32());
                                gfx.draw_text(cx + 10, cy + 170, "", Color::BLACK.to_u32(), t.window_bg.to_u32());
                            }
                        }
                        "Settings" => {
                            let settings_bg = Color::rgb(245, 245, 245).to_u32();
                            gfx.fill_rect(cx, cy, win.width - 4, win.height - 28, settings_bg);

                            let header_h = 50;
                            gfx.fill_rect(cx, cy, win.width - 4, header_h, Color::rgb(150, 150, 150).to_u32());
                            gfx.draw_text(cx + 16, cy + 14, "Settings", Color::WHITE.to_u32(), Color::rgb(32, 32, 32).to_u32());

                            gfx.fill_rect(cx, cy + header_h, win.width - 4, 1, Color::rgb(200, 200, 200).to_u32());

                            let content_y = cy + header_h + 16;
                            let mut current_y = content_y;

                            gfx.fill_rect(cx + 16, current_y, win.width - 4 - 32, 48, Color::WHITE.to_u32());
                            gfx.draw_rect_border(cx + 16, current_y, win.width - 4 - 32, 48, Color::rgb(220, 220, 220).to_u32());
                            gfx.fill_rect(cx + 18, current_y + 4, 4, 40, Color::rgb(5, 5, 5).to_u32());
                            gfx.draw_text(cx + 32, current_y + 8, "Change Wallpaper", Color::rgb(40, 40, 40).to_u32(), Color::WHITE.to_u32());
                            gfx.draw_text(cx + 32, current_y + 28, "Select a color for your desktop background", Color::rgb(130, 130, 130).to_u32(), Color::WHITE.to_u32());
                            current_y += 52;

                            gfx.fill_rect(cx + 16, current_y, win.width - 4 - 32, 48, Color::WHITE.to_u32());
                            gfx.draw_rect_border(cx + 16, current_y, win.width - 4 - 32, 48, Color::rgb(220, 220, 220).to_u32());
                            gfx.fill_rect(cx + 18, current_y + 4, 4, 40, Color::rgb(5, 5, 5).to_u32());
                            gfx.draw_text(cx + 32, current_y + 8, "Change Theme", Color::rgb(40, 40, 40).to_u32(), Color::WHITE.to_u32());
                            gfx.draw_text(cx + 32, current_y + 28, "Switch between Aero, Dark and Light themes", Color::rgb(130, 130, 130).to_u32(), Color::WHITE.to_u32());
                            current_y += 52;

                            let bmp_status = unsafe { USE_BMP_WALLPAPER };
                            let status_text = if bmp_status { "ON" } else { "OFF" };
                            let status_color = if bmp_status { Color::rgb(0, 72, 85).to_u32() } else { Color::rgb(180, 0, 0).to_u32() };

                            gfx.fill_rect(cx + 16, current_y, win.width - 4 - 32, 48, Color::WHITE.to_u32());
                            gfx.draw_rect_border(cx + 16, current_y, win.width - 4 - 32, 48, Color::rgb(220, 220, 220).to_u32());
                            gfx.fill_rect(cx + 18, current_y + 4, 4, 40, Color::rgb(5, 5, 5).to_u32());
                            gfx.draw_text(cx + 32, current_y + 8, "BMP Wallpapers", Color::rgb(40, 40, 40).to_u32(), Color::WHITE.to_u32());
                            gfx.draw_text(cx + 32, current_y + 28, "Use image files as wallpaper", Color::rgb(130, 130, 130).to_u32(), Color::WHITE.to_u32());
                            gfx.draw_text(cx + win.width - 4 - 80, current_y + 14, status_text, status_color, Color::WHITE.to_u32());

                            let toggle_x = cx + win.width - 4 - 42;
                            let toggle_y = current_y + 12;
                            let toggle_bg = if bmp_status { Color::rgb(0, 120, 215).to_u32() } else { Color::rgb(180, 180, 180).to_u32() };
                            gfx.fill_rect(toggle_x, toggle_y, 36, 22, toggle_bg);
                            gfx.draw_rect_border(toggle_x, toggle_y, 36, 22, Color::rgb(150, 150, 150).to_u32());
                            let circle_x = if bmp_status { toggle_x + 18 } else { toggle_x + 2 };
                            gfx.fill_rect(circle_x, toggle_y + 2, 18, 18, Color::WHITE.to_u32());
                            gfx.draw_rect_border(circle_x, toggle_y + 2, 18, 18, Color::rgb(200, 200, 200).to_u32());
                            current_y += 52;

                            gfx.fill_rect(cx + 16, current_y, win.width - 4 - 32, 48, Color::WHITE.to_u32());
                            gfx.draw_rect_border(cx + 16, current_y, win.width - 4 - 32, 48, Color::rgb(220, 220, 220).to_u32());
                            gfx.fill_rect(cx + 18, current_y + 4, 4, 40, Color::rgb(0, 150, 100).to_u32());
                            gfx.draw_text(cx + 32, current_y + 8, " Select BMP Wallpaper", Color::rgb(40, 40, 40).to_u32(), Color::WHITE.to_u32());
                            gfx.draw_text(cx + 32, current_y + 28, "Choose from available wallpaper images", Color::rgb(130, 130, 130).to_u32(), Color::WHITE.to_u32());
                            current_y += 52;

                            let live_status = unsafe { LIVE_WALLPAPER_ENABLED };
                            let live_status_text = if live_status { "ON" } else { "OFF" };
                            let live_status_color = if live_status { Color::rgb(0, 72, 85).to_u32() } else { Color::rgb(180, 0, 0).to_u32() };

                            gfx.fill_rect(cx + 16, current_y, win.width - 4 - 32, 48, Color::WHITE.to_u32());
                            gfx.draw_rect_border(cx + 16, current_y, win.width - 4 - 32, 48, Color::rgb(220, 220, 220).to_u32());
                            gfx.fill_rect(cx + 18, current_y + 4, 4, 40, Color::rgb(0, 150, 100).to_u32());
                            gfx.draw_text(cx + 32, current_y + 8, "Live Wallpaper", Color::rgb(40, 40, 40).to_u32(), Color::WHITE.to_u32());
                            gfx.draw_text(cx + 32, current_y + 28, "Auto-switch wallpaper at random intervals", Color::rgb(130, 130, 130).to_u32(), Color::WHITE.to_u32());
                            gfx.draw_text(cx + win.width - 4 - 80, current_y + 14, live_status_text, live_status_color, Color::WHITE.to_u32());
                            current_y += 52;

                            gfx.fill_rect(cx + 16, current_y, win.width - 4 - 32, 48, Color::WHITE.to_u32());
                            gfx.draw_rect_border(cx + 16, current_y, win.width - 4 - 32, 48, Color::rgb(220, 220, 220).to_u32());
                            gfx.fill_rect(cx + 18, current_y + 4, 4, 40, Color::rgb(0, 120, 215).to_u32());
                            gfx.draw_text(cx + 32, current_y + 8, "Network", Color::rgb(40, 40, 40).to_u32(), Color::WHITE.to_u32());
                            gfx.draw_text(cx + 32, current_y + 28, "Open network connection settings", Color::rgb(130, 130, 130).to_u32(), Color::WHITE.to_u32());
                            current_y += 52;

                            let info_y = cy + win.height - 28 - 32;
                            gfx.fill_rect(cx, info_y, win.width - 4, 32, Color::rgb(235, 235, 235).to_u32());
                            gfx.fill_rect(cx, info_y, win.width - 4, 1, Color::rgb(200, 200, 200).to_u32());

                            let tidx = wm::theme_index();
                            let theme_name = match tidx { 0 => "Aero", 1 => "Aero Dark", 2 => "Aero Cool", 3 => "Default", 4 => "Classic", _ => "???" };

                            let res_w = num_to_str(gfx.width);
                            let res_h = num_to_str(gfx.height);
                            let mut res_text = [0u8; 32];
                            let mut p = 0;
                            let prefix = b"Resolution: ";
                            for &b in prefix { if p < 31 { res_text[p] = b; p += 1; } }
                            for b in res_w.bytes() { if p < 31 { res_text[p] = b; p += 1; } }
                            if p < 31 { res_text[p] = b'x'; p += 1; }
                            for b in res_h.bytes() { if p < 31 { res_text[p] = b; p += 1; } }
                            res_text[p] = 0;
                            let res_str = core::str::from_utf8(&res_text[..p]).unwrap_or("Resolution: 1920x1080");
                            gfx.draw_text(cx + 16, info_y + 8, res_str, Color::rgb(100, 100, 100).to_u32(), Color::rgb(235, 235, 235).to_u32());
                            gfx.draw_text(cx + 250, info_y + 8, "Theme: ", Color::rgb(100, 100, 100).to_u32(), Color::rgb(235, 235, 235).to_u32());
                            gfx.draw_text(cx + 320, info_y + 8, theme_name, Color::rgb(0, 120, 215).to_u32(), Color::rgb(235, 235, 235).to_u32());
                        }
                        "Live Wallpaper Settings" => {
                            let bg = Color::rgb(245, 245, 245).to_u32();
                            gfx.fill_rect(cx, cy, win.width - 4, win.height - 28, bg);

                            gfx.draw_text(cx + 10, cy + 10, "Live Wallpaper Settings", Color::BLACK.to_u32(), bg);
                            gfx.fill_rect(cx + 10, cy + 26, win.width - 4 - 20, 1, Color::rgb(210, 210, 210).to_u32());

                            let enabled = unsafe { LIVE_WALLPAPER_ENABLED };
                            let interval = unsafe { LIVE_WALLPAPER_INTERVAL_SEC };

                            let row1_y = cy + 40;
                            gfx.draw_text(cx + 10, row1_y + 6, "Enabled:", Color::rgb(40, 40, 40).to_u32(), bg);
                            let toggle_x = cx + win.width - 4 - 58;
                            let toggle_y = row1_y;
                            let toggle_bg = if enabled { Color::rgb(0, 120, 215).to_u32() } else { Color::rgb(180, 180, 180).to_u32() };
                            gfx.fill_rect(toggle_x, toggle_y, 36, 22, toggle_bg);
                            gfx.draw_rect_border(toggle_x, toggle_y, 36, 22, Color::rgb(150, 150, 150).to_u32());
                            let circle_x = if enabled { toggle_x + 18 } else { toggle_x + 2 };
                            gfx.fill_rect(circle_x, toggle_y + 2, 18, 18, Color::WHITE.to_u32());
                            gfx.draw_rect_border(circle_x, toggle_y + 2, 18, 18, Color::rgb(200, 200, 200).to_u32());

                            let row2_y = row1_y + 40;
                            gfx.draw_text(cx + 10, row2_y + 6, "Interval:", Color::rgb(40, 40, 40).to_u32(), bg);

                            let minus_x = cx + 110;
                            wm::draw_raised_rect(gfx, minus_x, row2_y, 26, 26);
                            gfx.draw_text(minus_x + 9, row2_y + 6, "-", Color::BLACK.to_u32(), Color::rgb(200, 200, 200).to_u32());

                            let val_str = num_to_str(interval as usize);
                            let val_x = minus_x + 34;
                            gfx.fill_rect(val_x, row2_y, 50, 26, Color::WHITE.to_u32());
                            gfx.draw_rect_border(val_x, row2_y, 50, 26, Color::rgb(200, 200, 200).to_u32());
                            gfx.draw_text(val_x + 8, row2_y + 6, val_str, Color::BLACK.to_u32(), Color::WHITE.to_u32());
                            gfx.draw_text(val_x + 8 + val_str.len() * 8, row2_y + 6, "s", Color::rgb(130, 130, 130).to_u32(), Color::WHITE.to_u32());

                            let plus_x = val_x + 58;
                            wm::draw_raised_rect(gfx, plus_x, row2_y, 26, 26);
                            gfx.draw_text(plus_x + 8, row2_y + 6, "+", Color::BLACK.to_u32(), Color::rgb(200, 200, 200).to_u32());

                            let hint_y = row2_y + 40;
                            gfx.draw_text(cx + 10, hint_y, "Range: 7s - 60s, always random", Color::rgb(130, 130, 130).to_u32(), bg);
                            gfx.draw_text(cx + 10, hint_y + 20, "Wallpaper changes on its own while", Color::rgb(130, 130, 130).to_u32(), bg);
                            gfx.draw_text(cx + 10, hint_y + 38, "enabled, at a random interval.", Color::rgb(130, 130, 130).to_u32(), bg);
                        }
                        "Paint" => {
                            gfx.fill_rect(cx, cy, win.width - 4, 40, Color::rgb(200, 200, 200).to_u32());
                            gfx.fill_rect(cx, cy + 40, win.width - 4, 1, Color::rgb(128, 128, 128).to_u32());

                            wm::draw_raised_rect(gfx, cx + 5, cy + 5, 30, 30);
                            gfx.fill_rect(cx + 10, cy + 15, 20, 10, unsafe { PAINT_COLOR });

                            wm::draw_raised_rect(gfx, cx + 40, cy + 5, 30, 30);
                            gfx.fill_rect(cx + 45, cy + 10, 20, 20, unsafe { PAINT_COLOR });

                            wm::draw_raised_rect(gfx, cx + 75, cy + 5, 30, 30);
                            draw_line(gfx, cx + 80, cy + 25, cx + 100, cy + 15, Color::BLACK.to_u32());

                            wm::draw_raised_rect(gfx, cx + 110, cy + 5, 30, 30);
                            gfx.draw_rect_border(cx + 117, cy + 10, 16, 16, Color::BLACK.to_u32());

                            let colors = [
                                0x000000, 0xFF0000, 0x00FF00, 0x0000FF,
                                0xFFFF00, 0xFF00FF, 0x00FFFF, 0xFFFFFF,
                            ];
                            for i in 0..8 {
                                let color_x = cx + 150 + i * 25;
                                if color_x + 20 < cx + win.width - 4 - 80 {
                                    wm::draw_raised_rect(gfx, color_x, cy + 5, 20, 30);
                                    gfx.fill_rect(color_x + 3, cy + 8, 14, 24, colors[i]);
                                }
                            }

                            let sizes = ["1", "3", "5"];
                            for i in 0..3 {
                                let size_x = cx + 350 + i * 30;
                                if size_x + 25 < cx + win.width - 4 - 80 {
                                    wm::draw_raised_rect(gfx, size_x, cy + 5, 25, 30);
                                    gfx.draw_text(size_x + 8, cy + 15, sizes[i], Color::BLACK.to_u32(), Color::rgb(200, 200, 200).to_u32());
                                }
                            }

                            let save_btn_x = cx + win.width - 4 - 85;
                            if save_btn_x > cx + 10 {
                                wm::draw_raised_rect(gfx, save_btn_x, cy + 5, 80, 30);
                                gfx.draw_text(save_btn_x + 12, cy + 12, "Save Pic", Color::BLACK.to_u32(), Color::rgb(200, 200, 200).to_u32());
                            }

                            let canvas_x = cx + 5;
                            let canvas_y = cy + 45;
                            let canvas_w = (win.width - 10).min(400);
                            let canvas_h = (win.height - 55).min(300);

                            gfx.fill_rect(canvas_x, canvas_y, canvas_w, canvas_h, 0xFFFFFF);
                            gfx.draw_rect_border(canvas_x, canvas_y, canvas_w, canvas_h, Color::BLACK.to_u32());

                            unsafe {
                                for y in 0..canvas_h.min(300) {
                                    for x in 0..canvas_w.min(400) {
                                        gfx.put_pixel(canvas_x + x, canvas_y + y, PAINT_CANVAS[y][x]);
                                    }
                                }
                            }

                            let tool_name = unsafe {
                                match PAINT_TOOL {
                                    0 => "Brush",
                                    1 => "Line",
                                    2 => "Rect",
                                    _ => "Brush",
                                }
                            };
                            let color_val = unsafe { PAINT_COLOR };
                            let color_r = (color_val >> 16) & 0xFF;
                            let color_g = (color_val >> 8) & 0xFF;
                            let color_b = color_val & 0xFF;

                            let mut color_str = [0u8; 20];
                            color_str[0] = b'C';
                            color_str[1] = b'o';
                            color_str[2] = b'l';
                            color_str[3] = b'o';
                            color_str[4] = b'r';
                            color_str[5] = b':';
                            color_str[6] = b' ';
                            color_str[7] = b'#';
                            let hex_chars = b"0123456789ABCDEF";
                            color_str[8] = hex_chars[(color_r >> 4) as usize];
                            color_str[9] = hex_chars[(color_r & 0xF) as usize];
                            color_str[10] = hex_chars[(color_g >> 4) as usize];
                            color_str[11] = hex_chars[(color_g & 0xF) as usize];
                            color_str[12] = hex_chars[(color_b >> 4) as usize];
                            color_str[13] = hex_chars[(color_b & 0xF) as usize];
                            color_str[14] = 0;

                            let color_info = core::str::from_utf8(&color_str[..14]).unwrap_or("Color: #------");
                            gfx.draw_text(cx + 5, cy + win.height - 25, tool_name, Color::BLACK.to_u32(), t.window_bg.to_u32());
                            gfx.draw_text(cx + 100, cy + win.height - 25, color_info, Color::BLACK.to_u32(), t.window_bg.to_u32());
                            unsafe {
                                if PAINT_STATUS_LEN > 0 {
                                    let st = core::str::from_utf8(&PAINT_STATUS[..PAINT_STATUS_LEN]).unwrap_or("");
                                    let col = if PAINT_STATUS_OK { Color::rgb(0, 120, 0) } else { Color::rgb(190, 0, 0) };
                                    gfx.draw_text(cx + 216, cy + win.height - 25, st, col.to_u32(), t.window_bg.to_u32());
                                }
                            }
                        }
                        "Wallpaper Selector" => {
                            gfx.draw_text(cx + 10, cy + 10, "Select Wallpaper Color:", Color::BLACK.to_u32(), t.window_bg.to_u32());
                            let colors = [
                                ("Sky Blue", Color::rgb(89, 0, 255)),
                                ("Dark Teal", Color::rgb(0, 23, 128)),
                                ("Red", Color::rgb(192, 0, 0)),
                                ("Yellow", Color::rgb(192, 192, 0)),
                                ("Dark Blue", Color::rgb(0, 0, 128)),
                                ("Purple", Color::rgb(128, 0, 128)),
                            ];
                            for (i, (name, color)) in colors.iter().enumerate() {
                                let y_pos = cy + 30 + i * 30;
                                wm::draw_raised_rect(gfx, cx + 10, y_pos, 250, 24);
                                gfx.fill_rect(cx + 15, y_pos + 4, 16, 16, color.to_u32());
                                gfx.draw_text(cx + 40, y_pos + 6, name, Color::BLACK.to_u32(), t.button_face.to_u32());
                            }
                        }
                        "BMP Wallpaper Selector" => {
                            gfx.draw_text(cx + 10, cy + 10, "Select BMP Wallpaper:", Color::BLACK.to_u32(), t.window_bg.to_u32());

                            let wallpapers = [
                                ("Wallpaper 1", 0),
                                ("Wallpaper 2", 1),
                                ("Wallpaper 3", 2),
                            ];

                            for (i, (name, idx)) in wallpapers.iter().enumerate() {
                                let y_pos = cy + 30 + i * 60;
                                wm::draw_raised_rect(gfx, cx + 10, y_pos, 250, 50);

                                let preview_bmp = match idx {
                                    0 => unsafe { &BMP_IMAGE },
                                    1 => unsafe { &BMP_IMAGE2 },
                                    2 => unsafe { &BMP_IMAGE3 },
                                    _ => unsafe { &BMP_IMAGE },
                                };

                                if let Some(ref bmp) = preview_bmp {
                                    let row_size = ((bmp.width * 3 + 3) / 4) * 4;
                                    for dy in 0..40.min(bmp.height) {
                                        let bmp_y = (bmp.height - 1).saturating_sub(dy);
                                        let bmp_row_start = bmp_y * row_size;
                                        for dx in 0..40.min(bmp.width) {
                                            let pixel_offset = bmp_row_start + dx * 3;
                                            if pixel_offset + 2 < bmp.data.len() {
                                                let b = bmp.data[pixel_offset] as u32;
                                                let g = bmp.data[pixel_offset + 1] as u32;
                                                let r = bmp.data[pixel_offset + 2] as u32;
                                                let color = (r << 16) | (g << 8) | b;
                                                gfx.put_pixel(cx + 15 + dx, y_pos + 5 + dy, color);
                                            }
                                        }
                                    }
                                }

                                gfx.draw_text(cx + 65, y_pos + 20, name, Color::BLACK.to_u32(), t.button_face.to_u32());
                            }

                            let y4 = cy + 30 + 3 * 60;
                            wm::draw_raised_rect(gfx, cx + 10, y4, 250, 50);
                            gfx.draw_text(cx + 20, y4 + 10, "From disk...", Color::BLACK.to_u32(), t.button_face.to_u32());
                            gfx.draw_text(cx + 20, y4 + 28, "Choose a .BMP file", Color::rgb(90, 90, 90).to_u32(), t.button_face.to_u32());
                        }
                        "Disk Wallpaper Selector" => {
                            gfx.draw_text(cx + 10, cy + 10, "Select wallpaper from disk:", Color::BLACK.to_u32(), t.window_bg.to_u32());
                            unsafe {
                                let count = DWP_COUNT;
                                let page = DWP_PAGE;
                                for r in 0..DWP_ROWS {
                                    let i = page * DWP_ROWS + r;
                                    if i >= count { break; }
                                    let y_pos = cy + 30 + r * 26;
                                    wm::draw_raised_rect(gfx, cx + 10, y_pos, 250, 22);
                                    let en = DWP_LIST[i];
                                    let nm = core::str::from_utf8(&en.1.name[..en.1.name_len]).unwrap_or("?");
                                    gfx.draw_text(cx + 20, y_pos + 4, nm, Color::BLACK.to_u32(), t.button_face.to_u32());
                                    if DWP_SYS != 0 && en.0 == DWP_SYS {
                                        gfx.draw_text(cx + 200, y_pos + 4, "[MiUX]", Color::rgb(90, 90, 90).to_u32(), t.button_face.to_u32());
                                    }
                                }
                                let by = cy + 30 + DWP_ROWS * 26 + 4;
                                wm::draw_raised_rect(gfx, cx + 10, by, 60, 22);
                                gfx.draw_text(cx + 36, by + 4, "<", Color::BLACK.to_u32(), t.button_face.to_u32());
                                wm::draw_raised_rect(gfx, cx + 200, by, 60, 22);
                                gfx.draw_text(cx + 226, by + 4, ">", Color::BLACK.to_u32(), t.button_face.to_u32());
                                if !DWP_STATUS.is_empty() {
                                    let col = if DWP_STATUS_OK { Color::rgb(0, 120, 0) } else { Color::rgb(190, 0, 0) };
                                    gfx.draw_text(cx + 10, by + 30, DWP_STATUS, col.to_u32(), t.window_bg.to_u32());
                                }
                            }
                        }
                        "Theme Selector" => {
                            gfx.draw_text(cx + 10, cy + 10, "Select Window Theme:", Color::BLACK.to_u32(), t.window_bg.to_u32());
                            let themes = [("Aero", 0usize), ("Aero Dark", 1), ("Aero Cool (Light)", 2), ("Default", 3), ("Classic", 4)];
                            for (i, (name, _)) in themes.iter().enumerate() {
                                let y_pos = cy + 30 + i * 30;
                                wm::draw_raised_rect(gfx, cx + 10, y_pos, 250, 24);
                                gfx.draw_text(cx + 20, y_pos + 6, name, Color::BLACK.to_u32(), t.button_face.to_u32());
                            }
                        }
                        "Manage Documents" => {
                            let mode = unsafe { DOCUMENT_MANAGER_MODE };
                            let mode_text = if mode == 0 { "Save Document" } else { "Load Document" };
                            gfx.draw_text(cx + 10, cy + 10, mode_text, Color::BLACK.to_u32(), t.window_bg.to_u32());
                            gfx.draw_text(cx + 10, cy + 30, "=======================", Color::BLACK.to_u32(), t.window_bg.to_u32());

                            let count = unsafe { SAVED_DOCUMENTS_COUNT };
                            if count == 0 {
                                gfx.draw_text(cx + 10, cy + 50, "No documents saved yet", Color::rgb(128, 128, 128).to_u32(), t.window_bg.to_u32());
                            } else {
                                for i in 0..count {
                                    let y_pos = cy + 50 + i * 30;
                                    wm::draw_raised_rect(gfx, cx + 10, y_pos, 250, 24);
                                    let label = ["Document 1", "Document 2", "Document 3", "Document 4", "Document 5"];
                                    gfx.draw_text(cx + 20, y_pos + 6, label[i], Color::BLACK.to_u32(), t.button_face.to_u32());
                                    let doc = unsafe { &SAVED_DOCUMENTS[i] };
                                    let doc_len = unsafe { SAVED_DOCUMENTS_LEN[i] };
                                    let preview_len = if doc_len < 20 { doc_len } else { 20 };
                                    let preview = core::str::from_utf8(&doc[..preview_len]).unwrap_or("");
                                    gfx.draw_text(cx + 120, y_pos + 6, preview, Color::rgb(100, 100, 100).to_u32(), t.button_face.to_u32());
                                }
                            }

                            if mode == 0 && count < 5 {
                                let new_y = cy + 50 + count * 30;
                                wm::draw_raised_rect(gfx, cx + 10, new_y, 250, 24);
                                gfx.draw_text(cx + 20, new_y + 6, "Save as new document...", Color::BLACK.to_u32(), t.button_face.to_u32());
                            }

                            let close_y = cy + 200;
                            wm::draw_raised_rect(gfx, cx + 10, close_y, 100, 24);
                            gfx.draw_text(cx + 20, close_y + 6, "Close", Color::BLACK.to_u32(), t.button_face.to_u32());
                        }

                        "GL" => {
                            let app = unsafe {
                                if GL_APP_INSTANCE.is_none() {
                                    GL_APP_INSTANCE = Some(gl::GlApp::new());
                                }
                                GL_APP_INSTANCE.as_mut().unwrap()
                            };
                            gl::draw_gl_scene(gfx, cx, cy, win.width - 4, win.height - 28, app);
                        }
                        "Preview" => {
                            cre_tool::draw_preview(gfx, cx, cy, win.width - 4, win.height - 28);
                        }
                        "Calendar" => {
                            calend::draw_calendar(gfx, cx, cy, win.width - 4, win.height - 28);
                        }
                        "Clock" => {
                            gfx.fill_rect(cx, cy, win.width - 4, win.height - 28, Color::rgb(5, 5, 5).to_u32());

                            let (h, m, s) = get_time();
                            let time_str = time_to_str(h, m, s);

                            let scale = 6usize;
                            let char_w = 8 * scale;
                            let total_w = time_str.len() * char_w;
                            let start_x = cx + if win.width > total_w { (win.width - total_w) / 2 } else { 4 };
                            let start_y = cy + if win.height > 28 + 8 * scale { (win.height - 28 - 8 * scale) / 2 } else { 4 };

                            for (i, ch) in time_str.bytes().enumerate() {
                                let ch_x = start_x + i * char_w;
                                draw_big_char(gfx, ch_x, start_y, ch, scale, Color::rgb(255, 255, 255).to_u32(), Color::rgb(255, 255, 255).to_u32());
                            }

                            let small_y = start_y + 8 * scale + 20;
                            gfx.draw_text(cx + 10, small_y, "Miux 64 bit time", Color::rgb(150, 150, 150).to_u32(), Color::rgb(20, 20, 30).to_u32());
                            gfx.draw_text(cx + 10, small_y + 20, time_str, Color::rgb(200, 200, 200).to_u32(), Color::rgb(20, 20, 30).to_u32());
                        }
                        "Notepad" => {
                            gfx.fill_rect(cx, cy, win.width - 4, 28, Color::rgb(236, 233, 216).to_u32());
                            gfx.fill_rect(cx, cy + 28, win.width - 4, 1, Color::rgb(172, 168, 153).to_u32());

                            wm::draw_raised_rect(gfx, cx + 10, cy + 4, 60, 20);
                            gfx.draw_text(cx + 20, cy + 7, "Save", Color::BLACK.to_u32(), Color::rgb(212, 208, 200).to_u32());

                            wm::draw_raised_rect(gfx, cx + 80, cy + 4, 60, 20);
                            gfx.draw_text(cx + 90, cy + 7, "Load", Color::BLACK.to_u32(), Color::rgb(212, 208, 200).to_u32());

                            unsafe {
                                if NOTEPAD_STATUS_LEN > 0 {
                                    let st = core::str::from_utf8(&NOTEPAD_STATUS[..NOTEPAD_STATUS_LEN]).unwrap_or("");
                                    let col = if NOTEPAD_STATUS_OK { Color::rgb(0, 120, 0) } else { Color::rgb(190, 0, 0) };
                                    gfx.draw_text(cx + 150, cy + 7, st, col.to_u32(), Color::rgb(236, 233, 216).to_u32());
                                }
                            }

                            let text_y = cy + 32;
                            gfx.draw_text(cx + 10, text_y + 10, "Notepad", Color::BLACK.to_u32(), t.window_bg.to_u32());
                            gfx.draw_text(cx + 10, text_y + 30, "=======================", Color::BLACK.to_u32(), t.window_bg.to_u32());

                            let notepad_len = unsafe { NOTEPAD_LEN };
                            let notepad_text = unsafe { &NOTEPAD_TEXT };
                            unsafe { np_fix(); }
                            let np_cur = unsafe { NP_CUR };
                            let np_sel_range = unsafe { np_sel() };
                            let base_y = text_y + 50;
                            let max_y = text_y + 300;
                            let sel_bg = Color::rgb(49, 106, 197).to_u32();
                            let mut caret = (0usize, 0usize);
                            let mut caret_found = false;
                            let (end_row, end_col) = np_layout(notepad_text, notepad_len, |s, e, row, col| {
                                let y = base_y + row * 16;
                                let x = cx + 10 + col * 8;
                                let selected = match np_sel_range { Some((a, b)) => s >= a && s < b, None => false };
                                if s == np_cur { caret = (row, col); caret_found = true; }
                                if y < max_y {
                                    if notepad_text[s] == b'\n' {
                                        if selected { gfx.fill_rect(x, y, 8, 16, sel_bg); }
                                    } else {
                                        let st = core::str::from_utf8(&notepad_text[s..e]).unwrap_or(" ");
                                        if selected {
                                            gfx.draw_text(x, y, st, Color::WHITE.to_u32(), sel_bg);
                                        } else {
                                            gfx.draw_text(x, y, st, Color::BLACK.to_u32(), t.window_bg.to_u32());
                                        }
                                    }
                                }
                            });
                            if !caret_found { caret = (end_row, end_col); }
                            let caret_y = base_y + caret.0 * 16;
                            if caret_y < max_y {
                                gfx.fill_rect(cx + 10 + caret.1 * 8, caret_y, 1, 16, Color::BLACK.to_u32());
                            }
                        }
                        "File Manager" => {
                            draw_file_manager_content(gfx, cx, cy, win.width - 4, win.height - 28);
                        }
                        "EXT4 Files" => {
                            draw_ext4_window_content(gfx, cx, cy, win.width - 4, win.height - 28);
                        }
                        "About FileMGR" => {
                            gfx.draw_text(cx + 20, cy + 40, "File MGR MiUX 7.0.0", Color::BLACK.to_u32(), t.window_bg.to_u32());
                            gfx.draw_text(cx + 20, cy + 68, "By Mikhail", Color::BLACK.to_u32(), t.window_bg.to_u32());
                        }
                        "About Files" => {
                            gfx.draw_text(cx + 10, cy + 10, "About Files", Color::BLACK.to_u32(), t.window_bg.to_u32());
                            gfx.draw_text(cx + 10, cy + 30, "=======================", Color::BLACK.to_u32(), t.window_bg.to_u32());
                            gfx.draw_text(cx + 10, cy + 50, "File Manager Application", Color::BLACK.to_u32(), t.window_bg.to_u32());
                            gfx.draw_text(cx + 10, cy + 70, "Created by Mikhail", Color::BLACK.to_u32(), t.window_bg.to_u32());
                            gfx.draw_text(cx + 10, cy + 90, "Year: 2026", Color::BLACK.to_u32(), t.window_bg.to_u32());
                            gfx.draw_text(cx + 10, cy + 110, "=======================", Color::BLACK.to_u32(), t.window_bg.to_u32());
                            gfx.draw_text(cx + 10, cy + 130, "MiUX 7.0.0", Color::BLACK.to_u32(), t.window_bg.to_u32());
                        }
                        "Task Manager" => {
                            task_manager::draw_task_manager(gfx, cx, cy, win.width - 4, win.height - 28);
                        }
                        "Cre Tool" => {
                            cre_tool::draw_cre_tool(gfx, cx, cy, win.width - 4, win.height - 28);
                        }
                        _ => {
                            gfx.draw_text(cx + 10, cy + 10, "File Manager", Color::BLACK.to_u32(), t.window_bg.to_u32());
                            gfx.draw_text(cx + 10, cy + 30, "File System: EXT4", Color::BLACK.to_u32(), t.window_bg.to_u32());
                            gfx.draw_text(cx + 10, cy + 46, "All system files:", Color::BLACK.to_u32(), t.window_bg.to_u32());
                            gfx.draw_text(cx + 10, cy + 62, "Kernel.bin , MiDE.bin , kursor.bin", Color::BLACK.to_u32(), t.window_bg.to_u32());
                            gfx.draw_text(cx + 10, cy + 78, "mouse.rs , boot.cfg , config(GRUB)", Color::BLACK.to_u32(), t.window_bg.to_u32());
                            gfx.draw_text(cx + 10, cy + 88, "shell.rs , gui , vesa.rs", Color::BLACK.to_u32(), t.window_bg.to_u32());
                            gfx.draw_text(cx + 10, cy + 97, "boot.cfg , termenal.rs , pakeg.zip", Color::BLACK.to_u32(), t.window_bg.to_u32());
                        }
                    }
                }
            }

            fn redraw_all(gfx: &mut Graphics, wm: &WindowManager, term_input: &[u8], term_len: usize) {
                if unsafe { !GUI_MODE } {
                    draw_text_console(gfx);
                    return;
                }
                if BARE_DESKTOP {
                    gfx.clear(Color::rgb(0, 0, 170).to_u32());
                    wm.draw_all_with_content(gfx, &|gfx, win, idx| {
                        draw_window_content(gfx, win, idx);
                    });
                    unsafe {
                        if CONTEXT_MENU_OPEN {
                            draw_context_menu(gfx, CONTEXT_MENU_X, CONTEXT_MENU_Y);
                        }
                        if NAME_INPUT_OPEN {
                            draw_name_input_dialog(gfx);
                        }
                        if TUTORIAL_OPEN {
                            draw_tutorial_window(gfx);
                        }
                    }
                    return;
                }
                let use_bmp = unsafe { USE_BMP_WALLPAPER };
                let current_idx = unsafe { CURRENT_BMP_INDEX };

                if use_bmp {
                    let bmp_to_use = match current_idx {
                        0 => unsafe { &BMP_IMAGE },
                        1 => unsafe { &BMP_IMAGE2 },
                        2 => unsafe { &BMP_IMAGE3 },
                        3 => unsafe { &DISK_WP },
                        _ => unsafe { &BMP_IMAGE },
                    };

                    if let Some(ref bmp) = bmp_to_use {
                        bmp.draw_as_wallpaper(gfx);
                    } else {
                        let wallpaper = unsafe { WALLPAPER };
                        gfx.clear(wallpaper.to_u32());
                    }
                } else {
                    let wallpaper = unsafe { WALLPAPER };
                    gfx.clear(wallpaper.to_u32());
                }

                let t = wm::get_theme();
                let taskbar_y = gfx.height - cui::TASKBAR_H;

                let icon_size = 64;
                let icon_y = 50;
                let icon_bg_color = if use_bmp { Color::rgb(0, 0, 0) } else { unsafe { WALLPAPER } };
                unsafe { ICON_LIM = (gfx.width, taskbar_y); }

                let icons_data: [(usize, &str, &Option<BmpImage>); 7] = [
                    (50, "Terminal", unsafe { &ICON_TERMINAL }),
                    (150, "Files", unsafe { &ICON_FILES }),
                    (250, "Settings", unsafe { &ICON_SETTINGS }),
                    (350, "AboutOS", unsafe { &ICON_ABOUT }),
                    (450, "Notepad", unsafe { &ICON_NOTEPAD }),
                    (550, "Paint", unsafe { &ICON_PAINT }),
                    (650, "Cre Tool", unsafe { &ICON_TOOL }),
                ];

                for (n, (_x, lbl, icon)) in icons_data.iter().enumerate() {
                    let (xx, icon_y) = sys_icon_pos(n);
                    let x = &xx;
                    wm::draw_raised_rect(gfx, *x, icon_y, icon_size, icon_size);
                    draw_icon(gfx, *x + 4, icon_y + 4, icon, icon_size - 8);
                    gfx.draw_text(*x + (icon_size - lbl.len() * 8) / 2, icon_y + icon_size + 5, lbl, t.desktop_icon_text.to_u32(), icon_bg_color.to_u32());
                }

                for (i, lbl) in ["Browser", "Network"].iter().enumerate() {
                    let (x, icon_y) = sys_icon_pos(7 + i);
                    wm::draw_raised_rect(gfx, x, icon_y, icon_size, icon_size);
                    if i == 0 { netwin::draw_browser_icon(gfx, x + 4, icon_y + 4, icon_size - 8); }
                    else { netwin::draw_network_icon(gfx, x + 4, icon_y + 4, icon_size - 8); }
                    gfx.draw_text(x + (icon_size - lbl.len() * 8) / 2, icon_y + icon_size + 5, lbl, t.desktop_icon_text.to_u32(), icon_bg_color.to_u32());
                }

                unsafe {
                    let mut row = 0;
                    let folders_per_row = 6;

                    for i in 0..DESKTOP_FOLDERS_COUNT {
                        let folder_name = core::str::from_utf8(&DESKTOP_FOLDERS[i]).unwrap_or("");
                        let folder_name_trimmed = folder_name.trim_end_matches('\0');

                        if folder_name_trimmed.len() > 0 {
                            let (current_x, current_y) = folder_pos_or(i, 50 + (i % folders_per_row) * 100, icon_y + icon_size + 40 + (row * (icon_size + 30)));

                            if current_y + icon_size < taskbar_y {
                                draw_folder_icon(gfx, current_x, current_y, folder_name_trimmed);
                            }

                            if (i + 1) % folders_per_row == 0 {
                                row += 1;
                            }
                        }
                    }
                }

                for i in 0..syscfg::shortcut_count() {
                    if let Some(sc) = syscfg::shortcut(i) {
                        let (sx, sy) = shortcut_pos(i);
                        if sy + icon_size < taskbar_y {
                            if sc.is_dir {
                                draw_folder_icon(gfx, sx, sy, sc.label());
                            } else {
                                draw_shortcut_icon(gfx, sx, sy, sc.label());
                            }
                        }
                    }
                }

                wm.draw_all_with_content(gfx, &|gfx, win, idx| {
                    draw_window_content(gfx, win, idx);
                });

                let (h, m, s) = get_time();
                wm.draw_taskbar(gfx, time_to_str(h, m, s));

                unsafe {
                    if CONTEXT_MENU_OPEN {
                        draw_context_menu(gfx, CONTEXT_MENU_X, CONTEXT_MENU_Y);
                    }
                    if NAME_INPUT_OPEN {
                        draw_name_input_dialog(gfx);
                    }
                    if TUTORIAL_OPEN {
                        draw_tutorial_window(gfx);
                    }
                }
            }

            fn drag_redraw(gfx: &mut Graphics, wm: &WindowManager, term_input: &[u8], term_len: usize, _r: (usize, usize, usize, usize)) {
                redraw_all(gfx, wm, term_input, term_len);
            }

            fn draw_cursor_now(gfx: &mut Graphics, cx: usize, cy: usize, backup: &mut [[u32; 16]; 16], shape: &[[u32; 16]; 16]) {
                for dy in 0..16 {
                    for dx in 0..16 {
                        if cx + dx < gfx.width && cy + dy < gfx.height {
                            backup[dy][dx] = unsafe { graphics::BACKBUFFER[(cy + dy) * gfx.width + cx + dx] };
                        }
                    }
                }
                for dy in 0..16 {
                    for dx in 0..16 {
                        let p = shape[dy][dx];
                        if p == 0 { continue; }
                        let c = if p == 1 { Color::BLACK.to_u32() } else { Color::WHITE.to_u32() };
                        gfx.put_pixel(cx + dx, cy + dy, c);
                    }
                }
            }

            fn pkg_set_last_line(line: &str) {
                unsafe {
                    if TERM_HIST_LEN == 0 { return; }
                    let l = TERM_HIST_LEN - 1;
                    TERM_HISTORY[l] = [0; 80];
                    let bytes = clip_str(line, 80).as_bytes();
                    TERM_HISTORY[l][..bytes.len()].copy_from_slice(bytes);
                }
            }

            fn pkg_print(text: &str, first_replaces: bool) {
                let mut first = first_replaces;
                let mut chunk = [0u8; 80];
                let mut n = 0usize;
                let mut chars = 0usize;
                for ch in text.chars() {
                    let mut tmp = [0u8; 4];
                    let s = ch.encode_utf8(&mut tmp);
                    if chars >= 36 || n + s.len() > 78 {
                        let line = core::str::from_utf8(&chunk[..n]).unwrap_or("");
                        if first { pkg_set_last_line(line); first = false; } else { push_history(line); }
                        n = 0;
                        chars = 0;
                    }
                    chunk[n..n + s.len()].copy_from_slice(s.as_bytes());
                    n += s.len();
                    chars += 1;
                }
                let line = core::str::from_utf8(&chunk[..n]).unwrap_or("");
                if first { pkg_set_last_line(line); } else if n > 0 { push_history(line); }
            }

            fn pkg_tick() {
                unsafe {
                    if !PKG.active || PKG.busy { return; }
                    let now = timer::millis();
                    if now.wrapping_sub(PKG.last) < 120 { return; }
                    PKG.last = now;
                    PKG.busy = true;
                    pkg_set_last_line(PKG_FRAMES[PKG.frame]);
                    PKG.frame = (PKG.frame + 1) % PKG_FRAMES.len();
                    redraw_all(&mut *PKG.gfx, &*PKG.wm, &*PKG.ti, PKG.tl);
                    if GUI_MODE {
                        draw_cursor_now(&mut *PKG.gfx, PKG.cx, PKG.cy, &mut *PKG.cb, &*PKG.cs);
                    }
                    (&mut *PKG.gfx).flush();
                    PKG.busy = false;
                }
            }

            fn pkg_install_tar(stem: &str, body: &[u8]) {
                let res = unsafe {
                    let mut r: Option<Result<fs::fat32::TarInstall, &'static str>> = None;
                    if FILE_MANAGER_MOUNTED && FILE_MANAGER_DIALOG_DISK < 8 {
                        if let Some(f) = &FAT32_FILESYSTEMS[FILE_MANAGER_DIALOG_DISK] {
                            r = Some(f.install_tar(stem, body));
                        }
                    }
                    match r {
                        Some(x) => x,
                        None => fs::fat32::install_tar(stem, body),
                    }
                };
                match res {
                    Err(e) => pkg_print(&alloc::format!("pkg: {}", e), true),
                    Ok(t) => {
                        let mut dn = [0u8; 8];
                        let mut dl = 0usize;
                        if let Some(ds) = fs::fat32::make_dir_name(stem) {
                            while dl < 8 && ds[dl] != b' ' {
                                dn[dl] = ds[dl];
                                dl += 1;
                            }
                        }
                        let dname = core::str::from_utf8(&dn[..dl]).unwrap_or("");
                        pkg_print(&alloc::format!("Installed: {} ({} files)", dname, t.files), true);
                        if t.has_main && t.has_app {
                            pkg_print("main.py and app.py both found: no shortcut", false);
                        } else if !t.has_main && !t.has_app {
                            pkg_print("no main.py or app.py: no shortcut", false);
                        } else {
                            let target = if t.has_main { "main.py" } else { "app.py" };
                            let path = alloc::format!("/{}/{}", dname, target);
                            match syscfg::add_shortcut(t.disk, &path) {
                                Ok(()) => pkg_print(&alloc::format!("Shortcut: {}", path), false),
                                Err("Shortcut already exists") => pkg_print("Shortcut already exists", false),
                                Err(e) => pkg_print(&alloc::format!("pkg: {}", e), false),
                            }
                        }
                    }
                }
            }

            fn pkg_install(gfx: &mut Graphics, wm: &WindowManager, ti: &[u8; 256], tl: usize,
                           cx: usize, cy: usize, cb: &mut [[u32; 16]; 16], cs: &[[u32; 16]; 16]) {
                let url_s = unsafe { core::str::from_utf8(&PKG_URL[..PKG_URL_LEN]).unwrap_or("") };
                let url_full = if url_s.contains("://") { alloc::string::String::from(url_s) }
                               else { alloc::format!("https://{}", url_s) };
                let url = match net::http::Url::parse(&url_full) {
                    Some(u) => u,
                    None => { push_history("pkg: bad address"); return; }
                };
                let path_only = match url.path.find('?') { Some(i) => &url.path[..i], None => &url.path[..] };
                let fname = path_only.rsplit('/').next().unwrap_or("");
                if fname.is_empty() || fs::fat32::make_short_name(fname).is_none() {
                    push_history("pkg: no file name in address");
                    push_history("example: pkg install https://site.com/app.py");
                    return;
                }

                push_history(PKG_FRAMES[0]);
                unsafe {
                    PKG.gfx = gfx as *mut Graphics;
                    PKG.wm = wm as *const WindowManager;
                    PKG.ti = ti as *const [u8; 256];
                    PKG.tl = tl;
                    PKG.cx = cx;
                    PKG.cy = cy;
                    PKG.cb = cb as *mut [[u32; 16]; 16];
                    PKG.cs = cs as *const [[u32; 16]; 16];
                    PKG.frame = 0;
                    PKG.last = 0;
                    PKG.busy = false;
                    PKG.active = true;
                    net::TICK_HOOK = Some(pkg_tick);
                }
                pkg_tick();

                let mut nop = |_: &str| {};
                let res = net::http::fetch(&url, None, &mut nop);

                unsafe {
                    net::TICK_HOOK = None;
                    PKG.active = false;
                }

                match res {
                    Err(e) => pkg_print(&e, true),
                    Ok(r) => {
                        if r.status != 200 {
                            pkg_print(&alloc::format!("pkg: server answered HTTP {}", r.status), true);
                        } else if fname.len() > 4 && fname.as_bytes()[fname.len() - 4..].eq_ignore_ascii_case(b".tar") {
                            pkg_install_tar(&fname[..fname.len() - 4], &r.body);
                        } else {
                            match save_bytes_fat32(fname, &r.body) {
                                Ok(()) => {
                                    let mut d = [0u8; 13];
                                    let dl = fs::fat32::display_name(fname, &mut d);
                                    let ds = core::str::from_utf8(&d[..dl]).unwrap_or("");
                                    pkg_print(&alloc::format!("Installed: {} ({} bytes)", ds, r.body.len()), true);
                                }
                                Err(e) => pkg_print(&alloc::format!("pkg: {}", e), true),
                            }
                        }
                    }
                }
            }

            fn open_or_focus(wm: &mut WindowManager, title: &'static str, w: usize, h: usize) {
                let x = if wm.screen_w > w + 200 { 160 } else { 20 };
                wm.create_window(title, x, 60, w, h);
            }

            fn text_banner() {
                push_history("MiUX 7.0.0");
                push_history("Text mode. Please log in.");
                push_history("(after login: 'gui /start --all' starts the graphical mode)");
                push_history("");
            }

            fn text_key_char(sc: u8) -> Option<char> {
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
                c.map(|ch| web::keys::sys_char(sc, ch))
            }

            text_banner();
            redraw_all(&mut gfx, &wm, &term_input, term_len);
            gfx.flush();

            unsafe { idt::init_idt(); }
            mouse::init_ps2_mouse();
            unsafe { mouse::init_mouse_interrupts(); }

            let mut cursor_x = 400usize;
            let mut cursor_y = 300usize;
            let mut cursor_backup = [[0u32; 16]; 16];
            let mut left_prev = false;
            let mut right_prev = false;

            let cursor_size = 16;
            let cursor_shape: [[u32; 16]; 16] = [
                [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
                [0,0,1,0,0,0,0,0,0,0,0,0,0,0,0,0],
                [0,1,2,1,0,0,0,0,0,0,0,0,0,0,0,0],
                [0,1,2,2,1,0,0,0,0,0,0,0,0,0,0,0],
                [0,1,2,2,2,1,0,0,0,0,0,0,0,0,0,0],
                [0,1,2,2,2,2,1,0,0,0,0,0,0,0,0,0],
                [0,1,2,2,2,2,2,1,0,0,0,0,0,0,0,0],
                [0,1,2,2,2,2,2,2,1,0,0,0,0,0,0,0],
                [0,1,2,2,2,2,2,2,2,1,0,0,0,0,0,0],
                [0,1,2,2,2,2,2,2,2,2,1,0,0,0,0,0],
                [0,1,2,2,2,2,2,1,1,1,1,1,1,0,0,0],
                [0,1,2,2,2,1,2,2,1,0,0,0,0,0,0,0],
                [0,1,2,2,1,0,1,2,2,1,0,0,0,0,0,0],
                [0,1,2,1,0,0,1,2,2,1,0,0,0,0,0,0],
                [0,1,1,0,0,0,0,1,2,2,1,0,0,0,0,0],
                [0,0,0,0,0,0,0,0,1,1,1,0,0,0,0,0],
            ];

            let mut clock_ticks: u32 = 0;

            loop {
                syscfg::poll();
                let mut need_flush = false;
                let mut dirty = fastblit::Dirty::new();

                let net_changed = net::poll();

                unsafe {
                    if POWER_REQUEST {
                        POWER_REQUEST = false;
                        shutdown_sequence(&mut gfx);
                        gfx.clear(Color::BLACK.to_u32());
                        gfx.draw_text(
                            (gfx.width - 16 * 17) / 2,
                            gfx.height / 2 - 8,
                            "Now you can turn off the power",
                            Color::WHITE.to_u32(),
                            Color::BLACK.to_u32()
                        );
                        gfx.flush();
                        loop { core::arch::asm!("hlt") }
                    }

                    if GUI_START_REQUESTED {
                        GUI_START_REQUESTED = false;
                        if LOGGED_IN && !GUI_MODE {
                            GUI_MODE = true;
                            cursor_x = (gfx.width / 2).min(gfx.width - cursor_size);
                            cursor_y = (gfx.height / 2).min(gfx.height - cursor_size);
                            left_prev = false;
                            right_prev = false;
                            TERM_SCROLL = 0;
                            wm.create_window("Terminal", 100, 100, 500, 350);
                            redraw_all(&mut gfx, &wm, &term_input, term_len);
                            draw_cursor_now(&mut gfx, cursor_x, cursor_y, &mut cursor_backup, &cursor_shape);
                            need_flush = true;
                        }
                    }

                }
                unsafe {
                    if NOTEPAD_WINDOW_REQUESTED {
                        NOTEPAD_WINDOW_REQUESTED = false;
                        if GUI_MODE {
                            wm.create_window("Notepad", 160, 140, 500, 400);
                            redraw_all(&mut gfx, &wm, &term_input, term_len);
                            draw_cursor_now(&mut gfx, cursor_x, cursor_y, &mut cursor_backup, &cursor_shape);
                            need_flush = true;
                        }
                    }

                    if GUI_MODE {
                        if let Some(ti) = wm.find_window("Terminal") {
                            let minimized = match wm.windows[ti] {
                                Some(ref w) => w.state == WindowState::Minimized,
                                None => false,
                            };
                            if minimized {
                                wm.restore_window(ti);
                                redraw_all(&mut gfx, &wm, &term_input, term_len);
                                draw_cursor_now(&mut gfx, cursor_x, cursor_y, &mut cursor_backup, &cursor_shape);
                                need_flush = true;
                            }
                        }
                    }

                    if GUI_MODE && (GUI_STOP_REQUESTED || !LOGGED_IN || wm.find_window("Terminal").is_none()) {
                        GUI_STOP_REQUESTED = false;
                        while wm.window_count > 0 {
                            let last = wm.window_count - 1;
                            wm.close_window(last);
                        }
                        GUI_MODE = false;
                        TERM_SCROLL = 0;
                        if LOGGED_IN {
                            push_history("GUI stopped. Back to text mode.");
                        } else {
                            clear_history();
                            text_banner();
                            LOGIN_IS_PASSWORD = false;
                            LOGIN_USERNAME_LEN = 0;
                            LOGIN_PASSWORD_LEN = 0;
                        }
                        term_len = 0;
                        TERM_LEN_GLOBAL = 0;
                        redraw_all(&mut gfx, &wm, &term_input, term_len);
                        need_flush = true;
                    }
                }

                unsafe {
                    if !LOGGED_IN && users::autologin_pending() {
                        if users::try_autologin() {
                            LOGGED_IN = true;
                            LOGIN_ERROR_LEN = 0;
                            LOGIN_USERNAME_LEN = 0;
                            LOGIN_PASSWORD_LEN = 0;
                            USER_LOGIN_ATTEMPTS = 0;
                            push_history("Logged in automatically (autologin).");
                            redraw_all(&mut gfx, &wm, &term_input, term_len);
                            need_flush = true;
                        }
                    }
                }
                unsafe {
                    if LOGGED_IN && GUI_MODE {
                        if NET_APP_REQUEST != 0 {
                            let (title, w, h) = if NET_APP_REQUEST == 1 { ("Browser", 1000, 720) } else { (netwin::TITLE, 560, 560) };
                            NET_APP_REQUEST = 0;
                            open_or_focus(&mut wm, title, w, h);
                            redraw_all(&mut gfx, &wm, &term_input, term_len);
                            need_flush = true;
                        }
                        if mouse::take_layout_changed() {
                            redraw_all(&mut gfx, &wm, &term_input, term_len);
                            draw_cursor_now(&mut gfx, cursor_x, cursor_y, &mut cursor_backup, &cursor_shape);
                            need_flush = true;
                        }
                        let browser_open = wm.windows.iter().flatten().any(|w| w.title == "Browser" && w.state != WindowState::Minimized);
                        let net_open = wm.windows.iter().flatten().any(|w| w.title == netwin::TITLE && w.state != WindowState::Minimized);
                        let htmlapp_open = wm.windows.iter().flatten().any(|w| w.title == "HTML App" && w.state != WindowState::Minimized);
                        let mut again = false;
                        if browser_open && web::app::poll(&mut gfx) { again = true; }
                        if htmlapp_open && web::app::app_poll(&mut gfx) { again = true; }
                        if net_open && (net_changed || netwin::changed()) { again = true; }
                        if again && wm.dragged_window.is_none() {
                            redraw_all(&mut gfx, &wm, &term_input, term_len);
                            draw_cursor_now(&mut gfx, cursor_x, cursor_y, &mut cursor_backup, &cursor_shape);
                            need_flush = true;
                        }
                    }
                }

                clock_ticks = clock_ticks.wrapping_add(1);
                unsafe {
                    if LOGGED_IN && GUI_MODE && clock_ticks >= 10000 {
                        clock_ticks = 0;
                        let (_, _, s) = get_time();
                        if s != LAST_CLOCK_SEC {
                            LAST_CLOCK_SEC = s;

                            if LIVE_WALLPAPER_ENABLED {
                                LIVE_WALLPAPER_ELAPSED_SEC += 1;
                                if LIVE_WALLPAPER_ELAPSED_SEC >= LIVE_WALLPAPER_INTERVAL_SEC {
                                    LIVE_WALLPAPER_ELAPSED_SEC = 0;
                                    CURRENT_BMP_INDEX = pick_random_bmp_index(CURRENT_BMP_INDEX);
                                    USE_BMP_WALLPAPER = true;
                                }
                            }

                            let mut saved_cursor = [[0u32; 16]; 16];
                            for dy in 0..cursor_size {
                                for dx in 0..cursor_size {
                                    if cursor_x + dx < gfx.width && cursor_y + dy < gfx.height {
                                        let offset = (cursor_y + dy) * gfx.width + (cursor_x + dx);
                                        saved_cursor[dy][dx] = graphics::BACKBUFFER[offset];
                                    }
                                }
                            }

                            redraw_all(&mut gfx, &wm, &term_input, term_len);

                            for dy in 0..cursor_size {
                                for dx in 0..cursor_size {
                                    let pixel = cursor_shape[dy][dx];
                                    if pixel == 0 { continue; }

                                    let color = match pixel {
                                        1 => Color::BLACK.to_u32(),
                                        2 => Color::WHITE.to_u32(),
                                        _ => continue,
                                    };

                                    let mut is_border = false;
                                    if pixel == 1 {
                                        for ndy in -1..=1 {
                                            for ndx in -1..=1 {
                                                if ndx == 0 && ndy == 0 { continue; }
                                                let nx = dx as i32 + ndx;
                                                let ny = dy as i32 + ndy;
                                                if nx >= 0 && nx < cursor_size as i32 && ny >= 0 && ny < cursor_size as i32 {
                                                    if cursor_shape[ny as usize][nx as usize] == 0 {
                                                        is_border = true;
                                                        break;
                                                    }
                                                } else {
                                                    is_border = true;
                                                    break;
                                                }
                                            }
                                            if is_border { break; }
                                        }
                                    }

                                    let final_color = if is_border {
                                        Color::BLACK.to_u32()
                                    } else {
                                        color
                                    };

                                    gfx.put_pixel(cursor_x + dx, cursor_y + dy, final_color);
                                }
                            }

                            need_flush = true;
                        }
                    }
                }

                if let Some(sc) = mouse::get_key() {
                    unsafe {
                        if !GUI_MODE {
                            if sc & 0x80 == 0 {
                                if let Some(k) = web::keys::ext(sc) {
                                    let rows = gfx.height.saturating_sub(16) / 16;
                                    let step = (rows / 2).max(1);
                                    if k == web::keys::PGUP {
                                        let max_scroll = TERM_HIST_LEN.saturating_sub(rows);
                                        TERM_SCROLL = (TERM_SCROLL + step).min(max_scroll);
                                    } else if k == web::keys::PGDN {
                                        TERM_SCROLL = TERM_SCROLL.saturating_sub(step);
                                    }
                                    redraw_all(&mut gfx, &wm, &term_input, term_len);
                                    need_flush = true;
                                } else if let Some(ch) = text_key_char(sc) {
                                    TERM_SCROLL = 0;
                                    if !LOGGED_IN {
                                        if ch == '\x08' {
                                            if LOGIN_IS_PASSWORD {
                                                if LOGIN_PASSWORD_LEN > 0 { LOGIN_PASSWORD_LEN -= 1; }
                                            } else if LOGIN_USERNAME_LEN > 0 {
                                                LOGIN_USERNAME_LEN -= 1;
                                            }
                                        } else if ch == '\n' {
                                            if !LOGIN_IS_PASSWORD {
                                                LOGIN_IS_PASSWORD = true;
                                            } else {
                                                let uname = alloc::string::String::from(
                                                    core::str::from_utf8(&LOGIN_USERNAME_INPUT[..LOGIN_USERNAME_LEN]).unwrap_or("")
                                                );
                                                let ok = users::login(
                                                    &uname,
                                                    core::str::from_utf8(&LOGIN_PASSWORD_INPUT[..LOGIN_PASSWORD_LEN]).unwrap_or("")
                                                );
                                                push_history(&alloc::format!("miux login: {}", uname));
                                                push_history("Password: ");
                                                LOGIN_PASSWORD_INPUT = [0; 32];
                                                LOGIN_PASSWORD_LEN = 0;
                                                LOGIN_USERNAME_LEN = 0;
                                                LOGIN_IS_PASSWORD = false;
                                                LOGIN_ERROR_LEN = 0;
                                                if ok {
                                                    LOGGED_IN = true;
                                                    USER_LOGIN_ATTEMPTS = 0;
                                                    term_len = 0;
                                                    TERM_LEN_GLOBAL = 0;
                                                    push_history(&alloc::format!("Welcome, {}!  Type 'help' for the command list.", uname));
                                                    push_history("Graphical mode: gui /start --all");
                                                    push_history("");
                                                } else {
                                                    USER_LOGIN_ATTEMPTS += 1;
                                                    if USER_LOGIN_ATTEMPTS >= 3 {
                                                        USER_LOGIN_LOCKED = true;
                                                        push_history("System locked!");
                                                    } else {
                                                        push_history("Login incorrect");
                                                    }
                                                    push_history("");
                                                }
                                            }
                                        } else if (ch as u32) >= 0x20 && (ch as u32) < 0x7F {
                                            if LOGIN_IS_PASSWORD {
                                                if LOGIN_PASSWORD_LEN < 31 {
                                                    LOGIN_PASSWORD_INPUT[LOGIN_PASSWORD_LEN] = ch as u8;
                                                    LOGIN_PASSWORD_LEN += 1;
                                                }
                                            } else if LOGIN_USERNAME_LEN < 31 {
                                                LOGIN_USERNAME_INPUT[LOGIN_USERNAME_LEN] = ch as u8;
                                                LOGIN_USERNAME_LEN += 1;
                                            }
                                        }
                                    } else {
                                        if ch == '\n' {
                                            let cmd_owned = alloc::string::String::from(
                                                core::str::from_utf8(&term_input[..term_len]).unwrap_or("")
                                            );
                                            let cmd = cmd_owned.as_str();
                                            let pline = alloc::format!("{}{}", termfs::prompt(), cmd);
                                            let out = exec_command(cmd);
                                            if out == "\x04" {
                                                clear_history();
                                            } else if out == "\x05" {
                                                show_blue_screen_of_death(&mut gfx);
                                            } else if out == "\x06" {
                                                show_blue_screen_stop_gui(&mut gfx);
                                            } else if out == "\x0E" {
                                                push_history(&pline);
                                                let rows = tech_info_lines();
                                                for (k, row) in rows.iter().enumerate() {
                                                    let mut line = alloc::string::String::new();
                                                    line.push('\x0E');
                                                    line.push((b'0' + k as u8) as char);
                                                    line.push_str(row);
                                                    push_history(&line);
                                                }
                                            } else {
                                                push_history(&pline);
                                                if !out.is_empty() {
                                                    for line in out.split('\n') {
                                                        push_history(line);
                                                    }
                                                }
                                            }

                                            term_len = 0;
                                            TERM_LEN_GLOBAL = 0;

                                            if PKG_PENDING {
                                                PKG_PENDING = false;
                                                pkg_install(&mut gfx, &wm, &term_input, term_len,
                                                            cursor_x, cursor_y, &mut cursor_backup, &cursor_shape);
                                            }

                                            if !LOGGED_IN {
                                                clear_history();
                                                text_banner();
                                                LOGIN_IS_PASSWORD = false;
                                                LOGIN_USERNAME_LEN = 0;
                                                LOGIN_PASSWORD_LEN = 0;
                                            }
                                        } else if ch == '\x08' {
                                            if term_len > 0 {
                                                utf8_pop(&term_input, &mut term_len);
                                                TERM_LEN_GLOBAL = term_len;
                                                TERM_INPUT_GLOBAL[..term_len].copy_from_slice(&term_input[..term_len]);
                                            }
                                        } else if (ch as u32) >= 0x20 && utf8_push(&mut term_input, &mut term_len, 255, ch) {
                                            TERM_LEN_GLOBAL = term_len;
                                            TERM_INPUT_GLOBAL[..term_len].copy_from_slice(&term_input[..term_len]);
                                        }
                                    }
                                    redraw_all(&mut gfx, &wm, &term_input, term_len);
                                    need_flush = true;
                                }
                            }
                        } else if !LOGGED_IN {
                            if sc & 0x80 == 0 {
                                let enter_pressed = handle_login_input(
                                    sc,
                                    &mut LOGIN_USERNAME_INPUT,
                                    &mut LOGIN_USERNAME_LEN,
                                    &mut LOGIN_PASSWORD_INPUT,
                                    &mut LOGIN_PASSWORD_LEN,
                                    &mut LOGIN_IS_PASSWORD
                                );

                                if enter_pressed {
                                    let username = core::str::from_utf8(&LOGIN_USERNAME_INPUT[..LOGIN_USERNAME_LEN]).unwrap_or("");
                                    let password = core::str::from_utf8(&LOGIN_PASSWORD_INPUT[..LOGIN_PASSWORD_LEN]).unwrap_or("");

                                    if users::login(username, password) {
                                        LOGGED_IN = true;
                                        LOGIN_ERROR_LEN = 0;
                                        LOGIN_USERNAME_LEN = 0;
                                        LOGIN_PASSWORD_LEN = 0;
                                        USER_LOGIN_ATTEMPTS = 0;
                                        redraw_all(&mut gfx, &wm, &term_input, term_len);
                                        need_flush = true;
                                    } else {
                                        USER_LOGIN_ATTEMPTS += 1;
                                        if USER_LOGIN_ATTEMPTS >= 3 {
                                            USER_LOGIN_LOCKED = true;
                                            let msg = b"System locked!";
                                            LOGIN_ERROR_LEN = msg.len();
                                            LOGIN_ERROR_MSG[..msg.len()].copy_from_slice(msg);
                                        } else {
                                            let msg = b"Error!";
                                            LOGIN_ERROR_LEN = msg.len();
                                            LOGIN_ERROR_MSG[..msg.len()].copy_from_slice(msg);
                                        }
                                        draw_login_window(&mut gfx);
                                        need_flush = true;
                                    }
                                }
                                draw_login_window(&mut gfx);
                                need_flush = true;
                            }
                        } else {
                            if sc & 0x80 == 0 {
                                if NAME_INPUT_OPEN {
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
                                    let c = c.map(|ch| web::keys::sys_char(sc, ch));
                                    if let Some(ch) = c {
                                        if ch == '\x08' && NAME_INPUT_LEN > 0 {
                                            utf8_pop(&NAME_INPUT_BUFFER, &mut NAME_INPUT_LEN);
                                        } else if ch == '\n' {
                                            if name_input_confirm() {
                                                NAME_INPUT_OPEN = false;
                                            }
                                        } else if (ch as u32) >= 0x20 {
                                            utf8_push(&mut NAME_INPUT_BUFFER, &mut NAME_INPUT_LEN, 63, ch);
                                        }
                                        redraw_all(&mut gfx, &wm, &term_input, term_len);
                                        need_flush = true;
                                    }
                                } else if let Some(active) = wm.active_window {
                                    if let Some(ref win) = wm.windows[active] {
                                        let is_term = win.title == "Terminal" && win.state != WindowState::Minimized;
                                        let is_notepad = win.title == "Notepad" && win.state != WindowState::Minimized;

                                        if win.title == "Browser" && win.state != WindowState::Minimized {
                                            if web::app::key(sc) {
                                                redraw_all(&mut gfx, &wm, &term_input, term_len);
                                                need_flush = true;
                                            }
                                        } else if win.title == "HTML App" && win.state != WindowState::Minimized {
                                            if web::app::app_key(sc) {
                                                redraw_all(&mut gfx, &wm, &term_input, term_len);
                                                need_flush = true;
                                            }
                                        } else if win.title == "File Manager" && win.state != WindowState::Minimized && FM_PATH_EDIT {
                                            if sc == web::keys::ESC {
                                                FM_PATH_EDIT = false;
                                                FM_PATH_ERR = "";
                                            } else if let Some(ch) = fm_sc_char(sc).map(|c| web::keys::sys_char(sc, c)) {
                                                if ch == '\x08' {
                                                    if FM_PATH_LEN > 0 {
                                                        utf8_pop(&FM_PATH_BUF, &mut FM_PATH_LEN);
                                                    }
                                                    FM_PATH_ERR = "";
                                                } else if ch == '\n' {
                                                    fm_path_confirm();
                                                } else if (ch as u32) >= 0x20 {
                                                    utf8_push(&mut FM_PATH_BUF, &mut FM_PATH_LEN, 159, ch);
                                                    FM_PATH_ERR = "";
                                                }
                                            }
                                            redraw_all(&mut gfx, &wm, &term_input, term_len);
                                            need_flush = true;
                                        } else if win.title == "Network" && win.state != WindowState::Minimized {
                                            if netwin::key(sc) {
                                                redraw_all(&mut gfx, &wm, &term_input, term_len);
                                                need_flush = true;
                                            }
                                        } else if is_term {
                                            unsafe { TERM_SCROLL = 0; }
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
                                            let c = c.map(|ch| web::keys::sys_char(sc, ch));
                                            if let Some(ch) = c {
                                                if ch == '\n' {
                                                    let cmd = core::str::from_utf8(&term_input[..term_len]).unwrap_or("");
                                                    let out = exec_command(cmd);
                                                    if out == "\x04" {
                                                        clear_history();
                                                        push_history("miux@system:~$ cls");
                                                    } else if out == "\x05" {
                                                        show_blue_screen_of_death(&mut gfx);
                                                    } else if out == "\x06" {
                                                        show_blue_screen_stop_gui(&mut gfx);
                                                    } else if out == "\x0E" {
                                                        push_history("miux@system:~$ ");
                                                        push_history(cmd);
                                                        let rows = tech_info_lines();
                                                        for (k, row) in rows.iter().enumerate() {
                                                            let mut line = alloc::string::String::new();
                                                            line.push('\x0E');
                                                            line.push((b'0' + k as u8) as char);
                                                            line.push_str(row);
                                                            push_history(&line);
                                                        }
                                                    } else if out == "\x07" {
                                                        create_folder_on_desktop("Folder");
                                                        push_history("miux@system:~$ new folder");
                                                        push_history("Folder created on desktop");
                                                    } else {
                                                        push_history("miux@system:~$ ");
                                                        if !cmd.is_empty() {
                                                            push_history(cmd);
                                                            if !out.is_empty() {
                                                                for line in out.split('\n') {
                                                                    push_history(line);
                                                                }
                                                            }
                                                        }
                                                    }

                                                    term_len = 0;

                                                    TERM_LEN_GLOBAL = 0;

                                                    if PKG_PENDING {
                                                        PKG_PENDING = false;
                                                        pkg_install(&mut gfx, &wm, &term_input, term_len,
                                                                    cursor_x, cursor_y, &mut cursor_backup, &cursor_shape);
                                                    }
                                                } else if ch == '\x08' && term_len > 0 {
                                                    utf8_pop(&term_input, &mut term_len);

                                                    TERM_LEN_GLOBAL = term_len;
                                                    TERM_INPUT_GLOBAL[..term_len].copy_from_slice(&term_input[..term_len]);
                                                } else if (ch as u32) >= 0x20 && utf8_push(&mut term_input, &mut term_len, 255, ch) {
                                                    TERM_LEN_GLOBAL = term_len;
                                                    TERM_INPUT_GLOBAL[..term_len].copy_from_slice(&term_input[..term_len]);
                                                }
                                                redraw_all(&mut gfx, &wm, &term_input, term_len);
                                                need_flush = true;
                                            }
                                        } else if is_notepad {
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
                                            let c = c.map(|ch| web::keys::sys_char(sc, ch));
                                            if let Some(ch) = c {
                                                if ch == '\x08' {
                                                    np_backspace();
                                                } else if ch == '\n' {
                                                    np_insert('\n');
                                                } else if (ch as u32) >= 0x20 {
                                                    np_insert(ch);
                                                }
                                                redraw_all(&mut gfx, &wm, &term_input, term_len);
                                                need_flush = true;
                                            }
                                        } else if win.title == "Cre Tool" {
                                            if cre_tool::handle_keyboard(sc) {
                                                redraw_all(&mut gfx, &wm, &term_input, term_len);
                                                need_flush = true;
                                            }
                                        } else if win.title == "HTML Viewer" {
                                            if let Some(ref mut app) = HTML_APP_INSTANCE {
                                                if html::handle_html_keyboard(app, sc) {
                                                    redraw_all(&mut gfx, &wm, &term_input, term_len);
                                                    need_flush = true;
                                                }
                                            }
                                        } else if win.title == "Support Center" {
                                            if let Some(ref mut app) = SUPPORT_APP_INSTANCE {
                                                if support::handle_support_keyboard(app, sc) {
                                                    redraw_all(&mut gfx, &wm, &term_input, term_len);
                                                    need_flush = true;
                                                }
                                            }
                                        } else if win.title == "PC Info" {
                                            if let Some(ref mut app) = PCINF_APP_INSTANCE {
                                                if pcinf::handle_pcinf_keyboard(app, sc) {
                                                    redraw_all(&mut gfx, &wm, &term_input, term_len);
                                                    need_flush = true;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                let mouse_pkt = if unsafe { GUI_MODE } {
                    mouse::get_mouse_packet()
                } else {
                    let _ = mouse::get_mouse_packet();
                    None
                };
                if let Some(packet) = mouse_pkt {
                    let old_x = cursor_x;
                    let old_y = cursor_y;
                    cursor_x = (cursor_x as i32 + packet.dx).max(0).min((gfx.width - cursor_size) as i32) as usize;
                    cursor_y = (cursor_y as i32 + packet.dy).max(0).min((gfx.height - cursor_size) as i32) as usize;

                    for dy in 0..cursor_size {
                        for dx in 0..cursor_size {
                            if old_x + dx < gfx.width && old_y + dy < gfx.height {
                                gfx.put_pixel(old_x + dx, old_y + dy, cursor_backup[dy][dx]);
                            }
                        }
                    }

                    let mut redraw = false;
                    let mut paint_update = false;
                    let mut drag_rect: Option<(usize, usize, usize, usize)> = None;
                    let mut raised = false;

                    if packet.wheel != 0 && unsafe { NAME_INPUT_OPEN && NAME_INPUT_MODE == 3 } {
                        unsafe { SD_VIEW.scroll_by(packet.wheel, SD_ROWS); }
                        redraw = true;
                    } else if packet.wheel != 0 && unsafe { LOGGED_IN } {
                        if let Some(i) = wm.top_at(cursor_x, cursor_y) {
                            if let Some(ref win) = wm.windows[i] {
                                if win.title == "Browser" && web::app::wheel(cursor_x, cursor_y, packet.wheel) { redraw = true; }
                                else if win.title == "HTML App" && web::app::app_wheel(cursor_x, cursor_y, packet.wheel) { redraw = true; }
                                else if win.title == "Terminal" && win.state != WindowState::Minimized {
                                    unsafe {
                                        let max_lines = win.height.saturating_sub(28 + 30) / 16;
                                        let max_scroll = TERM_HIST_LEN.saturating_sub(max_lines);
                                        let step = (packet.wheel.unsigned_abs() as usize) * 3;
                                        let old = TERM_SCROLL;
                                        if packet.wheel < 0 { TERM_SCROLL = (TERM_SCROLL + step).min(max_scroll); }
                                        else { TERM_SCROLL = TERM_SCROLL.saturating_sub(step); }
                                        if TERM_SCROLL != old { redraw = true; }
                                    }
                                }
                                else if win.title == "File Manager" && win.state != WindowState::Minimized {
                                    unsafe {
                                        if FILE_MANAGER_MOUNTED && !FILE_MANAGER_DIALOG_OPEN {
                                            FM_VIEW.scroll_by(packet.wheel, fm_visible_rows(win.height - 28));
                                            redraw = true;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    unsafe {
                        if !LOGGED_IN {
                            if packet.left && !left_prev {
                                let screen_w = gfx.width;
                                let screen_h = gfx.height;
                                let login_w = 380;
                                let login_h = 300;
                                let login_x = (screen_w - login_w) / 2;
                                let login_y = (screen_h - login_h) / 2;
                                let btn_w = 120;
                                let btn_h = 36;
                                let btn_x = login_x + (login_w - btn_w) / 2;
                                let btn_y = login_y + 240;

                                if cursor_x >= btn_x && cursor_x <= btn_x + btn_w &&
                                   cursor_y >= btn_y && cursor_y <= btn_y + btn_h {
                                    let username = core::str::from_utf8(&LOGIN_USERNAME_INPUT[..LOGIN_USERNAME_LEN]).unwrap_or("");
                                    let password = core::str::from_utf8(&LOGIN_PASSWORD_INPUT[..LOGIN_PASSWORD_LEN]).unwrap_or("");

                                    if users::login(username, password) {
                                        LOGGED_IN = true;
                                        LOGIN_ERROR_LEN = 0;
                                        LOGIN_USERNAME_LEN = 0;
                                        LOGIN_PASSWORD_LEN = 0;
                                        USER_LOGIN_ATTEMPTS = 0;
                                        redraw_all(&mut gfx, &wm, &term_input, term_len);
                                        redraw = true;
                                    } else {
                                        USER_LOGIN_ATTEMPTS += 1;
                                        if USER_LOGIN_ATTEMPTS >= 3 {
                                            USER_LOGIN_LOCKED = true;
                                            let msg = b"System locked!";
                                            LOGIN_ERROR_LEN = msg.len();
                                            LOGIN_ERROR_MSG[..msg.len()].copy_from_slice(msg);
                                        } else {
                                            let msg = b"Error!";
                                            LOGIN_ERROR_LEN = msg.len();
                                            LOGIN_ERROR_MSG[..msg.len()].copy_from_slice(msg);
                                        }
                                        draw_login_window(&mut gfx);
                                        redraw = true;
                                    }
                                }

                                let fields_y = login_y + 60;
                                if cursor_x >= login_x + 30 && cursor_x <= login_x + 350 {
                                    if cursor_y >= fields_y + 30 && cursor_y <= fields_y + 60 {
                                        LOGIN_IS_PASSWORD = false;
                                        draw_login_window(&mut gfx);
                                        redraw = true;
                                    } else if cursor_y >= fields_y + 100 && cursor_y <= fields_y + 130 {
                                        LOGIN_IS_PASSWORD = true;
                                        draw_login_window(&mut gfx);
                                        redraw = true;
                                    }
                                }
                            }
                        } else {
                            if TUTORIAL_OPEN {
                                if packet.left && !left_prev {
                                    if handle_tutorial_click(&gfx, cursor_x, cursor_y) {
                                        redraw = true;
                                    }
                                }
                            }

                            if let Some(active) = wm.active_window {
                                if let Some(ref win) = wm.windows[active] {
                                    if win.title == "Paint" && win.state != WindowState::Minimized && !NAME_INPUT_OPEN {
                                        let wx = win.x + 2;
                                        let wy = win.y + 26;
                                        let canvas_x = wx + 5;
                                        let canvas_y = wy + 45;
                                        let canvas_w = (win.width - 10).min(400);
                                        let canvas_h = (win.height - 55).min(300);

                                        if cursor_x >= canvas_x && cursor_x < canvas_x + canvas_w &&
                                           cursor_y >= canvas_y && cursor_y < canvas_y + canvas_h {

                                            let canvas_cx = cursor_x - canvas_x;
                                            let canvas_cy = cursor_y - canvas_y;

                                            if packet.left {
                                                let size = PAINT_PEN_SIZE;
                                                let color = PAINT_COLOR;
                                                let tool = PAINT_TOOL;

                                                match tool {
                                                    0 => {
                                                        for dy in 0..size {
                                                            for dx in 0..size {
                                                                let x = canvas_cx + dx - size/2;
                                                                let y = canvas_cy + dy - size/2;
                                                                if x < canvas_w && y < canvas_h && x < 400 && y < 300 && x >= 0 && y >= 0 {
                                                                    PAINT_CANVAS[y][x] = color;
                                                                }
                                                            }
                                                        }
                                                        paint_update = true;
                                                    }
                                                    1 => {
                                                        if PAINT_IS_DRAWING {
                                                            draw_line_on_canvas(PAINT_PREV_X, PAINT_PREV_Y, canvas_cx, canvas_cy, color);
                                                        }
                                                        PAINT_PREV_X = canvas_cx;
                                                        PAINT_PREV_Y = canvas_cy;
                                                        PAINT_IS_DRAWING = true;
                                                        paint_update = true;
                                                    }
                                                    2 => {
                                                        if PAINT_IS_DRAWING {
                                                            for y in 0..300 {
                                                                for x in 0..400 {
                                                                    PAINT_CANVAS[y][x] = PAINT_CANVAS_BACKUP[y][x];
                                                                }
                                                            }
                                                            draw_rect_on_canvas(PAINT_RECT_START_X, PAINT_RECT_START_Y, canvas_cx, canvas_cy, color);
                                                        } else {
                                                            for y in 0..300 {
                                                                for x in 0..400 {
                                                                    PAINT_CANVAS_BACKUP[y][x] = PAINT_CANVAS[y][x];
                                                                }
                                                            }
                                                            PAINT_RECT_START_X = canvas_cx;
                                                            PAINT_RECT_START_Y = canvas_cy;
                                                            PAINT_IS_DRAWING = true;
                                                        }
                                                        paint_update = true;
                                                    }
                                                    _ => {}
                                                }
                                            } else {
                                                if PAINT_IS_DRAWING && PAINT_TOOL == 2 {
                                                    for y in 0..300 {
                                                        for x in 0..400 {
                                                            PAINT_CANVAS_BACKUP[y][x] = PAINT_CANVAS[y][x];
                                                        }
                                                    }
                                                    PAINT_IS_DRAWING = false;
                                                }
                                                if PAINT_TOOL == 1 {
                                                    PAINT_IS_DRAWING = false;
                                                }
                                            }
                                        } else {
                                            if PAINT_IS_DRAWING && PAINT_TOOL == 2 {
                                                for y in 0..300 {
                                                    for x in 0..400 {
                                                        PAINT_CANVAS[y][x] = PAINT_CANVAS_BACKUP[y][x];
                                                    }
                                                }
                                                PAINT_IS_DRAWING = false;
                                                paint_update = true;
                                            }
                                            if PAINT_TOOL == 1 {
                                                PAINT_IS_DRAWING = false;
                                            }
                                        }

                                        if packet.left && !left_prev {
                                            if cursor_x >= wx + 40 && cursor_x <= wx + 70 &&
                                               cursor_y >= wy + 5 && cursor_y <= wy + 35 {
                                                PAINT_TOOL = 0;
                                                redraw = true;
                                            }
                                            if cursor_x >= wx + 75 && cursor_x <= wx + 105 &&
                                               cursor_y >= wy + 5 && cursor_y <= wy + 35 {
                                                PAINT_TOOL = 1;
                                                PAINT_IS_DRAWING = false;
                                                redraw = true;
                                            }
                                            if cursor_x >= wx + 110 && cursor_x <= wx + 140 &&
                                               cursor_y >= wy + 5 && cursor_y <= wy + 35 {
                                                PAINT_TOOL = 2;
                                                PAINT_IS_DRAWING = false;
                                                for y in 0..300 {
                                                    for x in 0..400 {
                                                        PAINT_CANVAS_BACKUP[y][x] = PAINT_CANVAS[y][x];
                                                    }
                                                }
                                                redraw = true;
                                            }
                                            for i in 0..8 {
                                                let color_x = wx + 150 + i * 25;
                                                if cursor_x >= color_x && cursor_x <= color_x + 20 &&
                                                   cursor_y >= wy + 5 && cursor_y <= wy + 35 {
                                                    let colors = [0x000000, 0xFF0000, 0x00FF00, 0x0000FF,
                                                                  0xFFFF00, 0xFF00FF, 0x00FFFF, 0xFFFFFF];
                                                    PAINT_COLOR = colors[i];
                                                    redraw = true;
                                                }
                                            }
                                            for i in 0..3 {
                                                let size_x = wx + 350 + i * 30;
                                                if cursor_x >= size_x && cursor_x <= size_x + 25 &&
                                                   cursor_y >= wy + 5 && cursor_y <= wy + 35 {
                                                    PAINT_PEN_SIZE = i * 2 + 1;
                                                    redraw = true;
                                                }
                                            }
                                            let save_btn_x = wx + win.width - 4 - 85;
                                            if cursor_x >= save_btn_x && cursor_x <= save_btn_x + 80 &&
                                               cursor_y >= wy + 5 && cursor_y <= wy + 35 {
                                                if SAVED_PAINTINGS_COUNT < 5 {
                                                    for y in 0..300 {
                                                        for x in 0..400 {
                                                            SAVED_PAINTINGS[SAVED_PAINTINGS_COUNT][y][x] = PAINT_CANVAS[y][x];
                                                        }
                                                    }
                                                    SAVED_PAINTINGS_COUNT += 1;
                                                    redraw = true;
                                                }
                                                show_save_dialog(&mut gfx, paint_save_bmp);
                                                redraw = true;
                                            }
                                        }
                                    }
                                }
                            }

                            let modal = CONTEXT_MENU_OPEN || NAME_INPUT_OPEN || TUTORIAL_OPEN;
                            let over_menu = wm.start_menu.open && wm.start_menu.contains(cursor_x, cursor_y, &gfx);
                            let on_taskbar = !BARE_DESKTOP && cursor_y >= gfx.height - cui::TASKBAR_H;
                            let click_top = if packet.left && !left_prev && !modal && !over_menu && !on_taskbar {
                                wm.top_at(cursor_x, cursor_y)
                            } else { None };
                            let click_consumed = packet.left && !left_prev && (CONTEXT_MENU_OPEN || NAME_INPUT_OPEN);
                            if let Some(i) = click_top {
                                wm.raise(i);
                                raised = true;
                            }

                            if packet.left && !left_prev && click_top.is_some() && click_top == wm.active_window {
                                if cre_tool::handle_mouse(cursor_x, cursor_y, &mut wm) {
                                    redraw = true;
                                    if cre_tool::is_preview_running() {
                                        let preview_win = wm.create_window("Preview", 150, 120, 400, 300);
                                        cre_tool::set_preview_window(preview_win);
                                        redraw = true;
                                    }
                                }
                            }

                            if NP_DRAG {
                                if packet.left {
                                    for i in 0..wm.window_count {
                                        if let Some(ref win) = wm.windows[i] {
                                            if win.title == "Notepad" && win.state != WindowState::Minimized {
                                                let pos = np_hit_test(cursor_x, cursor_y, win.x + 2, win.y + 26);
                                                if pos != NP_CUR {
                                                    NP_CUR = pos;
                                                    redraw = true;
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    NP_DRAG = false;
                                }
                            }

                            if paint_update {
                                redraw = true;
                            }

                            if packet.right && !right_prev && !TUTORIAL_OPEN {
                                if CONTEXT_MENU_OPEN {
                                    CONTEXT_MENU_OPEN = false;
                                    redraw = true;
                                } else {
                                    if !NAME_INPUT_OPEN && FILE_MANAGER_MOUNTED && !FILE_MANAGER_DIALOG_OPEN {
                                        if let Some(ti) = wm.top_at(cursor_x, cursor_y) {
                                            if let Some(ref win) = wm.windows[ti] {
                                                if win.title == "File Manager" && win.state != WindowState::Minimized {
                                                    let fcx = win.x + 2;
                                                    let fcy = win.y + 26;
                                                    let fw = win.width - 4;
                                                    let fh = win.height - 28;
                                                    if cursor_x >= fcx && cursor_x < fcx + fw &&
                                                       cursor_y >= fcy + 75 && cursor_y < fcy + fh - 40 {
                                                        let top = fcy + 90;
                                                        let on_row = cursor_y >= top && {
                                                            let r = (cursor_y - top) / 18;
                                                            r < fm_visible_rows(fh) && FM_VIEW.scroll + r < FM_VIEW.total_items()
                                                        };
                                                        if !on_row {
                                                            CONTEXT_MENU_KIND = 1;
                                                            CONTEXT_MENU_X = cursor_x;
                                                            CONTEXT_MENU_Y = cursor_y;
                                                            CONTEXT_MENU_OPEN = true;
                                                            redraw = true;
                                                        } else {
                                                            let r = (cursor_y - top) / 18;
                                                            if let Some(ViewItem::Entry(e)) = FM_VIEW.item(FM_VIEW.scroll + r) {
                                                                if !FM_VIEW.entries[e].is_dir || fm_is_sys_folder(&FM_VIEW.entries[e]) {
                                                                    let en = FM_VIEW.entries[e];
                                                                    FM_CTX_NAME[..en.name_len].copy_from_slice(&en.name[..en.name_len]);
                                                                    FM_CTX_LEN = en.name_len;
                                                                    CONTEXT_MENU_KIND = 2;
                                                                    CONTEXT_MENU_X = cursor_x;
                                                                    CONTEXT_MENU_Y = cursor_y;
                                                                    CONTEXT_MENU_OPEN = true;
                                                                    redraw = true;
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    let mut clicked_on_window = false;
                                    for i in 0..wm.window_count {
                                        if let Some(ref win) = wm.windows[i] {
                                            if win.contains(cursor_x, cursor_y) && win.state != WindowState::Minimized {
                                                clicked_on_window = true;
                                                break;
                                            }
                                        }
                                    }
                                    let taskbar_y = gfx.height - 30;
                                    if !BARE_DESKTOP && !clicked_on_window && !(cursor_y >= taskbar_y) {
                                        CONTEXT_MENU_KIND = 0;
                                        for i in 0..syscfg::shortcut_count() {
                                            let (sx, sy) = shortcut_pos(i);
                                            if sy + 64 < taskbar_y &&
                                               cursor_x >= sx && cursor_x <= sx + 64 &&
                                               cursor_y >= sy && cursor_y <= sy + 64 + 20 {
                                                SC_CTX_IDX = i;
                                                CONTEXT_MENU_KIND = 3;
                                                break;
                                            }
                                        }
                                        CONTEXT_MENU_X = cursor_x;
                                        CONTEXT_MENU_Y = cursor_y;
                                        CONTEXT_MENU_OPEN = true;
                                        redraw = true;
                                    }
                                }
                            }

                            if packet.left && !left_prev && !TUTORIAL_OPEN {
                                if CONTEXT_MENU_OPEN {
                                    let menu_x = CONTEXT_MENU_X;
                                    let menu_y = CONTEXT_MENU_Y;
                                    let menu_w = 160;
                                    let menu_h = context_menu_h();

                                    if cursor_x >= menu_x && cursor_x <= menu_x + menu_w &&
                                       cursor_y >= menu_y && cursor_y <= menu_y + menu_h {
                                        let item_index = cursor_y.saturating_sub(menu_y + 4) / 24;
                                        if CONTEXT_MENU_KIND == 3 {
                                            if item_index == 0 {
                                                let _ = syscfg::remove_shortcut_at(SC_CTX_IDX);
                                                redraw = true;
                                            }
                                        } else if CONTEXT_MENU_KIND == 2 {
                                            match item_index {
                                                0 => {
                                                    if !fm_try_protect(&mut wm) {
                                                        fm_delete_selected();
                                                    }
                                                    redraw = true;
                                                }
                                                1 => {
                                                    fm_make_shortcut();
                                                    redraw = true;
                                                }
                                                _ => {}
                                            }
                                        } else if CONTEXT_MENU_KIND == 1 {
                                            match item_index {
                                                0 => {
                                                    show_name_input_dialog(&mut gfx, fm_create_folder, 0, 0);
                                                    redraw = true;
                                                }
                                                1 => {
                                                    if wm.find_window("Terminal").is_none() { term_len = 0; TERM_LEN_GLOBAL = 0; clear_history(); }
                                                    wm.create_window("Terminal", 100, 100, 500, 350);
                                                    redraw = true;
                                                }
                                                _ => {}
                                            }
                                        } else {
                                        match item_index {
                                            0 => {
                                                if wm.find_window("Terminal").is_none() { term_len = 0; TERM_LEN_GLOBAL = 0; clear_history(); }
                                                wm.create_window("Terminal", 100, 100, 500, 350);
                                                redraw = true;
                                            }
                                            1 => {
                                                wm.create_window("AboutOS", 200, 100, 400, 350);
                                                redraw = true;
                                            }
                                            2 => {
                                                wm.create_window("Settings", 200, 180, 450, 450);
                                                redraw = true;
                                            }
                                            3 => {
                                                show_name_input_dialog(&mut gfx, create_folder_on_desktop, 0, 0);
                                                redraw = true;
                                            }
                                            _ => {}
                                        }
                                        }
                                    }
                                    CONTEXT_MENU_OPEN = false;
                                    redraw = true;
                                }

                                let name_input_handled = handle_name_input_click(cursor_x, cursor_y);
                                if name_input_handled {
                                    redraw = true;
                                }

                                let mut html_handled = click_consumed;
                                if let Some(i) = click_top {
                                    if let Some(ref win) = wm.windows[i] {
                                        let title = win.title;
                                        if (title == "Browser" || title == "HTML App" || title == netwin::TITLE) && cursor_y >= win.y + 30 {
                                            wm.active_window = Some(i);
                                            if title == "Browser" { web::app::click(cursor_x, cursor_y); } else if title == "HTML App" { web::app::app_click(cursor_x, cursor_y); } else { netwin::click(cursor_x, cursor_y); }
                                            if let Some(app_name) = web::app::take_launch() {
                                                match app_name.as_str() {
                                                    "terminal" => { if wm.find_window("Terminal").is_none() { term_len = 0; TERM_LEN_GLOBAL = 0; clear_history(); } wm.create_window("Terminal", 100, 100, 500, 350); }
                                                    "files" | "filemanager" => { wm.create_window("File Manager", 150, 120, 500, 350); }
                                                    "settings" => { wm.create_window("Settings", 200, 180, 450, 450); }
                                                    "notepad" => { wm.create_window("Notepad", 100, 100, 500, 400); }
                                                    "paint" => { wm.create_window("Paint", 100, 100, 420, 400); }
                                                    "about" => { wm.create_window("AboutOS", 200, 100, 400, 350); }
                                                    "media" => { MEDIA_WINDOW_REQUESTED = true; }
                                                    "cretool" | "cre" => { CRE_TOOL_WINDOW_REQUESTED = true; }
                                                    "clock" => { CLOCK_WINDOW_REQUESTED = true; }
                                                    "calendar" => { calend::CALENDAR_WINDOW_REQUESTED = true; }
                                                    "taskmgr" | "taskmanager" => { TASK_WINDOW_REQUESTED = true; }
                                                    "support" => { SUPPORT_WINDOW_REQUESTED = true; }
                                                    "pcinfo" => { PCINF_WINDOW_REQUESTED = true; }
                                                    _ => {}
                                                }
                                            }
                                            html_handled = true;
                                            redraw = true;
                                        }
                                    }
                                }
                                if !html_handled {
                                    if let Some(i) = click_top {
                                        let mut np_hit = 0u8;
                                        let mut np_pos = 0usize;
                                        if let Some(ref win) = wm.windows[i] {
                                            if win.title == "Notepad" && win.state != WindowState::Minimized {
                                                let ncx = win.x + 2;
                                                let ncy = win.y + 26;
                                                if cursor_y >= ncy + 4 && cursor_y <= ncy + 24 {
                                                    if cursor_x >= ncx + 10 && cursor_x <= ncx + 70 {
                                                        np_hit = 1;
                                                    } else if cursor_x >= ncx + 80 && cursor_x <= ncx + 140 {
                                                        np_hit = 2;
                                                    }
                                                } else if cursor_y >= ncy + 32 && cursor_y + 4 < win.y + win.height &&
                                                          cursor_x >= ncx + 4 && cursor_x + 8 < ncx + win.width {
                                                    np_hit = 3;
                                                    np_pos = np_hit_test(cursor_x, cursor_y, ncx, ncy);
                                                }
                                            }
                                        }
                                        if np_hit == 3 {
                                            NP_CUR = np_pos;
                                            NP_ANCHOR = np_pos;
                                            NP_DRAG = true;
                                            np_fix();
                                            html_handled = true;
                                            redraw = true;
                                        }
                                        if np_hit == 1 {
                                            show_save_dialog(&mut gfx, notepad_save_fat32);
                                            html_handled = true;
                                            redraw = true;
                                        } else if np_hit == 2 {
                                            show_name_input_dialog(&mut gfx, notepad_load_fat32, 2, 0);
                                            html_handled = true;
                                            redraw = true;
                                        }
                                    }
                                }
                                if !html_handled {
                                    if let Some(i) = click_top {
                                        let mut ct_hit = false;
                                        if let Some(ref win) = wm.windows[i] {
                                            if win.title == "Cre Tool" && win.state != WindowState::Minimized {
                                                let wx = win.x + 2;
                                                let wy = win.y + 26;
                                                let ww = win.width - 4;
                                                let sv_x = wx + ww - 180;
                                                if cursor_x >= sv_x && cursor_x <= sv_x + 80 &&
                                                   cursor_y >= wy + 4 && cursor_y <= wy + 26 {
                                                    ct_hit = true;
                                                }
                                            }
                                        }
                                        if ct_hit {
                                            show_save_dialog(&mut gfx, cretool_save_fat32);
                                            html_handled = true;
                                            redraw = true;
                                        }
                                    }
                                }
                                if !html_handled {
                                for i in 0..wm.window_count {
                                    if click_top != Some(i) { continue; }
                                    if let Some(ref win) = wm.windows[i] {
                                        if win.title == "HTML Viewer" && win.state != WindowState::Minimized {
                                            let wx = win.x + 2;
                                            let wy = win.y + 26;
                                            let ww = win.width - 4;
                                            let wh = win.height - 28;
                                            if cursor_x >= wx && cursor_x < wx + ww &&
                                               cursor_y >= wy && cursor_y < wy + wh {
                                                if let Some(ref mut app) = HTML_APP_INSTANCE {
                                                    if html::handle_html_click(app, wx, wy, ww, wh, cursor_x, cursor_y) {
                                                        redraw = true;
                                                        html_handled = true;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                }

                                let mut support_handled = false;
                                if !html_handled {
                                    for i in 0..wm.window_count {
                                        if click_top != Some(i) { continue; }
                                        if let Some(ref win) = wm.windows[i] {
                                            if win.title == "Support Center" && win.state != WindowState::Minimized {
                                                let wx = win.x + 2;
                                                let wy = win.y + 26;
                                                let ww = win.width - 4;
                                                let wh = win.height - 28;
                                                if cursor_x >= wx && cursor_x < wx + ww &&
                                                   cursor_y >= wy && cursor_y < wy + wh {
                                                    if let Some(ref mut app) = SUPPORT_APP_INSTANCE {
                                                        if support::handle_support_click(app, wx, wy, ww, wh, cursor_x, cursor_y) {
                                                            redraw = true;
                                                            support_handled = true;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                let mut pcinf_handled = false;
                                if !html_handled && !support_handled {
                                    for i in 0..wm.window_count {
                                        if click_top != Some(i) { continue; }
                                        if let Some(ref win) = wm.windows[i] {
                                            if win.title == "PC Info" && win.state != WindowState::Minimized {
                                                let wx = win.x + 2;
                                                let wy = win.y + 26;
                                                let ww = win.width - 4;
                                                let wh = win.height - 28;
                                                if cursor_x >= wx && cursor_x < wx + ww &&
                                                   cursor_y >= wy && cursor_y < wy + wh {
                                                    if let Some(ref mut app) = PCINF_APP_INSTANCE {
                                                        if pcinf::handle_pcinf_click(app, wx, wy, ww, wh, cursor_x, cursor_y) {
                                                            redraw = true;
                                                            pcinf_handled = true;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                let mut media_handled = false;
                                if html_handled || support_handled || pcinf_handled {
                                } else {
                                    for i in 0..wm.window_count {
                                        if click_top != Some(i) { continue; }
                                        if let Some(ref win) = wm.windows[i] {
                                            if win.title == "Media" && win.state != WindowState::Minimized {
                                                let wx = win.x + 2;
                                                let wy = win.y + 26;
                                                let ww = win.width - 4;
                                                let wh = win.height - 28;

                                                if cursor_x >= wx && cursor_x < wx + ww &&
                                                   cursor_y >= wy && cursor_y < wy + wh {

                                                    if cursor_x >= wx + ww - 100 && cursor_x <= wx + ww - 10 &&
                                                       cursor_y >= wy + 5 && cursor_y <= wy + 29 {
                                                        MEDIA_BROWSER_OPEN = !MEDIA_BROWSER_OPEN;
                                                        media_handled = true;
                                                        redraw = true;
                                                        break;
                                                    }

                                                    if MEDIA_BROWSER_OPEN {
                                                        let list_x = wx + 20;
                                                        let list_y = wy + 60;
                                                        let list_w = ww - 44;
                                                        let item_h = 30;
                                                        let visible_count = ((wh - 100) / item_h).min(MEDIA_BMP_COUNT);
                                                        let mut picked = false;
                                                        for k in 0..visible_count {
                                                            let iy = list_y + 5 + k * item_h;
                                                            if cursor_x >= list_x + 5 && cursor_x <= list_x + list_w - 5 &&
                                                               cursor_y >= iy && cursor_y <= iy + item_h - 3 {
                                                                media_open_image(k);
                                                                picked = true;
                                                                break;
                                                            }
                                                        }
                                                        if picked {
                                                            media_handled = true;
                                                            redraw = true;
                                                            break;
                                                        }
                                                    }

                                                    if MEDIA_LOADED_IMAGE.is_some() && !MEDIA_BROWSER_OPEN {
                                                        let info_y = wy + wh - 55;
                                                        if cursor_x >= wx + ww - 4 - 110 && cursor_x <= wx + ww - 4 - 10 &&
                                                           cursor_y >= info_y + 2 && cursor_y <= info_y + 28 {
                                                            MEDIA_LOADED_IMAGE = None;
                                                            MEDIA_NEEDS_REDRAW = true;
                                                            media_handled = true;
                                                            redraw = true;
                                                            break;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                let mut file_manager_handled = false;
                                if html_handled || support_handled || media_handled {
                                } else {
                                    for i in 0..wm.window_count {
                                        if click_top != Some(i) { continue; }
                                        if let Some(ref win) = wm.windows[i] {
                                            if win.title == "File Manager" && win.state != WindowState::Minimized {
                                                let cx = win.x + 2;
                                                let cy = win.y + 26;
                                                let win_w = win.width - 4;
                                                let win_h = win.height - 28;

                                                if cursor_x >= cx && cursor_x < cx + win_w &&
                                                   cursor_y >= cy && cursor_y < cy + win_h {

                                                    if !FILE_MANAGER_DIALOG_OPEN {
                                                        if fm_hit(cursor_x, cursor_y, cx + FM_BTN1_X, cy + FM_BTN_Y, FM_BTN_W, FM_BTN_H) {
                                                            FM_PATH_EDIT = false;
                                                            FM_PATH_ERR = "";
                                                            if wm.find_window("Terminal").is_none() { term_len = 0; TERM_LEN_GLOBAL = 0; clear_history(); }
                                                            wm.create_window("Terminal", 100, 100, 500, 350);
                                                            redraw = true;
                                                            file_manager_handled = true;
                                                        } else if fm_hit(cursor_x, cursor_y, cx + FM_BTN2_X, cy + FM_BTN_Y, FM_BTN_W, FM_BTN_H) {
                                                            FM_PATH_EDIT = false;
                                                            FM_PATH_ERR = "";
                                                            wm.create_window("About FileMGR", 220, 160, 320, 200);
                                                            redraw = true;
                                                            file_manager_handled = true;
                                                        } else if fm_hit(cursor_x, cursor_y, cx + 10, cy + FM_PATH_Y, win_w.saturating_sub(20), FM_PATH_H) {
                                                            if !FM_PATH_EDIT { fm_path_begin_edit(); }
                                                            redraw = true;
                                                            file_manager_handled = true;
                                                        } else if FM_PATH_EDIT {
                                                            FM_PATH_EDIT = false;
                                                            FM_PATH_ERR = "";
                                                            redraw = true;
                                                        }
                                                    }

                                                    if !FILE_MANAGER_DIALOG_OPEN && !FILE_MANAGER_MOUNTED && !file_manager_handled {
                                                        let (cols, rows) = fm_tile_layout(win_w, win_h);
                                                        for i in 0..DISK_COUNT {
                                                            if i >= cols * rows { break; }
                                                            let present = match ata::get_disk(i) {
                                                                Some(d) => d.present,
                                                                None => false,
                                                            };
                                                            if !present { continue; }
                                                            let (dx, dy) = fm_tile_pos(i, cols);
                                                            if fm_hit(cursor_x, cursor_y, cx + dx, cy + dy, FM_TILE_W, FM_TILE_H) {
                                                                FILE_MANAGER_DIALOG_OPEN = true;
                                                                FILE_MANAGER_DIALOG_DISK = i;
                                                                file_manager_handled = true;
                                                                redraw = true;
                                                                break;
                                                            }
                                                        }
                                                    }

                                                    if FILE_MANAGER_DIALOG_OPEN && !file_manager_handled {
                                                        let dialog_w = 300;
                                                        let dialog_h = 130;
                                                        let dialog_x = cx + (win_w - dialog_w) / 2;
                                                        let dialog_y = cy + (win_h - dialog_h) / 2;

                                                        if cursor_x >= dialog_x + 40 && cursor_x <= dialog_x + 120 &&
                                                           cursor_y >= dialog_y + 75 && cursor_y <= dialog_y + 103 {
                                                            let disk = FILE_MANAGER_DIALOG_DISK;
                                                            if mount_fat32_disk(disk) {
                                                                FM_VIEW.open_root(disk);
                                                                FM_MSG_LEN = 0;
                                                                FILE_MANAGER_MOUNTED = true;
                                                                FILE_MANAGER_DIALOG_OPEN = false;
                                                                redraw = true;
                                                                file_manager_handled = true;
                                                            } else {
                                                                FILE_MANAGER_DIALOG_OPEN = false;
                                                                redraw = true;
                                                                file_manager_handled = true;
                                                            }
                                                        }

                                                        if cursor_x >= dialog_x + 170 && cursor_x <= dialog_x + 250 &&
                                                           cursor_y >= dialog_y + 75 && cursor_y <= dialog_y + 103 {
                                                            FILE_MANAGER_DIALOG_OPEN = false;
                                                            redraw = true;
                                                            file_manager_handled = true;
                                                        }
                                                    }

                                                    if FILE_MANAGER_MOUNTED {
                                                        if !file_manager_handled && cursor_x >= cx + 10 && cursor_x < cx + win_w - 10 &&
                                                           cursor_y >= cy + 90 {
                                                            let r = (cursor_y - (cy + 90)) / 18;
                                                            if r < fm_visible_rows(win_h) {
                                                                let k = FM_VIEW.scroll + r;
                                                                match FM_VIEW.item(k) {
                                                                    Some(ViewItem::Up) => {
                                                                        FM_VIEW.up();
                                                                        redraw = true;
                                                                        file_manager_handled = true;
                                                                    }
                                                                    Some(ViewItem::Entry(e)) => {
                                                                        if FM_VIEW.entries[e].is_dir {
                                                                            FM_VIEW.enter(e);
                                                                        } else {
                                                                            let en = FM_VIEW.entries[e];
                                                                            let fname = core::str::from_utf8(&en.name[..en.name_len]).unwrap_or("");
                                                                            fm_open_file(&mut wm, FM_VIEW.disk, fname);
                                                                        }
                                                                        redraw = true;
                                                                        file_manager_handled = true;
                                                                    }
                                                                    None => {}
                                                                }
                                                            }
                                                        }
                                                        if cursor_x >= cx + 10 && cursor_x <= cx + 90 &&
                                                           cursor_y >= cy + win_h - 35 && cursor_y <= cy + win_h - 10 {
                                                            FILE_MANAGER_MOUNTED = false;
                                                            redraw = true;
                                                            file_manager_handled = true;
                                                        }
                                                        if cursor_x >= cx + 100 && cursor_x <= cx + 180 &&
                                                           cursor_y >= cy + win_h - 35 && cursor_y <= cy + win_h - 10 {
                                                            FM_VIEW.up();
                                                            redraw = true;
                                                            file_manager_handled = true;
                                                        }
                                                    }
                                                }
                                                break;
                                            }
                                        }
                                    }
                                }

                                if file_manager_handled {
                                } else if media_handled {
                                } else if html_handled {
                                } else if support_handled {
                                } else if pcinf_handled {
                                } else {
                                    let mut taskbar_handled = false;

                                    if !(wm.start_menu.open && wm.start_menu.contains(cursor_x, cursor_y, &gfx)) {
                                        let (hh, mm, ss) = get_time();
                                        let clock_len = time_to_str(hh, mm, ss).len();
                                        let hit = if BARE_DESKTOP { cui::TB_NONE } else { wm.taskbar_hit(cursor_x, cursor_y, clock_len) };
                                        if hit != cui::TB_NONE {
                                            taskbar_handled = true;
                                            redraw = true;
                                            if hit == cui::TB_START {
                                                wm.start_menu.open = !wm.start_menu.open;
                                            } else {
                                                wm.start_menu.open = false;
                                                if hit == cui::TB_CLOCK {
                                                    CLOCK_WINDOW_REQUESTED = true;
                                                } else if hit == cui::TB_LANG {
                                                    web::keys::toggle_layout();
                                                } else if hit >= 0 {
                                                    wm.taskbar_click(hit as usize);
                                                }
                                            }
                                        }
                                    }

                                    if !redraw && !name_input_handled && !taskbar_handled {
                                        let icon_y = 50;
                                        let icon_size = 64;
                                        let taskbar_y = gfx.height - 30;
                                        let mut folder_clicked = false;

                                        unsafe {
                                            let mut row = 0;
                                            let folders_per_row = 6;

                                            for i in 0..DESKTOP_FOLDERS_COUNT {
                                                let folder_name = core::str::from_utf8(&DESKTOP_FOLDERS[i]).unwrap_or("");
                                                let folder_name_trimmed = folder_name.trim_end_matches('\0');

                                                if folder_name_trimmed.len() > 0 {
                                                    let (current_x, current_y) = folder_pos_or(i, 50 + (i % folders_per_row) * 100, icon_y + icon_size + 40 + (row * (icon_size + 30)));

                                                    if current_y + icon_size < taskbar_y && click_top.is_none() {
                                                        if cursor_x >= current_x && cursor_x <= current_x + icon_size &&
                                                           cursor_y >= current_y && cursor_y <= current_y + icon_size + 20 {
                                                            icon_drag_begin(100 + i, current_x, current_y, cursor_x, cursor_y);
                                                            folder_clicked = true;
                                                            redraw = true;
                                                            break;
                                                        }
                                                    }

                                                    if (i + 1) % folders_per_row == 0 {
                                                        row += 1;
                                                    }
                                                }
                                            }
                                        }

                                        if !folder_clicked && click_top.is_none() {
                                            for i in 0..syscfg::shortcut_count() {
                                                let (sx, sy) = shortcut_pos(i);
                                                if sy + icon_size < taskbar_y &&
                                                   cursor_x >= sx && cursor_x <= sx + icon_size &&
                                                   cursor_y >= sy && cursor_y <= sy + icon_size + 20 {
                                                    icon_drag_begin(200 + i, sx, sy, cursor_x, cursor_y);
                                                    folder_clicked = true;
                                                    redraw = true;
                                                    break;
                                                }
                                            }
                                        }

                                        if !folder_clicked {
                                            let mut tool_clicked = false;
                                            if false {
                                                CRE_TOOL_WINDOW_REQUESTED = true;
                                                tool_clicked = true;
                                                redraw = true;
                                                push_history("Cre Tool opened!");
                                            }

                                            if !tool_clicked {
                                                let mut folder_window_click = false;
                                                for i in 0..wm.window_count {
                                                    if click_top != Some(i) { continue; }
                                                    if let Some(ref win) = wm.windows[i] {
                                                        let mut is_folder = false;
                                                        let mut current_folder_idx = 0;
                                                        unsafe {
                                                            for j in 0..DESKTOP_FOLDERS_COUNT {
                                                                let folder_name = core::str::from_utf8(&DESKTOP_FOLDERS[j]).unwrap_or("");
                                                                if folder_name.trim_end_matches('\0') == win.title {
                                                                    is_folder = true;
                                                                    current_folder_idx = j;
                                                                    break;
                                                                }
                                                            }
                                                        }

                                                        if is_folder && win.state != WindowState::Minimized {
                                                            let wx = win.x + 2;
                                                            let wy = win.y + 26;

                                                            if cursor_x >= wx + 10 && cursor_x <= wx + 160 &&
                                                               cursor_y >= wy + 40 && cursor_y <= wy + 64 {
                                                                show_name_input_dialog(&mut gfx, save_file_to_folder, 1, current_folder_idx);
                                                                folder_window_click = true;
                                                                redraw = true;
                                                                break;
                                                            }

                                                            unsafe {
                                                                let file_count = FOLDER_FILES_COUNT[current_folder_idx];
                                                                let icon_sz = 32;
                                                                let start_y = wy + 80;
                                                                let mut x_pos = wx + 10;
                                                                let mut y_pos = start_y;
                                                                let items_per_row = 8;

                                                                for j in 0..file_count {
                                                                    if cursor_x >= x_pos && cursor_x <= x_pos + icon_sz &&
                                                                       cursor_y >= y_pos && cursor_y <= y_pos + icon_sz + 20 {
                                                                        open_file_from_folder(&mut wm, current_folder_idx, j);
                                                                        folder_window_click = true;
                                                                        redraw = true;
                                                                        break;
                                                                    }

                                                                    x_pos += icon_sz + 10;
                                                                    if (j + 1) % items_per_row == 0 {
                                                                        x_pos = wx + 10;
                                                                        y_pos += icon_sz + 20;
                                                                    }
                                                                }
                                                            }
                                                            break;
                                                        }
                                                    }
                                                }

                                                if !folder_window_click {
                                                    if wm.start_menu.open {
                                                        if let Some(action) = wm.start_menu.handle_click(&gfx, cursor_x, cursor_y) {
                                                            match action {
                                                                "Terminal" => { if wm.find_window("Terminal").is_none() { term_len = 0; TERM_LEN_GLOBAL = 0; clear_history(); } wm.create_window("Terminal", 100, 100, 500, 350); }
                                                                "Files" => { wm.create_window("File Manager", 150, 120, 500, 350); }
                                                                "Settings" => { wm.create_window("Settings", 200, 180, 450, 450); }
                                                                "Notepad" => { wm.create_window("Notepad", 100, 100, 500, 400); }
                                                                "Paint" => { wm.create_window("Paint", 100, 100, 420, 400); }
                                                                "Calculator" => { open_system_app(&mut wm, "calc.mp"); }
                                                                "Media" => { MEDIA_WINDOW_REQUESTED = true; }
                                                                "Cre Tool" => {
                                                                    CRE_TOOL_WINDOW_REQUESTED = true;
                                                                    redraw = true;
                                                                }
                                                                "Reboot" => {
                                                                    WALLPAPER = Color::rgb(89, 0, 255);
                                                                    THEME_IDX = 0;
                                                                    TERM_HIST_LEN = 0;
                                                                    NOTEPAD_LEN = 0;
                                                                    WALLPAPER_SELECTOR_OPEN = false;
                                                                    THEME_SELECTOR_OPEN = false;
                                                                    DOCUMENT_MANAGER_OPEN = false;
                                                                    DOCUMENT_MANAGER_MODE = 0;
                                                                    SAVED_DOCUMENTS_COUNT = 0;
                                                                    for i in 0..5 { SAVED_DOCUMENTS_LEN[i] = 0; }
                                                                    SAVED_PAINTINGS_COUNT = 0;
                                                                    GRADIENT_TOP_ACTIVE = Color::rgb(90, 90, 90);
                                                                    GRADIENT_BOTTOM_ACTIVE = Color::rgb(20, 20, 20);
                                                                    GRADIENT_TOP_INACTIVE = Color::rgb(105, 105, 105);
                                                                    GRADIENT_BOTTOM_INACTIVE = Color::rgb(50, 50, 50);
                                                                    USE_BMP_WALLPAPER = true;
                                                                    CURRENT_BMP_INDEX = 0;
                                                                    LOGGED_IN = false;
                                                                    DESKTOP_FOLDERS_COUNT = 0;
                                                                    for i in 0..10 {
                                                                        FOLDER_FILES_COUNT[i] = 0;
                                                                    }
                                                                    PAINT_IS_DRAWING = false;
                                                                    MEDIA_LOADED_IMAGE = None;
                                                                    MEDIA_BROWSER_OPEN = false;
                                                                    users::logout();
                                                                    syscfg::apply();
                                                                    draw_login_window(&mut gfx);
                                                                    redraw = true;
                                                                }
                                                                "Shutdown" => {
                                                                    shutdown_sequence(&mut gfx);
                                                                    gfx.clear(Color::BLACK.to_u32());
                                                                    gfx.draw_text(
                                                                        (gfx.width - 16 * 17) / 2,
                                                                        gfx.height / 2 - 8,
                                                                        "Now you can turn off the power",
                                                                        Color::WHITE.to_u32(),
                                                                        Color::BLACK.to_u32()
                                                                    );
                                                                    gfx.flush();
                                                                    loop { core::arch::asm!("hlt") }
                                                                }
                                                                _ => {}
                                                            }
                                                            wm.start_menu.open = false;
                                                            redraw = true;
                                                        } else if !wm.start_menu.contains(cursor_x, cursor_y, &gfx) {
                                                            wm.start_menu.open = false;
                                                            redraw = true;
                                                        }
                                                    } else if wm.handle_mouse_press(&mut gfx, cursor_x, cursor_y) {
                                                        redraw = true;
                                                    } else {
                                                        let mut icon_clicked = false;
                                                        let icon_y = 50;
                                                        let icon_size = 64;
                                                        if click_top.is_none() {
                                                            for n in 0..9 {
                                                                let (ix, iy) = sys_icon_pos(n);
                                                                if cursor_x >= ix && cursor_x <= ix + icon_size &&
                                                                   cursor_y >= iy && cursor_y <= iy + icon_size {
                                                                    icon_drag_begin(n, ix, iy, cursor_x, cursor_y);
                                                                    icon_clicked = true;
                                                                    redraw = true;
                                                                    break;
                                                                }
                                                            }
                                                        }

                                                        if !icon_clicked {
                                                            let mut dwp_handled = false;
                                                            for i in 0..wm.window_count {
                                                                if click_top != Some(i) { continue; }
                                                                if let Some(ref win) = wm.windows[i] {
                                                                    if win.title == "Disk Wallpaper Selector" && win.state != WindowState::Minimized {
                                                                        let wx = win.x + 2;
                                                                        let wy = win.y + 26;
                                                                        let mut applied = false;
                                                                        for r in 0..DWP_ROWS {
                                                                            let idx = DWP_PAGE * DWP_ROWS + r;
                                                                            if idx >= DWP_COUNT { break; }
                                                                            let y_pos = wy + 30 + r * 26;
                                                                            if cursor_x >= wx + 10 && cursor_x <= wx + 260 &&
                                                                               cursor_y >= y_pos && cursor_y <= y_pos + 22 {
                                                                                applied = disk_wp_apply(idx);
                                                                                dwp_handled = true;
                                                                                break;
                                                                            }
                                                                        }
                                                                        if !dwp_handled {
                                                                            let by = wy + 30 + DWP_ROWS * 26 + 4;
                                                                            if cursor_y >= by && cursor_y <= by + 22 {
                                                                                if cursor_x >= wx + 10 && cursor_x <= wx + 70 {
                                                                                    if DWP_PAGE > 0 { DWP_PAGE -= 1; }
                                                                                    dwp_handled = true;
                                                                                } else if cursor_x >= wx + 200 && cursor_x <= wx + 260 {
                                                                                    if (DWP_PAGE + 1) * DWP_ROWS < DWP_COUNT { DWP_PAGE += 1; }
                                                                                    dwp_handled = true;
                                                                                }
                                                                            }
                                                                        }
                                                                        if applied {
                                                                            wm.close_window(i);
                                                                        }
                                                                        break;
                                                                    }
                                                                }
                                                            }

                                                            if dwp_handled {
                                                                redraw = true;
                                                                continue;
                                                            }

                                                            let mut bmp_selected = false;
                                                            for i in 0..wm.window_count {
                                                                if click_top != Some(i) { continue; }
                                                                if let Some(ref win) = wm.windows[i] {
                                                                    if win.title == "BMP Wallpaper Selector" && win.state != WindowState::Minimized {
                                                                        let wx = win.x + 2;
                                                                        let wy = win.y + 26;

                                                                        for j in 0..3 {
                                                                            let y_pos = wy + 30 + j * 60;
                                                                            if cursor_x >= wx + 10 && cursor_x <= wx + 260 &&
                                                                               cursor_y >= y_pos && cursor_y <= y_pos + 50 {
                                                                                CURRENT_BMP_INDEX = j;
                                                                                USE_BMP_WALLPAPER = true;
                                                                                bmp_selected = true;
                                                                                wm.close_window(i);
                                                                                break;
                                                                            }
                                                                        }
                                                                        if !bmp_selected {
                                                                            let y4 = wy + 30 + 3 * 60;
                                                                            if cursor_x >= wx + 10 && cursor_x <= wx + 260 &&
                                                                               cursor_y >= y4 && cursor_y <= y4 + 50 {
                                                                                disk_wp_scan();
                                                                                wm.close_window(i);
                                                                                wm.create_window("Disk Wallpaper Selector", 200, 120, 300, 320);
                                                                                bmp_selected = true;
                                                                            }
                                                                        }
                                                                        if bmp_selected { break; }
                                                                    }
                                                                }
                                                            }

                                                            if bmp_selected {
                                                                redraw = true;
                                                                continue;
                                                            }

                                                            let mut live_wp_interacted = false;
                                                            for i in 0..wm.window_count {
                                                                if click_top != Some(i) { continue; }
                                                                if let Some(ref win) = wm.windows[i] {
                                                                    if win.title == "Live Wallpaper Settings" && win.state != WindowState::Minimized {
                                                                        let wx = win.x + 2;
                                                                        let wy = win.y + 26;

                                                                        let row1_y = wy + 40;
                                                                        let toggle_x = wx + win.width - 4 - 58;
                                                                        if cursor_x >= toggle_x && cursor_x <= toggle_x + 36 &&
                                                                           cursor_y >= row1_y && cursor_y <= row1_y + 22 {
                                                                            LIVE_WALLPAPER_ENABLED = !LIVE_WALLPAPER_ENABLED;
                                                                            LIVE_WALLPAPER_ELAPSED_SEC = 0;
                                                                            live_wp_interacted = true;
                                                                        }

                                                                        let row2_y = row1_y + 40;
                                                                        let minus_x = wx + 110;
                                                                        let val_x = minus_x + 34;
                                                                        let plus_x = val_x + 58;

                                                                        if !live_wp_interacted && cursor_x >= minus_x && cursor_x <= minus_x + 26 &&
                                                                           cursor_y >= row2_y && cursor_y <= row2_y + 26 {
                                                                            if LIVE_WALLPAPER_INTERVAL_SEC > LIVE_WALLPAPER_MIN_SEC {
                                                                                LIVE_WALLPAPER_INTERVAL_SEC -= 1;
                                                                            }
                                                                            live_wp_interacted = true;
                                                                        }

                                                                        if !live_wp_interacted && cursor_x >= plus_x && cursor_x <= plus_x + 26 &&
                                                                           cursor_y >= row2_y && cursor_y <= row2_y + 26 {
                                                                            if LIVE_WALLPAPER_INTERVAL_SEC < LIVE_WALLPAPER_MAX_SEC {
                                                                                LIVE_WALLPAPER_INTERVAL_SEC += 1;
                                                                            }
                                                                            live_wp_interacted = true;
                                                                        }

                                                                        if live_wp_interacted { break; }
                                                                    }
                                                                }
                                                            }

                                                            if live_wp_interacted {
                                                                redraw = true;
                                                                continue;
                                                            }

                                                            let mut wallpaper_changed = false;
                                                            for i in 0..wm.window_count {
                                                                if click_top != Some(i) { continue; }
                                                                if let Some(ref win) = wm.windows[i] {
                                                                    if win.title == "Wallpaper Selector" && win.state != WindowState::Minimized {
                                                                        let wx = win.x + 2;
                                                                        let wy = win.y + 26;
                                                                        let colors = [
                                                                            Color::rgb(89, 0, 255),
                                                                            Color::rgb(0, 23, 128),
                                                                            Color::rgb(192, 0, 0),
                                                                            Color::rgb(192, 192, 0),
                                                                            Color::rgb(0, 0, 128),
                                                                            Color::rgb(128, 0, 128),
                                                                        ];
                                                                        for (j, _) in colors.iter().enumerate() {
                                                                            let y_pos = wy + 30 + j * 30;
                                                                            if cursor_x >= wx + 10 && cursor_x <= wx + 260 &&
                                                                               cursor_y >= y_pos && cursor_y <= y_pos + 24 {
                                                                                WALLPAPER = colors[j];
                                                                                USE_BMP_WALLPAPER = false;
                                                                                wallpaper_changed = true;
                                                                                wm.close_window(i);
                                                                                break;
                                                                            }
                                                                        }
                                                                        if wallpaper_changed { break; }
                                                                    }
                                                                }
                                                            }

                                                            if !wallpaper_changed {
                                                                let mut theme_changed = false;
                                                                for i in 0..wm.window_count {
                                                                    if click_top != Some(i) { continue; }
                                                                    if let Some(ref win) = wm.windows[i] {
                                                                        if win.title == "Theme Selector" && win.state != WindowState::Minimized {
                                                                            let wx = win.x + 2;
                                                                            let wy = win.y + 26;
                                                                            for j in 0..5 {
                                                                                let y_pos = wy + 30 + j * 30;
                                                                                if cursor_x >= wx + 10 && cursor_x <= wx + 260 &&
                                                                                   cursor_y >= y_pos && cursor_y <= y_pos + 24 {
                                                                                    THEME_IDX = j;
                                                                                    wm::set_theme(j);
                                                                                    theme_changed = true;
                                                                                    wm.close_window(i);
                                                                                    break;
                                                                                }
                                                                            }
                                                                            if theme_changed { break; }
                                                                        }
                                                                    }
                                                                }

                                                                if !theme_changed {
                                                                    if let Some(active) = wm.active_window.filter(|a| click_top == Some(*a)) {
                                                                        if let Some(ref win) = wm.windows[active] {
                                                                            if win.title == "Settings" && win.state != WindowState::Minimized {
                                                                                let wx = win.x + 2;
                                                                                let wy = win.y + 26;

                                                                                let header_h = 50;
                                                                                let content_y = wy + header_h + 16;
                                                                                let btn1_y = content_y;
                                                                                let btn2_y = content_y + 52;
                                                                                let btn3_y = content_y + 104;
                                                                                let btn4_y = content_y + 156;
                                                                                let btn5_y = content_y + 208;
                                                                                let btn6_y = content_y + 260;

                                                                                if cursor_x >= wx + 16 && cursor_x <= wx + win.width - 4 - 16 &&
                                                                                   cursor_y >= btn1_y && cursor_y <= btn1_y + 48 {
                                                                                    wm.create_window("Wallpaper Selector", 250, 200, 300, 250);
                                                                                    redraw = true;
                                                                                } else if cursor_x >= wx + 16 && cursor_x <= wx + win.width - 4 - 16 &&
                                                                                        cursor_y >= btn2_y && cursor_y <= btn2_y + 48 {
                                                                                    wm.create_window("Theme Selector", 250, 200, 300, 230);
                                                                                    redraw = true;
                                                                                } else if cursor_x >= wx + 16 && cursor_x <= wx + win.width - 4 - 16 &&
                                                                                        cursor_y >= btn3_y && cursor_y <= btn3_y + 48 {
                                                                                    USE_BMP_WALLPAPER = !USE_BMP_WALLPAPER;
                                                                                    redraw = true;
                                                                                } else if cursor_x >= wx + 16 && cursor_x <= wx + win.width - 4 - 16 &&
                                                                                        cursor_y >= btn4_y && cursor_y <= btn4_y + 48 {
                                                                                    wm.create_window("BMP Wallpaper Selector", 200, 150, 300, 320);
                                                                                    redraw = true;
                                                                                } else if cursor_x >= wx + 16 && cursor_x <= wx + win.width - 4 - 16 &&
                                                                                        cursor_y >= btn5_y && cursor_y <= btn5_y + 48 {
                                                                                    wm.create_window("Live Wallpaper Settings", 220, 160, 300, 230);
                                                                                    redraw = true;
                                                                                } else if cursor_x >= wx + 16 && cursor_x <= wx + win.width - 4 - 16 &&
                                                                                        cursor_y >= btn6_y && cursor_y <= btn6_y + 48 {
                                                                                    NET_APP_REQUEST = 2;
                                                                                    redraw = true;
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            } else if !packet.left && left_prev {
                                if ICON_DRAG != usize::MAX {
                                    let id = ICON_DRAG;
                                    if ICON_DRAG_MOVED {
                                        let (px, py) = icon_snap(id, ICON_DRAG_POS.0, ICON_DRAG_POS.1);
                                        if id < 9 {
                                            SYS_ICON_POS[id] = icon_enc(px, py);
                                        } else if id < 200 {
                                            if id - 100 < 10 { FOLDER_ICON_POS[id - 100] = icon_enc(px, py); }
                                        } else {
                                            syscfg::set_shortcut_pos(id - 200, px, py);
                                        }
                                        ICON_DRAG = usize::MAX;
                                    } else {
                                        ICON_DRAG = usize::MAX;
                                        match id {
                                            0 => {
                                                if wm.find_window("Terminal").is_none() { term_len = 0; TERM_LEN_GLOBAL = 0; clear_history(); }
                                                wm.create_window("Terminal", 100, 100, 500, 350);
                                            }
                                            1 => { wm.create_window("File Manager", 150, 120, 500, 350); }
                                            2 => { wm.create_window("Settings", 200, 180, 450, 450); }
                                            3 => { wm.create_window("AboutOS", 200, 100, 400, 350); }
                                            4 => { wm.create_window("Notepad", 100, 100, 500, 400); }
                                            5 => { wm.create_window("Paint", 100, 100, 420, 400); }
                                            6 => {
                                                CRE_TOOL_WINDOW_REQUESTED = true;
                                                push_history("Cre Tool opened!");
                                            }
                                            7 => { NET_APP_REQUEST = 1; }
                                            8 => { NET_APP_REQUEST = 2; }
                                            100..=199 => {
                                                let fi = id - 100;
                                                if fi < DESKTOP_FOLDERS_COUNT {
                                                    let folder_name = core::str::from_utf8(&DESKTOP_FOLDERS[fi]).unwrap_or("");
                                                    let folder_name_trimmed = folder_name.trim_end_matches('\0');
                                                    open_folder_window(&mut wm, folder_name_trimmed, fi);
                                                }
                                            }
                                            _ => { if id >= 200 { open_shortcut(&mut wm, id - 200); } }
                                        }
                                    }
                                    ICON_DRAG_MOVED = false;
                                }
                                wm.handle_mouse_release(&mut gfx);
                                redraw = true;
                            } else if packet.left && ICON_DRAG != usize::MAX && (packet.dx != 0 || packet.dy != 0) {
                                if !ICON_DRAG_MOVED {
                                    let ddx = if cursor_x > ICON_DRAG_START.0 { cursor_x - ICON_DRAG_START.0 } else { ICON_DRAG_START.0 - cursor_x };
                                    let ddy = if cursor_y > ICON_DRAG_START.1 { cursor_y - ICON_DRAG_START.1 } else { ICON_DRAG_START.1 - cursor_y };
                                    if ddx > 4 || ddy > 4 { ICON_DRAG_MOVED = true; }
                                }
                                if ICON_DRAG_MOVED {
                                    let nx = cursor_x.saturating_sub(ICON_DRAG_OFF.0);
                                    let ny = cursor_y.saturating_sub(ICON_DRAG_OFF.1);
                                    ICON_DRAG_POS = icon_clamp(nx, ny);
                                    redraw = true;
                                }
                            } else if packet.left && (packet.dx != 0 || packet.dy != 0) {
                                let before = wm.dragged_window.and_then(|i| wm.windows[i].as_ref().map(|w| (w.x, w.y, w.width, w.height)));
                                if wm.resizing_window.is_some() {
                                    wm.handle_mouse_move(&mut gfx, cursor_x, cursor_y);
                                    redraw = true;
                                } else if !wm.handle_mouse_move(&mut gfx, cursor_x, cursor_y) {
                                } else if let (Some(b), Some(a)) = (before, wm.dragged_window.and_then(|i| wm.windows[i].as_ref().map(|w| (w.x, w.y, w.width, w.height)))) {
                                    let x0 = b.0.min(a.0);
                                    let y0 = b.1.min(a.1);
                                    let x1 = (b.0 + b.2).max(a.0 + a.2) + 8;
                                    let y1 = (b.1 + b.3).max(a.1 + a.3) + 8;
                                    drag_rect = Some((x0, y0, x1 - x0, y1 - y0));
                                } else {
                                    redraw = true;
                                }
                            }
                        }
                    }

                    if raised { redraw = true; }

                    unsafe {
                        if MEDIA_WINDOW_REQUESTED {
                            let win_idx = wm.create_window("Media", 200, 100, 700, 550);
                            MEDIA_WINDOW_INDEX = Some(win_idx);
                            MEDIA_BROWSER_OPEN = false;
                            MEDIA_WINDOW_REQUESTED = false;
                            redraw = true;
                        }
                    }

                    unsafe {
                        if GL_WINDOW_REQUESTED {
                            let win_idx = wm.create_window("GL", 150, 80, 500, 450);
                            if GL_APP_INSTANCE.is_none() {
                                GL_APP_INSTANCE = Some(gl::GlApp::new());
                            }
                            GL_WINDOW_INDEX = Some(win_idx);
                            GL_WINDOW_REQUESTED = false;
                            redraw = true;
                        }

                        if GL_WINDOW_CLOSE_REQUESTED {
                            if let Some(idx) = GL_WINDOW_INDEX {
                                if idx < wm.window_count {
                                    wm.close_window(idx);
                                }
                                GL_WINDOW_INDEX = None;
                            }
                            GL_WINDOW_CLOSE_REQUESTED = false;
                            redraw = true;
                        }
                    }

                    unsafe {
                        if CLOCK_WINDOW_REQUESTED {
                            let win_idx = wm.create_window("Clock", 200, 100, 500, 300);
                            CLOCK_WINDOW_INDEX = Some(win_idx);
                            CLOCK_WINDOW_REQUESTED = false;
                            redraw = true;
                        }
                    }

                    unsafe {
                        if calend::CALENDAR_WINDOW_REQUESTED {
                            let win_idx = wm.create_window("Calendar", 200, 100, 500, 320);
                            calend::CALENDAR_WINDOW_INDEX = Some(win_idx);
                            calend::CALENDAR_WINDOW_REQUESTED = false;
                            redraw = true;
                        }

                        if calend::CALENDAR_WINDOW_CLOSE_REQUESTED {
                            if let Some(idx) = calend::CALENDAR_WINDOW_INDEX {
                                if idx < wm.window_count {
                                    wm.close_window(idx);
                                }
                                calend::CALENDAR_WINDOW_INDEX = None;
                            }
                            calend::CALENDAR_WINDOW_CLOSE_REQUESTED = false;
                            redraw = true;
                        }
                    }

                    unsafe {
                        if TASK_WINDOW_REQUESTED {
                            let win_idx = wm.create_window("Task Manager", 200, 100, 500, 400);
                            TASK_WINDOW_INDEX = Some(win_idx);
                            TASK_WINDOW_REQUESTED = false;
                            redraw = true;
                        }

                        if TASK_WINDOW_CLOSE_REQUESTED {
                            if let Some(idx) = TASK_WINDOW_INDEX {
                                if idx < wm.window_count {
                                    wm.close_window(idx);
                                }
                                TASK_WINDOW_INDEX = None;
                            }
                            TASK_WINDOW_CLOSE_REQUESTED = false;
                            redraw = true;
                        }
                    }

                    unsafe {
                        if CRE_TOOL_WINDOW_REQUESTED {
                            let win_idx = wm.create_window("Cre Tool", 100, 100, 480, 400);
                            CRE_TOOL_WINDOW_INDEX = Some(win_idx);
                            CRE_TOOL_WINDOW_REQUESTED = false;
                            redraw = true;
                        }

                        if CRE_TOOL_WINDOW_CLOSE_REQUESTED {
                            if let Some(idx) = CRE_TOOL_WINDOW_INDEX {
                                if idx < wm.window_count {
                                    wm.close_window(idx);
                                }
                                CRE_TOOL_WINDOW_INDEX = None;
                            }
                            CRE_TOOL_WINDOW_CLOSE_REQUESTED = false;
                            redraw = true;
                        }
                    }

                    unsafe {
                        if HTML_WINDOW_REQUESTED {
                            let win_idx = wm.create_window("HTML Viewer", 120, 80, 640, 500);
                            HTML_WINDOW_INDEX = Some(win_idx);
                            if HTML_APP_INSTANCE.is_none() {
                                let mut app = html::HtmlApp::new();
                                app.load(html::TEST_HTML);
                                HTML_APP_INSTANCE = Some(app);
                            }
                            HTML_WINDOW_REQUESTED = false;
                            redraw = true;
                        }

                        if HTML_WINDOW_CLOSE_REQUESTED {
                            if let Some(idx) = HTML_WINDOW_INDEX {
                                if idx < wm.window_count {
                                    wm.close_window(idx);
                                }
                                HTML_WINDOW_INDEX = None;
                            }
                            HTML_WINDOW_CLOSE_REQUESTED = false;
                            redraw = true;
                        }
                    }

                    unsafe {
                        if SUPPORT_WINDOW_REQUESTED {
                            let win_idx = wm.create_window("Support Center", 100, 60, 700, 540);
                            SUPPORT_WINDOW_INDEX = Some(win_idx);
                            if SUPPORT_APP_INSTANCE.is_none() {
                                let mut app = support::SupportApp::new();
                                app.load(support::SUPPORT_HTML);
                                SUPPORT_APP_INSTANCE = Some(app);
                            }
                            SUPPORT_WINDOW_REQUESTED = false;
                            redraw = true;
                        }

                        if SUPPORT_WINDOW_CLOSE_REQUESTED {
                            if let Some(idx) = SUPPORT_WINDOW_INDEX {
                                if idx < wm.window_count {
                                    wm.close_window(idx);
                                }
                                SUPPORT_WINDOW_INDEX = None;
                            }
                            SUPPORT_WINDOW_CLOSE_REQUESTED = false;
                            redraw = true;
                        }

                        if PCINF_WINDOW_REQUESTED {
                            let win_idx = wm.create_window("PC Info", 140, 80, 620, 520);
                            PCINF_WINDOW_INDEX = Some(win_idx);
                            if PCINF_APP_INSTANCE.is_none() {
                                let mut app = pcinf::PcInfoApp::new();
                                app.load(640, 4 * 1024 * 1024);
                                PCINF_APP_INSTANCE = Some(app);
                            }
                            PCINF_WINDOW_REQUESTED = false;
                            redraw = true;
                        }

                        if PCINF_WINDOW_CLOSE_REQUESTED {
                            if let Some(idx) = PCINF_WINDOW_INDEX {
                                if idx < wm.window_count {
                                    wm.close_window(idx);
                                }
                                PCINF_WINDOW_INDEX = None;
                            }
                            PCINF_WINDOW_CLOSE_REQUESTED = false;
                            redraw = true;
                        }
                    }

                    unsafe {
                        if MEDIA_NEEDS_REDRAW {
                            MEDIA_NEEDS_REDRAW = false;
                            redraw = true;
                        }
                    }

                    for dy in 0..cursor_size {
                        for dx in 0..cursor_size {
                            if old_x + dx < gfx.width && old_y + dy < gfx.height {
                                gfx.put_pixel(old_x + dx, old_y + dy, cursor_backup[dy][dx]);
                            }
                        }
                    }

                    left_prev = packet.left;
                    right_prev = packet.right;

                    if redraw || paint_update {
                        unsafe {
                            if LOGGED_IN {
                                redraw_all(&mut gfx, &wm, &term_input, term_len);
                            } else {
                                draw_login_window(&mut gfx);
                            }
                        }
                        need_flush = true;
                    } else if let Some(r) = drag_rect {
                        drag_redraw(&mut gfx, &wm, &term_input, term_len, r);
                        dirty.add(r.0, r.1, r.2, r.3);
                    }
                    dirty.add(old_x, old_y, cursor_size, cursor_size);
                    dirty.add(cursor_x, cursor_y, cursor_size, cursor_size);

                    for dy in 0..cursor_size {
                        for dx in 0..cursor_size {
                            if cursor_x + dx < gfx.width && cursor_y + dy < gfx.height {
                                let offset = (cursor_y + dy) * gfx.width + (cursor_x + dx);
                                cursor_backup[dy][dx] = unsafe { graphics::BACKBUFFER[offset] };
                            }
                        }
                    }

                    for dy in 0..cursor_size {
                        for dx in 0..cursor_size {
                            let pixel = cursor_shape[dy][dx];
                            if pixel == 0 { continue; }

                            let color = match pixel {
                                1 => Color::BLACK.to_u32(),
                                2 => Color::WHITE.to_u32(),
                                _ => continue,
                            };

                            let mut is_border = false;
                            if pixel == 1 {
                                for ndy in -1..=1 {
                                    for ndx in -1..=1 {
                                        if ndx == 0 && ndy == 0 { continue; }
                                        let nx = dx as i32 + ndx;
                                        let ny = dy as i32 + ndy;
                                        if nx >= 0 && nx < cursor_size as i32 && ny >= 0 && ny < cursor_size as i32 {
                                            if cursor_shape[ny as usize][nx as usize] == 0 {
                                                is_border = true;
                                                break;
                                            }
                                        } else {
                                            is_border = true;
                                            break;
                                        }
                                    }
                                    if is_border { break; }
                                }
                            }

                            let final_color = if is_border {
                                Color::BLACK.to_u32()
                            } else {
                                color
                            };

                            gfx.put_pixel(cursor_x + dx, cursor_y + dy, final_color);
                        }
                    }
                }

                unsafe {
                    if LOGGED_IN {
                        GL_UPDATE_COUNTER += 1;
                        if GL_UPDATE_COUNTER % 2 == 0 {
                            if let Some(idx) = GL_WINDOW_INDEX {
                                if idx < wm.window_count {
                                    if let Some(ref win) = wm.windows[idx] {
                                        if win.state != WindowState::Minimized && win.title == "GL" {

                                            let mut cursor_save = [[0u32; 23]; 23];
                                            for dy in 0..cursor_size {
                                                for dx in 0..cursor_size {
                                                    if cursor_x + dx < gfx.width && cursor_y + dy < gfx.height {
                                                        let offset = (cursor_y + dy) * gfx.width + (cursor_x + dx);
                                                        cursor_save[dy][dx] = graphics::BACKBUFFER[offset];
                                                    }
                                                }
                                            }

                                            redraw_all(&mut gfx, &wm, &term_input, term_len);

                                            for dy in 0..cursor_size {
                                                for dx in 0..cursor_size {
                                                    let pixel = cursor_shape[dy][dx];
                                                    if pixel == 0 { continue; }

                                                    let color = match pixel {
                                                        1 => Color::BLACK.to_u32(),
                                                        2 => Color::WHITE.to_u32(),
                                                        _ => continue,
                                                    };

                                                    let mut is_border = false;
                                                    if pixel == 1 {
                                                        for ndy in -1..=1 {
                                                            for ndx in -1..=1 {
                                                                if ndx == 0 && ndy == 0 { continue; }
                                                                let nx = dx as i32 + ndx;
                                                                let ny = dy as i32 + ndy;
                                                                if nx >= 0 && nx < cursor_size as i32 && ny >= 0 && ny < cursor_size as i32 {
                                                                    if cursor_shape[ny as usize][nx as usize] == 0 {
                                                                        is_border = true;
                                                                        break;
                                                                    }
                                                                } else {
                                                                    is_border = true;
                                                                    break;
                                                                }
                                                            }
                                                            if is_border { break; }
                                                        }
                                                    }

                                                    let final_color = if is_border {
                                                        Color::BLACK.to_u32()
                                                    } else {
                                                        color
                                                    };

                                                    gfx.put_pixel(cursor_x + dx, cursor_y + dy, final_color);
                                                }
                                            }

                                            need_flush = true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if need_flush {
                    gfx.flush();
                } else {
                    dirty.flush(&mut gfx);
                }

                for _ in 0..5000 { unsafe { core::arch::asm!("nop") } }
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