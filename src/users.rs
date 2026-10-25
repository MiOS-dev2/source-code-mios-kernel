

#![allow(dead_code)]

use core::str;
use core::fmt::Write;


struct StringWriter {
    buf: [u8; 512],
    pos: usize,
}

impl StringWriter {
    fn new() -> Self {
        Self {
            buf: [0; 512],
            pos: 0,
        }
    }
    
    fn write_str(&mut self, s: &str) {
        let bytes = s.as_bytes();
        for &b in bytes.iter().take(512 - self.pos) {
            self.buf[self.pos] = b;
            self.pos += 1;
        }
    }
    
    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buf[..self.pos]).unwrap_or("")
    }
}


#[derive(Clone, Copy)]
pub struct User {
    pub username: [u8; 32],
    pub password_hash: [u8; 32],
    pub is_admin: bool,
    pub home_dir: [u8; 64],
    pub created: u32,
}

impl User {
    pub const fn new() -> Self {
        Self {
            username: [0; 32],
            password_hash: [0; 32],
            is_admin: false,
            home_dir: [0; 64],
            created: 0,
        }
    }
    
    pub fn set_username(&mut self, name: &str) {
        let bytes = name.as_bytes();
        let len = if bytes.len() > 31 { 31 } else { bytes.len() };
        self.username[..len].copy_from_slice(&bytes[..len]);
        self.username[len] = 0;
    }
    
    pub fn get_username(&self) -> &str {
        let end = self.username.iter().position(|&b| b == 0).unwrap_or(31);
        core::str::from_utf8(&self.username[..end]).unwrap_or("")
    }
    
    pub fn set_password(&mut self, password: &str) {
        let bytes = password.as_bytes();
        let len = if bytes.len() > 31 { 31 } else { bytes.len() };
        self.password_hash[..len].copy_from_slice(&bytes[..len]);
        self.password_hash[len] = 0;
        for i in 0..len {
            self.password_hash[i] ^= 0x5A;
        }
    }
    
    pub fn check_password(&self, password: &str) -> bool {
        let bytes = password.as_bytes();
        let len = if bytes.len() > 31 { 31 } else { bytes.len() };
        let mut hash = [0u8; 32];
        hash[..len].copy_from_slice(&bytes[..len]);
        for i in 0..len {
            hash[i] ^= 0x5A;
        }
        self.password_hash[..len] == hash[..len] && self.password_hash[len] == 0
    }
    
    pub fn set_home_dir(&mut self, path: &str) {
        let bytes = path.as_bytes();
        let len = if bytes.len() > 63 { 63 } else { bytes.len() };
        self.home_dir[..len].copy_from_slice(&bytes[..len]);
        self.home_dir[len] = 0;
    }
    
    pub fn get_home_dir(&self) -> &str {
        let end = self.home_dir.iter().position(|&b| b == 0).unwrap_or(63);
        core::str::from_utf8(&self.home_dir[..end]).unwrap_or("/")
    }
}


pub struct UserManager {
    users: [User; 16],
    user_count: usize,
    current_user: usize,
    logged_in: bool,
    session_id: u32,
}

impl UserManager {
    pub const fn new() -> Self {
        Self {
            users: [User::new(); 16],
            user_count: 0,
            current_user: 0,
            logged_in: false,
            session_id: 0,
        }
    }
    
    pub fn init(&mut self) {
        self.create_user("admin", "1234", true);
        self.create_user("system", "1234", false);
        self.create_user("user", "", false);
    }
    
    pub fn create_user(&mut self, username: &str, password: &str, is_admin: bool) -> bool {
        if self.user_count >= 16 {
            return false;
        }
        
        for i in 0..self.user_count {
            if self.users[i].get_username() == username {
                return false;
            }
        }
        
        let mut user = User::new();
        user.set_username(username);
        user.set_password(password);
        user.is_admin = is_admin;
        let home = "/miux/users/";
        user.set_home_dir(home);
        user.created = 0;
        
        self.users[self.user_count] = user;
        self.user_count += 1;
        true
    }
    
    pub fn delete_user(&mut self, username: &str) -> bool {
        if username == "admin" {
            return false;
        }
        
        for i in 0..self.user_count {
            if self.users[i].get_username() == username {
                for j in i..self.user_count - 1 {
                    self.users[j] = self.users[j + 1];
                }
                self.user_count -= 1;
                if self.current_user == i {
                    self.current_user = 0;
                }
                return true;
            }
        }
        false
    }
    
    pub fn login(&mut self, username: &str, password: &str) -> bool {
        for i in 0..self.user_count {
            if self.users[i].get_username() == username {
                if self.users[i].check_password(password) {
                    self.current_user = i;
                    self.logged_in = true;
                    self.session_id += 1;
                    return true;
                }
            }
        }
        false
    }
    
    pub fn logout(&mut self) {
        self.logged_in = false;
    }
    
    pub fn get_current_user(&self) -> Option<&User> {
        if self.logged_in {
            Some(&self.users[self.current_user])
        } else {
            None
        }
    }
    
    pub fn get_current_user_mut(&mut self) -> Option<&mut User> {
        if self.logged_in {
            Some(&mut self.users[self.current_user])
        } else {
            None
        }
    }
    
    pub fn is_logged_in(&self) -> bool {
        self.logged_in
    }
    
    pub fn is_admin(&self) -> bool {
        self.logged_in && self.users[self.current_user].is_admin
    }
    
    pub fn get_user_list(&self) -> &[User] {
        &self.users[..self.user_count]
    }
    
    pub fn get_user_count(&self) -> usize {
        self.user_count
    }
    
    pub fn change_password(&mut self, username: &str, new_password: &str) -> bool {
        for i in 0..self.user_count {
            if self.users[i].get_username() == username {
                self.users[i].set_password(new_password);
                return true;
            }
        }
        false
    }
    
    pub fn list_users(&self) -> &str {
        static mut OUTPUT: [u8; 512] = [0; 512];
        unsafe {
            let mut pos = 0;
            let header = b"Users:\n";
            for b in header {
                if pos < 511 { OUTPUT[pos] = *b; pos += 1; }
            }
            
            for i in 0..self.user_count {
                let name = self.users[i].get_username();
                let role = if self.users[i].is_admin { "[ADMIN]" } else { "[USER]" };
                let logged = if i == self.current_user && self.logged_in { "[*]" } else { "[ ]" };
                
                let line_bytes = logged.as_bytes();
                for b in line_bytes {
                    if pos < 511 { OUTPUT[pos] = *b; pos += 1; }
                }
                OUTPUT[pos] = b' ';
                pos += 1;
                for b in role.as_bytes() {
                    if pos < 511 { OUTPUT[pos] = *b; pos += 1; }
                }
                OUTPUT[pos] = b' ';
                pos += 1;
                for b in name.as_bytes() {
                    if pos < 511 { OUTPUT[pos] = *b; pos += 1; }
                }
                if pos < 511 { OUTPUT[pos] = b'\n'; pos += 1; }
            }
            OUTPUT[pos] = 0;
            core::str::from_utf8(&OUTPUT[..pos]).unwrap_or("")
        }
    }
}


static mut USER_MANAGER: UserManager = UserManager::new();


pub fn init_user_system() {
    unsafe {
        USER_MANAGER.init();
    }
}

pub fn login(username: &str, password: &str) -> bool {
    unsafe {
        USER_MANAGER.login(username, password)
    }
}

pub fn logout() {
    unsafe {
        USER_MANAGER.logout();
    }
}

pub fn is_logged_in() -> bool {
    unsafe {
        USER_MANAGER.is_logged_in()
    }
}

pub fn is_admin() -> bool {
    unsafe {
        USER_MANAGER.is_admin()
    }
}

pub fn create_user(username: &str, password: &str, is_admin: bool) -> bool {
    unsafe {
        USER_MANAGER.create_user(username, password, is_admin)
    }
}

pub fn delete_user(username: &str) -> bool {
    unsafe {
        USER_MANAGER.delete_user(username)
    }
}

pub fn change_password(username: &str, new_password: &str) -> bool {
    unsafe {
        USER_MANAGER.change_password(username, new_password)
    }
}

pub fn list_users() -> &'static str {
    unsafe {
        USER_MANAGER.list_users()
    }
}

pub fn get_current_username() -> &'static str {
    unsafe {
        if let Some(user) = USER_MANAGER.get_current_user() {
            user.get_username()
        } else {
            "not_logged_in"
        }
    }
}

pub fn get_current_user_home() -> &'static str {
    unsafe {
        if let Some(user) = USER_MANAGER.get_current_user() {
            user.get_home_dir()
        } else {
            "/"
        }
    }
}



static mut AUTO_PENDING: bool = false;
static mut AUTO_NAME: [u8; 32] = [0; 32];
static mut AUTO_NAME_LEN: usize = 0;
static mut AUTO_PASS: [u8; 32] = [0; 32];
static mut AUTO_PASS_LEN: usize = 0;

fn cfg_value<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    for line in text.split('\n') {
        let line = line.trim_end_matches('\r');
        if let Some(eq) = line.find('=') {
            if line[..eq].trim() == key {
                return Some(&line[eq + 1..]);
            }
        }
    }
    None
}

pub fn load_installed(info_addr: u32) {
    if !crate::syscfg::booted_from_disk(info_addr) {
        return;
    }
    let mut buf = [0u8; 256];
    let n = match unsafe {
        crate::fs::fat32::load_file_in_subdir(
            crate::fs::fat32::SYS_DIR,
            crate::fs::fat32::USERS_DIR,
            "user.cfg",
            &mut buf,
        )
    } {
        Ok(n) => n,
        Err(_) => return,
    };
    let n = if n > buf.len() { buf.len() } else { n };
    let text = match core::str::from_utf8(&buf[..n]) {
        Ok(t) => t,
        Err(_) => return,
    };
    let name = match cfg_value(text, "name") {
        Some(v) if !v.is_empty() => v,
        _ => return,
    };
    let pass = cfg_value(text, "password").unwrap_or("");
    if pass.is_empty() {
        return;
    }
    unsafe {
        if !USER_MANAGER.create_user(name, pass, true) {
            return;
        }
    }

    let mut buf2 = [0u8; 64];
    let m = match unsafe {
        crate::fs::fat32::load_file_in_subdir(
            crate::fs::fat32::SYS_DIR,
            crate::fs::fat32::USERS_DIR,
            "auto.cfg",
            &mut buf2,
        )
    } {
        Ok(m) => m,
        Err(_) => return,
    };
    let m = if m > buf2.len() { buf2.len() } else { m };
    let t2 = match core::str::from_utf8(&buf2[..m]) {
        Ok(t) => t,
        Err(_) => return,
    };
    if cfg_value(t2, "autologin").map(|v| v.trim() == "1").unwrap_or(false) {
        let nb = name.as_bytes();
        let pb = pass.as_bytes();
        let nl = if nb.len() > 31 { 31 } else { nb.len() };
        let pl = if pb.len() > 31 { 31 } else { pb.len() };
        unsafe {
            AUTO_NAME[..nl].copy_from_slice(&nb[..nl]);
            AUTO_NAME_LEN = nl;
            AUTO_PASS[..pl].copy_from_slice(&pb[..pl]);
            AUTO_PASS_LEN = pl;
            AUTO_PENDING = true;
        }
    }
}

pub fn autologin_pending() -> bool {
    unsafe { AUTO_PENDING }
}

pub fn try_autologin() -> bool {
    unsafe {
        if !AUTO_PENDING {
            return false;
        }
        AUTO_PENDING = false;
        let name = core::str::from_utf8(&AUTO_NAME[..AUTO_NAME_LEN]).unwrap_or("");
        let pass = core::str::from_utf8(&AUTO_PASS[..AUTO_PASS_LEN]).unwrap_or("");
        let ok = USER_MANAGER.login(name, pass);
        AUTO_PASS = [0; 32];
        AUTO_PASS_LEN = 0;
        ok
    }
}
