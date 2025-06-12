//! Lightweight HTTP server for serving metrics.

use std::convert::Infallible;

use anyhow::{Context, Result};
use http::{header, HeaderValue, StatusCode};
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::service::service_fn;
use hyper::{body::Incoming, Request, Response};
use hyper_util::rt::{TokioIo, TokioTimer};

use crate::collector::Collector;
use crate::http_utils::{internal_server_error, not_found};

async fn serve_metrics(collector: &Collector) -> Result<Response<BoxBody<Bytes, Infallible>>> {
    println!("serve metrics...");
    let metrics = collector.collect_and_encode().await?.into_bytes();
    let buf = Bytes::from(metrics);
    println!("buf created");

    let body = Full::new(buf).boxed();
    println!("body created");
    let mut res = Response::new(body);
    println!("res created");
    *res.status_mut() = StatusCode::OK;
    println!("res status");
    res.headers_mut().insert(
        header::SERVER,
        HeaderValue::from_str(&format!("litemon/{}", env!("CARGO_PKG_VERSION")))?,
    );
    println!("res header");

    Ok(res)
}

async fn serve_request(
    collector: Collector,
    req: Request<Incoming>,
) -> Result<Response<BoxBody<Bytes, Infallible>>> {
    use hyper::Method;

    dbg!(&req);
    println!("{:?} {}", req.method(), req.uri().path());
    match (req.method(), req.uri().path()) {
        // (&Method::GET, "/metrics") => {
        //     let res = serve_metrics(&collector)
        //         .await
        //         .inspect_err(|err| eprintln!("error serving metrics request: {err}"))
        //         .unwrap_or_else(|_err| internal_server_error());
        //     dbg!(&res);
        //     Ok(res)
        // }
        (_, _) => Ok(not_found()),
    }
}

async fn handle_client(
    collector: Collector,
    mut stream: tokio::net::TcpStream,
) -> anyhow::Result<()> {
    let service = service_fn(move |req| serve_request(collector.clone(), req));

    hyper::server::conn::http1::Builder::new()
        .timer(TokioTimer::new())
        .serve_connection(TokioIo::new(stream), service)
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
    println!("listening on {addr}:{listen_port}");

    let listener = tokio::net::TcpListener::bind((addr, listen_port))
        .await
        .with_context(|| format!("bind to {addr}"))?;

    loop {
        let (stream, _addr) = listener.accept().await.context("accepting connection")?;
        dbg!(&_addr);

        let collector = collector.clone();
        tokio::task::spawn(async move {
            if let Err(err) = handle_client(collector, stream).await {
                dbg!(&err);
                eprintln!("error: serving request: {err:?}");
            }
        });
    }
}
