# Drive ABCI Configuration

Drive ABCI contains the application logic for Dash Platform. Its configuration is divided into several functional areas.

## Docker

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.drive.abci.docker.image` | Docker image for Drive ABCI | `dashpay/drive:${version}` | `dashpay/drive:latest` |
| `platform.drive.abci.docker.build` | Build settings for Drive ABCI | Object | See below |

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

It allows you to specify a custom build path for the Drive ABCI Docker image. If `enabled` is set to `true`, Dashmate will build the Docker image from the specified path.

## Logging

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.drive.abci.logs` | Drive logging configuration | Object | See below |

The `logs` object supports multiple named log configurations with detailed settings:

```json
{
  "logs": {
    "default": {
      "destination": "stdout",
      "level": "info",
      "format": "full",
      "color": true
    },
    "database": {
      "destination": "/var/log/drive/db.log",
      "level": "error",
      "format": "json",
      "color": null
    }
  }
}
```

Each log configuration has the following properties:
- `destination`: Where logs are sent - `stdout`, `stderr`, or an absolute file path
- `level`: Log level - `error`, `warn`, `info`, `debug`, `trace`, `silent`, or a RUST_LOG format string
- `format`: Log format - `full`, `compact`, `pretty`, or `json`
- `color`: Whether to use colored output (`true`, `false`, or `null` to autodetect)

You can define multiple named logging configurations to route different types of logs to different destinations.

## LLMQ Configuration

These settings control quorum validation requirements:

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.drive.abci.validatorSet.quorum` | Validator set quorum configuration | Object | See below |
| `platform.drive.abci.chainLock.quorum` | Chain lock quorum configuration | Object | See below |
| `platform.drive.abci.instantLock.quorum` | Instant lock quorum configuration | Object | See below |

Quorum configuration properties:

```json
{
  "quorum": {
    "llmqType": 105,
    "dkgInterval": 24,
    "activeSigners": 300,
    "rotation": true
  }
}
```

Each quorum has the following properties:
- `llmqType`: Quorum type ID number (integer)
- `dkgInterval`: Distributed key generation interval
- `activeSigners`: Number of active signers in the quorum
- `rotation`: Whether quorum rotation is enabled

Available quorum type IDs correspond to the following LLMQ types:
- 1-6: Testnet/local quorums
- 100-107: Mainnet quorums like llmq_50_60, llmq_60_75, llmq_400_60, llmq_400_85, llmq_100_67, etc.

##  Metrics

These settings control the metrics endpoint for monitoring Drive ABCI:

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.drive.abci.metrics.enabled` | Enable metrics | `false` | `true` |
| `platform.drive.abci.metrics.host` | Host binding for metrics | `127.0.0.1` | `0.0.0.0` |
| `platform.drive.abci.metrics.port` | Port for metrics | `29090` | `29091` |

Metrics provide performance and health information about the Drive ABCI service. When enabled, the metrics server will be accessible at the specified host and port.

## Debug Tools

These settings control developer and debugging tools:

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.drive.abci.tokioConsole.enabled` | Enable Tokio debugging console | `false` | `true` |
| `platform.drive.abci.tokioConsole.port` | Tokio console port | `6669` | `6670` |
| `platform.drive.abci.tokioConsole.retention` | Tokio console data retention (seconds) | `180` | `300` |
| `platform.drive.abci.grovedbVisualizer.enabled` | Enable GroveDB visualization tool | `false` | `true` |
| `platform.drive.abci.grovedbVisualizer.port` | GroveDB visualization port | `8083` | `8084` |

- Tokio Console: A debugging tool for Rust's async runtime
- GroveDB Visualizer: A visualization tool for the GroveDB database structure

## Other options

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.drive.abci.proposer.txProcessingTimeLimit` | Transaction processing time limit | `null` | `"5s"` |
| `platform.drive.abci.epochTime` | Epoch time in seconds | `788400` | `1576800` |

- Transaction processing time limit: Maximum time allowed for processing a transaction before timeout
- Epoch time: Length of a single epoch in seconds

