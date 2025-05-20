# status masternode

The `status masternode` command displays detailed information about a masternode's status.

## Usage

```bash
dashmate status masternode [OPTIONS]
```

## Options

| Option | Description | Default |
|--------|-------------|--------|
| `-c, --config=<name>` | Configuration to check | *Uses default config if not specified* |
| `--format=<format>` | Display output format (`plain`, `json`, `yaml`) | `plain` |

## Description

This command provides detailed information about a masternode's status.
It is only applicable for configurations where the masternode functionality is enabled (`core.masternode.enable` is set to `true`). 

The command displays information such as:

- Masternode state (READY, POSE_BANNED, etc.)
- Masternode synchronization status
- ProTx hash (Provider Transaction hash that identifies the masternode)
- PoSe penalty score (Proof of Service penalty)
- Last paid block and time
- Total and enabled masternode counts
- Total and enabled evonode counts
- Payment queue position and estimated next payment time

The information is color-coded for better readability:
- Ready state is shown in green
- Other states are shown in red
- Synchronizing states are shown in yellow

This command is useful for masternode operators to monitor the status of their masternodes, check payment information, and diagnose issues.

## Examples

```bash
# Show masternode status for the default configuration
dashmate status masternode

# Show masternode status for a specific configuration
dashmate status masternode --config=mainnet

# Show masternode status in JSON format
dashmate status masternode --format=json
```

Example output:
```
Masternode State: READY
ProTx Hash: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
PoSe Penalty: 0
Last paid block: 1234567
Last paid time: 2023-01-01 12:00:00
Total Masternodes: 5000
Enabled Masternodes: 4800
Total Evonodes: 1000
Enabled Evonodes: 950
Payment queue position: 456
Next payment time: in 3 days, 4 hours
```

## Related Commands

- [status](./status.md) - Show overall node status
- [status core](./core.md) - Show detailed Core status
- [status platform](./platform.md) - Show detailed Platform status
