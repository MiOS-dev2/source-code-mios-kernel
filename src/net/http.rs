use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use alloc::format;
use embedded_tls::blocking::*;
use rand_core::{CryptoRng, RngCore};

use super::Tcp;
use crate::timer;

#[derive(Clone, Debug)]
pub struct Url {
    pub https: bool,
    pub host: String,
    pub port: u16,
    pub path: String,
    pub fragment: String,
}

impl Url {
    pub fn parse(s: &str) -> Option<Url> {
        let s = s.trim();
        let (https, rest) = if let Some(r) = strip_prefix_ci(s, "https://") { (true, r) }
            else if let Some(r) = strip_prefix_ci(s, "http://") { (false, r) }
            else { return None };
        let (hostport, path) = match rest.find(|c| c == '/' || c == '?' || c == '#') {
            Some(i) => (&rest[..i], &rest[i..]),
            None => (rest, "/"),
        };
        let hostport = match hostport.rfind('@') { Some(i) => &hostport[i + 1..], None => hostport };
        let (host, port) = match hostport.rfind(':') {
            Some(i) => (&hostport[..i], hostport[i + 1..].parse().ok()?),
            None => (hostport, if https { 443 } else { 80 }),
        };
        if host.is_empty() { return None; }
        let mut path = String::from(path);
        let mut fragment = String::new();
        if let Some(i) = path.find('#') { fragment = path[i + 1..].to_string(); path.truncate(i); }
        if !path.starts_with('/') { path.insert(0, '/'); }
        Some(Url { https, host: host.to_ascii_lowercase(), port, path, fragment })
    }

    pub fn to_string(&self) -> String {
        let def = if self.https { 443 } else { 80 };
        let mut s = format!("{}://{}", if self.https { "https" } else { "http" }, self.host);
        if self.port != def { s += &format!(":{}", self.port); }
        s += &self.path;
        if !self.fragment.is_empty() { s.push('#'); s += &self.fragment; }
        s
    }

    pub fn join(&self, href: &str) -> Option<Url> {
        let href = href.trim();
        if href.is_empty() { return Some(self.clone()); }
        if let Some(u) = Url::parse(href) { return Some(u); }
        let lower = href.to_ascii_lowercase();
        if lower.starts_with("javascript:") || lower.starts_with("mailto:") || lower.starts_with("data:") || lower.starts_with("tel:") {
            return None;
        }
        if href.starts_with("//") {
            return Url::parse(&format!("{}:{}", if self.https { "https" } else { "http" }, href));
        }
        let mut u = self.clone();
        u.fragment.clear();
        if href.starts_with('#') { u.fragment = href[1..].to_string(); return Some(u); }
        let (href, frag) = match href.find('#') { Some(i) => (&href[..i], &href[i + 1..]), None => (href, "") };
        u.fragment = frag.to_string();
        if href.starts_with('/') {
            u.path = normalize(href);
        } else if href.starts_with('?') {
            let base = match u.path.find('?') { Some(i) => &u.path[..i], None => &u.path[..] };
            u.path = format!("{}{}", base, href);
        } else {
            let base = match u.path.find('?') { Some(i) => &u.path[..i], None => &u.path[..] };
            let dir = match base.rfind('/') { Some(i) => &base[..i + 1], None => "/" };
            u.path = normalize(&format!("{}{}", dir, href));
        }
        Some(u)
    }
}

fn strip_prefix_ci<'a>(s: &'a str, p: &str) -> Option<&'a str> {
    if s.len() >= p.len() && s[..p.len()].eq_ignore_ascii_case(p) { Some(&s[p.len()..]) } else { None }
}

fn normalize(p: &str) -> String {
    let (path, query) = match p.find('?') { Some(i) => (&p[..i], &p[i..]), None => (p, "") };
    let mut parts: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." => { parts.pop(); }
            s => parts.push(s),
        }
    }
    let mut out = String::from("/");
    out += &parts.join("/");
    if path.ends_with('/') && out.len() > 1 { out.push('/'); }
    out += query;
    out
}

pub fn url_encode(s: &str) -> String {
    let mut o = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => o.push(b as char),
            b' ' => o.push('+'),
            _ => o += &format!("%{:02X}", b),
        }
    }
    o
}

pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub url: Url,
}

impl Response {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str())
    }
    pub fn content_type(&self) -> String {
        self.header("content-type").unwrap_or("text/html").to_ascii_lowercase()
    }
}

struct Cookie { domain: String, name: String, value: String }
static mut COOKIES: Vec<Cookie> = Vec::new();

fn cookies() -> &'static mut Vec<Cookie> { unsafe { &mut *(&raw mut COOKIES) } }

fn store_cookie(host: &str, v: &str) {
    let mut parts = v.split(';');
    let kv = match parts.next() { Some(k) => k.trim(), None => return };
    let (name, value) = match kv.find('=') { Some(i) => (&kv[..i], &kv[i + 1..]), None => return };
    let mut domain = host.to_string();
    for p in parts {
        let p = p.trim();
        if p.len() > 7 && p[..7].eq_ignore_ascii_case("domain=") {
            domain = p[7..].trim_start_matches('.').to_ascii_lowercase();
        }
    }
    let c = cookies();
    c.retain(|x| !(x.domain == domain && x.name == name));
    if c.len() < 300 { c.push(Cookie { domain, name: name.to_string(), value: value.to_string() }); }
}

fn cookie_header(host: &str) -> String {
    let mut v: Vec<String> = Vec::new();
    for c in cookies().iter() {
        if host == c.domain || host.ends_with(&format!(".{}", c.domain)) {
            v.push(format!("{}={}", c.name, c.value));
        }
    }
    v.join("; ")
}

struct Rng(u64, u64);
impl Rng {
    fn new() -> Self {
        let mut a = timer::rdtsc() ^ 0xA076_1D64_78BD_642F;
        let mut b = timer::rdtsc().rotate_left(29) ^ 0xE703_7ED1_A0B4_28DB;
        if let Some(r) = rdrand() { a ^= r; }
        if let Some(r) = rdrand() { b ^= r; }
        let mut r = Rng(a | 1, b | 2);
        for _ in 0..16 { r.next_u64(); }
        r
    }
}
fn rdrand() -> Option<u64> {
    let ecx: u32;
    unsafe {
        let r = core::arch::x86_64::__cpuid(1);
        ecx = r.ecx;
    }
    if ecx & (1 << 30) == 0 { return None; }
    let mut v: u64;
    let ok: u8;
    unsafe { core::arch::asm!("rdrand {0}", "setc {1}", out(reg) v, out(reg_byte) ok); }
    if ok == 1 { Some(v) } else { v = 0; let _ = v; None }
}
impl RngCore for Rng {
    fn next_u32(&mut self) -> u32 { self.next_u64() as u32 }
    fn next_u64(&mut self) -> u64 {
        let mut s1 = self.0;
        let s0 = self.1;
        self.0 = s0;
        s1 ^= s1 << 23;
        self.1 = s1 ^ s0 ^ (s1 >> 17) ^ (s0 >> 26);
        self.1.wrapping_add(s0) ^ (timer::rdtsc().rotate_left(17))
    }
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        for ch in dest.chunks_mut(8) {
            let v = self.next_u64().to_le_bytes();
            ch.copy_from_slice(&v[..ch.len()]);
        }
    }
    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand_core::Error> { self.fill_bytes(dest); Ok(()) }
}
impl CryptoRng for Rng {}

struct TcpIo(Tcp);
impl embedded_io::ErrorType for TcpIo { type Error = embedded_io::ErrorKind; }
impl embedded_io::Read for TcpIo {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.0.read(buf, 20000).map_err(|_| embedded_io::ErrorKind::TimedOut)
    }
}
impl embedded_io::Write for TcpIo {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.0.write_all(buf).map_err(|_| embedded_io::ErrorKind::BrokenPipe)?;
        Ok(buf.len())
    }
    fn flush(&mut self) -> Result<(), Self::Error> { Ok(()) }
}

pub type Progress<'a> = &'a mut dyn FnMut(&str);

fn user_agent(host: &str) -> &'static str {
    if host == "google.com" || host.ends_with(".google.com") || host.starts_with("google.") || host.contains(".google.") {
        "Lynx/2.9.0dev.12 libwww-FM/2.14 SSL-MM/1.4.1 OpenSSL/3.0.13"
    } else {
        "Mozilla/5.0 (X11; MiUX x86_64) MiUXBrowser/1.0 (KHTML, like Gecko)"
    }
}

fn build_request(method: &str, url: &Url, body: Option<&[u8]>, ctype: &str) -> Vec<u8> {
    let host = if (url.https && url.port == 443) || (!url.https && url.port == 80) { url.host.clone() } else { format!("{}:{}", url.host, url.port) };
    let mut r = format!(
        "{} {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: {}\r\nAccept: text/html,application/xhtml+xml,text/css,*/*;q=0.8\r\nAccept-Language: ru,en;q=0.8\r\nAccept-Encoding: gzip, deflate\r\nConnection: close\r\n",
        method, url.path, host, user_agent(&url.host));
    let ck = cookie_header(&url.host);
    if !ck.is_empty() { r += &format!("Cookie: {}\r\n", ck); }
    if let Some(b) = body {
        r += &format!("Content-Type: {}\r\nContent-Length: {}\r\n", ctype, b.len());
    }
    r += "\r\n";
    let mut v = r.into_bytes();
    if let Some(b) = body { v.extend_from_slice(b); }
    v
}

const MAX_BODY: usize = 12 * 1024 * 1024;

fn raw_request(url: &Url, req: &[u8], progress: Progress) -> Result<Vec<u8>, String> {
    progress(&format!("Поиск {}...", url.host));
    let ip = super::resolve(&url.host)?;
    progress(&format!("Подключение к {} ({})...", url.host, ip));
    let tcp = Tcp::connect(ip, url.port, 15000)?;
    let mut out: Vec<u8> = Vec::new();
    let t0 = timer::millis();
    if url.https {
        progress(&format!("Защищённое соединение с {}...", url.host));
        let mut rbuf = vec![0u8; 16640];
        let mut wbuf = vec![0u8; 16640];
        let config = TlsConfig::new().with_server_name(&url.host).enable_rsa_signatures();
        let mut tls: TlsConnection<TcpIo, Aes128GcmSha256> = TlsConnection::new(TcpIo(tcp), &mut rbuf, &mut wbuf);
        let rng = Rng::new();
        tls.open(TlsContext::new(&config, UnsecureProvider::new::<Aes128GcmSha256>(rng)))
            .map_err(|e| format!("Ошибка TLS: {:?}", e))?;
        tls.write(req).map_err(|e| format!("Ошибка TLS: {:?}", e))?;
        tls.flush().map_err(|e| format!("Ошибка TLS: {:?}", e))?;
        progress(&format!("Загрузка {}...", url.host));
        let mut buf = vec![0u8; 16384];
        loop {
            match tls.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    out.extend_from_slice(&buf[..n]);
                    if response_complete(&out) { break; }
                }
                Err(embedded_tls::TlsError::ConnectionClosed) => break,
                Err(e) => { if out.is_empty() { return Err(format!("Ошибка TLS: {:?}", e)); } break; }
            }
            if out.len() > MAX_BODY || timer::millis() - t0 > 60000 { break; }
        }
    } else {
        let mut tcp = tcp;
        tcp.write_all(req)?;
        progress(&format!("Загрузка {}...", url.host));
        let mut buf = vec![0u8; 32768];
        loop {
            let n = match tcp.read(&mut buf, 20000) { Ok(n) => n, Err(e) => { if out.is_empty() { return Err(e); } 0 } };
            if n == 0 { break; }
            out.extend_from_slice(&buf[..n]);
            if response_complete(&out) { break; }
            if out.len() > MAX_BODY || timer::millis() - t0 > 60000 { break; }
        }
    }
    if out.is_empty() { return Err(String::from("Сервер ничего не ответил")); }
    Ok(out)
}

fn find(h: &[u8], n: &[u8]) -> Option<usize> { h.windows(n.len()).position(|w| w == n) }

fn response_complete(d: &[u8]) -> bool {
    let he = match find(d, b"\r\n\r\n") { Some(i) => i, None => return false };
    let head = String::from_utf8_lossy(&d[..he]).to_ascii_lowercase();
    let body = &d[he + 4..];
    for line in head.split("\r\n") {
        if let Some(v) = line.strip_prefix("content-length:") {
            if let Ok(n) = v.trim().parse::<usize>() { return body.len() >= n; }
        }
        if line.starts_with("transfer-encoding:") && line.contains("chunked") {
            return body.ends_with(b"0\r\n\r\n");
        }
    }
    let first = head.lines().next().unwrap_or("");
    first.contains(" 204") || first.contains(" 304")
}

fn dechunk(b: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let le = match find(&b[i..], b"\r\n") { Some(x) => i + x, None => break };
        let line = core::str::from_utf8(&b[i..le]).unwrap_or("0");
        let hex = line.split(';').next().unwrap_or("0").trim();
        let n = usize::from_str_radix(hex, 16).unwrap_or(0);
        if n == 0 { break; }
        let s = le + 2;
        let e = (s + n).min(b.len());
        out.extend_from_slice(&b[s..e]);
        i = e + 2;
    }
    out
}

fn parse_response(raw: Vec<u8>, url: &Url) -> Result<Response, String> {
    let he = find(&raw, b"\r\n\r\n").ok_or_else(|| String::from("Неверный ответ сервера"))?;
    let head = String::from_utf8_lossy(&raw[..he]).to_string();
    let mut lines = head.split("\r\n");
    let first = lines.next().unwrap_or("");
    let status: u16 = first.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let mut headers = Vec::new();
    for l in lines {
        if let Some(i) = l.find(':') {
            headers.push((l[..i].trim().to_string(), l[i + 1..].trim().to_string()));
        }
    }
    let mut body = raw[he + 4..].to_vec();
    let mut r = Response { status, headers, body: Vec::new(), url: url.clone() };
    if r.header("transfer-encoding").map(|v| v.to_ascii_lowercase().contains("chunked")).unwrap_or(false) {
        body = dechunk(&body);
    }
    let enc = r.header("content-encoding").unwrap_or("").to_ascii_lowercase();
    if enc.contains("gzip") {
        body = gunzip(&body).unwrap_or(body);
    } else if enc.contains("deflate") {
        body = miniz_oxide::inflate::decompress_to_vec_zlib(&body)
            .or_else(|_| miniz_oxide::inflate::decompress_to_vec(&body)).unwrap_or(body);
    }
    for (k, v) in r.headers.iter() {
        if k.eq_ignore_ascii_case("set-cookie") { store_cookie(&url.host, v); }
    }
    r.body = body;
    Ok(r)
}

fn gunzip(d: &[u8]) -> Option<Vec<u8>> {
    if d.len() < 18 || d[0] != 0x1F || d[1] != 0x8B { return None; }
    let flg = d[3];
    let mut i = 10;
    if flg & 4 != 0 { let x = u16::from_le_bytes([d[i], d[i + 1]]) as usize; i += 2 + x; }
    if flg & 8 != 0 { while i < d.len() && d[i] != 0 { i += 1; } i += 1; }
    if flg & 16 != 0 { while i < d.len() && d[i] != 0 { i += 1; } i += 1; }
    if flg & 2 != 0 { i += 2; }
    if i >= d.len() { return None; }
    miniz_oxide::inflate::decompress_to_vec(&d[i..]).ok()
}

pub fn fetch(url: &Url, post: Option<(&[u8], &str)>, progress: Progress) -> Result<Response, String> {
    let mut u = url.clone();
    let mut method = if post.is_some() { "POST" } else { "GET" };
    let mut body = post;
    for _ in 0..10 {
        let req = build_request(method, &u, body.map(|b| b.0), body.map(|b| b.1).unwrap_or(""));
        let raw = raw_request(&u, &req, progress)?;
        let r = parse_response(raw, &u)?;
        if matches!(r.status, 301 | 302 | 303 | 307 | 308) {
            if let Some(loc) = r.header("location") {
                let frag = u.fragment.clone();
                u = u.join(loc).ok_or_else(|| String::from("Неверный адрес перенаправления"))?;
                if u.fragment.is_empty() { u.fragment = frag; }
                if r.status != 307 && r.status != 308 { method = "GET"; body = None; }
                progress(&format!("Перенаправление: {}", u.to_string()));
                continue;
            }
        }
        return Ok(r);
    }
    Err(String::from("Слишком много перенаправлений"))
}
