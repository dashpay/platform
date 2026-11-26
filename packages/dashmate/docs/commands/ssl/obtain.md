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

Before it enables TLS, the command checks the certificate files it leaves: the bundle and the private key must load as a matching pair, and the certificate must be inside its validity window (not expired, and not valid only from a later date). A bundle on disk counts as downloaded only when it passes this check and its certificate has the fingerprint ZeroSSL reports for the saved certificate ID. Any other bundle, for example one cut short by a full disk or the one a renewal is replacing, is downloaded again from ZeroSSL while ZeroSSL holds the certificate as issued, without creating a new one. A downloaded certificate that does not belong to the private key on disk is not written; obtain a new certificate with `--force` in that case, and also when ZeroSSL has no certificate with the saved ID.

Renewal sends the certificate request in `csr.pem` to ZeroSSL, and nothing else the file holds. When that file is missing or cannot be used (it does not parse, it is not for the private key on disk, or it also holds a private key), the command writes a new one for the private key on disk while the certificate is issued and valid. When the certificate is already due for renewal, it asks for `--force` instead.

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
