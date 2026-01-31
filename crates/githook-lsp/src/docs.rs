use std::collections::HashMap;
use once_cell::sync::Lazy;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct DocEntry {
    pub name: String,
    pub description: String,
    pub example: String,
}

#[derive(Debug, Deserialize)]
struct DocsDatabase {
    properties: HashMap<String, DocEntry>,
    methods: HashMap<String, DocEntry>,
}

static DOCS: Lazy<DocsDatabase> = Lazy::new(|| {
    let json = include_str!("generated_docs.json");
    serde_json::from_str(json).expect("Failed to parse docs.json")
});

pub fn get_property_doc(full_path: &str) -> Option<&'static DocEntry> {
    DOCS.properties.get(full_path)
}

pub fn get_method_doc(name: &str) -> Option<&'static DocEntry> {
    DOCS.methods.get(name)
}
