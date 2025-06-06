//! Command-line Arguments.

use std::path::PathBuf;

use anyhow::Result;

/// Args passed into the application.
#[derive(Debug, PartialEq)]
pub struct CliArgs {
    /// Optional listen address. By default, listens on `127.0.0.1`
    pub listen_address: String,
    /// Optional listen port, by default, listens on `9774`.
    pub listen_port: u16,
    /// Path to config.
    pub config_path: PathBuf,
}

impl Default for CliArgs {
    fn default() -> Self {
        Self {
            listen_address: "127.0.0.1".to_owned(),
            listen_port: 9774,
            config_path: PathBuf::from("/etc/litemon/config.kdl"),
        }
    }
}

impl CliArgs {
    /// Thin wrapper over [`Self::from_args`].
    pub fn from_env() -> Result<Self> {
        Self::from_args(std::env::args())
    }

    /// Parse the CLI arguments into [`Self`].
    pub fn from_args<I>(it: I) -> Result<Self>
    where
        I: IntoIterator,
        I::Item: Into<std::ffi::OsString>,
    {
        use lexopt::prelude::*;

        let mut ret = Self::default();
        let mut parser = lexopt::Parser::from_iter(it);
        while let Some(arg) = parser.next()? {
            match arg {
                Short('n') | Long("listen") => {
                    ret.listen_address = parser.value()?.to_string_lossy().to_string();
                }
                Short('P') | Long("port") => {
                    ret.listen_port = parser.value()?.parse()?;
                }
                Value(path) => {
                    ret.config_path = PathBuf::from(path);
                }
                _ => return Err(arg.unexpected().into()),
            }
        }

        Ok(ret)
    }
}

