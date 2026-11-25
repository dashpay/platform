# doctor

The `doctor` command performs diagnostics on your Dash node and suggests solutions for any issues found.

## Usage

```bash
dashmate doctor [OPTIONS]
```

## Options

| Option | Description | Default |
|--------|-------------|---------|
| `-c, --config=<name>` | Configuration name to use | *Uses default config if not specified* |
| `-s, --samples=<path>` | Path to a samples archive file for analysis | |
| `-v, --verbose` | Use verbose mode for output | `false` |

## Description

This command acts as a diagnostic tool for your Dash node, checking for common issues and providing solutions.

It works by:
1. Collecting data (samples) from your node's configuration and services
2. Analyzing this data to identify potential problems
3. Suggesting solutions for each identified issue

The command categorizes problems by severity (high, medium, low) and provides step-by-step instructions for resolving each issue.
Problems with high severity are highlighted in red, while medium severity issues appear in yellow.

Doctor also checks every node on the host, not only the selected config, for whether it was restarted on the installed dashmate.
After the dashmate package is upgraded, each node keeps running the services the previous version started until it is restarted.
The check reads the image of the node's dashmate helper container, which is pinned to the dashmate version that started it.
Such a node is reported with the command that restarts it (`dashmate restart --config <name>`, with `--safe` for a masternode).
A node whose state cannot be determined is reported as unknown. Stopped nodes and local network configs are not reported.

You can also analyze a previously created samples archive by specifying the `--samples` option.
This is useful when you want to diagnose a node without directly accessing it, or when analyzing historical data.

If high-severity issues are found, the command will exit with a non-zero status code (1), which can be useful for scripts or automation.

## Examples

```bash
# Run diagnostics on the default configuration
dashmate doctor

# Run diagnostics on a specific configuration
dashmate doctor --config=testnet

# Analyze a previously created samples archive
dashmate doctor --samples=/path/to/dashmate-samples.tar.gz

# Run diagnostics with verbose output
dashmate doctor --verbose
```

## Related Commands

- [doctor report](./report.md) - Generate a diagnostic report without applying fixes
