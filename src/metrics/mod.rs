//! All supported metrics.

use anyhow::Result;

pub mod collector;
pub mod cpu;
pub mod disk;
pub mod fs;
pub mod info;
pub mod memory;
pub mod net;
pub mod pressure;
pub mod systemd_unit_state;

/// A boxed future. Construct with `Box::pin(async move { ... })`.
pub type DynFuture<'a, T> = std::pin::Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Trait that's to be implemented by any supported metric.
pub trait Metric: Send + Sync + std::fmt::Debug {
    fn register(&self, registry: &mut prometheus_client::registry::Registry);
    fn collect(&self) -> DynFuture<'_, Result<()>>;
}
