
use crate::fs::fat32::{self, FAT32FS, FsEntry};
use crate::users;

const MAX_DEPTH: usize = 16;
const MAX_ENTRIES: usize = 1024;
const READ_MAX: usize = 65536;
const MAX_LINES: usize = 900;
const LINE_BYTES: usize = 79;
const OUT_MAX: usize = 73000;

#[derive(Clone, Copy)]
struct Level {
    cluster: u32,
    name: [u8; 13],
    len: usize,
}

#[derive(Clone, Copy)]
struct Cwd {
    depth: usize,
    lv: [Level; MAX_DEPTH],
}

impl Cwd {
    const fn new() -> Cwd {
        Cwd { depth: 0, lv: [Level { cluster: 0, name: [0; 13], len: 0 }; MAX_DEPTH] }
    }
}

static mut CWD: Cwd = Cwd::new();
static mut CWD_DISK: usize = usize::MAX;
static mut CWD_ROOT: u32 = 0;

static mut ENTRIES: [FsEntry; MAX_ENTRIES] = [FsEntry::EMPTY; MAX_ENTRIES];
static mut FILE_BUF: [u8; READ_MAX] = [0; READ_MAX];
static mut OUT: [u8; OUT_MAX] = [0; OUT_MAX];
static mut PROMPT: [u8; 160] = [0; 160];

struct Out {
    len: usize,
}

impl Out {
    fn new() -> Out {
        Out { len: 0 }
    }
    unsafe fn bytes(&mut self, b: &[u8]) {
        for &c in b {
            if self.len < OUT_MAX {
                OUT[self.len] = c;
                self.len += 1;
            }
        }
    }
    unsafe fn s(&mut self, s: &str) {
        self.bytes(s.as_bytes());
    }
    unsafe fn num(&mut self, n: u32) {
        let mut tmp = [0u8; 10];
        let mut i = 0;
        let mut v = n;
        if v == 0 {
            tmp[0] = b'0';
            i = 1;
        }
        while v > 0 {
            tmp[i] = b'0' + (v % 10) as u8;
            v /= 10;
            i += 1;
        }
        while i > 0 {
            i -= 1;
            self.bytes(&tmp[i..i + 1]);
        }
    }
    unsafe fn num_pad(&mut self, n: u32, width: usize) {
        let mut digits = 1usize;
        let mut v = n;
        while v >= 10 {
            v /= 10;
            digits += 1;
        }
        let mut k = digits;
        while k < width {
            self.bytes(b" ");
            k += 1;
        }
        self.num(n);
    }
    unsafe fn finish(&self) -> &'static str {
        core::str::from_utf8(&OUT[..self.len]).unwrap_or("")
    }
}

fn clip(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    let mut n = max;
    while n > 0 && !s.is_char_boundary(n) {
        n -= 1;
    }
    &s[..n]
}

fn eq_ci(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    for i in 0..a.len() {
        if a[i].to_ascii_uppercase() != b[i].to_ascii_uppercase() {
            return false;
        }
    }
    true
}

unsafe fn err3(cmd: &str, arg: &str, msg: &str) -> &'static str {
    let mut o = Out::new();
    o.s(cmd);
    o.s(": ");
    o.s(clip(arg, 40));
    o.s(": ");
    o.s(clip(msg, 30));
    o.finish()
}

unsafe fn get_fs() -> Result<&'static FAT32FS, &'static str> {
    if !fat32::ensure_mounted() {
        return Err("FAT32 not mounted! Use: mount <disk_number>");
    }
    match fat32::fs_ref() {
        Some(fs) => {
            if CWD_DISK != fs.disk() || CWD_ROOT != fs.root() {
                CWD.depth = 0;
                CWD_DISK = fs.disk();
                CWD_ROOT = fs.root();
            }
            Ok(fs)
        }
        None => Err("FAT32 not mounted! Use: mount <disk_number>"),
    }
}

fn cluster_of(fs: &FAT32FS, st: &Cwd) -> u32 {
    if st.depth == 0 {
        fs.root()
    } else {
        st.lv[st.depth - 1].cluster
    }
}

unsafe fn find_in_dir(fs: &FAT32FS, dir: u32, name: &str) -> Option<FsEntry> {
    let n = fs.list_dir(dir, &mut (*core::ptr::addr_of_mut!(ENTRIES))[..]);
    let nb = name.as_bytes();
    let mut short = [0u8; 13];
    let sl = fat32::display_name(name, &mut short);
    for i in 0..n {
        let e = ENTRIES[i];
        let en = &e.name[..e.name_len];
        if eq_ci(en, nb) || (sl > 0 && eq_ci(en, &short[..sl])) {
            return Some(e);
        }
    }
    None
}

unsafe fn step(fs: &FAT32FS, st: &mut Cwd, part: &str) -> Result<(), &'static str> {
    if part == "." {
        return Ok(());
    }
    if part == ".." {
        if st.depth > 0 {
            st.depth -= 1;
        }
        return Ok(());
    }
    let dir = cluster_of(fs, st);
    match find_in_dir(fs, dir, part) {
        Some(e) => {
            if !e.is_dir {
                return Err("Not a directory");
            }
            if st.depth >= MAX_DEPTH {
                return Err("Path too deep");
            }
            let d = st.depth;
            st.lv[d].cluster = if e.cluster < 2 { fs.root() } else { e.cluster };
            st.lv[d].name = e.name;
            st.lv[d].len = e.name_len;
            st.depth = d + 1;
            Ok(())
        }
        None => Err("No such file or directory"),
    }
}

unsafe fn walk_dirs(fs: &FAT32FS, st: &mut Cwd, path: &str) -> Result<(), &'static str> {
    if path.starts_with('/') {
        st.depth = 0;
    }
    for part in path.split('/') {
        if part.is_empty() {
            continue;
        }
        step(fs, st, part)?;
    }
    Ok(())
}

fn short11(e: &FsEntry) -> [u8; 11] {
    let mut out = [b' '; 11];
    let nm = &e.name[..e.name_len];
    let mut i = 0usize;
    let mut o = 0usize;
    while i < nm.len() && nm[i] != b'.' {
        if o < 8 {
            out[o] = nm[i];
            o += 1;
        }
        i += 1;
    }
    if i < nm.len() {
        i += 1;
        let mut x = 0usize;
        while i < nm.len() && x < 3 {
            out[8 + x] = nm[i];
            x += 1;
            i += 1;
        }
    }
    out
}

fn char_len(d: &[u8]) -> usize {
    let b = d[0];
    if b < 0x80 {
        return 1;
    }
    let need = if b >= 0xC2 && b <= 0xDF {
        2
    } else if b >= 0xE0 && b <= 0xEF {
        3
    } else if b >= 0xF0 && b <= 0xF4 {
        4
    } else {
        return 0;
    };
    if d.len() < need {
        return 0;
    }
    if core::str::from_utf8(&d[..need]).is_ok() {
        need
    } else {
        0
    }
}

unsafe fn do_ls() -> &'static str {
    let fs = match get_fs() {
        Ok(f) => f,
        Err(m) => return m,
    };
    let dir = cluster_of(fs, &CWD);
    let n = fs.list_dir(dir, &mut (*core::ptr::addr_of_mut!(ENTRIES))[..]);
    if n == 0 {
        return "(empty folder)";
    }
    let mut o = Out::new();
    let mut dirs = 0u32;
    let mut files = 0u32;
    for pass in 0..2 {
        for i in 0..n {
            let e = ENTRIES[i];
            if e.is_dir != (pass == 0) {
                continue;
            }
            if e.is_dir {
                o.s("<DIR>      ");
                dirs += 1;
            } else {
                o.num_pad(e.size, 10);
                o.s(" ");
                files += 1;
            }
            o.bytes(&e.name[..e.name_len]);
            o.s("\n");
        }
    }
    if n >= MAX_ENTRIES {
        o.s("(list truncated)\n");
    }
    o.num(dirs);
    o.s(" folder(s), ");
    o.num(files);
    o.s(" file(s)");
    o.finish()
}

unsafe fn do_cd(arg: &str) -> &'static str {
    let fs = match get_fs() {
        Ok(f) => f,
        Err(m) => return m,
    };
    let arg = arg.trim();
    let mut st = CWD;
    if arg.is_empty() || arg == "~" {
        st.depth = 0;
    } else if let Err(e) = walk_dirs(fs, &mut st, arg) {
        return err3("cd", arg, e);
    }
    CWD = st;
    ""
}

unsafe fn do_cat(arg: &str) -> &'static str {
    let arg = arg.trim();
    if arg.is_empty() {
        return "Usage: cat <file.ext>";
    }
    let fs = match get_fs() {
        Ok(f) => f,
        Err(m) => return m,
    };
    let (dpath, fname) = match arg.rfind('/') {
        Some(i) => (&arg[..i + 1], &arg[i + 1..]),
        None => ("", arg),
    };
    if fname.is_empty() || fname == "." || fname == ".." {
        return err3("cat", arg, "Is a directory");
    }
    let mut st = CWD;
    if let Err(e) = walk_dirs(fs, &mut st, dpath) {
        return err3("cat", arg, e);
    }
    let dir = cluster_of(fs, &st);
    let e = match find_in_dir(fs, dir, fname) {
        Some(e) => e,
        None => return err3("cat", arg, "No such file or directory"),
    };
    if e.is_dir {
        return err3("cat", arg, "Is a directory");
    }
    if e.size as usize > READ_MAX {
        return err3("cat", arg, "File too big (max 64 KB)");
    }
    if e.size == 0 {
        return "(empty file)";
    }
    let short = short11(&e);
    let n = match fs.read_file_bytes_in(dir, &short, &mut (*core::ptr::addr_of_mut!(FILE_BUF))[..]) {
        Ok(n) => n,
        Err(m) => return err3("cat", arg, m),
    };

    let mut o = Out::new();
    let mut col = 0usize;
    let mut lines = 1usize;
    let mut i = 0usize;
    while i < n {
        let b = FILE_BUF[i];
        if b == b'\r' {
            i += 1;
            continue;
        }
        if b == b'\n' {
            o.s("\n");
            col = 0;
            lines += 1;
            i += 1;
            if lines > MAX_LINES {
                o.s("[... output truncated]");
                break;
            }
            continue;
        }
        let mut one = [0u8; 4];
        let tab = [b' '; 4];
        let len: usize;
        let adv: usize;
        let cbytes: &[u8];
        if b == b'\t' {
            let k = 4 - (col % 4);
            len = k;
            adv = 1;
            cbytes = &tab[..k];
        } else if b < 0x20 || b == 0x7F {
            one[0] = b'.';
            len = 1;
            adv = 1;
            cbytes = &one[..1];
        } else {
            let cl = char_len(&FILE_BUF[i..n]);
            if cl == 0 {
                one[0] = b'?';
                len = 1;
                adv = 1;
                cbytes = &one[..1];
            } else {
                len = cl;
                adv = cl;
                cbytes = &FILE_BUF[i..i + cl];
            }
        }
        if col + len > LINE_BYTES {
            o.s("\n");
            col = 0;
            lines += 1;
            if lines > MAX_LINES {
                o.s("[... output truncated]");
                break;
            }
        }
        o.bytes(cbytes);
        col += len;
        i += adv;
    }
    while o.len > 0 && OUT[o.len - 1] == b'\n' {
        o.len -= 1;
    }
    o.finish()
}

unsafe fn put_pad(o: &mut Out, s: &[u8], width: usize) {
    o.bytes(s);
    let mut k = s.len();
    while k < width {
        o.bytes(b" ");
        k += 1;
    }
}

fn fmt_size(mb: u64, buf: &mut [u8; 20]) -> usize {
    let mut n = 0usize;
    let mut tmp = [0u8; 20];
    let (whole, frac, unit): (u64, Option<u64>, &[u8]) = if mb >= 1024 {
        let t = mb * 10 / 1024;
        (t / 10, Some(t % 10), &b" GB"[..])
    } else {
        (mb, None, &b" MB"[..])
    };
    let mut v = whole;
    let mut i = 0usize;
    if v == 0 {
        tmp[0] = b'0';
        i = 1;
    }
    while v > 0 && i < 20 {
        tmp[i] = b'0' + (v % 10) as u8;
        v /= 10;
        i += 1;
    }
    while i > 0 {
        i -= 1;
        buf[n] = tmp[i];
        n += 1;
    }
    if let Some(f) = frac {
        buf[n] = b'.';
        buf[n + 1] = b'0' + f as u8;
        n += 2;
    }
    for &c in unit {
        buf[n] = c;
        n += 1;
    }
    n
}

unsafe fn do_disks() -> &'static str {
    let count = crate::ata::disk_count();
    let mounted = fat32::fs_ref().map(|f| f.disk());
    let mut o = Out::new();
    let mut shown = 0u32;
    for i in 0..count {
        let d = match crate::ata::get_disk(i) {
            Some(d) if d.present => d,
            _ => continue,
        };
        if shown == 0 {
            o.s(" #  Type          Size       Model\n");
        }
        shown += 1;

        o.num_pad((i + 1) as u32, 2);
        o.s("  ");

        let t: &[u8] = match d.dev_type {
            crate::ata::DeviceType::ATA => b"ATA HDD",
            crate::ata::DeviceType::ATAPI => b"ATAPI CD-ROM",
            _ => b"Unknown",
        };
        put_pad(&mut o, t, 14);

        let ss = if d.sector_size == 0 { 512u64 } else { d.sector_size as u64 };
        let mb = d.sectors * ss / (1024 * 1024);
        let mut sb = [0u8; 20];
        let sl = fmt_size(mb, &mut sb);
        put_pad(&mut o, &sb[..sl], 11);

        let mut ml = 0usize;
        while ml < d.model.len() && d.model[ml] != 0 {
            ml += 1;
        }
        while ml > 0 && d.model[ml - 1] == b' ' {
            ml -= 1;
        }
        if ml > 28 {
            ml = 28;
        }
        let mut k = 0usize;
        while k < ml {
            let c = d.model[k];
            if c >= 0x20 && c < 0x7F {
                o.bytes(&d.model[k..k + 1]);
            } else {
                o.bytes(b"?");
            }
            k += 1;
        }
        if ml == 0 {
            o.s("(unknown)");
        }
        if mounted == Some(i) {
            o.s("  [mounted]");
        }
        o.s("\n");
    }
    if shown == 0 {
        return "No disks found!";
    }
    o.s("Use: mount <number>");
    o.finish()
}

unsafe fn do_open(name: &str) -> Option<&'static str> {
    let name = name.trim();
    if name.is_empty() || name.contains('/') {
        return None;
    }
    if !fat32::is_mounted() {
        return None;
    }
    let fs = match get_fs() {
        Ok(f) => f,
        Err(_) => return None,
    };
    let dir = cluster_of(fs, &CWD);
    match find_in_dir(fs, dir, name) {
        Some(e) if !e.is_dir => Some(do_cat(name)),
        _ => None,
    }
}

unsafe fn build_prompt() -> &'static str {
    unsafe fn put(b: &[u8], n: &mut usize) {
        for &c in b {
            if *n < 159 {
                PROMPT[*n] = c;
                *n += 1;
            }
        }
    }
    let mut n = 0usize;
    put(b"miux-", &mut n);
    let user = if users::is_logged_in() { users::get_current_username() } else { "kernel" };
    put(clip(user, 24).as_bytes(), &mut n);
    put(b":", &mut n);

    if fat32::is_mounted() && CWD.depth > 0 {
        let mut path = [0u8; MAX_DEPTH * 14];
        let mut pl = 0usize;
        for d in 0..CWD.depth {
            if d > 0 {
                path[pl] = b'/';
                pl += 1;
            }
            let l = CWD.lv[d].len;
            path[pl..pl + l].copy_from_slice(&CWD.lv[d].name[..l]);
            pl += l;
        }
        if pl > 40 {
            put(b"...", &mut n);
            put(&path[pl - 40..pl], &mut n);
        } else {
            put(&path[..pl], &mut n);
        }
    }
    put(b"~# ", &mut n);
    core::str::from_utf8(&PROMPT[..n]).unwrap_or("miux-kernel:~# ")
}


pub fn cmd_ls() -> &'static str {
    unsafe { do_ls() }
}

pub fn cmd_cd(arg: &str) -> &'static str {
    unsafe { do_cd(arg) }
}

pub fn cmd_cat(arg: &str) -> &'static str {
    unsafe { do_cat(arg) }
}

pub fn cmd_disks() -> &'static str {
    unsafe { do_disks() }
}

pub fn open_by_name(name: &str) -> Option<&'static str> {
    unsafe { do_open(name) }
}

pub fn prompt() -> &'static str {
    unsafe { build_prompt() }
}
