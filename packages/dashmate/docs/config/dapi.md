# DAPI Configuration

DAPI provides API services for Dash Platform, allowing external applications to interact with the platform.

## Docker

| Option | Description | Default | Example                 |
|--------|-------------|---------|-------------------------|
| `platform.dapi.api.docker.image` | Docker image for DAPI | `dashpay/dapi:${version}` | `dashpay/dapi:latest`   |
| `platform.dapi.api.docker.build.enabled` | Enable custom build | `false` | `true`                  |
| `platform.dapi.api.docker.build.context` | Build context directory | `null` | `"/path/to/context"`    |
| `platform.dapi.api.docker.build.dockerFile` | Path to Dockerfile | `null` | `"/path/to/Dockerfile"` |
| `platform.dapi.api.docker.build.target` | Target build stage in multi-stage builds | `null` | `"dapi"` |
| `platform.dapi.api.docker.deploy.replicas` | Number of DAPI replicas | `1` | `3`                     |

The `docker.build` object allows for custom build settings:
```json
{
  "build": {
    "enabled": true,
    "context": "/path/to/build/context",
    "dockerFile": "/path/to/Dockerfile",
    "target": "dapi"
  }
}
```

These settings allow you to build the DAPI API Docker image from source. If `enabled` is set to `true`, Dashmate will build the Docker image using the specified context directory and Dockerfile.

## Other

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.dapi.api.waitForStResultTimeout` | Timeout for state transitions (ms) | `120000` | `240000` |

This timeout setting controls how long DAPI will wait for state transition results before returning a timeout error to the client. It is specified in milliseconds.

## rs-dapi (Rust)

### Docker

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.dapi.rsDapi.docker.image` | Docker image for rs-dapi | `dashpay/rs-dapi:${version}` | `dashpay/rs-dapi:latest` |
| `platform.dapi.rsDapi.docker.build.enabled` | Enable custom build | `false` | `true` |
| `platform.dapi.rsDapi.docker.build.context` | Build context directory | `null` | `"/path/to/context"` |
| `platform.dapi.rsDapi.docker.build.dockerFile` | Path to Dockerfile | `null` | `"/path/to/Dockerfile"` |
| `platform.dapi.rsDapi.docker.build.target` | Target build stage | `rs-dapi` | `"rs-dapi"` |
| `platform.dapi.rsDapi.docker.deploy.replicas` | Number of replicas | `1` | `2` |

### Health Monitoring and Metrics

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.dapi.rsDapi.metrics.host` | Host interface exposed on the Docker host | `127.0.0.1` | `0.0.0.0` |
| `platform.dapi.rsDapi.metrics.port` | Host port for both health checks and Prometheus metrics | `9091` | `9191` |

The rs-dapi metrics server exposes `/health`, `/ready`, `/live`, and `/metrics`. Prometheus-compatible metrics are served from `/metrics` on the configured port, allowing separate node instances on the same machine to use distinct ports.

### Logging

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.dapi.rsDapi.logging.level` | rs-dapi log verbosity. Accepts standard levels (`error`, `warn`, `info`, `debug`, `trace`, `off`) or a full `RUST_LOG` filter string | `info` | `debug` |
