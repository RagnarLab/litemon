//! Node information metric.

use std::ffi::CStr;
use std::time::Duration;

use anyhow::{Context, Result};
use procfs::Current;

/// Information about the node.
#[derive(Debug)]
pub struct NodeInfo {
    /// Hostname.
    pub hostname: String,
    /// System architecture (e.g., `aarch64`, or `x86_64`)
    pub arch: String,
    /// Uptime.
    pub uptime: Duration,
}

impl NodeInfo {
    /// Collect information about the node.
    pub fn new() -> Result<Self> {
        let mut utsname = nix::libc::utsname {
            sysname: [0; 65],
            nodename: [0; 65],
            release: [0; 65],
            version: [0; 65],
            machine: [0; 65],
            domainname: [0; 65],
        };

        // SAFETY: `utsname` is initialized, and valid, and return value is checked.
        if unsafe { nix::libc::uname(&mut utsname) } != 0 {
            return Err(anyhow::anyhow!("uname return an error"));
        };

        // SAFETY: Pointers are guaranteed to be NULL terminated, and ret value is checked.
        let hostname = unsafe { CStr::from_ptr(utsname.nodename.as_ptr()) };
        // SAFETY: Pointers are guaranteed to be NULL terminated, and ret value is checked.
        let arch = unsafe { CStr::from_ptr(utsname.machine.as_ptr()) };
        let uptime = procfs::Uptime::current().context("reading uptime")?;

        Ok(Self {
            hostname: hostname.to_string_lossy().to_string(),
            arch: arch.to_string_lossy().to_string(),
            uptime: uptime.uptime_duration(),
        })
    }
}
