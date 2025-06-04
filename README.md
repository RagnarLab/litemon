<div align="center">
  <h1>LiteMon</h1>
  <p>
    <strong>A very minimal and lightweight metric collector for Linux systems.</strong>
  </p>
  <p>

[![Build Status](https://github.com/ragnarlab/litemon/actions/workflows/ci.yml/badge.svg)](https://github.com/ragnarlab/litemon/actions)
[![Crates.io](https://img.shields.io/crates/v/litemon.svg)](https://crates.io/crates/litemon)
![License](https://img.shields.io/crates/l/litemon.svg)
</div>

LiteMon allows lightweight monitoring of Linux nodes (VM, bare-metal, etc.).
LiteMon is carefully crafted to use as little resources as possible.

LiteMon is using less than 1 MiB of RAM, the binary size is less than 5 MB, and
uses very little I/O and CPU. Binaries are built with the musl libc and have no
external dependencies.

LiteMon is written in Rust, and uses the
[Smol](https://github.com/smol-rs/smol) async runtime.


## Installation

## Using with alloy

You can easily use litemon with [Grafana Alloy](https://grafana.com/docs/alloy/latest/).

```
// config.alloy

// https://grafana.com/docs/alloy/latest/reference/components/discovery/discovery.relabel/
discovery.relabel "integrations_litemon_exporter" {
  targets = [
    // Optionally, update to your listen address and port.
    {"__address__" = "localhost:9774"},
  ]

  rule {
    target_label = "instance"
    replacement  = constants.hostname
  }

  rule {
    target_label = "job"
    replacement  = "integrations/litemon"
  }
}

// https://grafana.com/docs/alloy/latest/reference/components/prometheus/prometheus.scrape/
prometheus.scrape "litemon_exporter" {
  job_name        = "integrations/litemon"
  targets         = discovery.relabel.integrations_litemon_exporter.output
  scheme          = "http"
  scrape_interval = "60s"
  metrics_path    = "/metrics"

  // Update endpoint to your prometheus ingest.
  forward_to = [prometheus.remote_write.EXAMPLE.receiver]
}
```

## Support

> Development of LiteMon is sponsored by [RagnarLab](https://ragnarlab.com). RagnarLab is a Rust consultancy based in Stuttgart, Germany. We provide Rust development from prototype to product, helping you write safer software. [Interested in Rust? Get in touch with us.](https://ragnarlab.com)

#### License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>
