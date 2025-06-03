use std::time::Duration;

use minimon::metrics::cpu::{CpuUsage, LoadAverages};
use minimon::metrics::fs::FilesystemUsage;
use minimon::metrics::memory::get_memory_used_percentage;
use minimon::metrics::systemd_unit_state::SystemdUnitState;
use tokio::runtime;

/// Synchronous entrypoint into the application.
fn main() {
    let rt = runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("building runtime");

    rt.block_on(async_main())
}

/// Real, asynchronous entrypoint.
async fn async_main() {
    let systemd_unit = SystemdUnitState::new().await.unwrap();
    let state = systemd_unit
        .active_state("NetworkManager.service")
        .await
        .unwrap();
    println!("state={state}");

    let mem = get_memory_used_percentage().await.unwrap();
    println!("mem={mem}");

    let avg = LoadAverages::current().await.unwrap();
    println!("load={avg:?}");
    let cpu = CpuUsage::period(Duration::from_millis(200)).await.unwrap();
    println!("cpu={cpu:?}");

    let rootfs = FilesystemUsage::new("/").unwrap();
    println!("fs={}", rootfs.usage_ratio);
}
