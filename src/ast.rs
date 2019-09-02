#[derive(Debug, Clone)]
pub struct Item {
    pub ident: String,
    pub value: Value,
}

#[derive(Debug, Clone)]
pub enum Value {
    List(Vec<Item>),
    Int(i32),
    Ident(String),
    String(String),
}
