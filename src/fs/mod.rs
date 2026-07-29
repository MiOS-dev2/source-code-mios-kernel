pub mod fat32;
use fat32::Fat32FS;
use crate::ata::AtaDrive;
use crate::console::Console;

pub struct FSManager { pub fat32: Option<Fat32FS> }
impl FSManager {
    pub const fn new() -> Self { Self { fat32: None } }
    pub fn mount_fat32(&mut self, ata: &AtaDrive) -> bool {
        if let Some(fs) = Fat32FS::new(ata) { self.fat32 = Some(fs); true } else { false }
    }
    pub fn read_file(&self, ata: &AtaDrive, path: &str, buf: &mut [u8]) -> Option<usize> {
        self.fat32.as_ref()?.read_file(ata, path, buf)
    }
    pub fn write_file(&self, ata: &AtaDrive, path: &str, data: &[u8]) -> bool {
        self.fat32.as_ref().map(|fs| fs.write_file(ata, path, data)).unwrap_or(false)
    }
    pub fn create_file(&self, ata: &AtaDrive, name: &str) -> bool {
        self.fat32.as_ref().map(|fs| fs.create_file(ata, name)).unwrap_or(false)
    }
    pub fn create_dir(&self, ata: &AtaDrive, name: &str) -> bool {
        self.fat32.as_ref().map(|fs| fs.create_dir(ata, name)).unwrap_or(false)
    }
    pub fn delete_file(&self, ata: &AtaDrive, path: &str) -> bool {
        self.fat32.as_ref().map(|fs| fs.delete_file(ata, path)).unwrap_or(false)
    }
    pub fn change_dir(&mut self, ata: &AtaDrive, path: &str) -> bool {
        self.fat32.as_mut().map(|fs| fs.change_dir(ata, path)).unwrap_or(false)
    }
    pub fn dir(&self, ata: &AtaDrive, console: &mut dyn Console) -> usize {
        self.fat32.as_ref().map(|fs| fs.dir(ata, console)).unwrap_or(0)
    }
}
