# restart

The `restart` command stops and then starts a node, restarting all services.

## Usage

```bash
dashmate restart [OPTIONS]
```

## Options

| Option | Description | Default |
|--------|-------------|--------|
| `-c, --config=<name>` | Configuration to use | *Uses default config if not specified* |
| `-p, --platform` | Restart only platform services (not Core) | `false` |
| `-s, --safe` | Wait for DKG (Distributed Key Generation) to complete before restart | `false` |
| `-f, --force` | Ignore DKG and force restart (masternode might be banned) | `false` |
| `-v, --verbose` | Use verbose mode for output | `false` |

## Description

This command restarts a Dash node by stopping and then starting its services.

This can be useful when:
- You've changed configuration options that require a service restart
- Services are in an inconsistent state and need to be restarted
- You want to restart after an update

By default, the command restarts all services.
Use the `--platform` flag to restart only the platform services while keeping Core running.

If the node is a masternode participating in a quorum, restarting during a DKG (Distributed Key Generation) session can cause issues.
Use the `--safe` flag to wait for any current DKG round to complete before restarting, or use `--force` to ignore this protection (which may result in the masternode being banned and lost rewards).

## Examples

```bash
# Restart all services in the default configuration
dashmate restart

# Restart a specific configuration
dashmate restart --config=testnet

# Restart only platform services
dashmate restart --platform

# Safely restart, waiting for DKG to complete
dashmate restart --safe

# Force restart even during DKG
dashmate restart --force
```

## Related Commands

- [start](./start.md) - Start a node
- [stop](./stop.md) - Stop a node
- [reset](./reset.md) - Reset a node to its initial state
- [group restart](./group/restart.md) - Restart all nodes in a group
