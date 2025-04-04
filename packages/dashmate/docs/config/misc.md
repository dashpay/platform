# Miscellaneous Configuration

This section covers various configuration options that don't fit into the other main categories.

## Docker Configuration

The `docker` section configures Docker-related settings for the entire platform.

### Docker Network Configuration

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `docker.network.subnet` | Docker network subnet | `0.0.0.0/0` | `172.20.0.0/16` |

The subnet controls the IP address range assigned to Docker containers.

### Docker Base Image Configuration

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `docker.baseImage.build.enabled` | Enable building base images | `false` | `true` |
| `docker.baseImage.build.repository` | Repository for base images | `dashpay/dashmate` | `your-repo/dashmate` |
| `docker.baseImage.build.tag` | Tag for base images | `latest` | `dev` |

Base image configuration example:
```javascript
"docker.baseImage.build": {
  "enabled": true,
  "repository": "dashpay/dashmate",
  "tag": "latest"
}
```

## Network and Environment

These top-level options set the network and environment:

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `network` | Dash network to use | `mainnet` | `testnet`, `devnet`, `local` |
| `environment` | Environment type | `production` | `development` |
| `externalIp` | External IP address | `null` | `"203.0.113.1"` |
| `description` | Human-readable description of the configuration | `base config for use as template` | `"My mainnet masternode"` |
| `group` | Configuration group for managing multiple nodes | `null` | `"testnet-cluster"` |
| `platform.sourcePath` | Path to Platform source code for development | `null` | `"/path/to/platform"` |
