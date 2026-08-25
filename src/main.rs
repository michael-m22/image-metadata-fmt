use std::env;
use std::fs;
use std::process;

mod error;
mod normalize;
mod parser;

fn main() {
    let mut args = env::args().skip(1);
    let path = match args.next() {
        Some(p) => p,
        None => {
            eprintln!("usage: image-metadata-fmt <file.imeta>");
            process::exit(2);
        }
    };

    let input = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: could not read {}: {}", path, e);
            process::exit(1);
        }
    };

    match parser::parse(&input) {
        Ok(entries) => print!("{}", normalize::normalize(&entries)),
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    }
}
