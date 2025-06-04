//! Memory metrics collection.

use std::cell::RefCell;

use anyhow::{Context, Result};
use procfs::Current;
use prometheus_client::metrics::gauge::ConstGauge;

use super::{DynFuture, Metric};

/// Returns the percentage of memory currently used on the system.
///
/// This function reads memory information from `/proc/meminfo` and calculates the percentage of
/// used memory based on total and available memory.
///
/// # Returns
/// The percentage of memory used (0.0 to 1.0) or an error if memory information cannot be read.
pub async fn get_memory_used_percentage() -> Result<f64> {
    let meminfo = smol::unblock(|| {
        let ret = procfs::Meminfo::current().context("reading /proc/meminfo")?;
        Ok::<procfs::Meminfo, anyhow::Error>(ret)
    })
    .await?;
    let total_kb = meminfo.mem_total;

    // Linux reports "available" memory which is an estimate of how much memory
    // is available for starting new applications, without swapping
    let available_kb = meminfo.mem_available.unwrap_or_else(|| {
        // Fallback calculation if mem_available is not present (older kernels)
        meminfo.mem_free + meminfo.buffers + meminfo.cached
    });

    let used_kb = total_kb - available_kb;
    let percentage = used_kb as f64 / total_kb as f64;

    Ok(percentage)
}

/// Detailed memory statistics.
#[derive(Debug)]
pub struct MemoryStats {
    /// Total physical memory in kilobytes
    pub total_kb: u64,
    /// Free memory in kilobytes
    pub free_kb: u64,
    /// Available memory in kilobytes (estimate of how much memory is available for starting new applications)
    pub available_kb: u64,
    /// Used memory in kilobytes
    pub used_kb: u64,
    /// Percentage of memory used (0.0 to 1.0)
    pub used_percent: f64,
    /// Buffers memory in kilobytes
    pub buffers_kb: u64,
    /// Cached memory in kilobytes
    pub cached_kb: u64,
    /// Swap total in kilobytes
    pub swap_total_kb: u64,
    /// Swap free in kilobytes
    pub swap_free_kb: u64,
    /// Swap used in kilobytes
    pub swap_used_kb: u64,
    /// Percentage of swap used (0.0 to 1.0)
    pub swap_used_percent: f64,
}

impl MemoryStats {
    /// Returns detailed memory statistics.
    ///
    /// This function provides more detailed memory information beyond just the usage percentage.
    ///
    /// # Returns
    /// An object containing detailed memory statistics or an error.
    pub async fn current() -> Result<Self> {
        let meminfo = smol::unblock(|| {
            let ret = procfs::Meminfo::current().context("reading /proc/meminfo")?;
            Ok::<procfs::Meminfo, anyhow::Error>(ret)
        })
        .await?;

        let total_kb = meminfo.mem_total;
        let free_kb = meminfo.mem_free;
        let available_kb = meminfo
            .mem_available
            .unwrap_or_else(|| free_kb + meminfo.buffers + meminfo.cached);
        let used_kb = total_kb - available_kb;
        let used_percent = used_kb as f64 / total_kb as f64;

        let swap_total_kb = meminfo.swap_total;
        let swap_free_kb = meminfo.swap_free;
        let swap_used_kb = swap_total_kb - swap_free_kb;
        let swap_used_percent = if swap_total_kb > 0 {
            swap_used_kb as f64 / swap_total_kb as f64
        } else {
            0.0
        };

        Ok(Self {
            total_kb,
            free_kb,
            available_kb,
            used_kb,
            used_percent,
            buffers_kb: meminfo.buffers,
            cached_kb: meminfo.cached,
            swap_total_kb,
            swap_free_kb,
            swap_used_kb,
            swap_used_percent,
        })
    }
}

/// Collector for memory stats.
#[derive(Debug)]
pub struct MemoryStatsCollector {
    stats: RefCell<Option<MemoryStats>>,
}

impl Metric for MemoryStatsCollector {
    fn collect(&self, _options: hashbrown::HashMap<String, String>) -> DynFuture<Result<()>> {
        Box::pin(async move {
            let stats = MemoryStats::current().await?;
            *self.stats.borrow_mut() = Some(stats);
            Ok(())
        })
    }

    fn register(&self, registry: &mut prometheus_client::registry::Registry) {
        if let Some(stats) = &*self.stats.borrow() {
            let percentage_gauge = ConstGauge::new(stats.used_percent);
            registry.register(
                "litemon_mem_used_percentage",
                "Memory used (0.0-1.0) in percent",
                percentage_gauge,
            );
        }
    }
}
