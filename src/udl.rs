
#![allow(dead_code)]

use core::str;


pub const MAX_USERS: usize = 10;
pub const MAX_NAME_LEN: usize = 32;


#[derive(Copy, Clone)]
pub struct User {
    pub id: u32,
    pub name: [u8; MAX_NAME_LEN],
    pub is_temp: bool,
    pub is_active: bool,
}


pub static mut USERS: [User; MAX_USERS] = [User {
    id: 0,
    name: [0; MAX_NAME_LEN],
    is_temp: false,
    is_active: false,
}; MAX_USERS];


pub static mut USER_COUNT: usize = 0;


pub static mut CURRENT_USER_ID: u32 = 0;


pub static mut IS_LOGGED_IN: bool = false;


pub fn init_users() {
    unsafe {
        let main_name = b"Main User";
        let name_len = main_name.len().min(MAX_NAME_LEN - 1);
        USERS[0].id = 4456;
        USERS[0].name[..name_len].copy_from_slice(&main_name[..name_len]);
        USERS[0].name[name_len] = 0;
        USERS[0].is_temp = false;
        USERS[0].is_active = true;
        
        USER_COUNT = 1;
        CURRENT_USER_ID = 4456;
        IS_LOGGED_IN = false; 
    }
}


pub fn get_user_by_id(id: u32) -> Option<User> {
    unsafe {
        for i in 0..USER_COUNT {
            if USERS[i].id == id && USERS[i].is_active {
                return Some(USERS[i]);
            }
        }
        None
    }
}


pub fn get_user_by_name(name: &str) -> Option<User> {
    unsafe {
        for i in 0..USER_COUNT {
            let user_name = core::str::from_utf8(&USERS[i].name).unwrap_or("");
            let trimmed = user_name.trim_end_matches('\0');
            if trimmed == name && USERS[i].is_active {
                return Some(USERS[i]);
            }
        }
        None
    }
}


pub fn get_user_name(id: u32) -> &'static str {
    unsafe {
        for i in 0..USER_COUNT {
            if USERS[i].id == id && USERS[i].is_active {
                let name = core::str::from_utf8(&USERS[i].name).unwrap_or("Unknown");
                return name.trim_end_matches('\0');
            }
        }
        "Unknown"
    }
}


pub fn user_exists(id: u32) -> bool {
    unsafe {
        for i in 0..USER_COUNT {
            if USERS[i].id == id && USERS[i].is_active {
                return true;
            }
        }
        false
    }
}


pub fn is_main_user(id: u32) -> bool {
    id == 4456
}


pub fn create_temp_user() -> Result<u32, &'static str> {
    unsafe {

        for i in 0..USER_COUNT {
            if USERS[i].is_temp && USERS[i].is_active {
                return Err("Temporary user already exists!");
            }
        }
        
        if USER_COUNT >= MAX_USERS {
            return Err("Maximum users reached!");
        }
        

        let temp_id = 4356;
        let temp_name = b"Temp User";
        let name_len = temp_name.len().min(MAX_NAME_LEN - 1);
        
        USERS[USER_COUNT].id = temp_id;
        USERS[USER_COUNT].name[..name_len].copy_from_slice(&temp_name[..name_len]);
        USERS[USER_COUNT].name[name_len] = 0;
        USERS[USER_COUNT].is_temp = true;
        USERS[USER_COUNT].is_active = true;
        
        USER_COUNT += 1;
        
        Ok(temp_id)
    }
}


pub fn remove_temp_user() -> Result<(), &'static str> {
    unsafe {
        for i in 0..USER_COUNT {
            if USERS[i].is_temp && USERS[i].is_active {

                USERS[i].is_active = false;

                if CURRENT_USER_ID == USERS[i].id {
                    CURRENT_USER_ID = 4456;
                    IS_LOGGED_IN = false;
                }
                return Ok(());
            }
        }
        Err("No temporary user found!")
    }
}

у
pub fn login(user_id: u32) -> bool {
    unsafe {
        if user_exists(user_id) {
            CURRENT_USER_ID = user_id;
            IS_LOGGED_IN = true;
            true
        } else {
            false
        }
    }
}


pub fn logout() -> bool {
    unsafe {
        if IS_LOGGED_IN {
            IS_LOGGED_IN = false;
            CURRENT_USER_ID = 0;
            true
        } else {
            false
        }
    }
}


pub fn get_current_user() -> Option<User> {
    unsafe {
        if IS_LOGGED_IN {
            get_user_by_id(CURRENT_USER_ID)
        } else {
            None
        }
    }
}


pub fn list_users() -> &'static str {
    unsafe {
        static mut BUFFER: [u8; 512] = [0; 512];
        let mut pos = 0;
        
        let header = b"=== Users ===\n";
        for b in header {
            if pos < 511 { BUFFER[pos] = *b; pos += 1; }
        }
        
        for i in 0..USER_COUNT {
            if USERS[i].is_active {
                let name = core::str::from_utf8(&USERS[i].name).unwrap_or("Unknown");
                let trimmed = name.trim_end_matches('\0');
                let status = if USERS[i].is_temp { " (TEMP)" } else { " (MAIN)" };
                let active = if CURRENT_USER_ID == USERS[i].id && IS_LOGGED_IN { " [ACTIVE]" } else { "" };
                
                
                let id_str = num_to_str(USERS[i].id);
                
                
                for b in id_str.as_bytes() {
                    if pos < 511 { BUFFER[pos] = *b; pos += 1; }
                }
                BUFFER[pos] = b':'; pos += 1;
                BUFFER[pos] = b' '; pos += 1;
                BUFFER[pos] = b'-'; pos += 1;
                BUFFER[pos] = b' '; pos += 1;
                
                
                for b in trimmed.as_bytes() {
                    if pos < 511 { BUFFER[pos] = *b; pos += 1; }
                }
                
               
                for b in status.as_bytes() {
                    if pos < 511 { BUFFER[pos] = *b; pos += 1; }
                }
                
                
                for b in active.as_bytes() {
                    if pos < 511 { BUFFER[pos] = *b; pos += 1; }
                }
                
                BUFFER[pos] = b'\n'; pos += 1;
            }
        }
        
        BUFFER[pos] = 0;
        core::str::from_utf8(&BUFFER[..pos]).unwrap_or("")
    }
}


fn num_to_str(mut num: u32) -> &'static str {
    static mut BUF: [u8; 12] = [0; 12];
    unsafe {
        if num == 0 {
            BUF[0] = b'0';
            BUF[1] = 0;
            return "0";
        }
        let mut digits = [0u8; 12];
        let mut i = 0;
        while num > 0 {
            digits[i] = b'0' + (num % 10) as u8;
            num /= 10;
            i += 1;
        }
        let mut j = 0;
        while i > 0 {
            i -= 1;
            BUF[j] = digits[i];
            j += 1;
        }
        BUF[j] = 0;
        core::str::from_utf8(&BUF[..j]).unwrap_or("0")
    }
}


pub fn is_current_user_main() -> bool {
    unsafe {
        IS_LOGGED_IN && CURRENT_USER_ID == 4456
    }
}