
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
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
}

impl MultibootInfo {
    pub unsafe fn parse(info_addr: u32) -> Self {
        let mut info = Self { framebuffer: None };
        

        let total_size = *(info_addr as *const u32);
        

        let mut offset: usize = 8;
        
        while offset < total_size as usize {
            let tag_ptr = (info_addr as usize + offset) as *const u32;
            let tag_type = *tag_ptr;
            let tag_size = *(tag_ptr.add(1)) as usize;
            

            if tag_type == 0 || tag_size < 8 {
                break;
            }
            

            if tag_type == 8 {
                let fb_ptr = tag_ptr.add(2) as *const FramebufferInfo;
                let fb = *fb_ptr;
                

                if fb.addr != 0 && fb.width > 0 && fb.height > 0 {
                    info.framebuffer = Some(fb);
                }
            }
            

            offset += (tag_size + 7) & !7;
        }
        
        info
    }
}