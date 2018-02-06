#![feature(nll)]

extern crate structopt;
#[macro_use]
extern crate structopt_derive;
extern crate lalrpop_util;

pub mod ast;
pub mod parser;
pub mod interpreter;

use std::fmt;
use std::fs;
use std::io::{self, Read, BufRead, Write};
use std::path::Path;
use structopt::StructOpt;
use interpreter::Vm;


#[derive(Debug, StructOpt)]
enum Opt {
    #[structopt(name = "repl")]
    Repl,
    #[structopt(name = "run")]
    Run {
        #[structopt(help = "input file")]
        file: String,
    }
}

#[derive(Debug)]
enum Error {
    Io(io::Error),
    Parse(String),
    Interpret(interpreter::Error),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

impl<T: fmt::Display> From<lalrpop_util::ParseError<usize, T, &'static str>> for Error {
    fn from(err: lalrpop_util::ParseError<usize, T, &'static str>) -> Error {
        Error::Parse(format!("{}", err))
    }
}

impl From<interpreter::Error> for Error {
    fn from(err: interpreter::Error) -> Error {
        Error::Interpret(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Io(ref err) => write!(f, "IO error: {}", err),
            Error::Parse(ref msg) => write!(f, "Parse error: {}", msg),
            Error::Interpret(ref err) => write!(f, "Interpreter error: {}", err),
        }
    }
}

fn main() {
    let opt = Opt::from_args();
    let result = match opt {
        Opt::Repl => run_repl(),
        Opt::Run { file } => run_file(&file),
    };
    if let Err(e) = result {
        eprintln!("{}", e);
        ::std::process::exit(1);
    }
}

fn run_repl() -> Result<(), Error> {
    let stdin = io::stdin();
    let mut stdin = stdin.lock();
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let mut input = String::new();
    let mut vm = Vm::new();
    loop {
        print!("> ");
        stdout.flush()?;
        input.clear();
        stdin.read_line(&mut input)?;
        match parser::parse_Stmt(&input) {
            Ok(stmt) => {
                vm.run_statement(stmt)?;
                while let Some(value) = vm.receive() {
                    println!("Received: {}", value);
                }
            }
            Err(e) => {
                println!("Parse error!");
                println!("{}", e);
            }
        }
    }
}

fn run_file<P: AsRef<Path>>(path: P) -> Result<(), Error> {
    let mut f = fs::File::open(path)?;
    let mut contents = String::new();
    f.read_to_string(&mut contents)?;
    let stmts = parser::parse_Stmts(&contents)?;
    let mut vm = Vm::new();
    for stmt in stmts {
        vm.run_statement(stmt)?;
        while let Some(value) = vm.receive() {
            println!("Received: {}", value);
        }
    }
    Ok(())
}
