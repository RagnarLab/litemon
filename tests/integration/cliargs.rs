//! Tests for the `CliArgs`.

use std::path::Path;

use litemon::args::CliArgs;

#[test]
fn parse_args_default() {
    let args = CliArgs::from_iter(["litemon"]).unwrap();
    assert_eq!(args, CliArgs::default());
}

#[test]
fn parse_args_short() {
    let args = CliArgs::from_iter([
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
    let args = CliArgs::from_iter([
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
