//! CPU metrics collection.

use anyhow::{Context, Result};
use procfs::{Current, CurrentSI};
use std::ops::{Add, Sub};
use std::time::Duration;

/// Represents CPU load averages over different time periods.
#[derive(Debug, Clone, Copy)]
pub struct LoadAverages {
    /// Load average over 1 minute
    pub one: f32,
    /// Load average over 5 minutes
    pub five: f32,
    /// Load average over 15 minutes
    pub fifteen: f32,
}

impl LoadAverages {
    /// Returns the load averages over 1, 5, and 15 minutes.
    ///
    /// This function reads load average information from `/proc/loadavg`.
    pub async fn current() -> Result<Self> {
        let load = smol::unblock(|| {
            let ret = procfs::LoadAverage::current().context("reading /proc/loadavg")?;
            Ok::<procfs::LoadAverage, anyhow::Error>(ret)
        }).await?;

        Ok(Self {
            one: load.one,
            five: load.five,
            fifteen: load.fifteen,
        })
    }
}

/// Represents CPU usage at a point in time since the boot.
#[derive(Debug, Clone)]
pub struct CpuUsage {
    /// Total CPU time.
    pub total_ticks: CpuTime,
    /// Per-core CPU time retrieved from `/proc/stat`.
    pub per_core_ticks: Vec<CpuTime>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CpuTime {
    /// Ticks spent idling.
    pub idle_ticks: u64,
    /// Ticks spent doing things since boot.
    pub total_ticks: u64,
}

impl Add for CpuTime {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            idle_ticks: self.idle_ticks + rhs.idle_ticks,
            total_ticks: self.total_ticks + rhs.total_ticks,
        }
    }
}

impl Sub for CpuTime {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            idle_ticks: self.idle_ticks - rhs.idle_ticks,
            total_ticks: self.total_ticks - rhs.total_ticks,
        }
    }
}

impl CpuUsage {
    /// Retrieve the current CPU usage in ticks from boot.
    pub async fn now() -> Result<Self> {
        let stat = smol::unblock(|| {
            let ret = procfs::KernelStats::current().context("reading /proc/stat")?;
            Ok::<procfs::KernelStats, anyhow::Error>(ret)
        })
        .await?;

        let per_core_ticks = stat
            .cpu_time
            .into_iter()
            .map(|cputime| {
                // Guest time is already accounted in usertime
                // <https://support.checkpoint.com/results/sk/sk65143>
                let total = cputime.user
                    + cputime.nice
                    + cputime.system
                    + cputime.idle
                    + cputime.iowait.unwrap_or(0)
                    + cputime.irq.unwrap_or(0)
                    + cputime.softirq.unwrap_or(0)
                    + cputime.steal.unwrap_or(0);

                CpuTime {
                    idle_ticks: cputime.idle + cputime.iowait.unwrap_or(0),
                    total_ticks: total,
                }
            })
            .collect::<Vec<_>>();

        let total_ticks = per_core_ticks
            .iter()
            .fold(CpuTime::default(), |acc, x| acc + *x);

        Ok(Self {
            total_ticks,
            per_core_ticks,
        })
    }

    /// Calculate the CPU usage between two snapshots taken via [`Self::now()`] for all CPU cores.
    #[allow(clippy::cast_precision_loss)]
    pub fn percentage_all_cores(&self, prev: &Self) -> f64 {
        let diff = self.total_ticks - prev.total_ticks;
        (diff.total_ticks - diff.idle_ticks) as f64 / diff.total_ticks as f64
    }

    /// Calculate the CPU usage between two snapshots taken via [`Self::now()`] for each CPU core
    /// separately.
    #[allow(clippy::cast_precision_loss)]
    pub fn percentage_per_core(&self, prev: &Self) -> Vec<f64> {
        self.per_core_ticks
            .iter()
            .zip(prev.per_core_ticks.iter())
            .map(|(now, prev)| {
                let diff = *now - *prev;
                (diff.total_ticks - diff.idle_ticks) as f64 / diff.total_ticks as f64
            })
            .collect::<Vec<_>>()
    }

    /// Calculate the CPU usage between two snapshots separated by `period` for each CPU core.
    pub async fn period(period: Duration) -> Result<Vec<f64>> {
        let stat1 = Self::now().await.context("creating first snapshot")?;
        smol::Timer::after(period).await;
        let stat2 = Self::now().await.context("creating second snapshot")?;

        Ok(stat2.percentage_per_core(&stat1))
    }
}
