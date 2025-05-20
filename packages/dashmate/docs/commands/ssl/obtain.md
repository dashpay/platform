# ssl obtain

The `ssl obtain` command creates or downloads an SSL certificate for secure communications.

## Usage

```bash
dashmate ssl obtain [OPTIONS]
```

## Options

| Option | Description | Default |
|--------|-------------|---------|
| `-c, --config=<name>` | Configuration name to use | *Uses default config if not specified* |
| `--no-retry` | Do not retry on IP verification failure | `false` |
| `--force` | Renew even if certificate is valid | `false` |
| `--expiration-days=<days>` | Renew if certificate expires in specified number of days | `30` |
| `-v, --verbose` | Use verbose mode for output | `false` |

## Description

This command obtains an SSL certificate using ZeroSSL as the provider.
The certificate enables secure HTTPS connections to your Dash node's services.

The command has three main functions:
1. Create a new SSL certificate if one doesn't exist
2. Download an existing certificate if it was already created
3. Renew an existing certificate if it's nearing expiration

By default, the command will renew certificates that are within 30 days of expiration.
This threshold can be adjusted using the `--expiration-days` option.

The certificate obtainment process involves domain verification, which usually requires your server to be publicly accessible.
If IP verification fails, the command will retry automatically unless the `--no-retry` flag is specified.

The `--force` flag can be used to replace a certificate even if it's still valid, which is useful for testing or when you need to replace a certificate for other reasons.

## Examples

```bash
# Obtain a certificate using the default configuration
dashmate ssl obtain

# Obtain a certificate for a specific configuration
dashmate ssl obtain --config=mainnet

# Force renewal of an existing certificate
dashmate ssl obtain --force

# Renew certificate if it expires in 60 days or less
dashmate ssl obtain --expiration-days=60

# Do not retry domain verification if it fails
dashmate ssl obtain --no-retry
```

## Related Commands

- [ssl cleanup](./cleanup.md) - Clean up SSL certificates and related files
