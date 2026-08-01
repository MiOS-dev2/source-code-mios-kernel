
use core::arch::naked_asm;
use core::sync::atomic::{AtomicBool, Ordering};

const MOUSE_DATA: u16 = 0x60;
const MOUSE_STATUS: u16 = 0x64;
const MOUSE_CMD: u16 = 0x64;

const PIC1_CMD: u16 = 0x20;
const PIC1_DATA: u16 = 0x21;
const PIC2_CMD: u16 = 0xA0;
const PIC2_DATA: u16 = 0xA1;
const PIC_EOI: u8 = 0x20;

static MOUSE_AVAILABLE: AtomicBool = AtomicBool::new(false);
static mut MOUSE_BUF: [u8; 4] = [0; 4]; 
static mut MOUSE_BUF_IDX: usize = 0;


static KEYBOARD_AVAILABLE: AtomicBool = AtomicBool::new(false);
static mut KEYBOARD_BUF: u8 = 0;

#[derive(Debug, Clone, Copy)]
pub struct MousePacket {
    pub dx: i32,
    pub dy: i32,
    pub left: bool,
    pub right: bool,
    pub middle: bool,
}

fn inb(port: u16) -> u8 {
    let v: u8;
    unsafe { core::arch::asm!("in al, dx", in("dx") port, out("al") v) }
    v
}

fn outb(port: u16, val: u8) {
    unsafe { core::arch::asm!("out dx, al", in("dx") port, in("al") val) }
}

fn mouse_wait(read: bool) {
    let timeout = 100_000;
    if read {
        for _ in 0..timeout {
            if inb(MOUSE_STATUS) & 1 == 1 { return; }
        }
    } else {
        for _ in 0..timeout {
            if inb(MOUSE_STATUS) & 2 == 0 { return; }
        }
    }
}

fn mouse_write(cmd: u8) {
    mouse_wait(false);
    outb(MOUSE_CMD, 0xD4);
    mouse_wait(false);
    outb(MOUSE_DATA, cmd);
}

fn mouse_read() -> u8 {
    mouse_wait(true);
    inb(MOUSE_DATA)
}

pub fn init_ps2_mouse() {
    
    mouse_wait(false);
    outb(MOUSE_CMD, 0xA8);

    
    mouse_wait(false);
    outb(MOUSE_CMD, 0x20);
    let mut config = mouse_read();

    
    config |= 0x03;  
    config &= !0x20; 
    config &= !0x10; 


    mouse_wait(false);
    outb(MOUSE_CMD, 0x60);
    mouse_wait(false);
    outb(MOUSE_DATA, config);


    mouse_write(0xFF);
    let ack = mouse_read();
    
    if ack == 0xFA {
        mouse_read(); // BAT (Basic Assurance Test)
        let id = mouse_read(); // Device ID
        

        if id != 0 {
            mouse_write(0xF3); // Set sample rate
            mouse_read();
            mouse_write(200);
            mouse_read();
            
            mouse_write(0xF3);
            mouse_read();
            mouse_write(100);
            mouse_read();
            
            mouse_write(0xF3);
            mouse_read();
            mouse_write(80);
            mouse_read();
            
            mouse_write(0xF2); 
            mouse_read();
            mouse_read(); 
        }
    }


    mouse_write(0xF4);
    mouse_read(); // ACK
}


#[unsafe(naked)]
extern "x86-interrupt" fn mouse_handler(_sf: &crate::idt::InterruptStackFrame, _error_code: u64) {
    naked_asm!(
        "push rax",
        "push rcx",
        "push rdx",
        "push rdi",
        "push rsi",
        "push r8",
        "push r9",
        "push r10",
        "push r11",

        // Читаем данные мыши
        "mov dx, 0x64",
        "in al, dx",
        "test al, 0x20",  // Проверяем, что это данные от мыши
        "jz 2f",
        
        "mov dx, 0x60",
        "in al, dx",

        // Сохраняем байт в буфер
        "lea rdi, [rip + {buf}]",
        "movzx rcx, byte ptr [rip + {idx}]",
        
        // Проверяем первый байт пакета (должен иметь бит 3 = 1)
        "cmp cl, 0",
        "jne 0f",
        "test al, 0x08",  // Бит выравнивания
        "jz 2f",          // Пропускаем если нет бита выравнивания
        "0:",
        
        "mov [rdi + rcx], al",
        "add cl, 1",
        "mov byte ptr [rip + {idx}], cl",

        // Если получили 3 байта (или 4 для мыши с колесом)
        "cmp cl, 3",
        "je 1f",
        // Для 4-байтовых пакетов
        "test byte ptr [rdi], 0x08",
        "jz 1f",
        "cmp cl, 4",
        "jne 2f",
        
        "1:",
        "mov byte ptr [rip + {flag}], 1",
        "mov byte ptr [rip + {idx}], 0",
        
        "2:",
        // Отправляем EOI
        "mov al, 0x20",
        "out 0xA0, al",  // Сначала ведомому
        "out 0x20, al",  // Потом ведущему

        "pop r11",
        "pop r10",
        "pop r9",
        "pop r8",
        "pop rsi",
        "pop rdi",
        "pop rdx",
        "pop rcx",
        "pop rax",
        "iretq",

        buf = sym MOUSE_BUF,
        idx = sym MOUSE_BUF_IDX,
        flag = sym MOUSE_AVAILABLE,
    );
}

/// Обработчик прерывания клавиатуры (IRQ1 → вектор 33)
#[unsafe(naked)]
extern "x86-interrupt" fn keyboard_handler(_sf: &crate::idt::InterruptStackFrame, _error_code: u64) {
    naked_asm!(
        "push rax",
        "push rdx",
        "mov dx, 0x60",
        "in al, dx",
        "mov byte ptr [rip + {keybuf}], al",
        "mov byte ptr [rip + {keyflag}], 1",
        "mov al, 0x20",
        "out 0x20, al",
        "pop rdx",
        "pop rax",
        "iretq",

        keybuf = sym KEYBOARD_BUF,
        keyflag = sym KEYBOARD_AVAILABLE,
    );
}

pub unsafe fn init_mouse_interrupts() {
    // Ремап PIC
    outb(PIC1_CMD, 0x11);
    outb(PIC1_DATA, 0x20);  // векторы ведущего: 0x20-0x27
    outb(PIC1_DATA, 0x04);  // ведомый на IRQ2
    outb(PIC1_DATA, 0x01);

    outb(PIC2_CMD, 0x11);
    outb(PIC2_DATA, 0x28);  // векторы ведомого: 0x28-0x2F
    outb(PIC2_DATA, 0x02);
    outb(PIC2_DATA, 0x01);

    // Маски: разрешаем IRQ1 (клавиатура) и IRQ2 (каскад)
    outb(PIC1_DATA, 0xF9);  // 11111001
    // Ведомый: разрешаем IRQ12 (мышь)
    outb(PIC2_DATA, 0xEF);  // 11101111

    crate::idt::set_handler(0x21, keyboard_handler); // IRQ1 -> 0x21
    crate::idt::set_handler(0x2C, mouse_handler);    // IRQ12 -> 0x2C

    core::arch::asm!("sti");
}

pub fn get_mouse_packet() -> Option<MousePacket> {
    if MOUSE_AVAILABLE.swap(false, Ordering::Relaxed) {
        let bytes = unsafe { MOUSE_BUF };
        
        // Проверяем выравнивание пакета
        if bytes[0] & 0x08 == 0 {
            return None; // Пакет не выровнен
        }
        
        // Проверяем переполнение
        if bytes[0] & 0xC0 != 0 {
            return None; // Переполнение X или Y
        }
        
        // Извлекаем биты знака
        let negative_x = bytes[0] & 0x10 != 0;
        let negative_y = bytes[0] & 0x20 != 0;
        
        // Конвертируем перемещения с учетом знака
        let mut dx = bytes[1] as i32;
        let mut dy = bytes[2] as i32;
        
        if negative_x {
            dx = (bytes[1] as i8) as i32;
        }
        if negative_y {
            dy = (bytes[2] as i8) as i32;
        }
        
        // Инвертируем Y (в мыши Y идет вниз, нам нужно вверх)
        dy = -dy;
        
        // Ограничиваем значения для предотвращения "убегания"
        dx = dx.clamp(-127, 127);
        dy = dy.clamp(-127, 127);
        
        Some(MousePacket {
            dx,
            dy,
            left: bytes[0] & 0x01 != 0,
            right: bytes[0] & 0x02 != 0,
            middle: bytes[0] & 0x04 != 0,
        })
    } else {
        None
    }
}

// Получить скан-код клавиши, если доступен
pub fn get_key() -> Option<u8> {
    if KEYBOARD_AVAILABLE.swap(false, Ordering::Relaxed) {
        Some(unsafe { KEYBOARD_BUF })
    } else {
        None
    }
}