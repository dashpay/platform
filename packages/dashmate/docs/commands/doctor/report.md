# doctor report

The `doctor report` command collects diagnostic information and creates an archive for troubleshooting.

## Usage

```bash
dashmate doctor report [--config=<name>] [--verbose]
```

## Options

| Option | Description | Default |
|--------|-------------|--------|
| `-c, --config=<name>` | Configuration to diagnose | *Uses default config if not specified* |
| `-v, --verbose` | Use verbose mode for output | `false` |

## Description

This command collects comprehensive diagnostic information about your Dash node and creates an obfuscated archive that can be used for troubleshooting and debugging.

The archive contains:
- System information (OS, memory, CPU, disk usage)
- Node configuration (with sensitive data obfuscated)
- Service logs, metrics, and status
- Docker container information

All sensitive data like private keys and passwords is automatically obfuscated to protect your security and privacy.

Before creating the archive, the command asks for your confirmation. The generated archive is compressed with TAR/GZIP and saved to the current working directory.

You can use this archive to analyze the node's condition yourself or send it to the Dash Core Group support team for assistance.
The archive is particularly useful when seeking help on issues that are difficult to diagnose.

## Examples

```bash
# Create a diagnostic report for the default configuration
dashmate doctor report

# Create a report for a specific configuration
dashmate doctor report --config=mainnet

# Create a report with verbose output
dashmate doctor report --verbose
```

## Related Commands

- [doctor](./doctor.md) - Run diagnostics and apply fixes
- [status](../status/index.md) - Show node status
