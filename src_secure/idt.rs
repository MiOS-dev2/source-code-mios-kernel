
use core::arch::naked_asm;
use core::mem::size_of;

const IDT_ENTRIES: usize = 256;

#[repr(C)]
pub struct InterruptStackFrame {
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
struct IdtEntry {
    offset_low: u16,
    selector: u16,
    ist: u8,
    flags: u8,
    offset_mid: u16,
    offset_high: u32,
    reserved: u32,
}

impl IdtEntry {
    const fn new() -> Self {
        Self {
            offset_low: 0,
            selector: 0,
            ist: 0,
            flags: 0,
            offset_mid: 0,
            offset_high: 0,
            reserved: 0,
        }
    }

    pub fn set_handler_addr(&mut self, handler_addr: u64) {
        self.offset_low = (handler_addr & 0xFFFF) as u16;
        self.selector = 0x08;
        self.ist = 0;
        self.flags = 0x8E;
        self.offset_mid = ((handler_addr >> 16) & 0xFFFF) as u16;
        self.offset_high = (handler_addr >> 32) as u32;
        self.reserved = 0;
    }
}

#[repr(C, packed)]
struct IdtPtr {
    limit: u16,
    base: u64,
}

static mut IDT: [IdtEntry; IDT_ENTRIES] = [IdtEntry::new(); IDT_ENTRIES];


macro_rules! gen_stub {
    ($name:ident, $num:expr) => {
        #[unsafe(naked)]
        extern "x86-interrupt" fn $name(_sf: &InterruptStackFrame) {
            naked_asm!(
                "push 0",
                "push {num}",
                "jmp isr_common",
                num = const $num
            );
        }
    };
}


macro_rules! gen_stub_error {
    ($name:ident, $num:expr) => {
        #[unsafe(naked)]
        extern "x86-interrupt" fn $name(_sf: &InterruptStackFrame, _error_code: u64) {
            naked_asm!(
                "push {num}",
                "jmp isr_common",
                num = const $num
            );
        }
    };
}

gen_stub!(isr0, 0);
gen_stub!(isr1, 1);
gen_stub!(isr2, 2);
gen_stub!(isr3, 3);
gen_stub!(isr4, 4);
gen_stub!(isr5, 5);
gen_stub!(isr6, 6);
gen_stub!(isr7, 7);
gen_stub!(isr9, 9);
gen_stub!(isr15, 15);
gen_stub!(isr16, 16);
gen_stub!(isr18, 18);
gen_stub!(isr19, 19);
gen_stub!(isr20, 20);
gen_stub!(isr21, 21);
gen_stub!(isr22, 22);
gen_stub!(isr23, 23);
gen_stub!(isr24, 24);
gen_stub!(isr25, 25);
gen_stub!(isr26, 26);
gen_stub!(isr27, 27);
gen_stub!(isr28, 28);
gen_stub!(isr29, 29);
gen_stub!(isr30, 30);
gen_stub!(isr31, 31);

gen_stub_error!(isr8, 8);
gen_stub_error!(isr10, 10);
gen_stub_error!(isr11, 11);
gen_stub_error!(isr12, 12);
gen_stub_error!(isr13, 13);
gen_stub_error!(isr14, 14);
gen_stub_error!(isr17, 17);

#[unsafe(no_mangle)]
#[unsafe(naked)]
pub extern "x86-interrupt" fn isr_common(_sf: &InterruptStackFrame, _error_code: u64) {
    naked_asm!(
        "cli",
        "1: hlt",
        "jmp 1b",
    );
}

pub unsafe fn set_handler(vector: usize, handler: extern "x86-interrupt" fn(&InterruptStackFrame, u64)) {
    if vector < IDT_ENTRIES {
        IDT[vector].set_handler_addr(handler as u64);
    }
}

pub unsafe fn init_idt() {
    IDT[0].set_handler_addr(isr0 as u64);
    IDT[1].set_handler_addr(isr1 as u64);
    IDT[2].set_handler_addr(isr2 as u64);
    IDT[3].set_handler_addr(isr3 as u64);
    IDT[4].set_handler_addr(isr4 as u64);
    IDT[5].set_handler_addr(isr5 as u64);
    IDT[6].set_handler_addr(isr6 as u64);
    IDT[7].set_handler_addr(isr7 as u64);
    IDT[8].set_handler_addr(isr8 as u64);
    IDT[9].set_handler_addr(isr9 as u64);
    IDT[10].set_handler_addr(isr10 as u64);
    IDT[11].set_handler_addr(isr11 as u64);
    IDT[12].set_handler_addr(isr12 as u64);
    IDT[13].set_handler_addr(isr13 as u64);
    IDT[14].set_handler_addr(isr14 as u64);
    IDT[15].set_handler_addr(isr15 as u64);
    IDT[16].set_handler_addr(isr16 as u64);
    IDT[17].set_handler_addr(isr17 as u64);
    IDT[18].set_handler_addr(isr18 as u64);
    IDT[19].set_handler_addr(isr19 as u64);
    IDT[20].set_handler_addr(isr20 as u64);
    IDT[21].set_handler_addr(isr21 as u64);
    IDT[22].set_handler_addr(isr22 as u64);
    IDT[23].set_handler_addr(isr23 as u64);
    IDT[24].set_handler_addr(isr24 as u64);
    IDT[25].set_handler_addr(isr25 as u64);
    IDT[26].set_handler_addr(isr26 as u64);
    IDT[27].set_handler_addr(isr27 as u64);
    IDT[28].set_handler_addr(isr28 as u64);
    IDT[29].set_handler_addr(isr29 as u64);
    IDT[30].set_handler_addr(isr30 as u64);
    IDT[31].set_handler_addr(isr31 as u64);

    let idt_ptr = IdtPtr {
        limit: (size_of::<[IdtEntry; IDT_ENTRIES]>() - 1) as u16,
        base: IDT.as_ptr() as u64,
    };

    core::arch::asm!("lidt [{}]", in(reg) &idt_ptr);
}
