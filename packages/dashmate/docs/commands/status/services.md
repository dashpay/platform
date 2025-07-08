# status services

The `status services` command displays the status of all node services and their containers.

## Usage

```bash
dashmate status services [OPTIONS]
```

## Options

| Option | Description | Default |
|--------|-------------|--------|
| `-c, --config=<name>` | Configuration to check | *Uses default config if not specified* |
| `--format=<format>` | Display output format (`plain`, `json`, `yaml`) | `plain` |

## Description

This command provides a comprehensive view of all Docker containers and services running as part of your Dash node.

For each service, it displays:
- Service name and title
- Container ID (abbreviated to 12 characters in plain format)
- Docker image being used
- Current status (running or stopped)

The status information is color-coded for easier interpretation:
- Running services are shown in green
- Stopped services are shown in red

This command is useful for getting a quick overview of which services are running and which may be experiencing issues.
It can help diagnose configuration problems, Docker-related issues, or service failures.

## Examples

```bash
# Show services status for the default configuration
dashmate status services

# Show services status for a specific configuration
dashmate status services --config=testnet

# Show services status in JSON format
dashmate status services --format=json
```

Example output:
```
┌───────────────┬──────────────┬─────────────────────────┬─────────┐
│ Service       │ Container ID │ Image                    │ Status   │
├───────────────┼──────────────┼─────────────────────────┼─────────┤
│ Core          │ a1b2c3d4e5f6 │ dashpay/dashd:19.2.0     │ running  │
│ Drive ABCI    │ b2c3d4e5f6a1 │ dashpay/drive:1.0.0      │ running  │
│ Tenderdash    │ c3d4e5f6a1b2 │ dashpay/tenderdash:0.10.0 │ running  │
│ DAPI API      │ d4e5f6a1b2c3 │ dashpay/dapi:1.0.0       │ running  │
│ Gateway       │ e5f6a1b2c3d4 │ dashpay/gateway:1.0.0    │ running  │
│ Rate Limiter  │ f6a1b2c3d4e5 │ dashpay/ratelimiter:1.0.0 │ running  │
└───────────────┴──────────────┴─────────────────────────┴─────────┘
```

## Related Commands

- [status](./status.md) - Show overall node status
- [status core](./core.md) - Show detailed Core status
- [status platform](./platform.md) - Show detailed Platform status
