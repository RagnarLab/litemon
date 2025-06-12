//! Network throughput metric.

use std::time::Duration;

use anyhow::{Context, Result};
use hashbrown::HashMap;

/// The statistics of a single network interface.
#[derive(Debug)]
pub struct InterfaceStats {
    /// Name of the network interface.
    pub name: String,
    /// Total bytes received.
    pub recv_bytes: u64,
    /// Bad packets received and packets dropped.
    pub recv_errors: u64,
    /// Total bytes sent.
    pub sent_bytes: u64,
    /// Number of transmission errors and packets dropped during transmission.
    pub sent_errors: u64,
    /// Optional period. If set, all values are over this period.
    pub period: Option<Duration>,
}

/// Network statistics by interface.
#[derive(Debug)]
pub struct NetworkStats {
    /// Network statistics for all interfaces.
    pub interfaces: HashMap<String, InterfaceStats>,
}

impl NetworkStats {
    /// Retrieve the network statistics for all currently attached network interfaces.
    pub async fn all() -> Result<Self> {
        let stats = tokio::task::spawn_blocking(|| {
            procfs::net::dev_status().context("reading network device status")
        })
        .await??;

        let mut ret = Self {
            interfaces: HashMap::new(),
        };
        for (key, val) in stats {
            ret.interfaces.insert(
                key.clone(),
                InterfaceStats {
                    name: key,
                    recv_bytes: val.recv_bytes,
                    recv_errors: val.recv_errs + val.recv_drop,
                    sent_bytes: val.sent_bytes,
                    sent_errors: val.sent_errs + val.sent_drop,
                    period: None,
                },
            );
        }

        Ok(ret)
    }

    /// Retrieve the network throughput statistics of the specified `period`.
    pub async fn period(period: Duration) -> Result<Self> {
        let stats1 = Self::all().await.context("reading networkstats")?;
        tokio::time::sleep(period).await;
        let stats2 = Self::all().await.context("reading networkstats")?;

        let mut ret = Self {
            interfaces: HashMap::new(),
        };
        for (key_prev, val_prev) in &stats1.interfaces {
            let Some(val_now) = stats2.interfaces.get(key_prev) else {
                continue;
            };

            let stats = InterfaceStats {
                name: key_prev.to_owned(),
                recv_bytes: val_now.recv_bytes - val_prev.recv_bytes,
                recv_errors: val_now.recv_errors - val_prev.recv_errors,
                sent_bytes: val_now.sent_bytes - val_prev.sent_bytes,
                sent_errors: val_now.sent_errors - val_prev.sent_errors,
                period: Some(period),
            };

            ret.interfaces.insert(key_prev.to_owned(), stats);
        }

        Ok(ret)
    }
}
