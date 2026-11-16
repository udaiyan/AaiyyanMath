use crate::value::{Function, FunctionBody, Value};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};

/// Worksheet scope plus function-local scopes (spec section 9).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Environment {
    scopes: Vec<HashMap<String, Value>>,
}

impl Environment {
    pub fn new() -> Environment {
        let mut globals = HashMap::new();
        globals.insert("pi".to_string(), Value::Scalar(std::f64::consts::PI));
        globals.insert("\u{03C0}".to_string(), Value::Scalar(std::f64::consts::PI));
        globals.insert("e".to_string(), Value::Scalar(std::f64::consts::E));
        globals.insert("i".to_string(), Value::Complex { re: 0.0, im: 1.0 });
        globals.insert("inf".to_string(), Value::Scalar(f64::INFINITY));
        Environment { scopes: vec![globals] }
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        for scope in self.scopes.iter().rev() {
            if let Some(v) = scope.get(name) { return Some(v); }
        }
        None
    }

    pub fn define_global(&mut self, name: String, value: Value) {
        if self.scopes.is_empty() { self.scopes.push(HashMap::new()); }
        self.scopes[0].insert(name, value);
    }

    pub fn remove_global(&mut self, name: &str) {
        if let Some(first) = self.scopes.first_mut() { first.remove(name); }
    }

    pub fn push_scope(&mut self, scope: HashMap<String, Value>) { self.scopes.push(scope); }
    pub fn pop_scope(&mut self) { if self.scopes.len() > 1 { self.scopes.pop(); } }

    pub fn globals(&self) -> &HashMap<String, Value> { &self.scopes[0] }

    pub fn names(&self) -> BTreeSet<String> {
        self.scopes.iter().flat_map(|s| s.keys().cloned()).collect()
    }

    pub fn define_builtin(&mut self, name: &str, params: &[&str]) {
        self.define_global(name.to_string(), Value::Function(Function {
            name: name.to_string(),
            params: params.iter().map(|s| s.to_string()).collect(),
            body: FunctionBody::Builtin(name.to_string()),
        }));
    }
}
