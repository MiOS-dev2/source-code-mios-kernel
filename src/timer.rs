use core::arch::x86_64::_rdtsc;

static mut TSC_PER_MS: u64 = 0;
static mut TSC_START: u64 = 0;

fn inb(port: u16) -> u8 {
    let v: u8;
    unsafe { core::arch::asm!("in al, dx", in("dx") port, out("al") v, options(nomem, nostack)) }
    v
}
fn outb(port: u16, val: u8) {
    unsafe { core::arch::asm!("out dx, al", in("dx") port, in("al") val, options(nomem, nostack)) }
}

pub fn rdtsc() -> u64 { unsafe { _rdtsc() } }

pub fn init() {
    unsafe {
        if TSC_PER_MS != 0 { return; }
        let mut best = u64::MAX;
        for _ in 0..3 {
            let count: u16 = (1193182u32 * 20 / 1000) as u16;
            let v = inb(0x61);
            outb(0x61, (v & 0xFD) | 0x01);
            outb(0x43, 0xB0);
            outb(0x42, (count & 0xFF) as u8);
            outb(0x42, (count >> 8) as u8);
            let v = inb(0x61) & 0xFE;
            outb(0x61, v);
            outb(0x61, v | 1);
            let t0 = rdtsc();
            let mut guard: u64 = 0;
            while inb(0x61) & 0x20 == 0 {
                guard += 1;
                if guard > 200_000_000 { break; }
            }
            let t1 = rdtsc();
            let d = (t1 - t0) / 20;
            if d > 0 && d < best { best = d; }
        }
        if best == u64::MAX || best < 1000 { best = 1_000_000; }
        TSC_PER_MS = best;
        TSC_START = rdtsc();
    }
}

pub fn millis() -> u64 {
    unsafe {
        if TSC_PER_MS == 0 { init(); }
        (rdtsc() - TSC_START) / TSC_PER_MS
    }
}

pub fn sleep_ms(ms: u64) {
    let end = millis() + ms;
    while millis() < end { core::hint::spin_loop(); }
}
