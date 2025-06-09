//! Tests for the `CliArgs`.

use std::path::Path;

use litemon::args::CliArgs;
use predicates::str::contains;

#[test]
fn parse_args_default() {
    let args = CliArgs::from_args(["litemon"]).unwrap();
    assert_eq!(args, CliArgs::default());
}

#[test]
fn parse_args_short() {
    let args = CliArgs::from_args([
        "litemon",
        "-n",
        "localhost",
        "-P",
        "1234",
        "test/config.kdl",
    ])
    .unwrap();
    let CliArgs {
        listen_address,
        listen_port,
        config_path,
    } = args;
    assert_eq!(listen_address, "localhost");
    assert_eq!(listen_port, 1234);
    assert_eq!(config_path, Path::new("test/config.kdl"));
}

#[test]
fn parse_args_long() {
    let args = CliArgs::from_args([
        "litemon",
        "--listen",
        "localhost",
        "--port",
        "1234",
        "test/config.kdl",
    ])
    .unwrap();
    let CliArgs {
        listen_address,
        listen_port,
        config_path,
    } = args;
    assert_eq!(listen_address, "localhost");
    assert_eq!(listen_port, 1234);
    assert_eq!(config_path, Path::new("test/config.kdl"));
}

#[test]
fn has_help() {
    assert_cmd::Command::cargo_bin("litemon")
        .unwrap()
        .args(&["-h"])
        .assert()
        .success()
        .stdout(contains("Usage"));

    assert_cmd::Command::cargo_bin("litemon")
        .unwrap()
        .args(&["--help"])
        .assert()
        .success()
        .stdout(contains("Usage"));
}

#[test]
fn has_version() {
    assert_cmd::Command::cargo_bin("litemon")
        .unwrap()
        .args(&["-V"])
        .assert()
        .success()
        .stdout(contains("litemon"));

    assert_cmd::Command::cargo_bin("litemon")
        .unwrap()
        .args(&["--version"])
        .assert()
        .success()
        .stdout(contains("litemon"));
}
