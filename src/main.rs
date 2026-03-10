use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        panic!("Usage:\n\t{} FILE\n", args[0]);
    }

    let source = fs::read_to_string(&args[1]).unwrap_or_else(|e| panic!("{}", e));

    match ggc::ast::parse(&source) {
        Ok(program) => println!("{:#?}", program),
        Err(e) => eprintln!("Parse error: {}", e),
    }
}
