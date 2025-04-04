# wallet mint

The `wallet mint` command generates test Dash (tDash) on a local development network.

## Usage

```bash
dashmate wallet mint AMOUNT [OPTIONS]
```

## Arguments

| Argument | Description | Required | Default |
|----------|-------------|----------|---------|
| `AMOUNT` | Amount of tDash to be generated | Yes | |

## Options

| Option | Description | Default |
|--------|-------------|---------|
| `-c, --config=<name>` | Configuration name to use | *Uses default config if not specified* |
| `-a, --address=<address>` | Recipient address (creates new address if not specified) | |
| `-v, --verbose` | Use verbose mode for output | `false` |

## Description

This command mints (generates) a specified amount of test Dash (tDash) and sends it to either a new wallet address or an address you specify.
It is only available in local development networks.

Key features:

1. **Local network only**: This command can only be used on `local` networks, not on testnet or mainnet.
2. **Not for masternodes**: This command cannot be used on nodes with masternode functionality enabled.
3. **Address creation**: If no recipient address is specified, the command creates a new address and sends the funds there.

This command is primarily useful for local development and testing when you need Dash for transactions, collateral, or other purposes.

## Examples

```bash
# Generate 100 tDash to a new address
dashmate wallet mint 100

# Generate 1000 tDash to a specific address
dashmate wallet mint 1000 --address=yNPbcFfabtNmmxKdGwhHomdYfVs6gikbPf

# Generate 500 tDash using a specific configuration
dashmate wallet mint 500 --config=local
```

## Limitations

- Only works on local networks
- Cannot be used on masternodes
- Requires that the node is running
- Core must be fully synced

## Related Commands

None
