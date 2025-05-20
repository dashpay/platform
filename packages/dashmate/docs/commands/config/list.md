# config list

The `config list` command displays all available Dashmate configurations.

## Usage

```bash
dashmate config list
```

## Description

This command lists all available configurations along with their descriptions.
It displays the information in a table format for easy reading.

The output includes:
- Configuration names
- Configuration descriptions (if set)

This command is useful for quickly seeing what configurations are available on your system and understanding their purpose through their descriptions.

## Examples

```bash
# List all available configurations
dashmate config list
```

Example output:
```
┌─────────┬─────────────────────────────────┐
│ mainnet │ Mainnet configuration           │
├─────────┼─────────────────────────────────┤
│ testnet │ Testnet configuration           │
├─────────┼─────────────────────────────────┤
│ local   │ Local development configuration │
└─────────┴─────────────────────────────────┘
```

## Related Commands

- [config default](./default.md) - Show or set the default configuration
- [config create](./create.md) - Create a new configuration
- [config remove](./remove.md) - Remove a configuration
