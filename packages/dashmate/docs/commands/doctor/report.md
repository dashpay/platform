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
- Docker container information (environment variable names without their values; the commands containers run and what their health checks print are left out)

Secrets in the current node configuration, such as private keys, passwords, API keys, build arguments, command-line option values and credentials in URLs, are masked in the archive's copy of the configuration. The values of container environment variables are left out, and so are the commands containers run and what their health checks print; the executable and the results of the health checks are kept. Your home directory is masked, and so is your user name in logs, paths, the node description and contact addresses; it is left as it is in other configuration values, which it could be spelled like.

Copies of the configuration's secrets elsewhere in the archive are masked when they are 8 characters or longer. That covers the text the archive carries: service logs, metrics, RPC responses, messages, file paths, and the node description and contact addresses in the copy of the configuration. Shorter values are not searched for, because they would match unrelated text. The values doctor decides on when it analyses the archive are kept as they are, so a secret spelled like one of them is not masked there: other values in the copy of the configuration, the error code of the SSL certificate check, the status, reason codes, dates and fingerprints of the installed and served certificates, and the provider, outcome, failure code and times of the last certificate renewal. A renewal provider that dashmate does not recognise is replaced with `unknown`, and a failure code with `UNKNOWN`.

The archive can still contain other sensitive text, such as earlier passwords, short values, or a secret spelled like one of the values kept above, so review it before you share it. The rest is diagnostic data about your node, such as its addresses, host details, certificate settings and logs, and is shared with whoever you send the archive to.

Before creating the archive, the command asks for your confirmation. The generated archive is compressed with TAR/GZIP and saved to the current working directory.

Once you confirm, the command also looks for the files that earlier dashmate versions left in the system temporary directory while creating reports: `dashmate-report-<date>` directories owned by the current user that hold only the files those versions wrote. It lists the directories it found and removes them only if you agree, since a report you unpacked there yourself looks the same. A directory under that name that holds anything else, such as an unpacked report with your own notes in it, is left in place. The command tries to close every directory it looks at to other users and says in its output whether it could; it cannot on a read-only file system, for example. Check that output, and restrict or remove any directory it reports it could not close. If it could not look in the temporary directory at all, it says so separately. Before removing a directory, the command moves it into a `.dashmate-report-removal-*` directory beside it that only you can open, and checks it again there; one that fails that check is left there, and the command says where. If such a directory is still there from an earlier run, for example because that run was stopped, the command lists it and leaves it for you to check and remove.

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
