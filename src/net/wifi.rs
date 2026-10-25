use alloc::string::String;
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::format;
use crate::pci::PciDev;

#[derive(Clone)]
pub struct WifiNetwork {
    pub ssid: String,
    pub signal: u8,
    pub secure: bool,
}

pub trait WifiDriver {
    fn scan(&mut self) -> Result<Vec<WifiNetwork>, String>;
    fn connect(&mut self, ssid: &str, password: &str) -> Result<(), String>;
    fn disconnect(&mut self);
    fn connected_ssid(&self) -> Option<String>;
}

static mut DRIVER: Option<Box<dyn WifiDriver>> = None;

pub fn init() {
}

pub fn driver() -> Option<&'static mut Box<dyn WifiDriver>> {
    unsafe { (*(&raw mut DRIVER)).as_mut() }
}

pub fn has_driver() -> bool { driver().is_some() }

pub fn adapter_name() -> Option<String> {
    for a in super::adapters().iter() {
        if a.kind == super::Kind::WiFi { return Some(a.name.clone()); }
    }
    None
}

pub fn scan() -> Result<Vec<WifiNetwork>, String> {
    match driver() {
        Some(d) => d.scan(),
        None => Err(no_driver_msg()),
    }
}

pub fn connect(ssid: &str, password: &str) -> Result<(), String> {
    if ssid.is_empty() { return Err(String::from("Выберите сеть")); }
    match driver() {
        Some(d) => d.connect(ssid, password),
        None => Err(no_driver_msg()),
    }
}

pub fn no_driver_msg() -> String {
    match adapter_name() {
        Some(n) => format!("Для адаптера \"{}\" нет драйвера Wi-Fi", n),
        None => String::from("Wi-Fi адаптер не найден"),
    }
}

pub fn is_wifi(d: &PciDev) -> bool {
    name_for(d).is_some()
}

pub fn name_for(d: &PciDev) -> Option<String> {
    let n = match (d.vendor, d.device) {
        (0x8086, 0x2723) => "Intel Wi-Fi 6 AX200",
        (0x8086, 0x2725) => "Intel Wi-Fi 6E AX210",
        (0x8086, 0x51F0) | (0x8086, 0x51F1) | (0x8086, 0x7AF0) | (0x8086, 0x54F0) | (0x8086, 0xA0F0) | (0x8086, 0x43F0) | (0x8086, 0x02F0) | (0x8086, 0x06F0) | (0x8086, 0x34F0) => "Intel Wi-Fi 6 AX201/AX211",
        (0x8086, 0x2526) | (0x8086, 0x9DF0) | (0x8086, 0x31DC) | (0x8086, 0x30DC) | (0x8086, 0x271B) | (0x8086, 0x271C) => "Intel Wireless-AC 9560/9260",
        (0x8086, 0x24FD) | (0x8086, 0x24F3) | (0x8086, 0x24F4) | (0x8086, 0x3165) | (0x8086, 0x3166) | (0x8086, 0x095A) | (0x8086, 0x095B) | (0x8086, 0x08B1) | (0x8086, 0x08B2) => "Intel Wireless-AC",
        (0x10EC, 0xB822) | (0x10EC, 0xC822) | (0x10EC, 0xC821) | (0x10EC, 0x8821) | (0x10EC, 0x8822) | (0x10EC, 0xB821) => "Realtek RTL8822/8821 Wi-Fi",
        (0x10EC, 0x8852) | (0x10EC, 0xB852) | (0x10EC, 0xC852) | (0x10EC, 0x885C) => "Realtek RTL8852 Wi-Fi 6",
        (0x10EC, 0x8179) | (0x10EC, 0x8176) | (0x10EC, 0x818B) | (0x10EC, 0xB723) | (0x10EC, 0x0179) => "Realtek RTL8188/8723 Wi-Fi",
        (0x14C3, 0x7961) | (0x14C3, 0x0608) | (0x14C3, 0x0616) | (0x14C3, 0x7922) => "MediaTek MT7921/MT7922 Wi-Fi 6",
        (0x168C, _) => "Qualcomm Atheros Wi-Fi",
        (0x17CB, 0x1101) | (0x17CB, 0x1103) => "Qualcomm QCA6390/WCN685x Wi-Fi",
        (0x14E4, id) if (0x4300..0x44FF).contains(&id) => "Broadcom Wi-Fi",
        _ => {
            if d.class == 0x02 && d.subclass == 0x80 {
                return Some(format!("Беспроводной адаптер {:04x}:{:04x}", d.vendor, d.device));
            }
            return None;
        }
    };
    Some(String::from(n))
}
