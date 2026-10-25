pub mod e1000;
pub mod rtl8139;
pub mod rtl8169;
pub mod wifi;
pub mod http;

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use alloc::format;
use smoltcp::iface::{Config, Interface, SocketHandle, SocketSet};
use smoltcp::phy::{Device, DeviceCapabilities, Medium, RxToken, TxToken};
use smoltcp::socket::{dhcpv4, dns, tcp};
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, HardwareAddress, IpAddress, IpCidr, Ipv4Address, Ipv4Cidr, DnsQueryType};

use crate::pci::{self, PciDev};
use crate::timer;

pub enum Nic {
    E1000(e1000::E1000),
    R8139(rtl8139::Rtl8139),
    R8169(rtl8169::Rtl8169),
}

impl Nic {
    fn recv(&mut self, b: &mut [u8]) -> Option<usize> {
        match self { Nic::E1000(n) => n.recv(b), Nic::R8139(n) => n.recv(b), Nic::R8169(n) => n.recv(b) }
    }
    fn send(&mut self, b: &[u8]) -> bool {
        match self { Nic::E1000(n) => n.send(b), Nic::R8139(n) => n.send(b), Nic::R8169(n) => n.send(b) }
    }
    fn can_send(&self) -> bool {
        match self { Nic::E1000(n) => n.can_send(), Nic::R8139(n) => n.can_send(), Nic::R8169(n) => n.can_send() }
    }
    pub fn link_up(&self) -> bool {
        match self { Nic::E1000(n) => n.link_up(), Nic::R8139(n) => n.link_up(), Nic::R8169(n) => n.link_up() }
    }
    pub fn speed(&self) -> u32 {
        match self { Nic::E1000(n) => n.speed_mbps(), Nic::R8139(n) => n.speed_mbps(), Nic::R8169(n) => n.speed_mbps() }
    }
    pub fn mac(&self) -> [u8; 6] {
        match self { Nic::E1000(n) => n.mac, Nic::R8139(n) => n.mac, Nic::R8169(n) => n.mac }
    }
}

struct Dev<'a> { nic: &'a mut Nic, rx: Vec<u8> }
struct Rx(Vec<u8>);
struct Tx<'a>(&'a mut Nic);

impl RxToken for Rx {
    fn consume<R, F: FnOnce(&[u8]) -> R>(self, f: F) -> R { f(&self.0) }
}
impl<'a> TxToken for Tx<'a> {
    fn consume<R, F: FnOnce(&mut [u8]) -> R>(self, len: usize, f: F) -> R {
        let mut buf = vec![0u8; len];
        let r = f(&mut buf);
        let t = timer::millis();
        while !self.0.can_send() && timer::millis() - t < 20 {}
        self.0.send(&buf);
        unsafe { STATS.tx += 1; }
        r
    }
}
impl<'d> Device for Dev<'d> {
    type RxToken<'a> = Rx where Self: 'a;
    type TxToken<'a> = Tx<'a> where Self: 'a;
    fn receive(&mut self, _t: Instant) -> Option<(Rx, Tx<'_>)> {
        loop {
            match self.nic.recv(&mut self.rx) {
                Some(0) => continue,
                Some(n) => {
                    unsafe { STATS.rx += 1; }
                    let p = self.rx[..n].to_vec();
                    return Some((Rx(p), Tx(self.nic)));
                }
                None => return None,
            }
        }
    }
    fn transmit(&mut self, _t: Instant) -> Option<Tx<'_>> { Some(Tx(self.nic)) }
    fn capabilities(&self) -> DeviceCapabilities {
        let mut c = DeviceCapabilities::default();
        c.medium = Medium::Ethernet;
        c.max_transmission_unit = 1514;
        c.max_burst_size = Some(16);
        c
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Kind { Ethernet, WiFi }

pub struct Adapter {
    pub kind: Kind,
    pub name: String,
    pub pci: PciDev,
    pub driver: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Status { NoAdapter, NoCable, Disabled, Dhcp, Connected }

pub struct Stats { pub rx: u64, pub tx: u64 }
pub static mut STATS: Stats = Stats { rx: 0, tx: 0 };

pub struct Net {
    nic: Nic,
    iface: Interface,
    sockets: SocketSet<'static>,
    dhcp: SocketHandle,
    dns: SocketHandle,
    rxbuf: Vec<u8>,
    pub ip: Option<Ipv4Cidr>,
    pub gateway: Option<Ipv4Address>,
    pub dns_servers: Vec<Ipv4Address>,
    pub enabled: bool,
    pub adapter_name: String,
    next_port: u16,
    dhcp_started: u64,
}

pub static mut NET: Option<Net> = None;
pub static mut ADAPTERS: Vec<Adapter> = Vec::new();
static mut INITED: bool = false;
static mut LAST_POLL: u64 = 0;
pub static mut TICK_HOOK: Option<fn()> = None;

fn now() -> Instant { Instant::from_millis(timer::millis() as i64) }

fn name_for(d: &PciDev) -> String {
    let base = match (d.vendor, d.device) {
        (0x8086, 0x100E) => "Intel PRO/1000 MT (82540EM)",
        (0x8086, 0x100F) => "Intel PRO/1000 MT (82545EM)",
        (0x8086, 0x10D3) => "Intel 82574L Gigabit",
        (0x8086, 0x153A) | (0x8086, 0x1502) => "Intel Ethernet I217/82579",
        (0x8086, _) if e1000::supported(d) => "Intel PRO/1000",
        (0x10EC, 0x8139) => "Realtek RTL8139 Fast Ethernet",
        (0x10EC, 0x8168) | (0x10EC, 0x8161) => "Realtek RTL8111/8168 Gigabit",
        (0x10EC, 0x8169) => "Realtek RTL8169 Gigabit",
        (0x10EC, 0x8136) => "Realtek RTL810x Fast Ethernet",
        (0x1AF4, 0x1000) | (0x1AF4, 0x1041) => "VirtIO Network",
        _ => "",
    };
    if !base.is_empty() { return String::from(base); }
    if let Some(n) = wifi::name_for(d) { return n; }
    let vend = match d.vendor {
        0x8086 => "Intel", 0x10EC => "Realtek", 0x14E4 => "Broadcom", 0x168C => "Qualcomm Atheros",
        0x14C3 => "MediaTek", 0x1969 => "Atheros", 0x11AB => "Marvell", 0x1AF4 => "VirtIO", _ => "",
    };
    format!("{} сетевой контроллер {:04x}:{:04x}", vend, d.vendor, d.device)
}

pub fn init() {
    unsafe {
        if INITED { return; }
        INITED = true;
    }
    timer::init();
    let mut found: Vec<PciDev> = Vec::new();
    pci::scan(|d| if d.class == 0x02 { found.push(d) });
    let mut nic: Option<(Nic, String)> = None;
    for d in found.iter() {
        let wifi = d.subclass == 0x80 || wifi::is_wifi(d);
        let drv = e1000::supported(d) || rtl8139::supported(d) || rtl8169::supported(d);
        let name = name_for(d);
        unsafe {
            (*(&raw mut ADAPTERS)).push(Adapter { kind: if wifi { Kind::WiFi } else { Kind::Ethernet }, name: name.clone(), pci: *d, driver: drv && !wifi });
        }
        if nic.is_some() || wifi { continue; }
        if e1000::supported(d) {
            if let Some(n) = e1000::E1000::new(d) { nic = Some((Nic::E1000(n), name)); }
        } else if rtl8169::supported(d) {
            if let Some(n) = rtl8169::Rtl8169::new(d) { nic = Some((Nic::R8169(n), name)); }
        } else if rtl8139::supported(d) {
            if let Some(n) = rtl8139::Rtl8139::new(d) { nic = Some((Nic::R8139(n), name)); }
        }
    }
    wifi::init();
    let (mut nic, name) = match nic { Some(x) => x, None => return };

    let mac = nic.mac();
    let mut cfg = Config::new(HardwareAddress::Ethernet(EthernetAddress(mac)));
    cfg.random_seed = timer::rdtsc() ^ 0x9E37_79B9_7F4A_7C15;
    let mut dev = Dev { nic: &mut nic, rx: vec![0u8; 2048] };
    let iface = Interface::new(cfg, &mut dev, now());
    let mut sockets = SocketSet::new(Vec::new());
    let mut d = dhcpv4::Socket::new();
    d.set_max_lease_duration(Some(smoltcp::time::Duration::from_secs(3600)));
    let dhcp = sockets.add(d);
    let dns_sock = dns::Socket::new(&[], vec![None, None, None, None]);
    let dns = sockets.add(dns_sock);
    let port = 49152 + (timer::rdtsc() % 8000) as u16;
    unsafe {
        NET = Some(Net {
            nic, iface, sockets, dhcp, dns, rxbuf: vec![0u8; 2048],
            ip: None, gateway: None, dns_servers: Vec::new(), enabled: true,
            adapter_name: name, next_port: port, dhcp_started: timer::millis(),
        });
    }
}

pub fn net() -> Option<&'static mut Net> { unsafe { (*(&raw mut NET)).as_mut() } }
pub fn adapters() -> &'static Vec<Adapter> { unsafe { &*(&raw const ADAPTERS) } }

pub fn status() -> Status {
    match net() {
        None => Status::NoAdapter,
        Some(n) => {
            if !n.enabled { Status::Disabled }
            else if !n.nic.link_up() { Status::NoCable }
            else if n.ip.is_none() { Status::Dhcp }
            else { Status::Connected }
        }
    }
}

pub fn status_text() -> &'static str {
    match status() {
        Status::NoAdapter => "Сетевой адаптер не найден",
        Status::NoCable => "Кабель не подключен",
        Status::Disabled => "Отключено",
        Status::Dhcp => "Получение IP-адреса (DHCP)...",
        Status::Connected => "Подключено к Интернету",
    }
}

pub fn link_speed() -> u32 { net().map(|n| n.nic.speed()).unwrap_or(0) }
pub fn mac_string() -> String {
    match net() {
        Some(n) => { let m = n.nic.mac(); format!("{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}", m[0], m[1], m[2], m[3], m[4], m[5]) }
        None => String::from("-"),
    }
}

pub fn set_enabled(on: bool) {
    if let Some(n) = net() {
        n.enabled = on;
        if !on {
            n.iface.update_ip_addrs(|a| a.clear());
            n.iface.routes_mut().remove_default_ipv4_route();
            n.ip = None;
            n.gateway = None;
        } else {
            let s = n.sockets.get_mut::<dhcpv4::Socket>(n.dhcp);
            s.reset();
            n.dhcp_started = timer::millis();
        }
    }
}

pub fn poll() -> bool {
    if let Some(h) = unsafe { TICK_HOOK } { h(); }
    let n = match net() { Some(n) => n, None => return false };
    unsafe { LAST_POLL = timer::millis(); }
    if !n.enabled {
        let mut b = [0u8; 2048];
        while let Some(_) = n.nic.recv(&mut b) {}
        return false;
    }
    let mut changed = false;
    {
        let mut dev = Dev { nic: &mut n.nic, rx: core::mem::take(&mut n.rxbuf) };
        n.iface.poll(now(), &mut dev, &mut n.sockets);
        n.rxbuf = dev.rx;
    }
    let ev = n.sockets.get_mut::<dhcpv4::Socket>(n.dhcp).poll();
    match ev {
        Some(dhcpv4::Event::Configured(c)) => {
            n.iface.update_ip_addrs(|a| { a.clear(); let _ = a.push(IpCidr::Ipv4(c.address)); });
            if let Some(r) = c.router {
                let _ = n.iface.routes_mut().add_default_ipv4_route(r);
            } else {
                n.iface.routes_mut().remove_default_ipv4_route();
            }
            n.ip = Some(c.address);
            n.gateway = c.router;
            n.dns_servers = c.dns_servers.iter().cloned().collect();
            let servers: Vec<IpAddress> = if n.dns_servers.is_empty() {
                vec![IpAddress::Ipv4(Ipv4Address::new(8, 8, 8, 8))]
            } else {
                n.dns_servers.iter().map(|a| IpAddress::Ipv4(*a)).collect()
            };
            n.sockets.get_mut::<dns::Socket>(n.dns).update_servers(&servers);
            changed = true;
        }
        Some(dhcpv4::Event::Deconfigured) => {
            n.iface.update_ip_addrs(|a| a.clear());
            n.iface.routes_mut().remove_default_ipv4_route();
            n.ip = None;
            n.gateway = None;
            changed = true;
        }
        None => {}
    }
    changed
}

pub fn ip_string() -> String {
    match net().and_then(|n| n.ip) { Some(c) => format!("{}", c), None => String::from("-") }
}
pub fn gw_string() -> String {
    match net().and_then(|n| n.gateway) { Some(c) => format!("{}", c), None => String::from("-") }
}
pub fn dns_string() -> String {
    match net() {
        Some(n) if !n.dns_servers.is_empty() => {
            let v: Vec<String> = n.dns_servers.iter().map(|a| format!("{}", a)).collect();
            v.join(", ")
        }
        _ => String::from("-"),
    }
}

pub fn wait_ready(timeout_ms: u64) -> Result<(), &'static str> {
    let t = timer::millis();
    loop {
        poll();
        match status() {
            Status::Connected => return Ok(()),
            Status::NoAdapter => return Err("Сетевой адаптер не найден"),
            Status::Disabled => return Err("Сеть отключена (Сеть -> Подключить)"),
            _ => {}
        }
        if timer::millis() - t > timeout_ms {
            return Err(if status() == Status::NoCable { "Кабель не подключен" } else { "Нет IP-адреса (DHCP не ответил)" });
        }
    }
}

pub fn resolve(host: &str) -> Result<Ipv4Address, String> {
    if let Some(ip) = parse_ipv4(host) { return Ok(ip); }
    if host == "localhost" { return Ok(Ipv4Address::new(127, 0, 0, 1)); }
    wait_ready(8000).map_err(String::from)?;
    let n = net().unwrap();
    let h = {
        let cx = n.iface.context();
        let s = n.sockets.get_mut::<dns::Socket>(n.dns);
        s.start_query(cx, host, DnsQueryType::A).map_err(|e| format!("DNS: {:?}", e))?
    };
    let t = timer::millis();
    loop {
        poll();
        let n = net().unwrap();
        let s = n.sockets.get_mut::<dns::Socket>(n.dns);
        match s.get_query_result(h) {
            Ok(addrs) => {
                for a in addrs.iter() {
                    #[allow(irrefutable_let_patterns)]
                    if let IpAddress::Ipv4(v4) = a { return Ok(*v4); }
                }
                return Err(format!("DNS: нет IPv4 адреса для {}", host));
            }
            Err(dns::GetQueryResultError::Pending) => {}
            Err(_) => return Err(format!("Не удалось найти сервер {}", host)),
        }
        if timer::millis() - t > 10000 {
            s.cancel_query(h);
            return Err(format!("DNS: нет ответа для {}", host));
        }
    }
}

pub fn parse_ipv4(s: &str) -> Option<Ipv4Address> {
    let mut o = [0u8; 4];
    let mut i = 0;
    for p in s.split('.') {
        if i >= 4 || p.is_empty() || p.len() > 3 { return None; }
        o[i] = p.parse().ok()?;
        i += 1;
    }
    if i == 4 { Some(Ipv4Address::new(o[0], o[1], o[2], o[3])) } else { None }
}

pub struct Tcp { h: SocketHandle }

impl Tcp {
    pub fn connect(ip: Ipv4Address, port: u16, timeout_ms: u64) -> Result<Tcp, String> {
        wait_ready(8000).map_err(String::from)?;
        let n = net().unwrap();
        let sock = tcp::Socket::new(tcp::SocketBuffer::new(vec![0u8; 256 * 1024]), tcp::SocketBuffer::new(vec![0u8; 64 * 1024]));
        let h = n.sockets.add(sock);
        n.next_port = if n.next_port >= 65000 { 49152 } else { n.next_port + 1 };
        let lp = n.next_port;
        {
            let cx = n.iface.context();
            let s = n.sockets.get_mut::<tcp::Socket>(h);
            s.set_timeout(Some(smoltcp::time::Duration::from_secs(20)));
            s.set_nagle_enabled(false);
            if s.connect(cx, (IpAddress::Ipv4(ip), port), lp).is_err() {
                n.sockets.remove(h);
                return Err(String::from("Ошибка подключения"));
            }
        }
        let t = timer::millis();
        let conn = Tcp { h };
        loop {
            poll();
            let s = conn.sock();
            if s.may_send() { return Ok(conn); }
            if s.state() == tcp::State::Closed {
                return Err(format!("Сервер {}:{} отклонил соединение", ip, port));
            }
            if timer::millis() - t > timeout_ms {
                return Err(format!("Время ожидания истекло ({}:{})", ip, port));
            }
        }
    }

    fn sock(&self) -> &mut tcp::Socket<'static> {
        let n = net().unwrap();
        n.sockets.get_mut::<tcp::Socket>(self.h)
    }

    pub fn write_all(&mut self, mut data: &[u8]) -> Result<(), String> {
        let t = timer::millis();
        while !data.is_empty() {
            poll();
            let s = self.sock();
            if !s.may_send() { return Err(String::from("Соединение закрыто сервером")); }
            if s.can_send() {
                let k = s.send_slice(data).map_err(|_| String::from("Ошибка отправки"))?;
                data = &data[k..];
            }
            if timer::millis() - t > 20000 { return Err(String::from("Таймаут отправки")); }
        }
        poll();
        Ok(())
    }

    pub fn read(&mut self, buf: &mut [u8], timeout_ms: u64) -> Result<usize, String> {
        let t = timer::millis();
        loop {
            poll();
            let s = self.sock();
            if s.can_recv() {
                return s.recv_slice(buf).map_err(|_| String::from("Ошибка приёма"));
            }
            if !s.may_recv() { return Ok(0); }
            if timer::millis() - t > timeout_ms { return Err(String::from("Сервер не отвечает (таймаут)")); }
        }
    }
}

impl Drop for Tcp {
    fn drop(&mut self) {
        if let Some(n) = net() {
            let s = n.sockets.get_mut::<tcp::Socket>(self.h);
            s.abort();
            poll();
            let n = net().unwrap();
            n.sockets.remove(self.h);
        }
    }
}
