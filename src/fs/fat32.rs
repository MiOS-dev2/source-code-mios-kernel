#![allow(dead_code)]
#![allow(non_snake_case)]
 
use crate::ata;
 
pub struct FAT32FS {
    disk: usize,
    sectors_per_cluster: u8,
    root_cluster: u32,
    data_start: u32,
    fat_start: u32,
    num_fats: u8,
    fat_sectors: u32,
    total_clusters: u32,
    part_lba: u32,
    fsinfo_sector: u16,
}
 
fn le32(s: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([s[o], s[o + 1], s[o + 2], s[o + 3]])
}
 
fn le64(s: &[u8], o: usize) -> u64 {
    (le32(s, o) as u64) | ((le32(s, o + 4) as u64) << 32)
}
 
const GUID_ESP: [u8; 16] = [
    0x28, 0x73, 0x2A, 0xC1, 0x1F, 0xF8, 0xD2, 0x11,
    0xBA, 0x4B, 0x00, 0xA0, 0xC9, 0x3E, 0xC9, 0x3B,
];
 
struct PartScan {
    disk: usize,
    first: Option<u32>,
    esp: Option<u32>,
}
 
impl PartScan {
    fn check(&mut self, lba: u32, is_esp: bool) -> bool {
        if lba == 0 {
            return false;
        }
        let mut s = [0u8; 512];
        if !unsafe { ata::read_sector(self.disk, lba, &mut s) } {
            return false;
        }
        if !FAT32FS::is_fat32_boot(&s) {
            return false;
        }
        if is_esp {
            if self.esp.is_none() {
                self.esp = Some(lba);
            }
            false
        } else {
            self.first = Some(lba);
            true
        }
    }
 
    fn scan_extended(&mut self, ext_base: u32) -> bool {
        let mut cur = ext_base;
        let mut s = [0u8; 512];
        let mut n = 0;
        while n < 64 {
            n += 1;
            if !unsafe { ata::read_sector(self.disk, cur, &mut s) } {
                return false;
            }
            if s[510] != 0x55 || s[511] != 0xAA {
                return false;
            }
            let t1 = s[446 + 4];
            let rel1 = le32(&s, 446 + 8);
            if t1 != 0 && rel1 != 0 && t1 != 0x05 && t1 != 0x0F && t1 != 0x85 {
                if let Some(l) = cur.checked_add(rel1) {
                    if self.check(l, t1 == 0xEF) {
                        return true;
                    }
                }
            }
            let t2 = s[462 + 4];
            let rel2 = le32(&s, 462 + 8);
            if (t2 != 0x05 && t2 != 0x0F && t2 != 0x85) || rel2 == 0 {
                break;
            }
            let next = match ext_base.checked_add(rel2) {
                Some(v) => v,
                None => break,
            };
            if next == cur {
                break;
            }
            cur = next;
        }
        false
    }
 
    fn scan_gpt(&mut self) -> bool {
        let mut h = [0u8; 512];
        if !unsafe { ata::read_sector(self.disk, 1, &mut h) } {
            return false;
        }
        if &h[0..8] != b"EFI PART" {
            return false;
        }
        let entries_lba = le64(&h, 72);
        let count = le32(&h, 80);
        let esize = le32(&h, 84) as usize;
        if entries_lba == 0 || entries_lba > 0xFFFF_FFFF {
            return false;
        }
        if esize < 128 || esize > 512 || 512 % esize != 0 {
            return false;
        }
        let count = if count > 256 { 256usize } else { count as usize };
        let per_sec = 512 / esize;
        let sectors = (count + per_sec - 1) / per_sec;
 
        let mut s = [0u8; 512];
        for k in 0..sectors {
            let lba = match (entries_lba as u32).checked_add(k as u32) {
                Some(v) => v,
                None => return false,
            };
            if !unsafe { ata::read_sector(self.disk, lba, &mut s) } {
                return false;
            }
            for e in 0..per_sec {
                if k * per_sec + e >= count {
                    break;
                }
                let o = e * esize;
                if s[o..o + 16].iter().all(|&b| b == 0) {
                    continue;
                }
                let first = le64(&s, o + 32);
                if first == 0 || first > 0xFFFF_FFFF {
                    continue;
                }
                let is_esp = s[o..o + 16] == GUID_ESP;
                if self.check(first as u32, is_esp) {
                    return true;
                }
            }
        }
        false
    }
}
 
impl FAT32FS {
    fn has_fat32_sig(sector: &[u8; 512]) -> bool {
        sector[82] == b'F' && sector[83] == b'A' &&
        sector[84] == b'T' && sector[85] == b'3' &&
        sector[86] == b'2'
    }
 
    fn is_fat32_boot(s: &[u8; 512]) -> bool {
        if s[510] != 0x55 || s[511] != 0xAA {
            return false;
        }
        if u16::from_le_bytes([s[11], s[12]]) != 512 {
            return false;
        }
        let spc = s[13];
        if spc == 0 || spc > 128 || (spc & (spc - 1)) != 0 {
            return false;
        }
        let reserved = u16::from_le_bytes([s[14], s[15]]);
        if reserved == 0 || s[16] == 0 {
            return false;
        }
        if u16::from_le_bytes([s[17], s[18]]) != 0 || u16::from_le_bytes([s[22], s[23]]) != 0 {
            return false;
        }
        let fat_sz = le32(s, 36);
        if fat_sz == 0 || le32(s, 44) < 2 {
            return false;
        }
        if Self::has_fat32_sig(s) {
            return true;
        }
        let total16 = u16::from_le_bytes([s[19], s[20]]) as u32;
        let total = if total16 != 0 { total16 } else { le32(s, 32) };
        let meta = (reserved as u32) + (s[16] as u32) * fat_sz;
        total > meta && (total - meta) / (spc as u32) >= 65525
    }
 
    fn find_partition(disk_index: usize, mbr: &[u8; 512]) -> Option<u32> {
        let mut scan = PartScan { disk: disk_index, first: None, esp: None };
 
        for p in 0..4usize {
            let o = 446 + p * 16;
            let ptype = mbr[o + 4];
            let lba = le32(mbr, o + 8);
            match ptype {
                0x00 => {}
                0xEE => {}
                0x05 | 0x0F | 0x85 => {
                    if lba != 0 && scan.scan_extended(lba) {
                        return scan.first;
                    }
                }
                _ => {
                    if scan.check(lba, ptype == 0xEF) {
                        return scan.first;
                    }
                }
            }
        }
 
        if scan.first.is_none() && scan.scan_gpt() {
            return scan.first;
        }
 
        scan.first.or(scan.esp)
    }
 
    pub fn new(disk_index: usize) -> Option<Self> {
        let mut sector = [0u8; 512];
        
        for _ in 0..500000 {
            unsafe { core::arch::asm!("nop", options(nostack)); }
        }
        
        unsafe {
            if !ata::read_sector(disk_index, 0, &mut sector) {
                return None;
            }
        }
        
        if sector[510] != 0x55 || sector[511] != 0xAA {
            return None;
        }
        
        let mut part_lba = 0u32;
        if !Self::is_fat32_boot(&sector) {
            part_lba = Self::find_partition(disk_index, &sector)?;
            unsafe {
                if !ata::read_sector(disk_index, part_lba, &mut sector) {
                    return None;
                }
            }
            if !Self::is_fat32_boot(&sector) {
                return None;
            }
        }
        
        let bytes_per_sector = u16::from_le_bytes([sector[11], sector[12]]);
        if bytes_per_sector != 512 {
            return None;
        }
        
        let sectors_per_cluster = sector[13];
        if sectors_per_cluster == 0 || sectors_per_cluster > 128 {
            return None;
        }
        let reserved_sectors = u16::from_le_bytes([sector[14], sector[15]]);
        let num_fats = sector[16];
        let total16 = u16::from_le_bytes([sector[19], sector[20]]) as u32;
        let total32 = u32::from_le_bytes([sector[32], sector[33], sector[34], sector[35]]);
        let sectors_per_fat = u32::from_le_bytes([sector[36], sector[37], sector[38], sector[39]]);
        let root_cluster = u32::from_le_bytes([sector[44], sector[45], sector[46], sector[47]]);
        let fsinfo_sector = u16::from_le_bytes([sector[48], sector[49]]);
        if num_fats == 0 || sectors_per_fat == 0 || root_cluster < 2 {
            return None;
        }
        
        let total_sectors = if total16 != 0 { total16 } else { total32 };
        let meta = (reserved_sectors as u32) + (num_fats as u32) * sectors_per_fat;
        let total_clusters = if total_sectors > meta {
            (total_sectors - meta) / (sectors_per_cluster as u32)
        } else {
            0
        };
        
        let fat_start = part_lba + reserved_sectors as u32;
        let data_start = fat_start + (num_fats as u32) * sectors_per_fat;
        
        Some(FAT32FS {
            disk: disk_index,
            sectors_per_cluster,
            root_cluster,
            data_start,
            fat_start,
            num_fats,
            fat_sectors: sectors_per_fat,
            total_clusters,
            part_lba,
            fsinfo_sector,
        })
    }
    
    pub fn cluster_to_sector(&self, cluster: u32) -> u32 {
        self.data_start + (cluster - 2) * (self.sectors_per_cluster as u32)
    }
    
    pub fn read_cluster(&self, cluster: u32, buffer: &mut [u8]) -> bool {
        let sector = self.cluster_to_sector(cluster);
        let cluster_size = (self.sectors_per_cluster as usize) * 512;
        
        if buffer.len() < cluster_size {
            return false;
        }
        
        for i in 0..self.sectors_per_cluster as u32 {
            let mut sector_buf = [0u8; 512];
            unsafe {
                if !ata::read_sector(self.disk, (sector + i) as u32, &mut sector_buf) {
                    return false;
                }
            }
            let offset = (i as usize) * 512;
            for j in 0..512 {
                if offset + j < buffer.len() {
                    buffer[offset + j] = sector_buf[j];
                }
            }
        }
        
        true
    }
    
    pub fn next_cluster(&self, cluster: u32) -> u32 {
        let fat_offset = cluster * 4;
        let fat_sector = self.fat_start + (fat_offset / 512);
        let offset = (fat_offset % 512) as usize;
        
        let mut sector = [0u8; 512];
        unsafe {
            if !ata::read_sector(self.disk, fat_sector as u32, &mut sector) {
                return 0x0FFFFFFF;
            }
        }
        
        let next = u32::from_le_bytes([
            sector[offset],
            sector[offset + 1],
            sector[offset + 2],
            sector[offset + 3],
        ]);
        
        next & 0x0FFFFFFF
    }
    
    pub fn list_files(&self) -> &'static str {
        static mut RESULT: [u8; 8192] = [0; 8192];
        static mut CLUSTER_DATA: [u8; 65536] = [0; 65536];
        
        unsafe {
            let mut pos = 0;
            let mut cluster = self.root_cluster;
            let mut has_files = false;
            let mut guard = 0u32;
            
            loop {
                guard += 1;
                if guard > 65536 || cluster < 2 { break; }
                if !self.read_cluster(cluster, &mut CLUSTER_DATA) {
                    break;
                }
                
                let entries = (self.sectors_per_cluster as usize) * 16;
                
                for i in 0..entries {
                    let off = i * 32;
                    if off + 32 > CLUSTER_DATA.len() {
                        break;
                    }
                    
                    let first = CLUSTER_DATA[off];
                    if first == 0x00 { break; }
                    if first == 0xE5 { continue; }
                    
                    let attr = CLUSTER_DATA[off + 11];
                    if attr == 0x0F { continue; }
                    
                    let is_dir = (attr & 0x10) != 0;
                    
                    let mut name = [0u8; 13];
                    let mut npos = 0;
                    
                    for j in 0..8 {
                        let c = CLUSTER_DATA[off + j];
                        if c == b' ' || c == 0 { break; }
                        if c >= 0x20 && c < 0x7F {
                            name[npos] = c;
                            npos += 1;
                        }
                    }
                    
                    let mut has_ext = false;
                    for j in 0..3 {
                        if CLUSTER_DATA[off + 8 + j] != b' ' && CLUSTER_DATA[off + 8 + j] != 0 {
                            has_ext = true;
                            break;
                        }
                    }
                    
                    if has_ext && !is_dir && npos < 12 {
                        name[npos] = b'.';
                        npos += 1;
                        for j in 0..3 {
                            let c = CLUSTER_DATA[off + 8 + j];
                            if c == b' ' || c == 0 { break; }
                            if c >= 0x20 && c < 0x7F && npos < 12 {
                                name[npos] = c;
                                npos += 1;
                            }
                        }
                    }
                    
                    if npos == 0 { continue; }
                    
                    let name_str = core::str::from_utf8(&name[..npos]).unwrap_or("");
                    if name_str == "." || name_str == ".." { continue; }
                    
                    has_files = true;
                    
                    if is_dir {
                        for b in b"[DIR]  " { if pos < 8191 { RESULT[pos] = *b; pos += 1; } }
                    } else {
                        for b in b"[FILE] " { if pos < 8191 { RESULT[pos] = *b; pos += 1; } }
                    }
                    
                    for b in name_str.as_bytes() { if pos < 8191 { RESULT[pos] = *b; pos += 1; } }
                    
                    if !is_dir {
                        let size = u32::from_le_bytes([
                            CLUSTER_DATA[off + 28],
                            CLUSTER_DATA[off + 29],
                            CLUSTER_DATA[off + 30],
                            CLUSTER_DATA[off + 31],
                        ]);
                        
                        if pos < 8191 { RESULT[pos] = b' '; pos += 1; }
                        if pos < 8191 { RESULT[pos] = b'('; pos += 1; }
                        
                        let size_str = num_to_str(size as usize);
                        for b in size_str.as_bytes() { if pos < 8191 { RESULT[pos] = *b; pos += 1; } }
                        
                        for b in b" bytes)" { if pos < 8191 { RESULT[pos] = *b; pos += 1; } }
                    }
                    
                    if pos < 8191 { RESULT[pos] = b'\n'; pos += 1; }
                }
                
                let next = self.next_cluster(cluster);
                if next >= 0x0FFFFFF8 { break; }
                cluster = next;
            }
            
            if !has_files {
                let msg = b"\n  No files found\n";
                for b in msg { if pos < 8191 { RESULT[pos] = *b; pos += 1; } }
            }
            
            RESULT[pos] = 0;
            core::str::from_utf8(&RESULT[..pos]).unwrap_or("")
        }
    }
    
    pub fn read_file(&self, filename: &str) -> &'static str {
        static mut RESULT: [u8; 16384] = [0; 16384];
        static mut CLUSTER_DATA: [u8; 65536] = [0; 65536];
        
        unsafe {
            let mut pos = 0;
            let mut cluster = self.root_cluster;
            let mut found = false;
            let mut file_size = 0u32;
            let mut start_cluster = 0u32;
            let mut guard = 0u32;
 
            loop {
                guard += 1;
                if guard > 65536 || cluster < 2 { break; }
                if !self.read_cluster(cluster, &mut CLUSTER_DATA) {
                    break;
                }
                
                let entries = (self.sectors_per_cluster as usize) * 16;
                
                for i in 0..entries {
                    let off = i * 32;
                    if off + 32 > CLUSTER_DATA.len() {
                        break;
                    }
                    
                    let first = CLUSTER_DATA[off];
                    if first == 0x00 { break; }
                    if first == 0xE5 { continue; }
                    
                    let attr = CLUSTER_DATA[off + 11];
                    if attr == 0x0F { continue; }
                    
                    let is_dir = (attr & 0x10) != 0;
                    if is_dir { continue; }
                    
                    let mut name = [0u8; 13];
                    let mut npos = 0;
                    
                    for j in 0..8 {
                        let c = CLUSTER_DATA[off + j];
                        if c == b' ' || c == 0 { break; }
                        if c >= 0x20 && c < 0x7F {
                            name[npos] = c;
                            npos += 1;
                        }
                    }
                    
                    let mut has_ext = false;
                    for j in 0..3 {
                        if CLUSTER_DATA[off + 8 + j] != b' ' && CLUSTER_DATA[off + 8 + j] != 0 {
                            has_ext = true;
                            break;
                        }
                    }
                    
                    if has_ext && npos < 12 {
                        name[npos] = b'.';
                        npos += 1;
                        for j in 0..3 {
                            let c = CLUSTER_DATA[off + 8 + j];
                            if c == b' ' || c == 0 { break; }
                            if c >= 0x20 && c < 0x7F && npos < 12 {
                                name[npos] = c;
                                npos += 1;
                            }
                        }
                    }
                    
                    if npos == 0 { continue; }
                    
                    let name_str = core::str::from_utf8(&name[..npos]).unwrap_or("");
                    
 
                    let fname = filename.as_bytes();
                    let nbytes = name_str.as_bytes();
                    let mut match_ok = true;
                    
                    if nbytes.len() != fname.len() {
                        match_ok = false;
                    } else {
                        for k in 0..nbytes.len() {
                            let mut a = nbytes[k];
                            let mut b = fname[k];
                            if a >= b'a' && a <= b'z' { a = a - 32; }
                            if b >= b'a' && b <= b'z' { b = b - 32; }
                            if a != b { match_ok = false; break; }
                        }
                    }
                    
                    if match_ok {
                        found = true;
                        file_size = u32::from_le_bytes([
                            CLUSTER_DATA[off + 28],
                            CLUSTER_DATA[off + 29],
                            CLUSTER_DATA[off + 30],
                            CLUSTER_DATA[off + 31],
                        ]);
                        start_cluster = ((CLUSTER_DATA[off + 20] as u32) << 16) | 
                                         (CLUSTER_DATA[off + 26] as u32) |
                                         ((CLUSTER_DATA[off + 27] as u32) << 8);
                        break;
                    }
                }
                
                if found { break; }
                
                let next = self.next_cluster(cluster);
                if next >= 0x0FFFFFF8 { break; }
                cluster = next;
            }
            
            if !found {
                let msg = b"File not found!\n";
                for b in msg { if pos < 16383 { RESULT[pos] = *b; pos += 1; } }
                RESULT[pos] = 0;
                return core::str::from_utf8(&RESULT[..pos]).unwrap_or("");
            }
            
 
            let mut bytes_read = 0;
            let mut current_cluster = start_cluster;
            let max_read = if file_size > 15000 { 15000 } else { file_size };
            
            let header = b"\n=== File Content ===\n\n";
            for b in header { if pos < 16383 { RESULT[pos] = *b; pos += 1; } }
            
            let mut cluster_data = [0u8; 65536];
            
            while current_cluster >= 2 && current_cluster < 0x0FFFFFF8 && bytes_read < max_read {
                if !self.read_cluster(current_cluster, &mut cluster_data) {
                    break;
                }
                
                let cluster_size = (self.sectors_per_cluster as usize) * 512;
                let remaining = (max_read - bytes_read) as usize;
                let to_read = if remaining < cluster_size { remaining } else { cluster_size };
                
                for i in 0..to_read {
                    if pos < 16383 {
                        let c = cluster_data[i];
                        if c >= 0x20 && c < 0x7F || c == b'\n' || c == b'\r' || c == b'\t' {
                            RESULT[pos] = c;
                        } else {
                            RESULT[pos] = b'.';
                        }
                        pos += 1;
                        bytes_read += 1;
                    }
                }
                
                let next = self.next_cluster(current_cluster);
                if next >= 0x0FFFFFF8 { break; }
                current_cluster = next;
            }
            
            if pos < 16383 { RESULT[pos] = b'\n'; pos += 1; }
            
            let footer = b"\n=== End of File ===\n";
            for b in footer { if pos < 16383 { RESULT[pos] = *b; pos += 1; } }
            
            RESULT[pos] = 0;
            core::str::from_utf8(&RESULT[..pos]).unwrap_or("")
        }
    }
    
    pub fn get_info(&self) -> &'static str {
        static mut BUF: [u8; 256] = [0; 256];
        unsafe {
            let mut pos = 0;
            
            let msg = b"FAT32 Filesystem\n";
            for b in msg { if pos < 255 { BUF[pos] = *b; pos += 1; } }
            
            let msg2 = b"Root cluster: ";
            for b in msg2 { if pos < 255 { BUF[pos] = *b; pos += 1; } }
            let root = num_to_str(self.root_cluster as usize);
            for b in root.as_bytes() { if pos < 255 { BUF[pos] = *b; pos += 1; } }
            let msg3 = b"\n";
            for b in msg3 { if pos < 255 { BUF[pos] = *b; pos += 1; } }
            
            let msg4 = b"Sectors/cluster: ";
            for b in msg4 { if pos < 255 { BUF[pos] = *b; pos += 1; } }
            let sc = num_to_str(self.sectors_per_cluster as usize);
            for b in sc.as_bytes() { if pos < 255 { BUF[pos] = *b; pos += 1; } }
            let msg5 = b"\n";
            for b in msg5 { if pos < 255 { BUF[pos] = *b; pos += 1; } }
            
            let msg6 = b"Data start: ";
            for b in msg6 { if pos < 255 { BUF[pos] = *b; pos += 1; } }
            let ds = num_to_str(self.data_start as usize);
            for b in ds.as_bytes() { if pos < 255 { BUF[pos] = *b; pos += 1; } }
            let msg7 = b"\n";
            for b in msg7 { if pos < 255 { BUF[pos] = *b; pos += 1; } }
            
            BUF[pos] = 0;
            core::str::from_utf8(&BUF[..pos]).unwrap_or("")
        }
    }
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
 
static mut FS: Option<FAT32FS> = None;
 
pub unsafe fn mount(disk_index: usize) -> bool {
    for _ in 0..1000000 {
        unsafe { core::arch::asm!("nop", options(nostack)); }
    }
    
    if let Some(fs) = FAT32FS::new(disk_index) {
        FS = Some(fs);
        true
    } else {
        false
    }
}
 
pub unsafe fn list_files() -> &'static str {
    if let Some(fs) = &FS {
        fs.list_files()
    } else {
        "Not mounted!"
    }
}
 
pub unsafe fn read_file(filename: &str) -> &'static str {
    if let Some(fs) = &FS {
        fs.read_file(filename)
    } else {
        "Not mounted!"
    }
}
 
pub unsafe fn is_mounted() -> bool {
    FS.is_some()
}
 
pub unsafe fn info() -> &'static str {
    if let Some(fs) = &FS {
        fs.get_info()
    } else {
        "Not mounted!"
    }
}
 
 
const EOC: u32 = 0x0FFFFFFF;
const FAT_DATE: u16 = ((2026 - 1980) << 9) | (9 << 5) | 29;
const FAT_TIME: u16 = 12 << 11;
 
static mut NEXT_FREE: u32 = 2;
 
struct DirLoc {
    lba: u32,
    idx: usize,
    start: u32,
    size: u32,
    is_dir: bool,
}
 
impl FAT32FS {
    fn rd(&self, lba: u32, buf: &mut [u8; 512]) -> bool {
        unsafe { ata::read_sector(self.disk, lba, buf) }
    }
 
    fn wr(&self, lba: u32, buf: &[u8; 512]) -> bool {
        unsafe { ata::write_sector(self.disk, lba, buf) }
    }
 
    fn fat_get(&self, cluster: u32) -> Option<u32> {
        let off = cluster.checked_mul(4)?;
        let mut s = [0u8; 512];
        if !self.rd(self.fat_start + off / 512, &mut s) {
            return None;
        }
        let o = (off % 512) as usize;
        Some(u32::from_le_bytes([s[o], s[o + 1], s[o + 2], s[o + 3]]) & 0x0FFFFFFF)
    }
 
    fn fat_set(&self, cluster: u32, value: u32) -> bool {
        let off = cluster * 4;
        let idx = off / 512;
        let o = (off % 512) as usize;
        let mut s = [0u8; 512];
        if !self.rd(self.fat_start + idx, &mut s) {
            return false;
        }
        let old = u32::from_le_bytes([s[o], s[o + 1], s[o + 2], s[o + 3]]);
        let nv = (old & 0xF0000000) | (value & 0x0FFFFFFF);
        s[o..o + 4].copy_from_slice(&nv.to_le_bytes());
        for k in 0..self.num_fats as u32 {
            if !self.wr(self.fat_start + k * self.fat_sectors + idx, &s) {
                return false;
            }
        }
        true
    }
 
    fn alloc_cluster(&self) -> Option<u32> {
        let max = self.total_clusters + 2;
        let mut s = [0u8; 512];
        unsafe {
            let hint = if NEXT_FREE >= 2 && NEXT_FREE < max { NEXT_FREE } else { 2 };
            let mut pass = 0;
            while pass < 2 {
                let (from, to) = if pass == 0 { (hint, max) } else { (2, hint) };
                let mut c = from;
                while c < to {
                    let idx = c / 128;
                    if !self.rd(self.fat_start + idx, &mut s) {
                        return None;
                    }
                    let sector_end = (idx + 1) * 128;
                    let end = if sector_end < to { sector_end } else { to };
                    let mut found = 0u32;
                    let mut e = c;
                    while e < end {
                        let o = ((e % 128) * 4) as usize;
                        let v = u32::from_le_bytes([s[o], s[o + 1], s[o + 2], s[o + 3]]) & 0x0FFFFFFF;
                        if v == 0 {
                            found = e;
                            break;
                        }
                        e += 1;
                    }
                    if found >= 2 {
                        let o = ((found % 128) * 4) as usize;
                        let old = u32::from_le_bytes([s[o], s[o + 1], s[o + 2], s[o + 3]]);
                        let nv = (old & 0xF0000000) | EOC;
                        s[o..o + 4].copy_from_slice(&nv.to_le_bytes());
                        for k in 0..self.num_fats as u32 {
                            if !self.wr(self.fat_start + k * self.fat_sectors + idx, &s) {
                                return None;
                            }
                        }
                        NEXT_FREE = found + 1;
                        return Some(found);
                    }
                    c = end;
                }
                pass += 1;
            }
        }
        None
    }
 
    fn free_chain(&self, start: u32) {
        let limit = self.total_clusters + 2;
        let mut c = start;
        let mut guard = 0u32;
        while c >= 2 && c < limit && guard < 0x1000000 {
            let next = match self.fat_get(c) {
                Some(n) => n,
                None => break,
            };
            if !self.fat_set(c, 0) {
                break;
            }
            c = next;
            guard += 1;
        }
    }
 
    fn zero_cluster(&self, cluster: u32) -> bool {
        let zero = [0u8; 512];
        let base = self.cluster_to_sector(cluster);
        for i in 0..self.sectors_per_cluster as u32 {
            if !self.wr(base + i, &zero) {
                return false;
            }
        }
        true
    }
 
    fn find_entry(&self, name: &[u8; 11]) -> Option<DirLoc> {
        self.find_entry_in(self.root_cluster, name)
    }

    fn find_entry_in(&self, dir: u32, name: &[u8; 11]) -> Option<DirLoc> {
        let mut cluster = dir;
        let mut guard = 0u32;
        let mut s = [0u8; 512];
        while cluster >= 2 && cluster < 0x0FFFFFF8 && guard < 65536 {
            guard += 1;
            let base = self.cluster_to_sector(cluster);
            for sec in 0..self.sectors_per_cluster as u32 {
                if !self.rd(base + sec, &mut s) {
                    return None;
                }
                for i in 0..16usize {
                    let o = i * 32;
                    let first = s[o];
                    if first == 0x00 {
                        return None;
                    }
                    if first == 0xE5 {
                        continue;
                    }
                    let attr = s[o + 11];
                    if attr == 0x0F || (attr & 0x08) != 0 {
                        continue;
                    }
                    if s[o..o + 11] == name[..] {
                        let hi = u16::from_le_bytes([s[o + 20], s[o + 21]]) as u32;
                        let lo = u16::from_le_bytes([s[o + 26], s[o + 27]]) as u32;
                        return Some(DirLoc {
                            lba: base + sec,
                            idx: i,
                            start: (hi << 16) | lo,
                            size: u32::from_le_bytes([s[o + 28], s[o + 29], s[o + 30], s[o + 31]]),
                            is_dir: (attr & 0x10) != 0,
                        });
                    }
                }
            }
            cluster = self.fat_get(cluster)?;
        }
        None
    }
 
    fn find_free_slot(&self) -> Option<(u32, usize)> {
        self.find_free_slot_in(self.root_cluster)
    }

    fn find_free_slot_in(&self, dir: u32) -> Option<(u32, usize)> {
        let mut cluster = dir;
        let mut last = cluster;
        let mut guard = 0u32;
        let mut s = [0u8; 512];
        while cluster >= 2 && cluster < 0x0FFFFFF8 && guard < 65536 {
            guard += 1;
            let base = self.cluster_to_sector(cluster);
            for sec in 0..self.sectors_per_cluster as u32 {
                if !self.rd(base + sec, &mut s) {
                    return None;
                }
                for i in 0..16usize {
                    let first = s[i * 32];
                    if first == 0x00 || first == 0xE5 {
                        return Some((base + sec, i));
                    }
                }
            }
            last = cluster;
            let next = self.fat_get(cluster)?;
            if next < 2 || next >= 0x0FFFFFF8 {
                break;
            }
            cluster = next;
        }
        let newc = self.alloc_cluster()?;
        if !self.zero_cluster(newc) || !self.fat_set(last, newc) {
            self.free_chain(newc);
            return None;
        }
        Some((self.cluster_to_sector(newc), 0))
    }
 
    fn write_chain(&self, first: u32, data: &[u8]) -> bool {
        let mut c = first;
        let mut off = 0usize;
        let mut guard = 0u32;
        while off < data.len() {
            guard += 1;
            if c < 2 || c >= 0x0FFFFFF8 || guard > 0x1000000 {
                return false;
            }
            let base = self.cluster_to_sector(c);
            for sec in 0..self.sectors_per_cluster as u32 {
                if off >= data.len() {
                    break;
                }
                let mut buf = [0u8; 512];
                let n = if data.len() - off < 512 { data.len() - off } else { 512 };
                buf[..n].copy_from_slice(&data[off..off + n]);
                if !self.wr(base + sec, &buf) {
                    return false;
                }
                off += n;
            }
            if off < data.len() {
                c = match self.fat_get(c) {
                    Some(n) => n,
                    None => return false,
                };
            }
        }
        true
    }
 
    fn invalidate_fsinfo(&self) {
        if self.fsinfo_sector == 0 || self.fsinfo_sector == 0xFFFF {
            return;
        }
        let lba = self.part_lba + self.fsinfo_sector as u32;
        let mut s = [0u8; 512];
        if !self.rd(lba, &mut s) {
            return;
        }
        if s[0] == 0x52 && s[1] == 0x52 && s[2] == 0x61 && s[3] == 0x41 {
            for i in 488..496 {
                s[i] = 0xFF;
            }
            let _ = self.wr(lba, &s);
        }
    }
 
    pub fn write_file(&self, name: &[u8; 11], data: &[u8]) -> Result<(), &'static str> {
        self.write_file_in(self.root_cluster, name, data)
    }

    pub fn write_file_in(&self, dir: u32, name: &[u8; 11], data: &[u8]) -> Result<(), &'static str> {
        let existing = self.find_entry_in(dir, name);
        if let Some(ref e) = existing {
            if e.is_dir {
                return Err("Name is used by a folder");
            }
        }
 
        let cluster_bytes = (self.sectors_per_cluster as usize) * 512;
        let need = (data.len() + cluster_bytes - 1) / cluster_bytes;
        let mut first = 0u32;
        let mut prev = 0u32;
        for _ in 0..need {
            let c = match self.alloc_cluster() {
                Some(c) => c,
                None => {
                    if first != 0 { self.free_chain(first); }
                    return Err("Disk full");
                }
            };
            if prev != 0 {
                if !self.fat_set(prev, c) {
                    self.free_chain(first);
                    return Err("Write error");
                }
            } else {
                first = c;
            }
            prev = c;
        }
 
        if need > 0 && !self.write_chain(first, data) {
            self.free_chain(first);
            return Err("Write error");
        }
 
        let (lba, idx, is_new) = match existing {
            Some(ref e) => (e.lba, e.idx, false),
            None => match self.find_free_slot_in(dir) {
                Some((l, i)) => (l, i, true),
                None => {
                    if first != 0 { self.free_chain(first); }
                    return Err("Directory full");
                }
            },
        };
        let mut s = [0u8; 512];
        if !self.rd(lba, &mut s) {
            if first != 0 { self.free_chain(first); }
            return Err("Read error");
        }
        let o = idx * 32;
        if is_new {
            for i in 0..32 { s[o + i] = 0; }
            s[o..o + 11].copy_from_slice(name);
            s[o + 11] = 0x20;
            s[o + 14..o + 16].copy_from_slice(&FAT_TIME.to_le_bytes());
            s[o + 16..o + 18].copy_from_slice(&FAT_DATE.to_le_bytes());
            s[o + 18..o + 20].copy_from_slice(&FAT_DATE.to_le_bytes());
        }
        s[o + 20..o + 22].copy_from_slice(&((first >> 16) as u16).to_le_bytes());
        s[o + 22..o + 24].copy_from_slice(&FAT_TIME.to_le_bytes());
        s[o + 24..o + 26].copy_from_slice(&FAT_DATE.to_le_bytes());
        s[o + 26..o + 28].copy_from_slice(&((first & 0xFFFF) as u16).to_le_bytes());
        s[o + 28..o + 32].copy_from_slice(&(data.len() as u32).to_le_bytes());
        if !self.wr(lba, &s) {
            if first != 0 { self.free_chain(first); }
            return Err("Write error");
        }
 
        if let Some(ref e) = existing {
            if e.start >= 2 {
                self.free_chain(e.start);
            }
        }
        self.invalidate_fsinfo();
        Ok(())
    }
 
    pub fn read_file_bytes(&self, name: &[u8; 11], out: &mut [u8]) -> Result<usize, &'static str> {
        self.read_file_bytes_in(self.root_cluster, name, out)
    }

    pub fn read_file_bytes_in(&self, dir: u32, name: &[u8; 11], out: &mut [u8]) -> Result<usize, &'static str> {
        let e = match self.find_entry_in(dir, name) {
            Some(e) => e,
            None => return Err("File not found"),
        };
        if e.is_dir {
            return Err("This is a folder");
        }
        let size = e.size as usize;
        if size > out.len() {
            return Err("File too big");
        }
        let mut c = e.start;
        let mut off = 0usize;
        let mut s = [0u8; 512];
        let mut guard = 0u32;
        while off < size {
            guard += 1;
            if c < 2 || c >= 0x0FFFFFF8 || guard > 0x1000000 {
                return Err("Read error");
            }
            let base = self.cluster_to_sector(c);
            for sec in 0..self.sectors_per_cluster as u32 {
                if off >= size {
                    break;
                }
                if !self.rd(base + sec, &mut s) {
                    return Err("Read error");
                }
                let n = if size - off < 512 { size - off } else { 512 };
                out[off..off + n].copy_from_slice(&s[..n]);
                off += n;
            }
            if off < size {
                c = match self.fat_get(c) {
                    Some(n) => n,
                    None => return Err("Read error"),
                };
            }
        }
        Ok(size)
    }
}
 

#[derive(Clone, Copy)]
pub struct FsEntry {
    pub name: [u8; 13],
    pub name_len: usize,
    pub is_dir: bool,
    pub size: u32,
    pub cluster: u32,
}

impl FsEntry {
    pub const EMPTY: FsEntry = FsEntry { name: [0; 13], name_len: 0, is_dir: false, size: 0, cluster: 0 };
}

impl FAT32FS {
    pub fn root(&self) -> u32 {
        self.root_cluster
    }

    pub fn mkdir_in(&self, dir: u32, name: &[u8; 11]) -> Result<(), &'static str> {
        if self.find_entry_in(dir, name).is_some() {
            return Err("Name already exists");
        }
        let c = match self.alloc_cluster() {
            Some(c) => c,
            None => return Err("Disk full"),
        };
        if !self.zero_cluster(c) {
            self.free_chain(c);
            return Err("Write error");
        }

        let parent = if dir == self.root_cluster { 0u32 } else { dir };
        let mut s = [0u8; 512];
        for i in 0..11 {
            s[i] = b' ';
            s[32 + i] = b' ';
        }
        s[0] = b'.';
        s[32] = b'.';
        s[33] = b'.';
        s[11] = 0x10;
        s[32 + 11] = 0x10;
        for e in 0..2usize {
            let o = e * 32;
            let cl = if e == 0 { c } else { parent };
            s[o + 14..o + 16].copy_from_slice(&FAT_TIME.to_le_bytes());
            s[o + 16..o + 18].copy_from_slice(&FAT_DATE.to_le_bytes());
            s[o + 18..o + 20].copy_from_slice(&FAT_DATE.to_le_bytes());
            s[o + 20..o + 22].copy_from_slice(&((cl >> 16) as u16).to_le_bytes());
            s[o + 22..o + 24].copy_from_slice(&FAT_TIME.to_le_bytes());
            s[o + 24..o + 26].copy_from_slice(&FAT_DATE.to_le_bytes());
            s[o + 26..o + 28].copy_from_slice(&((cl & 0xFFFF) as u16).to_le_bytes());
        }
        if !self.wr(self.cluster_to_sector(c), &s) {
            self.free_chain(c);
            return Err("Write error");
        }

        let (lba, idx) = match self.find_free_slot_in(dir) {
            Some(x) => x,
            None => {
                self.free_chain(c);
                return Err("Directory full");
            }
        };
        let mut p = [0u8; 512];
        if !self.rd(lba, &mut p) {
            self.free_chain(c);
            return Err("Read error");
        }
        let o = idx * 32;
        for i in 0..32 { p[o + i] = 0; }
        p[o..o + 11].copy_from_slice(name);
        p[o + 11] = 0x10;
        p[o + 14..o + 16].copy_from_slice(&FAT_TIME.to_le_bytes());
        p[o + 16..o + 18].copy_from_slice(&FAT_DATE.to_le_bytes());
        p[o + 18..o + 20].copy_from_slice(&FAT_DATE.to_le_bytes());
        p[o + 20..o + 22].copy_from_slice(&((c >> 16) as u16).to_le_bytes());
        p[o + 22..o + 24].copy_from_slice(&FAT_TIME.to_le_bytes());
        p[o + 24..o + 26].copy_from_slice(&FAT_DATE.to_le_bytes());
        p[o + 26..o + 28].copy_from_slice(&((c & 0xFFFF) as u16).to_le_bytes());
        if !self.wr(lba, &p) {
            self.free_chain(c);
            return Err("Write error");
        }
        self.invalidate_fsinfo();
        Ok(())
    }

    pub fn list_dir(&self, dir: u32, out: &mut [FsEntry]) -> usize {
        let mut n = 0usize;
        let mut cluster = dir;
        let mut guard = 0u32;
        let mut s = [0u8; 512];
        while cluster >= 2 && cluster < 0x0FFFFFF8 && guard < 65536 {
            guard += 1;
            let base = self.cluster_to_sector(cluster);
            for sec in 0..self.sectors_per_cluster as u32 {
                if !self.rd(base + sec, &mut s) {
                    return n;
                }
                for i in 0..16usize {
                    let o = i * 32;
                    let first = s[o];
                    if first == 0x00 {
                        return n;
                    }
                    if first == 0xE5 {
                        continue;
                    }
                    let attr = s[o + 11];
                    if attr == 0x0F || (attr & 0x08) != 0 {
                        continue;
                    }
                    if n >= out.len() {
                        return n;
                    }
                    let mut e = FsEntry::EMPTY;
                    let mut p = 0usize;
                    for j in 0..8usize {
                        let ch = s[o + j];
                        if ch == b' ' { break; }
                        if ch >= 0x20 && ch < 0x7F {
                            e.name[p] = ch;
                            p += 1;
                        }
                    }
                    if p == 0 { continue; }
                    if s[o + 8] != b' ' && s[o + 8] != 0 {
                        e.name[p] = b'.';
                        p += 1;
                        for j in 0..3usize {
                            let ch = s[o + 8 + j];
                            if ch == b' ' || ch == 0 { break; }
                            if ch >= 0x20 && ch < 0x7F {
                                e.name[p] = ch;
                                p += 1;
                            }
                        }
                    }
                    if (p == 1 && e.name[0] == b'.') || (p == 2 && e.name[0] == b'.' && e.name[1] == b'.') {
                        continue;
                    }
                    e.name_len = p;
                    e.is_dir = (attr & 0x10) != 0;
                    e.size = u32::from_le_bytes([s[o + 28], s[o + 29], s[o + 30], s[o + 31]]);
                    let hi = u16::from_le_bytes([s[o + 20], s[o + 21]]) as u32;
                    let lo = u16::from_le_bytes([s[o + 26], s[o + 27]]) as u32;
                    e.cluster = (hi << 16) | lo;
                    out[n] = e;
                    n += 1;
                }
            }
            cluster = match self.fat_get(cluster) {
                Some(c) => c,
                None => return n,
            };
        }
        n
    }
}

impl FAT32FS {
    pub fn delete_file_in(&self, dir: u32, name: &[u8; 11]) -> Result<(), &'static str> {
        let e = match self.find_entry_in(dir, name) {
            Some(e) => e,
            None => return Err("File not found"),
        };
        if e.is_dir {
            return Err("This is a folder");
        }
        let mut s = [0u8; 512];
        if !self.rd(e.lba, &mut s) {
            return Err("Read error");
        }
        s[e.idx * 32] = 0xE5;
        let mut k = e.idx;
        while k > 0 {
            k -= 1;
            let first = s[k * 32];
            if s[k * 32 + 11] == 0x0F && first != 0xE5 && first != 0x00 {
                s[k * 32] = 0xE5;
            } else {
                break;
            }
        }
        if !self.wr(e.lba, &s) {
            return Err("Write error");
        }
        if e.start >= 2 {
            self.free_chain(e.start);
        }
        self.invalidate_fsinfo();
        Ok(())
    }

    pub fn resolve_file(&self, path: &str) -> Result<(u32, [u8; 11]), &'static str> {
        let mut dir = self.root_cluster;
        let mut parts = path.split('/').filter(|p| !p.is_empty()).peekable();
        loop {
            let part = match parts.next() {
                Some(p) => p,
                None => return Err("Bad path"),
            };
            let short = match make_short_name(part) {
                Some(n) => n,
                None => return Err("Bad path"),
            };
            if parts.peek().is_none() {
                return match self.find_entry_in(dir, &short) {
                    Some(e) => {
                        if e.is_dir { Err("This is a folder") } else { Ok((dir, short)) }
                    }
                    None => Err("File not found"),
                };
            }
            match self.find_entry_in(dir, &short) {
                Some(e) => {
                    if !e.is_dir {
                        return Err("Folder not found");
                    }
                    dir = if e.start < 2 { self.root_cluster } else { e.start };
                }
                None => return Err("Folder not found"),
            }
        }
    }
}

pub fn make_dir_name(name: &str) -> Option<[u8; 11]> {
    let mut out = [b' '; 11];
    let mut n = 0usize;
    for &b in name.as_bytes() {
        if b == b' ' || b == b'.' { continue; }
        if n >= 8 { break; }
        out[n] = short_char(b);
        n += 1;
    }
    if n == 0 { None } else { Some(out) }
}

 
fn short_char(b: u8) -> u8 {
    let u = if b >= b'a' && b <= b'z' { b - 32 } else { b };
    if (u >= b'A' && u <= b'Z') || (u >= b'0' && u <= b'9') {
        return u;
    }
    match u {
        b'!' | b'#' | b'$' | b'%' | b'&' | b'\'' | b'(' | b')' | b'-' | b'@' |
        b'^' | b'_' | b'`' | b'{' | b'}' | b'~' => u,
        _ => b'_',
    }
}
 
pub fn make_short_name(name: &str) -> Option<[u8; 11]> {
    let bytes = name.as_bytes();
    let mut dot: Option<usize> = None;
    for i in 0..bytes.len() {
        if bytes[i] == b'.' && i > 0 {
            dot = Some(i);
        }
    }
    let (base, ext): (&[u8], &[u8]) = match dot {
        Some(p) => (&bytes[..p], &bytes[p + 1..]),
        None => (bytes, &bytes[bytes.len()..]),
    };
 
    let mut out = [b' '; 11];
    let mut n = 0usize;
    for &b in base {
        if b == b' ' || b == b'.' { continue; }
        if n >= 8 { break; }
        out[n] = short_char(b);
        n += 1;
    }
    if n == 0 {
        return None;
    }
    let mut m = 0usize;
    for &b in ext {
        if b == b' ' || b == b'.' { continue; }
        if m >= 3 { break; }
        out[8 + m] = short_char(b);
        m += 1;
    }
    Some(out)
}
 
pub fn display_name(name: &str, out: &mut [u8; 13]) -> usize {
    let n = match make_short_name(name) {
        Some(n) => n,
        None => return 0,
    };
    let mut p = 0usize;
    for i in 0..8 {
        if n[i] != b' ' { out[p] = n[i]; p += 1; }
    }
    if n[8] != b' ' {
        out[p] = b'.';
        p += 1;
        for i in 8..11 {
            if n[i] != b' ' { out[p] = n[i]; p += 1; }
        }
    }
    p
}
 
 
const MSG_NOT_FAT32: &str = "Not FAT32! Terminal: format 1 /y";
 
pub unsafe fn ensure_mounted() -> bool {
    if FS.is_some() {
        return true;
    }
    let n = ata::disk_count();
    for i in 0..n {
        match ata::get_disk(i) {
            Some(info) => {
                if !info.present || info.dev_type != ata::DeviceType::ATA {
                    continue;
                }
            }
            None => continue,
        }
        if mount(i) {
            return true;
        }
    }
    false
}
 
pub unsafe fn save_file(name: &str, data: &[u8]) -> Result<(), &'static str> {
    if ata::disk_count() == 0 {
        return Err("No disk found");
    }
    if !ensure_mounted() {
        return Err(MSG_NOT_FAT32);
    }
    let short = match make_short_name(name) {
        Some(n) => n,
        None => return Err("Bad file name"),
    };
    let r = match &FS {
        Some(fs) => fs.write_file(&short, data),
        None => Err(MSG_NOT_FAT32),
    };
    if let Some(fs) = &FS {
        let _ = ata::flush(fs.disk);
    }
    r
}
 
pub unsafe fn load_file(name: &str, out: &mut [u8]) -> Result<usize, &'static str> {
    if ata::disk_count() == 0 {
        return Err("No disk found");
    }
    if !ensure_mounted() {
        return Err(MSG_NOT_FAT32);
    }
    let short = match make_short_name(name) {
        Some(n) => n,
        None => return Err("Bad file name"),
    };
    match &FS {
        Some(fs) => fs.read_file_bytes(&short, out),
        None => Err(MSG_NOT_FAT32),
    }
}
 

pub const SYS_DIR: &str = "MiUX";

fn dir_cluster(fs: &FAT32FS, name: &str, create: bool) -> Result<u32, &'static str> {
    let short = match make_dir_name(name) {
        Some(n) => n,
        None => return Err("Bad folder name"),
    };
    if let Some(e) = fs.find_entry_in(fs.root_cluster, &short) {
        if !e.is_dir {
            return Err("Name is used by a file");
        }
        return Ok(if e.start < 2 { fs.root_cluster } else { e.start });
    }
    if !create {
        return Err("Folder not found");
    }
    fs.mkdir_in(fs.root_cluster, &short)?;
    match fs.find_entry_in(fs.root_cluster, &short) {
        Some(e) => {
            if e.is_dir && e.start >= 2 { Ok(e.start) } else { Err("Cannot create folder") }
        }
        None => Err("Cannot create folder"),
    }
}

pub unsafe fn create_dir(name: &str) -> Result<(), &'static str> {
    if ata::disk_count() == 0 {
        return Err("No disk found");
    }
    if !ensure_mounted() {
        return Err(MSG_NOT_FAT32);
    }
    let r = match &FS {
        Some(fs) => match dir_cluster(fs, name, true) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        },
        None => Err(MSG_NOT_FAT32),
    };
    if let Some(fs) = &FS {
        let _ = ata::flush(fs.disk);
    }
    r
}

pub unsafe fn create_root_dir_unique(name: &str, shown: &mut [u8; 13]) -> Result<(usize, usize), &'static str> {
    if ata::disk_count() == 0 {
        return Err("No disk found");
    }
    if !ensure_mounted() {
        return Err(MSG_NOT_FAT32);
    }
    let base = match make_dir_name(name) {
        Some(n) => n,
        None => return Err("Bad folder name"),
    };
    let fs = match &FS {
        Some(f) => f,
        None => return Err(MSG_NOT_FAT32),
    };
    let mut blen = 0usize;
    while blen < 8 && base[blen] != b' ' {
        blen += 1;
    }
    let mut short = base;
    let mut n = 0u32;
    while fs.find_entry_in(fs.root_cluster, &short).is_some() {
        n += 1;
        if n > 99 {
            return Err("Name already exists");
        }
        let mut digits = [0u8; 2];
        let dl;
        if n < 10 {
            digits[0] = b'0' + n as u8;
            dl = 1usize;
        } else {
            digits[0] = b'0' + (n / 10) as u8;
            digits[1] = b'0' + (n % 10) as u8;
            dl = 2usize;
        }
        let keep = if blen + dl > 8 { 8 - dl } else { blen };
        short = [b' '; 11];
        short[..keep].copy_from_slice(&base[..keep]);
        short[keep..keep + dl].copy_from_slice(&digits[..dl]);
    }
    let r = fs.mkdir_in(fs.root_cluster, &short);
    let _ = ata::flush(fs.disk);
    r?;
    let mut p = 0usize;
    for i in 0..8 {
        if short[i] != b' ' {
            shown[p] = short[i];
            p += 1;
        }
    }
    Ok((fs.disk, p))
}

pub unsafe fn save_file_in_dir(dir: &str, name: &str, data: &[u8]) -> Result<(), &'static str> {
    if ata::disk_count() == 0 {
        return Err("No disk found");
    }
    if !ensure_mounted() {
        return Err(MSG_NOT_FAT32);
    }
    let short = match make_short_name(name) {
        Some(n) => n,
        None => return Err("Bad file name"),
    };
    let r = match &FS {
        Some(fs) => match dir_cluster(fs, dir, true) {
            Ok(d) => fs.write_file_in(d, &short, data),
            Err(e) => Err(e),
        },
        None => Err(MSG_NOT_FAT32),
    };
    if let Some(fs) = &FS {
        let _ = ata::flush(fs.disk);
    }
    r
}

pub unsafe fn load_file_in_dir(dir: &str, name: &str, out: &mut [u8]) -> Result<usize, &'static str> {
    if ata::disk_count() == 0 {
        return Err("No disk found");
    }
    if !ensure_mounted() {
        return Err(MSG_NOT_FAT32);
    }
    let short = match make_short_name(name) {
        Some(n) => n,
        None => return Err("Bad file name"),
    };
    match &FS {
        Some(fs) => match dir_cluster(fs, dir, false) {
            Ok(d) => fs.read_file_bytes_in(d, &short, out),
            Err(e) => Err(e),
        },
        None => Err(MSG_NOT_FAT32),
    }
}

pub const USERS_DIR: &str = "users";

fn subdir_cluster(fs: &FAT32FS, dir: &str, sub: &str, create: bool) -> Result<u32, &'static str> {
    let parent = dir_cluster(fs, dir, create)?;
    let short = match make_dir_name(sub) {
        Some(n) => n,
        None => return Err("Bad folder name"),
    };
    if let Some(e) = fs.find_entry_in(parent, &short) {
        if !e.is_dir {
            return Err("Name is used by a file");
        }
        return Ok(if e.start < 2 { fs.root_cluster } else { e.start });
    }
    if !create {
        return Err("Folder not found");
    }
    fs.mkdir_in(parent, &short)?;
    match fs.find_entry_in(parent, &short) {
        Some(e) => {
            if e.is_dir && e.start >= 2 { Ok(e.start) } else { Err("Cannot create folder") }
        }
        None => Err("Cannot create folder"),
    }
}

pub unsafe fn create_subdir(dir: &str, sub: &str) -> Result<(), &'static str> {
    if ata::disk_count() == 0 {
        return Err("No disk found");
    }
    if !ensure_mounted() {
        return Err(MSG_NOT_FAT32);
    }
    let r = match &FS {
        Some(fs) => match subdir_cluster(fs, dir, sub, true) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        },
        None => Err(MSG_NOT_FAT32),
    };
    if let Some(fs) = &FS {
        let _ = ata::flush(fs.disk);
    }
    r
}

pub unsafe fn save_file_in_subdir(dir: &str, sub: &str, name: &str, data: &[u8]) -> Result<(), &'static str> {
    if ata::disk_count() == 0 {
        return Err("No disk found");
    }
    if !ensure_mounted() {
        return Err(MSG_NOT_FAT32);
    }
    let short = match make_short_name(name) {
        Some(n) => n,
        None => return Err("Bad file name"),
    };
    let r = match &FS {
        Some(fs) => match subdir_cluster(fs, dir, sub, true) {
            Ok(d) => fs.write_file_in(d, &short, data),
            Err(e) => Err(e),
        },
        None => Err(MSG_NOT_FAT32),
    };
    if let Some(fs) = &FS {
        let _ = ata::flush(fs.disk);
    }
    r
}

pub unsafe fn load_file_in_subdir(dir: &str, sub: &str, name: &str, out: &mut [u8]) -> Result<usize, &'static str> {
    if ata::disk_count() == 0 {
        return Err("No disk found");
    }
    if !ensure_mounted() {
        return Err(MSG_NOT_FAT32);
    }
    let short = match make_short_name(name) {
        Some(n) => n,
        None => return Err("Bad file name"),
    };
    match &FS {
        Some(fs) => match subdir_cluster(fs, dir, sub, false) {
            Ok(d) => fs.read_file_bytes_in(d, &short, out),
            Err(e) => Err(e),
        },
        None => Err(MSG_NOT_FAT32),
    }
}

 
fn write_zero_sectors(disk: usize, from: u32, count: u32) -> bool {
    let zero = [0u8; 512];
    for i in 0..count {
        unsafe {
            if !ata::write_sector(disk, from + i, &zero) {
                return false;
            }
        }
    }
    true
}
 
pub unsafe fn format(disk_index: usize, label: &str) -> Result<(), &'static str> {
    let info = match ata::get_disk(disk_index) {
        Some(i) => i,
        None => return Err("Disk not found"),
    };
    if !info.present {
        return Err("Disk not found");
    }
    if info.dev_type != ata::DeviceType::ATA {
        return Err("This device is not a hard disk");
    }
    if info.sectors == 0 {
        return Err("Disk size unknown (IDENTIFY failed)");
    }
    let disk_sectors: u32 = if info.sectors > 0x0FFF_FFFF { 0x0FFF_FFFF } else { info.sectors as u32 };
 
    const PART_START: u32 = 2048;
    const RESERVED: u32 = 32;
    if disk_sectors <= PART_START + 512 {
        return Err("Disk too small (min 2 MB)");
    }
    let part_sectors = disk_sectors - PART_START;
 
    let spc: u32 = if part_sectors <= 532480 { 1 }
        else if part_sectors <= 16777216 { 8 }
        else if part_sectors <= 33554432 { 16 }
        else if part_sectors <= 67108864 { 32 }
        else { 64 };
 
    let tmp1 = part_sectors - RESERVED;
    let tmp2 = (256 * spc + 2) / 2;
    let fat_sz = (tmp1 + tmp2 - 1) / tmp2;
    let data_sectors = part_sectors - RESERVED - 2 * fat_sz;
    let clusters = data_sectors / spc;
    if clusters < 64 {
        return Err("Disk too small (min 2 MB)");
    }
 
    let mut lab = [b' '; 11];
    let mut n = 0usize;
    for &b in label.as_bytes() {
        if n >= 11 { break; }
        let u = if b >= b'a' && b <= b'z' { b - 32 } else { b };
        if u >= 0x20 && u < 0x7F {
            lab[n] = u;
            n += 1;
        }
    }
    let vol_id: u32 = 0x4D69_4F53 ^ part_sectors;
 
    let mut mbr = [0u8; 512];
    mbr[440..444].copy_from_slice(&vol_id.to_le_bytes());
    mbr[446] = 0x00;
    mbr[447] = 0xFE; mbr[448] = 0xFF; mbr[449] = 0xFF;
    mbr[450] = 0x0C;
    mbr[451] = 0xFE; mbr[452] = 0xFF; mbr[453] = 0xFF;
    mbr[454..458].copy_from_slice(&PART_START.to_le_bytes());
    mbr[458..462].copy_from_slice(&part_sectors.to_le_bytes());
    mbr[510] = 0x55;
    mbr[511] = 0xAA;
    if !ata::write_sector(disk_index, 0, &mbr) {
        return Err("Write error (MBR)");
    }
    let _ = write_zero_sectors(disk_index, 1, PART_START - 1);
    if info.sectors <= 0x0FFF_FFFF {
        let _ = write_zero_sectors(disk_index, disk_sectors - 33, 33);
    }
 
    let mut b = [0u8; 512];
    b[0] = 0xEB; b[1] = 0x58; b[2] = 0x90;
    b[3..11].copy_from_slice(b"MSWIN4.1");
    b[11] = 0x00; b[12] = 0x02;
    b[13] = spc as u8;
    b[14..16].copy_from_slice(&(RESERVED as u16).to_le_bytes());
    b[16] = 2;
    b[21] = 0xF8;
    b[24..26].copy_from_slice(&63u16.to_le_bytes());
    b[26..28].copy_from_slice(&255u16.to_le_bytes());
    b[28..32].copy_from_slice(&PART_START.to_le_bytes());
    b[32..36].copy_from_slice(&part_sectors.to_le_bytes());
    b[36..40].copy_from_slice(&fat_sz.to_le_bytes());
    b[44..48].copy_from_slice(&2u32.to_le_bytes());
    b[48..50].copy_from_slice(&1u16.to_le_bytes());
    b[50..52].copy_from_slice(&6u16.to_le_bytes());
    b[64] = 0x80;
    b[66] = 0x29;
    b[67..71].copy_from_slice(&vol_id.to_le_bytes());
    b[71..82].copy_from_slice(&lab);
    b[82..90].copy_from_slice(b"FAT32   ");
    b[510] = 0x55;
    b[511] = 0xAA;
 
    let mut fi = [0u8; 512];
    fi[0..4].copy_from_slice(&0x41615252u32.to_le_bytes());
    fi[484..488].copy_from_slice(&0x61417272u32.to_le_bytes());
    fi[488..492].copy_from_slice(&(clusters - 1).to_le_bytes());
    fi[492..496].copy_from_slice(&3u32.to_le_bytes());
    fi[508] = 0x00; fi[509] = 0x00; fi[510] = 0x55; fi[511] = 0xAA;
 
    let p = PART_START;
    if !write_zero_sectors(disk_index, p, RESERVED) {
        return Err("Write error (reserved area)");
    }
    if !ata::write_sector(disk_index, p, &b) || !ata::write_sector(disk_index, p + 1, &fi)
        || !ata::write_sector(disk_index, p + 6, &b) || !ata::write_sector(disk_index, p + 7, &fi) {
        return Err("Write error (boot sector)");
    }
 
    let mut first_fat = [0u8; 512];
    first_fat[0..4].copy_from_slice(&0x0FFFFFF8u32.to_le_bytes());
    first_fat[4..8].copy_from_slice(&0x0FFFFFFFu32.to_le_bytes());
    first_fat[8..12].copy_from_slice(&EOC.to_le_bytes());
    for k in 0..2u32 {
        let start = p + RESERVED + k * fat_sz;
        if !ata::write_sector(disk_index, start, &first_fat) {
            return Err("Write error (FAT)");
        }
        if !write_zero_sectors(disk_index, start + 1, fat_sz - 1) {
            return Err("Write error (FAT)");
        }
    }
 
    let data_start = p + RESERVED + 2 * fat_sz;
    if !write_zero_sectors(disk_index, data_start, spc) {
        return Err("Write error (root folder)");
    }
 
    let _ = ata::flush(disk_index);
 
    NEXT_FREE = 2;
    FS = None;
    if !mount(disk_index) {
        return Err("Formatted, but mount failed");
    }
    Ok(())
}


pub struct TarInstall {
    pub disk: usize,
    pub files: usize,
    pub has_main: bool,
    pub has_app: bool,
}

impl FAT32FS {
    pub fn disk(&self) -> usize {
        self.disk
    }

    fn subdir(&self, parent: u32, short: &[u8; 11]) -> Result<u32, &'static str> {
        if let Some(e) = self.find_entry_in(parent, short) {
            if !e.is_dir {
                return Err("Name is used by a file");
            }
            return Ok(if e.start < 2 { self.root_cluster } else { e.start });
        }
        self.mkdir_in(parent, short)?;
        match self.find_entry_in(parent, short) {
            Some(e) if e.is_dir && e.start >= 2 => Ok(e.start),
            _ => Err("Cannot create folder"),
        }
    }

    pub fn install_tar(&self, folder: &str, tar: &[u8]) -> Result<TarInstall, &'static str> {
        let fshort = match make_dir_name(folder) {
            Some(n) => n,
            None => return Err("Bad folder name"),
        };
        let root = self.subdir(self.root_cluster, &fshort)?;
        let mut files = 0usize;

        let entries = crate::fs::pkgtar::walk(tar, |path, is_dir, data| {
            if path.split('/').any(|p| p == "..") {
                return Ok(());
            }
            let mut dir = root;
            let mut it = path.split('/').filter(|p| !p.is_empty() && *p != ".").peekable();
            loop {
                let part = match it.next() {
                    Some(p) => p,
                    None => break,
                };
                let last = it.peek().is_none();
                if last && !is_dir {
                    let short = match make_short_name(part) {
                        Some(s) => s,
                        None => return Ok(()),
                    };
                    self.write_file_in(dir, &short, data)?;
                    files += 1;
                } else {
                    let short = match make_dir_name(part) {
                        Some(s) => s,
                        None => return Ok(()),
                    };
                    dir = self.subdir(dir, &short)?;
                }
            }
            Ok(())
        });
        unsafe { let _ = ata::flush(self.disk); }
        let entries = entries?;
        if entries == 0 {
            return Err("empty file or not a tar");
        }

        let has = |name: &str| -> bool {
            match make_short_name(name) {
                Some(s) => match self.find_entry_in(root, &s) {
                    Some(e) => !e.is_dir,
                    None => false,
                },
                None => false,
            }
        };
        Ok(TarInstall {
            disk: self.disk,
            files,
            has_main: has("main.py"),
            has_app: has("app.py"),
        })
    }
}

pub unsafe fn install_tar(folder: &str, tar: &[u8]) -> Result<TarInstall, &'static str> {
    if ata::disk_count() == 0 {
        return Err("No disk found");
    }
    if !ensure_mounted() {
        return Err(MSG_NOT_FAT32);
    }
    match &FS {
        Some(fs) => fs.install_tar(folder, tar),
        None => Err(MSG_NOT_FAT32),
    }
}

pub unsafe fn fs_ref() -> Option<&'static FAT32FS> {
    (*core::ptr::addr_of!(FS)).as_ref()
}

pub unsafe fn sys_dir_cluster(name: &str) -> Option<u32> {
    let fs = fs_ref()?;
    dir_cluster(fs, name, false).ok()
}
