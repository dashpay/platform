# group default

The `group default` command manages the default group used by Dashmate.

## Usage

```bash
dashmate group default [GROUP]
```

## Arguments

| Argument | Description | Required | Default |
|----------|-------------|----------|--------|
| `GROUP` | Group name to set as default | No | *None* |

## Description

This command has two modes of operation:
1. When run without arguments, it displays the name of the current default group.
2. When run with a group name argument, it sets that group as the new default.

The default group is used by Dashmate when no specific group is specified with the `--group` flag in group-related commands.
Setting the appropriate default can simplify your workflow by eliminating the need to specify a group each time you run a group command.

## Examples

```bash
# Show the current default group
dashmate group default

# Set 'local' as the default group
dashmate group default local

# Set 'my-custom-group' as the default group
dashmate group default my-custom-group
```

## Related Commands

- [group list](./list.md) - List available groups
- [config default](../config/default.md) - Show or set the default configuration
