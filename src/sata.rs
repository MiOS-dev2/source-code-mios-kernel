
#![allow(dead_code)]
#![allow(non_snake_case)]



const IDE_PRIMARY: u16 = 0x1F0;
const IDE_SECONDARY: u16 = 0x170;
const IDE_PRIMARY_CTRL: u16 = 0x3F6;
const IDE_SECONDARY_CTRL: u16 = 0x376;


const IDE_REG_DATA: u16 = 0;
const IDE_REG_ERROR: u16 = 1;
const IDE_REG_SECTOR_COUNT: u16 = 2;
const IDE_REG_LBA0: u16 = 3;
const IDE_REG_LBA1: u16 = 4;
const IDE_REG_LBA2: u16 = 5;
const IDE_REG_DRIVE_SELECT: u16 = 6;
const IDE_REG_COMMAND: u16 = 7;
const IDE_REG_STATUS: u16 = 7;
const IDE_REG_CONTROL: u16 = 0;


const ATA_CMD_READ_PIO: u8 = 0x20;
const ATA_CMD_READ_PIO_EXT: u8 = 0x24;
const ATA_CMD_IDENTIFY: u8 = 0xEC;
const ATA_CMD_IDENTIFY_PACKET: u8 = 0xA1;


#[derive(Debug, PartialEq, Copy, Clone)]
pub enum DeviceType {
    None,
    ATA,
    ATAPI,
    Unknown,
}


#[derive(Debug, Clone, Copy)]
pub struct DiskInfo {
    pub model: [u8; 40],
    pub serial: [u8; 20],
    pub firmware: [u8; 8],
    pub size: u64,
    pub sectors: u64,
    pub sector_size: u16,
    pub device_type: DeviceType,
    pub drive: u8,
    pub channel: u8,
    pub present: bool,
    pub lba_supported: bool,
    pub lba48_supported: bool,
}



#[repr(packed)]
struct IdentifyData {
    config: u16,
    cylinders: u16,
    reserved: u16,
    heads: u16,
    sectors_per_track: u16,
    sectors: u16,
    serial: [u8; 20],
    _pad1: [u8; 14],
    commands: u16,
    _pad2: u8,
    features: u16,
    _pad3: [u8; 2],
    _pad4: [u8; 20],
    model: [u8; 40],
    _pad5: [u8; 36],
    lba_sectors: u32,
    _pad6: [u8; 24],
    lba48_sectors: u64,
    _pad7: [u8; 64],
}



static mut DISKS: [DiskInfo; 8] = [DiskInfo {
    model: [0; 40],
    serial: [0; 20],
    firmware: [0; 8],
    size: 0,
    sectors: 0,
    sector_size: 512,
    device_type: DeviceType::None,
    drive: 0,
    channel: 0,
    present: false,
    lba_supported: false,
    lba48_supported: false,
}; 8];

static mut DISK_COUNT: u8 = 0;
static mut INITIALIZED: bool = false;
static mut DRIVER_READY: bool = false;



#[inline(always)]
fn outb(port: u16, value: u8) {
    unsafe {
        core::arch::asm!("out dx, al", in("dx") port, in("al") value, options(nostack));
    }
}

#[inline(always)]
fn inb(port: u16) -> u8 {
    let result: u8;
    unsafe {
        core::arch::asm!("in al, dx", out("al") result, in("dx") port, options(nostack));
    }
    result
}

#[inline(always)]
fn inw(port: u16) -> u16 {
    let result: u16;
    unsafe {
        core::arch::asm!("in ax, dx", out("ax") result, in("dx") port, options(nostack));
    }
    result
}



struct IDEChannel {
    base: u16,
    ctrl: u16,
}

impl IDEChannel {
    fn new(base: u16, ctrl: u16) -> Self {
        IDEChannel { base, ctrl }
    }

    fn wait_for_ready(&self) -> bool {
        let mut timeout = 50000;
        while timeout > 0 {
            let status = inb(self.base + IDE_REG_STATUS);
            if (status & 0x80) == 0 {
                return true;
            }
            timeout -= 1;
            for _ in 0..10 {
                unsafe { core::arch::asm!("nop", options(nostack)); }
            }
        }
        false
    }

    fn wait_for_data(&self) -> bool {
        let mut timeout = 50000;
        while timeout > 0 {
            let status = inb(self.base + IDE_REG_STATUS);
            if (status & 0x08) != 0 {
                return true;
            }
            if (status & 0x01) != 0 {
                return false;
            }
            timeout -= 1;
            for _ in 0..10 {
                unsafe { core::arch::asm!("nop", options(nostack)); }
            }
        }
        false
    }

    fn check_error(&self) -> bool {
        (inb(self.base + IDE_REG_STATUS) & 0x01) != 0
    }

    fn select_drive(&self, drive: u8, lba: bool) {
        let dev = if drive == 0 { 0xA0 } else { 0xB0 };
        let lba_bit = if lba { 0x40 } else { 0 };
        outb(self.base + IDE_REG_DRIVE_SELECT, dev | lba_bit);
        

        self.wait_for_ready();
    }

    fn read_pio(&self, sector: u32, buffer: &mut [u8; 512]) -> bool {

        if !self.wait_for_ready() {
            return false;
        }


        self.select_drive(0, true);
        

        if !self.wait_for_ready() {
            return false;
        }

        outb(self.base + IDE_REG_SECTOR_COUNT, 1);
        outb(self.base + IDE_REG_LBA0, (sector & 0xFF) as u8);
        outb(self.base + IDE_REG_LBA1, ((sector >> 8) & 0xFF) as u8);
        outb(self.base + IDE_REG_LBA2, ((sector >> 16) & 0xFF) as u8);
        outb(self.base + IDE_REG_COMMAND, ATA_CMD_READ_PIO);

        if !self.wait_for_data() {
            return false;
        }

        for i in 0..256 {
            let word = inw(self.base + IDE_REG_DATA);
            buffer[i * 2] = (word & 0xFF) as u8;
            buffer[i * 2 + 1] = ((word >> 8) & 0xFF) as u8;
        }

        !self.check_error()
    }

    fn read_pio_ext(&self, sector: u64, buffer: &mut [u8; 512]) -> bool {

        if !self.wait_for_ready() {
            return false;
        }

        outb(self.base + IDE_REG_DRIVE_SELECT, 0x40);
        

        if !self.wait_for_ready() {
            return false;
        }
        
        outb(self.base + IDE_REG_SECTOR_COUNT, 1);
        outb(self.base + IDE_REG_LBA0, (sector & 0xFF) as u8);
        outb(self.base + IDE_REG_LBA1, ((sector >> 8) & 0xFF) as u8);
        outb(self.base + IDE_REG_LBA2, ((sector >> 16) & 0xFF) as u8);
        outb(self.base + IDE_REG_COMMAND, ATA_CMD_READ_PIO_EXT);

        if !self.wait_for_data() {
            return false;
        }

        for i in 0..256 {
            let word = inw(self.base + IDE_REG_DATA);
            buffer[i * 2] = (word & 0xFF) as u8;
            buffer[i * 2 + 1] = ((word >> 8) & 0xFF) as u8;
        }

        !self.check_error()
    }

    fn identify(&self, drive: u8) -> Option<IdentifyData> {
        outb(self.ctrl + IDE_REG_CONTROL, 0x04);
        for _ in 0..1000 {
            unsafe { core::arch::asm!("nop", options(nostack)); }
        }
        outb(self.ctrl + IDE_REG_CONTROL, 0x00);

        if !self.wait_for_ready() {
            return None;
        }

        self.select_drive(drive, true);

        if !self.wait_for_ready() {
            return None;
        }

        outb(self.base + IDE_REG_COMMAND, ATA_CMD_IDENTIFY);
        for _ in 0..1000 {
            unsafe { core::arch::asm!("nop", options(nostack)); }
        }

        let status = inb(self.base + IDE_REG_STATUS);
        if status == 0 || (status & 0x01) != 0 {
            return None;
        }

        if !self.wait_for_data() {
            return None;
        }

        let mut data = [0u16; 256];
        for i in 0..256 {
            data[i] = inw(self.base + IDE_REG_DATA);
        }

        if data[0] & 0x8000 != 0 {
            return None;
        }

        let mut id = IdentifyData {
            config: data[0],
            cylinders: data[1],
            reserved: data[2],
            heads: data[3],
            sectors_per_track: data[4],
            sectors: data[5],
            serial: [0; 20],
            _pad1: [0; 14],
            commands: data[7],
            _pad2: 0,
            features: data[8],
            _pad3: [0; 2],
            _pad4: [0; 20],
            model: [0; 40],
            _pad5: [0; 36],
            lba_sectors: 0,
            _pad6: [0; 24],
            lba48_sectors: 0,
            _pad7: [0; 64],
        };

        for i in 0..10 {
            let word = data[10 + i];
            id.serial[i * 2] = ((word >> 8) & 0xFF) as u8;
            id.serial[i * 2 + 1] = (word & 0xFF) as u8;
        }

        for i in 0..20 {
            let word = data[27 + i];
            id.model[i * 2] = ((word >> 8) & 0xFF) as u8;
            id.model[i * 2 + 1] = (word & 0xFF) as u8;
        }

        id.lba_sectors = (data[60] as u32) | ((data[61] as u32) << 16);
        let lba48_low = (data[100] as u64) | ((data[101] as u64) << 16);
        let lba48_high = (data[102] as u64) | ((data[103] as u64) << 16);
        id.lba48_sectors = (lba48_high << 32) | lba48_low;

        Some(id)
    }

    fn identify_packet(&self, drive: u8) -> Option<IdentifyData> {
        if !self.wait_for_ready() {
            return None;
        }

        self.select_drive(drive, true);

        if !self.wait_for_ready() {
            return None;
        }

        outb(self.base + IDE_REG_COMMAND, ATA_CMD_IDENTIFY_PACKET);
        for _ in 0..1000 {
            unsafe { core::arch::asm!("nop", options(nostack)); }
        }

        let status = inb(self.base + IDE_REG_STATUS);
        if status == 0 || (status & 0x01) != 0 {
            return None;
        }

        if !self.wait_for_data() {
            return None;
        }

        let mut data = [0u16; 256];
        for i in 0..256 {
            data[i] = inw(self.base + IDE_REG_DATA);
        }

        let mut id = IdentifyData {
            config: data[0],
            cylinders: data[1],
            reserved: data[2],
            heads: data[3],
            sectors_per_track: data[4],
            sectors: data[5],
            serial: [0; 20],
            _pad1: [0; 14],
            commands: data[7],
            _pad2: 0,
            features: data[8],
            _pad3: [0; 2],
            _pad4: [0; 20],
            model: [0; 40],
            _pad5: [0; 36],
            lba_sectors: 0,
            _pad6: [0; 24],
            lba48_sectors: 0,
            _pad7: [0; 64],
        };

        for i in 0..10 {
            let word = data[10 + i];
            id.serial[i * 2] = ((word >> 8) & 0xFF) as u8;
            id.serial[i * 2 + 1] = (word & 0xFF) as u8;
        }

        for i in 0..20 {
            let word = data[27 + i];
            id.model[i * 2] = ((word >> 8) & 0xFF) as u8;
            id.model[i * 2 + 1] = (word & 0xFF) as u8;
        }

        Some(id)
    }
}


fn trim_spaces(buf: &mut [u8]) -> usize {
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
    
    end - start
}

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


pub unsafe fn init_sata() -> bool {
    if INITIALIZED {
        return DRIVER_READY;
    }

    outb(IDE_PRIMARY_CTRL + IDE_REG_CONTROL, 0x04);
    outb(IDE_SECONDARY_CTRL + IDE_REG_CONTROL, 0x04);
    for _ in 0..1000 {
        unsafe { core::arch::asm!("nop", options(nostack)); }
    }
    outb(IDE_PRIMARY_CTRL + IDE_REG_CONTROL, 0x00);
    outb(IDE_SECONDARY_CTRL + IDE_REG_CONTROL, 0x00);

    let channels = [
        (IDE_PRIMARY, IDE_PRIMARY_CTRL, 0),
        (IDE_SECONDARY, IDE_SECONDARY_CTRL, 1),
    ];

    for (base, ctrl, channel_id) in channels {
        let channel = IDEChannel::new(base, ctrl);

        if let Some(id) = channel.identify(0) {
            add_disk(&id, 0, channel_id, DeviceType::ATA);
        } else if let Some(id) = channel.identify_packet(0) {
            add_disk(&id, 0, channel_id, DeviceType::ATAPI);
        }

        if let Some(id) = channel.identify(1) {
            add_disk(&id, 1, channel_id, DeviceType::ATA);
        } else if let Some(id) = channel.identify_packet(1) {
            add_disk(&id, 1, channel_id, DeviceType::ATAPI);
        }
    }

    INITIALIZED = true;
    DRIVER_READY = true;
    true
}

unsafe fn add_disk(id: &IdentifyData, drive: u8, channel: u8, dev_type: DeviceType) {
    if DISK_COUNT >= 8 {
        return;
    }

    let idx = DISK_COUNT as usize;
    let disk = &mut DISKS[idx];

    disk.model = id.model;
    let model_len = trim_spaces(&mut disk.model);
    disk.model[model_len] = 0;

    disk.serial = id.serial;
    let serial_len = trim_spaces(&mut disk.serial);
    disk.serial[serial_len] = 0;

    disk.sectors = if id.lba48_sectors > 0 {
        disk.lba48_supported = true;
        id.lba48_sectors
    } else {
        disk.lba_supported = true;
        id.lba_sectors as u64
    };

    disk.size = disk.sectors * 512;
    disk.sector_size = 512;
    disk.device_type = dev_type;
    disk.drive = drive;
    disk.channel = channel;
    disk.present = true;

    DISK_COUNT += 1;
}

pub unsafe fn get_disk_info() -> &'static [DiskInfo] {
    &DISKS[..DISK_COUNT as usize]
}

pub unsafe fn disk_count() -> u8 {
    DISK_COUNT
}

pub fn is_ready() -> bool {
    unsafe { DRIVER_READY }
}

pub unsafe fn read_sector(disk_index: u8, sector: u64, buffer: &mut [u8; 512]) -> bool {
    if disk_index >= DISK_COUNT {
        return false;
    }

    let disk = &DISKS[disk_index as usize];
    if !disk.present {
        return false;
    }

    let base = if disk.channel == 0 { IDE_PRIMARY } else { IDE_SECONDARY };
    let ctrl = if disk.channel == 0 { IDE_PRIMARY_CTRL } else { IDE_SECONDARY_CTRL };
    let channel = IDEChannel::new(base, ctrl);


    channel.select_drive(disk.drive, true);
    

    for _ in 0..1000 {
        unsafe { core::arch::asm!("nop", options(nostack)); }
    }

    if disk.lba48_supported {
        return channel.read_pio_ext(sector, buffer);
    } else if disk.lba_supported {
        return channel.read_pio(sector as u32, buffer);
    }

    false
}

pub unsafe fn test_driver() -> &'static str {
    if !DRIVER_READY {
        init_sata();
    }

    if DISK_COUNT == 0 {
        return "No disks found!";
    }

    static mut RESULT: [u8; 2048] = [0; 2048];
    let mut pos = 0;

    for i in 0..DISK_COUNT {
        let disk = &DISKS[i as usize];
        if disk.present {
            let model = core::str::from_utf8(&disk.model).unwrap_or("Unknown");
            let model_trimmed = model.trim_end_matches('\0');
            
            let type_str = match disk.device_type {
                DeviceType::ATA => "ATA",
                DeviceType::ATAPI => "ATAPI/CD",
                _ => "Unknown",
            };

            let s1 = "Disk ";
            let s2 = ": ";
            let s3 = " [";
            let s4 = "] ";
            let s5 = " MB\n";
            
            for b in s1.as_bytes() { if pos < 2047 { RESULT[pos] = *b; pos += 1; } }
            let n = num_to_str((i + 1) as usize);
            for b in n.as_bytes() { if pos < 2047 { RESULT[pos] = *b; pos += 1; } }
            for b in s2.as_bytes() { if pos < 2047 { RESULT[pos] = *b; pos += 1; } }
            for b in type_str.as_bytes() { if pos < 2047 { RESULT[pos] = *b; pos += 1; } }
            for b in s3.as_bytes() { if pos < 2047 { RESULT[pos] = *b; pos += 1; } }
            for b in model_trimmed.as_bytes() { if pos < 2047 { RESULT[pos] = *b; pos += 1; } }
            for b in s4.as_bytes() { if pos < 2047 { RESULT[pos] = *b; pos += 1; } }
            let sz = num_to_str((disk.size / (1024 * 1024)) as usize);
            for b in sz.as_bytes() { if pos < 2047 { RESULT[pos] = *b; pos += 1; } }
            for b in s5.as_bytes() { if pos < 2047 { RESULT[pos] = *b; pos += 1; } }
        }
    }

    RESULT[pos] = 0;
    core::str::from_utf8(&RESULT[..pos]).unwrap_or("")
}