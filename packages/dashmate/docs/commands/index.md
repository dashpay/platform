# Dashmate Commands

This documentation covers all available Dashmate CLI commands, their parameters, and behavior.

## Command Categories

Dashmate commands are organized into the following categories:

- [Config](./config/index.md) - Manage Dashmate configurations
- [Core](./core/index.md) - Interact with Dash Core functionality
- [Docker](./docker/index.md) - Manage Docker containers and builds
- [Doctor](./doctor/index.md) - Diagnose and resolve issues
- [Group](./group/index.md) - Manage groups of nodes
- [SSL](./ssl/index.md) - Configure and manage SSL certificates
- [Status](./status/index.md) - Check status of various components
- [Wallet](./wallet/index.md) - Manage Dash wallets and funds

## Top-Level Commands

The following commands are available at the top level:

- [setup](./setup.md) - Set up a new Dash node
- [start](./start.md) - Start a node
- [stop](./stop.md) - Stop a running node
- [restart](./restart.md) - Restart a node
- [reset](./reset.md) - Reset a node to its initial state
- [update](./update.md) - Update the node and its dependencies

## Command Usage

All commands follow the pattern:

```bash
dashmate [command] [subcommand] [options]
```

You can get help for any command by adding `--help`:

```bash
dashmate config --help
dashmate status core --help
```