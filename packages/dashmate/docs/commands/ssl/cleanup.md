# ssl cleanup

The `ssl cleanup` command removes pending or invalid SSL certificates from ZeroSSL.

## Usage

```bash
dashmate ssl cleanup [OPTIONS]
```

## Options

| Option | Description | Default |
|--------|-------------|--------|
| `-c, --config=<name>` | Configuration to use | *Uses default config if not specified* |
| `-v, --verbose` | Use verbose mode for output | `false` |

## Description

This command cleans up SSL certificates that are in a drafted or pending validation state on ZeroSSL.

It is useful for:
- Removing certificates that failed validation
- Cleaning up after unsuccessful certificate issuance attempts
- Removing old certificates before creating new ones

The command uses the ZeroSSL API credentials configured in your Dashmate configuration file.
It identifies and cancels certificates associated with your domain that are not fully issued or are invalid.

This command does not affect already issued and valid certificates that are in use by your node.

## Examples

```bash
# Clean up certificates for the default configuration
dashmate ssl cleanup

# Clean up certificates for a specific configuration
dashmate ssl cleanup --config=mainnet

# Clean up certificates with verbose output
dashmate ssl cleanup --verbose
```

## Related Commands

- [ssl obtain](./obtain.md) - Obtain a new SSL certificate
