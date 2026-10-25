use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::vec;
use alloc::rc::Rc;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::format;
use core::cell::RefCell;

const MAX_STEPS: u64 = 3_000_000;
const MAX_DEPTH: u32 = 48;


#[derive(Clone, Debug, PartialEq)]
enum Tok { Num(f64), Str(String), Tmpl(String), Id(String), P(&'static str), Eof }

static PUNCTS: &[&str] = &[
    "===", "!==", "**=", "...", "==", "!=", "<=", ">=", "&&", "||", "??", "++", "--", "+=", "-=", "*=", "/=", "%=", "=>", "<<", ">>", "**",
    "+", "-", "*", "/", "%", "=", "<", ">", "!", "&", "|", "^", "~", "?", ":", ";", ",", ".", "(", ")", "{", "}", "[", "]",
];

fn lex(src: &str) -> Result<Vec<Tok>, String> {
    let b: Vec<char> = src.chars().collect();
    let mut i = 0usize;
    let mut out: Vec<Tok> = Vec::new();
    while i < b.len() {
        let c = b[i];
        if c.is_whitespace() { i += 1; continue; }
        if c == '/' && i + 1 < b.len() && b[i + 1] == '/' {
            while i < b.len() && b[i] != '\n' { i += 1; }
            continue;
        }
        if c == '/' && i + 1 < b.len() && b[i + 1] == '*' {
            i += 2;
            while i + 1 < b.len() && !(b[i] == '*' && b[i + 1] == '/') { i += 1; }
            i += 2;
            continue;
        }
        if c.is_ascii_digit() || (c == '.' && i + 1 < b.len() && b[i + 1].is_ascii_digit()) {
            let st = i;
            if c == '0' && i + 1 < b.len() && (b[i + 1] == 'x' || b[i + 1] == 'X') {
                i += 2;
                let mut v = 0.0f64;
                while i < b.len() && b[i].is_ascii_hexdigit() {
                    v = v * 16.0 + b[i].to_digit(16).unwrap_or(0) as f64;
                    i += 1;
                }
                out.push(Tok::Num(v));
                continue;
            }
            while i < b.len() && (b[i].is_ascii_digit() || b[i] == '.') { i += 1; }
            if i < b.len() && (b[i] == 'e' || b[i] == 'E') {
                i += 1;
                if i < b.len() && (b[i] == '+' || b[i] == '-') { i += 1; }
                while i < b.len() && b[i].is_ascii_digit() { i += 1; }
            }
            let s: String = b[st..i].iter().collect();
            out.push(Tok::Num(s.parse::<f64>().unwrap_or(f64::NAN)));
            continue;
        }
        if c.is_alphabetic() || c == '_' || c == '$' {
            let st = i;
            while i < b.len() && (b[i].is_alphanumeric() || b[i] == '_' || b[i] == '$') { i += 1; }
            out.push(Tok::Id(b[st..i].iter().collect()));
            continue;
        }
        if c == '"' || c == '\'' {
            i += 1;
            let mut s = String::new();
            while i < b.len() && b[i] != c {
                if b[i] == '\\' && i + 1 < b.len() {
                    i += 1;
                    match b[i] {
                        'n' => s.push('\n'),
                        't' => s.push('\t'),
                        'r' => s.push('\r'),
                        '0' => s.push('\0'),
                        'u' => {
                            let mut v = 0u32;
                            let mut k = 0;
                            while k < 4 && i + 1 < b.len() && b[i + 1].is_ascii_hexdigit() {
                                v = v * 16 + b[i + 1].to_digit(16).unwrap_or(0);
                                i += 1;
                                k += 1;
                            }
                            s.push(core::char::from_u32(v).unwrap_or('?'));
                        }
                        other => s.push(other),
                    }
                    i += 1;
                } else {
                    s.push(b[i]);
                    i += 1;
                }
            }
            if i >= b.len() { return Err("незакрытая строка".to_string()); }
            i += 1;
            out.push(Tok::Str(s));
            continue;
        }
        if c == '`' {
            i += 1;
            let mut raw = String::new();
            let mut depth = 0i32;
            while i < b.len() && !(b[i] == '`' && depth == 0) {
                if b[i] == '\\' && i + 1 < b.len() {
                    raw.push(b[i]);
                    raw.push(b[i + 1]);
                    i += 2;
                    continue;
                }
                if b[i] == '$' && i + 1 < b.len() && b[i + 1] == '{' { depth += 1; raw.push('$'); raw.push('{'); i += 2; continue; }
                if b[i] == '}' && depth > 0 { depth -= 1; }
                raw.push(b[i]);
                i += 1;
            }
            if i >= b.len() { return Err("незакрытый шаблон".to_string()); }
            i += 1;
            out.push(Tok::Tmpl(raw));
            continue;
        }
        let end = core::cmp::min(i + 3, b.len());
        let rest: String = b[i..end].iter().collect();
        let mut found: Option<&'static str> = None;
        for p in PUNCTS.iter() {
            if rest.starts_with(*p) { found = Some(*p); break; }
        }
        match found {
            Some(p) => { out.push(Tok::P(p)); i += p.len(); }
            None => return Err(format!("неожиданный символ '{}'", c)),
        }
    }
    Ok(out)
}


struct Func { name: String, params: Vec<String>, body: Vec<Stmt>, arrow: bool }

enum Expr {
    Num(f64),
    Str(String),
    Bool(bool),
    Null,
    Undef,
    This,
    Ident(String),
    Template(Vec<String>, Vec<Expr>),
    Array(Vec<Expr>),
    Object(Vec<(String, Expr)>),
    Func(Rc<Func>),
    Unary(&'static str, Box<Expr>),
    Update(bool, bool, Box<Expr>),
    Bin(&'static str, Box<Expr>, Box<Expr>),
    Logic(u8, Box<Expr>, Box<Expr>),
    Assign(&'static str, Box<Expr>, Box<Expr>),
    Cond(Box<Expr>, Box<Expr>, Box<Expr>),
    Seq(Vec<Expr>),
    Member(Box<Expr>, String),
    Index(Box<Expr>, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    New(Box<Expr>, Vec<Expr>),
}

enum Stmt {
    Empty,
    Expr(Expr),
    Var(Vec<(String, Option<Expr>)>),
    Func(String, Rc<Func>),
    Block(Vec<Stmt>),
    If(Expr, Box<Stmt>, Option<Box<Stmt>>),
    While(Expr, Box<Stmt>),
    DoWhile(Box<Stmt>, Expr),
    For(Option<Box<Stmt>>, Option<Expr>, Option<Expr>, Box<Stmt>),
    ForIn(String, Expr, Box<Stmt>, bool),
    Return(Option<Expr>),
    Break,
    Continue,
    Throw(Expr),
    Try(Vec<Stmt>, Option<(String, Vec<Stmt>)>, Option<Vec<Stmt>>),
    Switch(Expr, Vec<(Option<Expr>, Vec<Stmt>)>),
}


struct Parser { t: Vec<Tok>, p: usize, depth: u32, eof: Tok }

fn prec(op: &str) -> u8 {
    match op {
        "??" => 1, "||" => 2, "&&" => 3, "|" => 4, "^" => 5, "&" => 6,
        "==" | "!=" | "===" | "!==" => 7,
        "<" | ">" | "<=" | ">=" | "instanceof" | "in" => 8,
        "<<" | ">>" => 9,
        "+" | "-" => 10,
        "*" | "/" | "%" => 11,
        "**" => 12,
        _ => 0,
    }
}

fn is_kw(n: &str) -> bool {
    matches!(n, "function" | "new" | "typeof" | "this" | "true" | "false" | "null" | "void" | "delete" | "var" | "let" | "const"
        | "if" | "else" | "for" | "while" | "do" | "return" | "break" | "continue" | "throw" | "try" | "catch" | "finally" | "switch" | "case" | "default")
}

impl Parser {
    fn new(t: Vec<Tok>) -> Parser { Parser { t, p: 0, depth: 0, eof: Tok::Eof } }

    fn peek(&self) -> &Tok { self.t.get(self.p).unwrap_or(&self.eof) }
    fn next(&mut self) -> Tok { let t = self.peek().clone(); self.p += 1; t }
    fn at_p(&self, s: &str) -> bool { matches!(self.peek(), Tok::P(x) if *x == s) }
    fn at_id(&self, s: &str) -> bool { matches!(self.peek(), Tok::Id(x) if x == s) }
    fn eat_p(&mut self, s: &str) -> bool { if self.at_p(s) { self.p += 1; true } else { false } }
    fn eat_id(&mut self, s: &str) -> bool { if self.at_id(s) { self.p += 1; true } else { false } }
    fn expect_p(&mut self, s: &str) -> Result<(), String> {
        if self.eat_p(s) { Ok(()) } else { Err(format!("ожидалось '{}', найдено {:?}", s, self.peek())) }
    }
    fn semi(&mut self) { self.eat_p(";"); }
    fn ident(&mut self) -> Result<String, String> {
        match self.next() { Tok::Id(s) => Ok(s), t => Err(format!("ожидался идентификатор, найдено {:?}", t)) }
    }

    fn program(&mut self) -> Result<Vec<Stmt>, String> {
        let mut v = Vec::new();
        while *self.peek() != Tok::Eof { v.push(self.stmt()?); }
        Ok(v)
    }

    fn block_body(&mut self) -> Result<Vec<Stmt>, String> {
        self.expect_p("{")?;
        let mut v = Vec::new();
        while !self.at_p("}") {
            if *self.peek() == Tok::Eof { return Err("не хватает '}'".to_string()); }
            v.push(self.stmt()?);
        }
        self.expect_p("}")?;
        Ok(v)
    }

    fn params(&mut self) -> Result<Vec<String>, String> {
        self.expect_p("(")?;
        let mut v = Vec::new();
        while !self.at_p(")") {
            v.push(self.ident()?);
            if !self.eat_p(",") { break; }
        }
        self.expect_p(")")?;
        Ok(v)
    }

    fn decls(&mut self) -> Result<Vec<(String, Option<Expr>)>, String> {
        let mut v = Vec::new();
        loop {
            let n = self.ident()?;
            let init = if self.eat_p("=") { Some(self.assign()?) } else { None };
            v.push((n, init));
            if !self.eat_p(",") { break; }
        }
        Ok(v)
    }

    fn stmt(&mut self) -> Result<Stmt, String> {
        self.depth += 1;
        if self.depth > 150 { return Err("слишком глубокая вложенность".to_string()); }
        let r = self.stmt_inner();
        self.depth -= 1;
        r
    }

    fn stmt_inner(&mut self) -> Result<Stmt, String> {
        let tok = self.peek().clone();
        match tok {
            Tok::P("{") => { let b = self.block_body()?; Ok(Stmt::Block(b)) }
            Tok::P(";") => { self.p += 1; Ok(Stmt::Empty) }
            Tok::Id(k) => match k.as_str() {
                "var" | "let" | "const" => {
                    self.p += 1;
                    let d = self.decls()?;
                    self.semi();
                    Ok(Stmt::Var(d))
                }
                "function" => {
                    self.p += 1;
                    let name = self.ident()?;
                    let params = self.params()?;
                    let body = self.block_body()?;
                    Ok(Stmt::Func(name.clone(), Rc::new(Func { name, params, body, arrow: false })))
                }
                "if" => {
                    self.p += 1;
                    self.expect_p("(")?;
                    let c = self.expr()?;
                    self.expect_p(")")?;
                    let t = self.stmt()?;
                    let e = if self.eat_id("else") { Some(Box::new(self.stmt()?)) } else { None };
                    Ok(Stmt::If(c, Box::new(t), e))
                }
                "while" => {
                    self.p += 1;
                    self.expect_p("(")?;
                    let c = self.expr()?;
                    self.expect_p(")")?;
                    let b = self.stmt()?;
                    Ok(Stmt::While(c, Box::new(b)))
                }
                "do" => {
                    self.p += 1;
                    let b = self.stmt()?;
                    if !self.eat_id("while") { return Err("ожидалось while".to_string()); }
                    self.expect_p("(")?;
                    let c = self.expr()?;
                    self.expect_p(")")?;
                    self.semi();
                    Ok(Stmt::DoWhile(Box::new(b), c))
                }
                "for" => self.for_stmt(),
                "return" => {
                    self.p += 1;
                    if self.at_p(";") || self.at_p("}") || *self.peek() == Tok::Eof {
                        self.semi();
                        return Ok(Stmt::Return(None));
                    }
                    let e = self.expr()?;
                    self.semi();
                    Ok(Stmt::Return(Some(e)))
                }
                "break" => { self.p += 1; self.semi(); Ok(Stmt::Break) }
                "continue" => { self.p += 1; self.semi(); Ok(Stmt::Continue) }
                "throw" => { self.p += 1; let e = self.expr()?; self.semi(); Ok(Stmt::Throw(e)) }
                "try" => {
                    self.p += 1;
                    let b = self.block_body()?;
                    let mut c = None;
                    let mut f = None;
                    if self.eat_id("catch") {
                        let mut name = String::new();
                        if self.eat_p("(") { name = self.ident()?; self.expect_p(")")?; }
                        c = Some((name, self.block_body()?));
                    }
                    if self.eat_id("finally") { f = Some(self.block_body()?); }
                    Ok(Stmt::Try(b, c, f))
                }
                "switch" => {
                    self.p += 1;
                    self.expect_p("(")?;
                    let d = self.expr()?;
                    self.expect_p(")")?;
                    self.expect_p("{")?;
                    let mut cases: Vec<(Option<Expr>, Vec<Stmt>)> = Vec::new();
                    while !self.at_p("}") {
                        let test = if self.eat_id("case") {
                            let e = self.expr()?;
                            Some(e)
                        } else if self.eat_id("default") {
                            None
                        } else {
                            return Err("ожидалось case или default".to_string());
                        };
                        self.expect_p(":")?;
                        let mut body = Vec::new();
                        while !self.at_id("case") && !self.at_id("default") && !self.at_p("}") {
                            if *self.peek() == Tok::Eof { return Err("не хватает '}'".to_string()); }
                            body.push(self.stmt()?);
                        }
                        cases.push((test, body));
                    }
                    self.expect_p("}")?;
                    Ok(Stmt::Switch(d, cases))
                }
                _ => { let e = self.expr()?; self.semi(); Ok(Stmt::Expr(e)) }
            },
            _ => { let e = self.expr()?; self.semi(); Ok(Stmt::Expr(e)) }
        }
    }

    fn for_stmt(&mut self) -> Result<Stmt, String> {
        self.p += 1;
        self.expect_p("(")?;
        let mut q = self.p;
        if let Some(Tok::Id(k)) = self.t.get(q) {
            if k == "var" || k == "let" || k == "const" { q += 1; }
        }
        if let (Some(Tok::Id(name)), Some(Tok::Id(kw))) = (self.t.get(q), self.t.get(q + 1)) {
            if kw == "of" || kw == "in" {
                let name = name.clone();
                let is_of = kw == "of";
                self.p = q + 2;
                let e = self.expr()?;
                self.expect_p(")")?;
                let body = self.stmt()?;
                return Ok(Stmt::ForIn(name, e, Box::new(body), is_of));
            }
        }
        let init: Option<Box<Stmt>> = if self.at_p(";") {
            None
        } else if self.at_id("var") || self.at_id("let") || self.at_id("const") {
            self.p += 1;
            Some(Box::new(Stmt::Var(self.decls()?)))
        } else {
            Some(Box::new(Stmt::Expr(self.expr()?)))
        };
        self.expect_p(";")?;
        let cond = if self.at_p(";") { None } else { Some(self.expr()?) };
        self.expect_p(";")?;
        let upd = if self.at_p(")") { None } else { Some(self.expr()?) };
        self.expect_p(")")?;
        let body = self.stmt()?;
        Ok(Stmt::For(init, cond, upd, Box::new(body)))
    }

    fn expr(&mut self) -> Result<Expr, String> {
        let first = self.assign()?;
        if self.at_p(",") {
            let mut v = vec![first];
            while self.eat_p(",") { v.push(self.assign()?); }
            return Ok(Expr::Seq(v));
        }
        Ok(first)
    }

    fn assign(&mut self) -> Result<Expr, String> {
        self.depth += 1;
        if self.depth > 150 { return Err("слишком глубокая вложенность".to_string()); }
        let r = self.assign_inner();
        self.depth -= 1;
        r
    }

    fn try_arrow(&mut self) -> Result<Option<Expr>, String> {
        let params: Vec<String>;
        if let Tok::Id(n) = self.peek().clone() {
            if matches!(self.t.get(self.p + 1), Some(Tok::P("=>"))) && !is_kw(&n) {
                params = vec![n];
                self.p += 2;
            } else {
                return Ok(None);
            }
        } else if self.at_p("(") {
            let mut d = 0i32;
            let mut q = self.p;
            loop {
                match self.t.get(q) {
                    Some(Tok::P("(")) => d += 1,
                    Some(Tok::P(")")) => { d -= 1; if d == 0 { break; } }
                    None | Some(Tok::Eof) => return Ok(None),
                    _ => {}
                }
                q += 1;
            }
            if !matches!(self.t.get(q + 1), Some(Tok::P("=>"))) { return Ok(None); }
            params = self.params()?;
            self.p += 1;
        } else {
            return Ok(None);
        }
        let body = if self.at_p("{") { self.block_body()? } else { let e = self.assign()?; vec![Stmt::Return(Some(e))] };
        Ok(Some(Expr::Func(Rc::new(Func { name: String::new(), params, body, arrow: true }))))
    }

    fn assign_inner(&mut self) -> Result<Expr, String> {
        if let Some(f) = self.try_arrow()? { return Ok(f); }
        let left = self.cond()?;
        if let Tok::P(op) = self.peek().clone() {
            let aop: Option<&'static str> = match op {
                "=" => Some(""), "+=" => Some("+"), "-=" => Some("-"), "*=" => Some("*"), "/=" => Some("/"), "%=" => Some("%"),
                _ => None,
            };
            if let Some(o) = aop {
                self.p += 1;
                let right = self.assign()?;
                return Ok(Expr::Assign(o, Box::new(left), Box::new(right)));
            }
        }
        Ok(left)
    }

    fn cond(&mut self) -> Result<Expr, String> {
        let c = self.binary(1)?;
        if self.eat_p("?") {
            let a = self.assign()?;
            self.expect_p(":")?;
            let b = self.assign()?;
            return Ok(Expr::Cond(Box::new(c), Box::new(a), Box::new(b)));
        }
        Ok(c)
    }

    fn binary(&mut self, min: u8) -> Result<Expr, String> {
        let mut left = self.unary()?;
        loop {
            let op: &'static str = match self.peek() {
                Tok::P(p) => *p,
                Tok::Id(k) if k == "instanceof" => "instanceof",
                Tok::Id(k) if k == "in" => "in",
                _ => break,
            };
            let pr = prec(op);
            if pr == 0 || pr < min { break; }
            self.p += 1;
            let right = if op == "**" { self.binary(pr)? } else { self.binary(pr + 1)? };
            left = match op {
                "&&" => Expr::Logic(0, Box::new(left), Box::new(right)),
                "||" => Expr::Logic(1, Box::new(left), Box::new(right)),
                "??" => Expr::Logic(2, Box::new(left), Box::new(right)),
                _ => Expr::Bin(op, Box::new(left), Box::new(right)),
            };
        }
        Ok(left)
    }

    fn unary(&mut self) -> Result<Expr, String> {
        self.depth += 1;
        if self.depth > 150 { return Err("слишком глубокая вложенность".to_string()); }
        let r = self.unary_inner();
        self.depth -= 1;
        r
    }

    fn unary_inner(&mut self) -> Result<Expr, String> {
        let tok = self.peek().clone();
        match tok {
            Tok::P("!") => { self.p += 1; let x = self.unary()?; Ok(Expr::Unary("!", Box::new(x))) }
            Tok::P("-") => { self.p += 1; let x = self.unary()?; Ok(Expr::Unary("-", Box::new(x))) }
            Tok::P("+") => { self.p += 1; let x = self.unary()?; Ok(Expr::Unary("+", Box::new(x))) }
            Tok::P("~") => { self.p += 1; let x = self.unary()?; Ok(Expr::Unary("~", Box::new(x))) }
            Tok::P("++") => { self.p += 1; let x = self.unary()?; Ok(Expr::Update(true, true, Box::new(x))) }
            Tok::P("--") => { self.p += 1; let x = self.unary()?; Ok(Expr::Update(false, true, Box::new(x))) }
            Tok::Id(k) if k == "typeof" => { self.p += 1; let x = self.unary()?; Ok(Expr::Unary("typeof", Box::new(x))) }
            Tok::Id(k) if k == "void" => { self.p += 1; let x = self.unary()?; Ok(Expr::Unary("void", Box::new(x))) }
            Tok::Id(k) if k == "delete" => { self.p += 1; let x = self.unary()?; Ok(Expr::Unary("delete", Box::new(x))) }
            _ => {
                let e = self.call_member()?;
                if self.at_p("++") { self.p += 1; return Ok(Expr::Update(true, false, Box::new(e))); }
                if self.at_p("--") { self.p += 1; return Ok(Expr::Update(false, false, Box::new(e))); }
                Ok(e)
            }
        }
    }

    fn name(&mut self) -> Result<String, String> {
        match self.next() { Tok::Id(s) => Ok(s), t => Err(format!("ожидалось имя свойства, найдено {:?}", t)) }
    }

    fn args(&mut self) -> Result<Vec<Expr>, String> {
        self.expect_p("(")?;
        let mut v = Vec::new();
        while !self.at_p(")") {
            v.push(self.assign()?);
            if !self.eat_p(",") { break; }
        }
        self.expect_p(")")?;
        Ok(v)
    }

    fn call_member(&mut self) -> Result<Expr, String> {
        let mut e = if self.at_id("new") { self.new_expr()? } else { self.primary()? };
        loop {
            if self.eat_p(".") {
                let n = self.name()?;
                e = Expr::Member(Box::new(e), n);
            } else if self.at_p("[") {
                self.p += 1;
                let i = self.expr()?;
                self.expect_p("]")?;
                e = Expr::Index(Box::new(e), Box::new(i));
            } else if self.at_p("(") {
                let a = self.args()?;
                e = Expr::Call(Box::new(e), a);
            } else {
                break;
            }
        }
        Ok(e)
    }

    fn new_expr(&mut self) -> Result<Expr, String> {
        self.p += 1;
        let mut c = if self.at_id("new") { self.new_expr()? } else { self.primary()? };
        loop {
            if self.eat_p(".") {
                let n = self.name()?;
                c = Expr::Member(Box::new(c), n);
            } else if self.at_p("[") {
                self.p += 1;
                let i = self.expr()?;
                self.expect_p("]")?;
                c = Expr::Index(Box::new(c), Box::new(i));
            } else {
                break;
            }
        }
        let a = if self.at_p("(") { self.args()? } else { Vec::new() };
        Ok(Expr::New(Box::new(c), a))
    }

    fn template(&mut self, raw: &str) -> Result<Expr, String> {
        let cs: Vec<char> = raw.chars().collect();
        let mut strs: Vec<String> = Vec::new();
        let mut exprs: Vec<Expr> = Vec::new();
        let mut cur = String::new();
        let mut i = 0usize;
        while i < cs.len() {
            if cs[i] == '\\' && i + 1 < cs.len() {
                match cs[i + 1] { 'n' => cur.push('\n'), 't' => cur.push('\t'), c => cur.push(c) }
                i += 2;
                continue;
            }
            if cs[i] == '$' && i + 1 < cs.len() && cs[i + 1] == '{' {
                let mut d = 1i32;
                let mut j = i + 2;
                while j < cs.len() && d > 0 {
                    if cs[j] == '{' { d += 1; } else if cs[j] == '}' { d -= 1; }
                    j += 1;
                }
                if d > 0 { return Err("незакрытое ${".to_string()); }
                let inner: String = cs[i + 2..j - 1].iter().collect();
                strs.push(core::mem::take(&mut cur));
                let toks = lex(&inner)?;
                let mut sub = Parser::new(toks);
                sub.depth = self.depth;
                exprs.push(sub.expr()?);
                i = j;
                continue;
            }
            cur.push(cs[i]);
            i += 1;
        }
        strs.push(cur);
        Ok(Expr::Template(strs, exprs))
    }

    fn primary(&mut self) -> Result<Expr, String> {
        let tok = self.next();
        match tok {
            Tok::Num(n) => Ok(Expr::Num(n)),
            Tok::Str(s) => Ok(Expr::Str(s)),
            Tok::Tmpl(raw) => self.template(&raw),
            Tok::Id(k) => match k.as_str() {
                "true" => Ok(Expr::Bool(true)),
                "false" => Ok(Expr::Bool(false)),
                "null" => Ok(Expr::Null),
                "undefined" => Ok(Expr::Undef),
                "this" => Ok(Expr::This),
                "function" => {
                    let name = if let Tok::Id(_) = self.peek() { self.ident()? } else { String::new() };
                    let params = self.params()?;
                    let body = self.block_body()?;
                    Ok(Expr::Func(Rc::new(Func { name, params, body, arrow: false })))
                }
                _ => Ok(Expr::Ident(k.clone())),
            },
            Tok::P("(") => {
                let e = self.expr()?;
                self.expect_p(")")?;
                Ok(e)
            }
            Tok::P("[") => {
                let mut v = Vec::new();
                while !self.at_p("]") {
                    v.push(self.assign()?);
                    if !self.eat_p(",") { break; }
                }
                self.expect_p("]")?;
                Ok(Expr::Array(v))
            }
            Tok::P("{") => {
                let mut props: Vec<(String, Expr)> = Vec::new();
                while !self.at_p("}") {
                    let key = match self.next() {
                        Tok::Id(s) => s,
                        Tok::Str(s) => s,
                        Tok::Num(n) => num_to_str(n),
                        t => return Err(format!("неверный ключ объекта {:?}", t)),
                    };
                    if self.eat_p(":") {
                        let v = self.assign()?;
                        props.push((key, v));
                    } else if self.at_p("(") {
                        let params = self.params()?;
                        let body = self.block_body()?;
                        props.push((key.clone(), Expr::Func(Rc::new(Func { name: key, params, body, arrow: false }))));
                    } else {
                        props.push((key.clone(), Expr::Ident(key)));
                    }
                    if !self.eat_p(",") { break; }
                }
                self.expect_p("}")?;
                Ok(Expr::Object(props))
            }
            t => Err(format!("неожиданный токен {:?}", t)),
        }
    }
}

fn parse_program(src: &str) -> Result<Vec<Stmt>, String> {
    let toks = lex(src)?;
    let mut p = Parser::new(toks);
    p.program()
}


type ObjRef = Rc<RefCell<Obj>>;

#[derive(Clone)]
enum Value { Undef, Null, Bool(bool), Num(f64), Str(String), Obj(ObjRef) }

struct Obj {
    props: BTreeMap<String, Value>,
    arr: Option<Vec<Value>>,
    func: Option<(Rc<Func>, Env)>,
    native: Option<&'static str>,
}

struct Scope { vars: RefCell<BTreeMap<String, Value>>, parent: Option<Env> }
type Env = Rc<Scope>;

enum Ctl { Break, Continue, Return(Value), Throw(Value) }

fn new_scope(parent: &Env) -> Env { Rc::new(Scope { vars: RefCell::new(BTreeMap::new()), parent: Some(parent.clone()) }) }

fn declare(env: &Env, name: &str, v: Value) { env.vars.borrow_mut().insert(name.to_string(), v); }

fn lookup(env: &Env, name: &str) -> Option<Value> {
    let mut cur: Option<Env> = Some(env.clone());
    while let Some(s) = cur {
        if let Some(v) = s.vars.borrow().get(name) { return Some(v.clone()); }
        cur = s.parent.clone();
    }
    None
}

fn assign_var(env: &Env, name: &str, v: &Value) -> bool {
    let mut cur: Option<Env> = Some(env.clone());
    while let Some(s) = cur {
        if s.vars.borrow().contains_key(name) {
            s.vars.borrow_mut().insert(name.to_string(), v.clone());
            return true;
        }
        cur = s.parent.clone();
    }
    false
}

fn mk_obj(arr: Option<Vec<Value>>, func: Option<(Rc<Func>, Env)>, native: Option<&'static str>) -> Value {
    Value::Obj(Rc::new(RefCell::new(Obj { props: BTreeMap::new(), arr, func, native })))
}
fn new_object() -> Value { mk_obj(None, None, None) }
fn new_array(v: Vec<Value>) -> Value { mk_obj(Some(v), None, None) }
fn native_fn(name: &'static str) -> Value { mk_obj(None, None, Some(name)) }
fn make_func(f: &Rc<Func>, env: &Env) -> Value { mk_obj(None, Some((f.clone(), env.clone())), None) }

fn set_obj_prop(o: &Value, k: &str, v: Value) {
    if let Value::Obj(r) = o { r.borrow_mut().props.insert(k.to_string(), v); }
}

fn throw_err(msg: &str) -> Ctl {
    let o = new_object();
    set_obj_prop(&o, "name", Value::Str("Error".to_string()));
    set_obj_prop(&o, "message", Value::Str(msg.to_string()));
    Ctl::Throw(o)
}


fn trunc(x: f64) -> f64 { if x.is_nan() || x.is_infinite() || x > 9.0e18 || x < -9.0e18 { x } else { x as i64 as f64 } }
fn floor(x: f64) -> f64 { let t = trunc(x); if t > x { t - 1.0 } else { t } }
fn ceil(x: f64) -> f64 { let t = trunc(x); if t < x { t + 1.0 } else { t } }
fn fabs(x: f64) -> f64 { if x < 0.0 { -x } else { x } }
fn fmod(a: f64, b: f64) -> f64 { if b == 0.0 || a.is_nan() || b.is_nan() || a.is_infinite() { f64::NAN } else if b.is_infinite() { a } else { a - b * trunc(a / b) } }
fn fpow(a: f64, b: f64) -> f64 {
    if b == 0.0 { return 1.0; }
    if b == trunc(b) && fabs(b) <= 1024.0 {
        let mut r = 1.0f64;
        let mut i = 0i64;
        let n = fabs(b) as i64;
        while i < n { r *= a; i += 1; }
        return if b < 0.0 { 1.0 / r } else { r };
    }
    f64::NAN
}
fn fsqrt(x: f64) -> f64 {
    if x < 0.0 || x.is_nan() { return f64::NAN; }
    if x == 0.0 || x.is_infinite() { return x; }
    let mut g = if x > 1.0 { x / 2.0 } else { 1.0 };
    let mut i = 0;
    while i < 60 { g = (g + x / g) / 2.0; i += 1; }
    g
}

fn num_to_str(n: f64) -> String {
    if n.is_nan() { return "NaN".to_string(); }
    if n.is_infinite() { return if n > 0.0 { "Infinity".to_string() } else { "-Infinity".to_string() }; }
    if n == trunc(n) && fabs(n) < 9.0e15 { return format!("{}", n as i64); }
    format!("{}", n)
}

fn to_str(v: &Value) -> String { to_str_d(v, 0) }

fn to_str_d(v: &Value, d: u32) -> String {
    match v {
        Value::Undef => "undefined".to_string(),
        Value::Null => "null".to_string(),
        Value::Bool(b) => if *b { "true".to_string() } else { "false".to_string() },
        Value::Num(n) => num_to_str(*n),
        Value::Str(s) => s.clone(),
        Value::Obj(o) => {
            if d > 8 { return String::new(); }
            let b = o.borrow();
            if let Some(a) = &b.arr {
                let mut s = String::new();
                for (i, x) in a.iter().enumerate() {
                    if i > 0 { s.push(','); }
                    if !matches!(x, Value::Undef | Value::Null) { s.push_str(&to_str_d(x, d + 1)); }
                }
                return s;
            }
            if b.func.is_some() || b.native.is_some() { return "function".to_string(); }
            if let Some(m) = b.props.get("message") {
                let name = match b.props.get("name") { Some(Value::Str(n)) => n.clone(), _ => "Error".to_string() };
                return format!("{}: {}", name, to_str_d(m, d + 1));
            }
            "[object Object]".to_string()
        }
    }
}

fn to_num(v: &Value) -> f64 {
    match v {
        Value::Undef => f64::NAN,
        Value::Null => 0.0,
        Value::Bool(b) => if *b { 1.0 } else { 0.0 },
        Value::Num(n) => *n,
        Value::Str(s) => {
            let t = s.trim();
            if t.is_empty() { return 0.0; }
            if let Some(h) = t.strip_prefix("0x") {
                let mut r = 0.0f64;
                for c in h.chars() { match c.to_digit(16) { Some(d) => r = r * 16.0 + d as f64, None => return f64::NAN } }
                return r;
            }
            if t == "Infinity" { return f64::INFINITY; }
            if t == "-Infinity" { return f64::NEG_INFINITY; }
            t.parse::<f64>().unwrap_or(f64::NAN)
        }
        Value::Obj(_) => f64::NAN,
    }
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Undef | Value::Null => false,
        Value::Bool(b) => *b,
        Value::Num(n) => !(*n == 0.0 || n.is_nan()),
        Value::Str(s) => !s.is_empty(),
        Value::Obj(_) => true,
    }
}

fn strict_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Undef, Value::Undef) | (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Num(x), Value::Num(y)) => x == y,
        (Value::Str(x), Value::Str(y)) => x == y,
        (Value::Obj(x), Value::Obj(y)) => Rc::ptr_eq(x, y),
        _ => false,
    }
}

fn loose_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Undef | Value::Null, Value::Undef | Value::Null) => true,
        (Value::Undef | Value::Null, _) | (_, Value::Undef | Value::Null) => false,
        (Value::Obj(_), _) | (_, Value::Obj(_)) => strict_eq(a, b),
        _ => {
            if core::mem::discriminant(a) == core::mem::discriminant(b) { strict_eq(a, b) } else { to_num(a) == to_num(b) }
        }
    }
}

fn is_obj(v: &Value) -> bool { matches!(v, Value::Obj(_)) }

fn type_of(v: &Value) -> &'static str {
    match v {
        Value::Undef => "undefined",
        Value::Null => "object",
        Value::Bool(_) => "boolean",
        Value::Num(_) => "number",
        Value::Str(_) => "string",
        Value::Obj(o) => { let b = o.borrow(); if b.func.is_some() || b.native.is_some() { "function" } else { "object" } }
    }
}

fn arg(args: &[Value], i: usize) -> Value { args.get(i).cloned().unwrap_or(Value::Undef) }
fn num_arg(args: &[Value], i: usize) -> f64 { to_num(&arg(args, i)) }

fn norm_idx(i: f64, len: usize) -> usize {
    if i.is_nan() { return 0; }
    let l = len as f64;
    let r = if i < 0.0 { let t = l + i; if t < 0.0 { 0.0 } else { t } } else if i > l { l } else { i };
    trunc(r) as usize
}

fn json_quote(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c { '"' => o.push_str("\\\""), '\\' => o.push_str("\\\\"), '\n' => o.push_str("\\n"), '\r' => o.push_str("\\r"), '\t' => o.push_str("\\t"), c => o.push(c) }
    }
    o.push('"');
    o
}

fn json(v: &Value, d: u32) -> Option<String> {
    if d > 20 { return None; }
    match v {
        Value::Undef => None,
        Value::Null => Some("null".to_string()),
        Value::Bool(b) => Some(if *b { "true".to_string() } else { "false".to_string() }),
        Value::Num(n) => Some(if n.is_finite() { num_to_str(*n) } else { "null".to_string() }),
        Value::Str(s) => Some(json_quote(s)),
        Value::Obj(o) => {
            let b = o.borrow();
            if b.func.is_some() || b.native.is_some() { return None; }
            if let Some(a) = &b.arr {
                let mut s = String::from("[");
                for (i, x) in a.iter().enumerate() {
                    if i > 0 { s.push(','); }
                    s.push_str(&json(x, d + 1).unwrap_or_else(|| "null".to_string()));
                }
                s.push(']');
                return Some(s);
            }
            let mut s = String::from("{");
            let mut first = true;
            for (k, x) in b.props.iter() {
                if let Some(js) = json(x, d + 1) {
                    if !first { s.push(','); }
                    first = false;
                    s.push_str(&json_quote(k));
                    s.push(':');
                    s.push_str(&js);
                }
            }
            s.push('}');
            Some(s)
        }
    }
}


struct Interp { out: String, steps: u64, depth: u32, globals: Env, seed: u64, els: BTreeMap<String, Value>, init: BTreeMap<String, (String, bool)> }

impl Interp {
    fn new() -> Interp {
        let g: Env = Rc::new(Scope { vars: RefCell::new(BTreeMap::new()), parent: None });
        let it = Interp { out: String::new(), steps: 0, depth: 0, globals: g.clone(), seed: 0x2545F4914F6CDD1D, els: BTreeMap::new(), init: BTreeMap::new() };
        let console = new_object();
        set_obj_prop(&console, "log", native_fn("console.log"));
        set_obj_prop(&console, "error", native_fn("console.log"));
        set_obj_prop(&console, "warn", native_fn("console.log"));
        declare(&g, "console", console);
        let document = new_object();
        set_obj_prop(&document, "write", native_fn("document.write"));
        set_obj_prop(&document, "writeln", native_fn("document.write"));
        set_obj_prop(&document, "getElementById", native_fn("document.getElementById"));
        set_obj_prop(&document, "querySelector", native_fn("document.querySelector"));
        declare(&g, "document", document);
        let math = new_object();
        set_obj_prop(&math, "floor", native_fn("Math.floor"));
        set_obj_prop(&math, "ceil", native_fn("Math.ceil"));
        set_obj_prop(&math, "round", native_fn("Math.round"));
        set_obj_prop(&math, "trunc", native_fn("Math.trunc"));
        set_obj_prop(&math, "abs", native_fn("Math.abs"));
        set_obj_prop(&math, "max", native_fn("Math.max"));
        set_obj_prop(&math, "min", native_fn("Math.min"));
        set_obj_prop(&math, "pow", native_fn("Math.pow"));
        set_obj_prop(&math, "sqrt", native_fn("Math.sqrt"));
        set_obj_prop(&math, "random", native_fn("Math.random"));
        set_obj_prop(&math, "sign", native_fn("Math.sign"));
        set_obj_prop(&math, "PI", Value::Num(3.141592653589793));
        set_obj_prop(&math, "E", Value::Num(2.718281828459045));
        declare(&g, "Math", math);
        let jsn = new_object();
        set_obj_prop(&jsn, "stringify", native_fn("JSON.stringify"));
        declare(&g, "JSON", jsn);
        let object = native_fn("Object");
        set_obj_prop(&object, "keys", native_fn("Object.keys"));
        set_obj_prop(&object, "values", native_fn("Object.values"));
        declare(&g, "Object", object);
        let array = native_fn("Array");
        set_obj_prop(&array, "isArray", native_fn("Array.isArray"));
        declare(&g, "Array", array);
        let date = new_object();
        set_obj_prop(&date, "now", native_fn("Date.now"));
        declare(&g, "Date", date);
        declare(&g, "parseInt", native_fn("parseInt"));
        declare(&g, "parseFloat", native_fn("parseFloat"));
        declare(&g, "isNaN", native_fn("isNaN"));
        declare(&g, "String", native_fn("String"));
        declare(&g, "Number", native_fn("Number"));
        declare(&g, "Boolean", native_fn("Boolean"));
        declare(&g, "Error", native_fn("Error"));
        declare(&g, "TypeError", native_fn("Error"));
        declare(&g, "alert", native_fn("alert"));
        declare(&g, "NaN", Value::Num(f64::NAN));
        declare(&g, "Infinity", Value::Num(f64::INFINITY));
        declare(&g, "undefined", Value::Undef);
        it
    }

    fn hoist(&mut self, body: &[Stmt], env: &Env) {
        for s in body.iter() {
            if let Stmt::Func(name, f) = s { declare(env, name, make_func(f, env)); }
        }
    }

    fn exec_block(&mut self, b: &[Stmt], env: &Env) -> Result<(), Ctl> {
        let e = new_scope(env);
        self.hoist(b, &e);
        for s in b.iter() { self.exec(s, &e)?; }
        Ok(())
    }

    fn exec(&mut self, s: &Stmt, env: &Env) -> Result<(), Ctl> {
        self.steps += 1;
        if self.steps > MAX_STEPS { return Err(throw_err("превышен лимит выполнения скрипта")); }
        match s {
            Stmt::Empty => Ok(()),
            Stmt::Func(_, _) => Ok(()),
            Stmt::Expr(e) => { self.eval(e, env)?; Ok(()) }
            Stmt::Var(ds) => {
                for (n, init) in ds.iter() {
                    let v = match init { Some(e) => self.eval(e, env)?, None => Value::Undef };
                    declare(env, n, v);
                }
                Ok(())
            }
            Stmt::Block(b) => self.exec_block(b, env),
            Stmt::If(c, t, e) => {
                if truthy(&self.eval(c, env)?) { self.exec(t, env) }
                else if let Some(e) = e { self.exec(e, env) }
                else { Ok(()) }
            }
            Stmt::While(c, b) => {
                loop {
                    if !truthy(&self.eval(c, env)?) { break; }
                    match self.exec(b, env) { Ok(()) => {}, Err(Ctl::Break) => break, Err(Ctl::Continue) => {}, Err(x) => return Err(x) }
                }
                Ok(())
            }
            Stmt::DoWhile(b, c) => {
                loop {
                    match self.exec(b, env) { Ok(()) => {}, Err(Ctl::Break) => break, Err(Ctl::Continue) => {}, Err(x) => return Err(x) }
                    if !truthy(&self.eval(c, env)?) { break; }
                }
                Ok(())
            }
            Stmt::For(init, cond, upd, body) => {
                let le = new_scope(env);
                if let Some(i) = init { self.exec(i, &le)?; }
                loop {
                    if let Some(c) = cond { if !truthy(&self.eval(c, &le)?) { break; } }
                    match self.exec(body, &le) { Ok(()) => {}, Err(Ctl::Break) => break, Err(Ctl::Continue) => {}, Err(x) => return Err(x) }
                    if let Some(u) = upd { self.eval(u, &le)?; }
                }
                Ok(())
            }
            Stmt::ForIn(name, e, body, is_of) => {
                let coll = self.eval(e, env)?;
                let items: Vec<Value> = if *is_of {
                    match &coll {
                        Value::Obj(o) => o.borrow().arr.clone().unwrap_or_default(),
                        Value::Str(s) => s.chars().map(|c| Value::Str(c.to_string())).collect(),
                        _ => Vec::new(),
                    }
                } else {
                    match &coll {
                        Value::Obj(o) => {
                            let b = o.borrow();
                            if let Some(a) = &b.arr { (0..a.len()).map(|i| Value::Str(i.to_string())).collect() }
                            else { b.props.keys().map(|k| Value::Str(k.clone())).collect() }
                        }
                        _ => Vec::new(),
                    }
                };
                for it in items {
                    let le = new_scope(env);
                    declare(&le, name, it);
                    match self.exec(body, &le) { Ok(()) => {}, Err(Ctl::Break) => break, Err(Ctl::Continue) => {}, Err(x) => return Err(x) }
                }
                Ok(())
            }
            Stmt::Return(e) => {
                let v = match e { Some(e) => self.eval(e, env)?, None => Value::Undef };
                Err(Ctl::Return(v))
            }
            Stmt::Break => Err(Ctl::Break),
            Stmt::Continue => Err(Ctl::Continue),
            Stmt::Throw(e) => { let v = self.eval(e, env)?; Err(Ctl::Throw(v)) }
            Stmt::Try(b, c, f) => {
                let r = self.exec_block(b, env);
                let r = match r {
                    Err(Ctl::Throw(v)) => match c {
                        Some((name, cb)) => {
                            let ce = new_scope(env);
                            if !name.is_empty() { declare(&ce, name, v); }
                            self.exec_block(cb, &ce)
                        }
                        None => Err(Ctl::Throw(v)),
                    },
                    other => other,
                };
                if let Some(fb) = f { self.exec_block(fb, env)?; }
                r
            }
            Stmt::Switch(d, cases) => {
                let dv = self.eval(d, env)?;
                let se = new_scope(env);
                let mut start: Option<usize> = None;
                for (i, (test, _)) in cases.iter().enumerate() {
                    if let Some(t) = test {
                        let tv = self.eval(t, &se)?;
                        if strict_eq(&dv, &tv) { start = Some(i); break; }
                    }
                }
                if start.is_none() {
                    for (i, (test, _)) in cases.iter().enumerate() { if test.is_none() { start = Some(i); break; } }
                }
                if let Some(st) = start {
                    for (_, body) in cases[st..].iter() {
                        for s in body.iter() {
                            match self.exec(s, &se) { Ok(()) => {}, Err(Ctl::Break) => return Ok(()), Err(x) => return Err(x) }
                        }
                    }
                }
                Ok(())
            }
        }
    }

    fn eval_args(&mut self, args: &[Expr], env: &Env) -> Result<Vec<Value>, Ctl> {
        let mut v = Vec::new();
        for a in args.iter() { v.push(self.eval(a, env)?); }
        Ok(v)
    }

    fn eval(&mut self, e: &Expr, env: &Env) -> Result<Value, Ctl> {
        match e {
            Expr::Num(n) => Ok(Value::Num(*n)),
            Expr::Str(s) => Ok(Value::Str(s.clone())),
            Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::Null => Ok(Value::Null),
            Expr::Undef => Ok(Value::Undef),
            Expr::This => Ok(lookup(env, "this").unwrap_or(Value::Undef)),
            Expr::Ident(n) => match lookup(env, n) {
                Some(v) => Ok(v),
                None => Err(throw_err(&format!("{} is not defined", n))),
            },
            Expr::Template(strs, exprs) => {
                let mut s = strs[0].clone();
                for (i, ex) in exprs.iter().enumerate() {
                    let v = self.eval(ex, env)?;
                    s.push_str(&to_str(&v));
                    s.push_str(&strs[i + 1]);
                }
                Ok(Value::Str(s))
            }
            Expr::Array(items) => {
                let mut v = Vec::new();
                for it in items.iter() { v.push(self.eval(it, env)?); }
                Ok(new_array(v))
            }
            Expr::Object(props) => {
                let o = new_object();
                for (k, x) in props.iter() {
                    let v = self.eval(x, env)?;
                    set_obj_prop(&o, k, v);
                }
                Ok(o)
            }
            Expr::Func(f) => Ok(make_func(f, env)),
            Expr::Unary(op, x) => {
                if *op == "typeof" {
                    if let Expr::Ident(n) = &**x {
                        return Ok(Value::Str(match lookup(env, n) { Some(v) => type_of(&v).to_string(), None => "undefined".to_string() }));
                    }
                }
                if *op == "delete" {
                    match &**x {
                        Expr::Member(o, n) => { let ov = self.eval(o, env)?; if let Value::Obj(r) = &ov { r.borrow_mut().props.remove(n.as_str()); } }
                        Expr::Index(o, i) => { let ov = self.eval(o, env)?; let k = to_str(&self.eval(i, env)?); if let Value::Obj(r) = &ov { r.borrow_mut().props.remove(k.as_str()); } }
                        _ => {}
                    }
                    return Ok(Value::Bool(true));
                }
                let v = self.eval(x, env)?;
                Ok(match *op {
                    "!" => Value::Bool(!truthy(&v)),
                    "-" => Value::Num(-to_num(&v)),
                    "+" => Value::Num(to_num(&v)),
                    "~" => Value::Num(!(to_num(&v) as i64 as i32) as f64),
                    "typeof" => Value::Str(type_of(&v).to_string()),
                    _ => Value::Undef,
                })
            }
            Expr::Update(inc, prefix, t) => {
                let old = to_num(&self.eval(t, env)?);
                let nv = if *inc { old + 1.0 } else { old - 1.0 };
                self.assign_to(t, env, Value::Num(nv))?;
                Ok(Value::Num(if *prefix { nv } else { old }))
            }
            Expr::Bin(op, a, b) => {
                let x = self.eval(a, env)?;
                let y = self.eval(b, env)?;
                self.binop(op, x, y)
            }
            Expr::Logic(k, a, b) => {
                let x = self.eval(a, env)?;
                match *k {
                    0 => if truthy(&x) { self.eval(b, env) } else { Ok(x) },
                    1 => if truthy(&x) { Ok(x) } else { self.eval(b, env) },
                    _ => if matches!(x, Value::Undef | Value::Null) { self.eval(b, env) } else { Ok(x) },
                }
            }
            Expr::Assign(op, t, v) => {
                let nv = if op.is_empty() {
                    self.eval(v, env)?
                } else {
                    let cur = self.eval(t, env)?;
                    let r = self.eval(v, env)?;
                    self.binop(op, cur, r)?
                };
                self.assign_to(t, env, nv.clone())?;
                Ok(nv)
            }
            Expr::Cond(c, a, b) => { if truthy(&self.eval(c, env)?) { self.eval(a, env) } else { self.eval(b, env) } }
            Expr::Seq(v) => {
                let mut last = Value::Undef;
                for x in v.iter() { last = self.eval(x, env)?; }
                Ok(last)
            }
            Expr::Member(o, n) => { let ov = self.eval(o, env)?; Ok(get_prop(&ov, n)) }
            Expr::Index(o, i) => { let ov = self.eval(o, env)?; let iv = self.eval(i, env)?; Ok(get_index(&ov, &iv)) }
            Expr::Call(c, args) => self.eval_call(c, args, env),
            Expr::New(c, args) => {
                let f = self.eval(c, env)?;
                let av = self.eval_args(args, env)?;
                if let Value::Obj(o) = &f {
                    let (func, native) = { let b = o.borrow(); (b.func.clone(), b.native) };
                    if let Some(n) = native { return self.call_native(n, Value::Undef, av); }
                    if let Some((def, cenv)) = func {
                        let obj = new_object();
                        let r = self.call_func(&def, &cenv, obj.clone(), av)?;
                        return Ok(if is_obj(&r) { r } else { obj });
                    }
                }
                Err(throw_err("значение не является конструктором"))
            }
        }
    }

    fn assign_to(&mut self, t: &Expr, env: &Env, v: Value) -> Result<(), Ctl> {
        match t {
            Expr::Ident(n) => {
                if !assign_var(env, n, &v) { let g = self.globals.clone(); declare(&g, n, v); }
                Ok(())
            }
            Expr::Member(o, n) => { let ov = self.eval(o, env)?; set_prop(&ov, n, v); Ok(()) }
            Expr::Index(o, i) => { let ov = self.eval(o, env)?; let iv = self.eval(i, env)?; set_index(&ov, &iv, v); Ok(()) }
            _ => Err(throw_err("неверная цель присваивания")),
        }
    }

    fn binop(&mut self, op: &str, a: Value, b: Value) -> Result<Value, Ctl> {
        Ok(match op {
            "+" => {
                if matches!(a, Value::Str(_)) || matches!(b, Value::Str(_)) || is_obj(&a) || is_obj(&b) {
                    Value::Str(format!("{}{}", to_str(&a), to_str(&b)))
                } else {
                    Value::Num(to_num(&a) + to_num(&b))
                }
            }
            "-" => Value::Num(to_num(&a) - to_num(&b)),
            "*" => Value::Num(to_num(&a) * to_num(&b)),
            "/" => Value::Num(to_num(&a) / to_num(&b)),
            "%" => Value::Num(fmod(to_num(&a), to_num(&b))),
            "**" => Value::Num(fpow(to_num(&a), to_num(&b))),
            "==" => Value::Bool(loose_eq(&a, &b)),
            "!=" => Value::Bool(!loose_eq(&a, &b)),
            "===" => Value::Bool(strict_eq(&a, &b)),
            "!==" => Value::Bool(!strict_eq(&a, &b)),
            "<" | ">" | "<=" | ">=" => {
                if let (Value::Str(x), Value::Str(y)) = (&a, &b) {
                    Value::Bool(match op { "<" => x < y, ">" => x > y, "<=" => x <= y, _ => x >= y })
                } else {
                    let x = to_num(&a);
                    let y = to_num(&b);
                    Value::Bool(match op { "<" => x < y, ">" => x > y, "<=" => x <= y, _ => x >= y })
                }
            }
            "&" => Value::Num(((to_num(&a) as i64 as i32) & (to_num(&b) as i64 as i32)) as f64),
            "|" => Value::Num(((to_num(&a) as i64 as i32) | (to_num(&b) as i64 as i32)) as f64),
            "^" => Value::Num(((to_num(&a) as i64 as i32) ^ (to_num(&b) as i64 as i32)) as f64),
            "<<" => Value::Num(((to_num(&a) as i64 as i32).wrapping_shl(to_num(&b) as i64 as u32 & 31)) as f64),
            ">>" => Value::Num(((to_num(&a) as i64 as i32).wrapping_shr(to_num(&b) as i64 as u32 & 31)) as f64),
            "instanceof" => Value::Bool(false),
            "in" => {
                let k = to_str(&a);
                match &b {
                    Value::Obj(o) => {
                        let ob = o.borrow();
                        let in_arr = match (&ob.arr, k.parse::<usize>()) { (Some(arr), Ok(i)) => i < arr.len(), _ => false };
                        Value::Bool(in_arr || ob.props.contains_key(k.as_str()))
                    }
                    _ => Value::Bool(false),
                }
            }
            _ => Value::Undef,
        })
    }

    fn eval_call(&mut self, c: &Expr, args: &[Expr], env: &Env) -> Result<Value, Ctl> {
        match c {
            Expr::Member(o, n) => {
                let ov = self.eval(o, env)?;
                let av = self.eval_args(args, env)?;
                self.call_method(ov, n, av)
            }
            Expr::Index(o, i) => {
                let ov = self.eval(o, env)?;
                let key = to_str(&self.eval(i, env)?);
                let av = self.eval_args(args, env)?;
                self.call_method(ov, &key, av)
            }
            _ => {
                let f = self.eval(c, env)?;
                let av = self.eval_args(args, env)?;
                self.call_value(&f, Value::Undef, av)
            }
        }
    }

    fn call_method(&mut self, ov: Value, name: &str, args: Vec<Value>) -> Result<Value, Ctl> {
        if let Value::Obj(o) = &ov {
            let f = o.borrow().props.get(name).cloned();
            if let Some(f) = f { return self.call_value(&f, ov.clone(), args); }
        }
        self.builtin_method(ov, name, args)
    }

    fn call_value(&mut self, f: &Value, this: Value, args: Vec<Value>) -> Result<Value, Ctl> {
        if let Value::Obj(o) = f {
            let (func, native) = { let b = o.borrow(); (b.func.clone(), b.native) };
            if let Some(n) = native { return self.call_native(n, this, args); }
            if let Some((def, cenv)) = func { return self.call_func(&def, &cenv, this, args); }
        }
        Err(throw_err("значение не является функцией"))
    }

    fn call_func(&mut self, def: &Rc<Func>, cenv: &Env, this: Value, args: Vec<Value>) -> Result<Value, Ctl> {
        if self.depth >= MAX_DEPTH { return Err(throw_err("слишком глубокая рекурсия")); }
        self.depth += 1;
        let scope = new_scope(cenv);
        if !def.arrow { declare(&scope, "this", this); }
        for (i, p) in def.params.iter().enumerate() {
            declare(&scope, p, args.get(i).cloned().unwrap_or(Value::Undef));
        }
        self.hoist(&def.body, &scope);
        let mut res: Result<Value, Ctl> = Ok(Value::Undef);
        for s in def.body.iter() {
            match self.exec(s, &scope) {
                Ok(()) => {}
                Err(Ctl::Return(v)) => { res = Ok(v); break; }
                Err(Ctl::Throw(v)) => { res = Err(Ctl::Throw(v)); break; }
                Err(_) => {}
            }
        }
        self.depth -= 1;
        res
    }

    fn element(&mut self, id: &str) -> Value {
        if let Some(v) = self.els.get(id) { return v.clone(); }
        let (text, is_in) = match self.init.get(id) { Some(t) => t.clone(), None => return Value::Null };
        let (tc, val) = if is_in { (String::new(), text) } else { (text, String::new()) };
        let o = new_object();
        set_obj_prop(&o, "id", Value::Str(id.to_string()));
        set_obj_prop(&o, "textContent", Value::Str(tc.clone()));
        set_obj_prop(&o, "innerText", Value::Str(tc.clone()));
        set_obj_prop(&o, "innerHTML", Value::Str(tc.clone()));
        set_obj_prop(&o, "value", Value::Str(val.clone()));
        set_obj_prop(&o, "__t", Value::Str(tc));
        set_obj_prop(&o, "__v", Value::Str(val));
        set_obj_prop(&o, "style", new_object());
        set_obj_prop(&o, "addEventListener", native_fn("el.addEventListener"));
        self.els.insert(id.to_string(), o.clone());
        o
    }

    fn next_rand(&mut self) -> f64 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        ((self.seed >> 11) as f64) / 9007199254740992.0
    }

    fn call_native(&mut self, name: &'static str, _this: Value, args: Vec<Value>) -> Result<Value, Ctl> {
        Ok(match name {
            "console.log" => {
                let parts: Vec<String> = args.iter().map(|a| to_str(a)).collect();
                self.out.push_str(&parts.join(" "));
                self.out.push('\n');
                Value::Undef
            }
            "alert" => {
                self.out.push_str(&format!("[alert] {}\n", to_str(&arg(&args, 0))));
                Value::Undef
            }
            "document.write" => {
                for a in args.iter() { self.out.push_str(&to_str(a)); }
                Value::Undef
            }
            "document.getElementById" => {
                let id = to_str(&arg(&args, 0));
                self.element(&id)
            }
            "document.querySelector" => {
                let q = to_str(&arg(&args, 0));
                let id = q.trim().trim_start_matches('#').to_string();
                self.element(&id)
            }
            "el.addEventListener" => {
                let ty = to_str(&arg(&args, 0));
                set_obj_prop(&_this, &format!("__on_{}", ty), arg(&args, 1));
                Value::Undef
            }
            "Math.floor" => Value::Num(floor(num_arg(&args, 0))),
            "Math.ceil" => Value::Num(ceil(num_arg(&args, 0))),
            "Math.round" => Value::Num(floor(num_arg(&args, 0) + 0.5)),
            "Math.trunc" => Value::Num(trunc(num_arg(&args, 0))),
            "Math.abs" => Value::Num(fabs(num_arg(&args, 0))),
            "Math.sign" => { let x = num_arg(&args, 0); Value::Num(if x > 0.0 { 1.0 } else if x < 0.0 { -1.0 } else { x }) }
            "Math.pow" => Value::Num(fpow(num_arg(&args, 0), num_arg(&args, 1))),
            "Math.sqrt" => Value::Num(fsqrt(num_arg(&args, 0))),
            "Math.random" => Value::Num(self.next_rand()),
            "Math.max" => {
                let mut m = f64::NEG_INFINITY;
                for a in args.iter() { let x = to_num(a); if x.is_nan() { m = f64::NAN; break; } if x > m { m = x; } }
                Value::Num(m)
            }
            "Math.min" => {
                let mut m = f64::INFINITY;
                for a in args.iter() { let x = to_num(a); if x.is_nan() { m = f64::NAN; break; } if x < m { m = x; } }
                Value::Num(m)
            }
            "parseInt" => {
                let s = to_str(&arg(&args, 0));
                let t = s.trim();
                let radix = if args.len() > 1 { to_num(&args[1]) as u32 } else { 10 };
                let (neg, body) = match t.strip_prefix('-') { Some(r) => (true, r), None => (false, t.strip_prefix('+').unwrap_or(t)) };
                let (radix, body) = if (radix == 16 || args.len() < 2) && (body.starts_with("0x") || body.starts_with("0X")) { (16, &body[2..]) } else { (if radix < 2 || radix > 36 { 10 } else { radix }, body) };
                let mut r = 0.0f64;
                let mut any = false;
                for c in body.chars() {
                    match c.to_digit(radix) { Some(d) => { r = r * radix as f64 + d as f64; any = true; } None => break }
                }
                Value::Num(if any { if neg { -r } else { r } } else { f64::NAN })
            }
            "parseFloat" => {
                let s = to_str(&arg(&args, 0));
                let t: Vec<char> = s.trim().chars().collect();
                let mut end = 0usize;
                while end < t.len() && (t[end].is_ascii_digit() || t[end] == '.' || t[end] == '-' || t[end] == '+' || t[end] == 'e' || t[end] == 'E') { end += 1; }
                let mut r = f64::NAN;
                while end > 0 {
                    let cand: String = t[..end].iter().collect();
                    if let Ok(x) = cand.parse::<f64>() { r = x; break; }
                    end -= 1;
                }
                Value::Num(r)
            }
            "isNaN" => Value::Bool(num_arg(&args, 0).is_nan()),
            "String" => Value::Str(if args.is_empty() { String::new() } else { to_str(&args[0]) }),
            "Number" => Value::Num(if args.is_empty() { 0.0 } else { to_num(&args[0]) }),
            "Boolean" => Value::Bool(truthy(&arg(&args, 0))),
            "JSON.stringify" => match json(&arg(&args, 0), 0) { Some(s) => Value::Str(s), None => Value::Undef },
            "Object" => new_object(),
            "Object.keys" => match arg(&args, 0) {
                Value::Obj(o) => {
                    let b = o.borrow();
                    if let Some(a) = &b.arr { new_array((0..a.len()).map(|i| Value::Str(i.to_string())).collect()) }
                    else { new_array(b.props.keys().map(|k| Value::Str(k.clone())).collect()) }
                }
                _ => new_array(Vec::new()),
            },
            "Object.values" => match arg(&args, 0) {
                Value::Obj(o) => {
                    let b = o.borrow();
                    if let Some(a) = &b.arr { new_array(a.clone()) } else { new_array(b.props.values().cloned().collect()) }
                }
                _ => new_array(Vec::new()),
            },
            "Array" => {
                if args.len() == 1 {
                    if let Value::Num(n) = &args[0] {
                        let len = if *n >= 0.0 && *n < 100000.0 { trunc(*n) as usize } else { 0 };
                        return Ok(new_array(vec![Value::Undef; len]));
                    }
                }
                new_array(args)
            }
            "Array.isArray" => Value::Bool(matches!(arg(&args, 0), Value::Obj(o) if o.borrow().arr.is_some())),
            "Error" => {
                let o = new_object();
                set_obj_prop(&o, "name", Value::Str("Error".to_string()));
                set_obj_prop(&o, "message", Value::Str(if args.is_empty() { String::new() } else { to_str(&args[0]) }));
                o
            }
            "Date.now" => Value::Num(crate::timer::millis() as f64),
            _ => Value::Undef,
        })
    }

    fn builtin_method(&mut self, ov: Value, name: &str, args: Vec<Value>) -> Result<Value, Ctl> {
        match &ov {
            Value::Str(s) => self.string_method(s, name, &args),
            Value::Num(n) => match name {
                "toFixed" => {
                    let d = trunc(num_arg(&args, 0)).max(0.0).min(20.0) as u32;
                    let mut m = 1u64;
                    for _ in 0..d { m *= 10; }
                    let neg = *n < 0.0;
                    let scaled = floor(fabs(*n) * m as f64 + 0.5) as u64;
                    let ip = scaled / m;
                    let fp = scaled % m;
                    let sign = if neg && scaled != 0 { "-" } else { "" };
                    Ok(Value::Str(if d == 0 { format!("{}{}", sign, ip) } else { format!("{}{}.{:0w$}", sign, ip, fp, w = d as usize) }))
                }
                "toString" => Ok(Value::Str(num_to_str(*n))),
                _ => Err(throw_err(&format!("{} не является функцией", name))),
            },
            Value::Obj(o) => {
                let is_arr = o.borrow().arr.is_some();
                if is_arr { return self.array_method(o, name, args); }
                match name {
                    "hasOwnProperty" => Ok(Value::Bool(o.borrow().props.contains_key(to_str(&arg(&args, 0)).as_str()))),
                    "toString" => Ok(Value::Str(to_str(&ov))),
                    _ => Err(throw_err(&format!("{} не является функцией", name))),
                }
            }
            Value::Bool(b) => match name {
                "toString" => Ok(Value::Str(if *b { "true".to_string() } else { "false".to_string() })),
                _ => Err(throw_err(&format!("{} не является функцией", name))),
            },
            _ => Err(throw_err(&format!("нельзя вызвать {} у undefined/null", name))),
        }
    }

    fn string_method(&mut self, s: &String, name: &str, args: &[Value]) -> Result<Value, Ctl> {
        let cs: Vec<char> = s.chars().collect();
        let len = cs.len();
        Ok(match name {
            "charAt" => { let i = num_arg(args, 0); if i >= 0.0 && (i as usize) < len { Value::Str(cs[i as usize].to_string()) } else { Value::Str(String::new()) } }
            "charCodeAt" => { let i = num_arg(args, 0); if i >= 0.0 && (i as usize) < len { Value::Num(cs[i as usize] as u32 as f64) } else { Value::Num(f64::NAN) } }
            "indexOf" => { let p = to_str(&arg(args, 0)); match s.find(p.as_str()) { Some(b) => Value::Num(s[..b].chars().count() as f64), None => Value::Num(-1.0) } }
            "lastIndexOf" => { let p = to_str(&arg(args, 0)); match s.rfind(p.as_str()) { Some(b) => Value::Num(s[..b].chars().count() as f64), None => Value::Num(-1.0) } }
            "includes" => Value::Bool(s.contains(to_str(&arg(args, 0)).as_str())),
            "startsWith" => Value::Bool(s.starts_with(to_str(&arg(args, 0)).as_str())),
            "endsWith" => Value::Bool(s.ends_with(to_str(&arg(args, 0)).as_str())),
            "slice" | "substring" => {
                let mut a = norm_idx(num_arg(args, 0), len);
                let mut b = if matches!(arg(args, 1), Value::Undef) { len } else { norm_idx(num_arg(args, 1), len) };
                if name == "substring" && a > b { core::mem::swap(&mut a, &mut b); }
                if a >= b { Value::Str(String::new()) } else { Value::Str(cs[a..b].iter().collect()) }
            }
            "substr" => {
                let a = norm_idx(num_arg(args, 0), len);
                let l = if matches!(arg(args, 1), Value::Undef) { len } else { norm_idx(num_arg(args, 1), len) };
                let b = core::cmp::min(a + l, len);
                Value::Str(cs[a..b].iter().collect())
            }
            "toUpperCase" => Value::Str(s.to_uppercase()),
            "toLowerCase" => Value::Str(s.to_lowercase()),
            "trim" => Value::Str(s.trim().to_string()),
            "toString" => Value::Str(s.clone()),
            "concat" => { let mut r = s.clone(); for a in args.iter() { r.push_str(&to_str(a)); } Value::Str(r) }
            "repeat" => { let n = num_arg(args, 0); let n = if n > 0.0 && n < 10000.0 { n as usize } else { 0 }; Value::Str(s.repeat(n)) }
            "padStart" | "padEnd" => {
                let target = num_arg(args, 0);
                let target = if target > 0.0 && target < 10000.0 { target as usize } else { 0 };
                let pad = if matches!(arg(args, 1), Value::Undef) { " ".to_string() } else { to_str(&arg(args, 1)) };
                let pc: Vec<char> = pad.chars().collect();
                if len >= target || pc.is_empty() { Value::Str(s.clone()) } else {
                    let mut fill = String::new();
                    let mut i = 0usize;
                    while fill.chars().count() < target - len { fill.push(pc[i % pc.len()]); i += 1; }
                    Value::Str(if name == "padStart" { format!("{}{}", fill, s) } else { format!("{}{}", s, fill) })
                }
            }
            "split" => {
                match arg(args, 0) {
                    Value::Undef => new_array(vec![Value::Str(s.clone())]),
                    sep => {
                        let sp = to_str(&sep);
                        if sp.is_empty() { new_array(cs.iter().map(|c| Value::Str(c.to_string())).collect()) }
                        else { new_array(s.split(sp.as_str()).map(|x| Value::Str(x.to_string())).collect()) }
                    }
                }
            }
            "replace" | "replaceAll" => {
                match arg(args, 0) {
                    Value::Str(p) => {
                        let rep = to_str(&arg(args, 1));
                        if name == "replaceAll" { Value::Str(s.replace(p.as_str(), rep.as_str())) }
                        else { match s.find(p.as_str()) { Some(i) => Value::Str(format!("{}{}{}", &s[..i], rep, &s[i + p.len()..])), None => Value::Str(s.clone()) } }
                    }
                    _ => Value::Str(s.clone()),
                }
            }
            _ => return Err(throw_err(&format!("строка не имеет метода {}", name))),
        })
    }

    fn array_method(&mut self, o: &ObjRef, name: &str, args: Vec<Value>) -> Result<Value, Ctl> {
        let items: Vec<Value> = o.borrow().arr.clone().unwrap_or_default();
        let len = items.len();
        Ok(match name {
            "push" => {
                let mut b = o.borrow_mut();
                let a = b.arr.get_or_insert_with(Vec::new);
                for x in args { a.push(x); }
                Value::Num(a.len() as f64)
            }
            "pop" => { let mut b = o.borrow_mut(); let r = b.arr.get_or_insert_with(Vec::new).pop(); r.unwrap_or(Value::Undef) }
            "shift" => { let mut b = o.borrow_mut(); let a = b.arr.get_or_insert_with(Vec::new); let r = if a.is_empty() { Value::Undef } else { a.remove(0) }; r }
            "unshift" => {
                let mut b = o.borrow_mut();
                let a = b.arr.get_or_insert_with(Vec::new);
                for (i, x) in args.into_iter().enumerate() { a.insert(i, x); }
                Value::Num(a.len() as f64)
            }
            "reverse" => { let mut r = items; r.reverse(); o.borrow_mut().arr = Some(r); Value::Obj(o.clone()) }
            "join" => {
                let sep = if matches!(arg(&args, 0), Value::Undef) { ",".to_string() } else { to_str(&arg(&args, 0)) };
                let parts: Vec<String> = items.iter().map(|x| if matches!(x, Value::Undef | Value::Null) { String::new() } else { to_str(x) }).collect();
                Value::Str(parts.join(sep.as_str()))
            }
            "indexOf" => { let t = arg(&args, 0); Value::Num(items.iter().position(|x| strict_eq(x, &t)).map(|i| i as f64).unwrap_or(-1.0)) }
            "includes" => { let t = arg(&args, 0); Value::Bool(items.iter().any(|x| strict_eq(x, &t))) }
            "slice" => {
                let a = norm_idx(num_arg(&args, 0), len);
                let b = if matches!(arg(&args, 1), Value::Undef) { len } else { norm_idx(num_arg(&args, 1), len) };
                if a >= b { new_array(Vec::new()) } else { new_array(items[a..b].to_vec()) }
            }
            "splice" => {
                let a = norm_idx(num_arg(&args, 0), len);
                let cnt = if args.len() < 2 { len - a } else { let c = num_arg(&args, 1); if c < 0.0 { 0 } else { core::cmp::min(trunc(c) as usize, len - a) } };
                let mut r = items;
                let removed: Vec<Value> = r.drain(a..a + cnt).collect();
                for (i, x) in args.iter().skip(2).enumerate() { r.insert(a + i, x.clone()); }
                o.borrow_mut().arr = Some(r);
                new_array(removed)
            }
            "concat" => {
                let mut r = items;
                for a in args.iter() {
                    match a {
                        Value::Obj(x) if x.borrow().arr.is_some() => { let v = x.borrow().arr.clone().unwrap_or_default(); r.extend(v); }
                        other => r.push(other.clone()),
                    }
                }
                new_array(r)
            }
            "toString" => Value::Str(to_str(&Value::Obj(o.clone()))),
            "forEach" | "map" | "filter" | "find" | "findIndex" | "some" | "every" => {
                let f = arg(&args, 0);
                let mut out: Vec<Value> = Vec::new();
                for (i, x) in items.iter().enumerate() {
                    let r = self.call_value(&f, Value::Undef, vec![x.clone(), Value::Num(i as f64)])?;
                    match name {
                        "map" => out.push(r),
                        "filter" => { if truthy(&r) { out.push(x.clone()); } }
                        "find" => { if truthy(&r) { return Ok(x.clone()); } }
                        "findIndex" => { if truthy(&r) { return Ok(Value::Num(i as f64)); } }
                        "some" => { if truthy(&r) { return Ok(Value::Bool(true)); } }
                        "every" => { if !truthy(&r) { return Ok(Value::Bool(false)); } }
                        _ => {}
                    }
                }
                match name {
                    "map" | "filter" => new_array(out),
                    "findIndex" => Value::Num(-1.0),
                    "some" => Value::Bool(false),
                    "every" => Value::Bool(true),
                    _ => Value::Undef,
                }
            }
            "reduce" => {
                let f = arg(&args, 0);
                let mut it = items.into_iter().enumerate();
                let mut acc = if args.len() > 1 { args[1].clone() } else { match it.next() { Some((_, x)) => x, None => return Err(throw_err("reduce пустого массива без начального значения")) } };
                for (i, x) in it { acc = self.call_value(&f, Value::Undef, vec![acc, x, Value::Num(i as f64)])?; }
                acc
            }
            "sort" => {
                let f = arg(&args, 0);
                let mut r = items;
                let n = r.len();
                let mut i = 1usize;
                while i < n {
                    let mut j = i;
                    while j > 0 {
                        let gt = if matches!(f, Value::Undef) {
                            to_str(&r[j - 1]) > to_str(&r[j])
                        } else {
                            to_num(&self.call_value(&f, Value::Undef, vec![r[j - 1].clone(), r[j].clone()])?) > 0.0
                        };
                        if gt { r.swap(j - 1, j); j -= 1; } else { break; }
                    }
                    i += 1;
                }
                o.borrow_mut().arr = Some(r);
                Value::Obj(o.clone())
            }
            _ => return Err(throw_err(&format!("массив не имеет метода {}", name))),
        })
    }
}


fn get_prop(ov: &Value, key: &str) -> Value {
    match ov {
        Value::Obj(o) => {
            let b = o.borrow();
            if let Some(a) = &b.arr {
                if key == "length" { return Value::Num(a.len() as f64); }
                if let Ok(i) = key.parse::<usize>() { return a.get(i).cloned().unwrap_or(Value::Undef); }
            }
            b.props.get(key).cloned().unwrap_or(Value::Undef)
        }
        Value::Str(s) => {
            if key == "length" { return Value::Num(s.chars().count() as f64); }
            if let Ok(i) = key.parse::<usize>() { return s.chars().nth(i).map(|c| Value::Str(c.to_string())).unwrap_or(Value::Undef); }
            Value::Undef
        }
        _ => Value::Undef,
    }
}

fn get_index(ov: &Value, iv: &Value) -> Value {
    if let Value::Num(n) = iv {
        if *n >= 0.0 && *n == trunc(*n) { return get_prop(ov, &format!("{}", *n as usize)); }
    }
    get_prop(ov, &to_str(iv))
}

fn set_prop(ov: &Value, key: &str, v: Value) {
    if let Value::Obj(o) = ov {
        let mut b = o.borrow_mut();
        if let Some(a) = b.arr.as_mut() {
            if key == "length" {
                let n = to_num(&v);
                if n >= 0.0 && n < 100000.0 { a.resize(trunc(n) as usize, Value::Undef); }
                return;
            }
            if let Ok(i) = key.parse::<usize>() {
                if i < 100000 {
                    if i >= a.len() { a.resize(i + 1, Value::Undef); }
                    a[i] = v;
                }
                return;
            }
        }
        b.props.insert(key.to_string(), v);
    }
}

fn set_index(ov: &Value, iv: &Value, v: Value) {
    if let Value::Num(n) = iv {
        if *n >= 0.0 && *n == trunc(*n) { set_prop(ov, &format!("{}", *n as usize), v); return; }
    }
    set_prop(ov, &to_str(iv), v);
}


fn strip_tags(s: &str) -> String {
    if !s.contains('<') { return s.to_string(); }
    let mut out = String::new();
    let mut inside = false;
    for c in s.chars() {
        if c == '<' { inside = true; }
        else if c == '>' && inside { inside = false; }
        else if !inside { out.push(c); }
    }
    out
}

pub struct Change { pub id: String, pub text: String, pub is_value: bool }

pub struct Session { it: Interp }

fn run_code(it: &mut Interp, src: &str) {
    match parse_program(src) {
        Ok(prog) => {
            let g = it.globals.clone();
            it.hoist(&prog, &g);
            for s in prog.iter() {
                match it.exec(s, &g) {
                    Ok(()) => {}
                    Err(Ctl::Throw(v)) => {
                        let msg = format!("\nJS error: {}\n", to_str(&v));
                        it.out.push_str(&msg);
                        break;
                    }
                    Err(_) => {}
                }
            }
        }
        Err(e) => { let msg = format!("\nJS syntax error: {}\n", e); it.out.push_str(&msg); }
    }
}

pub fn start(src: &str, elems: Vec<(String, String, bool)>) -> (Session, String) {
    let mut it = Interp::new();
    for (id, t, is_in) in elems { it.init.insert(id, (t, is_in)); }
    run_code(&mut it, src);
    let out = core::mem::take(&mut it.out);
    (Session { it }, out)
}

impl Session {
    pub fn dispatch(&mut self, code: &str, id: &str, inputs: &[(String, String)]) -> (String, Vec<Change>) {
        let it = &mut self.it;
        it.steps = 0;
        it.depth = 0;
        it.out.clear();
        for (iid, val) in inputs.iter() {
            it.init.insert(iid.clone(), (val.clone(), true));
            if let Some(el) = it.els.get(iid) {
                set_obj_prop(el, "value", Value::Str(val.clone()));
                set_obj_prop(el, "__v", Value::Str(val.clone()));
            }
        }
        if !code.trim().is_empty() { run_code(it, code); }
        if !id.is_empty() {
            let el = it.els.get(id).cloned();
            if let Some(el) = el {
                let f = get_prop(&el, "__on_click");
                if is_obj(&f) {
                    let ev = new_object();
                    set_obj_prop(&ev, "type", Value::Str("click".to_string()));
                    set_obj_prop(&ev, "target", el.clone());
                    if let Err(Ctl::Throw(v)) = it.call_value(&f, el.clone(), vec![ev]) {
                        let msg = format!("\nJS error: {}\n", to_str(&v));
                        it.out.push_str(&msg);
                    }
                }
            }
        }
        let mut changes: Vec<Change> = Vec::new();
        for (iid, el) in it.els.iter() {
            let last_t = to_str(&get_prop(el, "__t"));
            let mut new_t: Option<String> = None;
            for k in ["textContent", "innerText", "innerHTML"].iter() {
                let cur = strip_tags(&to_str(&get_prop(el, k)));
                if cur != last_t { new_t = Some(cur); break; }
            }
            if let Some(t) = new_t {
                for k in ["textContent", "innerText", "innerHTML"].iter() { set_obj_prop(el, k, Value::Str(t.clone())); }
                set_obj_prop(el, "__t", Value::Str(t.clone()));
                changes.push(Change { id: iid.clone(), text: t, is_value: false });
            }
            let last_v = to_str(&get_prop(el, "__v"));
            let cur_v = to_str(&get_prop(el, "value"));
            if cur_v != last_v {
                set_obj_prop(el, "__v", Value::Str(cur_v.clone()));
                changes.push(Change { id: iid.clone(), text: cur_v, is_value: true });
            }
        }
        let out = core::mem::take(&mut it.out);
        (out, changes)
    }
}

pub fn run(src: &str) -> String {
    start(src, Vec::new()).1
}
