use crate::pci::{self, PciDev};

fn outb(p: u16, v: u8) { unsafe { core::arch::asm!("out dx, al", in("dx") p, in("al") v, options(nomem, nostack)) } }
fn outw(p: u16, v: u16) { unsafe { core::arch::asm!("out dx, ax", in("dx") p, in("ax") v, options(nomem, nostack)) } }
fn outl(p: u16, v: u32) { unsafe { core::arch::asm!("out dx, eax", in("dx") p, in("eax") v, options(nomem, nostack)) } }
fn inb(p: u16) -> u8 { let v: u8; unsafe { core::arch::asm!("in al, dx", in("dx") p, out("al") v, options(nomem, nostack)) } v }
fn inl(p: u16) -> u32 { let v: u32; unsafe { core::arch::asm!("in eax, dx", in("dx") p, out("eax") v, options(nomem, nostack)) } v }

const RX_LEN: usize = 8192;

#[repr(C, align(16))]
struct Bufs { rx: [u8; RX_LEN + 16 + 2048], tx: [[u8; 2048]; 4] }
static mut BUFS: Bufs = Bufs { rx: [0; RX_LEN + 16 + 2048], tx: [[0; 2048]; 4] };

pub struct Rtl8139 { io: u16, rx_off: usize, tx_cur: usize, pub mac: [u8; 6] }

pub fn supported(d: &PciDev) -> bool {
    (d.vendor == 0x10EC && d.device == 0x8139) || (d.vendor == 0x1113 && d.device == 0x1211)
}

impl Rtl8139 {
    pub fn new(d: &PciDev) -> Option<Self> {
        pci::enable(d);
        let (base, io) = pci::bar(d, 0);
        if !io || base == 0 { return None; }
        let io = base as u16;
        outb(io + 0x52, 0x00);
        outb(io + 0x37, 0x10);
        for _ in 0..1_000_000 { if inb(io + 0x37) & 0x10 == 0 { break; } }
        let mut mac = [0u8; 6];
        for i in 0..6 { mac[i] = inb(io + i as u16); }
        unsafe {
            let b = &raw mut BUFS;
            outl(io + 0x30, (*b).rx.as_ptr() as u32);
        }
        outw(io + 0x3C, 0x0000);
        outl(io + 0x44, 0x0F | (1 << 7));
        outb(io + 0x37, 0x0C);
        outl(io + 0x40, 0x0300_0700);
        Some(Rtl8139 { io, rx_off: 0, tx_cur: 0, mac })
    }

    pub fn link_up(&self) -> bool { inb(self.io + 0x58) & 0x04 == 0 }
    pub fn speed_mbps(&self) -> u32 { if inb(self.io + 0x58) & 0x08 != 0 { 10 } else { 100 } }

    pub fn recv(&mut self, out: &mut [u8]) -> Option<usize> {
        if inb(self.io + 0x37) & 0x01 != 0 { return None; }
        unsafe {
            let b = &raw const BUFS;
            let rx = &(*b).rx;
            let off = self.rx_off;
            let status = u16::from_le_bytes([rx[off], rx[off + 1]]);
            let len = u16::from_le_bytes([rx[off + 2], rx[off + 3]]) as usize;
            if len == 0 || len > 1792 || status & 1 == 0 {
                outb(self.io + 0x37, 0x04);
                self.rx_off = 0;
                outl(self.io + 0x30, rx.as_ptr() as u32);
                outb(self.io + 0x37, 0x0C);
                return Some(0);
            }
            let plen = (len - 4).min(out.len());
            out[..plen].copy_from_slice(&rx[off + 4..off + 4 + plen]);
            self.rx_off = (off + len + 4 + 3) & !3;
            if self.rx_off >= RX_LEN { self.rx_off -= RX_LEN; }
            outw(self.io + 0x38, (self.rx_off as u16).wrapping_sub(16));
            Some(plen)
        }
    }

    pub fn can_send(&self) -> bool {
        let tsd = inl(self.io + 0x10 + (self.tx_cur as u16) * 4);
        tsd & (1 << 13) != 0 || tsd == 0 || tsd & (1 << 15) != 0
    }

    pub fn send(&mut self, data: &[u8]) -> bool {
        if !self.can_send() { return false; }
        let len = data.len().min(1792);
        unsafe {
            let b = &raw mut BUFS;
            (*b).tx[self.tx_cur][..len].copy_from_slice(&data[..len]);
            let mut l = len;
            if l < 60 { for i in l..60 { (*b).tx[self.tx_cur][i] = 0; } l = 60; }
            outl(self.io + 0x20 + (self.tx_cur as u16) * 4, (*b).tx[self.tx_cur].as_ptr() as u32);
            outl(self.io + 0x10 + (self.tx_cur as u16) * 4, l as u32);
        }
        self.tx_cur = (self.tx_cur + 1) % 4;
        true
    }
}
