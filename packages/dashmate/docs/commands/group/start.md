# group start

The `group start` command launches all nodes within a specified group.

## Usage

```bash
dashmate group start [OPTIONS]
```

## Options

| Option | Description | Default |
|--------|-------------|---------|
| `-g, --group=<name>` | Group name to use | *Uses default group if not specified* |
| `-w, --wait-for-readiness` | Wait for all nodes to be ready before returning | `false` |
| `-v, --verbose` | Use verbose mode for output | `false` |

## Description

This command starts all nodes in a specified group (or the default group if none is specified).
Groups are collections of nodes that are managed together, typically used for local development networks that include multiple masternodes.

The command starts all Docker containers for every node in the group in a coordinated manner.
This is particularly useful for local development networks, where multiple nodes need to be started together to form a functioning network.

When the `--wait-for-readiness` flag is set, the command will wait until all nodes in the group are fully initialized and ready to accept connections before completing.
This is useful in scripts or when you need to ensure the entire group is operational before proceeding with other operations.

## Examples

```bash
# Start nodes in the default group
dashmate group start

# Start nodes in a specific group
dashmate group start --group=local

# Start nodes and wait until they're all ready
dashmate group start --wait-for-readiness

# Start nodes with verbose output
dashmate group start --verbose
```

## Related Commands

- [group stop](./stop.md) - Stop all nodes in a group
- [group restart](./restart.md) - Restart all nodes in a group
- [group status](./status.md) - Show status for all nodes in a group
- [group list](./list.md) - List all node groups
