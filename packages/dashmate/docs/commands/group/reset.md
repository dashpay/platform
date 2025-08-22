# group reset

The `group reset` command resets all nodes in a group to their initial state.

## Usage

```bash
dashmate group reset [OPTIONS]
```

## Options

| Option | Description | Default |
|--------|-------------|--------|
| `-g, --group=<name>` | Group to reset | *Uses default group if not specified* |
| `--hard` | Reset configuration as well as data | `false` |
| `-f, --force` | Reset even running nodes without confirmation | `false` |
| `-p, --platform` | Reset only platform services and data | `false` |
| `-v, --verbose` | Use verbose mode for output | `false` |

## Description

This command resets all nodes in a specified group to their initial state. 
By default, it removes service containers and data while preserving the configuration.

For the `local` group (used for local development networks), the command will also reconfigure Core and Tenderdash nodes after resetting them, unless the `--hard` flag is used.

Various flags can modify the behavior:
- Use `--hard` to reset both configuration and data
- Use `--platform` to reset only platform-related components
- Use `--force` to skip confirmation prompts and service checks

This command is particularly useful when:
- You want to start fresh with a clean group of nodes
- You're experiencing issues that might be resolved by removing data
- You've made configuration changes that require a complete reset

## Examples

```bash
# Reset data for all nodes in the default group
dashmate group reset

# Reset a specific group's data
dashmate group reset --group=local

# Hard reset - removes both configuration and data
dashmate group reset --hard

# Reset only platform services and data
dashmate group reset --platform

# Force reset without confirmation
dashmate group reset --force
```

## Related Commands

- [group start](./start.md) - Start all nodes in a group
- [group stop](./stop.md) - Stop all nodes in a group
- [reset](../reset.md) - Reset a single node
