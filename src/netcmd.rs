use alloc::string::String;
use alloc::format;
use alloc::boxed::Box;
use crate::net;

fn leak(s: String) -> &'static str { Box::leak(s.into_boxed_str()) }

pub fn exec(cmd: &str) -> Option<&'static str> {
    let mut parts = cmd.split_whitespace();
    let c = parts.next()?;
    match c {
        "ifconfig" | "ipconfig" | "netinfo" => {
            let mut s = format!("Состояние: {}\n", net::status_text());
            if let Some(n) = net::net() {
                s += &format!("Адаптер: {} ({} Мбит/с)\nMAC: {}\n", n.adapter_name, net::link_speed(), net::mac_string());
            }
            for a in net::adapters().iter() {
                if a.kind == net::Kind::WiFi { s += &format!("Wi-Fi: {} (нет драйвера)\n", a.name); }
            }
            s += &format!("IP: {}\nШлюз: {}\nDNS: {}", net::ip_string(), net::gw_string(), net::dns_string());
            Some(leak(s))
        }
        "nslookup" | "resolve" => {
            let host = parts.next().unwrap_or("google.com");
            Some(leak(match net::resolve(host) { Ok(ip) => format!("{} -> {}", host, ip), Err(e) => e }))
        }
        "wget" | "curl" | "fetch" => {
            let mut u = parts.next().unwrap_or("google.com");
            let mut scheme = "http";
            if u == "-s" { scheme = "https"; u = parts.next().unwrap_or("google.com"); }
            let url = if u.contains("://") { String::from(u) } else { format!("{}://{}", scheme, u) };
            let url = match net::http::Url::parse(&url) { Some(x) => x, None => return Some("Неверный адрес") };
            let mut p = |_: &str| {};
            Some(leak(match net::http::fetch(&url, None, &mut p) {
                Ok(r) => {
                    let body = String::from_utf8_lossy(&r.body);
                    let first: String = body.chars().take(160).filter(|c| *c != '\r').collect();
                    format!("HTTP {} , {} байт, {}\n{}", r.status, r.body.len(), r.content_type(), first)
                }
                Err(e) => e,
            }))
        }
        "browser" | "web" => {
            if let Some(u) = parts.next() {
                let url = if u.contains("://") { String::from(u) } else { format!("https://{}", u) };
                crate::web::app::open(&url);
            }
            unsafe { crate::NET_APP_REQUEST = 1; }
            Some("Открываю браузер...")
        }
        "net" | "network" | "wifi" => { unsafe { crate::NET_APP_REQUEST = 2; } Some("Открываю настройки сети...") }
        _ => None,
    }
}
