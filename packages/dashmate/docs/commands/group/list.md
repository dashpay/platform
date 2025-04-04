# group list

The `group list` command shows all available configuration groups.

## Usage

```bash
dashmate group list [OPTIONS]
```

## Options

| Option | Description | Default |
|--------|-------------|--------|
| `-g, --group=<name>` | Group to list | *Lists all groups if not specified* |

## Description

This command displays all available configuration groups in your Dashmate installation.
Groups are collections of nodes that are managed together, typically used for local development networks that include multiple nodes.

The command prints a table with each group's name and description.
This information helps you identify the purpose of each group and select the appropriate one for subsequent commands.

## Examples

```bash
# List all available groups
dashmate group list
```

Example output:
```
┌──────┬─────────────────────────────────────────────────────────┐
│ local │ Local development network with 3 masternodes and 1 seed node │
└──────┴─────────────────────────────────────────────────────────┘
```

## Related Commands

- [group start](./start.md) - Start all nodes in a group
- [group stop](./stop.md) - Stop all nodes in a group
- [group status](./status.md) - Show status for all nodes in a group
