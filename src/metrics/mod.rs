//! All supported metrics.

use anyhow::Result;
use hashbrown::HashMap;

pub mod cpu;
pub mod fs;
pub mod memory;
pub mod net;
pub mod systemd_unit_state;

/// A boxed future. Construct with `Box::pin(async move { ... })`.
pub type DynFuture<'a, T> = std::pin::Pin<Box<dyn Future<Output = T> + 'a>>;

/// Trait that's to be implemented by any supported metric.
pub trait Metric {
    fn collect(
        &self,
        options: HashMap<String, String>,
    ) -> DynFuture<'_, Result<()>>;

    fn register(&self, registry: &mut prometheus_client::registry::Registry);
}
