# group status

The `group status` command displays an overview of all nodes in a group.

## Usage

```bash
dashmate group status [OPTIONS]
```

## Options

| Option | Description | Default |
|--------|-------------|--------|
| `-g, --group=<name>` | Group to check status for | *Uses default group if not specified* |
| `--format=<format>` | Display output format (`plain`, `json`, `yaml`) | `plain` |

## Description

This command provides a summary status overview for all nodes in a specified group.

For each node in the group, it displays information such as:
- Network (mainnet, testnet, local)
- Core status and block height
- Platform enabled status
- Platform status, version, and block height
- Platform peers and network

The status information is color-coded for easier interpretation:
- Running services are shown in green
- Syncing services are shown in yellow
- Stopped or error services are shown in red

This command is useful for quickly checking the status of all nodes in a local development network or other node groups.

## Examples

```bash
# Show status for all nodes in the default group
dashmate group status

# Show status for a specific group
dashmate group status --group=local

# Show status in JSON format
dashmate group status --format=json
```

Example output:
```
Node local_seed
Network: local
Core Status: running
Core Height: 150
Platform Enabled: true
Platform Status: running
Platform Version: 0.23.0
Platform Block Height: 100
Platform Peers: 3
Platform Network: dash-local

Node local_node_1
Network: local
Core Status: running
Core Height: 150
Platform Enabled: true
Platform Status: running
Platform Version: 0.23.0
Platform Block Height: 100
Platform Peers: 3
Platform Network: dash-local
```

## Related Commands

- [status](../status/index.md) - Show status for a single node
- [group list](./list.md) - List available groups
