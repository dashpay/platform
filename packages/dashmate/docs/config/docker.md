# Docker Configuration

The `docker` section configures Docker-related settings for the entire platform.

## Network

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `docker.network.subnet` | Docker network subnet | `0.0.0.0/0` | `172.20.0.0/16` |

The subnet controls the IP address range assigned to Docker containers.

## Base Image

The base image build might be helpful for multi-stage docker images.
If base image is enabled, Dashmate will build the base image first and then use it for the rest of the images.

| Option | Description | Default | Example                       |
|--------|-------------|---------|-------------------------------|
| `docker.baseImage.build.enabled` | Enable building base images | `false` | `true`               |
| `docker.baseImage.build.context` | Build context directory | `null` | `"/path/to/context"`      |
| `docker.baseImage.build.dockerFile` | Path to Dockerfile | `null` | `"/path/to/Dockerfile"`       |
| `docker.baseImage.build.target` | Target build stage in multi-stage builds | `null` | `"base"`  |

Base image configuration example:
```json
{
  "build": {
    "enabled": true,
    "context": "/path/to/build/context",
    "dockerFile": "/path/to/Dockerfile",
    "target": "base"
  }
}
```
