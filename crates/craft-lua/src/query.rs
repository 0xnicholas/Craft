use std::collections::HashMap;

use serde_json::Value;

pub type QueryResult = Result<Value, String>;

pub trait QueryHandler: 'static {
    fn call(&self, args: Value) -> QueryResult;
}

impl<F> QueryHandler for F
where
    F: Fn(Value) -> QueryResult + 'static,
{
    fn call(&self, args: Value) -> QueryResult {
        (self)(args)
    }
}

#[derive(Default)]
pub struct QueryRegistry {
    handlers: HashMap<String, Box<dyn QueryHandler>>,
}

impl QueryRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<F>(&mut self, name: impl Into<String>, handler: F) -> &mut Self
    where
        F: Fn(Value) -> QueryResult + 'static,
    {
        self.handlers.insert(name.into(), Box::new(handler));
        self
    }

    pub fn call(&self, name: &str, args: Value) -> QueryResult {
        match self.handlers.get(name) {
            Some(handler) => handler.call(args),
            None => Err(format!("unknown query \"{name}\"")),
        }
    }

    pub fn contains(&self, name: &str) -> bool {
        self.handlers.contains_key(name)
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.handlers.keys().map(|s| s.as_str())
    }
}
