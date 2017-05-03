extern crate git2;

use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};

fn edit_locally(dir: &Path, krate: &str) -> Command {
    let mut me = env::current_exe().unwrap();
    me.pop();
    if me.ends_with("deps") {
        me.pop();
    }
    me.push("cargo-edit-locally");
    let mut cmd = Command::new(&me);
    cmd.arg("edit-locally");
    cmd.arg(krate);
    cmd.current_dir(dir);
    return cmd
}

static CNT: AtomicUsize = ATOMIC_USIZE_INIT;

fn dir() -> PathBuf {
    let i = CNT.fetch_add(1, Ordering::SeqCst);
    let mut dir = env::current_exe().unwrap();
    dir.pop();
    if dir.ends_with("deps") {
        dir.pop();
    }
    dir.pop();
    dir.push("tmp");
    drop(fs::create_dir(&dir));
    dir.push(&format!("test{}", i));
    drop(fs::remove_dir_all(&dir));
    fs::create_dir(&dir).unwrap();
    return dir
}

fn file(dir: &Path, path: &str, contents: &str) {
    let path = dir.join(path);
    println!("writing {:?}", path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    File::create(path).unwrap().write_all(contents.as_bytes()).unwrap();
}

#[test]
fn probe() {
    // the log 0.3.5 version does not have a tag
    let dir = dir();

    file(&dir, "Cargo.toml", r#"
        [package]
        name = "foo"
        version = "0.1.0"

        [dependencies]
        log = "=0.3.5"
    "#);
    file(&dir, "src/lib.rs", "");

    run(&mut edit_locally(&dir, "log"));

    let repo = git2::Repository::open(dir.join("log")).unwrap();
    let id = repo.head().unwrap().target().unwrap().to_string();
    assert_eq!(id, "5785c972e6037fb15d88486b78163856f1115cc1");
}

#[test]
fn tag() {
    // the log 0.3.6 version has a tag
    let dir = dir();

    file(&dir, "Cargo.toml", r#"
        [package]
        name = "foo"
        version = "0.1.0"

        [dependencies]
        log = "=0.3.6"
    "#);
    file(&dir, "src/lib.rs", "");

    run(&mut edit_locally(&dir, "log"));

    let repo = git2::Repository::open(dir.join("log")).unwrap();
    let id = repo.head().unwrap().target().unwrap().to_string();
    assert_eq!(id, "1c79a9c8ddebce3f0037fcdc6783e682cb87bce2");
}

#[test]
fn nested() {
    let dir = dir();

    file(&dir, "Cargo.toml", r#"
        [package]
        name = "foo"
        version = "0.1.0"

        [dependencies]
        env_logger = "=0.4.0"
    "#);
    file(&dir, "src/lib.rs", "");

    run(&mut edit_locally(&dir, "env_logger"));

    let repo = git2::Repository::open(dir.join("env_logger")).unwrap();
    let id = repo.head().unwrap().target().unwrap().to_string();
    assert_eq!(id, "1c79a9c8ddebce3f0037fcdc6783e682cb87bce2");
}

fn run(cmd: &mut Command) {
    println!("running {:?}", cmd);
    let output = cmd.output().unwrap();
    if !output.status.success() {
        println!("stdout: ----------\n{}", String::from_utf8_lossy(&output.stdout));
        println!("stderr: ----------\n{}", String::from_utf8_lossy(&output.stderr));
        panic!("not successful: {}", output.status);
    }
}
