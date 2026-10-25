use crate::vga::VGA;
pub struct Gui { }
impl Gui { pub const fn new() -> Self { Self { } } pub fn run(&mut self, _vga: &mut VGA) {} }
