//! Tests for the config module.

use std::path::PathBuf;
use std::time::Duration;

use litemon::config::UserConfig;

// #[test]
// fn load_config_from_path() {
//     let config = r#"
// metrics {
//   cpu_seconds enabled=#true period_ms=200
//   loadavg enabled=#false
//   memory_used enabled=#true
//   systemd_unit_state enabled=#true {
//     units "valkey.service" "postgresql.service"
//   }
//   network_throughput enabled=#true {
//     interfaces "eth0" "lo"
//   }
//   disk_usage enabled=#false {
//     mountpoints "/"
//   }
// }
//         "#;
//     let tmp =
//         std::env::var("CARGO_TARGET_TMPDIR").map_or_else(|_| std::env::temp_dir(), PathBuf::from);
//     let filepath = tmp.join("load_config_from_path_test.kdl");
//     std::fs::write(&filepath, config).unwrap();

//     smol::block_on(async move {
//         let config = UserConfig::from_path(&filepath).await.unwrap();
//         assert!(config.metrics.cpu_seconds.enabled);
//         assert_eq!(config.metrics.cpu_seconds.period, Duration::from_millis(200));
//         assert!(!config.metrics.loadavg.enabled);
//         assert!(config.metrics.systemd_unit_state.enabled);
//         assert_eq!(config.metrics.systemd_unit_state.units.len(), 2);
//         assert_eq!(config.metrics.systemd_unit_state.units[0], "valkey.service");
//         assert_eq!(config.metrics.systemd_unit_state.units[1], "postgresql.service");
//         assert!(config.metrics.network_throughput.enabled);
//         assert_eq!(config.metrics.network_throughput.interfaces.len(), 2);
//         assert_eq!(config.metrics.network_throughput.interfaces[0], "eth0");
//         assert_eq!(config.metrics.network_throughput.interfaces[1], "lo");
//         assert!(!config.metrics.disk_usage.enabled);
//         assert_eq!(config.metrics.disk_usage.mountpoints.len(), 1);
//         assert_eq!(config.metrics.disk_usage.mountpoints[0], "/");
//     });
// }
