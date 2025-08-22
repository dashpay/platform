# config set

The `config set` command modifies a configuration option.

## Usage

```bash
dashmate config set OPTION VALUE [--config=<name>]
```

## Arguments

| Argument | Description | Required | Default |
|----------|-------------|----------|--------|
| `OPTION` | Option path to set | Yes | |
| `VALUE` | New value for the option | Yes | |

## Options

| Option | Description | Default |
|--------|-------------|--------|
| `-c, --config=<name>` | Configuration to modify | *Uses default config if not specified* |

## Description

This command sets a configuration option to a specified value.
The option is identified by its path in the configuration object, using dot notation (e.g., `core.p2p.port`).

The command will automatically try to parse the provided value as JSON.
If parsing fails, it will use the raw string value.
This allows you to set values of different types (strings, numbers, booleans, arrays, objects) without additional formatting.

## Examples

```bash
# Set a string value
dashmate config set externalIp "192.168.1.100"

# Set a number value
dashmate config set core.p2p.port 20001

# Set a boolean value
dashmate config set core.masternode.enable true

# Set an array value
dashmate config set "core.p2p.seeds" '[{"host":"seed.dash.org","port":9999}]'

# Set a value in a specific configuration
dashmate config set core.p2p.port 20001 --config=testnet
```

## Related Commands

- [config get](./get.md) - Get the value of a configuration option
- [config list](./list.md) - List available configurations
