use crate::vga::VGA;

pub struct Shell { buffer: [u8; 256], len: usize }
impl Shell {
    pub const fn new() -> Self { Self { buffer: [0; 256], len: 0 } }
    pub fn read_command(&mut self, vga: &mut VGA) {
        self.len = 0;
        loop {
            let key = crate::keyboard::get_key();
            match key {
                crate::keyboard::Key::Char(c) if self.len < 255 => {
                    self.buffer[self.len] = c as u8; self.len += 1;
                    vga.put_char(c);
                }
                crate::keyboard::Key::Enter => {
                    self.buffer[self.len] = 0; vga.write_string("\n");
                    break;
                }
                crate::keyboard::Key::Backspace if self.len > 0 => {
                    self.len -= 1; vga.backspace();
                }
                _ => {}
            }
        }
    }
    pub fn get_buffer(&self) -> &str { core::str::from_utf8(&self.buffer[..self.len]).unwrap_or("") }
}
