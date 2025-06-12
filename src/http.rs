//! Lightweight HTTP server for serving metrics.

use std::convert::Infallible;

use anyhow::{Context, Result};
use http::{header, HeaderValue, StatusCode};
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::service::service_fn;
use hyper::{body::Incoming, Request, Response};
use smol_hyper::rt::{FuturesIo, SmolTimer};

use crate::collector::Collector;
use crate::http_utils::{internal_server_error, not_found};

async fn serve_metrics(collector: &Collector) -> Result<Response<BoxBody<Bytes, Infallible>>> {
    let metrics = collector.collect_and_encode().await?.into_bytes();
    let buf = Bytes::from(metrics);

    let body = Full::new(buf).boxed();
    let mut res = Response::new(body);
    *res.status_mut() = StatusCode::OK;
    res.headers_mut().insert(
        header::SERVER,
        HeaderValue::from_str(&format!("litemon/{}", env!("CARGO_PKG_VERSION")))?,
    );

    Ok(res)
}

async fn serve_request(
    collector: Collector,
    req: Request<Incoming>,
) -> Result<Response<BoxBody<Bytes, Infallible>>> {
    use hyper::Method;

    match (req.method(), req.uri().path()) {
        (&Method::GET, "/metrics") => {
            let res = serve_metrics(&collector)
                .await
                .inspect_err(|err| eprintln!("error serving metrics request: {err}"))
                .unwrap_or_else(|_err| internal_server_error());
            Ok(res)
        }

        (_, _) => Ok(not_found()),
    }
}

async fn handle_client(collector: Collector, stream: smol::net::TcpStream) -> anyhow::Result<()> {
    let service = service_fn(move |req| serve_request(collector.clone(), req));

    hyper::server::conn::http1::Builder::new()
        .header_read_timeout(None)
        .timer(SmolTimer::new())
        .serve_connection(FuturesIo::new(stream), service)
        .await?;

    Ok(())
}

/// Serves the metrics endpoint.
pub async fn listen(
    collector: Collector,
    listen_addr: &str,
    listen_port: u16,
) -> anyhow::Result<()> {
    let addr: std::net::IpAddr = listen_addr
        .parse()
        .with_context(|| format!("parsing listen addr: {listen_addr}"))?;
    tracing::info!("listening on {addr}:{listen_port}");

    let listener = smol::net::TcpListener::bind((addr, listen_port))
        .await
        .with_context(|| format!("bind to {addr}"))?;

    loop {
        let (stream, _addr) = listener.accept().await.context("accepting connection")?;

        let collector = collector.clone();
        smol::spawn(async move {
            if let Err(err) = handle_client(collector, stream).await {
                tracing::error!(err = ?err, "error: serving request: {err}");
            }
        })
        .detach();
    }
}
