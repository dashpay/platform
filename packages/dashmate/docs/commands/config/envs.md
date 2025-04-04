# config envs

The `config envs` command exports configuration options as environment variables.

## Usage

```bash
dashmate config envs [--config=<name>] [--output-file=<path>]
```

## Options

| Option | Description | Default |
|--------|-------------|--------|
| `-c, --config=<name>` | Configuration to export | *Uses default config if not specified* |
| `-o, --output-file=<path>` | Save output to a file instead of stdout | |

## Description

This command converts a Dashmate configuration into environment variables that can be used with Docker Compose.
It flattens the configuration structure into key-value pairs suitable for environment variable usage.

The exported environment variables follow a naming convention that makes them compatible with Docker Compose and other systems that use environment variables for configuration.

By default, the command outputs the environment variables to the console (stdout).
You can use the `--output-file` option to save them to a file instead, which is useful for creating `.env` files for Docker Compose or other applications.

## Examples

```bash
# Export default configuration to stdout
dashmate config envs

# Export a specific configuration to stdout
dashmate config envs --config=testnet

# Export configuration to a file
dashmate config envs --output-file=.env

# Export specific configuration to a file
dashmate config envs --config=mainnet --output-file=mainnet.env
```

Example output:
```
EXTERNAL_IP=127.0.0.1
CORE_P2P_PORT=19999
CORE_P2P_HOST=0.0.0.0
CORE_RPC_PORT=19998
CORE_RPC_HOST=0.0.0.0
...
```

## Related Commands

- [config get](./get.md) - Get the value of a configuration option
- [config set](./set.md) - Set a configuration option
