//! Collector for metrics.
#![allow(clippy::new_without_default)]

use std::sync::Arc;

use anyhow::{Context, Result};
use futures_concurrency::future::Join;
use hashbrown::HashMap;
use prometheus_client::encoding::text::encode;
use prometheus_client::registry::Registry;
use smol::lock::RwLock;

use crate::config::UserConfig;
use crate::metrics::collector::{
    CpuStatsCollector, DiskStatsCollector, FilesystemStatsCollector, MemoryStatsCollector, NetworkStatsCollector, NodeInfoCollector, NodeUptimeCollector, PressureCollector, SystemdUnitStateCollector
};
use crate::metrics::Metric;

#[derive(Debug)]
struct CollectorInner {
    registry: Registry,
    metrics: Vec<Box<dyn Metric>>,
}

#[derive(Debug, Clone)]
pub struct Collector {
    inner: Arc<RwLock<CollectorInner>>,
}

impl CollectorInner {
    pub(crate) fn new() -> Self {
        Self {
            registry: <Registry>::default(),
            metrics: Vec::new(),
        }
    }
}

impl Collector {
    pub fn new() -> Self {
        let inner = CollectorInner::new();
        Self {
            inner: Arc::new(RwLock::new(inner)),
        }
    }

    /// Create collectors from configuration.
    pub async fn create_from_config(&self, config: &UserConfig) -> Result<()> {
        let mut inner = self.inner.write().await;

        let metrics = &config.metrics;
        {
            let collector = Box::new(NodeInfoCollector::new()?);
            inner.metrics.push(collector);
        }
        {
            let collector = Box::new(NodeUptimeCollector::default());
            inner.metrics.push(collector);
        }

        if metrics.cpu_seconds.enabled || metrics.loadavg.enabled {
            // TODO: Fix disabling of CPU seconds.
            let collector = Box::new(
                CpuStatsCollector::new(if metrics.loadavg.enabled {
                    HashMap::from([("load_avg_enabled".to_owned(), "true".to_owned())])
                } else {
                    Default::default()
                })
                .await?,
            );
            inner.metrics.push(collector);
        }

        if metrics.memory_used.enabled {
            let collector = Box::new(MemoryStatsCollector::default());
            inner.metrics.push(collector);
        }

        if metrics.systemd_unit_state.enabled {
            let units = metrics.systemd_unit_state.units.join(",");
            let options = HashMap::from([("units".to_owned(), units)]);
            let collector = Box::new(SystemdUnitStateCollector::new(&options).await?);
            inner.metrics.push(collector);
        }

        if metrics.network_throughput.enabled {
            let interfaces = metrics.network_throughput.interfaces.join(",");
            let options = HashMap::from([("interfaces".to_owned(), interfaces)]);
            let collector = Box::new(NetworkStatsCollector::new(&options)?);
            inner.metrics.push(collector);
        }

        if metrics.disk_usage.enabled {
            let mountpoints = metrics.disk_usage.mountpoints.join(",");
            let options = HashMap::from([("mountpoints".to_owned(), mountpoints)]);
            let collector = Box::new(FilesystemStatsCollector::new(&options)?);
            inner.metrics.push(collector);
        }

        if metrics.pressure.enabled {
            let collector = Box::new(PressureCollector::default());
            inner.metrics.push(collector);
        }

        if metrics.disk_stats.enabled {
            let mountpoints = metrics.disk_stats.mountpoints.join(",");
            let options = HashMap::from([("mountpoints".to_owned(), mountpoints)]);
            let collector = Box::new(DiskStatsCollector::new(&options)?);
            inner.metrics.push(collector);
        }

        Ok(())
    }

    /// Register all previously created metrics.
    pub async fn register(&self) -> Result<()> {
        let mut writer = self.inner.write().await;
        let CollectorInner { metrics, registry } = &mut *writer;
        for metric in metrics {
            metric.register(registry);
        }

        Ok(())
    }

    /// Collect all metrics and return the serialized response in OpenMetrics format as a String.
    pub async fn collect_and_encode(&self) -> Result<String> {
        let inner = self.inner.read().await;
        let futs: Vec<_> = inner
            .metrics
            .iter()
            .map(|metric| metric.collect())
            .collect();
        let results = futs.join().await;
        for res in results {
            res.inspect_err(|err| eprintln!("{err}"))
                .context("collecting metrics")?;
        }

        let mut buf = String::with_capacity(2048);
        encode(&mut buf, &inner.registry)?;

        Ok(buf)
    }
}
