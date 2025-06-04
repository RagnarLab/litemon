use std::rc::Rc;

use litemon::args::CliArgs;
use litemon::config::UserConfig;
use litemon::http;

#[global_allocator]
static GLOBAL_ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

/// Synchronous entrypoint into the application.
fn main() {
    let ex = Rc::new(smol::LocalExecutor::new());
    smol::block_on(ex.run(async {
        async_main(&ex).await;
    }));
}

/// Real, asynchronous entrypoint.
async fn async_main(_ex: &Rc<smol::LocalExecutor<'_>>) {
    let args = CliArgs::from_env().expect("invalid args");
    let config = UserConfig::from_path(&args.config_path)
        .await
        .expect("invalid config");

    http::listen(&args.listen_address, args.listen_port)
        .await
        .unwrap();
}
