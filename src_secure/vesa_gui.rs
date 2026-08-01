use crate::vesa::{VesaDisplay, Color};
use crate::ata::AtaDrive;
use crate::fs::FSManager;
use crate::keyboard;          
use crate::keyboard::Key;     

pub fn run(_ata: &AtaDrive, _fs: &mut FSManager) -> bool {
    let info = match unsafe { crate::VESA_INFO.as_ref() } {
        Some(i) => i.clone(),
        None => return false,
    };
    let mut disp = unsafe { VesaDisplay::from_multiboot(&info) };
    
    
    disp.clear(Color::GREEN);
    let w = 200;
    let h = 100;
    let x = (info.width - w) / 2;
    let y = (info.height - h) / 2;
    disp.fill_rect(x, y, w, h, Color::RED.0);
    
    loop {
        if keyboard::get_key() == Key::Escape {
            return true;   // выход из GUI, возврат в шелл
        }
    }
}
