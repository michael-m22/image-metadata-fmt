use std::env;
use std::fs;
use std::path::Path;
use std::process;

mod error;
mod normalize;
mod parser;

fn main() {
    let mut args = env::args().skip(1);
    let path = match args.next() {
        Some(p) => p,
        None => {
            eprintln!("usage: image-metadata-fmt <file.imeta | directory>");
            process::exit(2);
        }
    };

    let metadata = match fs::metadata(&path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: could not read {}: {}", path, e);
            process::exit(1);
        }
    };

    let ok = if metadata.is_dir() {
        process_dir(Path::new(&path))
    } else {
        process_file(Path::new(&path))
    };

    if !ok {
        process::exit(1);
    }
}

/// Normalizes every `.imeta` file directly inside `dir` (not recursive - a
/// sidecar sits next to the photo it describes, so one level is the only
/// level that makes sense). Files are visited in name order so output is
/// stable between runs. One bad file doesn't stop the rest: everything that
/// parses gets printed, and the exit code reflects whether anything failed.
fn process_dir(dir: &Path) -> bool {
    let read_dir = match fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(e) => {
            eprintln!("error: could not read directory {}: {}", dir.display(), e);
            return false;
        }
    };

    let mut paths: Vec<_> = read_dir
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "imeta"))
        .collect();
    paths.sort();

    if paths.is_empty() {
        eprintln!("error: no .imeta files found in {}", dir.display());
        return false;
    }

    let mut ok = true;
    for path in paths {
        println!("== {} ==", path.display());
        if !process_file(&path) {
            ok = false;
        }
    }
    ok
}

fn process_file(path: &Path) -> bool {
    let input = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: could not read {}: {}", path.display(), e);
            return false;
        }
    };

    match parser::parse(&input) {
        Ok(entries) => {
            print!("{}", normalize::normalize(&entries));
            true
        }
        Err(e) => {
            eprintln!("{}", e);
            false
        }
    }
}
