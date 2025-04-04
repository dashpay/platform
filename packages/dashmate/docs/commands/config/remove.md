# config remove

The `config remove` command deletes a Dashmate configuration.

## Usage

```bash
dashmate config remove CONFIG
```

## Arguments

| Argument | Description | Required | Default |
|----------|-------------|----------|--------|
| `CONFIG` | Name of configuration to remove | Yes | |

## Description

This command removes a specified configuration from Dashmate.

It will:
1. Delete the configuration entry from the configuration file
2. Delete any service configuration files associated with this configuration

System configurations (`mainnet`, `testnet`, `local`) cannot be removed using this command.
If you attempt to remove a system configuration, the command will suggest using `dashmate reset --hard` instead.

## Examples

```bash
# Remove a custom configuration
dashmate config remove my-custom-config
```

## Related Commands

- [config list](./list.md) - List available configurations
- [config create](./create.md) - Create a new configuration
- [reset](../reset.md) - Reset a configuration to its initial state
