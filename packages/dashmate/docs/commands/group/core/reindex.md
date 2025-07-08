# group core reindex

The `group core reindex` command reindexes the Core blockchain data for all nodes in a group.

## Usage

```bash
dashmate group core reindex [OPTIONS]
```

## Options

| Option | Description | Default |
|--------|-------------|--------|
| `-g, --group=<name>` | Group to reindex | *Uses default group if not specified* |
| `-v, --verbose` | Use verbose mode for output | `false` |
| `-d, --detach` | Run the reindex process in the background | `false` |
| `-f, --force` | Reindex already running nodes without confirmation | `false` |

## Description

This command reindexes the blockchain data for all Dash Core nodes in a specified group.

This operation rebuilds the blockchain indexes for each node and is necessary in certain situations:
- After enabling or disabling indexes (address index, transaction index, etc.)
- When blockchain data becomes corrupted
- After a software upgrade that requires reindexing

The command will check if any of the group nodes are running.
If they are, it will warn you that the nodes will be unavailable until the reindex is complete and ask for confirmation before proceeding, unless the `--force` flag is used.

By default, the command will run in the foreground, showing progress for each node. Use the `--detach` flag to run it in the background.

Reindexing is a resource-intensive operation that involves rescanning the entire blockchain.
For a group of nodes, this can take a significant amount of time depending on your hardware and the number of nodes in the group.

## Examples

```bash
# Reindex all nodes in the default group
dashmate group core reindex

# Reindex all nodes in a specific group
dashmate group core reindex --group=local

# Reindex in the background
dashmate group core reindex --detach

# Force reindex without confirmation
dashmate group core reindex --force

# Reindex with verbose output
dashmate group core reindex --verbose
```

## Related Commands

- [core reindex](../../core/reindex.md) - Reindex a single Core node
- [group reset](../reset.md) - Reset all nodes in a group
