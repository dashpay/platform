# Dashmate Usage Guide

This guide covers the basic usage of Dashmate, including running nodes and common operations.

## Command Line Interface

The Dashmate CLI can be used to perform routine tasks. Invoke the CLI with `dashmate`. To list available commands, either run `dashmate` with no parameters or execute `dashmate help`. To list the help on any command, execute the command followed by the `--help` option.

For detailed documentation on each command, see the [Commands](./commands/index.md) section.

## Common Operations

### Node Setup

The `setup` command is used to quickly configure common node configurations. See [Node Setup](./commands/setup.md) for details.

Supported presets:
 * `mainnet` - a node connected to the Dash main network
 * `testnet` - a node connected to the Dash test network
 * `local` - a full network environment on your machine for local development

Example:
```bash
$ dashmate setup testnet
```

### Node Configuration

The `config` command is used to manage your node configuration. See [Configuration Commands](./commands/config/index.md) for details.

### Starting a Node

The `start` command starts a node with the default or specified config. See [Start Node](./commands/start.md) for details.

Example:
```bash
$ dashmate start
```

### Stopping a Node

The `stop` command stops a running node. See [Stop Node](./commands/stop.md) for details.

Example:
```bash
$ dashmate stop
```

### Restarting a Node

The `restart` command restarts a node. See [Restart Node](./commands/restart.md) for details.

Example:
```bash
$ dashmate restart
```

### Checking Node Status

The `status` command outputs status information about the node. See [Status Commands](./commands/status/index.md) for details.

Example:
```bash
$ dashmate status
```

### Using Core CLI

The `core cli` command executes a `dash-cli` command to the Core container. See [Core CLI](./commands/core/cli.md) for details.

Example:
```bash
$ dashmate core cli "getblockcount"
1337
```

### Resetting a Node

The `reset` command removes all data for a node. See [Reset Command](./commands/reset.md) for details.

Example:
```bash
$ dashmate reset
```

## Node Types

### Full Node

To run a full node instead of a masternode, modify the config setting:
```bash
$ dashmate config set core.masternode.enable false
```

### Node Groups

For managing multiple nodes together, especially for local development, see [Node Groups](./commands/group/index.md).

## Docker Compose Usage

If you want to use Docker Compose directly with Dashmate configurations, you can:

1. Output a config to a dotenv file:
```bash
$ dashmate config envs --config=testnet --output-file .env.testnet
```

2. Use the dotenv file with Docker Compose:
```bash
$ docker compose --env-file=.env.testnet up -d
```

## Development Environments

To set up a local development environment:

1. Use the `setup` command with the `local` preset to generate configs and set up a local network
2. For testing changes to DAPI and Drive, specify a local path using the `platform.sourcePath` config option

## Troubleshooting

For common issues and solutions, see the [Troubleshooting Guide](./troubleshooting.md).