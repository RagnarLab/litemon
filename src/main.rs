use easy_parallel::Parallel;
use smol::channel::unbounded;
use smol::future;
use std::sync::Arc;
use tracing::Level;

use litemon::args::CliArgs;
use litemon::collector::Collector;
use litemon::config::UserConfig;
use litemon::http;

#[global_allocator]
static GLOBAL_ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

/// Synchronous entrypoint into the application.
fn main() {
    // let rt = tokio::runtime::Builder::new_current_thread()
    //     .enable_all()
    //     .build()
    //     .expect("building tokio runtime failed");
    // rt.block_on(async move { async_main().await })

    let ex = Arc::new(smol::Executor::new());
    let (signal, shutdown) = unbounded::<()>();

    Parallel::new()
        .each(0..2, |_| future::block_on(ex.run(shutdown.recv())))
        .finish(|| {
            future::block_on(async {
                async_main(ex.clone()).await;
            })
        });
}

/// Real, asynchronous entrypoint.
async fn async_main(_ex: Arc<smol::Executor<'_>>) {
    tracing_subscriber::fmt::fmt()
        .compact()
        .with_max_level(Level::TRACE)
        .init();

    let args = CliArgs::from_env().expect("invalid args");
    let config = UserConfig::from_path(&args.config_path)
        .await
        .expect("invalid config");

    let collector = Collector::new();
    collector
        .create_from_config(&config)
        .await
        .expect("creating collectors failed");
    collector
        .register()
        .await
        .expect("registering metrics failed");

    // Figlet font: Standard
    // Alternatives: Sland, Big
    println!(r"._.    _ _       __  __");
    println!(r"| |   (_) |_ ___|  \/  | ___  _ __");
    println!(r"| |   | | __/ _ \ |\/| |/ _ \| '_ \");
    println!(r"| |___| | ||  __/ |  | | (_) | | | |");
    println!(r"|_____|_|\__\___|_|  |_|\___/|_| |_|");

    http::listen(collector.clone(), &args.listen_address, args.listen_port)
        .await
        .unwrap();
}
