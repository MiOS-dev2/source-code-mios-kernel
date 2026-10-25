use alloc::collections::BTreeMap;
use alloc::rc::Rc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format;
use super::css::*;
use super::dom::*;
use super::style::*;

struct Index { map: BTreeMap<String, Vec<(u16, u32, u16)>>, }

fn key_of(sel: &Selector) -> String {
    let c = &sel.parts[0].0;
    if let Some(i) = &c.id { return format!("#{}", i); }
    if let Some(cl) = c.classes.first() { return format!(".{}", cl); }
    if let Some(t) = &c.tag { return format!("t{}", t); }
    String::from("*")
}

fn build_index(sheets: &[Sheet]) -> Index {
    let mut map: BTreeMap<String, Vec<(u16, u32, u16)>> = BTreeMap::new();
    for (si, sh) in sheets.iter().enumerate() {
        for (ri, r) in sh.rules.iter().enumerate() {
            for (sj, s) in r.sels.iter().enumerate() {
                map.entry(key_of(s)).or_insert_with(Vec::new).push((si as u16, ri as u32, sj as u16));
            }
        }
    }
    Index { map }
}

fn subst_vars(v: &str, vars: &[(String, String)], depth: u32) -> String {
    if !v.contains("var(") || depth > 8 { return v.to_string(); }
    let mut out = String::new();
    let mut rest = v;
    while let Some(i) = rest.find("var(") {
        out.push_str(&rest[..i]);
        let after = &rest[i + 4..];
        let mut d = 1; let mut e = after.len();
        for (k, ch) in after.char_indices() { if ch == '(' { d += 1 } else if ch == ')' { d -= 1; if d == 0 { e = k; break; } } }
        let inner = &after[..e];
        let (name, fallback) = match inner.find(',') { Some(c) => (inner[..c].trim(), Some(inner[c + 1..].trim())), None => (inner.trim(), None) };
        let val = vars.iter().rev().find(|(k, _)| k == name).map(|(_, v)| v.as_str()).or(fallback).unwrap_or("");
        out.push_str(&subst_vars(val, vars, depth + 1));
        rest = if e + 1 <= after.len() { &after[(e + 1).min(after.len())..] } else { "" };
    }
    out.push_str(rest);
    out
}

fn attr_len(v: &str) -> Option<Len> {
    let v = v.trim();
    if v.ends_with('%') { v[..v.len() - 1].trim().parse::<f32>().ok().map(Len::Pct) }
    else { v.trim_end_matches("px").parse::<f32>().ok().map(Len::Px) }
}

fn presentational(e: &Elem, s: &mut Style, parent: &Style) {
    if let Some(c) = e.attr("bgcolor") { if let Some(c) = parse_color(c, s.color) { s.bg = c; } }
    match e.tag.as_str() {
        "font" => {
            if let Some(c) = e.attr("color") { if let Some(c) = parse_color(c, s.color) { s.color = c; } }
            if let Some(sz) = e.attr("size") {
                let t = sz.trim();
                let n: i32 = if let Some(x) = t.strip_prefix('+') { 3 + x.parse::<i32>().unwrap_or(0) } else if let Some(x) = t.strip_prefix('-') { 3 - x.parse::<i32>().unwrap_or(0) } else { t.parse().unwrap_or(3) };
                s.font_size = match n { i32::MIN..=1 => 10.0, 2 => 13.0, 3 => 16.0, 4 => 18.0, 5 => 24.0, 6 => 32.0, _ => 48.0 };
            }
            if let Some(f) = e.attr("face") { let f = f.to_ascii_lowercase(); if f.contains("courier") || f.contains("mono") { s.mono = true; } }
        }
        "body" => {
            if let Some(c) = e.attr("text") { if let Some(c) = parse_color(c, s.color) { s.color = c; } }
        }
        "table" => {
            if let Some(b) = e.attr("border") { let w: f32 = b.trim().parse().unwrap_or(1.0); if w > 0.0 { s.border_w = [w; 4]; s.border_c = [0xFF808080; 4]; } }
            if let Some(a) = e.attr("align") { if a.eq_ignore_ascii_case("center") { s.margin[1] = Len::Auto; s.margin[3] = Len::Auto; } }
        }
        "td" | "th" | "tr" | "tbody" | "thead" => {
            if let Some(a) = e.attr("align") { s.align = match a.to_ascii_lowercase().as_str() { "center" => 1, "right" => 2, _ => 0 }; }
            if e.attr("nowrap").is_some() { s.ws = 2; }
            if let Some(v) = e.attr("valign") { s.valign = match v.to_ascii_lowercase().as_str() { "top" => 2, "bottom" => 3, _ => 1 }; }
        }
        "div" | "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
            if let Some(a) = e.attr("align") { s.align = match a.to_ascii_lowercase().as_str() { "center" | "middle" => 1, "right" => 2, "justify" => 3, _ => 0 }; }
        }
        "hr" => {
            if let Some(c) = e.attr("color") { if let Some(c) = parse_color(c, s.color) { s.border_c = [c; 4]; s.bg = c; } }
            if let Some(w) = e.attr("width") { if let Some(l) = attr_len(w) { s.width = l; } }
        }
        "ul" | "ol" => {
            if let Some(t) = e.attr("type") { s.list = match t { "1" => 4, "a" => 5, "A" => 6, "circle" => 2, "square" => 3, "disc" => 1, _ => s.list }; }
        }
        _ => {}
    }
    if matches!(e.tag.as_str(), "table" | "td" | "th" | "img" | "iframe" | "canvas" | "video" | "embed" | "object" | "col" | "input") {
        if let Some(w) = e.attr("width") { if let Some(l) = attr_len(w) { s.width = l; } }
        if let Some(h) = e.attr("height") { if let Some(l) = attr_len(h) { if e.tag != "td" && e.tag != "th" { s.height = l; } } }
    }
    if e.tag == "img" {
        if let Some(a) = e.attr("align") { let a = a.to_ascii_lowercase(); if a == "left" { s.float = 1; } else if a == "right" { s.float = 2; } }
    }
    let _ = parent;
}

pub fn compute(dom: &Dom, sheets: &[Sheet]) -> Vec<Style> {
    let idx = build_index(sheets);
    let mut styles: Vec<Style> = Vec::with_capacity(dom.nodes.len());
    let root_style = Style::initial();
    styles.push({ let mut s = root_style.clone(); s.display = Display::Block; s });
    let empty: Vec<(u16, u32, u16)> = Vec::new();
    for id in 1..dom.nodes.len() {
        let parent_id = dom.nodes[id].parent;
        let parent = styles[parent_id].clone();
        let e = match &dom.nodes[id].data {
            NodeData::Element(e) => e,
            _ => { let mut s = Style::inherit_pub(&parent); s.display = Display::Inline; styles.push(s); continue; }
        };
        let mut s = Style::inherit_pub(&parent);
        let mut cand: Vec<(u16, u32, u16)> = Vec::new();
        let mut add = |k: &str| { if let Some(v) = idx.map.get(k) { cand.extend_from_slice(v); } };
        if !e.id.is_empty() { add(&format!("#{}", e.id)); }
        for c in e.classes.iter() { add(&format!(".{}", c)); }
        add(&format!("t{}", e.tag));
        add("*");
        let _ = &empty;
        let mut matched: Vec<(u32, u32, u16, u32)> = Vec::new();
        for (si, ri, sj) in cand.iter() {
            let r = &sheets[*si as usize].rules[*ri as usize];
            let sel = &r.sels[*sj as usize];
            if selector_match(dom, id, sel) {
                matched.push((sel.spec, r.order, *si, *ri));
            }
        }
        matched.sort_by(|a, b| (a.2 != 0, a.0, a.1).cmp(&(b.2 != 0, b.0, b.1)));
        matched.dedup_by(|a, b| a.2 == b.2 && a.3 == b.3);
        let inline: Vec<Decl> = e.attr("style").map(parse_decls).unwrap_or_default();
        let mut decls: Vec<(&Decl, u8)> = Vec::new();
        let mut ua_done = false;
        let mut phase_decls: Vec<(u8, &Decl)> = Vec::new();
        for m in matched.iter() {
            let r = &sheets[m.2 as usize].rules[m.3 as usize];
            for d in r.decls.iter() { phase_decls.push((if m.2 == 0 { 0 } else { 1 }, d)); }
        }
        for d in inline.iter() { phase_decls.push((2, d)); }
        let mut vars: Option<Vec<(String, String)>> = None;
        for (_, d) in phase_decls.iter() {
            if d.name.starts_with("--") {
                let v = vars.get_or_insert_with(|| (*s.vars).clone());
                v.push((d.name.clone(), d.value.clone()));
            }
        }
        if let Some(v) = vars { s.vars = Rc::new(v); }
        for (phase, d) in phase_decls.iter() {
            if *phase >= 1 && !ua_done { presentational(e, &mut s, &parent); ua_done = true; }
            if d.important { decls.push((d, 1)); continue; }
            if d.name.starts_with("--") { continue; }
            let val = subst_vars(&d.value, &s.vars, 0);
            apply(&mut s, &parent, &d.name, &val);
        }
        if !ua_done { presentational(e, &mut s, &parent); }
        for (d, _) in decls.iter() {
            if d.name.starts_with("--") { continue; }
            let val = subst_vars(&d.value, &s.vars, 0);
            apply(&mut s, &parent, &d.name, &val);
        }
        if e.attr("hidden").is_some() { s.display = Display::None; }
        if let Some(t) = e.attr("type") { if e.tag == "input" && t.eq_ignore_ascii_case("hidden") { s.display = Display::None; } }
        if s.pos == 2 || s.pos == 3 {
            let tiny = |l: Len| matches!(l, Len::Px(p) if p <= 2.0);
            if tiny(s.width) || tiny(s.height) { s.sr_only = true; }
        }
        if s.text_indent < -500.0 { s.sr_only = true; }
        styles.push(s);
    }
    styles
}

impl Style {
    pub fn inherit_pub(p: &Style) -> Style {
        let mut s = Style::initial();
        s.color = p.color; s.font_size = p.font_size; s.bold = p.bold; s.italic = p.italic; s.mono = p.mono;
        s.underline = p.underline; s.strike = p.strike; s.align = p.align; s.ws = p.ws; s.line_h = p.line_h;
        s.list = p.list; s.visible = p.visible; s.transform = p.transform; s.vars = p.vars.clone();
        s
    }
}
