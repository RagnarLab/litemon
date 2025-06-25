//! Metrics about bytes read/written to disks.

use anyhow::{Context, Result};
use hashbrown::HashMap;

/// Metrics about the bytes read/written to each attached disk.
#[derive(Debug)]
pub struct IOMetrics {
    /// Each disk is one entry.
    pub disks: HashMap<String, DiskMetrics>,
}

/// Metrics about a single disk.
#[derive(Debug)]
pub struct DiskMetrics {
    /// Bytes read since boot.
    pub bytes_read_total: u64,
    /// Bytes written since boot.
    pub bytes_written_total: u64,
    /// Mountpoint
    pub mountpoint: String,
}

impl IOMetrics {
    /// Retrieve disk metrics for all attached disks.
    pub async fn all() -> Result<Self> {
        let stats =
            smol::unblock(|| procfs::diskstats().context("reading /proc/diskstats")).await?;
        // We try to only collect statistics about disks which are mounted.
        let mounts = smol::unblock(|| procfs::mounts().context("reading /proc/mounts")).await?;

        let mut ret = Self {
            disks: HashMap::new(),
        };

        for mount in &mounts {
            let device = &mount.fs_spec;
            let mount_point = &mount.fs_file;

            // Skip non-physical mounts.
            let Some(device_name) = device.strip_prefix("/dev/") else {
                continue;
            };

            // Skip pseudo filesystems
            if mount_point.starts_with("/proc")
                || mount_point.starts_with("/sys")
                || mount_point.starts_with("/dev")
                || mount_point.starts_with("/run")
            {
                continue;
            }

            let Some(stat) = stats.iter().find(|el| el.name == device_name) else {
                continue;
            };

            let Some(root_device) = stats.iter().find(|el| device_name.starts_with(&el.name))
            else {
                continue;
            };

            let sector_size_str =
                smol::fs::read_to_string(format!("/sys/block/{}/queue/hw_sector_size", root_device.name))
                    .await
                    .context("reading sector size")?;
            let sector_size = sector_size_str
                .trim()
                .parse::<u64>()
                .context("parsing sector size")?;
            let metrics = DiskMetrics {
                bytes_read_total: stat.sectors_read * sector_size,
                bytes_written_total: stat.sectors_written * sector_size,
                mountpoint: mount_point.clone(),
            };

            ret.disks.insert(stat.name.clone(), metrics);
        }

        Ok(ret)
    }
}
