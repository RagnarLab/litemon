## Testing with Grafana OTEL LGTM Docker container

The Grafana OTEL LGTM Docker container is a container that bundles Grafana,
Prometheus, Loki, Tempo and a OTEL collector into a single image for testing,
development and demonstration purposes. We can use it to test `LiteMon` with
`Alloy`.

Below the process is described:


### 1. Spin up Grafana & Prometheus

Run Grafana and Prometheus using the described container image.

```bash
docker run \
    -p 3000:3000 \
    -p 4317:4317 \
    -p 4318:4318 \
    -p 3100:3100 \
    -p 9090:9090 \
    -p 4040:4040 \
    -p 3200:3200 \
    -ti \
    grafana/otel-lgtm
```


### 2. Download and run Alloy

Alloy is the scraper that scrapes `LiteMon` and pushes metrics to Prometheus.

```bash
# Download & install alloy (e.g., on RHEL)
wget https://github.com/grafana/alloy/releases/download/v1.9.1/alloy-1.9.1-1.amd64.rpm
sudo yum installlocal ./alloy-1.9.1-1.amd64.rpm

# Run alloy with testing config
./alloy-linux-amd64 run ./testing/config.alloy
```


### 3. Run LiteMon

Last step is to run LiteMon to allow it to be scraped by Alloy.

```bash
cargo run
```


### 4. Open Grafana

In your web browser navigate to [localhost:3000](http://localhost) to open the
Grafana Web UI.
