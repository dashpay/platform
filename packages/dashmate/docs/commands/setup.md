# setup

The `setup` command initializes a new Dash node with the specified configuration preset.

## Usage

```bash
dashmate setup [PRESET] [OPTIONS]
```

## Arguments

| Argument | Description | Required | Default |
|----------|-------------|----------|---------|
| `preset` | Node configuration preset (`mainnet`, `testnet`, `local`) | No | *Prompts if not provided* |

## Options

| Option | Description | Default |
|--------|-------------|---------|
| `-d, --debug-logs` | Enable debug logs | `false` |
| `--no-debug-logs` | Disable debug logs | |
| `-c, --node-count=<number>` | Number of nodes to setup (for `local` preset, minimum 3) | |
| `-m, --miner-interval=<interval>` | Interval between blocks (for `local` preset) | |
| `-v, --verbose` | Use verbose mode for output | `false` |

## Description

The `setup` command configures a new Dash node with one of the following presets:

- **mainnet**: Run a node connected to the Dash main network
- **testnet**: Run a node connected to the Dash test network
- **local**: Run a full network environment on your machine for local development

If no preset is specified, the command will prompt you to select one.

For the `local` preset, you can specify how many nodes to create with the `--node-count` option. At least 3 nodes are required for proper operation.

The command prevents you from overwriting an existing configuration.
If a configuration for the specified preset already exists, the command will suggest using `dashmate reset` or `dashmate group reset` to start from scratch.

## Examples

```bash
# Set up a mainnet node
dashmate setup mainnet

# Set up a testnet node with debug logs enabled
dashmate setup testnet --debug-logs

# Set up a local development environment with 3 nodes
dashmate setup local --node-count=3

# Set up a local environment with custom block mining interval
dashmate setup local --miner-interval=1m
```

## Related Commands

- [reset](./reset.md) - Reset a node to its initial state
- [config create](./config/create.md) - Create a new configuration
- [group reset](./group/reset.md) - Reset all nodes in a group
