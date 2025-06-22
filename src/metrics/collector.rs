//! Collectors for all supported metrics.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64};

use anyhow::Result;
use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::metrics::info::Info;
use smol::lock::Mutex;

use super::cpu::{CpuUsage, LoadAverages};
use super::fs::FilesystemUsage;
use super::info::NodeInfo;
use super::memory::MemoryStats;
use super::net::NetworkStats;
use super::systemd_unit_state::{ActiveState, SystemdUnitState};
use super::{DynFuture, Metric};

/// Collector for memory stats.
#[derive(Debug, Default)]
pub struct MemoryStatsCollector {
    gauge: Gauge<f64, AtomicU64>,
}

impl Metric for MemoryStatsCollector {
    fn init(&self, _options: hashbrown::HashMap<String, String>) -> DynFuture<'_, Result<()>> {
        Box::pin(async move { Ok(()) })
    }

    fn register(&self, registry: &mut prometheus_client::registry::Registry) {
        let gauge_ref = &self.gauge;
        registry.register(
            "litemon_mem_used_percentage",
            "Memory used (0.0-1.0) in percent",
            gauge_ref.clone(),
        );
    }

    fn collect(&self) -> DynFuture<'_, Result<()>> {
        Box::pin(async move {
            let stats = MemoryStats::current().await?;
            self.gauge.set(stats.used_percent);
            Ok(())
        })
    }
}

/// Collector for CPU metrics (load averages and CPU usage).
#[derive(Debug)]
pub struct CpuStatsCollector {
    load_avg_1m: Gauge<f64, AtomicU64>,
    load_avg_5m: Gauge<f64, AtomicU64>,
    load_avg_15m: Gauge<f64, AtomicU64>,
    cpu_usage_overall: Gauge<f64, AtomicU64>,
    cpu_usage_per_core: Family<CpuCoreLabels, Gauge<f64, AtomicU64>>,
    last_cpu_snapshot: Mutex<Option<CpuUsage>>,
    load_avg_enabled: AtomicBool,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct CpuCoreLabels {
    core: String,
}

impl Default for CpuStatsCollector {
    fn default() -> Self {
        Self {
            load_avg_1m: Gauge::default(),
            load_avg_5m: Gauge::default(),
            load_avg_15m: Gauge::default(),
            cpu_usage_overall: Gauge::default(),
            cpu_usage_per_core: Family::default(),
            last_cpu_snapshot: Mutex::new(None),
            load_avg_enabled: AtomicBool::new(false),
        }
    }
}

impl Metric for CpuStatsCollector {
    fn init(&self, options: hashbrown::HashMap<String, String>) -> DynFuture<'_, Result<()>> {
        Box::pin(async move {
            let initial_snapshot = CpuUsage::now().await?;
            *self.last_cpu_snapshot.lock().await = Some(initial_snapshot);
            self.load_avg_enabled.store(
                options.get("load_avg_enabled").is_some_and(|b| b == "true"),
                std::sync::atomic::Ordering::SeqCst,
            );

            Ok(())
        })
    }

    fn register(&self, registry: &mut prometheus_client::registry::Registry) {
        if self
            .load_avg_enabled
            .load(std::sync::atomic::Ordering::Acquire)
        {
            registry.register(
                "litemon_load_avg_1m",
                "Load average over 1 minute",
                self.load_avg_1m.clone(),
            );
            registry.register(
                "litemon_load_avg_5m",
                "Load average over 5 minutes",
                self.load_avg_5m.clone(),
            );
            registry.register(
                "litemon_load_avg_15m",
                "Load average over 15 minutes",
                self.load_avg_15m.clone(),
            );
        }
        registry.register(
            "litemon_cpu_usage_overall",
            "Overall CPU usage percentage (0.0-1.0)",
            self.cpu_usage_overall.clone(),
        );
        registry.register(
            "litemon_cpu_usage_per_core",
            "Per-core CPU usage percentage (0.0-1.0)",
            self.cpu_usage_per_core.clone(),
        );
    }

    fn collect(&self) -> DynFuture<'_, Result<()>> {
        Box::pin(async move {
            let load_avg = LoadAverages::current().await?;
            if self
                .load_avg_enabled
                .load(std::sync::atomic::Ordering::Acquire)
            {
                self.load_avg_1m.set(load_avg.one as f64);
                self.load_avg_5m.set(load_avg.five as f64);
                self.load_avg_15m.set(load_avg.fifteen as f64);
            }

            let current_snapshot = CpuUsage::now().await?;
            if let Some(prev_snapshot) = self.last_cpu_snapshot.lock().await.as_ref() {
                let overall_usage = current_snapshot.percentage_all_cores(prev_snapshot);
                self.cpu_usage_overall.set(overall_usage);

                let per_core_usage = current_snapshot.percentage_per_core(prev_snapshot);
                for (core_idx, usage) in per_core_usage.iter().enumerate() {
                    let labels = CpuCoreLabels {
                        core: core_idx.to_string(),
                    };
                    self.cpu_usage_per_core.get_or_create(&labels).set(*usage);
                }
            }

            *self.last_cpu_snapshot.lock().await = Some(current_snapshot);

            Ok(())
        })
    }
}

/// Collector for filesystem usage metrics.
#[derive(Debug, Default)]
pub struct FilesystemStatsCollector {
    fs_usage_ratio: Family<FilesystemLabels, Gauge<f64, AtomicU64>>,
    mountpoints: Mutex<Vec<String>>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct FilesystemLabels {
    mountpoint: String,
    device: String,
    fstype: String,
}

impl Metric for FilesystemStatsCollector {
    fn init(&self, options: hashbrown::HashMap<String, String>) -> DynFuture<'_, Result<()>> {
        Box::pin(async move {
            let mountpoints = if let Some(mountpoints_str) = options.get("mountpoints") {
                mountpoints_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect()
            } else {
                vec!["/".to_string()] // Default to root filesystem
            };

            *self.mountpoints.lock().await = mountpoints;
            Ok(())
        })
    }

    fn register(&self, registry: &mut prometheus_client::registry::Registry) {
        registry.register(
            "litemon_fs_usage_ratio",
            "Filesystem usage ratio (0.0-1.0)",
            self.fs_usage_ratio.clone(),
        );
    }

    fn collect(&self) -> DynFuture<'_, Result<()>> {
        Box::pin(async move {
            let mountpoints = self.mountpoints.lock().await;

            for mountpoint in &*mountpoints {
                match FilesystemUsage::new(&mountpoint).await {
                    Ok(usage) => {
                        let labels = FilesystemLabels {
                            mountpoint: usage.mount_point.clone(),
                            device: usage.device.clone(),
                            fstype: usage.fs_type.clone(),
                        };
                        self.fs_usage_ratio
                            .get_or_create(&labels)
                            .set(usage.usage_ratio);
                    }
                    Err(e) => {
                        return Err(anyhow::anyhow!(
                            "Failed to collect filesystem stats for {}: {}",
                            mountpoint,
                            e
                        ));
                    }
                }
            }

            Ok(())
        })
    }
}

/// Collector for network interface metrics.
#[derive(Debug, Default)]
pub struct NetworkStatsCollector {
    bytes_received: Family<NetworkLabels, Counter<f64, AtomicU64>>,
    errors_received: Family<NetworkLabels, Counter<f64, AtomicU64>>,
    bytes_sent: Family<NetworkLabels, Counter<f64, AtomicU64>>,
    errors_sent: Family<NetworkLabels, Counter<f64, AtomicU64>>,
    interfaces: Mutex<Vec<String>>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct NetworkLabels {
    interface: String,
}

impl Metric for NetworkStatsCollector {
    fn init(&self, options: hashbrown::HashMap<String, String>) -> DynFuture<'_, Result<()>> {
        Box::pin(async move {
            let interfaces = if let Some(interfaces_str) = options.get("interfaces") {
                interfaces_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect()
            } else {
                // Default to common interfaces if none specified
                vec!["eth0".to_string()]
            };

            *self.interfaces.lock().await = interfaces;
            Ok(())
        })
    }

    fn register(&self, registry: &mut prometheus_client::registry::Registry) {
        registry.register(
            "litemon_net_bytes_received",
            "Network bytes received",
            self.bytes_received.clone(),
        );
        registry.register(
            "litemon_net_errors_received",
            "Network errors received",
            self.errors_received.clone(),
        );
        registry.register(
            "litemon_net_bytes_sent",
            "Network bytes sent",
            self.bytes_sent.clone(),
        );
        registry.register(
            "litemon_net_errors_sent",
            "Network errors sent",
            self.errors_sent.clone(),
        );
    }

    fn collect(&self) -> DynFuture<'_, Result<()>> {
        Box::pin(async move {
            let interfaces = self.interfaces.lock().await;
            let network_stats = NetworkStats::all().await?;

            for interface_name in &*interfaces {
                if let Some(interface_stats) = network_stats.interfaces.get(interface_name) {
                    let labels = NetworkLabels {
                        interface: interface_name.clone(),
                    };

                    let bytes_received_prev = self.bytes_received.get_or_create(&labels).get();
                    self.bytes_received
                        .get_or_create(&labels)
                        .inc_by(interface_stats.recv_bytes as f64 - bytes_received_prev);

                    let errors_received_prev = self.errors_received.get_or_create(&labels).get();
                    self.errors_received
                        .get_or_create(&labels)
                        .inc_by(interface_stats.recv_errors as f64 - errors_received_prev);

                    let bytes_sent_prev = self.bytes_sent.get_or_create(&labels).get();
                    self.bytes_sent
                        .get_or_create(&labels)
                        .inc_by(interface_stats.sent_bytes as f64 - bytes_sent_prev);

                    let errors_sent_prev = self.errors_sent.get_or_create(&labels).get();
                    self.errors_sent
                        .get_or_create(&labels)
                        .inc_by(interface_stats.sent_errors as f64 - errors_sent_prev);
                }
            }

            Ok(())
        })
    }
}

/// Collector for systemd unit state metrics.
#[derive(Debug)]
pub struct SystemdUnitStateCollector {
    unit_state: Family<SystemdUnitLabels, Gauge<u32, AtomicU32>>,
    units: Mutex<Vec<String>>,
    systemd_client: Mutex<Option<SystemdUnitState<'static>>>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct SystemdUnitLabels {
    unit: String,
    state: String,
}

impl Default for SystemdUnitStateCollector {
    fn default() -> Self {
        Self {
            unit_state: Family::default(),
            units: Mutex::new(Vec::new()),
            systemd_client: Mutex::new(None),
        }
    }
}

impl Metric for SystemdUnitStateCollector {
    fn init(&self, options: hashbrown::HashMap<String, String>) -> DynFuture<'_, Result<()>> {
        Box::pin(async move {
            // Parse units from options
            let units = if let Some(units_str) = options.get("units") {
                units_str.split(',').map(|s| s.trim().to_string()).collect()
            } else {
                Vec::new()
            };

            *self.units.lock().await = units;

            let client = SystemdUnitState::new().await?;
            *self.systemd_client.lock().await = Some(client);

            Ok(())
        })
    }

    fn register(&self, registry: &mut prometheus_client::registry::Registry) {
        registry.register(
            "litemon_systemd_unit_state",
            "Systemd unit state (1 for current state, 0 otherwise)",
            self.unit_state.clone(),
        );
    }

    fn collect(&self) -> DynFuture<'_, Result<()>> {
        Box::pin(async move {
            let units = self.units.lock().await;

            if let Some(client) = self.systemd_client.lock().await.as_ref() {
                for unit_name in &*units {
                    match client.active_state(unit_name).await {
                        Ok(state) => {
                            for state_name in ActiveState::all_states() {
                                let labels = SystemdUnitLabels {
                                    unit: unit_name.clone(),
                                    state: state_name.to_string(),
                                };
                                self.unit_state.get_or_create(&labels).set(0);
                            }

                            let current_labels = SystemdUnitLabels {
                                unit: unit_name.clone(),
                                state: state.to_string(),
                            };
                            self.unit_state.get_or_create(&current_labels).set(1);
                        }
                        Err(e) => {
                            return Err(anyhow::anyhow!(
                                "Failed to get state for unit {}: {}",
                                unit_name,
                                e
                            ));
                        }
                    }
                }
            }

            Ok(())
        })
    }
}

#[derive(Debug, Default)]
#[allow(clippy::type_complexity)]
pub struct NodeInfoCollector {
    metric: std::sync::Mutex<Option<Info<Vec<(&'static str, String)>>>>,
    uname: Mutex<Option<NodeInfo>>,
}

impl Metric for NodeInfoCollector {
    fn init(&self, _options: hashbrown::HashMap<String, String>) -> DynFuture<'_, Result<()>> {
        Box::pin(async move {
            let info = NodeInfo::new()?;

            {
                let mut lock = self.metric.lock().expect("not poisoned");
                *lock = Some(Info::new(vec![
                    ("hostname", info.hostname.clone()),
                    ("arch", info.arch.clone()),
                    ("uptime", format!("{}", info.uptime.as_secs())),
                ]))
            }

            {
                let mut lock = self.uname.lock().await;
                *lock = Some(info);
            }

            Ok(())
        })
    }

    fn register(&self, registry: &mut prometheus_client::registry::Registry) {
        registry.register(
            // The `_info` is automatically appended.
            "litemon_node",
            "System information about the node",
            self.metric
                .lock()
                .expect("not poisoned")
                .take()
                .expect("must be initialized"),
        )
    }

    fn collect(&self) -> DynFuture<'_, Result<()>> {
        Box::pin(async move { Ok::<(), _>(()) })
    }
}
