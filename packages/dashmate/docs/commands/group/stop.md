# group stop

The `group stop` command shuts down all nodes in a group.

## Usage

```bash
dashmate group stop [OPTIONS]
```

## Options

| Option | Description | Default |
|--------|-------------|--------|
| `-g, --group=<name>` | Group to stop | *Uses default group if not specified* |
| `-f, --force` | Force stop even if any service is running or DKG is in progress | `false` |
| `-s, --safe` | Wait for DKG (Distributed Key Generation) to complete before stopping | `false` |
| `-v, --verbose` | Use verbose mode for output | `false` |

## Description

This command stops all nodes in a specified group.
It gracefully shuts down all services for each node in the group in a coordinated manner, with miner nodes being stopped first to prevent masternode banning.

The command handles the stop sequence appropriately to ensure that interdependent services are stopped in the correct order.

If the nodes are masternodes participating in DKG (Distributed Key Generation) session, stopping them can cause issues.
Use the `--safe` flag to wait for any current DKG round to complete before stopping the nodes, or use `--force` to ignore this protection (which may result in masternodes being temporarily banned from quorums).

## Examples

```bash
# Stop all nodes in the default group
dashmate group stop

# Stop all nodes in a specific group
dashmate group stop --group=local

# Safely stop all nodes, waiting for DKG to complete
dashmate group stop --safe

# Force stop all nodes regardless of conditions
dashmate group stop --force

# Stop all nodes with verbose output
dashmate group stop --verbose
```

## Related Commands

- [group start](./start.md) - Start all nodes in a group
- [group restart](./restart.md) - Restart all nodes in a group
- [stop](../stop.md) - Stop a single node
