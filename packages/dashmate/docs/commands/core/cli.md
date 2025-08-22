# core cli

The `core cli` command passes commands directly to the Dash Core RPC.

## Usage

```bash
dashmate core cli "COMMAND" [--config=<name>]
```

## Arguments

| Argument | Description | Required | Default |
|----------|-------------|----------|--------|
| `COMMAND` | Dash Core command (in double quotes) | Yes | |

## Options

| Option | Description | Default |
|--------|-------------|--------|
| `-c, --config=<name>` | Configuration to use | *Uses default config if not specified* |

## Description

This command allows you to run Dash Core RPC commands directly from Dashmate.
It passes the command to the Dash Core service running in Docker and returns the output.

The command provides direct access to all Dash Core functions via its command-line interface, including wallet operations, blockchain queries, network control, and more.

The core service must be running for this command to work.
If the service is not running, the command will fail with an error message.

## Examples

```bash
# Get blockchain information
dashmate core cli "getblockchaininfo"

# Get network information
dashmate core cli "getnetworkinfo"

# Get wallet information
dashmate core cli "getwalletinfo"

# Get help for available commands
dashmate core cli "help"

# Run a command on a specific configuration
dashmate core cli "getblockcount" --config=testnet
```

## Related Commands

- [core reindex](./reindex.md) - Reindex the Core node's blockchain data
- [status core](../status/core.md) - Show Core node status
