# status

The `status` command shows an overview of your Dash node's current state.

## Usage

```bash
dashmate status [OPTIONS]
```

## Options

| Option | Description | Default |
|--------|-------------|---------|
| `-c, --config=<name>` | Configuration name to use | *Uses default config if not specified* |
| `--format=<format>` | Display output format (`plain`, `json`, `yaml`) | `plain` |
| `-v, --verbose` | Use verbose mode for output | `false` |

## Description

This command provides a comprehensive overview of your Dash node's status, including both Core and Platform components.

It displays information such as:
- Network (mainnet, testnet, local)
- Core version and status
- Core blockchain height and sync progress
- Core service status
- Blockchain data size
- Masternode status (if enabled)
- Masternode payment information
- Platform status (if enabled)
- Platform block height and peers

The information displayed is useful for quickly checking if your node is operating correctly and for troubleshooting issues.

You can choose to output the status in different formats using the `--format` option, which is particularly useful when integrating with scripts or other tools.

## Examples

```bash
# Show status for default configuration
dashmate status

# Show status for a specific configuration
dashmate status --config=testnet

# Output status in JSON format
dashmate status --format=json

# Output status in YAML format
dashmate status --format=yaml
```

## Related Commands

- [status core](./core.md) - Show detailed Dash Core status
- [status platform](./platform.md) - Show detailed Platform status
- [status masternode](./masternode.md) - Show detailed masternode status
- [status services](./services.md) - Show all services status
