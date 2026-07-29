
use crate::graphics::{Graphics, Color};

pub struct BmpImage {
    pub width: usize,
    pub height: usize,
    pub data: &'static [u8],
}

impl BmpImage {

    pub fn from_bytes(data: &'static [u8]) -> Option<Self> {
       
        if data.len() < 54 || data[0] != b'B' || data[1] != b'M' {
            return None;
        }
        

        let pixel_offset = u32::from_le_bytes([data[10], data[11], data[12], data[13]]) as usize;
        let width = u32::from_le_bytes([data[18], data[19], data[20], data[21]]) as usize;
        let height = u32::from_le_bytes([data[22], data[23], data[24], data[25]]) as usize;
        let bits_per_pixel = u16::from_le_bytes([data[28], data[29]]);
        

        if bits_per_pixel != 24 {
            return None;
        }
        
        Some(BmpImage {
            width,
            height,
            data: &data[pixel_offset..],
        })
    }
    

    pub fn draw_as_wallpaper(&self, gfx: &mut Graphics) {

        let row_size = ((self.width * 3 + 3) / 4) * 4; 
        
        for y in 0..gfx.height.min(self.height) {
            let bmp_y = (self.height - 1).saturating_sub(y); 
            let bmp_row_start = bmp_y * row_size;
            
            for x in 0..gfx.width.min(self.width) {
                let pixel_offset = bmp_row_start + x * 3;
                if pixel_offset + 2 < self.data.len() {
                    let b = self.data[pixel_offset] as u32;
                    let g = self.data[pixel_offset + 1] as u32;
                    let r = self.data[pixel_offset + 2] as u32;
                    let color = (r << 16) | (g << 8) | b;
                    gfx.put_pixel(x, y, color);
                }
            }
        }
    }
}