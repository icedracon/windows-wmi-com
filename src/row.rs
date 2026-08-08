use std::collections::HashMap;

use crate::value::WmiValue;

#[derive(Debug, Clone, Default)]
pub struct Row {
    pub fields: HashMap<String, WmiValue>,
}

impl Row {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, name: &str) -> Option<&WmiValue> {
        self.fields.get(name)
    }

    pub fn take(&mut self, name: &str) -> Option<WmiValue> {
        self.fields.remove(name)
    }

    pub fn insert(&mut self, name: impl Into<String>, value: WmiValue) {
        self.fields.insert(name.into(), value);
    }

    pub fn len(&self) -> usize {
        self.fields.len()
    }

    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }
}
