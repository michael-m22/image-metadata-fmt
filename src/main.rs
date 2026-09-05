use std::env;
use std::fs;
use std::path::Path;
use std::process;

mod error;
mod normalize;
mod parser;
mod schema;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Print the normalized form to stdout.
    Print,
    /// Print the path of any file whose normalized form differs from what's
    /// on disk (or that fails to parse/validate) and exit non-zero, without
    /// printing normalized content. Mirrors `gofmt -l`.
    Check,
}

const USAGE: &str = "usage: image-metadata-fmt [--check] <file.imeta | directory>";

fn main() {
    let mut check = false;
    let mut path: Option<String> = None;
    for arg in env::args().skip(1) {
        if arg == "--check" {
            check = true;
        } else if path.is_none() {
            path = Some(arg);
        } else {
            eprintln!("{}", USAGE);
            process::exit(2);
        }
    }

    let path = match path {
        Some(p) => p,
        None => {
            eprintln!("{}", USAGE);
            process::exit(2);
        }
    };
    let mode = if check { Mode::Check } else { Mode::Print };

    let metadata = match fs::metadata(&path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: could not read {}: {}", path, e);
            process::exit(1);
        }
    };

    let ok = if metadata.is_dir() {
        process_dir(Path::new(&path), mode)
    } else {
        process_file(Path::new(&path), mode)
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
fn process_dir(dir: &Path, mode: Mode) -> bool {
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
        // The "== path ==" header decorates printed output; in check mode
        // any file that needs normalizing already prints its own path.
        if mode == Mode::Print {
            println!("== {} ==", path.display());
        }
        if !process_file(&path, mode) {
            ok = false;
        }
    }
    ok
}

fn process_file(path: &Path, mode: Mode) -> bool {
    let input = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: could not read {}: {}", path.display(), e);
            return false;
        }
    };

    let entries = match parser::parse(&input) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!("{}", e);
            return false;
        }
    };

    if let Err(e) = schema::validate(&entries, &input) {
        eprintln!("{}", e);
        return false;
    }

    let normalized = normalize::normalize(&entries);
    match mode {
        Mode::Print => {
            print!("{}", normalized);
            true
        }
        Mode::Check => {
            if normalized == input {
                true
            } else {
                println!("{}", path.display());
                false
            }
        }
    }
}
