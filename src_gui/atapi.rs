
#![allow(dead_code)]

pub const IDE_PRIMARY: u16 = 0x1F0;
pub const IDE_SECONDARY: u16 = 0x170;
pub const IDE_DATA: u16 = 0x00;
pub const IDE_ERROR: u16 = 0x01;
pub const IDE_FEATURES: u16 = 0x01;
pub const IDE_SECTOR_COUNT: u16 = 0x02;
pub const IDE_LBA0: u16 = 0x03;
pub const IDE_LBA1: u16 = 0x04;
pub const IDE_LBA2: u16 = 0x05;
pub const IDE_DRIVE: u16 = 0x06;
pub const IDE_STATUS: u16 = 0x07;
pub const IDE_COMMAND: u16 = 0x07;

pub const IDE_STATUS_BSY: u8 = 0x80;
pub const IDE_STATUS_DRDY: u8 = 0x40;
pub const IDE_STATUS_DF: u8 = 0x20;
pub const IDE_STATUS_DRQ: u8 = 0x08;
pub const IDE_STATUS_ERR: u8 = 0x01;

pub const ATAPI_PACKET: u8 = 0xA0;
pub const ATAPI_IDENTIFY: u8 = 0xA1;
pub const ATAPI_READ_10: u8 = 0x28;
pub const ATAPI_TEST_UNIT_READY: u8 = 0x00;

pub static mut CD_PRESENT: bool = false;
pub static mut CD_BASE: u16 = 0;
pub static mut CD_DRIVE: u8 = 0;

#[inline(always)]
pub fn inb(port: u16) -> u8 {
    unsafe {
        let result: u8;
        core::arch::asm!(
            "in al, dx",
            in("dx") port as u16,
            out("al") result,
            options(nostack, readonly)
        );
        result
    }
}

#[inline(always)]
pub fn outb(port: u16, value: u8) {
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") port as u16,
            in("al") value,
            options(nostack)
        );
    }
}

#[inline(always)]
pub fn inw(port: u16) -> u16 {
    unsafe {
        let result: u16;
        core::arch::asm!(
            "in ax, dx",
            in("dx") port as u16,
            out("ax") result,
            options(nostack, readonly)
        );
        result
    }
}

#[inline(always)]
pub fn outw(port: u16, value: u16) {
    unsafe {
        core::arch::asm!(
            "out dx, ax",
            in("dx") port as u16,
            in("ax") value,
            options(nostack)
        );
    }
}



pub fn ide_wait(base: u16, mask: u8, val: u8) -> bool {
    let mut timeout = 1000000;
    unsafe {
        while timeout > 0 {
            let status = inb(base + IDE_STATUS);
            if (status & IDE_STATUS_BSY) == 0 {
                if (status & mask) == val {
                    return true;
                }
                if (status & IDE_STATUS_ERR) != 0 {
                    return false;
                }
            }
            timeout -= 1;
            for _ in 0..10 {
                core::arch::asm!("nop", options(nostack));
            }
        }
    }
    false
}

pub fn ide_wait_ready(base: u16) -> bool {
    ide_wait(base, IDE_STATUS_DRDY, IDE_STATUS_DRDY)
}

pub fn ide_wait_drq(base: u16) -> bool {
    ide_wait(base, IDE_STATUS_DRQ, IDE_STATUS_DRQ)
}



pub fn atapi_identify(base: u16, slave: bool) -> bool {
    let drive = if slave { 0xB0 } else { 0xA0 };
    
    outb(base + IDE_DRIVE, drive);
    
    if !ide_wait_ready(base) {
        return false;
    }
    

    outb(base + IDE_SECTOR_COUNT, 0x00);
    outb(base + IDE_LBA0, 0x00);
    outb(base + IDE_LBA1, 0x00);
    outb(base + IDE_LBA2, 0x00);
    outb(base + IDE_COMMAND, ATAPI_IDENTIFY);
    
    if !ide_wait_ready(base) {
        return false;
    }
    
    let status = inb(base + IDE_STATUS);
    if (status & IDE_STATUS_ERR) != 0 {

        outb(base + IDE_COMMAND, 0xEC);
        if !ide_wait_ready(base) {
            return false;
        }
        let status2 = inb(base + IDE_STATUS);
        if (status2 & IDE_STATUS_ERR) != 0 {
            return false;
        }
    }
    

    let mut data = [0u16; 256];
    for i in 0..256 {
        data[i] = inw(base + IDE_DATA);
    }
    

    if (data[0] & 0x8000) != 0 {
        return true;
    }
    
    if (data[49] & 0x0100) != 0 {
        return true;
    }
    
    false
}



pub fn atapi_send_packet(base: u16, packet: &[u8; 12]) -> bool {
    let drive = if unsafe { CD_DRIVE } == 0 { 0xA0 } else { 0xB0 };
    outb(base + IDE_DRIVE, drive);
    
    if !ide_wait_ready(base) {
        return false;
    }
    
    outb(base + IDE_FEATURES, 0x00);
    outb(base + IDE_SECTOR_COUNT, 0x00);
    outb(base + IDE_LBA0, 0x00);
    outb(base + IDE_LBA1, 0x00);
    outb(base + IDE_LBA2, 0x00);
    outb(base + IDE_COMMAND, ATAPI_PACKET);
    
    if !ide_wait_drq(base) {
        return false;
    }
    

    for i in 0..6 {
        let word = ((packet[i*2 + 1] as u16) << 8) | (packet[i*2] as u16);
        outw(base + IDE_DATA, word);
    }
    
    true
}



pub fn atapi_test_unit_ready(base: u16) -> bool {
    let mut packet = [0u8; 12];
    packet[0] = ATAPI_TEST_UNIT_READY;
    packet[1] = 0x00;
    packet[2] = 0x00;
    packet[3] = 0x00;
    packet[4] = 0x00;
    packet[5] = 0x00;
    packet[6] = 0x00;
    packet[7] = 0x00;
    packet[8] = 0x00;
    packet[9] = 0x00;
    packet[10] = 0x00;
    packet[11] = 0x00;
    
    atapi_send_packet(base, &packet)
}



pub fn atapi_read_sector(base: u16, lba: u32, buf: &mut [u8; 2048]) -> bool {

    for _ in 0..3 {
        if atapi_test_unit_ready(base) {
            break;
        }
        for _ in 0..100000 {
            unsafe { core::arch::asm!("nop", options(nostack)); }
        }
    }
    
    let mut packet = [0u8; 12];
    packet[0] = ATAPI_READ_10;
    packet[1] = 0x00;
    packet[2] = ((lba >> 24) & 0xFF) as u8;
    packet[3] = ((lba >> 16) & 0xFF) as u8;
    packet[4] = ((lba >> 8) & 0xFF) as u8;
    packet[5] = (lba & 0xFF) as u8;
    packet[6] = 0x00;
    packet[7] = 0x00;
    packet[8] = 0x01;
    packet[9] = 0x00;
    packet[10] = 0x00;
    packet[11] = 0x00;
    
    if !atapi_send_packet(base, &packet) {
        return false;
    }
    
    if !ide_wait_drq(base) {
        return false;
    }
    

    for i in (0..2048).step_by(2) {
        let word = inw(base + IDE_DATA);
        buf[i] = (word & 0xFF) as u8;
        buf[i + 1] = ((word >> 8) & 0xFF) as u8;
    }
    
    true
}



pub fn init_cdrom() {
    unsafe {

        outb(IDE_PRIMARY + IDE_COMMAND, 0x08);
        outb(IDE_SECONDARY + IDE_COMMAND, 0x08);
        for _ in 0..10000 {
            core::arch::asm!("nop", options(nostack));
        }
        outb(IDE_PRIMARY + IDE_COMMAND, 0x00);
        outb(IDE_SECONDARY + IDE_COMMAND, 0x00);
        

        if atapi_identify(IDE_PRIMARY, false) {
            CD_PRESENT = true;
            CD_BASE = IDE_PRIMARY;
            CD_DRIVE = 0;
            return;
        }
        
        if atapi_identify(IDE_PRIMARY, true) {
            CD_PRESENT = true;
            CD_BASE = IDE_PRIMARY;
            CD_DRIVE = 1;
            return;
        }
        
        if atapi_identify(IDE_SECONDARY, false) {
            CD_PRESENT = true;
            CD_BASE = IDE_SECONDARY;
            CD_DRIVE = 0;
            return;
        }
        
        if atapi_identify(IDE_SECONDARY, true) {
            CD_PRESENT = true;
            CD_BASE = IDE_SECONDARY;
            CD_DRIVE = 1;
            return;
        }
    }
}

pub fn read_cd_sector(lba: u32, buf: &mut [u8; 2048]) -> bool {
    unsafe {
        if !CD_PRESENT {
            return false;
        }
        

        if atapi_read_sector(CD_BASE, lba, buf) {
            return true;
        }
        

        for _ in 0..100000 {
            core::arch::asm!("nop", options(nostack));
        }
        
        atapi_read_sector(CD_BASE, lba, buf)
    }
}

pub fn get_cd_info() -> &'static str {
    unsafe {
        if CD_PRESENT {
            let base_str = if CD_BASE == IDE_PRIMARY { "Primary" } else { "Secondary" };
            let drive_str = if CD_DRIVE == 0 { "Master" } else { "Slave" };
            
            static mut BUF: [u8; 64] = [0; 64];
            let mut pos = 0;
            let msg = b"ATAPI ";
            for b in msg {
                if pos < 63 { BUF[pos] = *b; pos += 1; }
            }
            for b in base_str.as_bytes() {
                if pos < 63 { BUF[pos] = *b; pos += 1; }
            }
            BUF[pos] = b' '; pos += 1;
            for b in drive_str.as_bytes() {
                if pos < 63 { BUF[pos] = *b; pos += 1; }
            }
            BUF[pos] = 0;
            core::str::from_utf8(&BUF[..pos]).unwrap_or("ATAPI drive")
        } else {
            "No ATAPI drive"
        }
    }
}