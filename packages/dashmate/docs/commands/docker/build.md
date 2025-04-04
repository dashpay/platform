# docker build

The `docker build` command builds Docker images for services configured to be built from source.

## Usage

```bash
dashmate docker build [--config=<name>]
```

## Options

| Option | Description | Default |
|--------|-------------|--------|
| `-c, --config=<name>` | Configuration to use | *Uses default config if not specified* |
| `-v, --verbose` | Use verbose mode for output | `false` |

## Description

This command builds Docker images for any services that are configured to be built from source code.
It is useful when you want to use custom or development versions of Dash Platform services.

For a service to be built, it must have its build configuration properly set in the configuration file.

This typically includes:
- Setting `build.enabled` to `true`
- Providing a valid `build.context` (source code directory)
- Specifying a `build.dockerFile` path

If no services are configured to be built from source in the specified configuration, the command will show an error message.

Building images can take a significant amount of time depending on your hardware and the complexity of the services being built.

## Examples

```bash
# Build services for the default configuration
dashmate docker build

# Build services for a specific configuration
dashmate docker build --config=testnet

# Build services with verbose output
dashmate docker build --verbose
```

## Related Commands

- [config set](../config/set.md) - Configure services to be built from source
- [start](../start.md) - Start a node (uses built images if available)
