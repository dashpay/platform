# config default

The `config default` command manages the default configuration used by Dashmate.

## Usage

```bash
dashmate config default [CONFIG]
```

## Arguments

| Argument | Description | Required | Default |
|----------|-------------|----------|--------|
| `CONFIG` | Configuration name to set as default | No | *None* |

## Description

This command has two modes of operation:

1. When run without arguments, it displays the name of the current default configuration.
2. When run with a configuration name argument, it sets that configuration as the new default.

The default configuration is used by Dashmate commands when no specific configuration is specified with the `--config` flag.
Setting the appropriate default can simplify your workflow by eliminating the need to specify a configuration each time you run a command.

## Examples

```bash
# Show the current default configuration
dashmate config default

# Set 'mainnet' as the default configuration
dashmate config default mainnet

# Set 'my-custom-config' as the default configuration
dashmate config default my-custom-config
```

## Related Commands

- [config list](./list.md) - List available configurations
- [config create](./create.md) - Create a new configuration
