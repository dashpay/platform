# Miscellaneous Configuration

This section covers various configuration options.

## Platform Configuration

| Option | Description                  | Default | Example |
|--------|------------------------------|---------|---------|
| `platform.enable` | Enable Dash Platform services | `false` | `true` |
| `platform.sourcePath` | Path to Platform source code | `null` | `"/path/to/platform"` |

When `platform.enable` is set to `true`, Dashmate will start all the necessary Platform services (Drive, DAPI, etc.). This automatically adds required indexes to Core.

## Config options

| Option | Description                  | Default | Example |
|--------|------------------------------|---------|---------|
| `description` | Human-readable description of the configuration | `base config for use as template` | `"My mainnet masternode"` |
| `group` | Configuration group for managing multiple nodes | `null` | `"testnet-cluster"` |

The config grouping enables the `dashmate group` commands to manage multiple nodes easily. You can create groups of configurations to manage them as a single unit.

## Network and Environment

These top-level options set the network and environment:

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `network` | Dash network to use | `mainnet` | `testnet`, `devnet`, `local` |
| `environment` | Environment type | `production` | `development` |
| `externalIp` | External IP address | `null` | `"203.0.113.1"` |
