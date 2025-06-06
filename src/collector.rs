//! Collector for metrics.
#![allow(clippy::new_without_default)]

use std::sync::Arc;

use anyhow::Result;
use prometheus_client::registry::Registry;
use smol::lock::RwLock;

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


    /// Register all previously created metrics.
    pub async fn register(&self) -> Result<()> {
        let mut writer = self.inner.write().await;
        let CollectorInner { metrics, registry } = &mut *writer;
        for metric in metrics {
            metric.register(registry);
        }

        Ok(())
    }
}
