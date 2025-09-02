use std::collections::HashMap;
use serde::Deserialize;


#[derive(Clone, Deserialize, Debug)]
pub struct Entry {
    pub convert: Option<String>,
    pub children: Option<HashMap<u32, Entry>>,
    pub data_type: Option<String>,
    pub json_path: Option<String>,
    pub literal: Option<String>,
    pub name: String,
    pub oid: Vec<u32>,
    pub unit: Option<String>,
}


/// For now, this iterator could work on anything with a `children` field.
pub struct RecursiveIterator<'a> {
    // These items will be visited in the future.
    stack: Vec<&'a Entry>,
}

impl<'a> RecursiveIterator<'a> {
    pub(crate) fn new(root: &'a HashMap<u32, Entry>) -> Self {
        let mut stack = Vec::new();
        for entry in root.values() {
            stack.push(entry);
        }
        Self { stack }
    }
}

impl<'a> Iterator for RecursiveIterator<'a> {
    type Item = &'a Entry;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(entry) = self.stack.pop() {
            if let Some(children) = &entry.children {
                for child in children.values() {
                    self.stack.push(child);
                }
            }
            return Some(entry);
        }
        None
    }
}
