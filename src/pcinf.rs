

use crate::graphics::Graphics;
use crate::ata;
use crate::VESA_INFO;
use core::arch::x86_64::__cpuid;

pub const WINDOW_TITLE: &str = "PC Info";


fn copy_str(dst: &mut [u8], dst_len: &mut usize, src: &[u8]) {
    let n = src.len().min(dst.len());
    dst[..n].copy_from_slice(&src[..n]);
    *dst_len = n;
}

fn trim_ascii(dst: &mut [u8], dst_len: &mut usize) {
    let mut end = *dst_len;
    while end > 0 && (dst[end - 1] == 0 || dst[end - 1] == b' ' || dst[end - 1] == b'\r' || dst[end - 1] == b'\n') {
        end -= 1;
    }
    *dst_len = end;
}



pub struct PcInfoApp {
    
    cpu_vendor: [u8; 16],
    cpu_vendor_len: usize,
    cpu_brand: [u8; 64],
    cpu_brand_len: usize,
    cpu_family: u32,
    cpu_model: u32,
    cpu_stepping: u32,
    cpu_ext_family: u32,
    cpu_ext_model: u32,
    cpu_logical: u32,
    cpu_has_brand: bool,
    cpu_has_htt: bool,
    cpu_has_sse2: bool,
    cpu_has_avx: bool,
    cpu_has_avx2: bool,
    cpu_has_vmx: bool,
    cpu_has_svm: bool,
    cpu_has_tsc: bool,
    cpu_has_fpu: bool,
    cpu_has_sse: bool,
    cpu_has_sse3: bool,
    cpu_has_ssse3: bool,
    cpu_has_sse41: bool,
    cpu_has_sse42: bool,
    cpu_has_popcnt: bool,
    cpu_has_aes: bool,
    cpu_has_rdrand: bool,

    
    bios_vendor: [u8; 64],
    bios_vendor_len: usize,
    bios_version: [u8; 64],
    bios_version_len: usize,
    bios_date: [u8; 32],
    bios_date_len: usize,
    sys_vendor: [u8; 64],
    sys_vendor_len: usize,
    sys_product: [u8; 64],
    sys_product_len: usize,
    sys_version: [u8; 64],
    sys_version_len: usize,
    smbios_found: bool,

    
    total_ram_mb: u64,
    mem_lower_kb: u32,
    mem_upper_kb: u32,

    
    vesa_width: u32,
    vesa_height: u32,
    vesa_bpp: u32,
    vesa_pitch: u32,
    vesa_addr: u64,

    disk_count: usize,

    scroll_y: usize,

    loaded: bool,
}

impl PcInfoApp {
    pub const fn new() -> Self {
        Self {
            cpu_vendor: [0; 16],
            cpu_vendor_len: 0,
            cpu_brand: [0; 64],
            cpu_brand_len: 0,
            cpu_family: 0,
            cpu_model: 0,
            cpu_stepping: 0,
            cpu_ext_family: 0,
            cpu_ext_model: 0,
            cpu_logical: 0,
            cpu_has_brand: false,
            cpu_has_htt: false,
            cpu_has_sse2: false,
            cpu_has_avx: false,
            cpu_has_avx2: false,
            cpu_has_vmx: false,
            cpu_has_svm: false,
            cpu_has_tsc: false,
            cpu_has_fpu: false,
            cpu_has_sse: false,
            cpu_has_sse3: false,
            cpu_has_ssse3: false,
            cpu_has_sse41: false,
            cpu_has_sse42: false,
            cpu_has_popcnt: false,
            cpu_has_aes: false,
            cpu_has_rdrand: false,
            bios_vendor: [0; 64],
            bios_vendor_len: 0,
            bios_version: [0; 64],
            bios_version_len: 0,
            bios_date: [0; 32],
            bios_date_len: 0,
            sys_vendor: [0; 64],
            sys_vendor_len: 0,
            sys_product: [0; 64],
            sys_product_len: 0,
            sys_version: [0; 64],
            sys_version_len: 0,
            smbios_found: false,
            total_ram_mb: 0,
            mem_lower_kb: 0,
            mem_upper_kb: 0,
            vesa_width: 0,
            vesa_height: 0,
            vesa_bpp: 0,
            vesa_pitch: 0,
            vesa_addr: 0,
            disk_count: 0,
            scroll_y: 0,
            loaded: false,
        }
    }

    pub fn load(&mut self, mem_lower_kb: u32, mem_upper_kb: u32) {
        self.mem_lower_kb = mem_lower_kb;
        self.mem_upper_kb = mem_upper_kb;
        self.total_ram_mb = ((mem_lower_kb as u64) + (mem_upper_kb as u64) + 1024) / 1024;

        unsafe {
            if let Some(ref v) = VESA_INFO {
                self.vesa_width = v.width as u32;
                self.vesa_height = v.height as u32;
                self.vesa_bpp = v.bpp as u32;
                self.vesa_pitch = v.pitch as u32;
                self.vesa_addr = v.addr as u64;
            }
        }

        self.disk_count = unsafe { ata::disk_count() };

        self.read_cpuid();
        self.read_smbios();

        self.loaded = true;
    }

    

    fn read_cpuid(&mut self) {
        unsafe {
            let leaf0 = __cpuid(0);
            let mut vendor = [0u8; 12];
            vendor[0..4].copy_from_slice(&leaf0.ebx.to_le_bytes());
            vendor[4..8].copy_from_slice(&leaf0.edx.to_le_bytes());
            vendor[8..12].copy_from_slice(&leaf0.ecx.to_le_bytes());
            copy_str(&mut self.cpu_vendor, &mut self.cpu_vendor_len, &vendor);

            let leaf1 = __cpuid(1);
            self.cpu_stepping = leaf1.eax & 0xF;
            self.cpu_model = (leaf1.eax >> 4) & 0xF;
            self.cpu_family = (leaf1.eax >> 8) & 0xF;
            self.cpu_ext_model = (leaf1.eax >> 16) & 0xF;
            self.cpu_ext_family = (leaf1.eax >> 20) & 0xFF;

            let edx = leaf1.edx;
            let ecx = leaf1.ecx;

            self.cpu_has_fpu = (edx & (1 << 0)) != 0;
            self.cpu_has_tsc = (edx & (1 << 4)) != 0;
            self.cpu_has_htt = (edx & (1 << 28)) != 0;
            self.cpu_has_sse = (edx & (1 << 25)) != 0;
            self.cpu_has_sse2 = (edx & (1 << 26)) != 0;
            self.cpu_has_vmx = (ecx & (1 << 5)) != 0;
            self.cpu_has_ssse3 = (ecx & (1 << 9)) != 0;
            self.cpu_has_sse3 = (ecx & (1 << 0)) != 0;
            self.cpu_has_sse41 = (ecx & (1 << 19)) != 0;
            self.cpu_has_sse42 = (ecx & (1 << 20)) != 0;
            self.cpu_has_popcnt = (ecx & (1 << 23)) != 0;
            self.cpu_has_aes = (ecx & (1 << 25)) != 0;
            self.cpu_has_avx = (ecx & (1 << 28)) != 0;
            self.cpu_has_rdrand = (ecx & (1 << 30)) != 0;

            
            let leaf_ext = __cpuid(0x80000000);
            let max_ext = leaf_ext.eax;
            if max_ext >= 0x80000001 {
                let ext = __cpuid(0x80000001);
                self.cpu_has_svm = (ext.ecx & (1 << 2)) != 0;
            }

            
            self.cpu_has_brand = max_ext >= 0x80000004;
            if self.cpu_has_brand {
                let mut brand = [0u8; 48];
                for i in 0..3u32 {
                    let l = __cpuid(0x80000002 + i);
                    let base = (i as usize) * 16;
                    brand[base..base + 4].copy_from_slice(&l.eax.to_le_bytes());
                    brand[base + 4..base + 8].copy_from_slice(&l.ebx.to_le_bytes());
                    brand[base + 8..base + 12].copy_from_slice(&l.ecx.to_le_bytes());
                    brand[base + 12..base + 16].copy_from_slice(&l.edx.to_le_bytes());
                }
                copy_str(&mut self.cpu_brand, &mut self.cpu_brand_len, &brand);
                let mut start = 0;
                while start < self.cpu_brand_len && self.cpu_brand[start] == b' ' { start += 1; }
                if start > 0 {
                    let n = self.cpu_brand_len - start;
                    for i in 0..n {
                        self.cpu_brand[i] = self.cpu_brand[start + i];
                    }
                    self.cpu_brand_len = n;
                }
                trim_ascii(&mut self.cpu_brand, &mut self.cpu_brand_len);
            }

            
            if leaf0.eax >= 0xB {
                let lvl = __cpuid(0xB);
                if (lvl.ebx & 0xFFFF) > 0 {
                    self.cpu_logical = lvl.ebx & 0xFFFF;
                }
            }
            if self.cpu_logical == 0 && self.cpu_has_htt {
                self.cpu_logical = (leaf1.ebx >> 16) & 0xFF;
            }
            if self.cpu_logical == 0 {
                self.cpu_logical = 1;
            }
        }
    }

    

    fn read_smbios(&mut self) {
        unsafe {
            let ebda_seg: u16 = {
                let p = 0x40E as *const u16;
                core::ptr::read_volatile(p)
            };
            let ebda_phys = (ebda_seg as u32) << 4;
            if self.scan_smbios(ebda_phys, 0x4000) { return; }
            if self.scan_smbios(0xE0000, 0x20000) { return; }
            self.scan_smbios(0xF0000, 0x10000);
        }
    }

    unsafe fn scan_smbios(&mut self, start: u32, len: u32) -> bool {
        let mut addr = start;
        let end = start + len;
        while addr + 16 <= end {
            let ptr = addr as *const u8;
            let b0 = core::ptr::read_volatile(ptr);
            let b1 = core::ptr::read_volatile(ptr.add(1));
            let b2 = core::ptr::read_volatile(ptr.add(2));
            let b3 = core::ptr::read_volatile(ptr.add(3));
            if b0 == b'_' && b1 == b'S' && b2 == b'M' && b3 == b'_' {
                let struct_table_addr = core::ptr::read_unaligned(ptr.add(0x19) as *const u32);
                let struct_table_len = core::ptr::read_unaligned(ptr.add(0x17) as *const u16) as u32;
                if struct_table_addr != 0 && struct_table_len > 0 {
                    self.parse_smbios_table(struct_table_addr, struct_table_len);
                    self.smbios_found = true;
                    return true;
                }
            }
            addr += 16;
        }
        false
    }

    unsafe fn parse_smbios_table(&mut self, addr: u32, len: u32) {
        let mut p = addr;
        let end = addr + len;
        while p + 4 <= end {
            let ty = core::ptr::read_volatile(p as *const u8);
            let hdr_len = core::ptr::read_volatile((p + 1) as *const u8) as u32;
            if hdr_len < 4 { break; }

            let str_start = p + hdr_len;
            let mut str_end = str_start;
            let mut zero_count = 0;
            while str_end < end {
                let b = core::ptr::read_volatile(str_end as *const u8);
                if b == 0 {
                    zero_count += 1;
                    if zero_count == 2 { break; }
                } else {
                    zero_count = 0;
                }
                str_end += 1;
            }

            match ty {
                0 => self.parse_bios_info(p, hdr_len, str_start, str_end),
                1 => self.parse_sys_info(p, hdr_len, str_start, str_end),
                _ => {}
            }

            let next = str_end + 1;
            if next <= p { break; }
            p = next;
        }
    }

    unsafe fn get_smbios_string(&self, str_start: u32, str_end: u32, index: u8) -> ([u8; 64], usize) {
        let mut out = [0u8; 64];
        if index == 0 { return (out, 0); }
        let mut cur = 1u8;
        let mut p = str_start;
        while p < str_end && cur < index {
            while p < str_end && core::ptr::read_volatile(p as *const u8) != 0 {
                p += 1;
            }
            p += 1;
            cur += 1;
        }
        if cur == index {
            let mut n = 0usize;
            while p < str_end && n < out.len() {
                let b = core::ptr::read_volatile(p as *const u8);
                if b == 0 { break; }
                out[n] = b;
                n += 1;
                p += 1;
            }
            (out, n)
        } else {
            (out, 0)
        }
    }

    unsafe fn parse_bios_info(&mut self, p: u32, hdr_len: u32, str_start: u32, str_end: u32) {
        if hdr_len < 0x12 { return; }
        let vendor_idx = core::ptr::read_volatile((p + 0x04) as *const u8);
        let version_idx = core::ptr::read_volatile((p + 0x05) as *const u8);
        let date_idx = core::ptr::read_volatile((p + 0x08) as *const u8);

        let (v, vl) = self.get_smbios_string(str_start, str_end, vendor_idx);
        copy_str(&mut self.bios_vendor, &mut self.bios_vendor_len, &v[..vl]);
        let (v, vl) = self.get_smbios_string(str_start, str_end, version_idx);
        copy_str(&mut self.bios_version, &mut self.bios_version_len, &v[..vl]);
        let (v, vl) = self.get_smbios_string(str_start, str_end, date_idx);
        copy_str(&mut self.bios_date, &mut self.bios_date_len, &v[..vl]);
    }

    unsafe fn parse_sys_info(&mut self, p: u32, hdr_len: u32, str_start: u32, str_end: u32) {
        if hdr_len < 0x1B { return; }
        let manu_idx = core::ptr::read_volatile((p + 0x04) as *const u8);
        let product_idx = core::ptr::read_volatile((p + 0x05) as *const u8);
        let version_idx = core::ptr::read_volatile((p + 0x06) as *const u8);

        let (v, vl) = self.get_smbios_string(str_start, str_end, manu_idx);
        copy_str(&mut self.sys_vendor, &mut self.sys_vendor_len, &v[..vl]);
        let (v, vl) = self.get_smbios_string(str_start, str_end, product_idx);
        copy_str(&mut self.sys_product, &mut self.sys_product_len, &v[..vl]);
        let (v, vl) = self.get_smbios_string(str_start, str_end, version_idx);
        copy_str(&mut self.sys_version, &mut self.sys_version_len, &v[..vl]);
    }

    

    pub fn scroll_up(&mut self) {
        if self.scroll_y >= 30 { self.scroll_y -= 30; } else { self.scroll_y = 0; }
    }
    pub fn scroll_down(&mut self) {
        self.scroll_y = self.scroll_y.saturating_add(30);
    }
}


const LINE_H: usize = 16;
const FONT_W: usize = 8;

struct Ctx {
    y: usize,
    cx: usize,
    cy: usize,
    w: usize,
    h: usize,
    scroll: usize,
}

impl Ctx {
    fn screen_y(&self) -> Option<usize> {
        if self.y + LINE_H <= self.scroll { return None; }
        if self.y >= self.scroll + self.h { return None; }
        Some(self.cy + (self.y - self.scroll))
    }
    fn newline(&mut self) { self.y += LINE_H; }
    fn blank(&mut self) { self.y += LINE_H / 2; }
}

fn draw_line_text(gfx: &mut Graphics, ctx: &mut Ctx, text: &str, color: u32) {
    if let Some(sy) = ctx.screen_y() {
        gfx.draw_text(ctx.cx + 8, sy, text, color, 0xFFFFFF);
    }
    ctx.newline();
}

fn draw_kv(gfx: &mut Graphics, ctx: &mut Ctx, key: &str, val: &str, key_color: u32, val_color: u32) {
    if let Some(sy) = ctx.screen_y() {
        gfx.draw_text(ctx.cx + 16, sy, key, key_color, 0xFFFFFF);
        gfx.draw_text(ctx.cx + 16 + 22 * FONT_W, sy, val, val_color, 0xFFFFFF);
    }
    ctx.newline();
}

fn draw_section(gfx: &mut Graphics, ctx: &mut Ctx, title: &str) {
    ctx.blank();
    if let Some(sy) = ctx.screen_y() {
        gfx.fill_rect(ctx.cx + 8, sy + 12, ctx.w - 16, 1, 0x808080);
        gfx.draw_text(ctx.cx + 8, sy, title, 0x000080, 0xFFFFFF);
    }
    ctx.newline();
    ctx.newline();
}

fn bytes_to_str(buf: &[u8]) -> &str {
    core::str::from_utf8(buf).unwrap_or("?")
}

fn u32_to_str(mut n: u32) -> &'static str {
    static mut BUF: [u8; 16] = [0; 16];
    unsafe {
        if n == 0 { return "0"; }
        let mut digits = [0u8; 16];
        let mut i = 0;
        while n > 0 && i < 16 {
            digits[i] = b'0' + (n % 10) as u8;
            n /= 10;
            i += 1;
        }
        let mut j = 0;
        while i > 0 {
            i -= 1;
            BUF[j] = digits[i];
            j += 1;
        }
        BUF[j] = 0;
        core::str::from_utf8(&BUF[..j]).unwrap_or("?")
    }
}

fn u64_to_str(mut n: u64) -> &'static str {
    static mut BUF: [u8; 24] = [0; 24];
    unsafe {
        if n == 0 { return "0"; }
        let mut digits = [0u8; 24];
        let mut i = 0;
        while n > 0 && i < 24 {
            digits[i] = b'0' + (n % 10) as u8;
            n /= 10;
            i += 1;
        }
        let mut j = 0;
        while i > 0 {
            i -= 1;
            BUF[j] = digits[i];
            j += 1;
        }
        BUF[j] = 0;
        core::str::from_utf8(&BUF[..j]).unwrap_or("?")
    }
}

pub fn draw_pcinf_content(gfx: &mut Graphics, cx: usize, cy: usize, w: usize, h: usize, app: &mut PcInfoApp) {
    gfx.fill_rect(cx, cy, w, h, 0xFFFFFF);
    gfx.draw_rect_border(cx, cy, w, h, 0x787878);

    if !app.loaded {
        gfx.draw_text(cx + 10, cy + 10, "Loading PC Info...", 0x000000, 0xFFFFFF);
        return;
    }

    let view_h = if h > 40 { h - 40 } else { h };

    let mut ctx = Ctx {
        y: 10,
        cx,
        cy,
        w,
        h: view_h,
        scroll: app.scroll_y,
    };

    
    if let Some(sy) = ctx.screen_y() {
        gfx.draw_text(cx + (w - 14 * FONT_W) / 2, sy, "PC Information", 0x000080, 0xFFFFFF);
    }
    ctx.newline();
    ctx.newline();

    
    draw_section(gfx, &mut ctx, "System");

    if app.smbios_found {
        if app.sys_vendor_len > 0 {
            draw_kv(gfx, &mut ctx, "Manufacturer:",
                    bytes_to_str(&app.sys_vendor[..app.sys_vendor_len]), 0x404040, 0x000000);
        }
        if app.sys_product_len > 0 {
            draw_kv(gfx, &mut ctx, "Product:",
                    bytes_to_str(&app.sys_product[..app.sys_product_len]), 0x404040, 0x000000);
        }
        if app.sys_version_len > 0 {
            draw_kv(gfx, &mut ctx, "Version:",
                    bytes_to_str(&app.sys_version[..app.sys_version_len]), 0x404040, 0x000000);
        }
    } else {
        draw_kv(gfx, &mut ctx, "Manufacturer:", "(SMBIOS not found)", 0x404040, 0x808080);
        draw_kv(gfx, &mut ctx, "Product:", "(SMBIOS not found)", 0x404040, 0x808080);
    }

    
    draw_section(gfx, &mut ctx, "BIOS / Firmware");

    if app.bios_vendor_len > 0 {
        draw_kv(gfx, &mut ctx, "Vendor:",
                bytes_to_str(&app.bios_vendor[..app.bios_vendor_len]), 0x404040, 0x000000);
    }
    if app.bios_version_len > 0 {
        draw_kv(gfx, &mut ctx, "Version:",
                bytes_to_str(&app.bios_version[..app.bios_version_len]), 0x404040, 0x000000);
    }
    if app.bios_date_len > 0 {
        draw_kv(gfx, &mut ctx, "Date:",
                bytes_to_str(&app.bios_date[..app.bios_date_len]), 0x404040, 0x000000);
    }
    if app.bios_vendor_len == 0 && app.bios_version_len == 0 && app.bios_date_len == 0 {
        draw_kv(gfx, &mut ctx, "Info:", "(BIOS doesn't expose SMBIOS)", 0x404040, 0x808080);
    }

    
    draw_section(gfx, &mut ctx, "Processor");

    if app.cpu_brand_len > 0 {
        draw_kv(gfx, &mut ctx, "Model:",
                bytes_to_str(&app.cpu_brand[..app.cpu_brand_len]), 0x404040, 0x000000);
    } else {
        draw_kv(gfx, &mut ctx, "Model:", "(no brand string)", 0x404040, 0x808080);
    }

    draw_kv(gfx, &mut ctx, "Vendor:",
            bytes_to_str(&app.cpu_vendor[..app.cpu_vendor_len]), 0x404040, 0x000000);

    draw_kv(gfx, &mut ctx, "Family:", u32_to_str(app.cpu_family), 0x404040, 0x000000);
    draw_kv(gfx, &mut ctx, "Model ID:", u32_to_str(app.cpu_model), 0x404040, 0x000000);
    draw_kv(gfx, &mut ctx, "Stepping:", u32_to_str(app.cpu_stepping), 0x404040, 0x000000);
    if app.cpu_ext_family > 0 {
        draw_kv(gfx, &mut ctx, "Ext. Family:", u32_to_str(app.cpu_ext_family), 0x404040, 0x000000);
    }
    if app.cpu_ext_model > 0 {
        draw_kv(gfx, &mut ctx, "Ext. Model:", u32_to_str(app.cpu_ext_model), 0x404040, 0x000000);
    }
    draw_kv(gfx, &mut ctx, "Logical CPUs:", u32_to_str(app.cpu_logical), 0x404040, 0x000000);

    ctx.blank();
    draw_line_text(gfx, &mut ctx, "Instruction set extensions:", 0x404040);
    ctx.newline();

    let mut ext_line: [u8; 128] = [0; 128];
    let mut ep = 0usize;
    macro_rules! add_ext {
        ($name:expr, $cond:expr) => {
            if $cond {
                let s = $name.as_bytes();
                for &b in s { if ep < 127 { ext_line[ep] = b; ep += 1; } }
                if ep < 127 { ext_line[ep] = b' '; ep += 1; }
            }
        };
    }
    add_ext!("FPU", app.cpu_has_fpu);
    add_ext!("TSC", app.cpu_has_tsc);
    add_ext!("SSE", app.cpu_has_sse);
    add_ext!("SSE2", app.cpu_has_sse2);
    add_ext!("SSE3", app.cpu_has_sse3);
    add_ext!("SSSE3", app.cpu_has_ssse3);
    add_ext!("SSE4.1", app.cpu_has_sse41);
    add_ext!("SSE4.2", app.cpu_has_sse42);
    add_ext!("AVX", app.cpu_has_avx);
    add_ext!("AES-NI", app.cpu_has_aes);
    add_ext!("POPCNT", app.cpu_has_popcnt);
    add_ext!("RDRAND", app.cpu_has_rdrand);
    add_ext!("HTT", app.cpu_has_htt);
    add_ext!("VMX", app.cpu_has_vmx);
    add_ext!("SVM", app.cpu_has_svm);

    if ep > 0 {
        if ext_line[ep - 1] == b' ' { ep -= 1; }
        let s = core::str::from_utf8(&ext_line[..ep]).unwrap_or("?");
        draw_line_text(gfx, &mut ctx, s, 0x000000);
    } else {
        draw_line_text(gfx, &mut ctx, "(none detected)", 0x808080);
    }

    
    draw_section(gfx, &mut ctx, "Memory");

    if let Some(sy) = ctx.screen_y() {
        gfx.draw_text(ctx.cx + 16, sy, "Total RAM:", 0x404040, 0xFFFFFF);
        let s = u64_to_str(app.total_ram_mb);
        gfx.draw_text(ctx.cx + 16 + 22 * FONT_W, sy, s, 0x000000, 0xFFFFFF);
        gfx.draw_text(ctx.cx + 16 + 22 * FONT_W + s.len() * FONT_W, sy, " MB", 0x606060, 0xFFFFFF);
    }
    ctx.newline();

    if let Some(sy) = ctx.screen_y() {
        gfx.draw_text(ctx.cx + 16, sy, "Base memory:", 0x404040, 0xFFFFFF);
        let s = u32_to_str(app.mem_lower_kb);
        gfx.draw_text(ctx.cx + 16 + 22 * FONT_W, sy, s, 0x000000, 0xFFFFFF);
        gfx.draw_text(ctx.cx + 16 + 22 * FONT_W + s.len() * FONT_W, sy, " KB", 0x606060, 0xFFFFFF);
    }
    ctx.newline();

    if let Some(sy) = ctx.screen_y() {
        gfx.draw_text(ctx.cx + 16, sy, "Extended:", 0x404040, 0xFFFFFF);
        let s = u32_to_str(app.mem_upper_kb);
        gfx.draw_text(ctx.cx + 16 + 22 * FONT_W, sy, s, 0x000000, 0xFFFFFF);
        gfx.draw_text(ctx.cx + 16 + 22 * FONT_W + s.len() * FONT_W, sy, " KB", 0x606060, 0xFFFFFF);
    }
    ctx.newline();

    
    draw_section(gfx, &mut ctx, "Display (VESA)");

    if app.vesa_width > 0 {
        let mut buf = [0u8; 64];
        let mut p = 0usize;
        for b in u32_to_str(app.vesa_width).bytes() { if p < 63 { buf[p] = b; p += 1; } }
        if p < 63 { buf[p] = b'x'; p += 1; }
        for b in u32_to_str(app.vesa_height).bytes() { if p < 63 { buf[p] = b; p += 1; } }
        let s = core::str::from_utf8(&buf[..p]).unwrap_or("?");
        draw_kv(gfx, &mut ctx, "Resolution:", s, 0x404040, 0x000000);

        if let Some(sy) = ctx.screen_y() {
            gfx.draw_text(ctx.cx + 16, sy, "Color depth:", 0x404040, 0xFFFFFF);
            let s = u32_to_str(app.vesa_bpp);
            gfx.draw_text(ctx.cx + 16 + 22 * FONT_W, sy, s, 0x000000, 0xFFFFFF);
            gfx.draw_text(ctx.cx + 16 + 22 * FONT_W + s.len() * FONT_W, sy, " bpp", 0x606060, 0xFFFFFF);
        }
        ctx.newline();

        if let Some(sy) = ctx.screen_y() {
            gfx.draw_text(ctx.cx + 16, sy, "Pitch:", 0x404040, 0xFFFFFF);
            let s = u32_to_str(app.vesa_pitch);
            gfx.draw_text(ctx.cx + 16 + 22 * FONT_W, sy, s, 0x000000, 0xFFFFFF);
            gfx.draw_text(ctx.cx + 16 + 22 * FONT_W + s.len() * FONT_W, sy, " bytes", 0x606060, 0xFFFFFF);
        }
        ctx.newline();

        let mut hexbuf = [0u8; 32];
        let mut hp = 0usize;
        hexbuf[hp] = b'0'; hp += 1;
        hexbuf[hp] = b'x'; hp += 1;
        let hex_chars = b"0123456789ABCDEF";
        let mut started = false;
        for shift in (0..16).rev() {
            let nib = ((app.vesa_addr >> (shift * 4)) & 0xF) as usize;
            if nib != 0 || started || shift == 0 {
                hexbuf[hp] = hex_chars[nib]; hp += 1;
                started = true;
            }
        }
        let s = core::str::from_utf8(&hexbuf[..hp]).unwrap_or("?");
        draw_kv(gfx, &mut ctx, "Framebuffer:", s, 0x404040, 0x000000);
    } else {
        draw_line_text(gfx, &mut ctx, "(no VESA framebuffer)", 0x808080);
    }

    
    draw_section(gfx, &mut ctx, "Storage Devices");

    if app.disk_count == 0 {
        draw_line_text(gfx, &mut ctx, "No ATA/ATAPI devices detected", 0x808080);
    } else {
        for i in 0..app.disk_count {
            if let Some(disk) = unsafe { ata::get_disk(i) } {
                if !disk.present { continue; }
                ctx.blank();

                let mut numbuf = [0u8; 16];
                let mut np = 0usize;
                for &b in b"Disk ".iter() { numbuf[np] = b; np += 1; }
                for b in u32_to_str(i as u32 + 1).bytes() { numbuf[np] = b; np += 1; }
                numbuf[np] = b':'; np += 1;
                let ds = core::str::from_utf8(&numbuf[..np]).unwrap_or("Disk:");
                draw_line_text(gfx, &mut ctx, ds, 0x000080);

                let model = core::str::from_utf8(&disk.model).unwrap_or("?");
                let m = model.trim_end_matches('\0');
                draw_kv(gfx, &mut ctx, "  Model:", m, 0x404040, 0x000000);

                let t = match disk.dev_type {
                    ata::DeviceType::ATA => "ATA (Hard Disk)",
                    ata::DeviceType::ATAPI => "ATAPI (CD/DVD)",
                    _ => "Unknown",
                };
                draw_kv(gfx, &mut ctx, "  Type:", t, 0x404040, 0x000000);

                let sectors = disk.sectors as u64;
                let bytes = sectors * 512;
                let mb = bytes / (1024 * 1024);
                let mut szbuf = [0u8; 32];
                let mut sp = 0usize;
                for b in u64_to_str(mb).bytes() { szbuf[sp] = b; sp += 1; }
                for &b in b" MB".iter() { szbuf[sp] = b; sp += 1; }
                let szs = core::str::from_utf8(&szbuf[..sp]).unwrap_or("?");
                draw_kv(gfx, &mut ctx, "  Size:", szs, 0x404040, 0x000000);

                draw_kv(gfx, &mut ctx, "  Sectors:", u64_to_str(sectors), 0x404040, 0x000000);
            }
        }
    }

    
    draw_section(gfx, &mut ctx, "Operating System");

    draw_kv(gfx, &mut ctx, "Name:", "MiUX 7.0.0", 0x404040, 0x000000);
    draw_kv(gfx, &mut ctx, "Kernel:", "Miux 7.0.0 (x64)", 0x404040, 0x000000);
    draw_kv(gfx, &mut ctx, "Build:", "2026", 0x404040, 0x000000);
    draw_kv(gfx, &mut ctx, "Vendor:", "MDEVS", 0x404040, 0x000000);

    let content_h = ctx.y;

    
    if content_h > view_h {
        let max_scroll = content_h - view_h;
        if app.scroll_y > max_scroll { app.scroll_y = max_scroll; }
    } else {
        app.scroll_y = 0;
    }

    
    let arrow_size = 26usize;
    let arrow_y = cy + h - arrow_size - 4;
    let arrow_up_x = cx + w - arrow_size * 2 - 12;
    let arrow_dn_x = cx + w - arrow_size - 8;

    
    gfx.fill_rect(arrow_up_x, arrow_y, arrow_size, arrow_size, 0xD4D0C8);
    gfx.draw_rect_border(arrow_up_x, arrow_y, arrow_size, arrow_size, 0x000000);
    for r in 0..9 {
        gfx.fill_rect(arrow_up_x + 13 - r, arrow_y + 8 + r, 1 + r * 2, 1, 0x000000);
    }

    
    gfx.fill_rect(arrow_dn_x, arrow_y, arrow_size, arrow_size, 0xD4D0C8);
    gfx.draw_rect_border(arrow_dn_x, arrow_y, arrow_size, arrow_size, 0x000000);
    for r in 0..9 {
        gfx.fill_rect(arrow_dn_x + 4 + r, arrow_y + 8 + r, 17 - r * 2, 1, 0x000000);
    }

    
    if arrow_y > cy + 4 {
        let sy = arrow_y + 8;
        let max_scroll = if content_h > view_h { content_h - view_h } else { 0 };
        let mut buf = [0u8; 32];
        let mut p = 0usize;
        for &b in b"".iter() { buf[p] = b; p += 1; }
        for b in u32_to_str(app.scroll_y as u32).bytes() { if p < 31 { buf[p] = b; p += 1; } }
        if p < 31 { buf[p] = b'/'; p += 1; }
        for b in u32_to_str(max_scroll as u32).bytes() { if p < 31 { buf[p] = b; p += 1; } }
        let s = core::str::from_utf8(&buf[..p]).unwrap_or("Scroll");
        gfx.draw_text(cx + 6, sy, s, 0x606060, 0xFFFFFF);
    }
}



pub fn handle_pcinf_click(
    app: &mut PcInfoApp,
    cx: usize,
    cy: usize,
    w: usize,
    h: usize,
    mx: usize,
    my: usize,
) -> bool {
    let _ = (cx, cy, w, h);

    let arrow_size = 26usize;
    let arrow_y = cy + h - arrow_size - 4;
    let arrow_up_x = cx + w - arrow_size * 2 - 12;
    let arrow_dn_x = cx + w - arrow_size - 8;

    if mx >= arrow_up_x && mx < arrow_up_x + arrow_size
        && my >= arrow_y && my < arrow_y + arrow_size
    {
        app.scroll_up();
        return true;
    }
    if mx >= arrow_dn_x && mx < arrow_dn_x + arrow_size
        && my >= arrow_y && my < arrow_y + arrow_size
    {
        app.scroll_down();
        return true;
    }
    false
}



pub fn handle_pcinf_keyboard(app: &mut PcInfoApp, scancode: u8) -> bool {
    if scancode & 0x80 != 0 { return false; }
    match scancode {
        0x48 => { app.scroll_up(); true }    
        0x50 => { app.scroll_down(); true }  
        _ => false,
    }
}