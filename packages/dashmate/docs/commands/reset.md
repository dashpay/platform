# reset

The `reset` command resets a node, optionally including its configuration and data.

## Usage

```bash
dashmate reset [OPTIONS]
```

## Options

| Option | Description | Default |
|--------|-------------|--------|
| `-c, --config=<name>` | Configuration to reset | *Uses default config if not specified* |
| `--hard` | Reset configuration as well as services and data | `false` |
| `-f, --force` | Skip running services check and confirmation prompt | `false` |
| `-p, --platform` | Reset only platform services and data | `false` |
| `--keep-data` | Keep data when resetting | `false` |
| `-v, --verbose` | Use verbose mode for output | `false` |

## Description

This command resets a Dash node to its initial state.
By default, it removes service containers and data while preserving the configuration.

Various flags can modify this behavior:
- Use `--hard` to reset both configuration and data. Running the [setup](./setup.md) command will be required afterward.
- Use `--keep-data` to preserve data but reset service containers
- Use `--platform` to reset only platform-related components
- Use `--force` to skip confirmation prompts and service checks

The reset command will normally ask for confirmation before proceeding, unless the `--force` flag is used.

This command is particularly useful when:
- You want to start fresh with a clean node
- You're experiencing issues that might be resolved by removing data
- You've made configuration changes that require a complete reset

## Examples

```bash
# Reset data for the default configuration (preserves configuration)
dashmate reset

# Reset a specific configuration's data
dashmate reset --config=testnet

# Hard reset - removes both configuration and data
dashmate reset --hard

# Reset only platform services and data
dashmate reset --platform

# Reset services but keep data
dashmate reset --keep-data

# Force reset without confirmation
dashmate reset --force
```

## Related Commands

- [restart](./restart.md) - Restart a node without resetting data
- [group reset](./group/reset.md) - Reset all nodes in a group
