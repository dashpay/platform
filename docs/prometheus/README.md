# Prometheus Configuration

This directory contains a minimal Prometheus stack for scraping metrics from a Dashmate-managed Platform network. It is meant for local debugging and pairs with the metric endpoints that Dashmate exposes when you enable them in your node configuration.

## Files
- `docs/prometheus/docker-compose.yml` brings up two containers: a read-only Docker Socket Proxy that exposes `tcp://127.0.0.1:2375`, and Prometheus itself listening on `:9080`.
- `docs/prometheus/prometheus.yml` configures Prometheus to use Docker service discovery against the proxy and scrape every container that carries Dashmate’s `prometheus.io/*` labels.

## Prepare Dashmate
1. Enable metrics on the services you want Prometheus to monitor (examples):
   - `dashmate config set platform.drive.abci.metrics.enabled true`
   - `dashmate config set platform.drive.tenderdash.metrics.enabled true`
   - `dashmate config set platform.dapi.rsDapi.metrics.enabled true`
   - `dashmate config set platform.gateway.metrics.enabled true`
   - `dashmate config set platform.gateway.rateLimiter.metrics.enabled true`
2. Restart your network so the containers are recreated with the Prometheus labels: `yarn restart`.

The labels Dashmate adds map cleanly onto the Prometheus discovery config:
- `prometheus.io/scrape=true` enables scraping.
- `prometheus.io/path` overrides the metrics path (defaults to `/metrics`).
- `prometheus.io/port` is rewritten to `127.0.0.1:<port>` so scraping stays on the host network.
- `org_dashmate_service_title` and `org_dashmate_config_name` become `service` and `config` labels on each time series.

## Run Prometheus
```bash
docker compose -f docs/prometheus/docker-compose.yml up -d
```

Prometheus stores data in the `prometheus-data` volume and exposes its UI at `http://127.0.0.1:9080`. Reload the configuration after any edits with:
```bash
curl -X POST http://127.0.0.1:9080/-/reload
```

### Customisation Tips
- Adjust `scrape_interval` or add more `scrape_configs` in `prometheus.yml` as needed.
- To scrape additional containers, attach the same `prometheus.io/*` labels to them, or add dedicated jobs to the configuration.
- If your Docker daemon is not local, update the proxy service or mount a different socket before starting the stack.
