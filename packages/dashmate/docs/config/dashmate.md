# Dashmate Helper Configuration

The `dashmate` section configures the Dashmate helper service that assists with various operations.

## Dashmate API

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `dashmate.helper.api.enable` | Enable helper API | `false` | `true` |
| `dashmate.helper.api.port` | Helper API port | `9100` | `9101` |

The helper JSON RPC API provides auxiliary endpoints for monitoring and management.
When enabled, it will listen on the specified port for API requests.
RPC accepts dashmate command names and arguments as parameters.

## Docker

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `dashmate.helper.docker.build.enabled` | Enable custom build | `false` | `true` |
| `dashmate.helper.docker.build.context` | Build context directory | `null` | `"/path/to/context"` |
| `dashmate.helper.docker.build.dockerFile` | Path to Dockerfile | `null` | `"/path/to/Dockerfile"` |
| `dashmate.helper.docker.build.target` | Target build stage in multi-stage builds | `null` | `"dashmate_helper"` |

Docker build configuration example:
```json
{
  "build": {
    "enabled": true,
    "context": "/path/to/build/context",
    "dockerFile": "/path/to/Dockerfile",
    "target": "dashmate_helper"
  }
}
```

These settings allow you to build the Dashmate helper Docker image from source. If `enabled` is set to `true`, Dashmate will build the Docker image using the specified context directory and Dockerfile.
