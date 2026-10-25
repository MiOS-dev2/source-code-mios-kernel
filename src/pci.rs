
fn outl(port: u16, val: u32) {
    unsafe { core::arch::asm!("out dx, eax", in("dx") port, in("eax") val, options(nomem, nostack)) }
}
fn inl(port: u16) -> u32 {
    let v: u32;
    unsafe { core::arch::asm!("in eax, dx", in("dx") port, out("eax") v, options(nomem, nostack)) }
    v
}

#[derive(Clone, Copy, Debug)]
pub struct PciDev {
    pub bus: u8,
    pub dev: u8,
    pub func: u8,
    pub vendor: u16,
    pub device: u16,
    pub class: u8,
    pub subclass: u8,
}

fn addr(bus: u8, dev: u8, func: u8, off: u8) -> u32 {
    0x8000_0000 | ((bus as u32) << 16) | ((dev as u32) << 11) | ((func as u32) << 8) | ((off as u32) & 0xFC)
}

pub fn read32(d: &PciDev, off: u8) -> u32 {
    outl(0xCF8, addr(d.bus, d.dev, d.func, off));
    inl(0xCFC)
}
pub fn write32(d: &PciDev, off: u8, v: u32) {
    outl(0xCF8, addr(d.bus, d.dev, d.func, off));
    outl(0xCFC, v);
}

fn read_raw(bus: u8, dev: u8, func: u8, off: u8) -> u32 {
    outl(0xCF8, addr(bus, dev, func, off));
    inl(0xCFC)
}

pub fn enable(d: &PciDev) {
    let cmd = read32(d, 0x04);
    write32(d, 0x04, (cmd & 0xFFFF_0000) | (cmd & 0xFFFF) | 0x07);
}

pub fn bar(d: &PciDev, idx: u8) -> (u64, bool) {
    let off = 0x10 + idx * 4;
    let v = read32(d, off);
    if v & 1 == 1 {
        ((v & 0xFFFF_FFFC) as u64, true)
    } else {
        let typ = (v >> 1) & 3;
        let mut a = (v & 0xFFFF_FFF0) as u64;
        if typ == 2 {
            let hi = read32(d, off + 4) as u64;
            a |= hi << 32;
        }
        (a, false)
    }
}

pub fn scan<F: FnMut(PciDev)>(mut f: F) {
    for bus in 0..=255u16 {
        for dev in 0..32u8 {
            let v = read_raw(bus as u8, dev, 0, 0);
            if v & 0xFFFF == 0xFFFF { continue; }
            let hdr = (read_raw(bus as u8, dev, 0, 0x0C) >> 16) & 0xFF;
            let funcs = if hdr & 0x80 != 0 { 8 } else { 1 };
            for func in 0..funcs {
                let v = read_raw(bus as u8, dev, func, 0);
                if v & 0xFFFF == 0xFFFF { continue; }
                let cls = read_raw(bus as u8, dev, func, 0x08);
                f(PciDev {
                    bus: bus as u8, dev, func,
                    vendor: (v & 0xFFFF) as u16,
                    device: (v >> 16) as u16,
                    class: (cls >> 24) as u8,
                    subclass: ((cls >> 16) & 0xFF) as u8,
                });
            }
        }
    }
}
