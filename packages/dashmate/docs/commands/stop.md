# stop

The `stop` command shuts down a running Dash node and its associated services.

## Usage

```bash
dashmate stop [OPTIONS]
```

## Options

| Option | Description | Default |
|--------|-------------|---------|
| `-c, --config=<name>` | Configuration name to use | *Uses default config if not specified* |
| `-f, --force` | Force stop even if any service is running or DKG is in progress | `false` |
| `-p, --platform` | Stop only platform services (not Core) | `false` |
| `-s, --safe` | Wait for DKG (Distributed Key Generation) to complete before stopping | `false` |
| `-v, --verbose` | Use verbose mode for output | `false` |

## Description

This command stops a running Dash node and its associated Docker containers. By default, it stops all services for the node.

If you only want to stop the Platform services while keeping Core running, use the `--platform` flag.

Normally, the command will stop services immediately. If a Distributed Key Generation (DKG) round is in progress, this could lead to node being banned.
Using the `--safe` flag ensures the command waits for any current DKG round to complete before stopping the node.

The `--force` flag can be used to forcibly stop services even if there's a DKG in progress or other conditions that would normally prevent shutdown.

## Examples

```bash
# Stop node with default configuration
dashmate stop

# Stop node with a specific configuration
dashmate stop --config=testnet

# Stop only platform services
dashmate stop --platform

# Safely stop a node, waiting for DKG to complete
dashmate stop --safe

# Force stop a node regardless of conditions
dashmate stop --force
```

## Related Commands

- [start](./start.md) - Start a node
- [restart](./restart.md) - Restart a node
- [status](./status/index.md) - Show node status
