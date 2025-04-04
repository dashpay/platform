# update

The `update` command updates the node's Docker images to their latest versions.

## Usage

```bash
dashmate update [OPTIONS]
```

## Options

| Option | Description | Default |
|--------|-------------|--------|
| `-c, --config=<name>` | Configuration to update | *Uses default config if not specified* |
| `--format=<format>` | Display output format (`plain`, `json`, `yaml`) | `plain` |

## Description

This command updates the Docker images used by a Dash node to their latest versions.
It pulls the latest images specified in the configuration and reports on which services were updated.

The command displays a table showing:
- Service names
- Docker image identifiers
- Update status (updated, up to date, or error)

Services that were updated are highlighted in yellow, services that were already up to date are shown in green, and any errors are displayed in red.

This command only updates the Docker images; it does not automatically restart services to use the new images.
After updating, you typically need to restart the node with the `restart` command to apply the updates.

## Examples

```bash
# Update the default configuration
dashmate update

# Update a specific configuration
dashmate update --config=testnet

# Update and display results in JSON format
dashmate update --format=json
```

Example output:
```
┌─────────────────┬──────────────────────────────┬────────────┐
│ Service         │ Image                        │ Updated    │
├─────────────────┼──────────────────────────────┼────────────┤
│ Core            │ dashpay/dashd:19.2.0        │ up to date │
│ Drive ABCI      │ dashpay/drive:1.0.0         │ updated    │
│ Tenderdash      │ dashpay/tenderdash:0.10.0   │ updated    │
│ DAPI API        │ dashpay/dapi:1.0.0          │ up to date │
│ Envoy Gateway   │ dashpay/platform-gateway:1.0 │ up to date │
└─────────────────┴──────────────────────────────┴────────────┘
```

## Related Commands

- [restart](./restart.md) - Restart node to apply updates
- [docker build](./docker/build.md) - Build custom Docker images
