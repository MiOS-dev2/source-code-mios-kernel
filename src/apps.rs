
fn f64_to_str(num: f64, buffer: &mut [u8]) -> &str {
    let int_part = num as i64;
    let frac_part = ((num - int_part as f64).abs() * 1000.0) as i64;
    
    let mut temp = int_part;
    let mut len = 0;
    
    if num < 0.0 && int_part == 0 {
        buffer[0] = b'-';
        len = 1;
    }
    if temp < 0 {
        temp = -temp;
    }
    
    let mut digits = [0u8; 20];
    let mut i = 0;
    if temp == 0 {
        buffer[len] = b'0';
        len += 1;
    } else {
        while temp > 0 {
            digits[i] = b'0' + (temp % 10) as u8;
            temp /= 10;
            i += 1;
        }
        while i > 0 {
            i -= 1;
            buffer[len] = digits[i];
            len += 1;
        }
    }
    
    if frac_part > 0 {
        buffer[len] = b'.';
        len += 1;
        let mut frac = frac_part;
        let mut frac_digits = [0u8; 3];
        let mut j = 0;
        while frac > 0 && j < 3 {
            frac_digits[j] = b'0' + (frac % 10) as u8;
            frac /= 10;
            j += 1;
        }
        while j > 0 {
            j -= 1;
            buffer[len] = frac_digits[j];
            len += 1;
        }
    }
    
    core::str::from_utf8(&buffer[..len]).unwrap_or("")
}


fn parse_f64(s: &str) -> Option<f64> {
    let bytes = s.as_bytes();
    let mut result = 0.0;
    let mut sign = 1.0;
    let mut i = 0;
    let mut frac = false;
    let mut frac_div = 10.0;
    
    if i < bytes.len() && bytes[i] == b'-' {
        sign = -1.0;
        i += 1;
    }
    
    while i < bytes.len() {
        let c = bytes[i];
        if c == b'.' {
            frac = true;
            i += 1;
            continue;
        }
        if c < b'0' || c > b'9' {
            return None;
        }
        let digit = (c - b'0') as f64;
        if frac {
            result += digit / frac_div;
            frac_div *= 10.0;
        } else {
            result = result * 10.0 + digit;
        }
        i += 1;
    }
    
    Some(result * sign)
}


pub fn calculator(expr: &str) -> &'static str {
    static mut RESULT: [u8; 128] = [0; 128];
    
    let trimmed = expr.trim();
    if trimmed.is_empty() {
        return "Usage: calc <number> <operator> <number>";
    }
    
    let mut parts: [&str; 3] = ["", "", ""];
    let mut part_count = 0;
    for word in trimmed.split_whitespace() {
        if part_count < 3 {
            parts[part_count] = word;
            part_count += 1;
        } else {
            return "Error: Too many arguments!";
        }
    }
    
    if part_count != 3 {
        return "Usage: calc <num1> <op> <num2>";
    }
    
    let num1: f64 = match parse_f64(parts[0]) {
        Some(n) => n,
        None => return "Error: Invalid first number",
    };
    let num2: f64 = match parse_f64(parts[2]) {
        Some(n) => n,
        None => return "Error: Invalid second number",
    };
    
    let result = match parts[1] {
        "+" => num1 + num2,
        "-" => num1 - num2,
        "*" | "×" => num1 * num2,
        "/" | "÷" => {
            if num2 == 0.0 {
                return "Error: Division by zero!";
            }
            num1 / num2
        }
        "^" => {
            let mut res = 1.0;
            let exp = num2 as i32;
            if exp < 0 {
                return "Error: Negative exponent not supported";
            }
            for _ in 0..exp {
                res *= num1;
            }
            res
        }
        "%" => num1 % num2,
        _ => return "Error: Unknown operator",
    };
    
    unsafe {
        let result_str = f64_to_str(result, &mut RESULT);
        let msg = "Result: ";
        let mut pos = 0;
        for &b in msg.as_bytes() {
            if pos < 127 { RESULT[pos] = b; pos += 1; }
        }
        for &b in result_str.as_bytes() {
            if pos < 127 { RESULT[pos] = b; pos += 1; }
        }
        RESULT[pos] = 0;
        core::str::from_utf8(&RESULT[..pos]).unwrap_or("")
    }
}


pub fn echo(args: &str) -> &'static str {
    static mut RESULT: [u8; 256] = [0; 256];
    if args.is_empty() {
        return "Usage: echo <text>";
    }
    unsafe {
        let bytes = args.as_bytes();
        let len = bytes.len().min(255);
        RESULT[..len].copy_from_slice(&bytes[..len]);
        RESULT[len] = 0;
        core::str::from_utf8(&RESULT[..len]).unwrap_or("")
    }
}


pub fn strlen(args: &str) -> &'static str {
    static mut RESULT: [u8; 32] = [0; 32];
    if args.is_empty() {
        return "Usage: strlen <text>";
    }
    unsafe {
        let len = args.len();
        let mut temp = len;
        let mut i = 0;
        if temp == 0 {
            RESULT[0] = b'0';
            RESULT[1] = 0;
            return "0";
        }
        let mut digits = [0u8; 10];
        let mut j = 0;
        while temp > 0 {
            digits[j] = b'0' + (temp % 10) as u8;
            temp /= 10;
            j += 1;
        }
        while j > 0 {
            j -= 1;
            RESULT[i] = digits[j];
            i += 1;
        }
        RESULT[i] = 0;
        core::str::from_utf8(&RESULT[..i]).unwrap_or("0")
    }
}


pub fn reverse(args: &str) -> &'static str {
    static mut RESULT: [u8; 256] = [0; 256];
    if args.is_empty() {
        return "Usage: reverse <text>";
    }
    unsafe {
        let bytes = args.as_bytes();
        let len = bytes.len().min(255);
        for i in 0..len {
            RESULT[i] = bytes[len - 1 - i];
        }
        RESULT[len] = 0;
        core::str::from_utf8(&RESULT[..len]).unwrap_or("")
    }
}


pub fn to_upper(args: &str) -> &'static str {
    static mut RESULT: [u8; 256] = [0; 256];
    if args.is_empty() {
        return "Usage: to-upper <text>";
    }
    unsafe {
        let bytes = args.as_bytes();
        let len = bytes.len().min(255);
        for i in 0..len {
            let c = bytes[i];
            if c >= b'a' && c <= b'z' {
                RESULT[i] = c - 32;
            } else {
                RESULT[i] = c;
            }
        }
        RESULT[len] = 0;
        core::str::from_utf8(&RESULT[..len]).unwrap_or("")
    }
}


pub fn to_lower(args: &str) -> &'static str {
    static mut RESULT: [u8; 256] = [0; 256];
    if args.is_empty() {
        return "Usage: to-lower <text>";
    }
    unsafe {
        let bytes = args.as_bytes();
        let len = bytes.len().min(255);
        for i in 0..len {
            let c = bytes[i];
            if c >= b'A' && c <= b'Z' {
                RESULT[i] = c + 32;
            } else {
                RESULT[i] = c;
            }
        }
        RESULT[len] = 0;
        core::str::from_utf8(&RESULT[..len]).unwrap_or("")
    }
}


pub fn greet(args: &str) -> &'static str {
    static mut RESULT: [u8; 256] = [0; 256];
    
    let name = if args.is_empty() { "World" } else { args };
    
    unsafe {
        let msg = "Hello, ";
        let mut pos = 0;
        for &b in msg.as_bytes() {
            if pos < 255 { RESULT[pos] = b; pos += 1; }
        }
        for &b in name.as_bytes() {
            if pos < 255 { RESULT[pos] = b; pos += 1; }
        }
        if pos < 255 { RESULT[pos] = b'!'; pos += 1; }
        RESULT[pos] = 0;
        core::str::from_utf8(&RESULT[..pos]).unwrap_or("")
    }
}


pub fn list_apps() -> &'static str {
    "Доступные приложения:\n\
     \x20 calc/calculator - Калькулятор\n\
     \x20 echo           - Выводит текст\n\
     \x20 strlen/len     - Длина строки\n\
     \x20 reverse/rev    - Перевернуть строку\n\
     \x20 to-upper/upper - В верхний регистр\n\
     \x20 to-lower/lower - В нижний регистр\n\
     \x20 greet          - Приветствие\n\
     \x20 cre-tool       - Конструктор приложений\n\
     \n\
     Использование: call <app_name> [args...]"
}




#[derive(Clone, Copy)]
pub struct GuiElement {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
    pub kind: u8, 
    pub text: [u8; 64],
    pub text_len: usize,
    pub visible: bool,
}


pub fn cre_tool(_args: &str) -> &'static str {
    static mut RESULT: [u8; 256] = [0; 256];
    let msg = "Cre Tool opened! Use GUI to build apps.";
    unsafe {
        let bytes = msg.as_bytes();
        let len = bytes.len().min(255);
        RESULT[..len].copy_from_slice(&bytes[..len]);
        RESULT[len] = 0;
        core::str::from_utf8(&RESULT[..len]).unwrap_or("")
    }
}