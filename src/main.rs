use std::rc::Rc;
use std::time::Duration;

use anyhow::Result;
use minimon::http;
use minimon::metrics::cpu::{CpuUsage, LoadAverages};
use minimon::metrics::fs::FilesystemUsage;
use minimon::metrics::memory::get_memory_used_percentage;
use minimon::metrics::systemd_unit_state::SystemdUnitState;
use prometheus_client::encoding::text;
use prometheus_client::metrics::gauge::ConstGauge;
use prometheus_client::registry::Registry;

/// Synchronous entrypoint into the application.
fn main() {
    let ex = Rc::new(smol::LocalExecutor::new());
    smol::block_on(ex.run(async {
        async_main(&ex).await;
    }));
}

#[derive(Debug)]
struct CliArgs {
    /// Optional listen address. By default, listens on `127.0.0.1`
    listen_address: String,
    /// Optional listen port, by default, listens on `9774`.
    listen_port: u16,
}

impl Default for CliArgs {
    fn default() -> Self {
        Self {
            listen_address: "127.0.0.1".to_owned(),
            listen_port: 9774,
        }
    }
}

impl CliArgs {
    pub fn from_env() -> Result<Self> {
        use lexopt::prelude::*;

        let mut ret = Self::default();
        let mut parser = lexopt::Parser::from_env();
        while let Some(arg) = parser.next()? {
            match arg {
                Short('n') | Long("listen") => {
                    ret.listen_address = parser.value()?.to_string_lossy().to_string();
                }
                Short('P') | Long("port") => {
                    ret.listen_port = parser.value()?.parse()?;
                }
                _ => return Err(arg.unexpected().into()),
            }
        }

        Ok(ret)
    }
}

/// Real, asynchronous entrypoint.
async fn async_main(_ex: &Rc<smol::LocalExecutor<'_>>) {
    let args = CliArgs::from_env().expect("invalid args");

    http::listen(&args.listen_address, args.listen_port)
        .await
        .unwrap();

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

    let mut registry = <Registry>::default();
    let mem_gauge = ConstGauge::new(mem);
    registry.register(
        "minimon_mem_percentage",
        "Percentage (0-1) of used memory",
        mem_gauge,
    );

    let mut buffer = String::new();
    text::encode(&mut buffer, &registry).unwrap();
    println!("{buffer}");
}
