use alloc::string::{String, ToString};
use alloc::vec::Vec;
use super::dom::*;

#[derive(Clone, Debug, PartialEq)]
pub enum AttrOp { Exists, Eq, Contains, Prefix, Suffix, Word, Dash }

#[derive(Clone, Debug)]
pub struct Compound {
    pub tag: Option<String>,
    pub id: Option<String>,
    pub classes: Vec<String>,
    pub attrs: Vec<(String, AttrOp, String)>,
    pub pseudo: Vec<String>,
    pub never: bool,
    pub not: Vec<Compound>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Comb { Desc, Child, Adj, Sib }

#[derive(Clone, Debug)]
pub struct Selector {
    pub parts: Vec<(Compound, Comb)>,
    pub spec: u32,
}

#[derive(Clone, Debug)]
pub struct Decl { pub name: String, pub value: String, pub important: bool }

#[derive(Clone, Debug)]
pub struct Rule { pub sels: Vec<Selector>, pub decls: Vec<Decl>, pub order: u32 }

pub struct Sheet { pub rules: Vec<Rule> }

fn strip_comments(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find("/*") {
        out.push_str(&rest[..i]);
        match rest[i + 2..].find("*/") { Some(j) => rest = &rest[i + 2 + j + 2..], None => { rest = ""; } }
    }
    out.push_str(rest);
    out
}

fn match_brace(b: &[u8], mut i: usize) -> usize {
    let mut depth = 1;
    let mut q = 0u8;
    while i < b.len() {
        let c = b[i];
        if q != 0 { if c == q && b[i - 1] != b'\\' { q = 0; } }
        else if c == b'"' || c == b'\'' { q = c; }
        else if c == b'{' { depth += 1; }
        else if c == b'}' { depth -= 1; if depth == 0 { return i; } }
        i += 1;
    }
    b.len()
}

pub fn parse_decls(s: &str) -> Vec<Decl> {
    let mut v = Vec::new();
    let mut depth = 0i32;
    let mut q = 0u8;
    let b = s.as_bytes();
    let mut start = 0;
    let mut push = |part: &str, v: &mut Vec<Decl>| {
        if let Some(c) = part.find(':') {
            let name = part[..c].trim().to_ascii_lowercase();
            let mut value = part[c + 1..].trim().to_string();
            let mut important = false;
            if let Some(p) = value.to_ascii_lowercase().rfind("!important") {
                important = true;
                value.truncate(p);
                value = value.trim().to_string();
            }
            if !name.is_empty() { v.push(Decl { name, value, important }); }
        }
    };
    for i in 0..b.len() {
        let c = b[i];
        if q != 0 { if c == q { q = 0; } continue; }
        match c {
            b'"' | b'\'' => q = c,
            b'(' => depth += 1,
            b')' => depth -= 1,
            b';' if depth <= 0 => { push(&s[start..i], &mut v); start = i + 1; }
            _ => {}
        }
    }
    push(&s[start..], &mut v);
    v
}

fn parse_compound(s: &str) -> Option<Compound> {
    let mut c = Compound { tag: None, id: None, classes: Vec::new(), attrs: Vec::new(), pseudo: Vec::new(), never: false, not: Vec::new() };
    let b = s.as_bytes();
    let mut i = 0;
    let ident_end = |mut j: usize| { while j < b.len() && (b[j].is_ascii_alphanumeric() || b[j] == b'-' || b[j] == b'_' || b[j] >= 0x80 || b[j] == b'\\') { if b[j] == b'\\' { j += 1; } j += 1; } j.min(b.len()) };
    if i < b.len() && (b[i] == b'*') { i += 1; }
    else if i < b.len() && (b[i].is_ascii_alphabetic()) {
        let e = ident_end(i);
        c.tag = Some(s[i..e].to_ascii_lowercase());
        i = e;
    }
    while i < b.len() {
        match b[i] {
            b'#' => { let e = ident_end(i + 1); c.id = Some(unescape(&s[i + 1..e])); i = e; }
            b'.' => { let e = ident_end(i + 1); c.classes.push(unescape(&s[i + 1..e])); i = e; }
            b'[' => {
                let e = s[i..].find(']').map(|x| i + x)?;
                let inner = &s[i + 1..e];
                let ops = [("~=", AttrOp::Word), ("|=", AttrOp::Dash), ("^=", AttrOp::Prefix), ("$=", AttrOp::Suffix), ("*=", AttrOp::Contains), ("=", AttrOp::Eq)];
                let mut done = false;
                for (op, kind) in ops.iter() {
                    if let Some(p) = inner.find(op) {
                        let name = inner[..p].trim().to_ascii_lowercase();
                        let mut val = inner[p + op.len()..].trim();
                        if val.ends_with(" i") || val.ends_with(" s") { val = val[..val.len() - 2].trim(); }
                        let val = val.trim_matches(|c| c == '"' || c == '\'').to_string();
                        c.attrs.push((name, kind.clone(), val));
                        done = true;
                        break;
                    }
                }
                if !done { c.attrs.push((inner.trim().to_ascii_lowercase(), AttrOp::Exists, String::new())); }
                i = e + 1;
            }
            b':' => {
                let dbl = i + 1 < b.len() && b[i + 1] == b':';
                let st = i + if dbl { 2 } else { 1 };
                let e = ident_end(st);
                let name = s[st..e].to_ascii_lowercase();
                let mut j = e;
                let mut arg = "";
                if j < b.len() && b[j] == b'(' {
                    let mut depth = 1; let mut k = j + 1;
                    while k < b.len() && depth > 0 { if b[k] == b'(' { depth += 1 } else if b[k] == b')' { depth -= 1 } k += 1; }
                    arg = &s[j + 1..k.saturating_sub(1)];
                    j = k;
                }
                if dbl { c.never = true; }
                else {
                    match name.as_str() {
                        "not" => { for a in arg.split(',') { if let Some(n) = parse_compound(a.trim()) { c.not.push(n); } } }
                        "first-child" | "last-child" | "only-child" | "root" | "link" | "any-link" | "first-of-type" | "last-of-type" | "empty" | "checked" | "disabled" | "enabled" => c.pseudo.push(name),
                        "nth-child" | "nth-of-type" => c.pseudo.push(alloc::format!("{}({})", name, arg.trim())),
                        "is" | "where" | "matches" => {
                            if let Some(n) = arg.split(',').next().and_then(|a| parse_compound(a.trim())) {
                                if c.tag.is_none() { c.tag = n.tag; }
                                c.classes.extend(n.classes); c.attrs.extend(n.attrs);
                                if n.id.is_some() { c.id = n.id; }
                            }
                        }
                        "visited" | "focus-within" | "focus-visible" | "target" | "placeholder-shown" => c.never = true,
                        "hover" | "focus" | "active" | "before" | "after" | "first-line" | "first-letter" | "selection" | "placeholder" | "marker" => c.never = true,
                        _ => c.never = true,
                    }
                }
                i = j;
            }
            _ => return None,
        }
    }
    Some(c)
}

fn unescape(s: &str) -> String {
    if !s.contains('\\') { return s.to_string(); }
    let mut o = String::new();
    let mut esc = false;
    for ch in s.chars() { if esc { o.push(ch); esc = false; } else if ch == '\\' { esc = true; } else { o.push(ch); } }
    o
}

pub fn parse_selector(s: &str) -> Option<Selector> {
    let s = s.trim();
    if s.is_empty() { return None; }
    let mut tokens: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut depth = 0;
    for ch in s.chars() {
        match ch {
            '(' | '[' => { depth += 1; cur.push(ch); }
            ')' | ']' => { depth -= 1; cur.push(ch); }
            '>' | '+' | '~' if depth == 0 => {
                if !cur.trim().is_empty() { tokens.push(cur.trim().to_string()); }
                tokens.push(ch.to_string()); cur.clear();
            }
            c if c.is_whitespace() && depth == 0 => {
                if !cur.trim().is_empty() { tokens.push(cur.trim().to_string()); cur.clear(); }
            }
            _ => cur.push(ch),
        }
    }
    if !cur.trim().is_empty() { tokens.push(cur.trim().to_string()); }
    let mut parts: Vec<(Compound, Comb)> = Vec::new();
    let mut comb = Comb::Desc;
    let mut seq: Vec<(Compound, Comb)> = Vec::new();
    for t in tokens.iter() {
        match t.as_str() {
            ">" => comb = Comb::Child,
            "+" => comb = Comb::Adj,
            "~" => comb = Comb::Sib,
            _ => {
                let c = parse_compound(t)?;
                seq.push((c, comb));
                comb = Comb::Desc;
            }
        }
    }
    if seq.is_empty() { return None; }
    let len = seq.len();
    for i in (0..len).rev() {
        let link = if i > 0 { seq[i].1 } else { Comb::Desc };
        parts.push((seq[i].0.clone(), link));
    }
    let mut spec = 0u32;
    for (c, _) in parts.iter() {
        if c.id.is_some() { spec += 10000; }
        spec += (c.classes.len() + c.attrs.len() + c.pseudo.len()) as u32 * 100;
        if c.tag.is_some() { spec += 1; }
    }
    Some(Selector { parts, spec })
}

pub struct MediaEnv { pub width: i32 }

fn media_ok(q: &str, env: &MediaEnv) -> bool {
    let q = q.to_ascii_lowercase();
    for part in q.split(',') {
        let part = part.trim();
        if part.contains("print") && !part.contains("not print") { continue; }
        if part.contains("speech") { continue; }
        if part.contains("prefers-color-scheme: dark") || part.contains("prefers-color-scheme:dark") { continue; }
        if part.contains("prefers-reduced-motion") && !part.contains("no-preference") { }
        let mut ok = true;
        for cond in part.split("and") {
            let cond = cond.trim().trim_start_matches("only").trim().trim_start_matches("screen").trim();
            let cond = cond.trim_matches(|c| c == '(' || c == ')').trim();
            if let Some(v) = cond.strip_prefix("min-width:") { if let Some(px) = media_px(v) { if env.width < px { ok = false; } } }
            else if let Some(v) = cond.strip_prefix("max-width:") { if let Some(px) = media_px(v) { if env.width > px { ok = false; } } }
            else if cond.starts_with("width >=") || cond.starts_with("width>=") {
                if let Some(px) = media_px(cond.split(">=").nth(1).unwrap_or("")) { if env.width < px { ok = false; } }
            } else if cond.starts_with("width <") || cond.starts_with("width<") {
                if let Some(px) = media_px(cond.trim_start_matches("width").trim_start_matches(|c| c == ' ' || c == '<' || c == '=')) { if env.width >= px { ok = false; } }
            } else if cond.starts_with("hover") || cond.starts_with("pointer: fine") { }
            else if cond.starts_with("orientation: portrait") { ok = false; }
        }
        if ok { return true; }
    }
    false
}

fn media_px(v: &str) -> Option<i32> {
    let v = v.trim();
    let num: String = v.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
    let n: f32 = num.parse().ok()?;
    if v.ends_with("em") { Some((n * 16.0) as i32) } else { Some(n as i32) }
}

pub fn parse_sheet(src: &str, env: &MediaEnv, order_base: u32, imports: &mut Vec<String>) -> Sheet {
    let src = strip_comments(src);
    let mut rules = Vec::new();
    parse_block(&src, env, &mut rules, order_base, imports);
    Sheet { rules }
}

fn parse_block(src: &str, env: &MediaEnv, rules: &mut Vec<Rule>, order_base: u32, imports: &mut Vec<String>) {
    let b = src.as_bytes();
    let mut i = 0;
    while i < b.len() {
        while i < b.len() && (b[i].is_ascii_whitespace() || b[i] == b';' || b[i] == b'}') { i += 1; }
        if i >= b.len() { break; }
        if b[i] == b'@' {
            let semi = src[i..].find(';').map(|x| i + x);
            let brace = src[i..].find('{').map(|x| i + x);
            let head_end = match (semi, brace) { (Some(s), Some(bb)) => s.min(bb), (Some(s), None) => s, (None, Some(bb)) => bb, _ => b.len() };
            let head = &src[i..head_end];
            if head_end < b.len() && b[head_end] == b';' {
                if head.starts_with("@import") {
                    let u = head[7..].trim();
                    let u = u.trim_start_matches("url(").split(|c| c == ')' || c == ' ').next().unwrap_or("").trim_matches(|c| c == '"' || c == '\'');
                    if !u.is_empty() && imports.len() < 6 { imports.push(u.to_string()); }
                }
                i = head_end + 1;
                continue;
            }
            let close = match_brace(b, head_end + 1);
            let inner = &src[(head_end + 1).min(b.len())..close.min(b.len())];
            let hl = head.to_ascii_lowercase();
            if hl.starts_with("@media") {
                if media_ok(&hl[6..], env) { parse_block(inner, env, rules, order_base, imports); }
            } else if hl.starts_with("@supports") || hl.starts_with("@layer") || hl.starts_with("@container") || hl.starts_with("@scope") {
                if !hl.contains("not (") { parse_block(inner, env, rules, order_base, imports); }
            }
            i = close + 1;
            continue;
        }
        let brace = match src[i..].find('{') { Some(x) => i + x, None => break };
        let close = match_brace(b, brace + 1);
        let sel_text = &src[i..brace];
        let body = &src[brace + 1..close.min(b.len())];
        let sels: Vec<Selector> = split_top(sel_text, ',').iter().filter_map(|s| parse_selector(s)).collect();
        if !sels.is_empty() {
            let decls = parse_decls(body);
            if !decls.is_empty() {
                let order = order_base + rules.len() as u32;
                rules.push(Rule { sels, decls, order });
            }
        }
        i = close + 1;
    }
}

fn split_top(s: &str, sep: char) -> Vec<String> {
    let mut v = Vec::new();
    let mut depth = 0;
    let mut cur = String::new();
    for ch in s.chars() {
        match ch {
            '(' | '[' => { depth += 1; cur.push(ch); }
            ')' | ']' => { depth -= 1; cur.push(ch); }
            c if c == sep && depth == 0 => { v.push(core::mem::take(&mut cur)); }
            c => cur.push(c),
        }
    }
    if !cur.trim().is_empty() { v.push(cur); }
    v
}

fn attr_match(e: &Elem, name: &str, op: &AttrOp, val: &str) -> bool {
    let v = match e.attr(name) { Some(v) => v, None => return false };
    match op {
        AttrOp::Exists => true,
        AttrOp::Eq => v == val,
        AttrOp::Contains => !val.is_empty() && v.contains(val),
        AttrOp::Prefix => !val.is_empty() && v.starts_with(val),
        AttrOp::Suffix => !val.is_empty() && v.ends_with(val),
        AttrOp::Word => v.split_whitespace().any(|w| w == val),
        AttrOp::Dash => v == val || v.starts_with(&alloc::format!("{}-", val)),
    }
}

fn nth_match(expr: &str, idx1: i32) -> bool {
    let e: String = expr.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_ascii_lowercase();
    if e == "odd" { return idx1 % 2 == 1; }
    if e == "even" { return idx1 % 2 == 0; }
    if let Some(p) = e.find('n') {
        let a_s = &e[..p];
        let a: i32 = match a_s { "" | "+" => 1, "-" => -1, s => s.parse().unwrap_or(1) };
        let b: i32 = e[p + 1..].parse().unwrap_or(0);
        if a == 0 { return idx1 == b; }
        let d = idx1 - b;
        return d % a == 0 && d / a >= 0;
    }
    e.parse::<i32>().map(|v| v == idx1).unwrap_or(false)
}

pub fn compound_match(dom: &Dom, id: usize, c: &Compound) -> bool {
    if c.never { return false; }
    let e = match dom.elem(id) { Some(e) => e, None => return false };
    if let Some(t) = &c.tag { if *t != e.tag { return false; } }
    if let Some(i) = &c.id { if *i != e.id { return false; } }
    for cl in c.classes.iter() { if !e.classes.iter().any(|x| x == cl) { return false; } }
    for (n, op, v) in c.attrs.iter() { if !attr_match(e, n, op, v) { return false; } }
    for p in c.pseudo.iter() {
        let ok = match p.as_str() {
            "first-child" => dom.element_index_among_siblings(id).0 == 0,
            "last-child" => { let (i, t) = dom.element_index_among_siblings(id); i + 1 == t }
            "only-child" => dom.element_index_among_siblings(id).1 == 1,
            "first-of-type" => dom.element_index_among_siblings(id).0 == 0,
            "last-of-type" => { let (i, t) = dom.element_index_among_siblings(id); i + 1 == t }
            "root" => e.tag == "html",
            "link" | "any-link" => e.tag == "a" && e.attr("href").is_some(),
            "empty" => dom.nodes[id].children.is_empty(),
            "checked" => e.attr("checked").is_some() || e.attr("selected").is_some(),
            "disabled" => e.attr("disabled").is_some(),
            "enabled" => e.attr("disabled").is_none(),
            s if s.starts_with("nth-child(") || s.starts_with("nth-of-type(") => {
                let arg = &s[s.find('(').unwrap() + 1..s.len() - 1];
                nth_match(arg, dom.element_index_among_siblings(id).0 as i32 + 1)
            }
            _ => false,
        };
        if !ok { return false; }
    }
    for n in c.not.iter() { if compound_match(dom, id, n) { return false; } }
    true
}

pub fn selector_match(dom: &Dom, id: usize, sel: &Selector) -> bool {
    match_from(dom, id, sel, 0)
}

fn match_from(dom: &Dom, id: usize, sel: &Selector, k: usize) -> bool {
    let (c, comb) = &sel.parts[k];
    if !compound_match(dom, id, c) { return false; }
    if k + 1 == sel.parts.len() { return true; }
    match comb {
        Comb::Child => {
            let p = dom.nodes[id].parent;
            p != usize::MAX && match_from(dom, p, sel, k + 1)
        }
        Comb::Desc => {
            let mut p = dom.nodes[id].parent;
            while p != usize::MAX {
                if match_from(dom, p, sel, k + 1) { return true; }
                p = dom.nodes[p].parent;
            }
            false
        }
        Comb::Adj => match dom.prev_element_sibling(id) { Some(s) => match_from(dom, s, sel, k + 1), None => false },
        Comb::Sib => {
            let mut s = dom.prev_element_sibling(id);
            while let Some(x) = s {
                if match_from(dom, x, sel, k + 1) { return true; }
                s = dom.prev_element_sibling(x);
            }
            false
        }
    }
}
