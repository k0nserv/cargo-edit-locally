use std::env;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

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
    return cmd;
}

static CNT: AtomicUsize = AtomicUsize::new(0);

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
    return dir;
}

fn file(dir: &Path, path: &str, contents: &str) {
    let path = dir.join(path);
    println!("writing {:?}", path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    File::create(path)
        .unwrap()
        .write_all(contents.as_bytes())
        .unwrap();
}

fn read(path: &Path) -> String {
    let mut contents = String::new();
    File::open(path)
        .unwrap()
        .read_to_string(&mut contents)
        .unwrap();
    contents
}

#[test]
fn path() {
    // the log 0.3.5 version does not have a tag
    let dir = dir();

    file(
        &dir,
        "Cargo.toml",
        r#"
        [package]
        name = "foo"
        version = "0.1.0"

        [dependencies]
        log = "=0.3.5"
    "#,
    );
    file(&dir, "src/lib.rs", "");

    file(
        &dir,
        "foo/Cargo.toml",
        r#"
        [package]
        name = "log"
        version = "0.3.5"
    "#,
    );
    file(&dir, "foo/src/lib.rs", "");

    run(edit_locally(&dir, "log").arg("--path").arg(dir.join("foo")));

    str_eq(
        &read(&dir.join("Cargo.toml")),
        r#"
        [package]
        name = "foo"
        version = "0.1.0"

        [dependencies]
        log = "=0.3.5"

[replace]
"log:0.3.5" = { path = "foo" }
"#,
    );

    let lock = read(&dir.join("Cargo.lock"));
    assert!(lock.contains("replace"));
}

#[test]
fn no_newlines() {
    // the log 0.3.5 version does not have a tag
    let dir = dir();

    file(
        &dir,
        "Cargo.toml",
        r#"
        [package]
        name = "foo"
        version = "0.1.0"

        [dependencies]
        log = "=0.3.5""#,
    );
    file(&dir, "src/lib.rs", "");

    file(
        &dir,
        "foo/Cargo.toml",
        r#"
        [package]
        name = "log"
        version = "0.3.5"
    "#,
    );
    file(&dir, "foo/src/lib.rs", "");

    run(edit_locally(&dir, "log").arg("--path").arg(dir.join("foo")));

    str_eq(
        &read(&dir.join("Cargo.toml")),
        r#"
        [package]
        name = "foo"
        version = "0.1.0"

        [dependencies]
        log = "=0.3.5"

[replace]
"log:0.3.5" = { path = "foo" }
"#,
    );
}

#[test]
fn add_to_existing_replace() {
    // the log 0.3.5 version does not have a tag
    let dir = dir();

    file(
        &dir,
        "Cargo.toml",
        r#"
        [package]
        name = "foo"
        version = "0.1.0"

        [dependencies]
        log = "=0.3.5"

[replace]
"wut:0.3.5" = { path = "wut" }
    "#,
    );
    file(&dir, "src/lib.rs", "");

    file(
        &dir,
        "foo/Cargo.toml",
        r#"
        [package]
        name = "log"
        version = "0.3.5"
    "#,
    );
    file(&dir, "foo/src/lib.rs", "");

    run(edit_locally(&dir, "log").arg("--path").arg(dir.join("foo")));

    str_eq(
        &read(&dir.join("Cargo.toml")),
        r#"
        [package]
        name = "foo"
        version = "0.1.0"

        [dependencies]
        log = "=0.3.5"

[replace]
"log:0.3.5" = { path = "foo" }
"wut:0.3.5" = { path = "wut" }
    "#,
    );
}

#[test]
fn empty_replace() {
    // the log 0.3.5 version does not have a tag
    let dir = dir();

    file(
        &dir,
        "Cargo.toml",
        r#"
        [package]
        name = "foo"
        version = "0.1.0"

        [dependencies]
        log = "=0.3.5"

[replace]
    "#,
    );
    file(&dir, "src/lib.rs", "");

    file(
        &dir,
        "foo/Cargo.toml",
        r#"
        [package]
        name = "log"
        version = "0.3.5"
    "#,
    );
    file(&dir, "foo/src/lib.rs", "");

    run(edit_locally(&dir, "log").arg("--path").arg(dir.join("foo")));

    str_eq(
        &read(&dir.join("Cargo.toml")),
        r#"
        [package]
        name = "foo"
        version = "0.1.0"

        [dependencies]
        log = "=0.3.5"

[replace]
"log:0.3.5" = { path = "foo" }
    "#,
    );
}

#[test]
fn wrong_version() {
    // the log 0.3.5 version does not have a tag
    let dir = dir();

    file(
        &dir,
        "Cargo.toml",
        r#"
        [package]
        name = "foo"
        version = "0.1.0"

        [dependencies]
        log = "=0.3.5"
    "#,
    );
    file(&dir, "src/lib.rs", "");

    let out = edit_locally(&dir, "log")
        .arg("--git")
        .arg("https://github.com/rust-lang-nursery/log")
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("failed to find `log v0.3.5` inside of"));
    assert!(err.contains("perhaps a different branch/tag is needed?"));
}

#[test]
fn git() {
    // the log 0.3.5 version does not have a tag
    let dir = dir();

    file(
        &dir,
        "Cargo.toml",
        r#"
        [package]
        name = "foo"
        version = "0.1.0"

        [dependencies]
        log = "=0.3.6"
    "#,
    );
    file(&dir, "src/lib.rs", "");

    run(edit_locally(&dir, "log")
        .arg("--git")
        .arg("https://github.com/rust-lang-nursery/log")
        .arg("--tag")
        .arg("0.3.6"));
}

#[test]
fn git_remote() {
    // the log 0.3.5 version does not have a tag
    let dir = dir();

    file(
        &dir,
        "Cargo.toml",
        r#"
        [package]
        name = "foo"
        version = "0.1.0"

        [dependencies]
        log = { git = 'https://github.com/rust-lang-nursery/log', tag = '0.3.6' }
    "#,
    );
    file(&dir, "src/lib.rs", "");

    file(
        &dir,
        "foo/Cargo.toml",
        r#"
        [package]
        name = "log"
        version = "0.3.6"
    "#,
    );
    file(&dir, "foo/src/lib.rs", "");

    run(edit_locally(&dir, "log").arg("--path").arg("foo"));

    str_eq(
        &read(&dir.join("Cargo.toml")),
        r#"
        [package]
        name = "foo"
        version = "0.1.0"

        [dependencies]
        log = { git = 'https://github.com/rust-lang-nursery/log', tag = '0.3.6' }

[replace]
"https://github.com/rust-lang-nursery/log#log:0.3.6" = { path = "foo" }
"#,
    );
}

fn str_eq(a: &str, b: &str) {
    if a == b {
        return;
    }
    if a.lines().count() == b.lines().count() {
        for (a, b) in a.lines().zip(b.lines()) {
            if a.trim() == b.trim() {
                continue;
            }

            panic!("line difference\n{:?}\n{:?}", a, b);
        }
        return;
    }
    let a = xform(a);
    let b = xform(b);
    panic!("string differences:\n`{}`\n\n`{}`", a, b);

    fn xform(s: &str) -> String {
        s.replace("\n", "\u{21b5}\n")
            .chars()
            .map(|c| if c == ' ' { '\u{b7}' } else { c })
            .collect()
    }
}

fn run(cmd: &mut Command) {
    println!("running {:?}", cmd);
    let output = cmd.output().unwrap();
    if !output.status.success() {
        println!(
            "stdout: ----------\n{}",
            String::from_utf8_lossy(&output.stdout)
        );
        println!(
            "stderr: ----------\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        panic!("not successful: {}", output.status);
    }
}
