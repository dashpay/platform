# Dashmate Configuration Guide

This document provides information about Dashmate configuration system.
Dashmate supports multiple configs and uses a hierarchical configuration system with various sections that control different aspects of a Dash Platform node.

## Overview

Dashmate configuration is organized into a hierarchical structure with the following main sections:

- **core**: Options for Dash Core node
- **platform**: Options for Dash Platform components
- **docker**: Docker-related configuration
- **dashmate**: Dashmate-specific configuration
- **externalIp**: External IP address for the node
- **network**: Network selection (mainnet, testnet, etc.)
- **environment**: Environment type (production, development)

## Components

- [Core](./core.md) - Dash Core node settings
- [Gateway](./gateway.md) - Platform Gateway settings
- [Drive ABCI](./drive-abci.md) - Drive ABCI application logic
- [Tenderdash](./tenderdash.md) - Consensus engine settings
- [DAPI](./dapi.md) - Platform API services
- [Dashmate Helper](./dashmate.md) - Dashmate helper service
- [Docker](./docker.md) - Docker configuration
- [Miscellaneous configuration](./misc.md) - Network, Config and Environment settings

## Setting Up a Node

To setup a new node use the `dashmate setup` command.
This interactive command will guide you through the process of creating a new configuration.

```bash
dashmate setup
```

## Configuration Basics

### Configuration Presets

Dashmate comes with predefined configuration presets for different environments:

- **mainnet**: For production nodes on the main Dash network
- **testnet**: For testing on the Dash testnet
- **local**: For local development with all services

You can create as many custom configurations as you need based on these presets or existing configs.

### Config Commands

Dashmate provides several commands to manage configurations:

```bash
# Display current default config
dashmate config

# List all available configs
dashmate config list

# Create a new config
dashmate config create <n> [--preset=<preset>]

# Set a config as default
dashmate config default <n>

# Get a specific config option
dashmate config get <option>

# Set a specific config option
dashmate config set <option> <value>

# Remove a config
dashmate config remove <n>

# Export config as environment variables
dashmate config envs [--output-file]

# Render service configurations
dashmate config render
```

When running dashmate commands, you can specify, which config to use:

```bash
dashmate start --config=<preset>
```

If no config is specified, the default config will be used.

### Config Files Location

Configuration files are stored in the Dashmate home directory:

- Default location: `~/.dashmate/config.json`
- Can be changed with the `DASHMATE_HOME_DIR` environment variable

## Configuration Migration

When updating Dashmate, configurations are automatically migrated to the new format.

## Troubleshooting

### Dashmate doctor

Use the `dashmate doctor` command to check for common issues in your configuration:

```bash
dashmate doctor
```

### Common Configuration Issues

- **Port conflicts**: If a port is already in use, try changing the port in the configuration.
- **Networking issues**: Check if the `externalIp` is correctly set and accessible.
- **Docker permission issues**: Make sure your user has permissions to access the Docker socket.

### Debugging Configuration

```bash
# Check the current configuration
dashmate config

# Check a specific option
dashmate config get <option>

# Enable debug logging
dashmate config set core.log.debug.enabled true
```
