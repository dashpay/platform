# status platform

The `status platform` command displays detailed information about the Dash Platform components.

## Usage

```bash
dashmate status platform [OPTIONS]
```

## Options

| Option | Description | Default |
|--------|-------------|--------|
| `-c, --config=<name>` | Configuration to check | *Uses default config if not specified* |
| `--format=<format>` | Display output format (`plain`, `json`, `yaml`) | `plain` |

## Description

This command provides detailed information about the status of Dash Platform components.

It displays metrics and status information for Tenderdash, Drive, and related services, including:
- Platform activation status
- HTTP, P2P, and RPC service status
- Network ports and their states
- Docker container status for Tenderdash and Drive
- Service status for Tenderdash and Drive
- Network information
- Tenderdash version and protocol versions
- Block height and peer count
- Current application hash

The information is color-coded for better readability:
- Running services are shown in green
- Stopped services are shown in red
- Warning states are shown in yellow

This command is useful for diagnosing issues with Platform services, checking synchronization status, and monitoring the health of Platform components.

## Examples

```bash
# Show Platform status for the default configuration
dashmate status platform

# Show Platform status for a specific configuration
dashmate status platform --config=testnet

# Show Platform status in JSON format
dashmate status platform --format=json
```

Example output:
```
Platform Activation: enabled
HTTP service: envoy
HTTP port: 3000 open
P2P service: tenderdash
P2P port: 26656 open
RPC service: tenderdash
Tenderdash Docker Status: running
Tenderdash Service Status: running
Drive Docker Status: running
Drive Service Status: running
Network: dash-local
Tenderdash Version: 0.10.0
Protocol Version: v1
Desired Protocol Version: v1
Block height: 1000
Peer count: 3
App hash: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
```

## Related Commands

- [status](./status.md) - Show overall node status
- [status core](./core.md) - Show Core status
- [status services](./services.md) - Show all services status
