# config get

The `config get` command retrieves the value of a configuration option.

## Usage

```bash
dashmate config get OPTION [--config=<name>] [--format=<format>]
```

## Arguments

| Argument | Description | Required | Default |
|----------|-------------|----------|--------|
| `OPTION` | Option path to retrieve | Yes | |

## Options

| Option | Description | Default |
|--------|-------------|--------|
| `-c, --config=<name>` | Configuration to query | *Uses default config if not specified* |
| `--format=<format>` | Output format (`plain`, `json`, `yaml`) | `plain` |

## Description

This command retrieves and displays the value of a specified configuration option.
The option is identified by its path in the configuration object, using dot notation (e.g., `core.p2p.port`).

The command can display output in different formats:
- `plain`: Default format, displays simple values as-is and complex objects in a readable format
- `json`: Formats the output as a JSON string
- `yaml`: Formats the output as YAML

For complex objects and arrays, the output will be formatted for better readability.

## Examples

```bash
# Get a simple value
dashmate config get core.p2p.port

# Get a complex object
dashmate config get core.p2p

# Get a value from a specific configuration
dashmate config get core.p2p.port --config=testnet

# Get a value in JSON format
dashmate config get platform.drive --format=json
```

## Related Commands

- [config set](./set.md) - Set a configuration option
- [config list](./list.md) - List available configurations
