# core reindex

The `core reindex` command reindexes the Core node's blockchain data.

## Usage

```bash
dashmate core reindex [OPTIONS]
```

## Options

| Option | Description | Default |
|--------|-------------|--------|
| `-c, --config=<name>` | Configuration to use | *Uses default config if not specified* |
| `-v, --verbose` | Use verbose mode for output | `false` |
| `-d, --detach` | Run the reindex process in the background | `false` |
| `-f, --force` | Reindex already running node without confirmation | `false` |

## Description

This command reindexes the blockchain data for a Dash Core node.

Reindexing can be necessary in certain situations, such as:
- After enabling or disabling indexes (address index, transaction index, etc.)
- When blockchain data becomes corrupted
- After a software upgrade that requires reindexing

Reindexing is a resource-intensive operation that involves rescanning the entire blockchain.
Depending on the size of the blockchain and your hardware, this process can take a significant amount of time (hours to days).

By default, the command will run in the foreground, showing progress. Use the `--detach` flag to run it in the background.

If the node is already running, the command will prompt for confirmation before stopping and reindexing it.
Use the `--force` flag to skip this confirmation.

## Examples

```bash
# Reindex the default configuration
dashmate core reindex

# Reindex a specific configuration
dashmate core reindex --config=mainnet

# Reindex in the background
dashmate core reindex --detach

# Force reindex of a running node without confirmation
dashmate core reindex --force

# Reindex with verbose output
dashmate core reindex --verbose
```

## Related Commands

- [core cli](./cli.md) - Pass commands to the Dash Core CLI
- [group core reindex](../group/core/reindex.md) - Reindex all Core nodes in a group
