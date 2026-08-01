// src/gl.rs
// 3D куб с ортографической проекцией

use crate::graphics::{Graphics, Color};

// ============ 3D МАТЕМАТИКА ============

fn sin_approx(x: f32) -> f32 {
    let mut x = x % (2.0 * core::f32::consts::PI);
    if x > core::f32::consts::PI {
        x -= 2.0 * core::f32::consts::PI;
    } else if x < -core::f32::consts::PI {
        x += 2.0 * core::f32::consts::PI;
    }
    let x2 = x * x;
    let x3 = x2 * x;
    let x5 = x3 * x2;
    let x7 = x5 * x2;
    x - x3 / 6.0 + x5 / 120.0 - x7 / 5040.0
}

fn cos_approx(x: f32) -> f32 {
    let mut x = x % (2.0 * core::f32::consts::PI);
    if x > core::f32::consts::PI {
        x -= 2.0 * core::f32::consts::PI;
    } else if x < -core::f32::consts::PI {
        x += 2.0 * core::f32::consts::PI;
    }
    let x2 = x * x;
    let x4 = x2 * x2;
    let x6 = x4 * x2;
    1.0 - x2 / 2.0 + x4 / 24.0 - x6 / 720.0
}

pub struct GlApp {
    pub running: bool,
    pub angle_x: f32,
    pub angle_y: f32,
    pub angle_z: f32,
    pub time: f32,
}

impl GlApp {
    pub fn new() -> Self {
        GlApp {
            running: false,
            angle_x: 0.0,
            angle_y: 0.0,
            angle_z: 0.0,
            time: 0.0,
        }
    }

    pub fn run(&mut self) -> &'static str {
        self.running = true;
        "GL started! 3D cube spinning..."
    }

    pub fn stop(&mut self) -> &'static str {
        self.running = false;
        "GL stopped!"
    }
}

// ============ РИСОВАНИЕ 3D КУБА ============

pub fn draw_3d_cube(gfx: &mut Graphics, cx: usize, cy: usize, size: f32, angle_x: f32, angle_y: f32, angle_z: f32, color: u32) {
    // Вершины куба в 3D пространстве
    let half = size / 2.0;
    let vertices: [(f32, f32, f32); 8] = [
        (-half, -half, -half), // 0 - левый нижний задний
        ( half, -half, -half), // 1 - правый нижний задний
        ( half,  half, -half), // 2 - правый верхний задний
        (-half,  half, -half), // 3 - левый верхний задний
        (-half, -half,  half), // 4 - левый нижний передний
        ( half, -half,  half), // 5 - правый нижний передний
        ( half,  half,  half), // 6 - правый верхний передний
        (-half,  half,  half), // 7 - левый верхний передний
    ];

    // Рёбра куба
    let edges: [(usize, usize); 12] = [
        (0, 1), (1, 2), (2, 3), (3, 0), // Задняя грань
        (4, 5), (5, 6), (6, 7), (7, 4), // Передняя грань
        (0, 4), (1, 5), (2, 6), (3, 7), // Соединения
    ];

    // Матрица вращения X
    let cos_x = cos_approx(angle_x);
    let sin_x = sin_approx(angle_x);
    
    // Матрица вращения Y
    let cos_y = cos_approx(angle_y);
    let sin_y = sin_approx(angle_y);
    
    // Матрица вращения Z
    let cos_z = cos_approx(angle_z);
    let sin_z = sin_approx(angle_z);

    // Проецируем вершины на 2D (ортографическая проекция - без перспективы!)
    let mut projected = [(0.0, 0.0); 8];
    
    for i in 0..8 {
        let (x, y, z) = vertices[i];
        
        // Вращение по X
        let y1 = y * cos_x - z * sin_x;
        let z1 = y * sin_x + z * cos_x;
        
        // Вращение по Y
        let x1 = x * cos_y + z1 * sin_y;
        let z2 = -x * sin_y + z1 * cos_y;
        
        // Вращение по Z
        let x2 = x1 * cos_z - y1 * sin_z;
        let y2 = x1 * sin_z + y1 * cos_z;
        
        // Ортографическая проекция - просто берём X и Y
        let px = cx as f32 + x2;
        let py = cy as f32 + y2;
        
        projected[i] = (px, py);
    }

    // Рисуем рёбра куба (яркие)
    for (i, j) in edges.iter() {
        let (x1, y1) = projected[*i];
        let (x2, y2) = projected[*j];
        draw_line(gfx, x1 as usize, y1 as usize, x2 as usize, y2 as usize, color);
    }

    // Рисуем вершины (белые точки)
    for (px, py) in projected.iter() {
        let px_i = *px as i32;
        let py_i = *py as i32;
        for dy in -2..=2 {
            for dx in -2..=2 {
                let x = px_i + dx;
                let y = py_i + dy;
                if x >= 0 && x < gfx.width as i32 && y >= 0 && y < gfx.height as i32 {
                    gfx.put_pixel(x as usize, y as usize, Color::WHITE.to_u32());
                }
            }
        }
    }
}

fn draw_line(gfx: &mut Graphics, x1: usize, y1: usize, x2: usize, y2: usize, color: u32) {
    let mut x = x1 as i32;
    let mut y = y1 as i32;
    let dx = (x2 as i32 - x1 as i32).abs();
    let dy = -(y2 as i32 - y1 as i32).abs();
    let sx = if x1 < x2 { 1 } else { -1 };
    let sy = if y1 < y2 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        if x >= 0 && x < gfx.width as i32 && y >= 0 && y < gfx.height as i32 {
            gfx.put_pixel(x as usize, y as usize, color);
        }
        if x == x2 as i32 && y == y2 as i32 { break; }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

// ============ СЦЕНА ============

pub fn draw_gl_scene(gfx: &mut Graphics, win_x: usize, win_y: usize, win_w: usize, win_h: usize, app: &mut GlApp) {
    // Чистый фон
    for y in 0..win_h {
        let brightness = 10 + (y * 50 / win_h);
        let b = brightness as u8;
        let color = Color::rgb(b, (b as f32 * 0.6) as u8, (b as f32 + 20.0) as u8).to_u32();
        gfx.fill_rect(win_x, win_y + y, win_w, 1, color);
    }
    
    // Звёзды
    for i in 0..40 {
        let sx = win_x + (i * 53 + 19) % win_w;
        let sy = win_y + (i * 37 + 13) % win_h;
        let bright = 200 + (i * 11) % 55;
        gfx.put_pixel(sx, sy, Color::rgb(bright as u8, bright as u8, 255).to_u32());
    }
    
    let center_x = win_x + win_w / 2;
    let center_y = win_y + win_h / 2;
    
    // Вращение куба
    app.angle_x += 0.02;
    app.angle_y += 0.03;
    app.angle_z += 0.01;
    app.time += 0.02;
    
    // Размер куба
    let cube_size = 60.0;
    
    // Цвет куба (меняется)
    let r = ((sin_approx(app.time * 0.3) * 0.5 + 0.5) * 200.0 + 55.0) as u8;
    let g = ((sin_approx(app.time * 0.4 + 2.0) * 0.5 + 0.5) * 200.0 + 55.0) as u8;
    let b = ((sin_approx(app.time * 0.5 + 4.0) * 0.5 + 0.5) * 200.0 + 55.0) as u8;
    let color = Color::rgb(r, g, b).to_u32();
    
    // Рисуем куб
    draw_3d_cube(gfx, center_x, center_y, cube_size, app.angle_x, app.angle_y, app.angle_z, color);
    
    // Информация
    gfx.draw_text(win_x + 10, win_y + 10, "3D CUBE", Color::rgb(255, 200, 100).to_u32(), Color::rgb(10, 10, 20).to_u32());
    gfx.draw_text(win_x + 10, win_y + 28, "Rotating on all axes", Color::rgb(150, 150, 200).to_u32(), Color::rgb(10, 10, 20).to_u32());
    
    // Показываем углы вращения
    let mut buf = [0u8; 64];
    let mut pos = 0;
    let prefix = b"Angles: X=";
    for &b in prefix {
        if pos < buf.len() - 1 { buf[pos] = b; pos += 1; }
    }
    
    let x_deg = ((app.angle_x * 180.0 / 3.14159) as i32) % 360;
    let y_deg = ((app.angle_y * 180.0 / 3.14159) as i32) % 360;
    let z_deg = ((app.angle_z * 180.0 / 3.14159) as i32) % 360;
    
    let x_str = num_to_str(if x_deg < 0 { -x_deg } else { x_deg } as usize);
    for b in x_str.bytes() {
        if pos < buf.len() - 1 { buf[pos] = b; pos += 1; }
    }
    
    if pos < buf.len() - 1 { buf[pos] = b' '; pos += 1; }
    if pos < buf.len() - 1 { buf[pos] = b'Y'; pos += 1; }
    if pos < buf.len() - 1 { buf[pos] = b'='; pos += 1; }
    
    let y_str = num_to_str(if y_deg < 0 { -y_deg } else { y_deg } as usize);
    for b in y_str.bytes() {
        if pos < buf.len() - 1 { buf[pos] = b; pos += 1; }
    }
    
    if pos < buf.len() - 1 { buf[pos] = b' '; pos += 1; }
    if pos < buf.len() - 1 { buf[pos] = b'Z'; pos += 1; }
    if pos < buf.len() - 1 { buf[pos] = b'='; pos += 1; }
    
    let z_str = num_to_str(if z_deg < 0 { -z_deg } else { z_deg } as usize);
    for b in z_str.bytes() {
        if pos < buf.len() - 1 { buf[pos] = b; pos += 1; }
    }
    
    buf[pos] = 0;
    let info = core::str::from_utf8(&buf[..pos]).unwrap_or("Angles: X=0 Y=0 Z=0");
    gfx.draw_text(win_x + 10, win_y + win_h - 20, info, Color::rgb(100, 100, 140).to_u32(), Color::rgb(10, 10, 20).to_u32());
}

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

// ============ ЭКСПОРТ ============

pub fn gl_start() -> &'static str {
    static mut APP: Option<GlApp> = None;
    unsafe {
        if APP.is_none() {
            APP = Some(GlApp::new());
        }
        if let Some(ref mut app) = APP {
            app.run()
        } else {
            "GL error!"
        }
    }
}

pub fn gl_stop() -> &'static str {
    unsafe {
        static mut APP: Option<GlApp> = None;
        if let Some(ref mut app) = APP {
            app.stop()
        } else {
            "GL not running!"
        }
    }
}

pub fn get_gl_app() -> &'static mut GlApp {
    unsafe {
        static mut APP: Option<GlApp> = None;
        if APP.is_none() {
            APP = Some(GlApp::new());
        }
        APP.as_mut().unwrap()
    }
}