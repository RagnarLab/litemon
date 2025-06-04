//! Filesystem metrics collection.

use anyhow::{Context, Result};
use nix::sys::statvfs::statvfs;
use std::collections::HashMap;
use std::path::Path;

/// Represents filesystem usage information for a specific mount point.
#[derive(Debug, Clone)]
pub struct FilesystemUsage {
    /// The mount point path
    pub mount_point: String,
    /// Total size of the filesystem in bytes
    pub total_bytes: u64,
    /// Used space in bytes
    pub used_bytes: u64,
    /// Available space in bytes
    pub available_bytes: u64,
    /// Usage ratio as a float between 0.0 and 1.0
    pub usage_ratio: f64,
    /// Reserved space in bytes (typically for root user)
    pub reserved_bytes: u64,
    /// Filesystem type (e.g., "ext4", "xfs", "btrfs")
    pub fs_type: String,
    /// Device name
    pub device: String,
}

impl FilesystemUsage {
    /// Creates a new `FilesystemUsage` instance for the specified mount point.
    ///
    /// This function retrieves filesystem statistics for the given mount point
    /// and calculates usage information.
    ///
    /// # Arguments
    ///
    /// * `mount_point` - The path to the mount point to analyze
    ///
    /// # Returns
    ///
    /// * `Result<FilesystemUsage>` - A struct containing filesystem usage information
    ///   or an error if the information cannot be retrieved.
    ///
    /// # Example
    ///
    /// ```
    /// use litemon::metrics::fs::FilesystemUsage;
    ///
    /// fn main() -> anyhow::Result<()> {
    ///     let usage = FilesystemUsage::new("/")?;
    ///     println!("Root filesystem: {:.1}% used ({} bytes free of {} total)",
    ///              usage.usage_ratio * 100.0, usage.available_bytes, usage.total_bytes);
    ///     Ok(())
    /// }
    /// ```
    pub fn new<P: AsRef<Path>>(mount_point: P) -> Result<Self> {
        // Convert the mount point to a string for storage
        let mount_point_str = mount_point
            .as_ref()
            .to_str()
            .context("Invalid mount point path")?
            .to_string();

        // Get filesystem statistics using libc's statvfs
        let stat = statvfs(mount_point.as_ref()).context("retrieving filesystem stats")?;

        // Calculate filesystem metrics
        let block_size = stat.block_size() as u64;
        let fragment_size = stat.fragment_size() as u64;
        let effective_size = if fragment_size > 0 {
            fragment_size
        } else {
            block_size
        };

        let total_blocks = stat.blocks() as u64;
        let free_blocks = stat.blocks_free() as u64;
        let available_blocks = stat.blocks_available() as u64;

        let total_bytes = total_blocks * effective_size;
        let free_bytes = free_blocks * effective_size;
        let available_bytes = available_blocks * effective_size;

        // Calculate reserved space (difference between free and available)
        let reserved_bytes = free_bytes.saturating_sub(available_bytes);

        // Calculate used space
        let used_bytes = total_bytes.saturating_sub(free_bytes);

        // Calculate usage ratio (between 0.0 and 1.0)
        let usage_ratio = if total_bytes > 0 {
            used_bytes as f64 / total_bytes as f64
        } else {
            0.0
        };

        // Get filesystem type and device information using procfs
        let (fs_type, device) =
            Self::get_fs_info(&mount_point_str).context("get info for mountpoint")?;

        Ok(FilesystemUsage {
            mount_point: mount_point_str,
            total_bytes,
            used_bytes,
            available_bytes,
            usage_ratio,
            reserved_bytes,
            fs_type,
            device,
        })
    }

    /// Helper function to get filesystem type and device information using procfs.
    ///
    /// # Returns
    /// A tuple of (<fstype>, <mountpoint>) format.
    fn get_fs_info(mount_point: &str) -> Result<(String, String)> {
        // Use procfs to read mount information
        let mounts = procfs::mounts().context("reading /proc/mounts")?;

        // Find the exact mount point
        for mount in mounts.iter() {
            if mount.fs_file == mount_point {
                return Ok((mount.fs_vfstype.clone(), mount.fs_spec.clone()));
            }
        }

        // If we can't find the exact mount point, try to find a parent mount point
        let path = Path::new(mount_point);
        for ancestor in path.ancestors().skip(1) {
            if let Some(ancestor_str) = ancestor.to_str() {
                for mount in mounts.iter() {
                    if mount.fs_file == ancestor_str {
                        return Ok((mount.fs_vfstype.clone(), mount.fs_spec.clone()));
                    }
                }
            }
        }

        Err(anyhow::anyhow!("no mountpoint at {mount_point} found"))
    }

    /// Returns the free space percentage (0.0 to 1.0).
    pub fn free_ratio(&self) -> f64 {
        1.0 - self.usage_ratio
    }
}

/// Returns a map of all mounted filesystems and their usage information.
///
/// # Returns
/// A map of mount points to their usage information or an error if the information cannot be
/// retrieved.
pub fn get_all_filesystems() -> Result<HashMap<String, FilesystemUsage>> {
    let mut ret = HashMap::new();
    let mounts = procfs::mounts().context("reading /proc/mounts")?;

    for mount in mounts.iter() {
        let mount_point = &mount.fs_spec;

        // Skip pseudo filesystems
        if mount_point.starts_with("/proc")
            || mount_point.starts_with("/sys")
            || mount_point.starts_with("/dev")
            || mount_point.starts_with("/run")
        {
            continue;
        }

        match FilesystemUsage::new(mount_point) {
            Ok(usage) => {
                ret.insert(mount_point.to_string(), usage);
            }
            Err(_) => {
                // Skip filesystems we can't get stats for
            }
        }
    }

    Ok(ret)
}
