use alloc::string::String;
use alloc::vec::Vec;

pub enum NodeData {
    Document,
    Element(Elem),
    Text(String),
}

pub struct Elem {
    pub tag: String,
    pub attrs: Vec<(String, String)>,
    pub id: String,
    pub classes: Vec<String>,
}

impl Elem {
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }
}

pub struct Node {
    pub parent: usize,
    pub children: Vec<usize>,
    pub data: NodeData,
}

pub struct Dom {
    pub nodes: Vec<Node>,
}

pub const ROOT: usize = 0;

impl Dom {
    pub fn new() -> Dom {
        Dom { nodes: alloc::vec![Node { parent: usize::MAX, children: Vec::new(), data: NodeData::Document }] }
    }
    pub fn add(&mut self, parent: usize, data: NodeData) -> usize {
        let id = self.nodes.len();
        self.nodes.push(Node { parent, children: Vec::new(), data });
        self.nodes[parent].children.push(id);
        id
    }
    pub fn elem(&self, id: usize) -> Option<&Elem> {
        match &self.nodes[id].data { NodeData::Element(e) => Some(e), _ => None }
    }
    pub fn tag(&self, id: usize) -> &str {
        match &self.nodes[id].data { NodeData::Element(e) => &e.tag, _ => "" }
    }
    pub fn find_first(&self, tag: &str) -> Option<usize> {
        (0..self.nodes.len()).find(|&i| self.tag(i) == tag)
    }
    pub fn text_content(&self, id: usize) -> String {
        let mut s = String::new();
        self.collect_text(id, &mut s);
        s
    }
    fn collect_text(&self, id: usize, out: &mut String) {
        match &self.nodes[id].data {
            NodeData::Text(t) => out.push_str(t),
            _ => for &c in self.nodes[id].children.iter() { self.collect_text(c, out); },
        }
    }
    pub fn element_index_among_siblings(&self, id: usize) -> (usize, usize) {
        let p = self.nodes[id].parent;
        if p == usize::MAX { return (0, 1); }
        let mut idx = 0;
        let mut total = 0;
        for &c in self.nodes[p].children.iter() {
            if let NodeData::Element(_) = self.nodes[c].data {
                if c == id { idx = total; }
                total += 1;
            }
        }
        (idx, total)
    }
    pub fn prev_element_sibling(&self, id: usize) -> Option<usize> {
        let p = self.nodes[id].parent;
        if p == usize::MAX { return None; }
        let mut prev = None;
        for &c in self.nodes[p].children.iter() {
            if c == id { return prev; }
            if let NodeData::Element(_) = self.nodes[c].data { prev = Some(c); }
        }
        None
    }
}
