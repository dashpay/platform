# status core

The `status core` command displays detailed status information about the Dash Core node.

## Usage

```bash
dashmate status core [OPTIONS]
```

## Options

| Option | Description | Default |
|--------|-------------|--------|
| `-c, --config=<name>` | Configuration to check | *Uses default config if not specified* |
| `--format=<format>` | Display output format (`plain`, `json`, `yaml`) | `plain` |

## Description

This command provides detailed information about the status of your Dash Core node. It displays various metrics including:

- Network information (mainnet, testnet, devnet)
- Core version and latest available version
- Chain type
- Docker container status
- Service status
- Difficulty
- Synchronization status and progress
- Peer count
- P2P and RPC service status
- Block height, header height, and remote block height

The information is color-coded for better readability:
- Version: Green if up to date, yellow if outdated
- Block height: Green if synced, yellow if syncing
- P2P port: Green if open, yellow if filtered, red if closed

This command is useful for diagnosing issues with Core synchronization, network connectivity, and service health.

## Examples

```bash
# Show Core status for the default configuration
dashmate status core

# Show Core status for a specific configuration
dashmate status core --config=testnet

# Show Core status in JSON format
dashmate status core --format=json
```

Example output:
```
Network: testnet
Version: 19.2.0
Chain: test
Docker Status: running
Service Status: syncing
Difficulty: 4194304
Latest version: testnet
Sync asset: MASTERNODE_SYNC_FINISHED
Peer count: 8
P2P service: up
P2P port: open
RPC service: up
Block height: 638992
Header height: 638992
Verification Progress: 99.99%
Remote Block Height: 638992
```

## Related Commands

- [status](./status.md) - Show overall node status
- [status services](./services.md) - Show all services status
- [status masternode](./masternode.md) - Show masternode status