
use crate::vga::VGA;

const ATA_PRIMARY_DATA: u16 = 0x1F0;
const ATA_PRIMARY_SECTOR_COUNT: u16 = 0x1F2;
const ATA_PRIMARY_LBA_LOW: u16 = 0x1F3;
const ATA_PRIMARY_LBA_MID: u16 = 0x1F4;
const ATA_PRIMARY_LBA_HIGH: u16 = 0x1F5;
const ATA_PRIMARY_DRIVE: u16 = 0x1F6;
const ATA_PRIMARY_COMMAND: u16 = 0x1F7;
const ATA_PRIMARY_STATUS: u16 = 0x1F7;

const ATA_CMD_READ: u8 = 0x20;
const ATA_CMD_WRITE: u8 = 0x30;
const ATA_CMD_IDENTIFY: u8 = 0xEC;

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
    
    fn inb(port: u16) -> u8 {
        unsafe {
            let value: u8;
            core::arch::asm!("in al, dx", in("dx") port, out("al") value);
            value
        }
    }
    
    fn outb(port: u16, value: u8) {
        unsafe {
            core::arch::asm!("out dx, al", in("dx") port, in("al") value);
        }
    }
    
    fn inw(port: u16) -> u16 {
        unsafe {
            let value: u16;
            core::arch::asm!("in ax, dx", in("dx") port, out("ax") value);
            value
        }
    }
    
    fn outw(port: u16, value: u16) {
        unsafe {
            core::arch::asm!("out dx, ax", in("dx") port, in("ax") value);
        }
    }
    
    fn wait_ready(&self) {
        while Self::inb(ATA_PRIMARY_STATUS) & 0x80 != 0 {}
    }
    
    fn wait_drq(&self) {
        while Self::inb(ATA_PRIMARY_STATUS) & 0x08 == 0 {}
    }
    
    pub fn init(&mut self, vga: &mut VGA) {
        Self::outb(ATA_PRIMARY_DRIVE, 0xA0);
        
        for _ in 0..100 {
            Self::inb(ATA_PRIMARY_STATUS);
        }
        
        let status = Self::inb(ATA_PRIMARY_STATUS);
        if status == 0xFF || status == 0x00 {
            vga.write_string("[ATA] No drive detected\n");
            return;
        }
        
        self.exists = true;
        
        Self::outb(ATA_PRIMARY_DRIVE, 0xA0);
        Self::outb(ATA_PRIMARY_SECTOR_COUNT, 0);
        Self::outb(ATA_PRIMARY_LBA_LOW, 0);
        Self::outb(ATA_PRIMARY_LBA_MID, 0);
        Self::outb(ATA_PRIMARY_LBA_HIGH, 0);
        Self::outb(ATA_PRIMARY_COMMAND, ATA_CMD_IDENTIFY);
        
        if Self::inb(ATA_PRIMARY_STATUS) == 0 {
            vga.write_string("[ATA] Drive does not support IDENTIFY\n");
            return;
        }
        
        self.wait_ready();
        
        let mut identify_data = [0u16; 256];
        for i in 0..256 {
            identify_data[i] = Self::inw(ATA_PRIMARY_DATA);
        }
        
        if identify_data[83] & (1 << 10) != 0 {
            self.sectors = unsafe { *(identify_data.as_ptr().add(100) as *const u32) };
        } else {
            self.sectors = identify_data[60] as u32 | ((identify_data[61] as u32) << 16);
        }
        
        for i in 0..20 {
            let word = identify_data[27 + i];
            self.model[i * 2] = (word >> 8) as u8;
            self.model[i * 2 + 1] = word as u8;
        }
        
        vga.write_string("[ATA] ");
        for i in 0..40 {
            if self.model[i] != 0 && self.model[i] != 0x20 {
                vga.put_char(self.model[i] as char);
            }
        }
        vga.write_string("\n[ATA] Sectors: ");
        vga.write_number(self.sectors as usize);
        vga.write_string("\n");
    }
    
    pub fn read_sector(&self, lba: u32, buf: &mut [u8; 512]) -> bool {
        if !self.exists { return false; }
        
        self.wait_ready();
        
        Self::outb(ATA_PRIMARY_DRIVE, 0xE0 | ((lba >> 24) & 0x0F) as u8);
        Self::outb(ATA_PRIMARY_SECTOR_COUNT, 1);
        Self::outb(ATA_PRIMARY_LBA_LOW, lba as u8);
        Self::outb(ATA_PRIMARY_LBA_MID, (lba >> 8) as u8);
        Self::outb(ATA_PRIMARY_LBA_HIGH, (lba >> 16) as u8);
        Self::outb(ATA_PRIMARY_COMMAND, ATA_CMD_READ);
        
        self.wait_drq();
        
        for i in 0..256 {
            let word = Self::inw(ATA_PRIMARY_DATA);
            buf[i * 2] = word as u8;
            buf[i * 2 + 1] = (word >> 8) as u8;
        }
        
        true
    }
    
    pub fn write_sector(&self, lba: u32, buf: &[u8; 512]) -> bool {
        if !self.exists { return false; }
        
        self.wait_ready();
        
        Self::outb(ATA_PRIMARY_DRIVE, 0xE0 | ((lba >> 24) & 0x0F) as u8);
        Self::outb(ATA_PRIMARY_SECTOR_COUNT, 1);
        Self::outb(ATA_PRIMARY_LBA_LOW, lba as u8);
        Self::outb(ATA_PRIMARY_LBA_MID, (lba >> 8) as u8);
        Self::outb(ATA_PRIMARY_LBA_HIGH, (lba >> 16) as u8);
        Self::outb(ATA_PRIMARY_COMMAND, ATA_CMD_WRITE);
        
        self.wait_drq();
        
        for i in 0..256 {
            let word = (buf[i * 2] as u16) | ((buf[i * 2 + 1] as u16) << 8);
            Self::outw(ATA_PRIMARY_DATA, word);
        }
        
        true
    }
}
