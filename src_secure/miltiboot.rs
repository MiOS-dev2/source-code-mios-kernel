
#[repr(C, packed)]
pub struct FramebufferInfo {
    pub addr: u64,
    pub pitch: u32,
    pub width: u32,
    pub height: u32,
    pub bpp: u8,
    pub fb_type: u8,
    _reserved: u16,
}

pub struct MultibootInfo {
    pub framebuffer: Option<FramebufferInfo>,
    pub initrd_start: u32,
    pub initrd_end: u32,
    pub memory_total: u64,
}

impl MultibootInfo {
    pub unsafe fn parse(info_addr: u32) -> Self {
        let mut info = Self {
            framebuffer: None,
            initrd_start: 0,
            initrd_end: 0,
            memory_total: 0,
        };
        
        let mut ptr = info_addr as *const u32;
        let total_size = *ptr;
        ptr = ptr.add(2);
        let end = (info_addr + total_size) as *const u32;
        
        while ptr < end {
            let typ = *ptr;
            let size = *(ptr.add(1)) as usize;
            
            match typ {
                8 => { // Framebuffer
                    let fb = &*(ptr.add(2) as *const FramebufferInfo);
                    info.framebuffer = Some(*fb);
                }
                3 => { // Module
                    let start = *(ptr.add(2));
                    let end = *(ptr.add(3));
                    let name_ptr = *(ptr.add(4)) as *const u8;
                    
                    // Проверяем имя модуля
                    let mut is_initrd = false;
                    let mut i = 0;
                    let expected = b"initrd.tar";
                    while i < 10 {
                        let c = *name_ptr.add(i);
                        if c != expected[i] {
                            break;
                        }
                        i += 1;
                        if c == 0 { break; }
                    }
                    if i == 10 || *name_ptr.add(i) == 0 {
                        is_initrd = true;
                    }
                    
                    if is_initrd {
                        info.initrd_start = start;
                        info.initrd_end = end;
                    }
                }
                6 => { // Memory map
                    let entry_size = *(ptr.add(2)) as usize;
                    let entries_ptr = ptr.add(4) as *const u8;
                    let entries_count = (size - 16) / entry_size;
                    
                    for i in 0..entries_count {
                        let entry = entries_ptr.add(i * entry_size) as *const u64;
                        let base = *entry;
                        let length = *(entry.add(1));
                        let typ = *(entry.add(2) as *const u32);
                        
                        if typ == 1 { // Usable RAM
                            info.memory_total += length;
                        }
                    }
                }
                _ => {}
            }
            
            ptr = ptr.add((size + 7) / 8 * 2);
        }
        
        info
    }
}
