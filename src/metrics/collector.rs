//! Collectors for all supported metrics.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64};

use anyhow::Result;
use futures_concurrency::future::Join;
use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::gauge::Gauge;
use smol::lock::Mutex;

use super::cpu::{CpuUsage, LoadAverages};
use super::disk::IOMetrics;
use super::fs::FilesystemUsage;
use super::info::NodeInfo;
use super::memory::MemoryStats;
use super::net::NetworkStats;
use super::pressure::SystemPressure;
use super::systemd_unit_state::{ActiveState, SystemdUnitState};
use super::{DynFuture, Metric};

/// Collector for memory stats.
#[derive(Debug, Default)]
pub struct MemoryStatsCollector {
    gauge: Gauge<f64, AtomicU64>,
}

impl Metric for MemoryStatsCollector {
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
    last_cpu_snapshot: Mutex<CpuUsage>,
    load_avg_enabled: AtomicBool,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct CpuCoreLabels {
    core: String,
}

impl CpuStatsCollector {
    pub async fn new(options: hashbrown::HashMap<String, String>) -> Result<Self> {
        let initial_snapshot = CpuUsage::now().await?;

        let ret = Self {
            load_avg_1m: Gauge::default(),
            load_avg_5m: Gauge::default(),
            load_avg_15m: Gauge::default(),
            cpu_usage_overall: Gauge::default(),
            cpu_usage_per_core: Family::default(),
            last_cpu_snapshot: Mutex::new(initial_snapshot),
            load_avg_enabled: AtomicBool::new(false),
        };
        ret.load_avg_enabled.store(
            options.get("load_avg_enabled").is_some_and(|b| b == "true"),
            std::sync::atomic::Ordering::SeqCst,
        );

        Ok(ret)
    }
}

impl Metric for CpuStatsCollector {
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
                self.load_avg_1m.set(f64::from(load_avg.one));
                self.load_avg_5m.set(f64::from(load_avg.five));
                self.load_avg_15m.set(f64::from(load_avg.fifteen));
            }

            let current_snapshot = CpuUsage::now().await?;
            let mut prev_snapshot = self.last_cpu_snapshot.lock().await;
            let overall_usage = current_snapshot.percentage_all_cores(&prev_snapshot);
            self.cpu_usage_overall.set(overall_usage);

            let per_core_usage = current_snapshot.percentage_per_core(&prev_snapshot);
            for (core_idx, usage) in per_core_usage.iter().enumerate() {
                let labels = CpuCoreLabels {
                    core: core_idx.to_string(),
                };
                self.cpu_usage_per_core.get_or_create(&labels).set(*usage);
            }

            *prev_snapshot = current_snapshot;

            Ok(())
        })
    }
}

/// Collector for filesystem usage metrics.
#[derive(Debug, Default)]
pub struct FilesystemStatsCollector {
    fs_usage_ratio: Family<FilesystemLabels, Gauge<f64, AtomicU64>>,
    mountpoints: Vec<String>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct FilesystemLabels {
    mountpoint: String,
    device: String,
    fstype: String,
}

impl FilesystemStatsCollector {
    pub fn new(options: &hashbrown::HashMap<String, String>) -> Result<Self> {
        let mountpoints = options.get("mountpoints").map_or_else(
            || vec!["/".to_string()],
            |mountpoints_str| {
                mountpoints_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect()
            },
        );

        Ok(Self {
            fs_usage_ratio: Default::default(),
            mountpoints,
        })
    }
}

impl Metric for FilesystemStatsCollector {
    fn register(&self, registry: &mut prometheus_client::registry::Registry) {
        registry.register(
            "litemon_fs_usage_ratio",
            "Filesystem usage ratio (0.0-1.0)",
            self.fs_usage_ratio.clone(),
        );
    }

    fn collect(&self) -> DynFuture<'_, Result<()>> {
        Box::pin(async move {
            let mountpoints = &self.mountpoints;

            for mountpoint in mountpoints {
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
    interfaces: Vec<String>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct NetworkLabels {
    interface: String,
}

impl NetworkStatsCollector {
    pub fn new(options: &hashbrown::HashMap<String, String>) -> Result<Self> {
        let interfaces = options.get("interfaces").map_or_else(
            || vec!["eth0".to_string()],
            |interfaces_str| {
                interfaces_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect()
            },
        );

        Ok(Self {
            bytes_received: Default::default(),
            errors_received: Default::default(),
            bytes_sent: Default::default(),
            errors_sent: Default::default(),
            interfaces,
        })
    }
}

impl Metric for NetworkStatsCollector {
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

    #[allow(clippy::cast_precision_loss)]
    fn collect(&self) -> DynFuture<'_, Result<()>> {
        Box::pin(async move {
            let interfaces = &self.interfaces;
            let network_stats = NetworkStats::all().await?;

            for interface_name in interfaces {
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
    units: Vec<String>,
    systemd_client: SystemdUnitState<'static>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct SystemdUnitLabels {
    unit: String,
    state: String,
}

impl SystemdUnitStateCollector {
    pub async fn new(options: &hashbrown::HashMap<String, String>) -> Result<Self> {
        // Parse units from options
        let units = options.get("units").map_or_else(Vec::new, |units_str| {
            units_str.split(',').map(|s| s.trim().to_string()).collect()
        });

        let client = SystemdUnitState::new().await?;

        Ok(Self {
            unit_state: Default::default(),
            units,
            systemd_client: client,
        })
    }
}

impl Metric for SystemdUnitStateCollector {
    fn register(&self, registry: &mut prometheus_client::registry::Registry) {
        registry.register(
            "litemon_systemd_unit_state",
            "Systemd unit state (1 for current state, 0 otherwise)",
            self.unit_state.clone(),
        );
    }

    fn collect(&self) -> DynFuture<'_, Result<()>> {
        Box::pin(async move {
            let units = &self.units;
            let client = &self.systemd_client;

            for unit_name in units {
                match client.active_state(unit_name).await {
                    Ok(state) => {
                        for state_name in ActiveState::all_states() {
                            let labels = SystemdUnitLabels {
                                unit: unit_name.clone(),
                                state: (*state_name).to_string(),
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
                        tracing::debug!("Failed to get state for unit {unit_name}: {e}");
                    }
                }
            }

            Ok(())
        })
    }
}

#[derive(Debug)]
pub struct NodeInfoCollector {
    metric: Family<NodeInfoLabels, Gauge>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct NodeInfoLabels {
    /// Hostname
    hostname: String,
    /// CPU architecture
    arch: String,
}

impl NodeInfoCollector {
    pub fn new() -> Result<Self> {
        let info = NodeInfo::new()?;

        let labels = NodeInfoLabels {
            hostname: info.hostname.clone(),
            arch: info.arch,
        };
        let metric = Family::<NodeInfoLabels, Gauge>::default();
        metric.get_or_create(&labels).set(1);

        Ok(Self { metric })
    }
}

impl Metric for NodeInfoCollector {
    fn register(&self, registry: &mut prometheus_client::registry::Registry) {
        registry.register(
            "litemon_node_info",
            "System information about the node",
            self.metric.clone(),
        );
    }

    fn collect(&self) -> DynFuture<'_, Result<()>> {
        Box::pin(async move { Ok::<(), _>(()) })
    }
}

#[derive(Debug, Default)]
pub struct PressureCollector {
    io_total: Gauge<u64, AtomicU64>,
    cpu_total: Gauge<u64, AtomicU64>,
    mem_total: Gauge<u64, AtomicU64>,
}

impl Metric for PressureCollector {
    fn register(&self, registry: &mut prometheus_client::registry::Registry) {
        let io_total = &self.io_total;
        registry.register(
            "litemon_io_pressure_total",
            "I/O pressure stall information (PSI) in microseconds",
            io_total.clone(),
        );

        let cpu_total = &self.cpu_total;
        registry.register(
            "litemon_cpu_pressure_total",
            "CPU pressure stall information (PSI) in microseconds",
            cpu_total.clone(),
        );

        let mem_total = &self.mem_total;
        registry.register(
            "litemon_memory_pressure_total",
            "Memory pressure stall information (PSI) in microseconds",
            mem_total.clone(),
        );
    }

    fn collect(&self) -> DynFuture<'_, Result<()>> {
        Box::pin(async move {
            let io = SystemPressure::io();
            let cpu = SystemPressure::cpu();
            let mem = SystemPressure::mem();
            let (io, cpu, mem) = (io, cpu, mem).join().await;
            let io = io?;
            let cpu = cpu?;
            let mem = mem?;

            self.io_total.set(io.total);
            self.cpu_total.set(cpu.total);
            self.mem_total.set(mem.total);

            Ok(())
        })
    }
}

#[derive(Debug)]
pub struct DiskStatsCollector {
    bytes_written: Family<DiskStatsLabels, Gauge<u64, AtomicU64>>,
    bytes_read: Family<DiskStatsLabels, Gauge<u64, AtomicU64>>,
    mountpoints: Vec<String>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct DiskStatsLabels {
    /// Device name
    device: String,
    /// Mountpoint
    mountpoint: String,
}

impl DiskStatsCollector {
    pub fn new(options: &hashbrown::HashMap<String, String>) -> Result<Self> {
        let mountpoints = options.get("mountpoints").map_or_else(
            || vec!["/".to_string()],
            |mountpoints_str| {
                mountpoints_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect()
            },
        );

        Ok(Self {
            bytes_written: Default::default(),
            bytes_read: Default::default(),
            mountpoints,
        })
    }
}

impl Metric for DiskStatsCollector {
    fn register(&self, registry: &mut prometheus_client::registry::Registry) {
        registry.register(
            "litemon_disk_bytes_written_total",
            "Number of bytes written to disk since boot",
            self.bytes_written.clone(),
        );
        registry.register(
            "litemon_disk_bytes_read_total",
            "Number of bytes read from disk since boot",
            self.bytes_read.clone(),
        );
    }

    fn collect(&self) -> DynFuture<'_, Result<()>> {
        Box::pin(async move {
            let io = IOMetrics::all().await?;
            for stats in &io.disks {
                if !self.mountpoints.iter().any(|el| el == &stats.mountpoint) {
                    continue;
                }

                let labels = DiskStatsLabels {
                    device: stats.device.clone(),
                    mountpoint: stats.mountpoint.clone(),
                };

                let bytes_read_prev = self.bytes_read.get_or_create(&labels).get();
                self.bytes_read
                    .get_or_create(&labels)
                    .inc_by(stats.bytes_read_total - bytes_read_prev);

                let bytes_written_prev = self.bytes_written.get_or_create(&labels).get();
                self.bytes_written
                    .get_or_create(&labels)
                    .inc_by(stats.bytes_written_total - bytes_written_prev);
            }

            Ok(())
        })
    }
}

#[derive(Debug, Default)]
pub struct NodeUptimeCollector {
    metric: Gauge<u64, AtomicU64>,
}

impl Metric for NodeUptimeCollector {
    fn register(&self, registry: &mut prometheus_client::registry::Registry) {
        registry.register(
            "litemon_node_uptime",
            "Uptime in seconds",
            self.metric.clone(),
        );
    }

    fn collect(&self) -> DynFuture<'_, Result<()>> {
        Box::pin(async move {
            let info = NodeInfo::new()?;
            self.metric.set(info.uptime.as_secs());
            Ok(())
        })
    }
}
