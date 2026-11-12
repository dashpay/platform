# stop

The `stop` command shuts down a running Dash node and its associated services.

## Usage

```bash
dashmate stop [OPTIONS]
```

## Options

| Option | Description | Default |
|--------|-------------|---------|
| `-c, --config=<name>` | Configuration name to use | *Uses default config if not specified* |
| `-f, --force` | Force stop even if any service is running or DKG is in progress | `false` |
| `-p, --platform` | Stop only platform services (not Core) | `false` |
| `-s, --safe` | Wait for DKG (Distributed Key Generation) to complete before stopping | `false` |
| `-v, --verbose` | Use verbose mode for output | `false` |

## Description

This command stops a running Dash node and its associated Docker containers. By default, it stops all services for the node.

If you only want to stop the Platform services while keeping Core running, use the `--platform` flag.

Normally, the command will stop services immediately. If a Distributed Key Generation (DKG) round is in progress, this could lead to node being banned.
Using the `--safe` flag ensures the command waits for any current DKG round to complete before stopping the node.

The `--force` flag can be used to forcibly stop services even if there's a DKG in progress or other conditions that would normally prevent shutdown.

## Stopping a masternode safely

A masternode that misses its part in a DKG, the session that creates a new quorum's keys, can get a PoSe penalty. Before stopping a masternode, dashmate asks Core whether stopping now could cause that:

- a DKG this node is a member of is still running: from its start block to the end of its Commit phase, 10 blocks for most quorum types and 20 for the 400-member ones;
- or one starts within the next 6 blocks, too soon for a restart to finish. Core 24 tells dashmate whether this node is a member; with older Core, any DKG counts.

Without `--safe` or `--force`, the command refuses in that case and prints what blocks the stop and the earliest block at which stopping is safe.

With `--safe`, it checks every 10 seconds and stops the node as soon as that is safe. While it waits it shows the DKGs it is waiting for and the earliest safe block, with a time estimate on mainnet, testnet and devnets. The estimate can grow: Core announces a node's place in a DKG only about 8 blocks before the DKG starts. With Core 24, an evonode usually waits under half an hour, longer around the twice-daily start of the InstantSend quorum cycle. Older Core does not report membership, so every start of that cycle blocks for about an hour and a half. Pressing Ctrl+C ends the wait and leaves the node running.

`--safe` gives up and leaves the node running when Core stops answering for about a minute, or when it keeps reporting DKG data dashmate cannot read.

On Core older than 24.1, dashmate checks a safe result again 15 seconds later, so stopping a masternode takes about that much longer.

How dashmate decides is described in [DKG-safe stop](../specs/dkg-safe-stop.md).

## Examples

```bash
# Stop node with default configuration
dashmate stop

# Stop node with a specific configuration
dashmate stop --config=testnet

# Stop only platform services
dashmate stop --platform

# Safely stop a node, waiting for DKG to complete
dashmate stop --safe

# Force stop a node regardless of conditions
dashmate stop --force
```

## Related Commands

- [start](./start.md) - Start a node
- [restart](./restart.md) - Restart a node
- [status](./status/index.md) - Show node status
