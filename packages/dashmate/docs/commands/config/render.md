# config render

The `config render` command generates service configuration files from templates.

## Usage

```bash
dashmate config render [--config=<name>] [--format=<format>]
```

## Options

| Option | Description | Default |
|--------|-------------|--------|
| `-c, --config=<name>` | Configuration to render | *Uses default config if not specified* |
| `--format=<format>` | Display output format (`plain`, `json`, `yaml`) | `plain` |

## Description

This command renders service configuration files from templates based on the current Dashmate configuration values.

These service configuration files are used by various components of the Dash platform, such as:
- Core configuration (`dash.conf`)
- Tenderdash configuration (`config.toml`, `genesis.json`, etc.)
- Drive configuration
- Gateway configuration (Envoy proxy configurations)
- And other service-specific configurations

The rendered files are written to the Dashmate home directory, typically in a subdirectory named after the configuration.

This command is primarily used to regenerate service configuration files after changing Dashmate configuration settings, ensuring that all services use the updated configuration values when they start.

## Examples

```bash
# Render service configs for the default configuration
dashmate config render

# Render service configs for a specific configuration
dashmate config render --config=testnet
```

## Related Commands

- [config set](./set.md) - Set a configuration option
- [config get](./get.md) - Get the value of a configuration option
- [start](../start.md) - Start a node (automatically renders configs)
