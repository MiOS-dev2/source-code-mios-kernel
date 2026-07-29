


#[derive(Debug, PartialEq, Eq)]
pub enum BootMode {
    Normal,
    Debug,
    Console,
}

pub struct BootConfig {
    pub boot_mode: BootMode,
    pub modules: ModuleConfig,
    pub vesa: VESAConfig,
    pub debug: DebugConfig,
}

pub struct ModuleConfig {
    pub vga: bool,
    pub keyboard: bool,
    pub shell: bool,
    pub commands: bool,
    pub gui: bool,
    pub utils: bool,
    pub fs: bool,
    pub ata: bool,
    pub fat32: bool,
    pub task: bool,
    pub multiboot: bool,
    pub graphics: bool,
    pub wm: bool,
}

impl ModuleConfig {
    pub fn all() -> Self {
        Self {
            vga: true, keyboard: true, shell: true, commands: true,
            gui: true, utils: true, fs: true, ata: true, fat32: true,
            task: false, multiboot: true, graphics: true, wm: true,
        }
    }
    
    pub fn console() -> Self {
        Self {
            vga: true, keyboard: true, shell: true, commands: true,
            gui: false, utils: true, fs: true, ata: true, fat32: true,
            task: false, multiboot: false, graphics: false, wm: false,
        }
    }
}

pub struct VESAConfig {
    pub enabled: bool,
    pub width: u32,
    pub height: u32,
    pub bpp: u32,
}

pub struct DebugConfig {
    pub verbose: bool,
    pub show_hex: bool,
    pub show_tags: bool,
    pub show_memory: bool,
    pub show_fb_info: bool,
}

impl BootConfig {
    pub fn parse(config_data: &[u8], boot_mode: BootMode) -> Self {
        let mut modules = match boot_mode {
            BootMode::Debug => ModuleConfig::all(),
            BootMode::Console => ModuleConfig::console(),
            BootMode::Normal => ModuleConfig::all(),
        };
        
        let mut vesa = VESAConfig {
            enabled: boot_mode != BootMode::Console,
            width: 1024,
            height: 768,
            bpp: 32,
        };
        
        let mut debug = DebugConfig {
            verbose: boot_mode == BootMode::Debug,
            show_hex: boot_mode == BootMode::Debug,
            show_tags: boot_mode == BootMode::Debug,
            show_memory: false,
            show_fb_info: boot_mode == BootMode::Debug,
        };
        
        if let Ok(text) = core::str::from_utf8(config_data) {
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') { continue; }
                
                if let Some(eq_pos) = line.find('=') {
                    let key = line[..eq_pos].trim();
                    let value = line[eq_pos+1..].trim();
                    let bool_val = value == "true";
                    
                    match key {
                        "vesa" => vesa.enabled = bool_val,
                        "verbose" => debug.verbose = bool_val,
                        "shell" => modules.shell = bool_val,
                        "graphics" => modules.graphics = bool_val,
                        "fat" | "fat32" | "fs" => {
                            modules.fs = bool_val;
                            modules.fat32 = bool_val;
                            modules.ata = bool_val;
                        }
                        _ => {}
                    }
                }
            }
        }
        
        Self { boot_mode, modules, vesa, debug }
    }
}

pub unsafe fn get_boot_mode(info_addr: u32) -> BootMode {
    let total_size = *(info_addr as *const u32);
    let mut offset: usize = 8;
    
    while offset < total_size as usize {
        let tag_ptr = (info_addr as usize + offset) as *const u32;
        let tag_type = *tag_ptr;
        let tag_size = *(tag_ptr.add(1)) as usize;
        
        if tag_type == 0 { break; }
        
        if tag_type == 3 {
            let name_ptr = *(tag_ptr.add(4)) as *const u8;
            let mut len = 0;
            while len < 64 && *name_ptr.add(len) != 0 { len += 1; }
            let name = core::str::from_utf8_unchecked(
                core::slice::from_raw_parts(name_ptr, len)
            );
            
            if name.contains("debug.cfg") || name.contains("debug") {
                return BootMode::Debug;
            }
            if name.contains("console.cfg") || name.contains("console") {
                return BootMode::Console;
            }
        }
        
        offset += tag_size;
        offset = (offset + 7) & !7;
    }
    BootMode::Normal
}

pub unsafe fn get_config_data(info_addr: u32) -> &'static [u8] {
    let total_size = *(info_addr as *const u32);
    let mut offset: usize = 8;
    
    while offset < total_size as usize {
        let tag_ptr = (info_addr as usize + offset) as *const u32;
        let tag_type = *tag_ptr;
        let tag_size = *(tag_ptr.add(1)) as usize;
        
        if tag_type == 0 { break; }
        
        if tag_type == 3 {
            let mod_start = *(tag_ptr.add(2)) as usize;
            let mod_end = *(tag_ptr.add(3)) as usize;
            let name_ptr = *(tag_ptr.add(4)) as *const u8;
            let mut len = 0;
            while len < 64 && *name_ptr.add(len) != 0 { len += 1; }
            let name = core::str::from_utf8_unchecked(
                core::slice::from_raw_parts(name_ptr, len)
            );
            
            if name.contains(".cfg") {
                return core::slice::from_raw_parts(
                    mod_start as *const u8,
                    mod_end - mod_start
                );
            }
        }
        
        offset += tag_size;
        offset = (offset + 7) & !7;
    }
    &[]
}
