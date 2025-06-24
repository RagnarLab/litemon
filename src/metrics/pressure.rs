//! System pressure metrics.

use anyhow::Result;
use procfs::{Current, PressureRecord};

/// Pressure stall information for either CPU, memory, or IO.
///
/// See also: <https://www.kernel.org/doc/Documentation/accounting/psi.txt>
#[derive(Debug)]
#[allow(missing_copy_implementations)]
pub struct SystemPressure {
    /// The percentage of time, over a 10 second window, that either some or all tasks were
    /// stalled waiting for a resource.
    pub avg10: f32,
    /// The percentage of time, over a 60 second window, that either some or all tasks were
    /// stalled waiting for a resource.
    pub avg60: f32,
    /// The percentage of time, over a 300 second window, that either some or all tasks were
    /// stalled waiting for a resource.
    pub avg300: f32,
    /// Total stall time (in microseconds).
    pub total: u64,
}

impl From<PressureRecord> for SystemPressure {
    fn from(value: PressureRecord) -> Self {
        Self {
            avg10: value.avg10,
            avg60: value.avg60,
            avg300: value.avg300,
            total: value.total,
        }
    }
}

impl SystemPressure {
    /// Returns I/O pressure information.
    pub async fn io() -> Result<Self> {
        let pressure = smol::unblock(|| {
            let ret = procfs::IoPressure::current()?;
            Ok::<procfs::IoPressure, anyhow::Error>(ret)
        })
        .await?;

        Ok(pressure.some.into())
    }

    /// Returns CPU pressure information.
    pub async fn cpu() -> Result<Self> {
        let pressure = smol::unblock(|| {
            let ret = procfs::CpuPressure::current()?;
            Ok::<procfs::CpuPressure, anyhow::Error>(ret)
        })
        .await?;

        Ok(pressure.some.into())
    }

    /// Returns memory pressure information.
    pub async fn mem() -> Result<Self> {
        let pressure = smol::unblock(|| {
            let ret = procfs::MemoryPressure::current()?;
            Ok::<procfs::MemoryPressure, anyhow::Error>(ret)
        })
        .await?;

        Ok(pressure.some.into())
    }
}
