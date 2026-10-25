

use crate::graphics::{Graphics, Color};


#[derive(Clone, Copy)]
pub struct Process {
    pub pid: usize,
    pub name: [u8; 32],
    pub name_len: usize,
    pub memory: usize,
    pub cpu: usize,
    pub state: ProcessState,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ProcessState {
    Running,
    Sleeping,
    Zombie,
    Stopped,
}

impl ProcessState {
    pub fn to_str(&self) -> &'static str {
        match self {
            ProcessState::Running => "Running",
            ProcessState::Sleeping => "Sleeping",
            ProcessState::Zombie => "Zombie",
            ProcessState::Stopped => "Stopped",
        }
    }
    
    pub fn color(&self) -> u32 {
        match self {
            ProcessState::Running => Color::rgb(0, 200, 0).to_u32(),
            ProcessState::Sleeping => Color::rgb(200, 200, 0).to_u32(),
            ProcessState::Zombie => Color::rgb(200, 0, 0).to_u32(),
            ProcessState::Stopped => Color::rgb(200, 100, 0).to_u32(),
        }
    }
}


static mut PROCESSES: [Process; 32] = [Process {
    pid: 0,
    name: [0; 32],
    name_len: 0,
    memory: 0,
    cpu: 0,
    state: ProcessState::Stopped,
}; 32];
static mut PROCESS_COUNT: usize = 0;
static mut PROCESS_TICK: usize = 0;
static mut CPU_LOAD: usize = 5;


fn num_to_str(num: usize) -> &'static str {
    static mut BUF: [u8; 10] = [0; 10];
    unsafe {
        let mut temp = num;
        let mut digits = [0u8; 10];
        let mut i = 0;
        if temp == 0 {
            BUF[0] = b'0';
            BUF[1] = 0;
            return "0";
        }
        while temp > 0 {
            digits[i] = b'0' + (temp % 10) as u8;
            temp /= 10;
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


pub fn init_task_manager() {
    unsafe {
        PROCESS_COUNT = 0;
        
        add_process("kernel", 2048, 2, ProcessState::Running);
        add_process("window_manager", 4096, 5, ProcessState::Running);
        add_process("display_server", 2048, 3, ProcessState::Running);
        add_process("input_driver", 512, 1, ProcessState::Running);
        add_process("ata_driver", 1024, 0, ProcessState::Sleeping);
        add_process("filesystem", 1536, 1, ProcessState::Sleeping);
        add_process("gui_shell", 2048, 4, ProcessState::Running);
        add_process("terminal", 512, 2, ProcessState::Running);
        add_process("task_manager", 1024, 0, ProcessState::Sleeping);
        add_process("mouse_driver", 256, 1, ProcessState::Running);
        add_process("vga_driver", 1024, 2, ProcessState::Running);
        add_process("keyboard_driver", 256, 1, ProcessState::Running);
    }
}


pub fn add_process(name: &str, memory: usize, cpu: usize, state: ProcessState) {
    unsafe {
        if PROCESS_COUNT >= 32 { return; }
        
        let bytes = name.as_bytes();
        let len = bytes.len().min(31);
        PROCESSES[PROCESS_COUNT].pid = PROCESS_COUNT + 1000;
        PROCESSES[PROCESS_COUNT].name[..len].copy_from_slice(&bytes[..len]);
        PROCESSES[PROCESS_COUNT].name[len] = 0;
        PROCESSES[PROCESS_COUNT].name_len = len;
        PROCESSES[PROCESS_COUNT].memory = memory;
        PROCESSES[PROCESS_COUNT].cpu = cpu;
        PROCESSES[PROCESS_COUNT].state = state;
        PROCESS_COUNT += 1;
    }
}


pub fn update_process_stats() {
    unsafe {
        PROCESS_TICK += 1;
        

        if PROCESS_TICK % 3 == 0 {
            let change = (PROCESS_TICK * 7) % 11;
            if change < 5 && CPU_LOAD > 3 {
                CPU_LOAD -= 1;
            } else if CPU_LOAD < 15 {
                CPU_LOAD += 1;
            }
        }
        

        if PROCESS_TICK % 2 == 0 {
            for i in 0..PROCESS_COUNT {

                if PROCESSES[i].cpu > 0 {
                    let change = (PROCESS_TICK * (i + 1) * 3) % 5;
                    if change < 2 && PROCESSES[i].cpu > 1 {
                        PROCESSES[i].cpu -= 1;
                    } else if PROCESSES[i].cpu < 8 {
                        PROCESSES[i].cpu += 1;
                    }
                }
                

                if PROCESS_TICK % 20 == 0 && i % 4 == 0 {
                    match PROCESSES[i].state {
                        ProcessState::Running => {
                            if PROCESS_TICK % 40 == 0 {
                                PROCESSES[i].state = ProcessState::Sleeping;
                            }
                        }
                        ProcessState::Sleeping => {
                            if PROCESS_TICK % 30 == 0 {
                                PROCESSES[i].state = ProcessState::Running;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}


pub fn draw_task_manager(gfx: &mut Graphics, win_x: usize, win_y: usize, win_w: usize, win_h: usize) {
    let bg_color = Color::rgb(30, 30, 35).to_u32();
    gfx.fill_rect(win_x, win_y, win_w, win_h, bg_color);
    

    gfx.fill_rect(win_x, win_y, win_w, 30, Color::rgb(50, 50, 55).to_u32());
    gfx.draw_text(win_x + 10, win_y + 8, "Task Manager", Color::rgb(200, 200, 220).to_u32(), Color::rgb(50, 50, 55).to_u32());
    
    unsafe {

        gfx.draw_text(win_x + win_w - 150, win_y + 8, "Processes: ", Color::rgb(150, 150, 180).to_u32(), Color::rgb(50, 50, 55).to_u32());
        let count_str = num_to_str(PROCESS_COUNT);
        gfx.draw_text(win_x + win_w - 60, win_y + 8, count_str, Color::rgb(200, 200, 220).to_u32(), Color::rgb(50, 50, 55).to_u32());
    }
    

    unsafe {
        let mut total_mem = 0;
        let mut total_cpu = 0;
        let mut running_count = 0;
        
        for i in 0..PROCESS_COUNT {
            total_mem += PROCESSES[i].memory;
            total_cpu += PROCESSES[i].cpu;
            if PROCESSES[i].state == ProcessState::Running {
                running_count += 1;
            }
        }
        
        let stats_y = win_y + 40;
        gfx.fill_rect(win_x + 10, stats_y, win_w - 20, 22, Color::rgb(40, 40, 45).to_u32());
        
        let mem_str = num_to_str(total_mem);
        gfx.draw_text(win_x + 20, stats_y + 4, "Memory: ", Color::rgb(150, 150, 180).to_u32(), Color::rgb(40, 40, 45).to_u32());
        gfx.draw_text(win_x + 90, stats_y + 4, mem_str, Color::rgb(200, 200, 220).to_u32(), Color::rgb(40, 40, 45).to_u32());
        gfx.draw_text(win_x + 160, stats_y + 4, "KB", Color::rgb(150, 150, 180).to_u32(), Color::rgb(40, 40, 45).to_u32());
        
        let cpu_str = num_to_str(CPU_LOAD);
        gfx.draw_text(win_x + 210, stats_y + 4, "CPU: ", Color::rgb(150, 150, 180).to_u32(), Color::rgb(40, 40, 45).to_u32());
        gfx.draw_text(win_x + 260, stats_y + 4, cpu_str, Color::rgb(200, 200, 220).to_u32(), Color::rgb(40, 40, 45).to_u32());
        gfx.draw_text(win_x + 300, stats_y + 4, "%", Color::rgb(150, 150, 180).to_u32(), Color::rgb(40, 40, 45).to_u32());
        
        let run_str = num_to_str(running_count);
        gfx.draw_text(win_x + 340, stats_y + 4, "Running: ", Color::rgb(150, 150, 180).to_u32(), Color::rgb(40, 40, 45).to_u32());
        gfx.draw_text(win_x + 410, stats_y + 4, run_str, Color::rgb(200, 200, 220).to_u32(), Color::rgb(40, 40, 45).to_u32());
    }
    

    let header_y = win_y + 72;
    gfx.fill_rect(win_x + 10, header_y, win_w - 20, 22, Color::rgb(60, 60, 70).to_u32());
    
    let headers = ["PID", "Process", "Memory", "CPU", "State"];
    let col_widths = [40, 150, 80, 50, 80];
    let mut x_pos = win_x + 20;
    
    for (i, header) in headers.iter().enumerate() {
        gfx.draw_text(x_pos, header_y + 5, header, Color::rgb(180, 180, 200).to_u32(), Color::rgb(60, 60, 70).to_u32());
        x_pos += col_widths[i];
    }
    

    unsafe {
        let mut y_pos = header_y + 28;
        let row_height = 22;
        let max_rows = (win_h - 120) / row_height;
        
        for i in 0..PROCESS_COUNT.min(max_rows) {
            let process = &PROCESSES[i];
            
            let row_color = if i % 2 == 0 {
                Color::rgb(40, 40, 45).to_u32()
            } else {
                Color::rgb(35, 35, 40).to_u32()
            };
            gfx.fill_rect(win_x + 10, y_pos, win_w - 20, row_height, row_color);
            

            let pid_str = num_to_str(process.pid);
            gfx.draw_text(win_x + 20, y_pos + 4, pid_str, Color::rgb(150, 150, 180).to_u32(), row_color);
            

            let name = core::str::from_utf8(&process.name[..process.name_len]).unwrap_or("Unknown");
            gfx.draw_text(win_x + 65, y_pos + 4, name, Color::rgb(200, 200, 220).to_u32(), row_color);
            

            let mem_str = num_to_str(process.memory);
            gfx.draw_text(win_x + 220, y_pos + 4, mem_str, Color::rgb(200, 200, 220).to_u32(), row_color);
            

            let cpu_str = num_to_str(process.cpu);
            let cpu_color = if process.cpu > 10 {
                Color::rgb(255, 100, 100).to_u32()
            } else if process.cpu > 5 {
                Color::rgb(255, 200, 100).to_u32()
            } else {
                Color::rgb(100, 200, 100).to_u32()
            };
            gfx.draw_text(win_x + 305, y_pos + 4, cpu_str, cpu_color, row_color);
            

            let state_str = process.state.to_str();
            let state_color = process.state.color();
            gfx.draw_text(win_x + 360, y_pos + 4, state_str, state_color, row_color);
            
            y_pos += row_height;
        }
    }
}


pub fn task_start() -> &'static str {
    unsafe {
        if PROCESS_COUNT == 0 {
            init_task_manager();
        }
        "Task Manager ready!"
    }
}

pub fn task_stop() -> &'static str {
    "Task Manager stopped!"
}