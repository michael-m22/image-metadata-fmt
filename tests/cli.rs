//! Integration tests that run the compiled binary against real files on
//! disk, covering the three CLI modes (print/check/write) end to end rather
//! than through the individual modules they're built from.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_image-metadata-fmt"))
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// A fixture file under the system temp dir, removed when the guard drops
/// so a failing assertion still cleans up instead of littering /tmp.
struct TempFile {
    path: PathBuf,
}

impl TempFile {
    fn new(contents: &str) -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let mut path = std::env::temp_dir();
        path.push(format!("imeta-cli-test-{}-{}.imeta", std::process::id(), n));
        fs::write(&path, contents).expect("write temp fixture");
        TempFile { path }
    }

    fn read(&self) -> String {
        fs::read_to_string(&self.path).expect("read temp fixture")
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new() -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let mut path = std::env::temp_dir();
        path.push(format!("imeta-cli-test-dir-{}-{}", std::process::id(), n));
        fs::create_dir(&path).expect("create temp fixture dir");
        TempDir { path }
    }

    fn file(&self, name: &str, contents: &str) {
        fs::write(self.path.join(name), contents).expect("write temp fixture");
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

const UNNORMALIZED: &str = "title: \"Sunset over the bay\"\nArtist:mike\nDate Taken: 2023-3-4\n";
const NORMALIZED: &str = "title = \"Sunset over the bay\"\nartist = \"mike\"\ndate_taken = 2023-03-04\n";

#[test]
fn print_mode_writes_normalized_form_to_stdout() {
    let file = TempFile::new(UNNORMALIZED);
    let output = bin().arg(&file.path).output().expect("run binary");

    assert!(output.status.success());
    assert_eq!(stdout(&output), NORMALIZED);
    assert_eq!(file.read(), UNNORMALIZED, "print mode must not touch the file");
}

#[test]
fn check_mode_reports_path_and_fails_for_unnormalized_file() {
    let file = TempFile::new(UNNORMALIZED);
    let output = bin().arg("--check").arg(&file.path).output().expect("run binary");

    assert!(!output.status.success());
    assert_eq!(stdout(&output).trim(), file.path.to_str().unwrap());
    assert_eq!(file.read(), UNNORMALIZED, "check mode must not touch the file");
}

#[test]
fn check_mode_is_silent_and_succeeds_for_already_normalized_file() {
    let file = TempFile::new(NORMALIZED);
    let output = bin().arg("--check").arg(&file.path).output().expect("run binary");

    assert!(output.status.success());
    assert!(stdout(&output).is_empty());
}

#[test]
fn write_mode_rewrites_file_and_prints_its_path() {
    let file = TempFile::new(UNNORMALIZED);
    let output = bin().arg("--write").arg(&file.path).output().expect("run binary");

    assert!(output.status.success());
    assert_eq!(stdout(&output).trim(), file.path.to_str().unwrap());
    assert_eq!(file.read(), NORMALIZED);
}

#[test]
fn write_mode_is_a_noop_for_already_normalized_file() {
    let file = TempFile::new(NORMALIZED);
    let output = bin().arg("--write").arg(&file.path).output().expect("run binary");

    assert!(output.status.success());
    assert!(stdout(&output).is_empty());
    assert_eq!(file.read(), NORMALIZED);
}

#[test]
fn check_and_write_together_are_rejected() {
    let file = TempFile::new(NORMALIZED);
    let output = bin()
        .arg("--check")
        .arg("--write")
        .arg(&file.path)
        .output()
        .expect("run binary");

    assert!(!output.status.success());
    assert!(stderr(&output).contains("mutually exclusive"));
}

#[test]
fn parse_error_exits_nonzero_and_points_at_the_offending_line() {
    let file = TempFile::new("title: \"Sunset over the bay\n");
    let output = bin().arg(&file.path).output().expect("run binary");

    assert!(!output.status.success());
    let err = stderr(&output);
    assert!(err.contains("unterminated string literal"));
    assert!(err.contains("at line 1, column 8"));
}

#[test]
fn directory_mode_prints_a_header_per_file_in_sorted_order() {
    let dir = TempDir::new();
    dir.file("b.imeta", "artist: mike\n");
    dir.file("a.imeta", "artist: mike\n");

    let output = bin().arg(&dir.path).output().expect("run binary");

    assert!(output.status.success());
    let out = stdout(&output);
    let a_pos = out.find("a.imeta").expect("a.imeta header present");
    let b_pos = out.find("b.imeta").expect("b.imeta header present");
    assert!(a_pos < b_pos, "files should be visited in name order");
    assert_eq!(out.matches("artist = \"mike\"").count(), 2);
}

#[test]
fn directory_mode_keeps_going_after_one_file_fails_to_parse() {
    let dir = TempDir::new();
    dir.file("good.imeta", "artist: mike\n");
    dir.file("bad.imeta", "no separator here\n");

    let output = bin().arg(&dir.path).output().expect("run binary");

    assert!(!output.status.success());
    assert!(stdout(&output).contains("artist = \"mike\""));
    assert!(stderr(&output).contains("expected ':' or '='"));
}

#[test]
fn missing_path_argument_prints_usage_and_exits_nonzero() {
    let output = bin().output().expect("run binary");

    assert!(!output.status.success());
    assert!(stderr(&output).contains("usage:"));
}
