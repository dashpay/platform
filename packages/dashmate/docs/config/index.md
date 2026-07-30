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

# Get what is stored, rather than the value in effect
dashmate config get --raw <option>

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

## Options with dynamic defaults

A few options are left unset, and their value is derived from the dashmate build
you are running rather than written into your config. Today these are the Drive
and rs-dapi images:

- `platform.drive.abci.docker.image`
- `platform.dapi.rsDapi.docker.image`

Unset means "use the image published for this dashmate version". Upgrading
dashmate moves them automatically — there is nothing pinned in your config to go
stale, and no config change to make.

Reads show the value that will actually be used, and `dashmate config` marks the
ones you have not set:

```
$ dashmate config get platform.drive.abci.docker.image
dashpay/drive:4-rc

$ dashmate config
    image: 'dashpay/drive:4-rc' (default),
```

`--raw` shows what is stored instead, which is how you tell the two apart:

```
$ dashmate config get --raw platform.drive.abci.docker.image
null
```

Setting one pins it, and dashmate will not change it again — including across
upgrades. That is deliberate: an image you chose is yours to manage.

```
$ dashmate config set platform.drive.abci.docker.image registry.example.com/drive:patched
$ dashmate config get --raw platform.drive.abci.docker.image
registry.example.com/drive:patched
```

Set it back to `null` to return to tracking the published image:

```
$ dashmate config set platform.drive.abci.docker.image null
```

Note `dashmate config --format=json` prints effective values without the
`(default)` marker, so it stays machine-readable.

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

## Running Dashmate commands concurrently

Dashmate keeps all configuration in a single `config.json` inside its home directory
(`~/.dashmate` by default).

Commands that change a configuration option — `dashmate config set` and friends — read,
change and save that file as one locked step. Two of them running at once cannot lose each
other's work: if one sets a Core RPC port while another pins a Drive image, both settings
survive. A command that has to wait for the lock waits milliseconds, since the lock is only
held for a read and a write.

Read-only commands such as `dashmate config get`, `dashmate status` and `dashmate core cli`
never write configuration at all, so they are always safe to run alongside anything else.

### While a node is being reconfigured

`dashmate setup`, `dashmate reset` and `dashmate ssl obtain` change configuration
repeatedly while doing long work, so they take the lock for their whole run. Another
command that changes configuration waits briefly and then reports that something else is
modifying it — nothing is lost, and running it again once the first command finishes
works normally.

Reading is never affected: `dashmate status`, `dashmate config get` and `dashmate core cli`
do not take the lock at all, so a node can still be inspected while it is being set up.

If a command holding the lock is killed, the lock is released. If the machine loses power
mid-command, the next writer takes over after about a minute.
