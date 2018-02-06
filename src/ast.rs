use std::rc::Rc;


#[derive(Debug, Clone)]
pub enum Expr {
    Var(String),
    Block(Rc<[Statement]>),
    Spawn(Box<Expr>),
    Receive,
    Symbol(Rc<str>),
    Root,
}

impl Expr {
    pub fn receive_count(&self) -> usize {
        match *self {
            Expr::Block(_) |
            Expr::Var(_) |
            Expr::Symbol(_) |
            Expr::Root |
            Expr::Spawn(_) => 0,
            Expr::Receive => 1,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Statement {
    Bind(Rc<str>, Expr),
    Send(Expr, Expr),
    Expr(Expr),
}

impl Statement {
    pub fn receive_count(&self) -> usize {
        match *self {
            Statement::Bind(_, ref e) |
            Statement::Expr(ref e) => e.receive_count(),
            Statement::Send(ref e1, ref e2) => e1.receive_count() + e2.receive_count(),
        }
    }
}
