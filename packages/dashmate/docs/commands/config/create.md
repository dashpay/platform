# config create

The `config create` command creates a new configuration based on an existing one.

## Usage

```bash
dashmate config create CONFIG [FROM]
```

## Arguments

| Argument | Description | Required | Default |
|----------|-------------|----------|---------|
| `CONFIG` | Name for the new configuration | Yes | |
| `FROM` | Base the new config on this existing config | No | `base` |

## Description

This command creates a new configuration with the specified name, based on an existing configuration.
If the `FROM` argument is not provided, the new configuration will be based on the `base` configuration, which contains minimal default settings.

Creating a new configuration is useful when you want to:

1. Create variants of existing configurations (e.g., different ports or settings)
2. Create specialized configurations for specific use cases
3. Back up a configuration before making changes

The command will fail if a configuration with the same name already exists.

## Examples

```bash
# Create a new configuration named 'my-mainnet' based on the default 'base' config
dashmate config create my-mainnet

# Create a new configuration based on the 'testnet' config
dashmate config create my-testnet testnet

# Create a customized local config based on the existing local config
dashmate config create local-custom local
```

## Related Commands

- [config list](./list.md) - List available configurations
- [config set](./set.md) - Set configuration options
- [config remove](./remove.md) - Remove a configuration
