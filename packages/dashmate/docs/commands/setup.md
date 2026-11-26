# setup

The `setup` command initializes a new Dash node with the specified configuration preset.

## Usage

```bash
dashmate setup [PRESET] [OPTIONS]
dashmate setup --platform [PRESET] [OPTIONS]
```

## Arguments

| Argument | Description | Required | Default |
|----------|-------------|----------|---------|
| `preset` | Node configuration preset (`mainnet`, `testnet`, `local`) | No | *Prompts if not provided* |

## Options

| Option | Description | Default |
|--------|-------------|---------|
| `-d, --debug-logs` | Enable debug logs | `false` |
| `--no-debug-logs` | Disable debug logs | |
| `-c, --node-count=<number>` | Number of nodes to setup (for `local` preset, minimum 3) | |
| `-m, --miner-interval=<interval>` | Interval between blocks (for `local` preset) | |
| `-v, --verbose` | Use verbose mode for output | `false` |
| `-p, --platform` | Set up only Platform; see [Platform-only setup](#platform-only-setup) | `false` |

## Description

The `setup` command configures a new Dash node with one of the following presets:

- **mainnet**: Run a node connected to the Dash main network
- **testnet**: Run a node connected to the Dash test network
- **local**: Run a full network environment on your machine for local development

If no preset is specified, the command will prompt you to select one.

For the `local` preset, you can specify how many nodes to create with the `--node-count` option. At least 3 nodes are required for proper operation.

The command prevents you from overwriting an existing configuration.
If a configuration for the specified preset already exists, the command will suggest using `dashmate reset` or `dashmate group reset` to start from scratch. When the preset's Platform part is not set up, it also suggests `dashmate setup --platform`, for a node whose Platform part was reset with `dashmate reset --platform --hard`.

## Platform-only setup

`dashmate setup --platform` configures the Platform part of a `mainnet` or `testnet` node whose Core part is set up and whose Platform part is not: the state `dashmate reset --platform --hard` leaves. It writes only Platform options (Platform enabled, the Tenderdash mode, the node key, the Platform P2P and HTTP ports, the gateway certificate), plus `externalIp` on a fullnode that has none. Other Platform options, such as image pins and log levels, stay as they are. The default config does not change.

- An evonode (`core.masternode.enable` is `true`) gets the identity and endpoints its registration records. Setup reads the registration from the masternode list by the node's operator key, starting Core for the duration of the command if it is stopped, and accepts only the node key whose node ID is registered. There is no way past this check: Tenderdash authenticates node IDs when validators connect. A self-signed certificate is not offered, because `update` refuses one on any masternode.
- An Evo fullnode keeps its stored node key unless you choose another one or give one, and chooses its ports (fixed on mainnet).

Setup refuses, changing nothing, when:

- the preset is `local` (set the group up again with `dashmate group reset --group local --hard` and `dashmate setup local`), or not a preset;
- Core was never set up (run `dashmate setup <preset>`);
- Platform was configured: a masternode's node key, TLS enabled, ZeroSSL settings, certificate files in the gateway directory, or Platform data volumes left by `reset --platform --hard --keep-data`. The message lists what it found, by path, and how to start over;
- Platform services are running;
- the masternode is registered as Regular, no registration uses its operator key, or its registered Platform endpoints are not on its external IP.

`PRESET` can also be given as `--config`. Without either, setup uses the default config when it is `mainnet` or `testnet`, and asks otherwise.

| Option | Description |
|--------|-------------|
| `--config=<name>` | The preset to set up, instead of `PRESET` |
| `--non-interactive` | Never prompt; every answer comes from a flag. Also `DASHMATE_NON_INTERACTIVE`, `CI`, and a stdin or stdout that is not a terminal |
| `--platform-node-key-file=<path\|->` | File holding the Tenderdash node key, as base64 or as a `node_key.json`; `-` reads stdin. Required for an evonode when not interactive. No flag takes the key itself, because command lines are visible to other users |
| `--platform-p2p-port=<port>`, `--platform-http-port=<port>` | Platform ports of a fullnode; without them the stored ones are kept. On mainnet they must be the mainnet ports (26656 and 443). For an evonode they must equal the registered ones |
| `--external-ip=<ipv4>` | External IP of a fullnode that has none; required then when not interactive. Setup does not change an existing one |
| `--ssl-provider=<zerossl\|letsencrypt\|file\|self-signed>` | How the gateway gets its certificate. Required when not interactive; `self-signed` only for fullnodes |
| `--zerossl-api-key-file=<path\|->` | File holding the ZeroSSL API key; required for ZeroSSL when not interactive |
| `--ssl-chain-file=<path>`, `--ssl-private-key-file=<path>` | Certificate chain and private key, for `--ssl-provider file`, in separate files (a chain file that holds a private key is refused); both required then when not interactive |
| `--accept-unmet-requirements` | Proceed although the system does not meet the requirements |

Only one flag can read stdin, and reading stdin makes the run non-interactive. Setup warns when a key file can be read by other users.

If setup fails or is interrupted, config.json is as it was, and certificate files the failed run installed are removed, so setup can simply run again. The exception is ZeroSSL: it saves the answers as soon as it creates a certificate, so a run that stops after that leaves them saved, and the next run prints the commands that finish it. Until the certificate is installed, `start` refuses to start Platform and prints the same commands. A Let's Encrypt certificate already issued is reused by the next run; do not reset Platform or run `dashmate ssl obtain` in between.

See [Updating Dashmate](../update.md#starting-platform-over) for the whole flow.

## Examples

```bash
# Set up a mainnet node
dashmate setup mainnet

# Set up a testnet node with debug logs enabled
dashmate setup testnet --debug-logs

# Set up a local development environment with 3 nodes
dashmate setup local --node-count=3

# Set up a local environment with custom block mining interval
dashmate setup local --miner-interval=1m
```

```bash
# Set up Platform again after `dashmate reset --platform --hard`
dashmate setup --platform testnet

# The same, unattended, with the node key read from stdin
dashmate setup --platform testnet --non-interactive --ssl-provider letsencrypt \
  --platform-node-key-file - < node-key-testnet
```

## Related Commands

- [reset](./reset.md) - Reset a node to its initial state
- [config create](./config/create.md) - Create a new configuration
- [group reset](./group/reset.md) - Reset all nodes in a group
