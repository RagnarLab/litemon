<div align="center">
  <h1>LiteMon</h1>
  <p>
    <strong>A very minimal and lightweight metric collector for Linux systems.</strong>
  </p>
  <p>

[![Build Status](https://github.com/ragnarlab/litemon/actions/workflows/ci.yml/badge.svg)](https://github.com/ragnarlab/litemon/actions)
[![dependency status](https://deps.rs/repo/github/RagnarLab/litemon/status.svg)](https://deps.rs/repo/github/RagnarLab/litemon)
![GitHub License](https://img.shields.io/github/license/RagnarLab/litemon)
</div>

LiteMon allows lightweight monitoring of Linux nodes (VM, bare-metal, etc.).
LiteMon is carefully crafted to use as little resources as possible.

LiteMon is using less than 5 MiB of RAM (99th percentile), the binary size is
less than 5 MB, and uses very little I/O and CPU. Binaries are built with the
musl libc and have no external dependencies.

LiteMon is written in Rust, and uses the
[Smol](https://github.com/smol-rs/smol) async runtime.


## Installation

**Latest version:**

```
0.1.0
```

### Ubuntu/Debian

```bash
# Install litemon
sudo dpkg -i $(curl -w "%{filename_effective}" -SsLO "https://github.com/RagnarLab/litemon/releases/download/0.1.0/litemon-0.1.0-1.$(uname -m).deb")

# Configure litemon
sudo mv /etc/litemon/config.kdl.example /etc/litemon/config.kdl
sudo $EDITOR /etc/litemon/config.kdl

# Restart litemon
sudo systemctl restart litemon
```

### RHEL/Fedora/CentOS Stream

```bash
# Install litemon
sudo yum -iv "https://github.com/RagnarLab/litemon/releases/download/0.1.0/litemon-0.1.0-1.$(uname -m).rpm"

# Configure litemon
sudo mv /etc/litemon/config.kdl.example /etc/litemon/config.kdl
sudo $EDITOR /etc/litemon/config.kdl

# Restart litemon
sudo systemctl restart litemon
```

## Configuration

By default, `litemon` reads the configuration from `/etc/litemon/config.kdl`.
The configuration is written in [KDL](https://kdl.dev/).

```kdl
metrics {
  cpu_seconds enabled=#true period_ms=200

  loadavg enabled=#true

  memory_used enabled=#true

  systemd_unit_state enabled=#true {
    // List all the units to monitor.
    units "docker.service"
  }

  network_throughput enabled=#true {
    // List all interfaces to monitor
    interfaces \
        "eth0" \
        "lo"
  }

  disk_usage enabled=#true {
    mountpoints "/"
  }
}
```


## CLI

```
Usage: litemon [OPTIONS] [PATH-TO-CONFIG]

Options:
-n, --listen          IP address to listen. Default: 127.0.0.1
-P, --port            Port to listen. Default: 9774
-V, --version         Print version info and exit
-h, --help            Print help and exit
```


## Using with alloy

You can easily use `litemon` with [Grafana Alloy](https://grafana.com/docs/alloy/latest/).

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


## Metrics

|          Metric Name          | Metric Type |          Description          |        Cardinality        |
| ----------------------------- | ----------- | ----------------------------- | ------------------------- |
| litemon_load_avg_1m           | Gauge       | Load average over 1 minute.   | 1 per host |
| litemon_load_avg_5m           | Gauge       | Load average over 5 minutes.  | 1 per host |
| litemon_load_avg_15m          | Gauge       | Load average over 15 minutes. | 1 per host |
| litemon_cpu_usage_overall     | Gauge       | Overall CPU usage percentage (0.0-1.0). | 1 per host |
| litemon_cpu_usage_per_core    | Gauge       | Per-core CPU usage percentage (0.0-1.0). | 1 per cpu core |
| litemon_mem_used_percentage   | Gauge       | Memory used (0.0-1.0) in percent. | 1 per host |
| litemon_systemd_unit_state    | Gauge       | Systemd unit state (1 for current state, 0 otherwise). | 1 per service, 1 per state, 8 states |
| litemon_net_bytes_received    | Counter     | Network bytes received.       | 1 per host, 1 per network interface |
| litemon_net_errors_received   | Counter     | Network errors received.      | 1 per host, 1 per network interface |
| litemon_net_bytes_sent        | Counter     | Network bytes sent.           | 1 per host, 1 per network interface |
| litemon_net_errors_sent       | Counter     | Network errors sent.          | 1 per host, 1 per network interface |
| litemon_fs_usage_ratio        | Gauge       | Filesystem usage ratio (0.0-1.0). | 1 per host, 1 per mount point |


## Support

> Development of LiteMon is sponsored by [RagnarLab](https://ragnarlab.com). RagnarLab is a Rust consultancy based in Stuttgart, Germany. We provide Rust development from prototype to product, helping you write safer software. [Interested in Rust? Get in touch with us.](https://ragnarlab.com)

#### License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>
