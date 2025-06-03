//! Lightweight HTTP server for serving metrics.

use anyhow::Context;
use smol::io::{AsyncReadExt, AsyncWriteExt};

async fn try_parse_http<F, Fut>(mut stream: smol::net::TcpStream, f: F) -> anyhow::Result<()>
where
    F: FnOnce(smol::net::TcpStream) -> Fut,
    Fut: Future<Output = anyhow::Result<()>>,
{
    let mut buf: Vec<u8> = Vec::new();
    let mut tmp = [0_u8; 4096];

    loop {
        match stream.read(&mut tmp).await {
            Ok(n) => {
                if n > 0 {
                    buf.extend_from_slice(&tmp[0..n]);
                }

                let mut headers = [httparse::EMPTY_HEADER; 64];
                let mut req = httparse::Request::new(&mut headers);
                if req
                    .parse(&buf)
                    .context("parsing http request")?
                    .is_complete()
                {
                    if req.method == Some("GET") && req.path == Some("/metrics") {
                        return f(stream).await;
                    }

                    stream.write_all(b"HTTP/1.1 404 Not Found\nServer: minimon\n\n").await?;
                    return Ok(());
                }
            }
            Err(err) => return Err(anyhow::anyhow!("reading from stream: {err}")),
        }
    }
}

async fn serve(stream: smol::net::TcpStream) -> anyhow::Result<()> {
    try_parse_http(stream, |mut stream| async move {
        stream.write_all(b"HTTP/1.1 200 OK\n\n").await?;
        Ok::<(), anyhow::Error>(())
    })
    .await?;

    Ok(())
}

/// Serves the metrics endpoint.
pub async fn listen(listen_addr: &str, listen_port: u16) -> anyhow::Result<()> {
    let addr: std::net::IpAddr = listen_addr
        .parse()
        .with_context(|| format!("parsing listen addr: {listen_addr}"))?;
    println!("listening on {addr}:{listen_port}");

    let listener = smol::net::TcpListener::bind((addr, listen_port))
        .await
        .with_context(|| format!("bind to {addr}"))?;

    loop {
        let (stream, _addr) = listener.accept().await.context("accepting connection")?;

        smol::spawn(async move {
            if let Err(err) = serve(stream).await {
                eprintln!("error: serving request: {err}");
            }
        })
        .detach();
    }
}
