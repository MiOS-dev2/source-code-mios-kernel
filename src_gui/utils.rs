pub struct Uptime { ticks: u64 }
impl Uptime { pub const fn new() -> Self { Self { ticks: 0 } } pub fn tick(&mut self) { self.ticks += 1; } pub fn get(&self) -> u64 { self.ticks } }
pub fn random() -> u64 { 42 }
