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
survive. When no long-running operation owns the lock, a command waits only for the locked
read and write.

Read-only commands such as `dashmate config get`, `dashmate status` and `dashmate core cli`
normally do not write configuration. The first command after an upgrade may migrate and save
`config.json`; that migration needs the same lock and can time out behind a long-running
configuration change.

### While a node is being reconfigured

`dashmate setup`, `dashmate reset`, `dashmate group reset`, `dashmate ssl obtain`,
`dashmate core reindex` and `dashmate group core reindex` change or render configuration
while doing long work, so they take the lock for their whole run. Another command that
needs the lock waits briefly and then reports that something else is modifying it — nothing
is lost, and running it again once the first command finishes works normally.

The Dashmate helper uses the same whole-operation lock while renewing an SSL certificate.
For ZeroSSL this includes HTTP validation and may take minutes. A configuration-changing
command started during background renewal can therefore reach its 15-second timeout and
report that another Dashmate command is modifying configuration. Retry it after renewal
finishes. Keeping the lock for issuance is intentional: releasing it earlier would require
replaying selected renewal fields later, which could undo an operator's provider switch or
SSL disable.

Ordinary reads remain available: `dashmate status`, `dashmate config get` and
`dashmate core cli` do not take the lock unless loading the configuration discovers a
migration that must be saved.

Graceful termination releases the lock. After `SIGKILL` or a power loss, the next writer
takes over after about a minute.

### Recovering interrupted filesystem work

`config.json` is the authoritative configuration. For normal configuration changes and
certificate renewal, Dashmate saves it before writing the derived service files. If a
command reports a service-file rendering error, or is killed after saving, those files can
still describe the previous configuration. Repair the affected config explicitly:

```bash
dashmate config render --config=<name>
```

Removing a config also saves `config.json` before deleting its service directory. If the
directory deletion fails, Dashmate leaves the orphan in place and will refuse to create a
new config with that name. Inspect `~/.dashmate/<name>` for keys or other state that must be
kept, then move or delete the directory manually before retrying `dashmate config create`.
An absent name passed to `dashmate config remove` is rejected; it is not an orphan-cleanup
command.

If a command loses its lock before saving, Dashmate preserves the pending JSON in a private
`~/.dashmate/.config.json.rescue-<id>` file and reports that path. Compare it with
`config.json` and retain any needed values. Delete the rescue file manually only after its
contents have been acknowledged; Dashmate does not remove rescue files automatically.

The lock coordinates versions of Dashmate that implement this protocol. An older Dashmate
process that does not use the lock can still write concurrently, so finish rolling out the
new version before relying on this guarantee. The protocol is intended for a Dashmate home
directory on a local filesystem; network filesystems may not provide the required lock and
atomic-replacement semantics.
