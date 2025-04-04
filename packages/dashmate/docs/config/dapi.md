# DAPI Configuration

DAPI provides API services for Dash Platform, allowing external applications to interact with the platform.

## DAPI Docker Configuration

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.dapi.api.docker.image` | Docker image for DAPI | `dashpay/dapi:${version}` | `dashpay/dapi:latest` |
| `platform.dapi.api.docker.build` | Build settings for DAPI API | Object | See below |
| `platform.dapi.api.docker.deploy.replicas` | Number of DAPI replicas | `1` | `3` |

The `docker.build` object allows for custom build settings:
```json
"platform.dapi.api.docker.build": {
  "enabled": true,
  "path": "/path/to/dapi/source"
}
```

It allows you to specify a custom build path for the DAPI API Docker image. If `enabled` is set to `true`, Dashmate will build the Docker image from the specified path.

## DAPI Configuration

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.dapi.api.waitForStResultTimeout` | Timeout for state transitions | `120000` | `240000` |

This timeout setting controls how long DAPI will wait for state transition results before returning a timeout error to the client.
