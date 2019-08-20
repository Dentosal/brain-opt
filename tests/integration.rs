use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use tempfile::tempdir;

use assert_cmd::prelude::*;

fn assert_output<P: AsRef<Path>>(path: P, input: &'static [u8], output: &'static [u8]) {
    let td = tempdir().unwrap();
    let execpath = td.path().join("executable");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let compiler = cmd
        .arg(path.as_ref().as_os_str())
        .arg("--assembly")
        .arg("-")
        .arg("--output")
        .arg(execpath.as_os_str())
        .output()
        .unwrap();
    println!("<compiler stdout>");
    println!("{}", String::from_utf8_lossy(&compiler.stdout));
    println!("</compiler stdout>");
    println!("<compiler stderr>");
    println!("{}", String::from_utf8_lossy(&compiler.stderr));
    println!("</compiler stderr>");
    assert!(compiler.status.success());

    let mut p = Command::new(execpath)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let stdin = p.stdin.as_mut().unwrap();
        stdin.write_all(input).unwrap();
    }
    let res = p.wait_with_output().unwrap();
    assert!(res.status.success());
    assert_eq!(res.stdout, output);
}

#[test]
fn test_helloworld() {
    assert_output("examples/helloworld.bf", b"", b"Hello World!\n");
}

#[test]
fn test_cat() {
    assert_output("examples/cat.bf", b"", b"");
    assert_output("examples/cat.bf", b"copypaste", b"copypaste");
    assert_output("examples/cat.bf", b"a\nb", b"a\nb");
}

#[test]
fn test_bubblesort() {
    assert_output("examples/bubblesort_bytes.bf", b"", b"");
    assert_output("examples/bubblesort_bytes.bf", b"213", b"123");
    assert_output("examples/bubblesort_bytes.bf", b"123", b"123");
    assert_output("examples/bubblesort_bytes.bf", b"987654321", b"123456789");
    assert_output("examples/bubblesort_bytes.bf", b"987654321", b"123456789");
}

#[test]
fn test_quicksort() {
    assert_output("examples/quicksort_bytes.bf", b"", b"");
    assert_output("examples/quicksort_bytes.bf", b"213", b"123");
    assert_output("examples/quicksort_bytes.bf", b"123", b"123");
    assert_output("examples/quicksort_bytes.bf", b"987654321", b"123456789");
    assert_output("examples/quicksort_bytes.bf", b"987654321", b"123456789");
}

#[test]
fn test_rot13() {
    assert_output("examples/rot13.bf", b"", b"");
    assert_output("examples/rot13.bf", b"a", b"n");
    assert_output("examples/rot13.bf", b"Hello World!", b"Uryyb Jbeyq!");
    assert_output("examples/rot13.bf", b"123456789", b"123456789");
    assert_output("examples/rot13.bf", b"a=1", b"n=1");
}

#[test]
#[should_panic]
fn fail_helloworld() {
    assert_output("examples/helloworld.bf", b"", b"Incorrect\n");
}

#[test]
#[should_panic]
fn fail_cat() {
    assert_output("examples/cat.bf", b"copypaste", b"cat");
}

#[test]
#[should_panic]
fn fail_bubblesort() {
    assert_output("examples/bubblesort_bytes.bf", b"321", b"321");
}

#[test]
#[should_panic]
fn fail_quicksort() {
    assert_output("examples/quicksort_bytes.bf", b"321", b"321");
}

#[test]
#[should_panic]
fn fail_rot13() {
    assert_output("examples/rot13.bf", b"abc", b"abc");
}
