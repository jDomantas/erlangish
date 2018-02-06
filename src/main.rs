#![feature(nll)]

pub mod ast;
pub mod parser;
pub mod interpreter;

use std::io::{self, BufRead, Write};
use interpreter::Vm;


fn main() {
    if run().is_err() {
        ::std::process::exit(1);
    }
}

fn run() -> io::Result<()> {
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
                vm.run_statement(stmt);
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
