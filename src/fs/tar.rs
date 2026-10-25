
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct TarHeader {
    pub name: [u8; 100],
    _mode: [u8; 8],
    _uid: [u8; 8],
    _gid: [u8; 8],
    pub size: [u8; 12],
    _mtime: [u8; 12],
    _checksum: [u8; 8],
    pub typeflag: u8,
    _linkname: [u8; 100],
    _magic: [u8; 6],
    _version: [u8; 2],
    _uname: [u8; 32],
    _gname: [u8; 32],
    _devmajor: [u8; 8],
    _devminor: [u8; 8],
    _prefix: [u8; 155],
}

impl TarHeader {
    pub fn file_size(&self) -> usize {
        let mut size = 0usize;
        for &byte in &self.size {
            if byte == 0 || byte == b' ' { continue; }
            if byte >= b'0' && byte <= b'7' {
                size = size * 8 + (byte - b'0') as usize;
            }
        }
        size
    }
    
    pub fn is_empty(&self) -> bool {
        self.name[0] == 0
    }
    
    pub fn name_str(&self) -> &str {
        let mut len = 0;
        while len < 100 && self.name[len] != 0 {
            len += 1;
        }
        core::str::from_utf8(&self.name[..len]).unwrap_or("")
    }
    
    pub fn is_directory(&self) -> bool {
        self.typeflag == b'5'
    }
}

pub struct TarFS {
    pub data: &'static [u8], 
}

impl TarFS {
    pub fn new(addr: usize, size: usize) -> Self {
        Self {
            data: unsafe { core::slice::from_raw_parts(addr as *const u8, size) },
        }
    }
    
    fn find_header(&self, path: &str) -> Option<(&TarHeader, usize)> {
        let mut offset = 0;
        let clean_path = path.trim_start_matches('/');
        
        while offset + 512 <= self.data.len() {
            let header = unsafe { &*(self.data.as_ptr().add(offset) as *const TarHeader) };
            
            if header.is_empty() {
                break;
            }
            
            let name = header.name_str();
            let clean_name = name.trim_start_matches('/').trim_end_matches('/');
            
            if clean_name == clean_path {
                return Some((header, offset));
            }
            
            let size = header.file_size();
            offset += 512;
            if size > 0 {
                offset += (size + 511) & !511;
            }
        }
        
        None
    }
    
    pub fn read_file(&self, path: &str, buf: &mut [u8]) -> Option<usize> {
        let (header, offset) = self.find_header(path)?;
        
        if header.is_directory() {
            return None;
        }
        
        let size = header.file_size();
        if size > buf.len() {
            return None;
        }
        
        let data_offset = offset + 512;
        buf[..size].copy_from_slice(&self.data[data_offset..data_offset + size]);
        Some(size)
    }
    
    pub fn list_dir(&self, _path: &str, buf: &mut [u8]) -> usize {
        let mut offset = 0;
        let mut buf_offset = 0;
        let mut count = 0;
        
        while offset + 512 <= self.data.len() && buf_offset + 128 < buf.len() {
            let header = unsafe { &*(self.data.as_ptr().add(offset) as *const TarHeader) };
            
            if header.is_empty() {
                break;
            }
            
            let name = header.name_str();
            
            if !name.is_empty() && !name.contains('/') {
                let name_bytes = name.as_bytes();
                let name_len = name_bytes.len();
                
                buf[buf_offset] = name_len as u8;
                buf[buf_offset + 1] = if header.is_directory() { 1 } else { 0 };
                let size = header.file_size() as u32;
                buf[buf_offset + 2..buf_offset + 6].copy_from_slice(&size.to_le_bytes());
                buf[buf_offset + 6..buf_offset + 6 + name_len].copy_from_slice(name_bytes);
                buf_offset += 6 + name_len;
                count += 1;
            }
            
            let size = header.file_size();
            offset += 512 + ((size + 511) & !511);
        }
        
        buf[buf_offset] = 0;
        count
    }
    
    pub fn exists(&self, path: &str) -> bool {
        self.find_header(path).is_some()
    }
}
