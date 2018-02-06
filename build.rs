extern crate lalrpop;

fn main() {
    lalrpop::process_root().expect("failed to run lalrpop");
}
