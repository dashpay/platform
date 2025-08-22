# group restart

The `group restart` command stops and then starts all nodes in a group.

## Usage

```bash
dashmate group restart [OPTIONS]
```

## Options

| Option | Description | Default |
|--------|-------------|--------|
| `-g, --group=<name>` | Group to restart | *Uses default group if not specified* |
| `-s, --safe` | Wait for DKG (Distributed Key Generation) to complete before stopping | `false` |
| `-v, --verbose` | Use verbose mode for output | `false` |

## Description

This command restarts all nodes in a specified group by first stopping and then starting them.
It handles the restart sequence in a coordinated manner, stopping miner nodes first (if present) to prevent masternode banning.

The restart process happens in two phases:
1. Stopping all nodes in the group in the proper sequence
2. Starting all nodes in the group in the proper sequence

If the nodes are masternodes participating in quorums, restarting them during a DKG (Distributed Key Generation) session can cause issues.
Use the `--safe` flag to wait for any current DKG round to complete before stopping the nodes.

This command is useful when:
- You've changed configuration options that require a service restart
- Services are in an inconsistent state and need to be refreshed
- You want to restart after an update

## Examples

```bash
# Restart all nodes in the default group
dashmate group restart

# Restart all nodes in a specific group
dashmate group restart --group=local

# Safely restart, waiting for DKG to complete
dashmate group restart --safe

# Restart with verbose output
dashmate group restart --verbose
```

## Related Commands

- [group start](./start.md) - Start all nodes in a group
- [group stop](./stop.md) - Stop all nodes in a group
- [restart](../restart.md) - Restart a single node
