#[derive(Debug, Clone)]
pub enum Decl {
    BlockItem(Ident, Vec<Box<Decl>>),
    SimpleItem(Ident, Box<Value>),
}

#[derive(Debug, Clone)]
pub struct Ident(pub String);

#[derive(Debug, Clone)]
pub enum Value {
    Int(i32),
    Ident(Ident),
    String(String),
}
