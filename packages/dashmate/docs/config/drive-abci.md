# Drive ABCI Configuration

Drive ABCI contains the application logic for Dash Platform. Its configuration is divided into several functional areas.

## Drive ABCI Docker Configuration

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.drive.abci.docker.image` | Docker image for Drive ABCI | `dashpay/drive:${version}` | `dashpay/drive:latest` |
| `platform.drive.abci.docker.build` | Build settings for Drive ABCI | Object | See below |

The `docker.build` object allows for custom build settings:
```json
"platform.drive.abci.docker.build": {
  "enabled": true,
  "path": "/path/to/drive/source"
}
```

It allows you to specify a custom build path for the Drive ABCI Docker image. If `enabled` is set to `true`, Dashmate will build the Docker image from the specified path.

## Drive ABCI Logging Configuration

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.drive.abci.logs` | Drive logging configuration | Object | See below |

The logs object includes logging settings:
```json
"platform.drive.abci.logs": {
  "level": "info",
  "stdout": true,
  "colorize": true
}
```

Available log levels: `trace`, `debug`, `info`, `warn`, `error`

## Drive ABCI Debug Tool Configuration

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

## Drive ABCI Quorum Configuration

These settings control quorum validation requirements:

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.drive.abci.validatorSet.quorum` | Validator set quorum configuration | Object | See below |
| `platform.drive.abci.chainLock.quorum` | Chain lock quorum configuration | Object | See below |
| `platform.drive.abci.instantLock.quorum` | Instant lock quorum configuration | Object | See below |

Quorum configuration example:
```json
"platform.drive.abci.validatorSet.quorum": {
  "llmqType": "llmq_100_67",
  "requestTimeout": "2s"
}
```

## Drive ABCI Performance Configuration

These settings control performance aspects:

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.drive.abci.proposer.txProcessingTimeLimit` | Transaction processing time limit | `null` | `"5s"` |
| `platform.drive.abci.epochTime` | Epoch time in seconds | `788400` | `1576800` |

- Transaction processing time limit: Maximum time allowed for processing a transaction before timeout
- Epoch time: Length of a single epoch in seconds (affects validator set rotation frequency)

## Drive ABCI Metrics Configuration

These settings control the metrics endpoint for monitoring Drive ABCI:

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.drive.abci.metrics.enabled` | Enable metrics | `false` | `true` |
| `platform.drive.abci.metrics.host` | Host binding for metrics | `127.0.0.1` | `0.0.0.0` |
| `platform.drive.abci.metrics.port` | Port for metrics | `29090` | `29091` |

Metrics provide performance and health information about the Drive ABCI service.
