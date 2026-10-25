use crate::graphics::{Graphics, Color};
use core::fmt::Write;

const PI: f32 = core::f32::consts::PI;
const TAU: f32 = 2.0 * PI;


const ZW: usize = 640;
const ZH: usize = 480;
static mut ZBUF: [f32; ZW * ZH] = [0.0; ZW * ZH];

const CAM_D: f32 = 40.0;

const LIGHT: [f32; 3] = [0.4082, 0.4082, 0.8165];



fn sin_approx(x: f32) -> f32 {
    let mut x = x % TAU;
    if x > PI { x -= TAU; } else if x < -PI { x += TAU; }
    
    if x > PI / 2.0 { x = PI - x; } else if x < -PI / 2.0 { x = -PI - x; }
    let x2 = x * x;
    x * (1.0 - x2 / 6.0 * (1.0 - x2 / 20.0 * (1.0 - x2 / 42.0 * (1.0 - x2 / 72.0))))
}

fn cos_approx(x: f32) -> f32 {
    sin_approx(x + PI / 2.0)
}

fn sqrt_approx(x: f32) -> f32 {
    if x <= 0.0 { return 0.0; }
    let mut y = f32::from_bits((x.to_bits() >> 1) + 0x1fbd_1df5);
    y = 0.5 * (y + x / y);
    y = 0.5 * (y + x / y);
    y
}



pub struct GlApp {
    pub running: bool,
    pub angle_x: f32,     
    pub angle_y: f32,     
    pub angle_z: f32,
    pub time: f32,
    pub gear_angle: f32,  
}

impl GlApp {
    pub const fn new() -> Self {
        GlApp {
            running: false,
            angle_x: 20.0,
            angle_y: 30.0,
            angle_z: 0.0,
            time: 0.0,
            gear_angle: 0.0,
        }
    }

    pub fn run(&mut self) -> &'static str {
        self.running = true;
        "GL started! Gears spinning..."
    }

    pub fn stop(&mut self) -> &'static str {
        self.running = false;
        "GL stopped!"
    }
}



struct View {
    rsx: f32, rcx: f32, 
    rsy: f32, rcy: f32, 
    mid_x: f32, mid_y: f32,
    focal: f32,
    w: usize, h: usize, 
    ox: usize, oy: usize, 
}

impl View {
    
    fn to_view(&self, p: [f32; 3]) -> [f32; 3] {
        let x1 = p[0] * self.rcy + p[2] * self.rsy;
        let z1 = -p[0] * self.rsy + p[2] * self.rcy;
        let y2 = p[1] * self.rcx - z1 * self.rsx;
        let z2 = p[1] * self.rsx + z1 * self.rcx;
        [x1, y2, z2]
    }

    
    fn project(&self, p: [f32; 3]) -> Option<(f32, f32, f32)> {
        let zc = CAM_D - p[2];
        if zc < 1.0 { return None; }
        let inv = 1.0 / zc;
        Some((
            self.mid_x + p[0] * self.focal * inv,
            self.mid_y - p[1] * self.focal * inv,
            inv,
        ))
    }
}

#[inline]
fn edge(ax: f32, ay: f32, bx: f32, by: f32, px: f32, py: f32) -> f32 {
    (bx - ax) * (py - ay) - (by - ay) * (px - ax)
}


fn draw_tri(gfx: &mut Graphics, zbuf: &mut [f32; ZW * ZH], v: &View,
            a: [f32; 3], b: [f32; 3], c: [f32; 3], color: u32) {
    let (pa, pb, pc) = match (v.project(a), v.project(b), v.project(c)) {
        (Some(x), Some(y), Some(z)) => (x, y, z),
        _ => return,
    };

    let min_x = pa.0.min(pb.0).min(pc.0);
    let max_x = pa.0.max(pb.0).max(pc.0);
    let min_y = pa.1.min(pb.1).min(pc.1);
    let max_y = pa.1.max(pb.1).max(pc.1);

    if max_x < 0.0 || max_y < 0.0 { return; }
    let x0 = (min_x as i32).max(0);
    let y0 = (min_y as i32).max(0);
    let x1 = (max_x as i32 + 1).min(v.w as i32 - 1);
    let y1 = (max_y as i32 + 1).min(v.h as i32 - 1);
    if x0 > x1 || y0 > y1 { return; }

    let area = edge(pa.0, pa.1, pb.0, pb.1, pc.0, pc.1);
    if area > -0.0001 && area < 0.0001 { return; }
    let inv_area = 1.0 / area;

    for y in y0..=y1 {
        let py = y as f32 + 0.5;
        for x in x0..=x1 {
            let px = x as f32 + 0.5;
            let w0 = edge(pb.0, pb.1, pc.0, pc.1, px, py) * inv_area;
            let w1 = edge(pc.0, pc.1, pa.0, pa.1, px, py) * inv_area;
            let w2 = edge(pa.0, pa.1, pb.0, pb.1, px, py) * inv_area;
            if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                let iz = w0 * pa.2 + w1 * pb.2 + w2 * pc.2; 
                let idx = y as usize * ZW + x as usize;
                if iz > zbuf[idx] {
                    zbuf[idx] = iz;
                    gfx.put_pixel(v.ox + x as usize, v.oy + y as usize, color);
                }
            }
        }
    }
}


fn draw_quad(gfx: &mut Graphics, zbuf: &mut [f32; ZW * ZH], v: &View,
             p: [[f32; 3]; 4], base: (f32, f32, f32)) {
    let d1 = [p[2][0] - p[0][0], p[2][1] - p[0][1], p[2][2] - p[0][2]];
    let d2 = [p[3][0] - p[1][0], p[3][1] - p[1][1], p[3][2] - p[1][2]];
    let nx = d1[1] * d2[2] - d1[2] * d2[1];
    let ny = d1[2] * d2[0] - d1[0] * d2[2];
    let nz = d1[0] * d2[1] - d1[1] * d2[0];
    let len = sqrt_approx(nx * nx + ny * ny + nz * nz);
    if len < 1e-6 { return; }

    let mut dot = (nx * LIGHT[0] + ny * LIGHT[1] + nz * LIGHT[2]) / len;
    if dot < 0.0 { dot = -dot; }
    let k = 0.25 + 0.75 * dot;

    let color = Color::rgb(
        (base.0 * k * 255.0) as u8,
        (base.1 * k * 255.0) as u8,
        (base.2 * k * 255.0) as u8,
    ).to_u32();

    draw_tri(gfx, zbuf, v, p[0], p[1], p[2], color);
    draw_tri(gfx, zbuf, v, p[0], p[2], p[3], color);
}


fn draw_gear(gfx: &mut Graphics, zbuf: &mut [f32; ZW * ZH], v: &View,
             inner: f32, outer: f32, width: f32, teeth: usize, tooth_depth: f32,
             pos: (f32, f32), rot_deg: f32, rgb: (f32, f32, f32)) {
    let r0 = inner;
    let r1 = outer - tooth_depth / 2.0;
    let r2 = outer + tooth_depth / 2.0;
    let hw = width * 0.5;
    let da = TAU / teeth as f32 / 4.0;

    let rot = rot_deg * PI / 180.0;
    let (rs, rc) = (sin_approx(rot), cos_approx(rot));

    for i in 0..teeth {
        let a = i as f32 * da * 4.0;
        
        let mut cs = [(0.0f32, 0.0f32); 5];
        for k in 0..5 {
            let ang = a + da * k as f32;
            cs[k] = (cos_approx(ang), sin_approx(ang));
        }

        
        let p = |r: f32, k: usize, z: f32| -> [f32; 3] {
            let lx = r * cs[k].0;
            let ly = r * cs[k].1;
            let x = lx * rc - ly * rs + pos.0;
            let y = lx * rs + ly * rc + pos.1;
            v.to_view([x, y, z])
        };

        
        for &z in [hw, -hw].iter() {
            draw_quad(gfx, zbuf, v, [p(r0, 0, z), p(r1, 0, z), p(r1, 3, z), p(r0, 3, z)], rgb);
            draw_quad(gfx, zbuf, v, [p(r0, 3, z), p(r1, 3, z), p(r1, 4, z), p(r0, 4, z)], rgb);
            draw_quad(gfx, zbuf, v, [p(r1, 0, z), p(r2, 1, z), p(r2, 2, z), p(r1, 3, z)], rgb);
        }

        
        draw_quad(gfx, zbuf, v, [p(r1, 0, hw), p(r2, 1, hw), p(r2, 1, -hw), p(r1, 0, -hw)], rgb);
        draw_quad(gfx, zbuf, v, [p(r2, 1, hw), p(r2, 2, hw), p(r2, 2, -hw), p(r2, 1, -hw)], rgb);
        draw_quad(gfx, zbuf, v, [p(r2, 2, hw), p(r1, 3, hw), p(r1, 3, -hw), p(r2, 2, -hw)], rgb);
        draw_quad(gfx, zbuf, v, [p(r1, 3, hw), p(r1, 4, hw), p(r1, 4, -hw), p(r1, 3, -hw)], rgb);

        
        draw_quad(gfx, zbuf, v, [p(r0, 0, hw), p(r0, 4, hw), p(r0, 4, -hw), p(r0, 0, -hw)], rgb);
    }
}



struct TextBuf {
    buf: [u8; 48],
    len: usize,
}

impl Write for TextBuf {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for b in s.bytes() {
            if self.len < self.buf.len() {
                self.buf[self.len] = b;
                self.len += 1;
            }
        }
        Ok(())
    }
}


pub fn draw_gl_scene(gfx: &mut Graphics, win_x: usize, win_y: usize, win_w: usize, win_h: usize, app: &mut GlApp) {
    
    for y in 0..win_h {
        let brightness = 10 + (y * 50 / win_h);
        let b = brightness as u8;
        let color = Color::rgb(b, (b as f32 * 0.6) as u8, (b as f32 + 20.0) as u8).to_u32();
        gfx.fill_rect(win_x, win_y + y, win_w, 1, color);
    }

    
    for i in 0..40 {
        let sx = win_x + (i * 53 + 19) % win_w;
        let sy = win_y + (i * 37 + 13) % win_h;
        let bright = 200 + (i * 11) % 55;
        gfx.put_pixel(sx, sy, Color::rgb(bright as u8, bright as u8, 255).to_u32());
    }

    
    app.gear_angle += 2.0;
    if app.gear_angle >= 360.0 { app.gear_angle -= 360.0; }
    app.time += 0.02;

    
    let rw = win_w.min(ZW).min(gfx.width.saturating_sub(win_x));
    let rh = win_h.min(ZH).min(gfx.height.saturating_sub(win_y));

    if rw > 0 && rh > 0 {
        let vx = app.angle_x * PI / 180.0;
        let vy = app.angle_y * PI / 180.0;
        let smaller = if rw < rh { rw } else { rh } as f32;

        let view = View {
            rsx: sin_approx(vx), rcx: cos_approx(vx),
            rsy: sin_approx(vy), rcy: cos_approx(vy),
            mid_x: rw as f32 / 2.0,
            mid_y: rh as f32 / 2.0,
            focal: (smaller / 17.0) * CAM_D,
            w: rw, h: rh,
            ox: win_x, oy: win_y,
        };

        
        let zbuf = unsafe { &mut *core::ptr::addr_of_mut!(ZBUF) };
        for y in 0..rh {
            let row = y * ZW;
            for x in 0..rw {
                zbuf[row + x] = 0.0;
            }
        }

        let ang = app.gear_angle;
        
        draw_gear(gfx, zbuf, &view, 1.0, 4.0, 1.0, 20, 0.7, (-2.0, -2.0), ang,
                  (0.8, 0.1, 0.0));
        draw_gear(gfx, zbuf, &view, 0.5, 2.0, 2.0, 10, 0.7, (4.1, -2.0), -2.0 * ang - 9.0,
                  (0.0, 0.8, 0.2));
        draw_gear(gfx, zbuf, &view, 1.3, 2.0, 0.5, 10, 0.7, (-2.1, 4.2), -2.0 * ang - 25.0,
                  (0.2, 0.2, 1.0));
    }

    
    let bg = Color::rgb(10, 10, 20).to_u32();
    gfx.draw_text(win_x + 10, win_y + 10, "", Color::rgb(255, 200, 100).to_u32(), bg);
    gfx.draw_text(win_x + 10, win_y + 28, "", Color::rgb(150, 150, 200).to_u32(), bg);

    let mut t = TextBuf { buf: [0; 48], len: 0 };
    let _ = write!(t, "Angle: {}", app.gear_angle as i32);
    let info = core::str::from_utf8(&t.buf[..t.len]).unwrap_or("Angle: ?");
    gfx.draw_text(win_x + 10, win_y + win_h - 20, info, Color::rgb(100, 100, 140).to_u32(), bg);
}



static mut APP: GlApp = GlApp::new();

pub fn get_gl_app() -> &'static mut GlApp {
    unsafe { &mut *core::ptr::addr_of_mut!(APP) }
}

pub fn gl_start() -> &'static str {
    get_gl_app().run()
}

pub fn gl_stop() -> &'static str {
    get_gl_app().stop()
}