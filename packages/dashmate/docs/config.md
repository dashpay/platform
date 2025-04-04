# Dashmate Configuration Guide

This document provides comprehensive information about Dashmate configuration options. Dashmate uses a hierarchical configuration system with various sections that control different aspects of the Dash Platform.

## Configuration Overview

Dashmate configuration is organized into a hierarchical structure with the following main sections:

- **core**: Options for Dash Core node
- **platform**: Options for Dash Platform components
- **docker**: Docker-related configuration
- **dashmate**: Dashmate-specific configuration
- **externalIp**: External IP address for the node
- **network**: Network selection (mainnet, testnet, etc.)
- **environment**: Environment type (production, development)

## Configuration Basics

### Managing Configs

Dashmate allows you to manage multiple configurations:

```bash
# List all available configs
dashmate config list

# Create a new config
dashmate config create <name> [--preset=<preset>]

# Get a specific config option
dashmate config get <option>

# Set a specific config option
dashmate config set <option> <value>

# Remove a config
dashmate config remove <name>
```

### Config Files Location

Configuration files are stored in the Dashmate home directory:

- Default location: `~/.dashmate/configs`
- Can be changed with the `DASHMATE_HOME_DIR` environment variable

## Core Configuration

The `core` section contains options for configuring the Dash Core node.

### Basic Core Options

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `core.docker.image` | Docker image for Dash Core | `dashpay/dashd:22` | `dashpay/dashd:latest` |
| `core.p2p.port` | P2P port for Dash Core | `9999` | `19999` |
| `core.p2p.host` | Host binding for P2P | `0.0.0.0` | `127.0.0.1` |
| `core.rpc.port` | RPC port for Dash Core | `9998` | `19998` |
| `core.rpc.host` | Host binding for RPC | `127.0.0.1` | `0.0.0.0` |

### Core RPC Users

The `core.rpc.users` section defines RPC users and their permissions:

```javascript
{
  "core": {
    "rpc": {
      "users": {
        "dashmate": {
          "password": "rpcpassword",
          "whitelist": null,
          "lowPriority": false
        },
        "dapi": {
          "password": "rpcpassword",
          "whitelist": ["getbestblockhash", "getblockhash", "sendrawtransaction", ...],
          "lowPriority": true
        },
        // More users...
      }
    }
  }
}
```

Each user has:
- `password`: RPC password for authentication
- `whitelist`: List of allowed RPC methods (null means all methods)
- `lowPriority`: Whether the user's requests have low priority

### Core Insight Configuration

Insight provides a block explorer for Dash Core:

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `core.insight.enabled` | Enable Insight API | `false` | `true` |
| `core.insight.ui.enabled` | Enable Insight UI | `false` | `true` |
| `core.insight.port` | Port for Insight API/UI | `3001` | `3002` |

## Platform Configuration

The `platform` section configures Dash Platform components.

### Platform Gateway

Gateway is the entry point for external clients:

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.gateway.docker.image` | Docker image for Gateway | `dashpay/envoy:1.30.2-impr.1` | `dashpay/envoy:latest` |
| `platform.gateway.listeners.dapiAndDrive.port` | Gateway API port | `443` | `8443` |
| `platform.gateway.listeners.dapiAndDrive.host` | Gateway API host binding | `0.0.0.0` | `127.0.0.1` |
| `platform.gateway.ssl.enabled` | Enable SSL | `false` | `true` |
| `platform.gateway.ssl.provider` | SSL provider | `zerossl` | `selfSigned` |

### Rate Limiter Configuration

Gateway includes rate limiting functionality:

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.gateway.rateLimiter.enabled` | Enable rate limiter | `true` | `false` |
| `platform.gateway.rateLimiter.requestsPerUnit` | Requests allowed per time unit | `150` | `300` |
| `platform.gateway.rateLimiter.unit` | Time unit for rate limiting | `minute` | `hour` |
| `platform.gateway.rateLimiter.whitelist` | IPs exempt from rate limiting | `[]` | `["192.168.1.1"]` |
| `platform.gateway.rateLimiter.blacklist` | IPs blocked from all requests | `[]` | `["10.0.0.1"]` |
| `platform.gateway.rateLimiter.metrics.enabled` | Enable metrics for rate limiter | `false` | `true` |
| `platform.gateway.rateLimiter.metrics.port` | Prometheus metrics port | `9102` | `9103` |

### Drive ABCI Configuration

Drive ABCI contains the application logic for Dash Platform:

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.drive.abci.docker.image` | Docker image for Drive ABCI | `dashpay/drive:${version}` | `dashpay/drive:latest` |
| `platform.drive.abci.tokioConsole.enabled` | Enable Tokio debugging console | `false` | `true` |
| `platform.drive.abci.tokioConsole.port` | Tokio console port | `6669` | `6670` |
| `platform.drive.abci.grovedbVisualizer.enabled` | Enable GroveDB visualization tool | `false` | `true` |
| `platform.drive.abci.grovedbVisualizer.port` | GroveDB visualization port | `8083` | `8084` |

### Tenderdash Configuration

Tenderdash is the consensus engine for Dash Platform:

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.drive.tenderdash.docker.image` | Docker image for Tenderdash | `dashpay/tenderdash:1` | `dashpay/tenderdash:latest` |
| `platform.drive.tenderdash.p2p.port` | P2P port for tenderdash | `26656` | `26657` |
| `platform.drive.tenderdash.p2p.host` | Host binding for P2P | `0.0.0.0` | `127.0.0.1` |
| `platform.drive.tenderdash.rpc.port` | RPC port for tenderdash | `26657` | `26658` |
| `platform.drive.tenderdash.rpc.host` | Host binding for RPC | `127.0.0.1` | `0.0.0.0` |
| `platform.drive.tenderdash.metrics.enabled` | Enable metrics | `false` | `true` |
| `platform.drive.tenderdash.metrics.port` | Metrics port | `26660` | `26661` |

### DAPI Configuration

DAPI provides API services for Dash Platform:

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.dapi.api.docker.image` | Docker image for DAPI | `dashpay/dapi:${version}` | `dashpay/dapi:latest` |
| `platform.dapi.api.docker.deploy.replicas` | Number of DAPI replicas | `1` | `3` |
| `platform.dapi.api.waitForStResultTimeout` | Timeout for state transitions | `120000` | `240000` |

## Docker Configuration

The `docker` section configures Docker-related settings:

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `docker.network.subnet` | Docker network subnet | `0.0.0.0/0` | `172.20.0.0/16` |
| `docker.baseImage.build.enabled` | Enable building base images | `false` | `true` |

## Dashmate Helper Configuration

The `dashmate` section configures the Dashmate helper service:

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `dashmate.helper.api.enable` | Enable helper API | `false` | `true` |
| `dashmate.helper.api.port` | Helper API port | `9100` | `9101` |

## Network and Environment

These top-level options set the network and environment:

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `network` | Dash network to use | `mainnet` | `testnet`, `devnet`, `local` |
| `environment` | Environment type | `production` | `development` |
| `externalIp` | External IP address | `null` | `"203.0.113.1"` |

## Config Presets

Dashmate provides several presets for quick configuration:

- **local**: Local development setup with all services
- **local_core**: Local Core-only setup
- **local_drive**: Local Drive setup (no Core)
- **fullnode**: Fullnode setup (with Core)
- **masternode**: Masternode setup for producing blocks
- **evonode**: Evonode setup (with Platform services)

## Example Configuration Tasks

### Setting Up a Local Development Environment

```bash
# Create a local config
dashmate config create local --preset=local

# Start the services
dashmate setup local
```

### Configuring a Masternode

```bash
# Create a masternode config
dashmate config create mainnet_masternode --preset=masternode

# Set external IP
dashmate config set externalIp <your-ip-address>

# Set BLS private key
dashmate config set core.masternode.operator.privateKey <your-bls-private-key>

# Setup and start the masternode
dashmate setup mainnet_masternode
```

### Enabling SSL for Gateway

```bash
# Enable SSL with ZeroSSL
dashmate config set platform.gateway.ssl.enabled true
dashmate config set platform.gateway.ssl.provider zerossl
dashmate config set platform.gateway.ssl.providerConfigs.zerossl.apiKey <your-api-key>
```

### Customizing Rate Limits

```bash
# Increase rate limits
dashmate config set platform.gateway.rateLimiter.requestsPerUnit 300
dashmate config set platform.gateway.rateLimiter.unit minute

# Add IP to whitelist
dashmate config set platform.gateway.rateLimiter.whitelist '["192.168.1.100"]'
```

## Advanced Configuration

### Environment Variables

Some settings can be overridden with environment variables:

- `DASHMATE_HOME_DIR`: Dashmate home directory
- `DASHMATE_CONFIG_NAME`: Default config name to use
- `LOCAL_UID` and `LOCAL_GID`: User and group IDs for running containers

### Configuration Files

The configuration is stored in JSON format and can be edited directly:

```bash
# Open the config file in an editor
nano ~/.dashmate/configs/<config-name>.json
```

## Troubleshooting

### Common Configuration Issues

- **Port conflicts**: If a port is already in use, try changing the port in the configuration.
- **Networking issues**: Check if the `externalIp` is correctly set and accessible.
- **Docker permission issues**: Make sure your user has permissions to access the Docker socket.

### Debugging Configuration

```bash
# Check the current configuration
dashmate config get

# Check a specific option
dashmate config get <option>

# Enable debug logging
dashmate config set core.log.debug.enabled true
```

## Configuration Migration

When updating Dashmate, configurations are automatically migrated to the new format. You can manually trigger migration with:

```bash
dashmate update
```