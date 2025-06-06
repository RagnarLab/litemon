//! LiteMon Configuration.

use std::path::Path;
use std::time::Duration;

use anyhow::Context;
use kdl::{KdlDocument, KdlNode};

/// Describes the user configuration.
///
/// The config implements `Default`, and will use this value if no configuration was found in the
/// environment.
#[derive(Debug, Default)]
pub struct UserConfig {
    /// Everything related to metrics.
    pub metrics: MetricsConfig,
}

/// Describes the configuration for each supported metric.
#[derive(Debug)]
pub struct MetricsConfig {
    pub cpu_seconds: CpuSecondsConfig,
    pub loadavg: LoadAvgConfig,
    pub memory_used: MemoryUsedConfig,
    pub systemd_unit_state: SystemdUnitStateConfig,
    pub network_throughput: NetworkThroughputConfig,
    pub disk_usage: DiskUsageConfig,
}

#[derive(Debug)]
pub struct CpuSecondsConfig {
    pub enabled: bool,
    pub period: Duration,
}

#[derive(Debug)]
pub struct LoadAvgConfig {
    pub enabled: bool,
}

#[derive(Debug)]
pub struct MemoryUsedConfig {
    pub enabled: bool,
}

#[derive(Debug)]
pub struct SystemdUnitStateConfig {
    pub enabled: bool,
    pub units: Vec<String>,
}

#[derive(Debug)]
pub struct NetworkThroughputConfig {
    pub enabled: bool,
    pub interfaces: Vec<String>,
}

#[derive(Debug)]
pub struct DiskUsageConfig {
    pub enabled: bool,
    pub mountpoints: Vec<String>,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            cpu_seconds: CpuSecondsConfig {
                enabled: true,
                period: Duration::from_millis(200),
            },
            loadavg: LoadAvgConfig { enabled: true },
            memory_used: MemoryUsedConfig { enabled: true },
            systemd_unit_state: SystemdUnitStateConfig {
                enabled: false,
                units: vec![],
            },
            network_throughput: NetworkThroughputConfig {
                enabled: false,
                interfaces: vec![],
            },
            disk_usage: DiskUsageConfig {
                enabled: false,
                mountpoints: vec![],
            },
        }
    }
}

impl UserConfig {
    /// Load the configuratin from the `path` specified.
    pub async fn from_path<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let docstr = smol::fs::read_to_string(path.as_ref())
            .await
            .with_context(|| format!("reading config file: {}", path.as_ref().display()))?;
        let doc: KdlDocument = docstr.parse().context("parsing config file")?;
        dbg!(&doc);

        let extract_metrics = |node: &KdlNode| -> MetricsConfig {
            let mut ret = MetricsConfig::default();
            if let Some(children) = node.children() {
                if let Some(node) = children.get("cpu_seconds") {
                    let enabled = node
                        .get("enabled")
                        .and_then(|el| el.as_bool())
                        .unwrap_or_default();
                    let period = node
                        .get("period_ms")
                        .and_then(|el| el.as_integer())
                        .unwrap_or(1000_i128) as u64;
                    ret.cpu_seconds = CpuSecondsConfig {
                        enabled,
                        period: Duration::from_millis(period),
                    }
                }

                if let Some(node) = children.get("loadavg") {
                    let enabled = node
                        .get("enabled")
                        .and_then(|el| el.as_bool())
                        .unwrap_or_default();
                    ret.loadavg = LoadAvgConfig { enabled };
                }

                if let Some(node) = children.get("memory_used") {
                    let enabled = node
                        .get("enabled")
                        .and_then(|el| el.as_bool())
                        .unwrap_or_default();
                    ret.memory_used = MemoryUsedConfig { enabled };
                }

                if let Some(node) = children.get("systemd_unit_state") {
                    let enabled = node
                        .get("enabled")
                        .and_then(|el| el.as_bool())
                        .unwrap_or_default();
                    let units = node
                        .children()
                        .and_then(|el| el.get("units"))
                        .map(|el| el.entries())
                        .map(|it| {
                            it.iter()
                                .filter_map(|el| el.value().as_string())
                                .map(ToOwned::to_owned)
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    ret.systemd_unit_state = SystemdUnitStateConfig { enabled, units };
                }

                if let Some(node) = children.get("network_throughput") {
                    let enabled = node
                        .get("enabled")
                        .and_then(|el| el.as_bool())
                        .unwrap_or_default();
                    let interfaces = node
                        .children()
                        .and_then(|el| el.get("interfaces"))
                        .map(|el| el.entries())
                        .map(|it| {
                            it.iter()
                                .filter_map(|el| el.value().as_string())
                                .map(ToOwned::to_owned)
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    ret.network_throughput = NetworkThroughputConfig {
                        enabled,
                        interfaces,
                    };
                }

                if let Some(node) = children.get("disk_usage") {
                    let enabled = node
                        .get("enabled")
                        .and_then(|el| el.as_bool())
                        .unwrap_or_default();
                    let mountpoints = node
                        .children()
                        .and_then(|el| el.get("mountpoints"))
                        .map(|el| el.entries())
                        .map(|it| {
                            it.iter()
                                .filter_map(|el| el.value().as_string())
                                .map(ToOwned::to_owned)
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    ret.disk_usage = DiskUsageConfig {
                        enabled,
                        mountpoints,
                    };
                }
            }

            ret
        };

        let metrics = doc
            .get("metrics")
            .map_or_else(MetricsConfig::default, extract_metrics);
        let ret = Self { metrics };

        Ok(ret)
    }
}
