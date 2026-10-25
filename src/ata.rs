#![allow(dead_code)]
#![allow(non_snake_case)]
 
use crate::vga::VGA;
 
const ATA_PRIMARY_DATA: u16 = 0x1F0;
const ATA_PRIMARY_SECTOR_COUNT: u16 = 0x1F2;
const ATA_PRIMARY_LBA_LOW: u16 = 0x1F3;
const ATA_PRIMARY_LBA_MID: u16 = 0x1F4;
const ATA_PRIMARY_LBA_HIGH: u16 = 0x1F5;
const ATA_PRIMARY_DRIVE: u16 = 0x1F6;
const ATA_PRIMARY_COMMAND: u16 = 0x1F7;
const ATA_PRIMARY_STATUS: u16 = 0x1F7;
const ATA_PRIMARY_ALT_STATUS: u16 = 0x3F6;
 
const ATA_CMD_READ_PIO: u8 = 0x20;
const ATA_CMD_WRITE_PIO: u8 = 0x30;
const ATA_CMD_IDENTIFY: u8 = 0xEC;
const ATA_CMD_FLUSH_CACHE: u8 = 0xE7;
 
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum DeviceType {
    None,
    ATA,
    ATAPI,
    Unknown,
}
 
#[derive(Debug, Clone, Copy)]
pub struct DiskInfo {
    pub present: bool,
    pub channel: u8,
    pub drive: u8,
    pub dev_type: DeviceType,
    pub sectors: u64,
    pub sector_size: u16,
    pub model: [u8; 40],
    pub serial: [u8; 20],
    pub lba_supported: bool,
    pub lba48_supported: bool,
}
 
impl DiskInfo {
    pub const fn new() -> Self {
        DiskInfo {
            present: false,
            channel: 0,
            drive: 0,
            dev_type: DeviceType::None,
            sectors: 0,
            sector_size: 512,
            model: [0; 40],
            serial: [0; 20],
            lba_supported: false,
            lba48_supported: false,
        }
    }
}
 
#[inline(always)]
fn inb(port: u16) -> u8 {
    let result: u8;
    unsafe {
        core::arch::asm!("in al, dx", in("dx") port, out("al") result, options(nostack));
    }
    result
}
 
#[inline(always)]
fn outb(port: u16, value: u8) {
    unsafe {
        core::arch::asm!("out dx, al", in("dx") port, in("al") value, options(nostack));
    }
}
 
#[inline(always)]
fn inw(port: u16) -> u16 {
    let result: u16;
    unsafe {
        core::arch::asm!("in ax, dx", in("dx") port, out("ax") result, options(nostack));
    }
    result
}
 
#[inline(always)]
fn outw(port: u16, value: u16) {
    unsafe {
        core::arch::asm!("out dx, ax", in("dx") port, in("ax") value, options(nostack));
    }
}
 
#[inline(always)]
fn io_delay() {
    for _ in 0..16 {
        inb(ATA_PRIMARY_ALT_STATUS);
    }
}
 
fn wait_ready(base: u16) -> bool {
    io_delay();
    let mut timeout: u32 = 4_000_000;
    while timeout > 0 {
        let status = inb(base + 7);
        if (status & 0x80) == 0 {
            return true;
        }
        timeout -= 1;
    }
    false
}
 
fn wait_drq(base: u16) -> bool {
    io_delay();
    let mut timeout: u32 = 4_000_000;
    while timeout > 0 {
        let status = inb(base + 7);
        if (status & 0x80) == 0 {
            if (status & 0x21) != 0 {
                return false;
            }
            if (status & 0x08) != 0 {
                return true;
            }
        }
        timeout -= 1;
    }
    false
}
 
fn trim_spaces(buf: &mut [u8]) {
    let mut start = 0;
    let mut end = buf.len();
 
    while start < end && buf[start] == b' ' {
        start += 1;
    }
    while end > start && (buf[end - 1] == b' ' || buf[end - 1] == 0) {
        end -= 1;
    }
 
    if start > 0 {
        for i in 0..(end - start) {
            buf[i] = buf[start + i];
        }
    }
    if end - start < buf.len() {
        buf[end - start] = 0;
    }
}
 
pub struct AtaDrive {
    pub exists: bool,
    pub sectors: u32,
    pub model: [u8; 40],
}
 
impl AtaDrive {
    pub const fn new() -> Self {
        Self {
            exists: false,
            sectors: 0,
            model: [0; 40],
        }
    }
 
    pub fn init(&mut self, vga: &mut VGA) {
        unsafe {
            self.exists = false;
 
            outb(ATA_PRIMARY_DRIVE, 0xA0);
            io_delay();
            let status = inb(ATA_PRIMARY_STATUS);
            if status == 0xFF || status == 0x00 {
                vga.write_string("[ATA] No drive detected\n");
                return;
            }
 
            outb(ATA_PRIMARY_ALT_STATUS, 0x06);
            for _ in 0..100 {
                inb(ATA_PRIMARY_ALT_STATUS);
            }
            outb(ATA_PRIMARY_ALT_STATUS, 0x02);
            for _ in 0..5000 {
                inb(ATA_PRIMARY_ALT_STATUS);
            }
 
            outb(ATA_PRIMARY_DRIVE, 0xA0);
            io_delay();
            let mut timeout: u32 = 5_000_000;
            loop {
                let st = inb(ATA_PRIMARY_ALT_STATUS);
                if (st & 0x80) == 0 {
                    break;
                }
                timeout -= 1;
                if timeout == 0 {
                    vga.write_string("[ATA] Drive not ready\n");
                    return;
                }
            }
 
            let sig_mid = inb(ATA_PRIMARY_LBA_MID);
            let sig_high = inb(ATA_PRIMARY_LBA_HIGH);
            if (sig_mid == 0x14 && sig_high == 0xEB) || (sig_mid == 0x69 && sig_high == 0x96) {
                vga.write_string("[ATA] ATAPI device on primary master, skipped\n");
                return;
            }
 
            outb(ATA_PRIMARY_DRIVE, 0xA0);
            io_delay();
            outb(ATA_PRIMARY_SECTOR_COUNT, 0);
            outb(ATA_PRIMARY_LBA_LOW, 0);
            outb(ATA_PRIMARY_LBA_MID, 0);
            outb(ATA_PRIMARY_LBA_HIGH, 0);
            outb(ATA_PRIMARY_COMMAND, ATA_CMD_IDENTIFY);
            io_delay();
 
            if inb(ATA_PRIMARY_STATUS) == 0 {
                vga.write_string("[ATA] Drive does not support IDENTIFY\n");
                return;
            }
 
            if !wait_drq(ATA_PRIMARY_DATA) {
                vga.write_string("[ATA] Drive not ready\n");
                return;
            }
 
            let mut identify_data = [0u16; 256];
            for i in 0..256 {
                identify_data[i] = inw(ATA_PRIMARY_DATA);
            }
 
            let lba28: u64 = (identify_data[60] as u64) | ((identify_data[61] as u64) << 16);
            let lba48: u64 = (identify_data[100] as u64)
                | ((identify_data[101] as u64) << 16)
                | ((identify_data[102] as u64) << 32)
                | ((identify_data[103] as u64) << 48);
            let total: u64 = if (identify_data[83] & (1 << 10)) != 0 && lba48 != 0 {
                lba48
            } else {
                lba28
            };
            self.sectors = if total > 0xFFFF_FFFF { 0xFFFF_FFFF } else { total as u32 };
 
            if self.sectors == 0 {
                vga.write_string("[ATA] IDENTIFY returned 0 sectors\n");
                return;
            }
            self.exists = true;
 
            for i in 0..20 {
                let word = identify_data[27 + i];
                self.model[i * 2] = (word >> 8) as u8;
                self.model[i * 2 + 1] = word as u8;
            }
            trim_spaces(&mut self.model);
 
            vga.write_string("[ATA] Drive mounted successfully!\n");
            vga.write_string("[ATA] Model: ");
            for i in 0..40 {
                if self.model[i] != 0 && self.model[i] != 0x20 {
                    vga.put_char(self.model[i] as char);
                }
            }
            vga.write_string("\n[ATA] Sectors: ");
            vga.write_number(self.sectors as usize);
            vga.write_string("\n");
        }
    }
 
    pub fn read_sector(&self, lba: u32, buf: &mut [u8; 512]) -> bool {
        unsafe {
            if !self.exists { return false; }
            if lba > 0x0FFF_FFFF { return false; }
 
            if !wait_ready(ATA_PRIMARY_DATA) { return false; }
 
            outb(ATA_PRIMARY_DRIVE, 0xE0 | ((lba >> 24) & 0x0F) as u8);
            if !wait_ready(ATA_PRIMARY_DATA) { return false; }
 
            outb(ATA_PRIMARY_SECTOR_COUNT, 1);
            outb(ATA_PRIMARY_LBA_LOW, (lba & 0xFF) as u8);
            outb(ATA_PRIMARY_LBA_MID, ((lba >> 8) & 0xFF) as u8);
            outb(ATA_PRIMARY_LBA_HIGH, ((lba >> 16) & 0xFF) as u8);
            outb(ATA_PRIMARY_COMMAND, ATA_CMD_READ_PIO);
 
            if !wait_drq(ATA_PRIMARY_DATA) { return false; }
 
            for i in 0..256 {
                let word = inw(ATA_PRIMARY_DATA);
                buf[i * 2] = word as u8;
                buf[i * 2 + 1] = (word >> 8) as u8;
            }
 
            true
        }
    }
 
    pub fn write_sector(&self, lba: u32, buf: &[u8; 512]) -> bool {
        unsafe {
            if !self.exists { return false; }
            if lba > 0x0FFF_FFFF { return false; }
 
            if !wait_ready(ATA_PRIMARY_DATA) { return false; }
 
            outb(ATA_PRIMARY_DRIVE, 0xE0 | ((lba >> 24) & 0x0F) as u8);
            if !wait_ready(ATA_PRIMARY_DATA) { return false; }
 
            outb(ATA_PRIMARY_SECTOR_COUNT, 1);
            outb(ATA_PRIMARY_LBA_LOW, (lba & 0xFF) as u8);
            outb(ATA_PRIMARY_LBA_MID, ((lba >> 8) & 0xFF) as u8);
            outb(ATA_PRIMARY_LBA_HIGH, ((lba >> 16) & 0xFF) as u8);
            outb(ATA_PRIMARY_COMMAND, ATA_CMD_WRITE_PIO);
 
            if !wait_drq(ATA_PRIMARY_DATA) { return false; }
 
            for i in 0..256 {
                let word = (buf[i * 2] as u16) | ((buf[i * 2 + 1] as u16) << 8);
                outw(ATA_PRIMARY_DATA, word);
            }
 
            if !wait_ready(ATA_PRIMARY_DATA) { return false; }
            (inb(ATA_PRIMARY_STATUS) & 0x21) == 0
        }
    }
 
    pub fn flush_cache(&self) -> bool {
        unsafe {
            if !self.exists { return false; }
 
            if !wait_ready(ATA_PRIMARY_DATA) { return false; }
 
            outb(ATA_PRIMARY_DRIVE, 0xE0);
            io_delay();
            outb(ATA_PRIMARY_COMMAND, ATA_CMD_FLUSH_CACHE);
 
            if !wait_ready(ATA_PRIMARY_DATA) { return false; }
            (inb(ATA_PRIMARY_STATUS) & 0x21) == 0
        }
    }
}
 
 
#[inline(always)]
fn outl(port: u16, value: u32) {
    unsafe {
        core::arch::asm!("out dx, eax", in("dx") port, in("eax") value, options(nostack));
    }
}
 
#[inline(always)]
fn inl(port: u16) -> u32 {
    let result: u32;
    unsafe {
        core::arch::asm!("in eax, dx", in("dx") port, out("eax") result, options(nostack));
    }
    result
}
 
fn pci_cfg_addr(bus: u32, dev: u32, func: u32, off: u32) -> u32 {
    0x8000_0000 | (bus << 16) | (dev << 11) | (func << 8) | (off & 0xFC)
}
 
fn pci_read(bus: u32, dev: u32, func: u32, off: u32) -> u32 {
    outl(0xCF8, pci_cfg_addr(bus, dev, func, off));
    inl(0xCFC)
}
 
fn pci_write(bus: u32, dev: u32, func: u32, off: u32, value: u32) {
    outl(0xCF8, pci_cfg_addr(bus, dev, func, off));
    outl(0xCFC, value);
}
 
#[inline(always)]
unsafe fn rd32(addr: usize) -> u32 {
    core::ptr::read_volatile(addr as *const u32)
}
 
#[inline(always)]
unsafe fn wr32(addr: usize, value: u32) {
    core::ptr::write_volatile(addr as *mut u32, value)
}
 
fn udelay(us: u32) {
    for _ in 0..us {
        outb(0x80, 0);
    }
}
 
static mut DBG_BUF: [u8; 2048] = [0; 2048];
static mut DBG_LEN: usize = 0;
 
struct DbgW;
 
impl core::fmt::Write for DbgW {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        unsafe {
            let bytes = s.as_bytes();
            let mut i = 0usize;
            while i < bytes.len() {
                let mut b = bytes[i];
                if b == b';' {
                    b = b'\n';
                    if i + 1 < bytes.len() && bytes[i + 1] == b' ' {
                        i += 1;
                    }
                }
                if DBG_LEN < 2047 {
                    DBG_BUF[DBG_LEN] = b;
                    DBG_LEN += 1;
                }
                i += 1;
            }
        }
        Ok(())
    }
}
 
macro_rules! dbg_line {
    ($($a:tt)*) => {{
        let _ = core::fmt::write(&mut DbgW, format_args!($($a)*));
    }};
}
 
pub fn debug_info() -> &'static str {
    unsafe { core::str::from_utf8(&DBG_BUF[..DBG_LEN]).unwrap_or("") }
}
 
const MAX_DISKS: usize = 8;
 
#[repr(C, align(1024))]
struct AhciMem {
    cl: [u8; 1024],
    fis: [u8; 256],
    ct: [u8; 256],
    _pad: [u8; 256],
}
 
const AHCI_MEM_ZERO: AhciMem = AhciMem {
    cl: [0; 1024],
    fis: [0; 256],
    ct: [0; 256],
    _pad: [0; 256],
};
 
static mut AHCI_MEM: [AhciMem; MAX_DISKS] = [AHCI_MEM_ZERO; MAX_DISKS];
 
#[repr(C, align(512))]
struct AhciBounce([u8; 512]);
 
static mut AHCI_BOUNCE: AhciBounce = AhciBounce([0; 512]);
 
#[derive(Clone, Copy)]
struct Entry {
    kind: u8,
    port: usize,
    mem: usize,
    info: DiskInfo,
}
 
impl Entry {
    const fn none() -> Self {
        Entry { kind: 0, port: 0, mem: 0, info: DiskInfo::new() }
    }
}
 
static mut TABLE: [Entry; MAX_DISKS] = [Entry::none(); MAX_DISKS];
static mut TABLE_COUNT: usize = 0;
 
const PX_CLB: usize = 0x00;
const PX_CLBU: usize = 0x04;
const PX_FB: usize = 0x08;
const PX_FBU: usize = 0x0C;
const PX_IS: usize = 0x10;
const PX_CMD: usize = 0x18;
const PX_TFD: usize = 0x20;
const PX_SIG: usize = 0x24;
const PX_SSTS: usize = 0x28;
const PX_SCTL: usize = 0x2C;
const PX_SERR: usize = 0x30;
const PX_CI: usize = 0x38;
 
unsafe fn ahci_port_stop(pb: usize) -> bool {
    wr32(pb + PX_CMD, rd32(pb + PX_CMD) & !1);
    let mut n = 0u32;
    while rd32(pb + PX_CMD) & (1 << 15) != 0 {
        udelay(100);
        n += 1;
        if n > 5000 { return false; }
    }
    wr32(pb + PX_CMD, rd32(pb + PX_CMD) & !(1 << 4));
    n = 0;
    while rd32(pb + PX_CMD) & (1 << 14) != 0 {
        udelay(100);
        n += 1;
        if n > 5000 { return false; }
    }
    true
}
 
unsafe fn ahci_port_start(pb: usize) {
    wr32(pb + PX_CMD, rd32(pb + PX_CMD) | (1 << 4));
    wr32(pb + PX_CMD, rd32(pb + PX_CMD) | 1);
}
 
unsafe fn ahci_wait_idle(pb: usize) -> bool {
    let mut n = 0u32;
    while n < 1_000_000 {
        if rd32(pb + PX_TFD) & 0x88 == 0 {
            return true;
        }
        udelay(1);
        n += 1;
    }
    false
}
 
unsafe fn ahci_wait_idle_ms(pb: usize, ms: u32) -> bool {
    let mut n = 0u32;
    while n < ms {
        if rd32(pb + PX_TFD) & 0x88 == 0 {
            return true;
        }
        udelay(1000);
        n += 1;
    }
    false
}
 
unsafe fn ahci_comreset(pb: usize) {
    let sctl = rd32(pb + PX_SCTL);
    wr32(pb + PX_SCTL, (sctl & !0xF) | 1);
    udelay(5000);
    wr32(pb + PX_SCTL, sctl & !0xF);
    let mut n = 0u32;
    while n < 2000 {
        if rd32(pb + PX_SSTS) & 0xF == 3 { break; }
        udelay(1000);
        n += 1;
    }
}
 
unsafe fn ahci_recover(pb: usize) {
    let _ = ahci_port_stop(pb);
    wr32(pb + PX_SERR, 0xFFFF_FFFF);
    wr32(pb + PX_IS, 0xFFFF_FFFF);
    ahci_port_start(pb);
}
 
unsafe fn ahci_exec(pb: usize, mem: usize, command: u8, device: u8, lba: u64, count: u16, data: bool, write: bool) -> bool {
    if !ahci_wait_idle(pb) {
        return false;
    }
 
    let m = core::ptr::addr_of_mut!(AHCI_MEM[mem]);
    let cl = core::ptr::addr_of_mut!((*m).cl) as *mut u8;
    let ct = core::ptr::addr_of_mut!((*m).ct) as *mut u8;
 
    for i in 0..0x90usize {
        core::ptr::write_volatile(ct.add(i), 0);
    }
 
    core::ptr::write_volatile(ct.add(0), 0x27);
    core::ptr::write_volatile(ct.add(1), 0x80);
    core::ptr::write_volatile(ct.add(2), command);
    core::ptr::write_volatile(ct.add(4), (lba & 0xFF) as u8);
    core::ptr::write_volatile(ct.add(5), ((lba >> 8) & 0xFF) as u8);
    core::ptr::write_volatile(ct.add(6), ((lba >> 16) & 0xFF) as u8);
    core::ptr::write_volatile(ct.add(7), device);
    core::ptr::write_volatile(ct.add(8), ((lba >> 24) & 0xFF) as u8);
    core::ptr::write_volatile(ct.add(9), ((lba >> 32) & 0xFF) as u8);
    core::ptr::write_volatile(ct.add(10), ((lba >> 40) & 0xFF) as u8);
    core::ptr::write_volatile(ct.add(12), (count & 0xFF) as u8);
    core::ptr::write_volatile(ct.add(13), (count >> 8) as u8);
 
    if data {
        let b = core::ptr::addr_of_mut!(AHCI_BOUNCE) as usize;
        let prdt = ct.add(0x80) as *mut u32;
        core::ptr::write_volatile(prdt.add(0), b as u32);
        core::ptr::write_volatile(prdt.add(1), ((b as u64) >> 32) as u32);
        core::ptr::write_volatile(prdt.add(2), 0);
        core::ptr::write_volatile(prdt.add(3), 511);
    }
 
    let h = cl as *mut u32;
    let flags: u32 = 5 | (if write { 1 << 6 } else { 0 }) | ((if data { 1u32 } else { 0 }) << 16);
    let ctp = ct as usize;
    core::ptr::write_volatile(h.add(0), flags);
    core::ptr::write_volatile(h.add(1), 0);
    core::ptr::write_volatile(h.add(2), ctp as u32);
    core::ptr::write_volatile(h.add(3), ((ctp as u64) >> 32) as u32);
    for i in 4..8usize {
        core::ptr::write_volatile(h.add(i), 0);
    }
 
    core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
    wr32(pb + PX_IS, 0xFFFF_FFFF);
    wr32(pb + PX_CI, 1);
 
    let mut n = 0u32;
    loop {
        if rd32(pb + PX_CI) & 1 == 0 {
            break;
        }
        if rd32(pb + PX_IS) & (1 << 30) != 0 {
            ahci_recover(pb);
            return false;
        }
        udelay(1);
        n += 1;
        if n > 3_000_000 {
            ahci_recover(pb);
            return false;
        }
    }
    core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
 
    if rd32(pb + PX_TFD) & 0x21 != 0 {
        ahci_recover(pb);
        return false;
    }
    true
}
 
unsafe fn ahci_init_port(pb: usize, port_no: u32, s64: bool, mem: usize) -> Option<DiskInfo> {
    let ssts = rd32(pb + PX_SSTS);
    let det = ssts & 0xF;
    let ipm = (ssts >> 8) & 0xF;
    if det == 0 {
        let _ = ahci_port_stop(pb);
        wr32(pb + PX_CMD, rd32(pb + PX_CMD) | (1 << 1) | (1 << 2));
        let mut n = 0u32;
        while rd32(pb + PX_SSTS) & 0xF == 0 && n < 300 {
            udelay(1000);
            n += 1;
        }
        if rd32(pb + PX_SSTS) & 0xF == 0 {
            return None;
        }
    }
 
    if det == 3 && ipm != 1 {
        let cmd = rd32(pb + PX_CMD);
        wr32(pb + PX_CMD, (cmd & 0x0FFF_FFFF) | (1 << 28));
        let mut n = 0u32;
        while rd32(pb + PX_CMD) >> 28 != 0 && n < 1000 {
            udelay(10);
            n += 1;
        }
    }
 
    if !ahci_port_stop(pb) {
        dbg_line!("port{}: cannot stop; ", port_no);
        return None;
    }
 
    let clb = core::ptr::addr_of_mut!(AHCI_MEM[mem].cl) as usize as u64;
    let fb = core::ptr::addr_of_mut!(AHCI_MEM[mem].fis) as usize as u64;
    if !s64 && ((clb >> 32) != 0 || (fb >> 32) != 0) {
        dbg_line!("port{}: DMA memory above 4GB; ", port_no);
        return None;
    }
    wr32(pb + PX_CLB, clb as u32);
    wr32(pb + PX_CLBU, (clb >> 32) as u32);
    wr32(pb + PX_FB, fb as u32);
    wr32(pb + PX_FBU, (fb >> 32) as u32);
    wr32(pb + PX_SERR, 0xFFFF_FFFF);
    wr32(pb + PX_IS, 0xFFFF_FFFF);
 
    wr32(pb + PX_CMD, rd32(pb + PX_CMD) | (1 << 1) | (1 << 2));
    wr32(pb + PX_CMD, rd32(pb + PX_CMD) | (1 << 4));
 
    if rd32(pb + PX_SSTS) & 0xF != 3 {
        let mut n = 0u32;
        while rd32(pb + PX_SSTS) & 0xF != 3 && n < 500 {
            udelay(1000);
            n += 1;
        }
        if rd32(pb + PX_SSTS) & 0xF != 3 {
            ahci_comreset(pb);
        }
        if rd32(pb + PX_SSTS) & 0xF != 3 {
            dbg_line!("port{}: no link (ssts {:x}); ", port_no, rd32(pb + PX_SSTS));
            let _ = ahci_port_stop(pb);
            return None;
        }
        wr32(pb + PX_SERR, 0xFFFF_FFFF);
    }
 
    if !ahci_wait_idle(pb) {
        let _ = ahci_port_stop(pb);
        wr32(pb + PX_CMD, rd32(pb + PX_CMD) | (1 << 4));
        ahci_comreset(pb);
        wr32(pb + PX_SERR, 0xFFFF_FFFF);
        if !ahci_wait_idle_ms(pb, 5000) {
            dbg_line!("port{}: device busy; ", port_no);
            let _ = ahci_port_stop(pb);
            return None;
        }
    }
 
    let sig = rd32(pb + PX_SIG);
    if sig == 0xEB14_0101 || sig == 0xC33C_0101 || sig == 0x9669_0101 {
        dbg_line!("port{}: not a hard disk (sig {:08x}); ", port_no, sig);
        let _ = ahci_port_stop(pb);
        return None;
    }
 
    ahci_port_start(pb);
 
    if !ahci_exec(pb, mem, ATA_CMD_IDENTIFY, 0, 0, 0, true, false) {
        dbg_line!("port{}: IDENTIFY failed (sig {:08x}); ", port_no, sig);
        let _ = ahci_port_stop(pb);
        return None;
    }
 
    let mut w = [0u16; 256];
    let bp = core::ptr::addr_of!(AHCI_BOUNCE) as *const u16;
    for i in 0..256usize {
        w[i] = core::ptr::read_volatile(bp.add(i));
    }
 
    if (w[106] & 0xC000) == 0x4000 && (w[106] & (1 << 12)) != 0 {
        let sz = (w[117] as u32) | ((w[118] as u32) << 16);
        if sz != 0 && sz != 256 {
            dbg_line!("port{}: sector size {} not supported; ", port_no, sz * 2);
            let _ = ahci_port_stop(pb);
            return None;
        }
    }
 
    let lba28: u64 = (w[60] as u64) | ((w[61] as u64) << 16);
    let lba48: u64 = (w[100] as u64) | ((w[101] as u64) << 16) | ((w[102] as u64) << 32) | ((w[103] as u64) << 48);
    let total = if (w[83] & (1 << 10)) != 0 && lba48 != 0 { lba48 } else { lba28 };
    if total == 0 {
        dbg_line!("port{}: 0 sectors; ", port_no);
        let _ = ahci_port_stop(pb);
        return None;
    }
 
    let mut info = DiskInfo::new();
    info.present = true;
    info.channel = 2;
    info.drive = port_no as u8;
    info.dev_type = DeviceType::ATA;
    info.sectors = total;
    info.sector_size = 512;
    info.lba_supported = true;
    info.lba48_supported = true;
    for i in 0..20usize {
        let word = w[27 + i];
        info.model[i * 2] = (word >> 8) as u8;
        info.model[i * 2 + 1] = word as u8;
    }
    trim_spaces(&mut info.model);
    dbg_line!("port{}: disk {} sectors; ", port_no, total);
    Some(info)
}
 
unsafe fn map_uncached(addr: usize) {
    let gb = addr >> 30;
    if gb > 3 {
        return;
    }
    let pdt: usize = match gb { 0 => 0x12000, 1 => 0x14000, 2 => 0x15000, _ => 0x16000 };
    let pdpte = core::ptr::read_volatile((0x11000 + gb * 8) as *const u64);
    if (pdpte & 1) == 0 || ((pdpte & 0x000F_FFFF_FFFF_F000) as usize) != pdt {
        return;
    }
    let e = pdt + ((addr >> 21) & 0x1FF) * 8;
    let v = core::ptr::read_volatile(e as *const u64);
    if (v & 1) == 0 || (v & 0x80) == 0 || (v & 0x18) == 0x18 {
        return;
    }
    core::ptr::write_volatile(e as *mut u64, v | 0x18);
    core::arch::asm!("mov {0}, cr3", "mov cr3, {0}", out(reg) _, options(nostack));
}
 
unsafe fn ahci_init_controller(abar: usize) {
    let cap = rd32(abar);
    let s64 = cap & (1 << 31) != 0;
 
    let cap2 = rd32(abar + 0x24);
    if cap2 & 1 != 0 {
        let bohc = rd32(abar + 0x28);
        if bohc & 1 != 0 {
            wr32(abar + 0x28, bohc | 2);
            let mut n = 0u32;
            while rd32(abar + 0x28) & 1 != 0 && n < 2500 {
                udelay(10);
                n += 1;
            }
        }
    }
 
    let ghc = rd32(abar + 0x04);
    wr32(abar + 0x04, (ghc | 0x8000_0000) & !2);
 
    let mut pi = rd32(abar + 0x0C);
    if pi == 0 {
        let np = (cap & 0x1F) + 1;
        pi = if np >= 32 { 0xFFFF_FFFF } else { (1u32 << np) - 1 };
    }
    dbg_line!("cap={:#x} pi={:#x} ghc={:#x}; ", cap, pi, rd32(abar + 0x04));
    for p in 0..32u32 {
        if pi & (1 << p) == 0 { continue; }
        if TABLE_COUNT >= MAX_DISKS { return; }
        let pb = abar + 0x100 + (p as usize) * 0x80;
        let mem = TABLE_COUNT;
        dbg_line!("p{} ssts={:x} sig={:08x}; ", p, rd32(pb + PX_SSTS), rd32(pb + PX_SIG));
        if let Some(info) = ahci_init_port(pb, p, s64, mem) {
            TABLE[TABLE_COUNT] = Entry { kind: 2, port: pb, mem, info };
            TABLE_COUNT += 1;
        }
    }
}
 
unsafe fn ahci_scan() {
    for bus in 0..256u32 {
        for dev in 0..32u32 {
            if pci_read(bus, dev, 0, 0) & 0xFFFF == 0xFFFF {
                continue;
            }
            let multi = (pci_read(bus, dev, 0, 0x0C) >> 16) & 0x80 != 0;
            let nfunc = if multi { 8 } else { 1 };
            for func in 0..nfunc {
                let id = pci_read(bus, dev, func, 0);
                if id & 0xFFFF == 0xFFFF { continue; }
                let cls = pci_read(bus, dev, func, 8);
                let class = cls >> 24;
                let sub = (cls >> 16) & 0xFF;
                let pif = (cls >> 8) & 0xFF;
                if class == 1 {
                    dbg_line!("pci {:02x}:{:02x}.{} {:04x}:{:04x} cls {:02x}{:02x}{:02x}; ", bus, dev, func, id & 0xFFFF, id >> 16, class, sub, pif);
                }
                if class == 1 && sub == 1 {
                    ide_scan_pci(bus, dev, func, pif);
                }
                let is_ahci = class == 1 && (sub == 6 || sub == 4);
                if !is_ahci { continue; }
 
                let bar5 = pci_read(bus, dev, func, 0x24);
                if bar5 & 1 != 0 {
                    dbg_line!("{:02x}:{:02x}.{} id={:08x}: BAR5 is I/O; ", bus, dev, func, id);
                    continue;
                }
                if (bar5 & 0x6) == 0x4 && pci_read(bus, dev, func, 0x28) != 0 {
                    dbg_line!("{:02x}:{:02x}.{} id={:08x}: BAR5 above 4GB; ", bus, dev, func, id);
                    continue;
                }
                let abar = (bar5 & 0xFFFF_FFF0) as usize;
                if abar == 0 {
                    dbg_line!("{:02x}:{:02x}.{} id={:08x}: BAR5 empty; ", bus, dev, func, id);
                    continue;
                }
 
                let cmd = pci_read(bus, dev, func, 4) & 0xFFFF;
                pci_write(bus, dev, func, 4, cmd | 0x406);
 
                dbg_line!("AHCI {:02x}:{:02x}.{} id={:08x} class={:02x}{:02x} abar={:#x}; ", bus, dev, func, id, class, sub, abar);
                map_uncached(abar);
                map_uncached(abar + 0x1FFF);
                ahci_init_controller(abar);
                if TABLE_COUNT >= MAX_DISKS { return; }
            }
        }
    }
}
 
unsafe fn ahci_read(e: &Entry, lba: u32, buf: &mut [u8; 512]) -> bool {
    if !ahci_exec(e.port, e.mem, 0x25, 0x40, lba as u64, 1, true, false) {
        return false;
    }
    let bp = core::ptr::addr_of!(AHCI_BOUNCE) as *const u8;
    for i in 0..512usize {
        buf[i] = core::ptr::read_volatile(bp.add(i));
    }
    true
}
 
unsafe fn ahci_write(e: &Entry, lba: u32, buf: &[u8; 512]) -> bool {
    let bp = core::ptr::addr_of_mut!(AHCI_BOUNCE) as *mut u8;
    for i in 0..512usize {
        core::ptr::write_volatile(bp.add(i), buf[i]);
    }
    ahci_exec(e.port, e.mem, 0x35, 0x40, lba as u64, 1, true, true)
}
 
 
unsafe fn ide_wait_bsy(base: u16) -> bool {
    let mut n = 0u32;
    while n < 1_000_000 {
        if inb(base + 7) & 0x80 == 0 {
            return true;
        }
        n += 1;
    }
    false
}
 
unsafe fn ide_wait_drq(base: u16) -> bool {
    let mut n = 0u32;
    while n < 1_000_000 {
        let st = inb(base + 7);
        if st & 0x80 == 0 {
            if st & 0x21 != 0 {
                return false;
            }
            if st & 0x08 != 0 {
                return true;
            }
        }
        n += 1;
    }
    false
}
 
unsafe fn ide_select(base: u16, ctrl: u16, sel: u8) {
    outb(base + 6, sel);
    for _ in 0..16 {
        inb(ctrl);
    }
}
 
unsafe fn ide_probe(base: u16, ctrl: u16, slave: u8) -> Option<DiskInfo> {
    outb(ctrl, 0x02);
    ide_select(base, ctrl, 0xA0 | (slave << 4));
    let st = inb(base + 7);
    dbg_line!("ide {:#x}/{} st={:02x}; ", base, slave, st);
    if st == 0xFF || st == 0x00 {
        return None;
    }
    if !ide_wait_bsy(base) {
        return None;
    }
    let mid = inb(base + 4);
    let high = inb(base + 5);
    if (mid == 0x14 && high == 0xEB) || (mid == 0x69 && high == 0x96) {
        return None;
    }
 
    outb(base + 2, 0);
    outb(base + 3, 0);
    outb(base + 4, 0);
    outb(base + 5, 0);
    outb(base + 7, ATA_CMD_IDENTIFY);
    for _ in 0..16 {
        inb(ctrl);
    }
    if inb(base + 7) == 0 {
        return None;
    }
    if !ide_wait_bsy(base) {
        return None;
    }
    if !ide_wait_drq(base) {
        dbg_line!("ide {:#x}/{}: IDENTIFY failed; ", base, slave);
        return None;
    }
 
    let mut w = [0u16; 256];
    for i in 0..256usize {
        w[i] = inw(base);
    }
    let lba28: u64 = (w[60] as u64) | ((w[61] as u64) << 16);
    let lba48: u64 = (w[100] as u64) | ((w[101] as u64) << 16) | ((w[102] as u64) << 32) | ((w[103] as u64) << 48);
    let total = if (w[83] & (1 << 10)) != 0 && lba48 != 0 { lba48 } else { lba28 };
    if total == 0 || lba28 == 0 {
        return None;
    }
 
    let mut info = DiskInfo::new();
    info.present = true;
    info.channel = if base == 0x170 { 1 } else if base == 0x1F0 { 0 } else { 2 };
    info.drive = slave;
    info.dev_type = DeviceType::ATA;
    info.sectors = total;
    info.sector_size = 512;
    info.lba_supported = true;
    info.lba48_supported = false;
    for i in 0..20usize {
        let word = w[27 + i];
        info.model[i * 2] = (word >> 8) as u8;
        info.model[i * 2 + 1] = word as u8;
    }
    trim_spaces(&mut info.model);
    for i in 0..10usize {
        let word = w[10 + i];
        info.serial[i * 2] = (word >> 8) as u8;
        info.serial[i * 2 + 1] = word as u8;
    }
    Some(info)
}
 
unsafe fn ide_read(base: u16, ctrl: u16, slave: u8, lba: u32, buf: &mut [u8; 512]) -> bool {
    if lba > 0x0FFF_FFFF {
        return false;
    }
    if !ide_wait_bsy(base) {
        return false;
    }
    ide_select(base, ctrl, 0xE0 | (slave << 4) | (((lba >> 24) & 0x0F) as u8));
    if !ide_wait_bsy(base) {
        return false;
    }
    outb(base + 2, 1);
    outb(base + 3, (lba & 0xFF) as u8);
    outb(base + 4, ((lba >> 8) & 0xFF) as u8);
    outb(base + 5, ((lba >> 16) & 0xFF) as u8);
    outb(base + 7, ATA_CMD_READ_PIO);
    if !ide_wait_drq(base) {
        return false;
    }
    for i in 0..256usize {
        let word = inw(base);
        buf[i * 2] = word as u8;
        buf[i * 2 + 1] = (word >> 8) as u8;
    }
    true
}
 
unsafe fn ide_write(base: u16, ctrl: u16, slave: u8, lba: u32, buf: &[u8; 512]) -> bool {
    if lba > 0x0FFF_FFFF {
        return false;
    }
    if !ide_wait_bsy(base) {
        return false;
    }
    ide_select(base, ctrl, 0xE0 | (slave << 4) | (((lba >> 24) & 0x0F) as u8));
    if !ide_wait_bsy(base) {
        return false;
    }
    outb(base + 2, 1);
    outb(base + 3, (lba & 0xFF) as u8);
    outb(base + 4, ((lba >> 8) & 0xFF) as u8);
    outb(base + 5, ((lba >> 16) & 0xFF) as u8);
    outb(base + 7, ATA_CMD_WRITE_PIO);
    if !ide_wait_drq(base) {
        return false;
    }
    for i in 0..256usize {
        let word = (buf[i * 2] as u16) | ((buf[i * 2 + 1] as u16) << 8);
        outw(base, word);
    }
    if !ide_wait_bsy(base) {
        return false;
    }
    (inb(base + 7) & 0x21) == 0
}
 
unsafe fn ide_flush(base: u16, ctrl: u16, slave: u8) -> bool {
    if !ide_wait_bsy(base) {
        return false;
    }
    ide_select(base, ctrl, 0xE0 | (slave << 4));
    outb(base + 7, ATA_CMD_FLUSH_CACHE);
    if !ide_wait_bsy(base) {
        return false;
    }
    (inb(base + 7) & 0x21) == 0
}
 
unsafe fn ide_is_duplicate(info: &DiskInfo) -> bool {
    for i in 0..TABLE_COUNT {
        let o = TABLE[i].info;
        if o.sectors != info.sectors || o.model != info.model {
            continue;
        }
        let o_zero = o.serial.iter().all(|&b| b == 0);
        if o_zero || o.serial == info.serial {
            return true;
        }
    }
    false
}
 
unsafe fn ide_add(base: u16, ctrl: u16, slave: u8) {
    if TABLE_COUNT >= MAX_DISKS {
        return;
    }
    if let Some(info) = ide_probe(base, ctrl, slave) {
        if ide_is_duplicate(&info) {
            return;
        }
        dbg_line!("ide {:#x}/{}: disk {} sectors; ", base, slave, info.sectors);
        TABLE[TABLE_COUNT] = Entry { kind: 3, port: (base as usize) | ((ctrl as usize) << 16), mem: slave as usize, info };
        TABLE_COUNT += 1;
    }
}
 
unsafe fn ide_scan_extra() {
    ide_add(0x1F0, 0x3F6, 1);
    ide_add(0x170, 0x376, 0);
    ide_add(0x170, 0x376, 1);
}
 
unsafe fn ide_scan_pci(bus: u32, dev: u32, func: u32, pif: u32) {
    let cmd = pci_read(bus, dev, func, 4) & 0xFFFF;
    pci_write(bus, dev, func, 4, cmd | 0x05);
    for ch in 0..2u32 {
        if pif & (1 << (ch * 2)) == 0 {
            continue;
        }
        let b0 = pci_read(bus, dev, func, 0x10 + ch * 8);
        let b1 = pci_read(bus, dev, func, 0x14 + ch * 8);
        if (b0 & 1) == 0 || (b1 & 1) == 0 {
            dbg_line!("ide-native ch{}: BAR is not I/O; ", ch);
            continue;
        }
        let base = (b0 & 0xFFFC) as u16;
        let ctrl = ((b1 & 0xFFFC) as u16).wrapping_add(2);
        if base == 0 || base == 0x1F0 || base == 0x170 {
            continue;
        }
        dbg_line!("ide-native {:02x}:{:02x}.{} ch{} io={:#x}/{:#x}; ", bus, dev, func, ch, base, ctrl);
        ide_add(base, ctrl, 0);
        ide_add(base, ctrl, 1);
    }
}
 
 
static mut CACHED_DRIVE: Option<AtaDrive> = None;
static mut CACHE_READY: bool = false;
static mut INIT_TRIES: u8 = 0;
 
unsafe fn ensure_cache() -> bool {
    if TABLE_COUNT > 0 {
        return true;
    }
    if INIT_TRIES >= 3 {
        return false;
    }
    INIT_TRIES += 1;
    DBG_LEN = 0;
 
    outb(ATA_PRIMARY_DRIVE, 0xA0);
    io_delay();
    dbg_line!("ide 0x1f0/0 st={:02x}; ", inb(ATA_PRIMARY_STATUS));
    let mut drive = AtaDrive::new();
    let mut vga = VGA::new();
    drive.init(&mut vga);
    if drive.exists {
        let mut disk = DiskInfo::new();
        disk.present = true;
        disk.channel = 0;
        disk.drive = 0;
        disk.dev_type = DeviceType::ATA;
        disk.sectors = drive.sectors as u64;
        disk.sector_size = 512;
        disk.model = drive.model;
        disk.lba_supported = true;
        disk.lba48_supported = false;
        TABLE[TABLE_COUNT] = Entry { kind: 1, port: 0, mem: 0, info: disk };
        TABLE_COUNT += 1;
        CACHED_DRIVE = Some(drive);
    }
 
    ahci_scan();
 
    ide_scan_extra();
 
    if TABLE_COUNT > 0 {
        CACHE_READY = true;
        true
    } else {
        if INIT_TRIES >= 3 {
            CACHE_READY = true;
        }
        false
    }
}
 
pub unsafe fn read_sector(disk_index: usize, sector: u32, buffer: &mut [u8; 512]) -> bool {
    if !ensure_cache() {
        return false;
    }
    if disk_index >= TABLE_COUNT {
        return false;
    }
    let e = TABLE[disk_index];
    match e.kind {
        1 => {
            if let Some(ref drive) = CACHED_DRIVE {
                drive.read_sector(sector, buffer)
            } else {
                false
            }
        }
        2 => ahci_read(&e, sector, buffer),
        3 => ide_read((e.port & 0xFFFF) as u16, ((e.port >> 16) & 0xFFFF) as u16, e.mem as u8, sector, buffer),
        _ => false,
    }
}
 
pub unsafe fn write_sector(disk_index: usize, sector: u32, buffer: &[u8; 512]) -> bool {
    if !ensure_cache() {
        return false;
    }
    if disk_index >= TABLE_COUNT {
        return false;
    }
    let e = TABLE[disk_index];
    match e.kind {
        1 => {
            if let Some(ref drive) = CACHED_DRIVE {
                drive.write_sector(sector, buffer)
            } else {
                false
            }
        }
        2 => ahci_write(&e, sector, buffer),
        3 => ide_write((e.port & 0xFFFF) as u16, ((e.port >> 16) & 0xFFFF) as u16, e.mem as u8, sector, buffer),
        _ => false,
    }
}
 
pub unsafe fn flush(disk_index: usize) -> bool {
    if !ensure_cache() {
        return false;
    }
    if disk_index >= TABLE_COUNT {
        return false;
    }
    let e = TABLE[disk_index];
    match e.kind {
        1 => {
            if let Some(ref drive) = CACHED_DRIVE {
                drive.flush_cache()
            } else {
                false
            }
        }
        2 => ahci_exec(e.port, e.mem, 0xEA, 0x40, 0, 0, false, false),
        3 => ide_flush((e.port & 0xFFFF) as u16, ((e.port >> 16) & 0xFFFF) as u16, e.mem as u8),
        _ => false,
    }
}
 
pub unsafe fn disk_count() -> usize {
    if ensure_cache() { TABLE_COUNT } else { 0 }
}
 
pub unsafe fn get_disk(index: usize) -> Option<DiskInfo> {
    if !ensure_cache() { return None; }
    if index >= TABLE_COUNT { return None; }
    Some(TABLE[index].info)
}
 
pub fn is_initialized() -> bool {
    unsafe { CACHE_READY }
}