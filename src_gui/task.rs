
use core::arch::asm;

pub const STACK_SIZE: usize = 4096;
pub const MAX_TASKS: usize = 8;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TaskContext {
    pub rip: u64,
    pub rsp: u64,
    pub rbx: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rbp: u64,
}

impl TaskContext {
    pub const fn new() -> Self {
        Self { rip: 0, rsp: 0, rbx: 0, r12: 0, r13: 0, r14: 0, r15: 0, rbp: 0 }
    }
}

pub struct Task {
    pub id: usize,
    pub name: &'static str,
    pub stack: [u8; STACK_SIZE],
    pub context: TaskContext,
}

impl Task {
    pub fn new(id: usize, name: &'static str, entry: extern "C" fn()) -> Self {
        let mut task = Self {
            id, name,
            stack: [0; STACK_SIZE],
            context: TaskContext::new(),
        };
        
        let stack_top = task.stack.as_ptr() as u64 + STACK_SIZE as u64;
        let rsp = (stack_top & !0xF) - 8;
        
        unsafe {
            let ptr = rsp as *mut u64;
            *ptr = entry as u64;
        }
        
        task.context.rsp = rsp;
        task.context.rip = entry as u64;
        task
    }
}

pub struct Scheduler {
    tasks: [Option<Task>; MAX_TASKS],
    pub current: usize,
    count: usize,
}

impl Scheduler {
    pub const fn new() -> Self {
        const NONE: Option<Task> = None;
        Self { tasks: [NONE; MAX_TASKS], current: 0, count: 0 }
    }
    
    pub fn spawn(&mut self, name: &'static str, entry: extern "C" fn()) -> usize {
        let id = self.count;
        if id < MAX_TASKS {
            self.tasks[id] = Some(Task::new(id, name, entry));
            self.count += 1;
        }
        id
    }
    
    pub fn next(&self) -> usize {
        if self.count == 0 { return 0; }
        (self.current + 1) % self.count
    }
}

// Переключение контекста
#[unsafe(no_mangle)]
pub unsafe extern "C" fn switch_to(old: *mut TaskContext, new: *const TaskContext) {
    asm!(

        "mov [rdi + 0x00], rbx",
        "mov [rdi + 0x08], r12",
        "mov [rdi + 0x10], r13",
        "mov [rdi + 0x18], r14",
        "mov [rdi + 0x20], r15",
        "mov [rdi + 0x28], rbp",
        // Сохраняем RIP (адрес возврата)
        "mov rax, [rsp]",
        "mov [rdi + 0x00], rax",
        // Сохраняем RSP (после возврата)
        "lea rax, [rsp + 8]",
        "mov [rdi + 0x08], rax",
        

        "mov rax, [rsi + 0x00]",  // rip
        "mov rbx, [rsi + 0x08]",
        "mov r12, [rsi + 0x10]",
        "mov r13, [rsi + 0x18]",
        "mov r14, [rsi + 0x20]",
        "mov r15, [rsi + 0x28]",
        "mov rbp, [rsi + 0x30]",
        
        // Устанавливаем новый стек и прыгаем
        "mov rsp, [rsi + 0x08]",  // rsp
        "jmp [rsi + 0x00]",       // rip
        
        options(noreturn)
    );
}

pub fn do_yield(sched: &mut Scheduler) {
    let next_id = sched.next();
    if next_id == sched.current || sched.count <= 1 {
        return;
    }
    

    let old_ctx_ptr: *mut TaskContext;
    let new_ctx_ptr: *const TaskContext;
    
    unsafe {
        let old_task = sched.tasks[sched.current].as_mut().unwrap();
        old_ctx_ptr = &mut old_task.context as *mut TaskContext;
        
        let new_task = sched.tasks[next_id].as_ref().unwrap();
        new_ctx_ptr = &new_task.context as *const TaskContext;
    }
    
    sched.current = next_id;
    
    unsafe {
        switch_to(old_ctx_ptr, new_ctx_ptr);
    }
}
