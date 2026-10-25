use crate::graphics::{Graphics, Color};
use crate::wm::WindowManager;
use crate::wm::WindowState;
 
const CODE_MAX: usize = 4096;
const STR_MAX: usize = 64;
const NAME_MAX: usize = 16;
const MAX_GLOBALS: usize = 40;
const MAX_LOCALS: usize = 40;
const MAX_FUNCS: usize = 16;
const MAX_PARAMS: usize = 4;
const MAX_WIDGETS: usize = 40;
const MAX_DECL: usize = 16;
const MAX_ARGS: usize = 6;
const MAX_CALL_DEPTH: usize = 10;
const MAX_EXPR_DEPTH: usize = 48;
const STEP_LIMIT: usize = 100000;
const CON_LINES: usize = 48;
const CON_COLS: usize = 80;
const CONSOLE_H: usize = 104;
 
const OP_OR: u8 = 1;
const OP_AND: u8 = 2;
const OP_EQ: u8 = 3;
const OP_NE: u8 = 4;
const OP_LT: u8 = 5;
const OP_GT: u8 = 6;
const OP_LE: u8 = 7;
const OP_GE: u8 = 8;
const OP_IN: u8 = 9;
const OP_ADD: u8 = 10;
const OP_SUB: u8 = 11;
const OP_MUL: u8 = 12;
const OP_DIV: u8 = 13;
const OP_MOD: u8 = 14;
const OP_POW: u8 = 15;
 
type Res<T> = Result<T, &'static str>;
 
 

 
#[derive(Clone, Copy)]
struct PStr {
    b: [u8; STR_MAX],
    len: usize,
}
 
impl PStr {
    const fn new() -> PStr {
        PStr { b: [0; STR_MAX], len: 0 }
    }
    fn push(&mut self, c: u8) {
        if self.len < STR_MAX {
            self.b[self.len] = c;
            self.len += 1;
        }
    }
    fn push_bytes(&mut self, s: &[u8]) {
        let mut i = 0;
        while i < s.len() {
            self.push(s[i]);
            i += 1;
        }
    }
    fn as_bytes(&self) -> &[u8] {
        &self.b[..self.len]
    }
    fn from_bytes(s: &[u8]) -> PStr {
        let mut p = PStr::new();
        p.push_bytes(s);
        p
    }
}
 
#[derive(Clone, Copy)]
enum Val {
    None,
    Bool(bool),
    Int(i64),
    Str(PStr),
    Obj(usize),
    Func(usize),
    Lambda(usize, usize),
}
 
#[derive(Clone, Copy)]
enum Flow {
    Normal,
    Break,
    Continue,
    Return(Val),
}
 
#[derive(Clone, Copy)]
struct Var {
    name: [u8; NAME_MAX],
    nlen: usize,
    val: Val,
}
 
const EMPTY_VAR: Var = Var { name: [0; NAME_MAX], nlen: 0, val: Val::None };
 
#[derive(Clone, Copy)]
struct Func {
    name: [u8; NAME_MAX],
    nlen: usize,
    params: [[u8; NAME_MAX]; MAX_PARAMS],
    plen: [usize; MAX_PARAMS],
    pcount: usize,
    body_s: usize,
    body_e: usize,
    used: bool,
}
 
const EMPTY_FUNC: Func = Func {
    name: [0; NAME_MAX],
    nlen: 0,
    params: [[0; NAME_MAX]; MAX_PARAMS],
    plen: [0; MAX_PARAMS],
    pcount: 0,
    body_s: 0,
    body_e: 0,
    used: false,
};
 
#[derive(Clone, Copy)]
struct Widget {
    used: bool,
    alive: bool,
    kind: u8,
    text: PStr,
    win: usize,
    layout: u8,
    side_left: bool,
    gx: usize,
    gy: usize,
    cmd: Val,
    checked: bool,
}
 
const EMPTY_WIDGET: Widget = Widget {
    used: false,
    alive: false,
    kind: 0,
    text: PStr::new(),
    win: 0,
    layout: 0,
    side_left: false,
    gx: 0,
    gy: 0,
    cmd: Val::None,
    checked: false,
};
 
#[derive(Clone, Copy)]
struct Rect {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    ok: bool,
}
 
const EMPTY_RECT: Rect = Rect { x: 0, y: 0, w: 0, h: 0, ok: false };
 
struct Cur {
    i: usize,
    e: usize,
    skip: bool,
}
 
struct Args {
    pos: [Val; MAX_ARGS],
    pc: usize,
    kn: [(usize, usize); MAX_ARGS],
    kv: [Val; MAX_ARGS],
    kc: usize,
}
 
impl Args {
    fn new() -> Args {
        Args {
            pos: [Val::None; MAX_ARGS],
            pc: 0,
            kn: [(0, 0); MAX_ARGS],
            kv: [Val::None; MAX_ARGS],
            kc: 0,
        }
    }
}
 
struct Body {
    s: usize,
    e: usize,
    after: usize,
}
 
 
 
static mut CODE_BUFFER: [u8; CODE_MAX] = [0; CODE_MAX];
static mut CODE_LEN: usize = 0;
static mut CURSOR_POS: usize = 0;
static mut TICK_COUNTER: usize = 0;
static mut PREVIEW_RUNNING: bool = false;
static mut PREVIEW_WINDOW: usize = 0;
 
static mut PROG: [u8; CODE_MAX] = [0; CODE_MAX];
static mut PROG_LEN: usize = 0;
 
static mut GLOBALS: [Var; MAX_GLOBALS] = [EMPTY_VAR; MAX_GLOBALS];
static mut GLOBAL_COUNT: usize = 0;
static mut LOCALS: [Var; MAX_LOCALS] = [EMPTY_VAR; MAX_LOCALS];
static mut LOCAL_COUNT: usize = 0;
static mut LOCAL_BASE: usize = 0;
static mut CALL_DEPTH: usize = 0;
static mut DECL: [[u8; NAME_MAX]; MAX_DECL] = [[0; NAME_MAX]; MAX_DECL];
static mut DECL_LEN: [usize; MAX_DECL] = [0; MAX_DECL];
static mut DECL_BASE: usize = 0;
static mut DECL_COUNT: usize = 0;
 
static mut FUNCS: [Func; MAX_FUNCS] = [EMPTY_FUNC; MAX_FUNCS];
static mut FUNC_COUNT: usize = 0;
static mut WIDGETS: [Widget; MAX_WIDGETS] = [EMPTY_WIDGET; MAX_WIDGETS];
static mut WIDGET_COUNT: usize = 0;
static mut ACTIVE_WIN: usize = usize::MAX;
static mut ROOT_WIN: usize = usize::MAX;
static mut TK_USED: bool = false;
static mut RUN_STARTED: bool = false;
static mut FILE_RUN: bool = false;
 
static mut STEPS: usize = 0;
static mut EXPR_DEPTH: usize = 0;
static mut CUR_POS: usize = 0;
static mut ERR_NS: usize = 0;
static mut ERR_NE: usize = 0;
 
static mut CON: [[u8; CON_COLS]; CON_LINES] = [[0; CON_COLS]; CON_LINES];
static mut CON_LEN: [usize; CON_LINES] = [0; CON_LINES];
static mut CON_COUNT: usize = 1;
 
 
 
fn draw_line_internal(gfx: &mut Graphics, x1: usize, y1: usize, x2: usize, y2: usize, color: u32) {
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
        if e2 >= dy { err += dy; x += sx; }
        if e2 <= dx { err += dx; y += sy; }
    }
}
 
fn text_of(p: &PStr) -> &str {
    core::str::from_utf8(&p.b[..p.len]).unwrap_or("")
}
 
 
 
fn con_clear() {
    unsafe {
        CON_COUNT = 1;
        CON_LEN[0] = 0;
    }
}
 
fn con_newline() {
    unsafe {
        if CON_COUNT < CON_LINES {
            CON_COUNT += 1;
            CON_LEN[CON_COUNT - 1] = 0;
        } else {
            let mut k = 1;
            while k < CON_LINES {
                CON[k - 1] = CON[k];
                CON_LEN[k - 1] = CON_LEN[k];
                k += 1;
            }
            CON_LEN[CON_LINES - 1] = 0;
        }
    }
}
 
fn con_put_byte(b: u8) {
    unsafe {
        if b == b'\n' {
            con_newline();
            return;
        }
        let ch = if b == b'\t' { b' ' } else if b < 0x20 || b >= 0x7F { b'?' } else { b };
        if CON_LEN[CON_COUNT - 1] >= CON_COLS {
            con_newline();
        }
        let li = CON_COUNT - 1;
        CON[li][CON_LEN[li]] = ch;
        CON_LEN[li] += 1;
    }
}
 
fn con_put_bytes(s: &[u8]) {
    let mut i = 0;
    while i < s.len() {
        con_put_byte(s[i]);
        i += 1;
    }
}
 
fn con_put_str(s: &str) {
    con_put_bytes(s.as_bytes());
}
 
 
 
fn pb(i: usize) -> u8 {
    unsafe {
        if i < PROG_LEN { PROG[i] } else { 0 }
    }
}
 
fn is_digit(c: u8) -> bool {
    c >= b'0' && c <= b'9'
}
 
fn is_ident_start(c: u8) -> bool {
    (c >= b'a' && c <= b'z') || (c >= b'A' && c <= b'Z') || c == b'_'
}
 
fn is_ident_char(c: u8) -> bool {
    is_ident_start(c) || is_digit(c)
}
 
fn skip_sp(mut i: usize, end: usize) -> usize {
    while i < end && (pb(i) == b' ' || pb(i) == b'\t') {
        i += 1;
    }
    i
}
 
fn line_end(pos: usize) -> usize {
    unsafe {
        let mut i = pos;
        while i < PROG_LEN && PROG[i] != b'\n' {
            i += 1;
        }
        i
    }
}
 
fn next_line(le: usize) -> usize {
    unsafe {
        if le < PROG_LEN { le + 1 } else { PROG_LEN }
    }
}
 
fn indent_of(ls: usize, le: usize) -> (usize, usize) {
    let mut i = ls;
    let mut n = 0;
    while i < le {
        let c = pb(i);
        if c == b' ' { n += 1; }
        else if c == b'\t' { n += 4; }
        else { break; }
        i += 1;
    }
    (n, i)
}
 
fn code_end(ls: usize, le: usize) -> usize {
    let mut i = ls;
    let mut q: u8 = 0;
    let mut e = le;
    while i < le {
        let c = pb(i);
        if q != 0 {
            if c == b'\\' { i += 2; continue; }
            if c == q { q = 0; }
        } else if c == b'"' || c == b'\'' {
            q = c;
        } else if c == b'#' {
            e = i;
            break;
        }
        i += 1;
    }
    if e > le { e = le; }
    while e > ls && (pb(e - 1) == b' ' || pb(e - 1) == b'\t' || pb(e - 1) == b'\r') {
        e -= 1;
    }
    e
}
 
fn block_end(start: usize, end: usize, ind: usize) -> usize {
    let mut pos = start;
    while pos < end {
        let le = line_end(pos);
        let (n, cs) = indent_of(pos, le);
        let ce = code_end(pos, le);
        if cs < ce && n <= ind {
            return pos;
        }
        pos = next_line(le);
    }
    end
}
 
fn bytes_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() { return false; }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] { return false; }
        i += 1;
    }
    true
}
 
fn bytes_cmp(a: &[u8], b: &[u8]) -> i32 {
    let mut i = 0;
    while i < a.len() && i < b.len() {
        if a[i] != b[i] {
            return if a[i] < b[i] { -1 } else { 1 };
        }
        i += 1;
    }
    if a.len() < b.len() { -1 } else if a.len() > b.len() { 1 } else { 0 }
}
 
fn contains(hay: &[u8], needle: &[u8]) -> bool {
    if needle.len() == 0 { return true; }
    if needle.len() > hay.len() { return false; }
    let mut i = 0;
    while i + needle.len() <= hay.len() {
        if bytes_eq(&hay[i..i + needle.len()], needle) { return true; }
        i += 1;
    }
    false
}
 
fn word_is(s: usize, e: usize, kw: &[u8]) -> bool {
    if e < s || e - s != kw.len() { return false; }
    let mut k = 0;
    while k < kw.len() {
        if pb(s + k) != kw[k] { return false; }
        k += 1;
    }
    true
}
 
fn ident_end(s: usize, e: usize) -> usize {
    let mut i = s;
    while i < e && is_ident_char(pb(i)) {
        i += 1;
    }
    i
}
 
fn find_close(s: usize, e: usize) -> Option<usize> {
    let mut depth = 0i32;
    let mut q: u8 = 0;
    let mut i = s;
    while i < e {
        let c = pb(i);
        if q != 0 {
            if c == b'\\' { i += 2; continue; }
            if c == q { q = 0; }
        } else if c == b'"' || c == b'\'' {
            q = c;
        } else if c == b'(' || c == b'[' || c == b'{' {
            depth += 1;
        } else if c == b')' || c == b']' || c == b'}' {
            depth -= 1;
            if depth == 0 { return Some(i); }
        }
        i += 1;
    }
    None
}
 
fn find_colon(s: usize, e: usize) -> Option<usize> {
    let mut depth = 0i32;
    let mut q: u8 = 0;
    let mut i = s;
    while i < e {
        let c = pb(i);
        if q != 0 {
            if c == b'\\' { i += 2; continue; }
            if c == q { q = 0; }
        } else if c == b'"' || c == b'\'' {
            q = c;
        } else if c == b'(' || c == b'[' || c == b'{' {
            depth += 1;
        } else if c == b')' || c == b']' || c == b'}' {
            depth -= 1;
        } else if c == b':' && depth == 0 {
            return Some(i);
        }
        i += 1;
    }
    None
}
 
fn find_assign(cs: usize, ce: usize) -> Option<(usize, usize, u8)> {
    let mut depth = 0i32;
    let mut q: u8 = 0;
    let mut i = cs;
    while i < ce {
        let c = pb(i);
        if q != 0 {
            if c == b'\\' { i += 2; continue; }
            if c == q { q = 0; }
        } else if c == b'"' || c == b'\'' {
            q = c;
        } else if c == b'(' || c == b'[' || c == b'{' {
            depth += 1;
        } else if c == b')' || c == b']' || c == b'}' {
            depth -= 1;
        } else if c == b'=' && depth == 0 {
            let nx = if i + 1 < ce { pb(i + 1) } else { 0 };
            let prev = if i > cs { pb(i - 1) } else { 0 };
            if nx == b'=' {
                i += 2;
                continue;
            }
            if prev == b'!' || prev == b'<' || prev == b'>' || prev == b'=' {
                i += 1;
                continue;
            }
            let mut aug = 0u8;
            let mut lhs_end = i;
            if prev == b'+' { aug = OP_ADD; lhs_end = i - 1; }
            else if prev == b'-' { aug = OP_SUB; lhs_end = i - 1; }
            else if prev == b'*' { aug = OP_MUL; lhs_end = i - 1; }
            else if prev == b'%' { aug = OP_MOD; lhs_end = i - 1; }
            else if prev == b'/' {
                aug = OP_DIV;
                lhs_end = i - 1;
                if i >= cs + 2 && pb(i - 2) == b'/' { lhs_end = i - 2; }
            }
            return Some((i, lhs_end, aug));
        }
        i += 1;
    }
    None
}
 
fn copy_name(s: usize, e: usize) -> Res<([u8; NAME_MAX], usize)> {
    if e <= s || e - s > NAME_MAX {
        return Err("SyntaxError: bad name (max 16 chars)");
    }
    let n = e - s;
    let mut out = [0u8; NAME_MAX];
    let mut k = 0;
    while k < n {
        out[k] = pb(s + k);
        k += 1;
    }
    Ok((out, n))
}
 
fn names_eq(stored: &[u8; NAME_MAX], slen: usize, s: usize, e: usize) -> bool {
    if e < s || slen != e - s { return false; }
    let mut k = 0;
    while k < slen {
        if stored[k] != pb(s + k) { return false; }
        k += 1;
    }
    true
}
 
fn step() -> Res<()> {
    unsafe {
        STEPS += 1;
        if STEPS > STEP_LIMIT {
            return Err("RuntimeError: program runs too long (infinite loop?)");
        }
    }
    Ok(())
}
 
 
 
fn push_int(p: &mut PStr, v: i64) {
    if v == 0 {
        p.push(b'0');
        return;
    }
    let neg = v < 0;
    let mut u: u64 = if neg { (!(v as u64)).wrapping_add(1) } else { v as u64 };
    let mut buf = [0u8; 20];
    let mut n = 0;
    while u > 0 {
        buf[n] = b'0' + (u % 10) as u8;
        u /= 10;
        n += 1;
    }
    if neg { p.push(b'-'); }
    while n > 0 {
        n -= 1;
        p.push(buf[n]);
    }
}
 
fn as_int(v: &Val) -> Option<i64> {
    match v {
        Val::Int(i) => Some(*i),
        Val::Bool(b) => Some(if *b { 1 } else { 0 }),
        _ => None,
    }
}
 
fn int_of(v: &Val) -> Res<i64> {
    match as_int(v) {
        Some(i) => Ok(i),
        None => Err("TypeError: an integer is required"),
    }
}
 
fn truthy(v: &Val) -> bool {
    match v {
        Val::None => false,
        Val::Bool(b) => *b,
        Val::Int(i) => *i != 0,
        Val::Str(s) => s.len > 0,
        _ => true,
    }
}
 
fn val_to_str(v: &Val) -> PStr {
    let mut p = PStr::new();
    match v {
        Val::None => p.push_bytes(b"None"),
        Val::Bool(b) => {
            if *b { p.push_bytes(b"True"); } else { p.push_bytes(b"False"); }
        }
        Val::Int(i) => push_int(&mut p, *i),
        Val::Str(s) => { p = *s; }
        Val::Obj(_) => p.push_bytes(b"<tkinter object>"),
        Val::Func(_) => p.push_bytes(b"<function>"),
        Val::Lambda(_, _) => p.push_bytes(b"<lambda>"),
    }
    p
}
 
fn val_eq(a: &Val, b: &Val) -> bool {
    if let (Some(x), Some(y)) = (as_int(a), as_int(b)) {
        return x == y;
    }
    match (a, b) {
        (Val::Str(x), Val::Str(y)) => bytes_eq(x.as_bytes(), y.as_bytes()),
        (Val::None, Val::None) => true,
        (Val::Obj(x), Val::Obj(y)) => x == y,
        (Val::Func(x), Val::Func(y)) => x == y,
        _ => false,
    }
}
 
fn floor_div(a: i64, b: i64) -> i64 {
    let q = a.wrapping_div(b);
    let r = a.wrapping_rem(b);
    if r != 0 && ((r < 0) != (b < 0)) { q - 1 } else { q }
}
 
fn floor_mod(a: i64, b: i64) -> i64 {
    let r = a.wrapping_rem(b);
    if r != 0 && ((r < 0) != (b < 0)) { r + b } else { r }
}
 
fn repeat_str(x: &PStr, n: i64) -> PStr {
    let mut p = PStr::new();
    if x.len == 0 || n <= 0 { return p; }
    let mut i = 0i64;
    while i < n && p.len < STR_MAX {
        p.push_bytes(x.as_bytes());
        i += 1;
    }
    p
}
 
fn binop(a: Val, op: u8, b: Val) -> Res<Val> {
    if op == OP_OR || op == OP_AND {
        return Ok(b);
    }
    if op == OP_EQ {
        return Ok(Val::Bool(val_eq(&a, &b)));
    }
    if op == OP_NE {
        return Ok(Val::Bool(!val_eq(&a, &b)));
    }
    if op == OP_IN {
        if let (Val::Str(x), Val::Str(y)) = (&a, &b) {
            return Ok(Val::Bool(contains(y.as_bytes(), x.as_bytes())));
        }
        return Err("TypeError: 'in' works only with strings");
    }
    if op == OP_LT || op == OP_GT || op == OP_LE || op == OP_GE {
        let c: i32 = if let (Some(x), Some(y)) = (as_int(&a), as_int(&b)) {
            if x < y { -1 } else if x > y { 1 } else { 0 }
        } else if let (Val::Str(x), Val::Str(y)) = (&a, &b) {
            bytes_cmp(x.as_bytes(), y.as_bytes())
        } else {
            return Err("TypeError: these values cannot be compared");
        };
        let r = if op == OP_LT { c < 0 }
                else if op == OP_GT { c > 0 }
                else if op == OP_LE { c <= 0 }
                else { c >= 0 };
        return Ok(Val::Bool(r));
    }
    if op == OP_ADD {
        if let (Val::Str(x), Val::Str(y)) = (&a, &b) {
            let mut p = *x;
            p.push_bytes(y.as_bytes());
            return Ok(Val::Str(p));
        }
    }
    if op == OP_MUL {
        if let (Val::Str(x), Some(n)) = (&a, as_int(&b)) {
            return Ok(Val::Str(repeat_str(x, n)));
        }
        if let (Some(n), Val::Str(x)) = (as_int(&a), &b) {
            return Ok(Val::Str(repeat_str(x, n)));
        }
    }
    let x = match as_int(&a) {
        Some(v) => v,
        None => return Err("TypeError: unsupported operand type(s)"),
    };
    let y = match as_int(&b) {
        Some(v) => v,
        None => return Err("TypeError: unsupported operand type(s)"),
    };
    if op == OP_ADD { return Ok(Val::Int(x.wrapping_add(y))); }
    if op == OP_SUB { return Ok(Val::Int(x.wrapping_sub(y))); }
    if op == OP_MUL { return Ok(Val::Int(x.wrapping_mul(y))); }
    if op == OP_DIV {
        if y == 0 { return Err("ZeroDivisionError: division by zero"); }
        return Ok(Val::Int(floor_div(x, y)));
    }
    if op == OP_MOD {
        if y == 0 { return Err("ZeroDivisionError: modulo by zero"); }
        return Ok(Val::Int(floor_mod(x, y)));
    }
    if op == OP_POW {
        if y < 0 { return Err("ValueError: negative exponent (float is not supported)"); }
        if y > 200 { return Err("ValueError: exponent too large"); }
        let mut r: i64 = 1;
        let mut k = 0;
        while k < y {
            r = r.wrapping_mul(x);
            k += 1;
        }
        return Ok(Val::Int(r));
    }
    Err("SyntaxError: bad operator")
}
 
 
 
fn is_declared_global(s: usize, e: usize) -> bool {
    unsafe {
        let mut i = DECL_BASE;
        while i < DECL_COUNT {
            if names_eq(&DECL[i], DECL_LEN[i], s, e) { return true; }
            i += 1;
        }
        false
    }
}
 
fn lookup_var(s: usize, e: usize) -> Option<Val> {
    unsafe {
        if CALL_DEPTH > 0 && !is_declared_global(s, e) {
            let mut i = LOCAL_COUNT;
            while i > LOCAL_BASE {
                i -= 1;
                if names_eq(&LOCALS[i].name, LOCALS[i].nlen, s, e) {
                    return Some(LOCALS[i].val);
                }
            }
        }
        let mut i = 0;
        while i < GLOBAL_COUNT {
            if names_eq(&GLOBALS[i].name, GLOBALS[i].nlen, s, e) {
                return Some(GLOBALS[i].val);
            }
            i += 1;
        }
        None
    }
}
 
fn set_global(s: usize, e: usize, v: Val) -> Res<()> {
    let (nm, nl) = copy_name(s, e)?;
    unsafe {
        let mut i = 0;
        while i < GLOBAL_COUNT {
            if names_eq(&GLOBALS[i].name, GLOBALS[i].nlen, s, e) {
                GLOBALS[i].val = v;
                return Ok(());
            }
            i += 1;
        }
        if GLOBAL_COUNT >= MAX_GLOBALS {
            return Err("RuntimeError: too many variables");
        }
        GLOBALS[GLOBAL_COUNT] = Var { name: nm, nlen: nl, val: v };
        GLOBAL_COUNT += 1;
    }
    Ok(())
}
 
fn set_var(s: usize, e: usize, v: Val) -> Res<()> {
    unsafe {
        if CALL_DEPTH > 0 && !is_declared_global(s, e) {
            let (nm, nl) = copy_name(s, e)?;
            let mut i = LOCAL_BASE;
            while i < LOCAL_COUNT {
                if names_eq(&LOCALS[i].name, LOCALS[i].nlen, s, e) {
                    LOCALS[i].val = v;
                    return Ok(());
                }
                i += 1;
            }
            if LOCAL_COUNT >= MAX_LOCALS {
                return Err("RuntimeError: too many local variables");
            }
            LOCALS[LOCAL_COUNT] = Var { name: nm, nlen: nl, val: v };
            LOCAL_COUNT += 1;
            return Ok(());
        }
    }
    set_global(s, e, v)
}
 
fn find_func(s: usize, e: usize) -> Option<usize> {
    unsafe {
        let mut i = 0;
        while i < FUNC_COUNT {
            if FUNCS[i].used && names_eq(&FUNCS[i].name, FUNCS[i].nlen, s, e) {
                return Some(i);
            }
            i += 1;
        }
        None
    }
}
 
fn name_error(s: usize, e: usize) -> &'static str {
    unsafe {
        ERR_NS = s;
        ERR_NE = e;
    }
    "NameError"
}
 
fn call_user(fi: usize, args: &Args) -> Res<Val> {
    unsafe {
        if CALL_DEPTH >= MAX_CALL_DEPTH {
            return Err("RecursionError: maximum recursion depth exceeded");
        }
        let f = FUNCS[fi];
        if args.pc != f.pcount {
            return Err("TypeError: wrong number of arguments");
        }
        let sv_base = LOCAL_BASE;
        let sv_count = LOCAL_COUNT;
        let sv_db = DECL_BASE;
        let sv_dc = DECL_COUNT;
 
        LOCAL_BASE = LOCAL_COUNT;
        DECL_BASE = DECL_COUNT;
        CALL_DEPTH += 1;
 
        let mut k = 0;
        while k < f.pcount {
            if LOCAL_COUNT >= MAX_LOCALS {
                CALL_DEPTH -= 1;
                LOCAL_BASE = sv_base;
                LOCAL_COUNT = sv_count;
                DECL_BASE = sv_db;
                DECL_COUNT = sv_dc;
                return Err("RuntimeError: too many local variables");
            }
            LOCALS[LOCAL_COUNT] = Var { name: f.params[k], nlen: f.plen[k], val: args.pos[k] };
            LOCAL_COUNT += 1;
            k += 1;
        }
 
        let r = exec_block(f.body_s, f.body_e);
 
        CALL_DEPTH -= 1;
        LOCAL_BASE = sv_base;
        LOCAL_COUNT = sv_count;
        DECL_BASE = sv_db;
        DECL_COUNT = sv_dc;
 
        match r? {
            Flow::Return(v) => Ok(v),
            _ => Ok(Val::None),
        }
    }
}
 
fn call_value(v: Val, args: &Args) -> Res<Val> {
    match v {
        Val::Func(fi) => call_user(fi, args),
        Val::Lambda(s, e) => eval_range(s, e),
        _ => Err("TypeError: object is not callable"),
    }
}
 
 
 
fn skip_ws(c: &mut Cur) {
    while c.i < c.e && (pb(c.i) == b' ' || pb(c.i) == b'\t') {
        c.i += 1;
    }
}
 
fn kw_at(c: &Cur, kw: &[u8]) -> bool {
    let n = kw.len();
    if c.i + n > c.e { return false; }
    let mut k = 0;
    while k < n {
        if pb(c.i + k) != kw[k] { return false; }
        k += 1;
    }
    if c.i + n < c.e && is_ident_char(pb(c.i + n)) { return false; }
    true
}
 
fn eval_range(s: usize, e: usize) -> Res<Val> {
    let mut c = Cur { i: s, e: e, skip: false };
    let v = parse_bin(&mut c, 0)?;
    skip_ws(&mut c);
    if c.i < c.e {
        return Err("SyntaxError: invalid syntax");
    }
    Ok(v)
}
 
fn peek_binop(c: &Cur) -> Option<(u8, u8, usize)> {
    if c.i >= c.e { return None; }
    let ch = pb(c.i);
    let nx = if c.i + 1 < c.e { pb(c.i + 1) } else { 0 };
    if ch == b'o' && kw_at(c, b"or") { return Some((OP_OR, 1, 2)); }
    if ch == b'a' && kw_at(c, b"and") { return Some((OP_AND, 2, 3)); }
    if ch == b'i' && kw_at(c, b"in") { return Some((OP_IN, 4, 2)); }
    match ch {
        b'=' => { if nx == b'=' { Some((OP_EQ, 4, 2)) } else { None } }
        b'!' => { if nx == b'=' { Some((OP_NE, 4, 2)) } else { None } }
        b'<' => { if nx == b'=' { Some((OP_LE, 4, 2)) } else { Some((OP_LT, 4, 1)) } }
        b'>' => { if nx == b'=' { Some((OP_GE, 4, 2)) } else { Some((OP_GT, 4, 1)) } }
        b'+' => Some((OP_ADD, 5, 1)),
        b'-' => Some((OP_SUB, 5, 1)),
        b'*' => { if nx == b'*' { Some((OP_POW, 8, 2)) } else { Some((OP_MUL, 6, 1)) } }
        b'/' => { if nx == b'/' { Some((OP_DIV, 6, 2)) } else { Some((OP_DIV, 6, 1)) } }
        b'%' => Some((OP_MOD, 6, 1)),
        _ => None,
    }
}
 
fn parse_bin(c: &mut Cur, min_prec: u8) -> Res<Val> {
    unsafe {
        EXPR_DEPTH += 1;
        if EXPR_DEPTH > MAX_EXPR_DEPTH {
            EXPR_DEPTH -= 1;
            return Err("RecursionError: expression is too deep");
        }
    }
    let r = parse_bin_inner(c, min_prec);
    unsafe {
        if EXPR_DEPTH > 0 { EXPR_DEPTH -= 1; }
    }
    r
}
 
fn parse_bin_inner(c: &mut Cur, min_prec: u8) -> Res<Val> {
    let mut left = parse_unary(c)?;
    loop {
        skip_ws(c);
        let (op, prec, len) = match peek_binop(c) {
            Some(x) => x,
            None => break,
        };
        if prec < min_prec { break; }
        c.i += len;
 
        let saved = c.skip;
        let short = !saved
            && ((op == OP_AND && !truthy(&left)) || (op == OP_OR && truthy(&left)));
        if short { c.skip = true; }
        let next_min = if op == OP_POW { prec } else { prec + 1 };
        let right = parse_bin(c, next_min);
        c.skip = saved;
        let right = right?;
 
        if short {
        } else if saved {
            left = Val::None;
        } else {
            left = binop(left, op, right)?;
        }
    }
    Ok(left)
}
 
fn parse_unary(c: &mut Cur) -> Res<Val> {
    skip_ws(c);
    if c.i < c.e {
        if kw_at(c, b"not") {
            c.i += 3;
            let v = parse_bin(c, 4)?;
            if c.skip { return Ok(Val::None); }
            return Ok(Val::Bool(!truthy(&v)));
        }
        let ch = pb(c.i);
        if ch == b'-' || ch == b'+' {
            c.i += 1;
            let v = parse_bin(c, 8)?;
            if c.skip { return Ok(Val::None); }
            let n = int_of(&v)?;
            return Ok(Val::Int(if ch == b'-' { n.wrapping_neg() } else { n }));
        }
    }
    parse_primary(c)
}
 
fn skip_quoted(c: &mut Cur) -> Res<()> {
    let q = pb(c.i);
    c.i += 1;
    loop {
        if c.i >= c.e { return Err("SyntaxError: unterminated string"); }
        let ch = pb(c.i);
        if ch == b'\\' { c.i += 2; continue; }
        c.i += 1;
        if ch == q { break; }
    }
    Ok(())
}
 
fn skip_primary(c: &mut Cur) -> Res<Val> {
    skip_ws(c);
    if c.i >= c.e { return Err("SyntaxError: unexpected end of expression"); }
    let ch = pb(c.i);
    if ch == b'(' || ch == b'[' {
        match find_close(c.i, c.e) {
            Some(x) => { c.i = x + 1; }
            None => return Err("SyntaxError: missing bracket"),
        }
    } else if ch == b'"' || ch == b'\'' {
        skip_quoted(c)?;
    } else if is_digit(ch) {
        while c.i < c.e && is_ident_char(pb(c.i)) { c.i += 1; }
    } else if is_ident_start(ch) {
        let s = c.i;
        while c.i < c.e && is_ident_char(pb(c.i)) { c.i += 1; }
        if c.i - s == 1 && c.i < c.e && (pb(c.i) == b'"' || pb(c.i) == b'\'') {
            skip_quoted(c)?;
        } else if word_is(s, c.i, b"lambda") {
            c.i = c.e;
            return Ok(Val::None);
        }
    } else {
        return Err("SyntaxError: invalid syntax");
    }
    loop {
        if c.i >= c.e { break; }
        let n = pb(c.i);
        if n == b'.' {
            c.i += 1;
            while c.i < c.e && is_ident_char(pb(c.i)) { c.i += 1; }
        } else if n == b'(' || n == b'[' {
            match find_close(c.i, c.e) {
                Some(x) => { c.i = x + 1; }
                None => return Err("SyntaxError: missing bracket"),
            }
        } else {
            break;
        }
    }
    Ok(Val::None)
}
 
fn parse_string(c: &mut Cur, is_f: bool) -> Res<Val> {
    let q = pb(c.i);
    c.i += 1;
    let mut out = PStr::new();
    loop {
        if c.i >= c.e { return Err("SyntaxError: unterminated string"); }
        let ch = pb(c.i);
        if ch == q {
            c.i += 1;
            break;
        }
        if ch == b'\\' {
            c.i += 1;
            if c.i >= c.e { return Err("SyntaxError: unterminated string"); }
            let n = pb(c.i);
            c.i += 1;
            if n == b'n' { out.push(b'\n'); }
            else if n == b't' { out.push(b' '); }
            else if n == b'\\' { out.push(b'\\'); }
            else if n == b'\'' { out.push(b'\''); }
            else if n == b'"' { out.push(b'"'); }
            else { out.push(b'\\'); out.push(n); }
            continue;
        }
        if is_f && ch == b'{' {
            if pb(c.i + 1) == b'{' {
                out.push(b'{');
                c.i += 2;
                continue;
            }
            let es = c.i + 1;
            let mut j = es;
            let mut depth = 1i32;
            let mut qq: u8 = 0;
            while j < c.e {
                let cj = pb(j);
                if qq != 0 {
                    if cj == qq { qq = 0; }
                } else if cj == b'"' || cj == b'\'' {
                    qq = cj;
                } else if cj == b'{' {
                    depth += 1;
                } else if cj == b'}' {
                    depth -= 1;
                    if depth == 0 { break; }
                }
                j += 1;
            }
            if j >= c.e { return Err("SyntaxError: f-string: expecting '}'"); }
            let v = eval_range(es, j)?;
            let sv = val_to_str(&v);
            out.push_bytes(sv.as_bytes());
            c.i = j + 1;
            continue;
        }
        if is_f && ch == b'}' && pb(c.i + 1) == b'}' {
            out.push(b'}');
            c.i += 2;
            continue;
        }
        out.push(ch);
        c.i += 1;
    }
    Ok(Val::Str(out))
}
 
fn parse_primary(c: &mut Cur) -> Res<Val> {
    skip_ws(c);
    if c.skip {
        return skip_primary(c);
    }
    if c.i >= c.e { return Err("SyntaxError: unexpected end of expression"); }
    let ch = pb(c.i);
    let mut v: Val;
    if is_digit(ch) {
        let mut n: i64 = 0;
        while c.i < c.e && is_digit(pb(c.i)) {
            n = n.wrapping_mul(10).wrapping_add((pb(c.i) - b'0') as i64);
            c.i += 1;
        }
        if c.i < c.e && pb(c.i) == b'.' && is_digit(pb(c.i + 1)) {
            return Err("TypeError: float numbers are not supported");
        }
        v = Val::Int(n);
    } else if ch == b'"' || ch == b'\'' {
        v = parse_string(c, false)?;
    } else if ch == b'(' {
        c.i += 1;
        v = parse_bin(c, 0)?;
        skip_ws(c);
        if c.i >= c.e || pb(c.i) != b')' {
            return Err("SyntaxError: expected ')'");
        }
        c.i += 1;
    } else if is_ident_start(ch) {
        v = parse_name(c)?;
    } else if ch == b'[' || ch == b'{' {
        return Err("SyntaxError: lists and dicts are not supported");
    } else {
        return Err("SyntaxError: invalid syntax");
    }
 
    loop {
        if c.i >= c.e { break; }
        let n = pb(c.i);
        if n == b'.' {
            c.i += 1;
            let ns = c.i;
            while c.i < c.e && is_ident_char(pb(c.i)) { c.i += 1; }
            let ne = c.i;
            if ne == ns { return Err("SyntaxError: invalid syntax"); }
            if c.i < c.e && pb(c.i) == b'(' {
                let a = parse_args(c)?;
                v = call_method(v, ns, ne, &a)?;
            } else {
                return Err("AttributeError: attributes are not supported");
            }
        } else if n == b'[' {
            let close = match find_close(c.i, c.e) {
                Some(x) => x,
                None => return Err("SyntaxError: missing ']'"),
            };
            let idx = eval_range(c.i + 1, close)?;
            c.i = close + 1;
            v = index_value(v, idx)?;
        } else if n == b'(' {
            let a = parse_args(c)?;
            v = call_value(v, &a)?;
        } else {
            break;
        }
    }
    Ok(v)
}
 
const TK_CONSTS: &[&[u8]] = &[
    b"LEFT", b"RIGHT", b"TOP", b"BOTTOM", b"BOTH", b"X", b"Y",
    b"W", b"E", b"N", b"S", b"NW", b"NE", b"SW", b"SE", b"CENTER",
    b"NORMAL", b"DISABLED", b"ACTIVE", b"WORD", b"YES", b"NO",
];
 
fn tk_constant(s: usize, e: usize) -> Option<Val> {
    if word_is(s, e, b"END") {
        return Some(Val::Int(9999));
    }
    let mut i = 0;
    while i < TK_CONSTS.len() {
        if word_is(s, e, TK_CONSTS[i]) {
            let mut p = PStr::new();
            let mut k = s;
            while k < e {
                let ch = pb(k);
                p.push(if ch >= b'A' && ch <= b'Z' { ch + 32 } else { ch });
                k += 1;
            }
            return Some(Val::Str(p));
        }
        i += 1;
    }
    None
}
 
fn parse_name(c: &mut Cur) -> Res<Val> {
    let s = c.i;
    while c.i < c.e && is_ident_char(pb(c.i)) { c.i += 1; }
    let e = c.i;
 
    if e - s == 1 && c.i < c.e && (pb(c.i) == b'"' || pb(c.i) == b'\'') {
        let p = pb(s);
        if p == b'f' || p == b'F' { return parse_string(c, true); }
        if p == b'r' || p == b'R' || p == b'b' || p == b'B' { return parse_string(c, false); }
    }
    if word_is(s, e, b"True") { return Ok(Val::Bool(true)); }
    if word_is(s, e, b"False") { return Ok(Val::Bool(false)); }
    if word_is(s, e, b"None") { return Ok(Val::None); }
    if word_is(s, e, b"__name__") { return Ok(Val::Str(PStr::from_bytes(b"__main__"))); }
 
    if word_is(s, e, b"lambda") {
        let mut k = c.i;
        while k < c.e && pb(k) != b':' { k += 1; }
        if k >= c.e { return Err("SyntaxError: lambda needs ':'"); }
        let bs = skip_sp(k + 1, c.e);
        let be = c.e;
        c.i = c.e;
        return Ok(Val::Lambda(bs, be));
    }
 
    let mut k = c.i;
    while k < c.e && pb(k) == b' ' { k += 1; }
    if k < c.e && pb(k) == b'(' {
        c.i = k;
        return call_by_name(s, e, c);
    }
 
    if let Some(v) = lookup_var(s, e) { return Ok(v); }
    if let Some(fi) = find_func(s, e) { return Ok(Val::Func(fi)); }
 
    if c.i < c.e && pb(c.i) == b'.' {
        let mut ns = c.i;
        let mut ne = c.i;
        while c.i < c.e && pb(c.i) == b'.' {
            c.i += 1;
            ns = c.i;
            while c.i < c.e && is_ident_char(pb(c.i)) { c.i += 1; }
            ne = c.i;
            if ne == ns { return Err("SyntaxError: invalid syntax"); }
        }
        if c.i < c.e && pb(c.i) == b'(' {
            return call_by_name(ns, ne, c);
        }
        if let Some(v) = tk_constant(ns, ne) { return Ok(v); }
        return Err(name_error(ns, ne));
    }
 
    if let Some(v) = tk_constant(s, e) { return Ok(v); }
    Err(name_error(s, e))
}
 
fn index_value(v: Val, idx: Val) -> Res<Val> {
    match v {
        Val::Str(p) => {
            let n = int_of(&idx)?;
            let l = p.len as i64;
            let k = if n < 0 { n + l } else { n };
            if k < 0 || k >= l {
                return Err("IndexError: string index out of range");
            }
            let mut o = PStr::new();
            o.push(p.b[k as usize]);
            Ok(Val::Str(o))
        }
        Val::Obj(wi) => {
            if let Val::Str(key) = idx {
                unsafe {
                    if wi < WIDGET_COUNT && bytes_eq(key.as_bytes(), b"text") {
                        return Ok(Val::Str(WIDGETS[wi].text));
                    }
                }
            }
            Err("KeyError: this widget option is not supported")
        }
        _ => Err("TypeError: object is not subscriptable"),
    }
}
 
 
 
fn add_arg(a: &mut Args, s0: usize, e0: usize) -> Res<()> {
    let s = skip_sp(s0, e0);
    let mut e = e0;
    while e > s && (pb(e - 1) == b' ' || pb(e - 1) == b'\t') {
        e -= 1;
    }
    if s >= e { return Ok(()); }
 
    let ie = ident_end(s, e);
    if ie > s && is_ident_start(pb(s)) {
        let k = skip_sp(ie, e);
        if k < e && pb(k) == b'=' && pb(k + 1) != b'=' {
            if a.kc >= MAX_ARGS { return Err("TypeError: too many arguments"); }
            let vs = skip_sp(k + 1, e);
            let v = eval_range(vs, e)?;
            a.kn[a.kc] = (s, ie);
            a.kv[a.kc] = v;
            a.kc += 1;
            return Ok(());
        }
    }
    if a.pc >= MAX_ARGS { return Err("TypeError: too many arguments"); }
    let v = eval_range(s, e)?;
    a.pos[a.pc] = v;
    a.pc += 1;
    Ok(())
}
 
fn parse_args(c: &mut Cur) -> Res<Args> {
    let close = match find_close(c.i, c.e) {
        Some(x) => x,
        None => return Err("SyntaxError: missing ')'"),
    };
    let from = c.i + 1;
    let mut a = Args::new();
    let mut part_s = from;
    let mut depth = 0i32;
    let mut q: u8 = 0;
    let mut j = from;
    loop {
        let at_end = j >= close;
        let mut is_sep = false;
        if !at_end {
            let ch = pb(j);
            if q != 0 {
                if ch == b'\\' { j += 2; continue; }
                if ch == q { q = 0; }
            } else if ch == b'"' || ch == b'\'' {
                q = ch;
            } else if ch == b'(' || ch == b'[' || ch == b'{' {
                depth += 1;
            } else if ch == b')' || ch == b']' || ch == b'}' {
                depth -= 1;
            } else if ch == b',' && depth == 0 {
                is_sep = true;
            }
        }
        if at_end || is_sep {
            let pe = if j > close { close } else { j };
            add_arg(&mut a, part_s, pe)?;
            part_s = j + 1;
            if at_end { break; }
        }
        j += 1;
    }
    c.i = close + 1;
    Ok(a)
}
 
fn kw_get(a: &Args, name: &[u8]) -> Option<Val> {
    let mut i = 0;
    while i < a.kc {
        let (s, e) = a.kn[i];
        if word_is(s, e, name) {
            return Some(a.kv[i]);
        }
        i += 1;
    }
    None
}
 
fn need(a: &Args, n: usize) -> Res<()> {
    if a.pc < n {
        return Err("TypeError: missing argument");
    }
    Ok(())
}
 
fn clamp_u(v: i64, max: i64) -> usize {
    if v < 0 { 0 } else if v > max { max as usize } else { v as usize }
}
 
 
 
fn call_by_name(ns: usize, ne: usize, c: &mut Cur) -> Res<Val> {
    let args = parse_args(c)?;
    if let Some(fi) = find_func(ns, ne) {
        return call_user(fi, &args);
    }
    if let Some(v) = lookup_var(ns, ne) {
        return call_value(v, &args);
    }
    match call_builtin(ns, ne, &args)? {
        Some(v) => Ok(v),
        None => Err(name_error(ns, ne)),
    }
}
 
fn do_print(a: &Args) {
    let mut sep = PStr::from_bytes(b" ");
    let mut end = PStr::from_bytes(b"\n");
    if let Some(Val::Str(p)) = kw_get(a, b"sep") { sep = p; }
    if let Some(Val::Str(p)) = kw_get(a, b"end") { end = p; }
    let mut i = 0;
    while i < a.pc {
        if i > 0 { con_put_bytes(sep.as_bytes()); }
        let s = val_to_str(&a.pos[i]);
        con_put_bytes(s.as_bytes());
        i += 1;
    }
    con_put_bytes(end.as_bytes());
}
 
fn parse_int_str(p: &PStr) -> Res<i64> {
    let b = p.as_bytes();
    let mut s = 0;
    let mut e = b.len();
    while s < e && b[s] == b' ' { s += 1; }
    while e > s && b[e - 1] == b' ' { e -= 1; }
    let mut neg = false;
    if s < e && (b[s] == b'-' || b[s] == b'+') {
        neg = b[s] == b'-';
        s += 1;
    }
    if s >= e { return Err("ValueError: invalid literal for int()"); }
    let mut n: i64 = 0;
    while s < e {
        if !is_digit(b[s]) { return Err("ValueError: invalid literal for int()"); }
        n = n.wrapping_mul(10).wrapping_add((b[s] - b'0') as i64);
        s += 1;
    }
    Ok(if neg { n.wrapping_neg() } else { n })
}
 
fn call_builtin(s: usize, e: usize, a: &Args) -> Res<Option<Val>> {
    if word_is(s, e, b"print") {
        do_print(a);
        return Ok(Some(Val::None));
    }
    if word_is(s, e, b"len") {
        need(a, 1)?;
        return match a.pos[0] {
            Val::Str(p) => Ok(Some(Val::Int(p.len as i64))),
            _ => Err("TypeError: object has no len()"),
        };
    }
    if word_is(s, e, b"str") {
        if a.pc == 0 { return Ok(Some(Val::Str(PStr::new()))); }
        return Ok(Some(Val::Str(val_to_str(&a.pos[0]))));
    }
    if word_is(s, e, b"int") {
        if a.pc == 0 { return Ok(Some(Val::Int(0))); }
        return match a.pos[0] {
            Val::Str(p) => Ok(Some(Val::Int(parse_int_str(&p)?))),
            ref other => Ok(Some(Val::Int(int_of(other)?))),
        };
    }
    if word_is(s, e, b"bool") {
        if a.pc == 0 { return Ok(Some(Val::Bool(false))); }
        return Ok(Some(Val::Bool(truthy(&a.pos[0]))));
    }
    if word_is(s, e, b"abs") {
        need(a, 1)?;
        let n = int_of(&a.pos[0])?;
        return Ok(Some(Val::Int(if n < 0 { n.wrapping_neg() } else { n })));
    }
    if word_is(s, e, b"min") || word_is(s, e, b"max") {
        need(a, 1)?;
        let is_min = word_is(s, e, b"min");
        let mut best = int_of(&a.pos[0])?;
        let mut i = 1;
        while i < a.pc {
            let n = int_of(&a.pos[i])?;
            if (is_min && n < best) || (!is_min && n > best) { best = n; }
            i += 1;
        }
        return Ok(Some(Val::Int(best)));
    }
    if word_is(s, e, b"input") {
        if a.pc > 0 {
            let p = val_to_str(&a.pos[0]);
            con_put_bytes(p.as_bytes());
            con_put_byte(b'\n');
        }
        return Ok(Some(Val::Str(PStr::new())));
    }
    if word_is(s, e, b"sleep") || word_is(s, e, b"exit") || word_is(s, e, b"quit") {
        return Ok(Some(Val::None));
    }
 
    if word_is(s, e, b"Tk") || word_is(s, e, b"Toplevel") {
        return Ok(Some(tk_create(4, a)?));
    }
    if word_is(s, e, b"Label") { return Ok(Some(tk_create(0, a)?)); }
    if word_is(s, e, b"Button") { return Ok(Some(tk_create(1, a)?)); }
    if word_is(s, e, b"Entry") { return Ok(Some(tk_create(2, a)?)); }
    if word_is(s, e, b"Checkbutton") { return Ok(Some(tk_create(3, a)?)); }
    if word_is(s, e, b"Frame") || word_is(s, e, b"LabelFrame") {
        unsafe {
            if a.pc > 0 {
                if let Val::Obj(p) = a.pos[0] {
                    if p < WIDGET_COUNT {
                        return Ok(Some(Val::Obj(WIDGETS[p].win)));
                    }
                }
            }
            if ROOT_WIN != usize::MAX {
                return Ok(Some(Val::Obj(ROOT_WIN)));
            }
        }
        return Err("RuntimeError: create the window first: root = tk.Tk()");
    }
    if word_is(s, e, b"showinfo") || word_is(s, e, b"showwarning") || word_is(s, e, b"showerror") {
        con_put_byte(b'[');
        if a.pc > 0 {
            let t = val_to_str(&a.pos[0]);
            con_put_bytes(t.as_bytes());
        }
        con_put_str("] ");
        if a.pc > 1 {
            let m = val_to_str(&a.pos[1]);
            con_put_bytes(m.as_bytes());
        }
        con_put_byte(b'\n');
        return Ok(Some(Val::None));
    }
    Ok(None)
}
 
fn apply_kwargs(wi: usize, a: &Args) {
    unsafe {
        let mut i = 0;
        while i < a.kc {
            let (s, e) = a.kn[i];
            if word_is(s, e, b"text") {
                WIDGETS[wi].text = val_to_str(&a.kv[i]);
            } else if word_is(s, e, b"command") {
                WIDGETS[wi].cmd = a.kv[i];
            }
            i += 1;
        }
    }
}
 
fn tk_create(kind: u8, a: &Args) -> Res<Val> {
    unsafe {
        if WIDGET_COUNT >= MAX_WIDGETS {
            return Err("RuntimeError: too many widgets");
        }
        let idx = WIDGET_COUNT;
        let mut w = EMPTY_WIDGET;
        w.used = true;
        w.alive = true;
        w.kind = kind;
 
        if kind == 4 {
            w.text = PStr::from_bytes(b"tk");
            w.win = idx;
            WIDGETS[idx] = w;
            WIDGET_COUNT += 1;
            ACTIVE_WIN = idx;
            if ROOT_WIN == usize::MAX { ROOT_WIN = idx; }
            TK_USED = true;
            return Ok(Val::Obj(idx));
        }
 
        let mut parent = ROOT_WIN;
        if a.pc > 0 {
            if let Val::Obj(p) = a.pos[0] {
                if p < WIDGET_COUNT { parent = WIDGETS[p].win; }
            }
        }
        if parent == usize::MAX {
            return Err("RuntimeError: create the window first: root = tk.Tk()");
        }
        w.win = parent;
        WIDGETS[idx] = w;
        WIDGET_COUNT += 1;
        apply_kwargs(idx, a);
        Ok(Val::Obj(idx))
    }
}
 
fn destroy_widget(wi: usize) {
    unsafe {
        if wi >= WIDGET_COUNT { return; }
        if WIDGETS[wi].kind != 4 {
            WIDGETS[wi].alive = false;
            return;
        }
        if wi == ROOT_WIN {
            let mut i = 0;
            while i < WIDGET_COUNT {
                WIDGETS[i].alive = false;
                i += 1;
            }
            ACTIVE_WIN = usize::MAX;
            return;
        }
        WIDGETS[wi].alive = false;
        let mut i = 0;
        while i < WIDGET_COUNT {
            if WIDGETS[i].win == wi { WIDGETS[i].alive = false; }
            i += 1;
        }
        if ACTIVE_WIN == wi {
            ACTIVE_WIN = usize::MAX;
            let mut k = WIDGET_COUNT;
            while k > 0 {
                k -= 1;
                if WIDGETS[k].kind == 4 && WIDGETS[k].alive {
                    ACTIVE_WIN = k;
                    break;
                }
            }
        }
    }
}
 
fn run_widget_command(wi: usize) {
    unsafe {
        if wi >= WIDGET_COUNT { return; }
        let cmd = WIDGETS[wi].cmd;
        match cmd {
            Val::Func(_) | Val::Lambda(_, _) => {
                reset_exec_state();
                let args = Args::new();
                if let Err(m) = call_value(cmd, &args) {
                    report_error(m);
                }
            }
            _ => {}
        }
    }
}
 
fn widget_method(wi: usize, ns: usize, ne: usize, a: &Args) -> Res<Val> {
    unsafe {
        if wi >= WIDGET_COUNT {
            return Err("RuntimeError: bad widget");
        }
        let kind = WIDGETS[wi].kind;
 
        if word_is(ns, ne, b"pack") {
            WIDGETS[wi].layout = 1;
            WIDGETS[wi].side_left = false;
            if let Some(Val::Str(p)) = kw_get(a, b"side") {
                WIDGETS[wi].side_left = bytes_eq(p.as_bytes(), b"left");
            }
            return Ok(Val::None);
        }
        if word_is(ns, ne, b"grid") {
            let mut row = 0i64;
            let mut col = 0i64;
            if let Some(v) = kw_get(a, b"row") { row = int_of(&v)?; }
            if let Some(v) = kw_get(a, b"column") { col = int_of(&v)?; }
            WIDGETS[wi].layout = 2;
            WIDGETS[wi].gy = clamp_u(row, 50);
            WIDGETS[wi].gx = clamp_u(col, 50);
            return Ok(Val::None);
        }
        if word_is(ns, ne, b"place") {
            let mut px = 0i64;
            let mut py = 0i64;
            if let Some(v) = kw_get(a, b"x") { px = int_of(&v)?; }
            if let Some(v) = kw_get(a, b"y") { py = int_of(&v)?; }
            WIDGETS[wi].layout = 3;
            WIDGETS[wi].gx = clamp_u(px, 2000);
            WIDGETS[wi].gy = clamp_u(py, 2000);
            return Ok(Val::None);
        }
        if word_is(ns, ne, b"pack_forget") || word_is(ns, ne, b"grid_forget") || word_is(ns, ne, b"place_forget") {
            WIDGETS[wi].layout = 0;
            return Ok(Val::None);
        }
        if word_is(ns, ne, b"config") || word_is(ns, ne, b"configure") {
            apply_kwargs(wi, a);
            return Ok(Val::None);
        }
        if word_is(ns, ne, b"title") {
            if kind == 4 && a.pc >= 1 {
                WIDGETS[wi].text = val_to_str(&a.pos[0]);
            }
            return Ok(Val::None);
        }
        if word_is(ns, ne, b"destroy") {
            destroy_widget(wi);
            return Ok(Val::None);
        }
        if word_is(ns, ne, b"quit") {
            if ROOT_WIN != usize::MAX { destroy_widget(ROOT_WIN); }
            return Ok(Val::None);
        }
        if word_is(ns, ne, b"get") {
            if kind == 2 {
                return Ok(Val::Str(WIDGETS[wi].text));
            }
            return Err("AttributeError: get() works only for Entry");
        }
        if word_is(ns, ne, b"insert") {
            if kind != 2 { return Err("AttributeError: insert() works only for Entry"); }
            need(a, 2)?;
            let pos = int_of(&a.pos[0])?;
            let ins = val_to_str(&a.pos[1]);
            let cur = WIDGETS[wi].text;
            let at = clamp_u(pos, cur.len as i64);
            let mut out = PStr::new();
            out.push_bytes(&cur.b[..at]);
            out.push_bytes(ins.as_bytes());
            out.push_bytes(&cur.b[at..cur.len]);
            WIDGETS[wi].text = out;
            return Ok(Val::None);
        }
        if word_is(ns, ne, b"delete") {
            if kind != 2 { return Err("AttributeError: delete() works only for Entry"); }
            need(a, 1)?;
            let cur = WIDGETS[wi].text;
            let first = clamp_u(int_of(&a.pos[0])?, cur.len as i64);
            let last = if a.pc > 1 {
                clamp_u(int_of(&a.pos[1])?, cur.len as i64)
            } else {
                clamp_u((first as i64) + 1, cur.len as i64)
            };
            let mut out = PStr::new();
            out.push_bytes(&cur.b[..first]);
            if last > first { out.push_bytes(&cur.b[last..cur.len]); }
            else { out.push_bytes(&cur.b[first..cur.len]); }
            WIDGETS[wi].text = out;
            return Ok(Val::None);
        }
        if word_is(ns, ne, b"select") {
            WIDGETS[wi].checked = true;
            return Ok(Val::None);
        }
        if word_is(ns, ne, b"deselect") {
            WIDGETS[wi].checked = false;
            return Ok(Val::None);
        }
        if word_is(ns, ne, b"toggle") {
            WIDGETS[wi].checked = !WIDGETS[wi].checked;
            return Ok(Val::None);
        }
        if word_is(ns, ne, b"invoke") {
            run_widget_command(wi);
            return Ok(Val::None);
        }
        let noops: [&[u8]; 14] = [
            b"mainloop", b"update", b"update_idletasks", b"geometry", b"resizable",
            b"minsize", b"maxsize", b"focus", b"focus_set", b"lift", b"bind",
            b"after", b"protocol", b"attributes",
        ];
        let mut i = 0;
        while i < noops.len() {
            if word_is(ns, ne, noops[i]) { return Ok(Val::None); }
            i += 1;
        }
        Err("AttributeError: this widget method is not supported")
    }
}
 
fn str_method(p: PStr, ns: usize, ne: usize) -> Res<Val> {
    let mut o = PStr::new();
    if word_is(ns, ne, b"upper") || word_is(ns, ne, b"lower") {
        let up = word_is(ns, ne, b"upper");
        let mut i = 0;
        while i < p.len {
            let ch = p.b[i];
            if up && ch >= b'a' && ch <= b'z' { o.push(ch - 32); }
            else if !up && ch >= b'A' && ch <= b'Z' { o.push(ch + 32); }
            else { o.push(ch); }
            i += 1;
        }
        return Ok(Val::Str(o));
    }
    if word_is(ns, ne, b"strip") {
        let mut s = 0;
        let mut e = p.len;
        while s < e && p.b[s] == b' ' { s += 1; }
        while e > s && p.b[e - 1] == b' ' { e -= 1; }
        o.push_bytes(&p.b[s..e]);
        return Ok(Val::Str(o));
    }
    if word_is(ns, ne, b"isdigit") {
        let mut ok = p.len > 0;
        let mut i = 0;
        while i < p.len {
            if !is_digit(p.b[i]) { ok = false; }
            i += 1;
        }
        return Ok(Val::Bool(ok));
    }
    Err("AttributeError: this str method is not supported")
}
 
fn call_method(recv: Val, ns: usize, ne: usize, a: &Args) -> Res<Val> {
    match recv {
        Val::Obj(wi) => widget_method(wi, ns, ne, a),
        Val::Str(p) => str_method(p, ns, ne),
        _ => Err("AttributeError: object has no such method"),
    }
}
 
 
 
fn parse_body(colon: usize, ce: usize, next: usize, end: usize, ind: usize) -> Body {
    let rest = skip_sp(colon + 1, ce);
    if rest < ce {
        Body { s: rest, e: next, after: next }
    } else {
        let be = block_end(next, end, ind);
        Body { s: next, e: be, after: be }
    }
}
 
fn exec_block(start: usize, end: usize) -> Res<Flow> {
    let mut pos = start;
    while pos < end {
        let le = line_end(pos);
        let (ind, cs) = indent_of(pos, le);
        let ce = code_end(pos, le);
        let next = next_line(le);
        if cs >= ce {
            pos = next;
            continue;
        }
        step()?;
        unsafe { CUR_POS = cs; }
        let (flow, np) = exec_stmt(cs, ce, ind, next, end)?;
        pos = if np > pos { np } else { next };
        match flow {
            Flow::Normal => {}
            other => return Ok(other),
        }
    }
    Ok(Flow::Normal)
}
 
fn exec_stmt(cs: usize, ce: usize, ind: usize, next: usize, end: usize) -> Res<(Flow, usize)> {
    let we = ident_end(cs, ce);
    if we > cs {
        if word_is(cs, we, b"if") { return do_if(cs, ce, ind, next, end); }
        if word_is(cs, we, b"while") { return do_while(cs, ce, ind, next, end); }
        if word_is(cs, we, b"for") { return do_for(cs, ce, ind, next, end); }
        if word_is(cs, we, b"def") {
            let after = do_def(cs, ce, ind, next, end)?;
            return Ok((Flow::Normal, after));
        }
        if word_is(cs, we, b"return") {
            let s = skip_sp(we, ce);
            let v = if s < ce { eval_range(s, ce)? } else { Val::None };
            return Ok((Flow::Return(v), next));
        }
        if word_is(cs, we, b"break") { return Ok((Flow::Break, next)); }
        if word_is(cs, we, b"continue") { return Ok((Flow::Continue, next)); }
        if word_is(cs, we, b"pass") { return Ok((Flow::Normal, next)); }
        if word_is(cs, we, b"import") || word_is(cs, we, b"from") {
            return Ok((Flow::Normal, next));
        }
        if word_is(cs, we, b"global") {
            do_global(we, ce)?;
            return Ok((Flow::Normal, next));
        }
        if word_is(cs, we, b"elif") || word_is(cs, we, b"else") {
            return Err("SyntaxError: unexpected elif / else");
        }
        if word_is(cs, we, b"class") || word_is(cs, we, b"try") || word_is(cs, we, b"except")
            || word_is(cs, we, b"with") || word_is(cs, we, b"del") || word_is(cs, we, b"raise")
            || word_is(cs, we, b"assert") || word_is(cs, we, b"yield") || word_is(cs, we, b"nonlocal") {
            return Err("SyntaxError: this statement is not supported");
        }
    }
    exec_simple(cs, ce)?;
    Ok((Flow::Normal, next))
}
 
fn exec_simple(cs: usize, ce: usize) -> Res<()> {
    if let Some((op_pos, lhs_end, aug)) = find_assign(cs, ce) {
        let ls = skip_sp(cs, lhs_end);
        let mut le = lhs_end;
        while le > ls && (pb(le - 1) == b' ' || pb(le - 1) == b'\t') {
            le -= 1;
        }
        let ie = ident_end(ls, le);
        if ie == ls || !is_ident_start(pb(ls)) {
            return Err("SyntaxError: cannot assign to this expression");
        }
        let rhs = eval_range(skip_sp(op_pos + 1, ce), ce)?;
        if ie == le {
            let mut v = rhs;
            if aug != 0 {
                let cur = match lookup_var(ls, ie) {
                    Some(x) => x,
                    None => return Err(name_error(ls, ie)),
                };
                v = binop(cur, aug, rhs)?;
            }
            return set_var(ls, ie, v);
        }
        if pb(ie) == b'[' {
            let close = match find_close(ie, le) {
                Some(x) => x,
                None => return Err("SyntaxError: missing ']'"),
            };
            let key = eval_range(ie + 1, close)?;
            let obj = match lookup_var(ls, ie) {
                Some(x) => x,
                None => return Err(name_error(ls, ie)),
            };
            if let (Val::Obj(wi), Val::Str(k)) = (obj, key) {
                unsafe {
                    if wi < WIDGET_COUNT {
                        if bytes_eq(k.as_bytes(), b"text") {
                            WIDGETS[wi].text = val_to_str(&rhs);
                        } else if bytes_eq(k.as_bytes(), b"command") {
                            WIDGETS[wi].cmd = rhs;
                        }
                    }
                }
                return Ok(());
            }
            return Err("TypeError: item assignment is not supported");
        }
        return Err("SyntaxError: cannot assign to this expression");
    }
    eval_range(cs, ce)?;
    Ok(())
}
 
fn do_global(from: usize, ce: usize) -> Res<()> {
    let mut i = from;
    loop {
        i = skip_sp(i, ce);
        let ne = ident_end(i, ce);
        if ne == i { break; }
        unsafe {
            if CALL_DEPTH > 0 {
                if DECL_COUNT >= MAX_DECL {
                    return Err("RuntimeError: too many global declarations");
                }
                let (nm, nl) = copy_name(i, ne)?;
                DECL[DECL_COUNT] = nm;
                DECL_LEN[DECL_COUNT] = nl;
                DECL_COUNT += 1;
            }
            let mut exists = false;
            let mut k = 0;
            while k < GLOBAL_COUNT {
                if names_eq(&GLOBALS[k].name, GLOBALS[k].nlen, i, ne) { exists = true; break; }
                k += 1;
            }
            if !exists {
                set_global(i, ne, Val::None)?;
            }
        }
        i = skip_sp(ne, ce);
        if i < ce && pb(i) == b',' { i += 1; } else { break; }
    }
    Ok(())
}
 
fn do_def(cs: usize, ce: usize, ind: usize, next: usize, end: usize) -> Res<usize> {
    let ns = skip_sp(cs + 3, ce);
    let ne = ident_end(ns, ce);
    if ne == ns { return Err("SyntaxError: function name expected"); }
    let (fname, flen) = copy_name(ns, ne)?;
    let po = skip_sp(ne, ce);
    if po >= ce || pb(po) != b'(' { return Err("SyntaxError: expected '('"); }
    let pclose = match find_close(po, ce) {
        Some(x) => x,
        None => return Err("SyntaxError: missing ')'"),
    };
    let colon = skip_sp(pclose + 1, ce);
    if colon >= ce || pb(colon) != b':' { return Err("SyntaxError: expected ':'"); }
    let body = parse_body(colon, ce, next, end, ind);
 
    let mut f = EMPTY_FUNC;
    f.used = true;
    f.name = fname;
    f.nlen = flen;
    f.body_s = body.s;
    f.body_e = body.e;
 
    let mut i = po + 1;
    loop {
        i = skip_sp(i, pclose);
        if i >= pclose { break; }
        let pe = ident_end(i, pclose);
        if pe == i { return Err("SyntaxError: bad parameter"); }
        if f.pcount >= MAX_PARAMS { return Err("SyntaxError: too many parameters (max 4)"); }
        let (pn, pl) = copy_name(i, pe)?;
        f.params[f.pcount] = pn;
        f.plen[f.pcount] = pl;
        f.pcount += 1;
        i = skip_sp(pe, pclose);
        if i < pclose && pb(i) == b',' {
            i += 1;
        } else if i < pclose {
            return Err("SyntaxError: default values are not supported");
        }
    }
 
    unsafe {
        let mut slot = FUNC_COUNT;
        let mut k = 0;
        while k < FUNC_COUNT {
            if names_eq(&FUNCS[k].name, FUNCS[k].nlen, ns, ne) { slot = k; break; }
            k += 1;
        }
        if slot >= MAX_FUNCS { return Err("RuntimeError: too many functions"); }
        FUNCS[slot] = f;
        if slot == FUNC_COUNT { FUNC_COUNT += 1; }
    }
    Ok(body.after)
}
 
fn do_if(cs: usize, ce: usize, ind: usize, next: usize, end: usize) -> Res<(Flow, usize)> {
    let mut ccs = cs;
    let mut cce = ce;
    let mut cnext = next;
    let mut done = false;
    loop {
        let we = ident_end(ccs, cce);
        let is_else = word_is(ccs, we, b"else");
        let hs = we;
        let colon = match find_colon(hs, cce) {
            Some(x) => x,
            None => return Err("SyntaxError: expected ':'"),
        };
        let body = parse_body(colon, cce, cnext, end, ind);
        if !done {
            let run = if is_else {
                true
            } else {
                unsafe { CUR_POS = ccs; }
                let v = eval_range(hs, colon)?;
                truthy(&v)
            };
            if run {
                done = true;
                let fl = exec_block(body.s, body.e)?;
                match fl {
                    Flow::Normal => {}
                    other => return Ok((other, body.after)),
                }
            }
        }
        if is_else {
            return Ok((Flow::Normal, body.after));
        }
        let mut p = body.after;
        let mut found = false;
        while p < end {
            let le = line_end(p);
            let (n, s2) = indent_of(p, le);
            let e2 = code_end(p, le);
            if s2 >= e2 {
                p = next_line(le);
                continue;
            }
            if n == ind {
                let w2 = ident_end(s2, e2);
                if word_is(s2, w2, b"elif") || word_is(s2, w2, b"else") {
                    ccs = s2;
                    cce = e2;
                    cnext = next_line(le);
                    found = true;
                }
            }
            break;
        }
        if !found {
            return Ok((Flow::Normal, body.after));
        }
    }
}
 
fn do_while(cs: usize, ce: usize, ind: usize, next: usize, end: usize) -> Res<(Flow, usize)> {
    let hs = cs + 5;
    let colon = match find_colon(hs, ce) {
        Some(x) => x,
        None => return Err("SyntaxError: expected ':'"),
    };
    let body = parse_body(colon, ce, next, end, ind);
    loop {
        step()?;
        unsafe { CUR_POS = cs; }
        let v = eval_range(hs, colon)?;
        if !truthy(&v) { break; }
        match exec_block(body.s, body.e)? {
            Flow::Break => break,
            Flow::Return(r) => return Ok((Flow::Return(r), body.after)),
            _ => {}
        }
    }
    Ok((Flow::Normal, body.after))
}
 
fn do_for(cs: usize, ce: usize, ind: usize, next: usize, end: usize) -> Res<(Flow, usize)> {
    let vs = skip_sp(cs + 3, ce);
    let ve = ident_end(vs, ce);
    if ve == vs { return Err("SyntaxError: loop variable expected"); }
    let k = skip_sp(ve, ce);
    let kw_e = ident_end(k, ce);
    if !word_is(k, kw_e, b"in") { return Err("SyntaxError: expected 'in'"); }
    let colon = match find_colon(kw_e, ce) {
        Some(x) => x,
        None => return Err("SyntaxError: expected ':'"),
    };
    let body = parse_body(colon, ce, next, end, ind);
    let es = skip_sp(kw_e, colon);
    let mut ee = colon;
    while ee > es && (pb(ee - 1) == b' ' || pb(ee - 1) == b'\t') {
        ee -= 1;
    }
    unsafe { CUR_POS = cs; }
 
    let we = ident_end(es, ee);
    if word_is(es, we, b"range") && pb(we) == b'(' {
        let mut c = Cur { i: we, e: ee, skip: false };
        let a = parse_args(&mut c)?;
        let (start, stop, stp) = if a.pc == 1 {
            (0i64, int_of(&a.pos[0])?, 1i64)
        } else if a.pc == 2 {
            (int_of(&a.pos[0])?, int_of(&a.pos[1])?, 1i64)
        } else if a.pc == 3 {
            (int_of(&a.pos[0])?, int_of(&a.pos[1])?, int_of(&a.pos[2])?)
        } else {
            return Err("TypeError: range expected 1 to 3 arguments");
        };
        if stp == 0 { return Err("ValueError: range() step must not be zero"); }
        let mut i = start;
        loop {
            if (stp > 0 && i >= stop) || (stp < 0 && i <= stop) { break; }
            step()?;
            set_var(vs, ve, Val::Int(i))?;
            match exec_block(body.s, body.e)? {
                Flow::Break => break,
                Flow::Return(r) => return Ok((Flow::Return(r), body.after)),
                _ => {}
            }
            i = i.wrapping_add(stp);
        }
    } else {
        let v = eval_range(es, ee)?;
        match v {
            Val::Str(p) => {
                let mut idx = 0;
                while idx < p.len {
                    step()?;
                    let mut o = PStr::new();
                    o.push(p.b[idx]);
                    set_var(vs, ve, Val::Str(o))?;
                    match exec_block(body.s, body.e)? {
                        Flow::Break => break,
                        Flow::Return(r) => return Ok((Flow::Return(r), body.after)),
                        _ => {}
                    }
                    idx += 1;
                }
            }
            _ => return Err("TypeError: object is not iterable (use range() or a string)"),
        }
    }
    Ok((Flow::Normal, body.after))
}
 
 
 
fn reset_exec_state() {
    unsafe {
        CALL_DEPTH = 0;
        LOCAL_BASE = 0;
        LOCAL_COUNT = 0;
        DECL_BASE = 0;
        DECL_COUNT = 0;
        EXPR_DEPTH = 0;
        STEPS = 0;
    }
}
 
fn reset_runtime() {
    reset_exec_state();
    unsafe {
        GLOBAL_COUNT = 0;
        FUNC_COUNT = 0;
        WIDGET_COUNT = 0;
        ACTIVE_WIN = usize::MAX;
        ROOT_WIN = usize::MAX;
        TK_USED = false;
        CUR_POS = 0;
        ERR_NS = 0;
        ERR_NE = 0;
    }
}
 
fn report_error(msg: &str) {
    unsafe {
        let mut line = 1usize;
        let mut i = 0;
        while i < CUR_POS && i < PROG_LEN {
            if PROG[i] == b'\n' { line += 1; }
            i += 1;
        }
        let mut p = PStr::new();
        push_int(&mut p, line as i64);
        con_put_str("Error, line ");
        con_put_bytes(p.as_bytes());
        con_put_str(": ");
        if msg == "NameError" {
            con_put_str("NameError: name '");
            let mut k = ERR_NS;
            while k < ERR_NE {
                con_put_byte(pb(k));
                k += 1;
            }
            con_put_str("' is not defined");
        } else {
            con_put_str(msg);
        }
        con_put_byte(b'\n');
    }
}
 
fn exec_program() -> &'static str {
    unsafe {
        reset_runtime();
        con_clear();
        RUN_STARTED = true;
 
        let r = exec_block(0, PROG_LEN);
        match r {
            Ok(_) => {
                PREVIEW_RUNNING = TK_USED;
                "OK"
            }
            Err(m) => {
                report_error(m);
                ACTIVE_WIN = usize::MAX;
                PREVIEW_RUNNING = false;
                m
            }
        }
    }
}
 
pub fn run_code() -> &'static str {
    unsafe {
        let mut i = 0;
        while i < CODE_LEN {
            PROG[i] = CODE_BUFFER[i];
            i += 1;
        }
        PROG_LEN = CODE_LEN;
        FILE_RUN = false;
 
        exec_program()
    }
}
 
pub fn run_file_bytes(data: &[u8]) -> &'static str {
    unsafe {
        FILE_RUN = true;
        let mut n = 0usize;
        let mut i = if data.len() >= 3 && data[0] == 0xEF && data[1] == 0xBB && data[2] == 0xBF { 3 } else { 0 };
        while i < data.len() {
            let b = data[i];
            i += 1;
            if b == b'\r' { continue; }
            let w = if b == b'\t' { 4 } else { 1 };
            if n + w > CODE_MAX {
                console_message("Error: file too big (max 4096 bytes)");
                return "Too big";
            }
            if b == b'\t' {
                let mut k = 0;
                while k < 4 { PROG[n] = b' '; n += 1; k += 1; }
            } else {
                PROG[n] = b;
                n += 1;
            }
        }
        PROG_LEN = n;
        let r = exec_program();
        PREVIEW_RUNNING = false;
        r
    }
}
 
pub fn console_message(msg: &str) {
    unsafe {
        reset_runtime();
        con_clear();
        RUN_STARTED = true;
        FILE_RUN = true;
        PREVIEW_RUNNING = false;
        con_put_str(msg);
        con_put_byte(b'\n');
    }
}
 
pub fn console_note(msg: &str) {
    con_put_str(msg);
    con_put_byte(b'\n');
}
 
pub fn code_bytes() -> &'static [u8] {
    unsafe { &CODE_BUFFER[..CODE_LEN] }
}
 
 
 
fn get_col_at_pos(pos: usize) -> usize {
    unsafe {
        let mut col = 0;
        let mut i = pos;
        while i > 0 && i <= CODE_LEN {
            i -= 1;
            if CODE_BUFFER[i] == b'\n' { break; }
            col += 1;
        }
        col
    }
}
 
fn get_line_at_pos(pos: usize) -> usize {
    unsafe {
        let mut line = 0;
        let mut i = 0;
        while i < pos && i < CODE_LEN {
            if CODE_BUFFER[i] == b'\n' { line += 1; }
            i += 1;
        }
        line
    }
}
 
fn num_to_str(num: usize) -> &'static str {
    static mut BUF: [u8; 10] = [0; 10];
    unsafe {
        let mut temp = num;
        let mut digits = [0u8; 10];
        let mut cnt = 0;
        if temp == 0 {
            BUF[0] = b'0'; BUF[1] = 0;
            return "0";
        }
        while temp > 0 && cnt < 10 {
            digits[cnt] = b'0' + (temp % 10) as u8;
            temp /= 10; cnt += 1;
        }
        let mut j = 0;
        while cnt > 0 {
            cnt -= 1;
            BUF[j] = digits[cnt]; j += 1;
        }
        BUF[j] = 0;
        core::str::from_utf8(&BUF[..j]).unwrap_or("0")
    }
}
 
 
pub fn handle_keyboard(sc: u8) -> bool {
    unsafe {
        match sc {
            0x4B => { if CURSOR_POS > 0 { CURSOR_POS -= 1; return true; } return false; }
            0x4D => { if CURSOR_POS < CODE_LEN { CURSOR_POS += 1; return true; } return false; }
            0x48 => {
                let mut pos = CURSOR_POS;
                while pos > 0 && CODE_BUFFER[pos - 1] != b'\n' { pos -= 1; }
                if pos > 0 {
                    pos -= 1;
                    let col = get_col_at_pos(CURSOR_POS);
                    let mut ls = pos;
                    while ls > 0 && CODE_BUFFER[ls - 1] != b'\n' { ls -= 1; }
                    let ll = pos - ls;
                    CURSOR_POS = ls + if col < ll { col } else { ll };
                    return true;
                }
                return false;
            }
            0x50 => {
                let mut pos = CURSOR_POS;
                while pos < CODE_LEN && CODE_BUFFER[pos] != b'\n' { pos += 1; }
                if pos < CODE_LEN {
                    pos += 1;
                    let col = get_col_at_pos(CURSOR_POS);
                    let mut ll = 0;
                    while pos + ll < CODE_LEN && CODE_BUFFER[pos + ll] != b'\n' { ll += 1; }
                    CURSOR_POS = pos + if col < ll { col } else { ll };
                    return true;
                }
                return false;
            }
            0x0F => {
                if CODE_LEN + 4 <= 4095 {
                    for i in (CURSOR_POS..CODE_LEN).rev() {
                        CODE_BUFFER[i + 4] = CODE_BUFFER[i];
                    }
                    for k in 0..4 {
                        CODE_BUFFER[CURSOR_POS + k] = b' ';
                    }
                    CODE_LEN += 4;
                    CURSOR_POS += 4;
                    return true;
                }
                return false;
            }
            _ => {}
        }
 
        let c = match sc {
            0x1E => Some('a'), 0x30 => Some('b'), 0x2E => Some('c'),
            0x20 => Some('d'), 0x12 => Some('e'), 0x21 => Some('f'),
            0x22 => Some('g'), 0x23 => Some('h'), 0x17 => Some('i'),
            0x24 => Some('j'), 0x25 => Some('k'), 0x26 => Some('l'),
            0x32 => Some('m'), 0x31 => Some('n'), 0x18 => Some('o'),
            0x19 => Some('p'), 0x10 => Some('q'), 0x13 => Some('r'),
            0x1F => Some('s'), 0x14 => Some('t'), 0x16 => Some('u'),
            0x2F => Some('v'), 0x11 => Some('w'), 0x2D => Some('x'),
            0x15 => Some('y'), 0x2C => Some('z'),
            0x39 => Some(' '),
            0x1C => Some('\n'),
            0x0E => Some('\x08'),
            0x02 => Some('1'), 0x03 => Some('2'), 0x04 => Some('3'),
            0x05 => Some('4'), 0x06 => Some('5'), 0x07 => Some('6'),
            0x08 => Some('7'), 0x09 => Some('8'), 0x0A => Some('9'),
            0x0B => Some('0'),
            0x33 => Some(','), 0x34 => Some('.'), 0x35 => Some('/'),
            0x27 => Some(';'), 0x28 => Some('\''), 0x2B => Some('\\'),
            0x29 => Some('`'), 0x0C => Some('-'), 0x0D => Some('='),
            0x1A => Some('['), 0x1B => Some(']'),
            _ => None,
        };
        let c = c.map(crate::mouse::apply_shift);
 
        if let Some(ch) = c {
            if ch == '\x08' {
                if CURSOR_POS > 0 && CODE_LEN > 0 {
                    for i in CURSOR_POS..CODE_LEN {
                        CODE_BUFFER[i - 1] = CODE_BUFFER[i];
                    }
                    CODE_LEN -= 1;
                    CURSOR_POS -= 1;
                    return true;
                }
            } else if ch == '\n' {
                if CODE_LEN < 4095 {
                    for i in (CURSOR_POS..CODE_LEN).rev() {
                        CODE_BUFFER[i + 1] = CODE_BUFFER[i];
                    }
                    CODE_BUFFER[CURSOR_POS] = b'\n';
                    CODE_LEN += 1;
                    CURSOR_POS += 1;
                    return true;
                }
            } else if (ch as u8) >= 0x20 {
                if CODE_LEN < 4095 {
                    for i in (CURSOR_POS..CODE_LEN).rev() {
                        CODE_BUFFER[i + 1] = CODE_BUFFER[i];
                    }
                    CODE_BUFFER[CURSOR_POS] = ch as u8;
                    CODE_LEN += 1;
                    CURSOR_POS += 1;
                    return true;
                }
            }
        }
        false
    }
}
 
 
 
fn widget_size(kind: u8, tlen: usize) -> (usize, usize) {
    match kind {
        0 => (tlen * 8, 16),
        1 => (tlen * 8 + 30, 28),
        2 => (180, 24),
        3 => (28 + tlen * 8, 20),
        _ => (0, 0),
    }
}
 
fn step_after(kind: u8) -> usize {
    match kind {
        0 => 25,
        1 => 35,
        2 => 30,
        3 => 25,
        _ => 0,
    }
}
 
fn compute_layout(win: usize, rects: &mut [Rect; MAX_WIDGETS]) {
    unsafe {
        let mut y = 30usize;
        let mut rx = 20usize;
        let mut rh = 0usize;
        let mut in_row = false;
        let mut i = 0;
        while i < WIDGET_COUNT {
            rects[i] = EMPTY_RECT;
            let w = &WIDGETS[i];
            if w.used && w.alive && w.kind != 4 && w.win == win && w.layout != 0 {
                let (bw, bh) = widget_size(w.kind, w.text.len);
                if w.layout == 1 {
                    if w.side_left {
                        rects[i] = Rect { x: rx, y: y, w: bw, h: bh, ok: true };
                        rx += bw + 10;
                        let sa = step_after(w.kind);
                        if sa > rh { rh = sa; }
                        in_row = true;
                    } else {
                        if in_row {
                            y += rh;
                            rx = 20;
                            rh = 0;
                            in_row = false;
                        }
                        rects[i] = Rect { x: 20, y: y, w: bw, h: bh, ok: true };
                        y += step_after(w.kind);
                    }
                } else if w.layout == 2 {
                    rects[i] = Rect { x: 20 + w.gx * 130, y: 30 + w.gy * 38, w: bw, h: bh, ok: true };
                } else {
                    rects[i] = Rect { x: w.gx, y: 25 + w.gy, w: bw, h: bh, ok: true };
                }
            }
            i += 1;
        }
    }
}
 
 
pub fn handle_mouse(cursor_x: usize, cursor_y: usize, wm: &mut WindowManager) -> bool {
    unsafe {
        let active = match wm.active_window {
            Some(a) => a,
            None => return false,
        };
        let win = match wm.windows[active].as_ref() {
            Some(w) => w,
            None => return false,
        };
 
        if win.title == "Cre Tool" && win.state != WindowState::Minimized {
            let wx = win.x + 2;
            let wy = win.y + 26;
            let ww = win.width - 4;
            let btn_x = wx + ww - 90;
            let btn_y = wy + 4;
            if cursor_x >= btn_x && cursor_x <= btn_x + 80 &&
               cursor_y >= btn_y && cursor_y <= btn_y + 22 {
                run_code();
                return true;
            }
        }
 
        if win.title == "Preview" && win.state != WindowState::Minimized {
            let wx = win.x + 2;
            let wy = win.y + 26;
            let ww = win.width - 4;
 
            if ACTIVE_WIN == usize::MAX || ACTIVE_WIN >= WIDGET_COUNT || !WIDGETS[ACTIVE_WIN].alive {
                return false;
            }
            let aw = ACTIVE_WIN;
 
            if aw != ROOT_WIN {
                let bx = wx + ww - 24;
                if cursor_x >= bx && cursor_x <= bx + 20 &&
                   cursor_y >= wy + 3 && cursor_y <= wy + 21 {
                    destroy_widget(aw);
                    return true;
                }
            }
 
            let mut rects = [EMPTY_RECT; MAX_WIDGETS];
            compute_layout(aw, &mut rects);
            let mut i = 0;
            while i < WIDGET_COUNT {
                if rects[i].ok {
                    let r = rects[i];
                    let el_x = wx + r.x;
                    let el_y = wy + r.y;
                    if cursor_x >= el_x && cursor_x <= el_x + r.w &&
                       cursor_y >= el_y && cursor_y <= el_y + r.h {
                        let kind = WIDGETS[i].kind;
                        if kind == 1 {
                            run_widget_command(i);
                            return true;
                        }
                        if kind == 3 {
                            WIDGETS[i].checked = !WIDGETS[i].checked;
                            run_widget_command(i);
                            return true;
                        }
                        return false;
                    }
                }
                i += 1;
            }
            return false;
        }
 
        false
    }
}
 
 
 
fn starts_word(t: &[u8], kw: &[u8]) -> bool {
    if t.len() < kw.len() { return false; }
    let mut k = 0;
    while k < kw.len() {
        if t[k] != kw[k] { return false; }
        k += 1;
    }
    if t.len() > kw.len() && is_ident_char(t[kw.len()]) { return false; }
    true
}
 
fn line_color(text: &str) -> u32 {
    let b = text.as_bytes();
    let mut s = 0;
    while s < b.len() && b[s] == b' ' { s += 1; }
    let t = &b[s..];
    if t.len() > 0 && t[0] == b'#' {
        Color::rgb(110, 170, 110).to_u32()
    } else if starts_word(t, b"def") || starts_word(t, b"return") || starts_word(t, b"global") {
        Color::rgb(255, 100, 100).to_u32()
    } else if starts_word(t, b"if") || starts_word(t, b"elif") || starts_word(t, b"else")
        || starts_word(t, b"while") || starts_word(t, b"for") || starts_word(t, b"break")
        || starts_word(t, b"continue") || starts_word(t, b"pass") {
        Color::rgb(255, 200, 100).to_u32()
    } else if starts_word(t, b"import") || starts_word(t, b"from") {
        Color::rgb(200, 200, 255).to_u32()
    } else if starts_word(t, b"print") {
        Color::rgb(100, 200, 255).to_u32()
    } else {
        Color::rgb(150, 150, 170).to_u32()
    }
}
 
pub fn draw_cre_tool(gfx: &mut Graphics, win_x: usize, win_y: usize, win_w: usize, win_h: usize) {
    unsafe {
        TICK_COUNTER = TICK_COUNTER.wrapping_add(1);
 
        gfx.fill_rect(win_x, win_y, win_w, win_h, Color::rgb(30, 30, 35).to_u32());
        gfx.fill_rect(win_x, win_y, win_w, 30, Color::rgb(50, 50, 55).to_u32());
        gfx.draw_text(win_x + 10, win_y + 8, "Python Editor",
                      Color::rgb(200, 200, 220).to_u32(),
                      Color::rgb(50, 50, 55).to_u32());
 
        let btn_x = win_x + win_w - 90;
        let btn_y = win_y + 4;
        gfx.fill_rect(btn_x, btn_y, 80, 22, Color::rgb(0, 150, 0).to_u32());
        gfx.draw_rect_border(btn_x, btn_y, 80, 22, Color::rgb(0, 200, 0).to_u32());
        gfx.draw_text(btn_x + 25, btn_y + 5, "Run",
                      Color::WHITE.to_u32(),
                      Color::rgb(0, 150, 0).to_u32());
 
        let sv_x = btn_x - 90;
        gfx.fill_rect(sv_x, btn_y, 80, 22, Color::rgb(0, 90, 170).to_u32());
        gfx.draw_rect_border(sv_x, btn_y, 80, 22, Color::rgb(0, 130, 220).to_u32());
        gfx.draw_text(sv_x + 20, btn_y + 5, "Save",
                      Color::WHITE.to_u32(),
                      Color::rgb(0, 90, 170).to_u32());
 
        let code_x = win_x + 5;
        let code_y = win_y + 35;
        let code_w = win_w.saturating_sub(10);
        let code_h = win_h.saturating_sub(35 + CONSOLE_H + 8);
        let code_bg = Color::rgb(20, 20, 25).to_u32();
 
        gfx.fill_rect(code_x, code_y, code_w, code_h, code_bg);
        gfx.draw_rect_border(code_x, code_y, code_w, code_h, Color::rgb(80, 80, 90).to_u32());
 
        let max_lines = if code_h > 10 { (code_h - 10) / 16 } else { 0 };
        let max_chars = if code_w > 45 { (code_w - 45) / 8 } else { 0 };
 
        let cl = get_line_at_pos(CURSOR_POS);
        let first_line = if max_lines > 0 && cl >= max_lines { cl - max_lines + 1 } else { 0 };
 
        let mut line_y = code_y + 5;
        let mut line_idx = 0usize;
        let mut i = 0;
 
        while i < CODE_LEN && line_idx < first_line + max_lines {
            let start = i;
            while i < CODE_LEN && CODE_BUFFER[i] != b'\n' { i += 1; }
            let end = i;
            if i < CODE_LEN && CODE_BUFFER[i] == b'\n' { i += 1; }
 
            if line_idx >= first_line {
                let num_str = num_to_str(line_idx + 1);
                gfx.draw_text(code_x + 5, line_y, num_str,
                              Color::rgb(80, 80, 90).to_u32(), code_bg);
 
                if end > start {
                    let shown_end = if end - start > max_chars { start + max_chars } else { end };
                    let text = core::str::from_utf8(&CODE_BUFFER[start..shown_end]).unwrap_or("");
                    let color = line_color(text);
                    gfx.draw_text(code_x + 35, line_y, text, color, code_bg);
                }
                line_y += 16;
            }
            line_idx += 1;
        }
 
        if TICK_COUNTER % 20 < 10 && cl >= first_line && max_lines > 0 {
            let cc = get_col_at_pos(CURSOR_POS);
            let cy = code_y + 5 + (cl - first_line) * 16;
            let cx = code_x + 35 + cc * 8;
            if cy + 14 < code_y + code_h && cx + 2 < code_x + code_w {
                gfx.fill_rect(cx, cy, 2, 14, Color::rgb(255, 255, 255).to_u32());
            }
        }
 
        let con_y = code_y + code_h + 4;
        let con_bg = Color::rgb(10, 10, 12).to_u32();
        gfx.fill_rect(code_x, con_y, code_w, CONSOLE_H, con_bg);
        gfx.draw_rect_border(code_x, con_y, code_w, CONSOLE_H, Color::rgb(80, 80, 90).to_u32());
        gfx.draw_text(code_x + 5, con_y + 3, "mkfs v1.1.0 python3",
                      Color::rgb(14, 82, 17).to_u32(), con_bg);
 
        let vis = (CONSOLE_H - 22) / 16;
        let mut shown = CON_COUNT;
        if shown > 0 && CON_LEN[shown - 1] == 0 { shown -= 1; }
        let first = if shown > vis { shown - vis } else { 0 };
        let con_chars = if code_w > 12 { (code_w - 12) / 8 } else { 0 };
        let mut k = first;
        while k < shown {
            let n = if CON_LEN[k] > con_chars { con_chars } else { CON_LEN[k] };
            if n > 0 {
                let text = core::str::from_utf8(&CON[k][..n]).unwrap_or("");
                gfx.draw_text(code_x + 6, con_y + 20 + (k - first) * 16, text,
                              Color::rgb(200, 255, 200).to_u32(), con_bg);
            }
            k += 1;
        }
    }
}
 
 
pub fn draw_preview(gfx: &mut Graphics, win_x: usize, win_y: usize, win_w: usize, win_h: usize) {
    unsafe {
        let bg = Color::rgb(240, 240, 240).to_u32();
        gfx.fill_rect(win_x, win_y, win_w, win_h, bg);
 
        if ACTIVE_WIN == usize::MAX || ACTIVE_WIN >= WIDGET_COUNT || !WIDGETS[ACTIVE_WIN].alive {
            if FILE_RUN && RUN_STARTED {
                let con_bg = Color::rgb(10, 10, 12).to_u32();
                gfx.fill_rect(win_x, win_y, win_w, win_h, con_bg);
                let vis = if win_h > 8 { (win_h - 8) / 16 } else { 0 };
                let mut shown = CON_COUNT;
                if shown > 0 && CON_LEN[shown - 1] == 0 { shown -= 1; }
                let first = if shown > vis { shown - vis } else { 0 };
                let con_chars = if win_w > 12 { (win_w - 12) / 8 } else { 0 };
                let mut k = first;
                while k < shown {
                    let n = if CON_LEN[k] > con_chars { con_chars } else { CON_LEN[k] };
                    if n > 0 {
                        let text = core::str::from_utf8(&CON[k][..n]).unwrap_or("");
                        gfx.draw_text(win_x + 6, win_y + 4 + (k - first) * 16, text,
                                      Color::rgb(200, 255, 200).to_u32(), con_bg);
                    }
                    k += 1;
                }
                return;
            }
            let msg = if RUN_STARTED { "No tkinter window" } else { "Press Run to start" };
            gfx.draw_text(win_x + 20, win_y + 30, msg,
                          Color::rgb(100, 100, 100).to_u32(), bg);
            return;
        }
 
        let aw = ACTIVE_WIN;
        let blue = Color::rgb(0, 80, 180).to_u32();
        gfx.fill_rect(win_x, win_y, win_w, 25, blue);
        let title = text_of(&WIDGETS[aw].text);
        gfx.draw_text(win_x + 10, win_y + 5, title, Color::WHITE.to_u32(), blue);
 
        if aw != ROOT_WIN && win_w > 30 {
            let bx = win_x + win_w - 24;
            gfx.fill_rect(bx, win_y + 3, 20, 18, Color::rgb(190, 50, 50).to_u32());
            gfx.draw_text(bx + 6, win_y + 5, "x", Color::WHITE.to_u32(), Color::rgb(190, 50, 50).to_u32());
        }
 
        let mut rects = [EMPTY_RECT; MAX_WIDGETS];
        compute_layout(aw, &mut rects);
 
        let mut i = 0;
        while i < WIDGET_COUNT {
            if rects[i].ok {
                let r = rects[i];
                let ex = win_x + r.x;
                let ey = win_y + r.y;
                if ey + r.h <= win_y + win_h {
                    let text = text_of(&WIDGETS[i].text);
                    match WIDGETS[i].kind {
                        0 => {
                            gfx.draw_text(ex, ey, text, Color::BLACK.to_u32(), bg);
                        }
                        1 => {
                            let bw = r.w;
                            let bh = r.h;
                            gfx.fill_rect(ex, ey, bw, bh, Color::rgb(212, 208, 200).to_u32());
                            gfx.draw_rect_border(ex, ey, bw, bh, Color::rgb(128, 128, 128).to_u32());
                            gfx.fill_rect(ex + 1, ey + 1, bw - 2, 1, Color::WHITE.to_u32());
                            gfx.fill_rect(ex + 1, ey + 1, 1, bh - 2, Color::WHITE.to_u32());
                            gfx.draw_text(ex + 10, ey + 7, text,
                                          Color::BLACK.to_u32(),
                                          Color::rgb(212, 208, 200).to_u32());
                        }
                        2 => {
                            gfx.fill_rect(ex, ey, r.w, r.h, Color::WHITE.to_u32());
                            gfx.draw_rect_border(ex, ey, r.w, r.h, Color::rgb(128, 128, 128).to_u32());
                            let max_c = if r.w > 10 { (r.w - 10) / 8 } else { 0 };
                            let n = if WIDGETS[i].text.len > max_c { max_c } else { WIDGETS[i].text.len };
                            let shown = core::str::from_utf8(&WIDGETS[i].text.b[..n]).unwrap_or("");
                            gfx.draw_text(ex + 5, ey + 5, shown,
                                          Color::BLACK.to_u32(), Color::WHITE.to_u32());
                        }
                        3 => {
                            let fs = 20;
                            gfx.draw_rect_border(ex, ey, fs, fs, Color::rgb(80, 80, 80).to_u32());
                            if WIDGETS[i].checked {
                                gfx.fill_rect(ex + 3, ey + 3, fs - 6, fs - 6, Color::rgb(0, 150, 0).to_u32());
                                let cc = Color::WHITE.to_u32();
                                draw_line_internal(gfx, ex + 5, ey + 10, ex + 9, ey + 14, cc);
                                draw_line_internal(gfx, ex + 9, ey + 14, ex + 16, ey + 5, cc);
                            }
                            gfx.draw_text(ex + fs + 8, ey + 3, text, Color::BLACK.to_u32(), bg);
                        }
                        _ => {}
                    }
                }
            }
            i += 1;
        }
    }
}
 
 
 
const DEMO_CODE: &str = concat!(
    "import tkinter as tk\n",
    "\n",
    "count = 0\n",
    "\n",
    "def hello():\n",
    "    global count\n",
    "    count = count + 1\n",
    "    print(\"clicked\", count)\n",
    "    lbl.config(text=\"Clicks: \" + str(count))\n",
    "\n",
    "root = tk.Tk()\n",
    "root.title(\"My app\")\n",
    "lbl = tk.Label(root, text=\"Clicks: 0\")\n",
    "lbl.pack()\n",
    "btn = tk.Button(root, text=\"Click me\", command=hello)\n",
    "btn.pack()\n",
    "\n",
    "print(\"Hello from Python!\")\n",
    "for i in range(3):\n",
    "    print(\"i =\", i)\n",
    "root.mainloop()\n",
);
 
pub fn cre_tool_start() -> &'static str {
    unsafe {
        if CODE_LEN == 0 {
            let bytes = DEMO_CODE.as_bytes();
            let len = bytes.len().min(4095);
            CODE_BUFFER[..len].copy_from_slice(&bytes[..len]);
            CODE_LEN = len;
            CURSOR_POS = len;
        }
    }
    "Python ready!"
}
 
pub fn cre_tool_stop() -> &'static str {
    "Python stopped!"
}
 
pub fn is_preview_running() -> bool {
    unsafe {
        let r = PREVIEW_RUNNING;
        PREVIEW_RUNNING = false;
        r
    }
}
 
pub fn set_preview_window(idx: usize) {
    unsafe { PREVIEW_WINDOW = idx; }
}
 
pub fn get_preview_window() -> usize {
    unsafe { PREVIEW_WINDOW }
}