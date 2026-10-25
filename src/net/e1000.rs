use crate::pci::{self, PciDev};
use core::ptr::{read_volatile, write_volatile};

const CTRL: u32 = 0x0000;
const STATUS: u32 = 0x0008;
const EERD: u32 = 0x0014;
const ICR: u32 = 0x00C0;
const IMC: u32 = 0x00D8;
const RCTL: u32 = 0x0100;
const TCTL: u32 = 0x0400;
const TIPG: u32 = 0x0410;
const RDBAL: u32 = 0x2800;
const RDBAH: u32 = 0x2804;
const RDLEN: u32 = 0x2808;
const RDH: u32 = 0x2810;
const RDT: u32 = 0x2818;
const TDBAL: u32 = 0x3800;
const TDBAH: u32 = 0x3804;
const TDLEN: u32 = 0x3808;
const TDH: u32 = 0x3810;
const TDT: u32 = 0x3818;
const MTA: u32 = 0x5200;
const RAL0: u32 = 0x5400;
const RAH0: u32 = 0x5404;

const NUM_RX: usize = 64;
const NUM_TX: usize = 32;
const BUF: usize = 2048;

#[repr(C, align(16))]
#[derive(Clone, Copy)]
struct RxDesc { addr: u64, len: u16, csum: u16, status: u8, errors: u8, special: u16 }
#[repr(C, align(16))]
#[derive(Clone, Copy)]
struct TxDesc { addr: u64, len: u16, cso: u8, cmd: u8, status: u8, css: u8, special: u16 }

#[repr(C, align(128))]
struct Rings {
    rx: [RxDesc; NUM_RX],
    tx: [TxDesc; NUM_TX],
}
#[repr(C, align(4096))]
struct Bufs { rx: [[u8; BUF]; NUM_RX], tx: [[u8; BUF]; NUM_TX] }

static mut RINGS: Rings = Rings {
    rx: [RxDesc { addr: 0, len: 0, csum: 0, status: 0, errors: 0, special: 0 }; NUM_RX],
    tx: [TxDesc { addr: 0, len: 0, cso: 0, cmd: 0, status: 0, css: 0, special: 0 }; NUM_TX],
};
static mut BUFS: Bufs = Bufs { rx: [[0; BUF]; NUM_RX], tx: [[0; BUF]; NUM_TX] };

pub struct E1000 {
    mmio: u64,
    rx_cur: usize,
    tx_cur: usize,
    pub mac: [u8; 6],
}

pub fn supported(d: &PciDev) -> bool {
    d.vendor == 0x8086 && matches!(d.device,
        0x100E | 0x100F | 0x1004 | 0x1000 | 0x1001 | 0x1026 | 0x1075 | 0x1076 | 0x1079 | 0x107C |
        0x10D3 | 0x10F5 | 0x10EA | 0x1502 | 0x153A)
}

impl E1000 {
    fn r(&self, reg: u32) -> u32 { unsafe { read_volatile((self.mmio + reg as u64) as *const u32) } }
    fn w(&self, reg: u32, v: u32) { unsafe { write_volatile((self.mmio + reg as u64) as *mut u32, v) } }

    pub fn new(d: &PciDev) -> Option<Self> {
        pci::enable(d);
        let (base, io) = pci::bar(d, 0);
        if io || base == 0 || base >= 0x1_0000_0000 { return None; }
        let mut nic = E1000 { mmio: base, rx_cur: 0, tx_cur: 0, mac: [0; 6] };

        nic.w(IMC, 0xFFFF_FFFF);
        nic.w(CTRL, nic.r(CTRL) | (1 << 26));
        crate::timer::sleep_ms(20);
        for _ in 0..100000 { if nic.r(CTRL) & (1 << 26) == 0 { break; } }
        nic.w(IMC, 0xFFFF_FFFF);
        let _ = nic.r(ICR);
        let ctrl = nic.r(CTRL);
        nic.w(CTRL, (ctrl | (1 << 6) | (1 << 5)) & !((1 << 3) | (1 << 31) | (1 << 7)));

        let ral = nic.r(RAL0);
        let rah = nic.r(RAH0);
        if ral != 0 && ral != 0xFFFF_FFFF {
            nic.mac = [ral as u8, (ral >> 8) as u8, (ral >> 16) as u8, (ral >> 24) as u8, rah as u8, (rah >> 8) as u8];
        } else {
            for i in 0..3u32 {
                let w = nic.eeprom_read(i as u8);
                nic.mac[(i * 2) as usize] = w as u8;
                nic.mac[(i * 2 + 1) as usize] = (w >> 8) as u8;
            }
            let m = nic.mac;
            nic.w(RAL0, u32::from_le_bytes([m[0], m[1], m[2], m[3]]));
            nic.w(RAH0, (m[4] as u32) | ((m[5] as u32) << 8) | (1 << 31));
        }
        for i in 0..128u32 { nic.w(MTA + i * 4, 0); }

        unsafe {
            let rings = &raw mut RINGS;
            let bufs = &raw mut BUFS;
            for i in 0..NUM_RX {
                (*rings).rx[i] = RxDesc { addr: (*bufs).rx[i].as_ptr() as u64, len: 0, csum: 0, status: 0, errors: 0, special: 0 };
            }
            for i in 0..NUM_TX {
                (*rings).tx[i] = TxDesc { addr: (*bufs).tx[i].as_ptr() as u64, len: 0, cso: 0, cmd: 0, status: 1, css: 0, special: 0 };
            }
            let rx_base = (*rings).rx.as_ptr() as u64;
            let tx_base = (*rings).tx.as_ptr() as u64;
            nic.w(RDBAL, rx_base as u32);
            nic.w(RDBAH, (rx_base >> 32) as u32);
            nic.w(RDLEN, (NUM_RX * 16) as u32);
            nic.w(RDH, 0);
            nic.w(RDT, (NUM_RX - 1) as u32);
            nic.w(RCTL, (1 << 1) | (1 << 15) | (1 << 26));

            nic.w(TDBAL, tx_base as u32);
            nic.w(TDBAH, (tx_base >> 32) as u32);
            nic.w(TDLEN, (NUM_TX * 16) as u32);
            nic.w(TDH, 0);
            nic.w(TDT, 0);
            nic.w(TCTL, (1 << 1) | (1 << 3) | (0x0F << 4) | (0x3F << 12));
            nic.w(TIPG, 0x0060_200A);
        }
        Some(nic)
    }

    fn eeprom_read(&self, addr: u8) -> u16 {
        self.w(EERD, 1 | ((addr as u32) << 8));
        for _ in 0..100000 {
            let v = self.r(EERD);
            if v & (1 << 4) != 0 { return (v >> 16) as u16; }
        }
        self.w(EERD, 1 | ((addr as u32) << 2));
        for _ in 0..100000 {
            let v = self.r(EERD);
            if v & (1 << 1) != 0 { return (v >> 16) as u16; }
        }
        0
    }

    pub fn link_up(&self) -> bool { self.r(STATUS) & 2 != 0 }

    pub fn speed_mbps(&self) -> u32 {
        match (self.r(STATUS) >> 6) & 3 { 0 => 10, 1 => 100, _ => 1000 }
    }

    pub fn recv(&mut self, out: &mut [u8]) -> Option<usize> {
        unsafe {
            let rings = &raw mut RINGS;
            let d = core::ptr::read_volatile(&(*rings).rx[self.rx_cur]);
            if d.status & 1 == 0 { return None; }
            let len = (d.len as usize).min(out.len()).min(BUF);
            let ok = d.status & 2 != 0 && d.errors == 0;
            if ok {
                let bufs = &raw const BUFS;
                out[..len].copy_from_slice(&(*bufs).rx[self.rx_cur][..len]);
            }
            let mut nd = d;
            nd.status = 0;
            core::ptr::write_volatile(&mut (*rings).rx[self.rx_cur], nd);
            let old = self.rx_cur;
            self.rx_cur = (self.rx_cur + 1) % NUM_RX;
            self.w(RDT, old as u32);
            if ok { Some(len) } else { Some(0) }
        }
    }

    pub fn can_send(&self) -> bool {
        unsafe {
            let rings = &raw const RINGS;
            core::ptr::read_volatile(&(*rings).tx[self.tx_cur]).status & 1 != 0
        }
    }

    pub fn send(&mut self, data: &[u8]) -> bool {
        if !self.can_send() { return false; }
        let len = data.len().min(BUF);
        unsafe {
            let rings = &raw mut RINGS;
            let bufs = &raw mut BUFS;
            (*bufs).tx[self.tx_cur][..len].copy_from_slice(&data[..len]);
            let d = TxDesc { addr: (*bufs).tx[self.tx_cur].as_ptr() as u64, len: len as u16, cso: 0,
                cmd: 0x01 | 0x02 | 0x08, status: 0, css: 0, special: 0 };
            core::ptr::write_volatile(&mut (*rings).tx[self.tx_cur], d);
            self.tx_cur = (self.tx_cur + 1) % NUM_TX;
            core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
            self.w(TDT, self.tx_cur as u32);
        }
        true
    }
}
