# Dashmate Helper Configuration

The `dashmate` section configures the Dashmate helper service that assists with various operations.

## Dashmate Helper API Configuration

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `dashmate.helper.api.enable` | Enable helper API | `false` | `true` |
| `dashmate.helper.api.port` | Helper API port | `9100` | `9101` |
| `dashmate.helper.api.host` | Host binding for helper API | `127.0.0.1` | `0.0.0.0` |

The helper API provides auxiliary endpoints for monitoring and management.

## Dashmate Helper Docker Configuration

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `dashmate.helper.docker.build.enabled` | Enable custom build | `false` | `true` |
| `dashmate.helper.docker.build.path` | Path to source code | `null` | `"/path/to/source"` |

Docker build configuration example:
```json
"dashmate.helper.docker.build": {
  "enabled": true,
  "path": "/path/to/dashmate/helper/source"
}
```

It allows you to specify a custom build path for the Dashmate helper Docker image. If `enabled` is set to `true`, Dashmate will build the Docker image from the specified path.
