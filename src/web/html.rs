use alloc::string::{String, ToString};
use alloc::vec::Vec;
use super::dom::*;

const VOID: &[&str] = &["area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source", "track", "wbr", "keygen"];
const RAW: &[&str] = &["script", "style", "textarea", "title", "xmp", "noframes", "iframe", "plaintext", "svg"];

fn is_void(t: &str) -> bool { VOID.contains(&t) }

fn auto_close(open: &str, new: &str) -> bool {
    match open {
        "p" => matches!(new, "p" | "div" | "ul" | "ol" | "table" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "pre" | "form" | "blockquote" | "section" | "article" | "header" | "footer" | "nav" | "aside" | "dl" | "hr" | "address" | "fieldset" | "figure" | "main" | "menu" | "details"),
        "li" => new == "li",
        "dt" | "dd" => new == "dt" || new == "dd",
        "option" => new == "option" || new == "optgroup",
        "tr" => new == "tr" || new == "tbody" || new == "thead" || new == "tfoot",
        "td" | "th" => matches!(new, "td" | "th" | "tr" | "tbody" | "thead" | "tfoot"),
        "thead" | "tbody" | "tfoot" => matches!(new, "tbody" | "thead" | "tfoot"),
        _ => false,
    }
}

pub fn decode_entities(s: &str) -> String {
    if !s.contains('&') { return s.to_string(); }
    let mut out = String::with_capacity(s.len());
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'&' {
            let rest = &s[i + 1..];
            let end = rest.find(|c: char| c == ';' || c == '&' || c == '<' || c.is_whitespace()).unwrap_or(rest.len()).min(32);
            let name = &rest[..end];
            let has_semi = rest[end..].starts_with(';');
            let mut done = false;
            if let Some(num) = name.strip_prefix('#') {
                let v = if let Some(h) = num.strip_prefix('x').or_else(|| num.strip_prefix('X')) {
                    u32::from_str_radix(h, 16).ok()
                } else { num.parse::<u32>().ok() };
                if let Some(v) = v {
                    let v = match v { 0x80..=0x9F => win1252(v as u8) as u32, _ => v };
                    out.push(char::from_u32(v).unwrap_or('?'));
                    i += 1 + end + if has_semi { 1 } else { 0 };
                    done = true;
                }
            } else if let Some(c) = named_entity(name) {
                out.push_str(c);
                i += 1 + end + if has_semi { 1 } else { 0 };
                done = true;
            }
            if !done { out.push('&'); i += 1; }
        } else {
            let ch = s[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

fn named_entity(n: &str) -> Option<&'static str> {
    Some(match n {
        "amp" | "AMP" => "&", "lt" | "LT" => "<", "gt" | "GT" => ">", "quot" | "QUOT" => "\"", "apos" => "'",
        "nbsp" => "\u{00A0}", "copy" => "©", "reg" => "®", "trade" => "™", "laquo" => "«", "raquo" => "»",
        "mdash" => "—", "ndash" => "–", "hellip" => "…", "bull" => "•", "middot" => "·", "rsquo" => "’",
        "lsquo" => "‘", "rdquo" => "”", "ldquo" => "“", "bdquo" => "„", "times" => "×", "divide" => "÷",
        "deg" => "°", "plusmn" => "±", "euro" => "€", "pound" => "£", "yen" => "¥", "cent" => "¢",
        "sect" => "§", "para" => "¶", "rarr" => "→", "larr" => "←", "uarr" => "↑", "darr" => "↓",
        "harr" => "↔", "hArr" => "⇔", "rArr" => "⇒", "frac12" => "½", "frac14" => "¼", "frac34" => "¾", "sup2" => "²",
        "sup3" => "³", "iexcl" => "¡", "iquest" => "¿", "shy" => "", "zwnj" => "", "zwj" => "", "lrm" => "", "rlm" => "",
        "thinsp" => " ", "ensp" => " ", "emsp" => " ", "numero" => "№", "check" => "✓", "minus" => "−",
        "eacute" => "é", "egrave" => "è", "aacute" => "á", "agrave" => "à", "ouml" => "ö", "uuml" => "ü",
        "auml" => "ä", "szlig" => "ß", "ccedil" => "ç", "ntilde" => "ñ", "oacute" => "ó", "iacute" => "í", "uacute" => "ú",
        _ => return None,
    })
}

fn win1252(b: u8) -> char {
    match b {
        0x80 => '€', 0x82 => '‚', 0x84 => '„', 0x85 => '…', 0x91 => '‘', 0x92 => '’', 0x93 => '“',
        0x94 => '”', 0x95 => '•', 0x96 => '–', 0x97 => '—', 0x99 => '™', _ => '?',
    }
}

struct P<'a> { s: &'a str, b: &'a [u8], i: usize }

impl<'a> P<'a> {
    fn starts_ci(&self, t: &str) -> bool {
        let e = self.i + t.len();
        e <= self.b.len() && self.s.is_char_boundary(e) && self.s[self.i..e].eq_ignore_ascii_case(t)
    }
    fn find_ci(&self, from: usize, t: &str) -> Option<usize> {
        let tb = t.as_bytes();
        if tb.is_empty() { return Some(from); }
        let mut j = from;
        while j + tb.len() <= self.b.len() {
            if self.b[j].to_ascii_lowercase() == tb[0] && self.b[j..j + tb.len()].eq_ignore_ascii_case(tb) { return Some(j); }
            j += 1;
        }
        None
    }
}

fn parse_attrs(src: &str) -> (Vec<(String, String)>, bool) {
    let b = src.as_bytes();
    let mut i = 0;
    let mut attrs = Vec::new();
    let mut self_close = false;
    while i < b.len() {
        while i < b.len() && (b[i].is_ascii_whitespace() || b[i] == b'/') {
            if b[i] == b'/' { self_close = true; }
            i += 1;
        }
        if i >= b.len() { break; }
        self_close = false;
        let ns = i;
        while i < b.len() && !b[i].is_ascii_whitespace() && b[i] != b'=' && b[i] != b'/' && b[i] != b'>' { i += 1; }
        if i == ns && (i >= b.len() || b[i] != b'=') { i += 1; continue; }
        let name = if src.is_char_boundary(ns) && src.is_char_boundary(i) { src[ns..i].to_ascii_lowercase() } else { String::new() };
        while i < b.len() && b[i].is_ascii_whitespace() { i += 1; }
        let mut val = String::new();
        if i < b.len() && b[i] == b'=' {
            i += 1;
            while i < b.len() && b[i].is_ascii_whitespace() { i += 1; }
            if i < b.len() && (b[i] == b'"' || b[i] == b'\'') {
                let q = b[i];
                i += 1;
                let vs = i;
                while i < b.len() && b[i] != q { i += 1; }
                val = decode_entities(&src[vs..i]);
                i += 1;
            } else {
                let vs = i;
                while i < b.len() && !b[i].is_ascii_whitespace() && b[i] != b'>' { i += 1; }
                val = decode_entities(&src[vs..i]);
            }
        }
        if !name.is_empty() && !attrs.iter().any(|(k, _): &(String, String)| *k == name) { attrs.push((name, val)); }
    }
    (attrs, self_close)
}

pub fn parse(src: &str) -> Dom {
    let mut dom = Dom::new();
    let mut stack: Vec<usize> = alloc::vec![ROOT];
    let mut p = P { s: src, b: src.as_bytes(), i: 0 };
    let n = p.b.len();
    let mut text_start = 0;

    fn flush_text(dom: &mut Dom, stack: &[usize], t: &str) {
        if t.is_empty() { return; }
        let parent = *stack.last().unwrap();
        let dec = decode_entities(t);
        if let Some(&last) = dom.nodes[parent].children.last() {
            if let NodeData::Text(ref mut s) = dom.nodes[last].data { s.push_str(&dec); return; }
        }
        dom.add(parent, NodeData::Text(dec));
    }

    while p.i < n {
        if p.b[p.i] != b'<' { p.i += 1; continue; }
        if p.starts_ci("<!--") {
            flush_text(&mut dom, &stack, &src[text_start..p.i]);
            let e = p.find_ci(p.i + 4, "-->").map(|x| x + 3).unwrap_or(n);
            p.i = e; text_start = e; continue;
        }
        if p.starts_ci("<!") || p.starts_ci("<?") {
            flush_text(&mut dom, &stack, &src[text_start..p.i]);
            let e = src[p.i..].find('>').map(|x| p.i + x + 1).unwrap_or(n);
            p.i = e; text_start = e; continue;
        }
        let closing = p.i + 1 < n && p.b[p.i + 1] == b'/';
        let ns = p.i + if closing { 2 } else { 1 };
        if ns >= n || !(p.b[ns].is_ascii_alphabetic()) { p.i += 1; continue; }
        flush_text(&mut dom, &stack, &src[text_start..p.i]);
        let mut ne = ns;
        while ne < n && (p.b[ne].is_ascii_alphanumeric() || p.b[ne] == b'-' || p.b[ne] == b':' || p.b[ne] == b'_') { ne += 1; }
        let tag = src[ns..ne].to_ascii_lowercase();
        let mut j = ne;
        let mut q = 0u8;
        while j < n {
            let c = p.b[j];
            if q != 0 { if c == q { q = 0; } }
            else if c == b'"' || c == b'\'' { q = c; }
            else if c == b'>' { break; }
            j += 1;
        }
        let tag_end = (j + 1).min(n);
        if closing {
            if let Some(pos) = stack.iter().rposition(|&id| dom.tag(id) == tag) {
                if pos > 0 { stack.truncate(pos); }
            }
            p.i = tag_end; text_start = tag_end; continue;
        }
        let attr_src = if ne < j { &src[ne..j.min(n)] } else { "" };
        let (attrs, self_close) = parse_attrs(attr_src);
        loop {
            let top = *stack.last().unwrap();
            if stack.len() > 1 && auto_close(dom.tag(top), &tag) { stack.pop(); } else { break; }
        }
        if tag == "li" || tag == "dt" || tag == "dd" {
            if let Some(pos) = stack.iter().rposition(|&id| { let t = dom.tag(id); t == "li" || t == "ul" || t == "ol" }) {
                if dom.tag(stack[pos]) == "li" && tag == "li" { stack.truncate(pos); }
            }
        }
        if tag == "tr" || tag == "td" || tag == "th" {
            if let Some(pos) = stack.iter().rposition(|&id| dom.tag(id) == "table") {
                let _ = pos;
            }
        }
        let id_attr = attrs.iter().find(|(k, _)| k == "id").map(|(_, v)| v.clone()).unwrap_or_default();
        let classes: Vec<String> = attrs.iter().find(|(k, _)| k == "class").map(|(_, v)| v.split_whitespace().map(|x| x.to_string()).collect()).unwrap_or_default();
        let parent = *stack.last().unwrap();
        let id = dom.add(parent, NodeData::Element(Elem { tag: tag.clone(), attrs, id: id_attr, classes }));
        p.i = tag_end;
        text_start = tag_end;
        if is_void(&tag) || (self_close && !matches!(tag.as_str(), "div" | "span" | "a" | "p" | "td" | "li")) { continue; }
        if RAW.contains(&tag.as_str()) {
            let close = alloc::format!("</{}", tag);
            let e = p.find_ci(p.i, &close).unwrap_or(n);
            let raw = &src[p.i..e];
            if !raw.is_empty() {
                let t = if tag == "textarea" || tag == "title" { decode_entities(raw) } else { raw.to_string() };
                dom.add(id, NodeData::Text(t));
            }
            let after = src[e..].find('>').map(|x| e + x + 1).unwrap_or(n);
            p.i = after; text_start = after;
            continue;
        }
        stack.push(id);
    }
    flush_text(&mut dom, &stack, &src[text_start..n]);
    dom
}
