use linked_list_allocator::LockedHeap;

const HEAP_SIZE: usize = 160 * 1024 * 1024;

#[repr(align(4096))]
struct HeapMem([u8; HEAP_SIZE]);

static mut HEAP_MEM: HeapMem = HeapMem([0; HEAP_SIZE]);

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

static mut INITED: bool = false;

pub fn init() {
    unsafe {
        if INITED { return; }
        INITED = true;
        let start = core::ptr::addr_of_mut!(HEAP_MEM) as *mut u8;
        ALLOCATOR.lock().init(start, HEAP_SIZE);
    }
}

pub fn used() -> usize { ALLOCATOR.lock().used() }
pub fn free() -> usize { ALLOCATOR.lock().free() }
