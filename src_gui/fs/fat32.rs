
use crate::ata::AtaDrive;

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Fat32BootSector {
    _jmp: [u8; 3],
    _oem: [u8; 8],
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sectors: u16,
    pub num_fats: u8,
    _root_entries: u16,
    _total_sectors_16: u16,
    _media: u8,
    _sectors_per_fat_16: u16,
    _sectors_per_track: u16,
    _num_heads: u16,
    _hidden_sectors: u32,
    _total_sectors_32: u32,
    pub sectors_per_fat_32: u32,
    _flags: u16,
    _version: u16,
    pub root_cluster: u32,
    _fsinfo_sector: u16,
    _backup_boot_sector: u16,
    _reserved: [u8; 12],
    _drive_number: u8,
    _nt_flags: u8,
    _signature: u8,
    _serial: u32,
    _label: [u8; 11],
    _system_id: [u8; 8],
    _boot_code: [u8; 420],
    boot_signature: u16,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct FatDirEntry {
    pub name: [u8; 8],
    pub ext: [u8; 3],
    pub attributes: u8,
    _reserved: u8,
    _create_time_tenth: u8,
    _create_time: u16,
    _create_date: u16,
    _access_date: u16,
    pub first_cluster_high: u16,
    _modify_time: u16,
    _modify_date: u16,
    pub first_cluster_low: u16,
    pub size: u32,
}

impl FatDirEntry {
    pub fn is_empty(&self) -> bool { self.name[0] == 0x00 }
    pub fn is_deleted(&self) -> bool { self.name[0] == 0xE5 }
    pub fn is_directory(&self) -> bool { self.attributes & 0x10 != 0 }
    pub fn is_long_name(&self) -> bool { self.attributes == 0x0F }
    pub fn first_cluster(&self) -> u32 {
        ((self.first_cluster_high as u32) << 16) | (self.first_cluster_low as u32)
    }
    
    pub fn set_first_cluster(&mut self, cluster: u32) {
        self.first_cluster_low = cluster as u16;
        self.first_cluster_high = (cluster >> 16) as u16;
    }
    
    pub fn clear(&mut self) {
        unsafe { core::ptr::write_bytes(self as *mut Self, 0, 1); }
    }
    
    pub fn set_deleted(&mut self) {
        self.name[0] = 0xE5;
    }
    
    pub fn set_name(&mut self, name: &str) {

        for i in 0..8 { self.name[i] = b' '; }
        for i in 0..3 { self.ext[i] = b' '; }
        
  
        let dot_pos = name.find('.');
        
        if let Some(pos) = dot_pos {
            let base = &name[..pos];
            let ext = &name[pos + 1..];
            
            for (i, &b) in base.as_bytes().iter().take(8).enumerate() {
                self.name[i] = b.to_ascii_uppercase();
            }
            for (i, &b) in ext.as_bytes().iter().take(3).enumerate() {
                self.ext[i] = b.to_ascii_uppercase();
            }
        } else {
            for (i, &b) in name.as_bytes().iter().take(8).enumerate() {
                self.name[i] = b.to_ascii_uppercase();
            }
        }
    }
    
    pub fn name_matches(&self, target: &str) -> bool {
        let mut buf = [0u8; 13];
        let len = self.name_to_buf(&mut buf);
        let entry_name = core::str::from_utf8(&buf[..len]).unwrap_or("");
        
        if entry_name.len() != target.len() { return false; }
        
        for (a, b) in entry_name.bytes().zip(target.bytes()) {
            if a.to_ascii_uppercase() != b.to_ascii_uppercase() { return false; }
        }
        true
    }
    
    pub fn name_to_buf(&self, buf: &mut [u8]) -> usize {
        let mut pos = 0;
        for &b in &self.name {
            if b == b' ' || b == 0 { break; }
            if pos < buf.len() { buf[pos] = b; pos += 1; }
        }
        if self.ext[0] != b' ' && !self.is_directory() {
            if pos < buf.len() { buf[pos] = b'.'; pos += 1; }
            for &b in &self.ext {
                if b == b' ' || b == 0 { break; }
                if pos < buf.len() { buf[pos] = b; pos += 1; }
            }
        }
        pos
    }
}

pub struct Fat32FS {
    pub boot: Fat32BootSector,
    pub fat_start_sector: u32,
    pub data_start_sector: u32,
    pub cluster_size: u32,
    pub sectors_per_cluster: u8,
    pub current_dir_cluster: u32,
}

impl Fat32FS {
    pub fn new(ata: &AtaDrive) -> Option<Self> {
        let mut sector = [0u8; 512];
        if !ata.read_sector(0, &mut sector) {
            return None;
        }
        
        let boot = unsafe { &*(sector.as_ptr() as *const Fat32BootSector) };
        if boot.boot_signature != 0xAA55 {
            return None;
        }
        
        let fat_start = boot.reserved_sectors as u32;
        let data_start = fat_start + (boot.num_fats as u32 * boot.sectors_per_fat_32);
        
        Some(Self {
            boot: *boot,
            fat_start_sector: fat_start,
            data_start_sector: data_start,
            cluster_size: boot.sectors_per_cluster as u32 * boot.bytes_per_sector as u32,
            sectors_per_cluster: boot.sectors_per_cluster,
            current_dir_cluster: boot.root_cluster,
        })
    }
    
    fn cluster_to_sector(&self, cluster: u32) -> u32 {
        self.data_start_sector + (cluster - 2) * self.sectors_per_cluster as u32
    }
    
    fn next_cluster(&self, ata: &AtaDrive, cluster: u32) -> u32 {
        let fat_offset = cluster * 4;
        let fat_sector = self.fat_start_sector + (fat_offset / 512);
        let offset_in_sector = (fat_offset % 512) as usize;
        
        let mut sector = [0u8; 512];
        if !ata.read_sector(fat_sector, &mut sector) {
            return 0;
        }
        
        let next = u32::from_le_bytes([
            sector[offset_in_sector],
            sector[offset_in_sector + 1],
            sector[offset_in_sector + 2],
            sector[offset_in_sector + 3],
        ]) & 0x0FFFFFFF;
        
        if next >= 0x0FFFFFF8 { 0 } else { next }
    }
    

    fn find_free_cluster(&self, ata: &AtaDrive) -> Option<u32> {
        let total_clusters = self.boot.sectors_per_fat_32 * 512 / 4;
        
        for cluster in 2..total_clusters {
            let fat_offset = cluster * 4;
            let fat_sector = self.fat_start_sector + (fat_offset / 512);
            let offset_in_sector = (fat_offset % 512) as usize;
            
            let mut sector = [0u8; 512];
            if !ata.read_sector(fat_sector, &mut sector) { continue; }
            
            let value = u32::from_le_bytes([
                sector[offset_in_sector],
                sector[offset_in_sector + 1],
                sector[offset_in_sector + 2],
                sector[offset_in_sector + 3],
            ]) & 0x0FFFFFFF;
            
            if value == 0 {
                return Some(cluster);
            }
        }
        None
    }
    

    fn alloc_cluster(&self, ata: &AtaDrive, cluster: u32) -> bool {
        let fat_offset = cluster * 4;
        let fat_sector = self.fat_start_sector + (fat_offset / 512);
        let offset_in_sector = (fat_offset % 512) as usize;
        
        let mut sector = [0u8; 512];
        if !ata.read_sector(fat_sector, &mut sector) { return false; }
        

        sector[offset_in_sector] = 0xF8;
        sector[offset_in_sector + 1] = 0xFF;
        sector[offset_in_sector + 2] = 0xFF;
        sector[offset_in_sector + 3] = 0x0F;
        
        ata.write_sector(fat_sector, &sector)
    }
    

    fn find_free_entry(&self, ata: &AtaDrive, dir_cluster: u32) -> Option<(u32, usize)> {
        let mut cluster = dir_cluster;
        
        while cluster >= 2 && cluster < 0x0FFFFFF8 {
            let sector = self.cluster_to_sector(cluster);
            let mut sector_buf = [0u8; 512];
            
            for i in 0..self.sectors_per_cluster as u32 {
                ata.read_sector(sector + i, &mut sector_buf);
                
                let mut offset = 0;
                while offset < 512 {
                    let entry = unsafe { &*(sector_buf.as_ptr().add(offset) as *const FatDirEntry) };
                    if entry.is_empty() || entry.is_deleted() {
                        return Some((sector + i, offset));
                    }
                    offset += 32;
                }
            }
            
            let next = self.next_cluster(ata, cluster);
            if next == 0 { break; }
            cluster = next;
        }
        
        None
    }
    

    fn write_entry(&self, ata: &AtaDrive, sector: u32, offset: usize, entry: &FatDirEntry) -> bool {
        let mut sector_buf = [0u8; 512];
        if !ata.read_sector(sector, &mut sector_buf) { return false; }
        
        let dst = unsafe { &mut *(sector_buf.as_ptr().add(offset) as *mut FatDirEntry) };
        *dst = *entry;
        
        ata.write_sector(sector, &sector_buf)
    }
    

    fn clear_cluster(&self, ata: &AtaDrive, cluster: u32) -> bool {
        let sector = self.cluster_to_sector(cluster);
        let zero_buf = [0u8; 512];
        
        for i in 0..self.sectors_per_cluster as u32 {
            if !ata.write_sector(sector + i, &zero_buf) {
                return false;
            }
        }
        true
    }
    

    pub fn create_file(&self, ata: &AtaDrive, name: &str) -> bool {
        let dir_cluster = self.current_dir_cluster;
        

        let (entry_sector, entry_offset) = match self.find_free_entry(ata, dir_cluster) {
            Some(pos) => pos,
            None => return false,
        };
        

        let data_cluster = match self.find_free_cluster(ata) {
            Some(cluster) => cluster,
            None => return false,
        };
        

        if !self.alloc_cluster(ata, data_cluster) { return false; }
        

        self.clear_cluster(ata, data_cluster);
        

        let mut entry = FatDirEntry {
            name: [b' '; 8],
            ext: [b' '; 3],
            attributes: 0x20, // Archive
            _reserved: 0,
            _create_time_tenth: 0,
            _create_time: 0,
            _create_date: 0,
            _access_date: 0,
            first_cluster_high: 0,
            _modify_time: 0,
            _modify_date: 0,
            first_cluster_low: 0,
            size: 0,
        };
        entry.set_name(name);
        entry.set_first_cluster(data_cluster);
        
        self.write_entry(ata, entry_sector, entry_offset, &entry)
    }
    

    pub fn create_dir(&self, ata: &AtaDrive, name: &str) -> bool {
        let parent_cluster = self.current_dir_cluster;
        

        let (entry_sector, entry_offset) = match self.find_free_entry(ata, parent_cluster) {
            Some(pos) => pos,
            None => return false,
        };
        

        let dir_cluster = match self.find_free_cluster(ata) {
            Some(cluster) => cluster,
            None => return false,
        };
        

        if !self.alloc_cluster(ata, dir_cluster) { return false; }
        

        self.clear_cluster(ata, dir_cluster);
        

        let dot_sector = self.cluster_to_sector(dir_cluster);
        let mut sector_buf = [0u8; 512];
        

        let mut dot_entry = FatDirEntry {
            name: [b' '; 8], ext: [b' '; 3], attributes: 0x10,
            _reserved: 0, _create_time_tenth: 0, _create_time: 0, _create_date: 0,
            _access_date: 0, first_cluster_high: 0, _modify_time: 0, _modify_date: 0,
            first_cluster_low: 0, size: 0,
        };
        dot_entry.name[0] = b'.';
        dot_entry.set_first_cluster(dir_cluster);
        

        let mut dotdot_entry = dot_entry;
        dotdot_entry.name[0] = b'.';
        dotdot_entry.name[1] = b'.';
        dotdot_entry.set_first_cluster(parent_cluster);
        
        unsafe {
            *(sector_buf.as_ptr() as *mut FatDirEntry) = dot_entry;
            *(sector_buf.as_ptr().add(32) as *mut FatDirEntry) = dotdot_entry;
        }
        
        ata.write_sector(dot_sector, &sector_buf);
        

        let mut entry = FatDirEntry {
            name: [b' '; 8], ext: [b' '; 3], attributes: 0x10,
            _reserved: 0, _create_time_tenth: 0, _create_time: 0, _create_date: 0,
            _access_date: 0, first_cluster_high: 0, _modify_time: 0, _modify_date: 0,
            first_cluster_low: 0, size: 0,
        };
        entry.set_name(name);
        entry.set_first_cluster(dir_cluster);
        
        self.write_entry(ata, entry_sector, entry_offset, &entry)
    }
    

    pub fn write_file(&self, ata: &AtaDrive, path: &str, data: &[u8]) -> bool {
        let dir_cluster = self.current_dir_cluster;
        let mut cluster = dir_cluster;
        

        while cluster >= 2 && cluster < 0x0FFFFFF8 {
            let sector = self.cluster_to_sector(cluster);
            let mut sector_buf = [0u8; 512];
            
            for i in 0..self.sectors_per_cluster as u32 {
                ata.read_sector(sector + i, &mut sector_buf);
                
                let mut offset = 0;
                while offset < 512 {
                    let entry = unsafe { &mut *(sector_buf.as_ptr().add(offset) as *mut FatDirEntry) };
                    if entry.is_empty() { return false; }
                    offset += 32;
                    
                    if !entry.is_deleted() && !entry.is_long_name() && entry.name_matches(path) {

                        return self.write_file_data(ata, entry, data);
                    }
                }
            }
            
            cluster = self.next_cluster(ata, cluster);
        }
        

        self.create_file(ata, path) && self.write_file(ata, path, data)
    }
    
    fn write_file_data(&self, ata: &AtaDrive, entry: &FatDirEntry, data: &[u8]) -> bool {
        let mut cluster = entry.first_cluster();
        let mut bytes_written = 0;
        
        while cluster >= 2 && cluster < 0x0FFFFFF8 && bytes_written < data.len() {
            let sector = self.cluster_to_sector(cluster);
            let mut sector_buf = [0u8; 512];
            
            for i in 0..self.sectors_per_cluster as u32 {
                if bytes_written >= data.len() { break; }
                
                let to_write = if data.len() - bytes_written < 512 { data.len() - bytes_written } else { 512 };
                sector_buf[..to_write].copy_from_slice(&data[bytes_written..bytes_written + to_write]);
                for j in to_write..512 { sector_buf[j] = 0; }
                
                ata.write_sector(sector + i, &sector_buf);
                bytes_written += to_write;
            }
            
            let next = self.next_cluster(ata, cluster);
            if next == 0 && bytes_written < data.len() {

                return false; 
            }
            cluster = next;
        }
        
        true
    }
    

    pub fn read_file(&self, ata: &AtaDrive, path: &str, buf: &mut [u8]) -> Option<usize> {
        let dir_cluster = self.current_dir_cluster;
        let mut cluster = dir_cluster;
        
        while cluster >= 2 && cluster < 0x0FFFFFF8 {
            let sector = self.cluster_to_sector(cluster);
            let mut sector_buf = [0u8; 512];
            
            for i in 0..self.sectors_per_cluster as u32 {
                ata.read_sector(sector + i, &mut sector_buf);
                
                let mut offset = 0;
                while offset < 512 {
                    let entry = unsafe { &*(sector_buf.as_ptr().add(offset) as *const FatDirEntry) };
                    if entry.is_empty() { return None; }
                    offset += 32;
                    
                    if !entry.is_deleted() && !entry.is_long_name() && entry.name_matches(path) {
                        return self.read_file_data(ata, entry.first_cluster(), entry.size as usize, buf);
                    }
                }
            }
            
            cluster = self.next_cluster(ata, cluster);
        }
        None
    }
    
    fn read_file_data(&self, ata: &AtaDrive, start_cluster: u32, size: usize, buf: &mut [u8]) -> Option<usize> {
        let mut cluster = start_cluster;
        let mut bytes_read = 0;
        
        while cluster >= 2 && cluster < 0x0FFFFFF8 && bytes_read < size {
            let sector = self.cluster_to_sector(cluster);
            let mut sector_buf = [0u8; 512];
            
            for i in 0..self.sectors_per_cluster as u32 {
                if bytes_read >= size { break; }
                ata.read_sector(sector + i, &mut sector_buf);
                
                let to_read = if size - bytes_read < 512 { size - bytes_read } else { 512 };
                if bytes_read + to_read <= buf.len() {
                    buf[bytes_read..bytes_read + to_read].copy_from_slice(&sector_buf[..to_read]);
                }
                bytes_read += to_read;
            }
            
            cluster = self.next_cluster(ata, cluster);
        }
        
        Some(bytes_read)
    }
    

    pub fn delete_file(&self, ata: &AtaDrive, path: &str) -> bool {
        let dir_cluster = self.current_dir_cluster;
        let mut cluster = dir_cluster;
        
        while cluster >= 2 && cluster < 0x0FFFFFF8 {
            let sector = self.cluster_to_sector(cluster);
            let mut sector_buf = [0u8; 512];
            
            for i in 0..self.sectors_per_cluster as u32 {
                ata.read_sector(sector + i, &mut sector_buf);
                
                let mut offset = 0;
                while offset < 512 {
                    let entry = unsafe { &mut *(sector_buf.as_ptr().add(offset) as *mut FatDirEntry) };
                    if entry.is_empty() { return false; }
                    offset += 32;
                    
                    if !entry.is_deleted() && !entry.is_long_name() && entry.name_matches(path) {
                        
                        entry.set_deleted();
                        return ata.write_sector(sector + i, &sector_buf);
                    }
                }
            }
            
            cluster = self.next_cluster(ata, cluster);
        }
        false
    }
    

    pub fn change_dir(&mut self, ata: &AtaDrive, path: &str) -> bool {
        if path == "/" {
            self.current_dir_cluster = self.boot.root_cluster;
            return true;
        }
        
        let dir_cluster = self.current_dir_cluster;
        let mut cluster = dir_cluster;
        
        while cluster >= 2 && cluster < 0x0FFFFFF8 {
            let sector = self.cluster_to_sector(cluster);
            let mut sector_buf = [0u8; 512];
            
            for i in 0..self.sectors_per_cluster as u32 {
                ata.read_sector(sector + i, &mut sector_buf);
                
                let mut offset = 0;
                while offset < 512 {
                    let entry = unsafe { &*(sector_buf.as_ptr().add(offset) as *const FatDirEntry) };
                    if entry.is_empty() { return false; }
                    offset += 32;
                    
                    if !entry.is_deleted() && !entry.is_long_name() && entry.is_directory() && entry.name_matches(path) {
                        self.current_dir_cluster = entry.first_cluster();
                        return true;
                    }
                }
            }
            
            cluster = self.next_cluster(ata, cluster);
        }
        false
    }
    

    pub fn dir(&self, ata: &AtaDrive, console: &mut dyn crate::console::Console) -> usize {
        let mut cluster = self.current_dir_cluster;
        let mut count = 0;
        
        while cluster >= 2 && cluster < 0x0FFFFFF8 {
            let sector = self.cluster_to_sector(cluster);
            let mut sector_buf = [0u8; 512];
            
            for i in 0..self.sectors_per_cluster as u32 {
                ata.read_sector(sector + i, &mut sector_buf);
                
                let mut offset = 0;
                while offset < 512 {
                    let entry = unsafe { &*(sector_buf.as_ptr().add(offset) as *const FatDirEntry) };
                    if entry.is_empty() { break; }
                    offset += 32;
                    
                    if !entry.is_deleted() && !entry.is_long_name() {
                        let mut name_buf = [0u8; 13];
                        let len = entry.name_to_buf(&mut name_buf);
                        if let Ok(name) = core::str::from_utf8(&name_buf[..len]) {
                            if name != "." && name != ".." {
                                console.write_string(if entry.is_directory() { "[DIR]  " } else { "[FILE] " });
                                console.write_string(name);
                                console.write_string("\n");
                                count += 1;
                            }
                        }
                    }
                }
            }
            
            cluster = self.next_cluster(ata, cluster);
        }
        count
    }
}
