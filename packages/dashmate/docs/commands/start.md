# start

The `start` command launches a Dash node and its associated services.

## Usage

```bash
dashmate start [OPTIONS]
```

## Options

| Option | Description | Default |
|--------|-------------|---------|
| `-c, --config=<name>` | Configuration name to use | *Uses default config if not specified* |
| `-w, --wait-for-readiness` | Wait for nodes to be ready before returning | `false` |
| `-p, --platform` | Start only platform services (not Core) | `false` |
| `-f, --force` | Force start even if any services are already running | `false` |
| `-v, --verbose` | Use verbose mode for output | `false` |

## Description

This command starts a Dash node using the specified configuration (or the default configuration if none is specified).
It will start all the Docker containers required for the node to function.

By default, the command starts both Core and Platform services.
If you only need to run the Platform services, use the `--platform` flag.

The `--wait-for-readiness` flag is useful in scripts or when you need to ensure the node is fully operational before proceeding.
When this flag is set, the command will wait until all services are fully initialized and ready to accept connections.

If any services are already running, the command will normally fail. Use the `--force` flag to stop and restart those services.

## Examples

```bash
# Start node with default configuration
dashmate start

# Start node with a specific configuration
dashmate start --config=testnet

# Start only platform services for a specific configuration
dashmate start --config=mainnet --platform

# Start node and wait until it's ready
dashmate start --wait-for-readiness

# Force restart of a running node
dashmate start --force
```

## Related Commands

- [stop](./stop.md) - Stop a running node
- [restart](./restart.md) - Restart a node
- [status](./status/index.md) - Show node status
