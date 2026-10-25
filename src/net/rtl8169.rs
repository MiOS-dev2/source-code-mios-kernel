use crate::pci::{self, PciDev};
use core::ptr::{read_volatile, write_volatile};

const NUM_RX: usize = 64;
const NUM_TX: usize = 32;
const BUF: usize = 2048;
const OWN: u32 = 1 << 31;
const EOR: u32 = 1 << 30;
const FS: u32 = 1 << 29;
const LS: u32 = 1 << 28;

#[repr(C, align(256))]
#[derive(Clone, Copy)]
struct Desc { opts1: u32, opts2: u32, addr: u64 }

#[repr(C, align(256))]
struct Rings { rx: [Desc; NUM_RX], tx: [Desc; NUM_TX] }
#[repr(C, align(4096))]
struct Bufs { rx: [[u8; BUF]; NUM_RX], tx: [[u8; BUF]; NUM_TX] }

static mut RINGS: Rings = Rings { rx: [Desc { opts1: 0, opts2: 0, addr: 0 }; NUM_RX], tx: [Desc { opts1: 0, opts2: 0, addr: 0 }; NUM_TX] };
static mut BUFS: Bufs = Bufs { rx: [[0; BUF]; NUM_RX], tx: [[0; BUF]; NUM_TX] };

pub struct Rtl8169 { mmio: u64, rx_cur: usize, tx_cur: usize, pub mac: [u8; 6] }

pub fn supported(d: &PciDev) -> bool {
    d.vendor == 0x10EC && matches!(d.device, 0x8161 | 0x8167 | 0x8168 | 0x8169 | 0x8136 | 0x2502 | 0x2600)
}

impl Rtl8169 {
    fn r8(&self, o: u64) -> u8 { unsafe { read_volatile((self.mmio + o) as *const u8) } }
    fn w8(&self, o: u64, v: u8) { unsafe { write_volatile((self.mmio + o) as *mut u8, v) } }
    fn w16(&self, o: u64, v: u16) { unsafe { write_volatile((self.mmio + o) as *mut u16, v) } }
    fn r32(&self, o: u64) -> u32 { unsafe { read_volatile((self.mmio + o) as *const u32) } }
    fn w32(&self, o: u64, v: u32) { unsafe { write_volatile((self.mmio + o) as *mut u32, v) } }

    pub fn new(d: &PciDev) -> Option<Self> {
        pci::enable(d);
        let mut mmio = 0u64;
        for i in [2u8, 1, 0] {
            let (a, io) = pci::bar(d, i);
            if !io && a != 0 && a < 0x1_0000_0000 { mmio = a; break; }
        }
        if mmio == 0 { return None; }
        let mut n = Rtl8169 { mmio, rx_cur: 0, tx_cur: 0, mac: [0; 6] };
        n.w8(0x37, 0x10);
        for _ in 0..1_000_000 { if n.r8(0x37) & 0x10 == 0 { break; } }
        for i in 0..6 { n.mac[i] = n.r8(i as u64); }
        n.w8(0x50, 0xC0);
        unsafe {
            let r = &raw mut RINGS;
            let b = &raw mut BUFS;
            for i in 0..NUM_RX {
                let mut o = OWN | BUF as u32;
                if i == NUM_RX - 1 { o |= EOR; }
                (*r).rx[i] = Desc { opts1: o, opts2: 0, addr: (*b).rx[i].as_ptr() as u64 };
            }
            for i in 0..NUM_TX {
                let o = if i == NUM_TX - 1 { EOR } else { 0 };
                (*r).tx[i] = Desc { opts1: o, opts2: 0, addr: (*b).tx[i].as_ptr() as u64 };
            }
            let rx = (*r).rx.as_ptr() as u64;
            let tx = (*r).tx.as_ptr() as u64;
            n.w32(0x20, tx as u32); n.w32(0x24, (tx >> 32) as u32);
            n.w32(0xE4, rx as u32); n.w32(0xE8, (rx >> 32) as u32);
        }
        n.w16(0xDA, BUF as u16);
        n.w8(0xEC, 0x3F);
        n.w8(0x37, 0x0C);
        n.w32(0x44, 0x0000_E70F);
        n.w32(0x40, 0x0300_0700);
        n.w16(0x3C, 0);
        n.w8(0x50, 0x00);
        Some(n)
    }

    pub fn link_up(&self) -> bool { self.r8(0x6C) & 0x02 != 0 }
    pub fn speed_mbps(&self) -> u32 {
        let s = self.r8(0x6C);
        if s & 0x10 != 0 { 1000 } else if s & 0x08 != 0 { 100 } else { 10 }
    }

    pub fn recv(&mut self, out: &mut [u8]) -> Option<usize> {
        unsafe {
            let r = &raw mut RINGS;
            let d = read_volatile(&(*r).rx[self.rx_cur]);
            if d.opts1 & OWN != 0 { return None; }
            let len = ((d.opts1 & 0x3FFF) as usize).saturating_sub(4).min(out.len());
            let ok = d.opts1 & (FS | LS) == (FS | LS) && d.opts1 & (1 << 21) == 0;
            if ok {
                let b = &raw const BUFS;
                out[..len].copy_from_slice(&(*b).rx[self.rx_cur][..len]);
            }
            let mut o = OWN | BUF as u32;
            if self.rx_cur == NUM_RX - 1 { o |= EOR; }
            write_volatile(&mut (*r).rx[self.rx_cur], Desc { opts1: o, opts2: 0, addr: d.addr });
            self.rx_cur = (self.rx_cur + 1) % NUM_RX;
            Some(if ok { len } else { 0 })
        }
    }

    pub fn can_send(&self) -> bool {
        unsafe { let r = &raw const RINGS; read_volatile(&(*r).tx[self.tx_cur]).opts1 & OWN == 0 }
    }

    pub fn send(&mut self, data: &[u8]) -> bool {
        if !self.can_send() { return false; }
        let len = data.len().min(BUF);
        unsafe {
            let r = &raw mut RINGS;
            let b = &raw mut BUFS;
            (*b).tx[self.tx_cur][..len].copy_from_slice(&data[..len]);
            let mut o = OWN | FS | LS | len as u32;
            if self.tx_cur == NUM_TX - 1 { o |= EOR; }
            let addr = (*b).tx[self.tx_cur].as_ptr() as u64;
            write_volatile(&mut (*r).tx[self.tx_cur], Desc { opts1: o, opts2: 0, addr });
        }
        self.tx_cur = (self.tx_cur + 1) % NUM_TX;
        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
        self.w8(0x38, 0x40);
        true
    }
}
