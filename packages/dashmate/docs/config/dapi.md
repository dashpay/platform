# DAPI Configuration

Dashmate runs the Rust implementation of DAPI (`rs-dapi`) to expose gRPC, gRPC-Web, JSON-RPC, and streaming endpoints for Dash Platform.

## rs-dapi (Rust)

### Docker

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.dapi.rsDapi.docker.image` | Docker image for rs-dapi | _unset_ (see below) | `dashpay/rs-dapi:latest` |
| `platform.dapi.rsDapi.docker.build.enabled` | Enable custom build | `false` | `true` |
| `platform.dapi.rsDapi.docker.build.context` | Build context directory | `path.join(PACKAGE_ROOT_DIR, '..', '..')` (Dash Platform repo root) | `"/path/to/context"` |
| `platform.dapi.rsDapi.docker.build.dockerFile` | Path to Dockerfile | `path.join(PACKAGE_ROOT_DIR, '..', '..', 'Dockerfile')` | `"/path/to/Dockerfile"` |
| `platform.dapi.rsDapi.docker.build.target` | Target build stage | `rs-dapi` | `"rs-dapi"` |
| `platform.dapi.rsDapi.docker.deploy.replicas` | Number of replicas | `1` | `2` |

### Health Monitoring and Metrics

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.dapi.rsDapi.metrics.host` | Host interface exposed on the Docker host | `127.0.0.1` | `0.0.0.0` |
| `platform.dapi.rsDapi.metrics.port` | Host port for both health checks and Prometheus metrics | `9091` (mainnet), `19091` (testnet), `29091` (local) | `9191` |

The rs-dapi metrics server exposes `/health` and `/metrics`. Prometheus-compatible metrics are served from `/metrics` on the configured port, allowing separate node instances on the same machine to use distinct ports. The `/health` endpoint aggregates dependency checks (Drive, Tenderdash, Core) and returns `503` when any upstream component is unhealthy.

Dashmate offsets the default metrics port per preset (mainnet 9091, testnet 19091, local 29091) to avoid clashes when running multiple environments concurrently.

### Logging

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.dapi.rsDapi.logs.level` | rs-dapi log verbosity. Accepts standard levels (`error`, `warn`, `info`, `debug`, `trace`, `off`) or a full `RUST_LOG` filter string | `info` | `debug` |
| `platform.dapi.rsDapi.logs.jsonFormat` | Enable structured JSON application logs (`true`) or human-readable logs (`false`) | `false` | `true` |
| `platform.dapi.rsDapi.logs.accessLogPath` | Absolute path for HTTP/gRPC access logs. Empty or `null` disables access logging | `null` | `"/var/log/rs-dapi/access.log"` |
| `platform.dapi.rsDapi.logs.accessLogFormat` | Access log output format | `combined` | `json` |

## Timeouts

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.dapi.rsDapi.waitForStResultTimeout` | Timeout for state transition results (ms) | `120000` | `240000` |

### Image versions

`platform.dapi.rsDapi.docker.image` is unset by default, which means "use the rs-dapi image published for
this dashmate version". Nothing pins it into your config, so upgrading dashmate moves it
automatically and no config change is needed.

`dashmate config get` shows the image that will actually run, and `dashmate config`
marks it `(default)`:

```
$ dashmate config get platform.dapi.rsDapi.docker.image
dashpay/rs-dapi:4-rc
$ dashmate config get --raw platform.dapi.rsDapi.docker.image
null
```

Setting it pins it, and dashmate will never change it again — including across upgrades:

```
$ dashmate config set platform.dapi.rsDapi.docker.image registry.example.com/rs-dapi:patched
```

Set it back to `null` to return to tracking the published image.


This timeout controls how long rs-dapi waits for Drive to report the outcome of a state transition before returning a timeout error to the client.
