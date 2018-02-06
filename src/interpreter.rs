use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::rc::Rc;
use ast;


#[derive(Debug)]
pub enum Error {
    SendToNonActor,
    UnboundVariable(String),
    SpawningNonBlock,
    RootDeadlock,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::SendToNonActor => write!(f, "tried to send message to non-actor"),
            Error::UnboundVariable(ref var) => write!(f, "unbound variable: `{}`", var),
            Error::SpawningNonBlock => write!(f, "tried to spawn a non-block"),
            Error::RootDeadlock => write!(f, "root got into a deadlock"),
        }
    }
}

pub type Result<T> = ::std::result::Result<T, Error>;

#[derive(Debug, Clone, Default)]
pub struct Env {
    bindings: HashMap<Rc<str>, Value>,
    next: Option<Rc<Env>>,
}

impl Env {
    fn lookup(&self, name: &str) -> Result<Value> {
        if let Some(value) = self.bindings.get(name).cloned() {
            Ok(value)
        } else if let Some(ref env) = self.next {
            env.lookup(name)
        } else {
            Err(Error::UnboundVariable(String::from(name)))
        }
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    ActorHandle(u64),
    Symbol(Rc<str>),
    Closure(Rc<Env>, Rc<[ast::Statement]>),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Value::ActorHandle(handle) => write!(f, "<actor {}>", handle),
            Value::Symbol(ref sym) => write!(f, "'{}", sym),
            Value::Closure(_, _) => write!(f, "<closure ...>"),
        }
    }
}

#[derive(Debug, Clone)]
struct Actor {
    code: Rc<[ast::Statement]>,
    position: usize,
    env: Env,
    queue: VecDeque<Value>,
}

impl Default for Actor {
    fn default() -> Actor {
        Actor {
            code: Vec::new().into(),
            position: 0,
            env: Env::default(),
            queue: VecDeque::default(),
        }
    }
}

impl Actor {
    fn is_completed(&self) -> bool {
        self.position >= self.code.len()
    }

    fn eval_expr<F>(&mut self, expr: &ast::Expr, spawner: &mut F) -> Result<Value>
    where
        F: FnMut(Actor) -> Value
    {
        match *expr {
            ast::Expr::Block(ref stmts) => {
                Ok(Value::Closure(Rc::new(self.env.clone()), stmts.clone()))
            }
            ast::Expr::Receive => {
                Ok(self.queue.pop_front().expect("cannot receive"))
            }
            ast::Expr::Spawn(ref body) => {
                if let Value::Closure(env, code) = self.eval_expr(body, spawner)? {
                    let env = Env {
                        bindings: HashMap::new(),
                        next: Some(env),
                    };
                    let actor = Actor {
                        code,
                        position: 0,
                        env,
                        queue: VecDeque::new(),
                    };
                    Ok(spawner(actor))
                } else {
                    Err(Error::SpawningNonBlock)
                }
            }
            ast::Expr::Var(ref name) => {
                self.env.lookup(name)
            }
            ast::Expr::Symbol(ref sym) => {
                Ok(Value::Symbol(sym.clone()))
            }
            ast::Expr::Root => {
                Ok(Value::ActorHandle(0))
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Vm {
    next_handle: u64,
    active_actors: HashMap<u64, Actor>,
    parked_actors: HashMap<u64, Actor>,
    updated_handles: Vec<u64>,
    spawn_list: Vec<(u64, Actor)>,
}

impl Default for Vm {
    fn default() -> Vm {
        Vm::new()
    }
}

impl Vm {
    pub fn new() -> Vm {
        let root = Actor::default();
        let mut parked_actors = HashMap::new();
        parked_actors.insert(0, root);
        Vm {
            next_handle: 1,
            active_actors: HashMap::new(),
            parked_actors,
            updated_handles: Vec::new(),
            spawn_list: Vec::new(),
        }
    }

    fn run_step(&mut self, handle: u64) -> Result<()> {
        let handle_ref = &mut self.next_handle;
        let list_ref = &mut self.spawn_list;
        let mut spawner = |actor| {
            let handle = Value::ActorHandle(*handle_ref);
            list_ref.push((*handle_ref, actor));
            *handle_ref += 1;
            handle
        };
        if let Some(actor) = self.active_actors.get_mut(&handle) {
            let code = actor.code.clone();
            match code.get(actor.position) {
                Some(stmt) if stmt.receive_count() > actor.queue.len() => {
                    let actor = self.active_actors.remove(&handle).unwrap();
                    self.parked_actors.insert(handle, actor);
                    Ok(())
                }
                Some(&ast::Statement::Bind(ref to, ref expr)) => {
                    let value = actor.eval_expr(expr, &mut spawner)?;
                    actor.env.bindings.insert(to.clone(), value);
                    actor.position += 1;
                    Ok(())
                }
                Some(&ast::Statement::Expr(ref expr)) => {
                    actor.eval_expr(expr, &mut spawner)?;
                    actor.position += 1;
                    Ok(())
                }
                Some(&ast::Statement::Send(ref to, ref value)) => {
                    let to = actor.eval_expr(to, &mut spawner)?;
                    let value = actor.eval_expr(value, &mut spawner)?;
                    if let Value::ActorHandle(handle) = to {
                        actor.position += 1;
                        self.send_to_actor(handle, value);
                        Ok(())
                    } else {
                        Err(Error::SendToNonActor)
                    }
                }
                None => Ok(()),
            }
        } else {
            Ok(())
        }
    }

    fn send_to_actor(&mut self, handle: u64, value: Value) {
        if let Some(actor) = self.active_actors.get_mut(&handle) {
            actor.queue.push_back(value);
        } else if let Some(mut actor) = self.parked_actors.remove(&handle) {
            // in addition to pushing value into the queue, remove
            // the actor from parked actor list and make it active
            actor.queue.push_back(value);
            self.active_actors.insert(handle, actor);
        }
    }

    fn step_all(&mut self) -> Result<()> {
        self.updated_handles.clear();
        self.updated_handles.extend(self.active_actors.keys().cloned());
        let handles = ::std::mem::replace(&mut self.updated_handles, Vec::new());
        for &handle in &handles {
            self.run_step(handle)?;
        }
        self.active_actors.extend(self.spawn_list.drain(..));
        self.updated_handles = handles;
        self.active_actors.retain(|&h, a| h == 0 || !a.is_completed());
        self.parked_actors.retain(|&h, a| h == 0 || !a.is_completed());
        Ok(())
    }

    fn remove_root(&mut self) -> Actor {
        if let Some(actor) = self.active_actors.remove(&0) {
            actor
        } else {
            self.parked_actors.remove(&0).expect("no root actor")
        }
    }

    fn root_mut(&mut self) -> &mut Actor {
        if let Some(actor) = self.active_actors.get_mut(&0) {
            actor
        } else {
            self.parked_actors.get_mut(&0).expect("no root actor")
        }
    }

    fn is_done(&self) -> bool {
        if self.active_actors.is_empty() {
            true
        } else if self.active_actors.len() > 1 {
            false
        } else if let Some(actor) = self.active_actors.get(&0) {
            actor.is_completed()
        } else {
            false
        }
    }

    pub fn run_statement(&mut self, stmt: ast::Statement) -> Result<()> {
        let mut root = self.remove_root();
        root.code = vec![stmt].into();
        root.position = 0;
        self.active_actors.insert(0, root);
        while !self.is_done() {
            self.step_all()?;
        }
        if self.root_mut().is_completed() {
            Ok(())
        } else {
            Err(Error::RootDeadlock)
        }
    }

    pub fn receive(&mut self) -> Option<Value> {
        self.root_mut().queue.pop_front()
    }
}
