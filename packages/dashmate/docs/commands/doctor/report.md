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

This command collects comprehensive diagnostic information about your Dash node and creates an archive, with the node's secrets masked, that can be used for troubleshooting and debugging.

The archive contains:
- System information (OS, memory, CPU, disk usage)
- Node configuration (with its secrets masked)
- Service logs, metrics, and status
- Docker container information (environment variable and command-line option names only, without their values, and no shell command lines)

Secrets in the current node configuration, such as private keys, passwords, API keys, build arguments, command-line option values and credentials in URLs, are masked in the archive's copy of the configuration. The values of container environment variables and command-line options are left out, and so is any shell command line a container runs. Your home directory is masked, and so is your user name in logs, paths, the node description and contact addresses; it is left as it is in other configuration values, which it could be spelled like. Copies of the configuration's secrets elsewhere in the archive, including service logs, are masked when they are 8 characters or longer; shorter values are not searched for, because they would match unrelated text.

Logs can still contain other sensitive text, such as earlier passwords or short values, so review the archive before you share it. The rest is diagnostic data about your node, such as its addresses, host details, certificate settings and logs, and is shared with whoever you send the archive to.

Before creating the archive, the command asks for your confirmation. The generated archive is compressed with TAR/GZIP and saved to the current working directory.

Once you confirm, the command also removes the files that earlier dashmate versions left in the system temporary directory while creating reports: `dashmate-report-<date>` directories owned by the current user that hold only the files those versions wrote. A directory under that name that holds anything else, such as an unpacked report with your own notes in it, is left in place and closed to other users. The command lists what it removed and what it left.

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
