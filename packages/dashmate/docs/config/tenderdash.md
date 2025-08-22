# Tenderdash Configuration

Tenderdash is the consensus engine for Dash Platform. Its configuration is divided into several functional areas.

## Mode

Tenderdash can operate in different modes depending on the node's role in the network:

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.drive.tenderdash.mode` | Node operation mode | `validator` | `full`, `seed` |

- `validator`: A node that participates in consensus by proposing and voting on blocks
- `full`: A node that verifies all blocks and transactions but doesn't participate in consensus
- `seed`: A node that helps other nodes discover peers but doesn't store the full state

## Docker

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.drive.tenderdash.docker.image` | Docker image for Tenderdash | `dashpay/tenderdash:1` | `dashpay/tenderdash:latest` |

## P2P

These settings control the peer-to-peer network for Tenderdash nodes:

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.drive.tenderdash.p2p.port` | P2P port for tenderdash | `26656` | `26657` |
| `platform.drive.tenderdash.p2p.host` | Host binding for P2P | `0.0.0.0` | `127.0.0.1` |
| `platform.drive.tenderdash.p2p.persistentPeers` | Array of peers to maintain persistent connections with | `[]` | See example below |
| `platform.drive.tenderdash.p2p.seeds` | Array of seed nodes for peer discovery | `[]` | See example below |
| `platform.drive.tenderdash.p2p.flushThrottleTimeout` | Throttle timeout for P2P data | `100ms` | `200ms` |
| `platform.drive.tenderdash.p2p.maxPacketMsgPayloadSize` | Maximum P2P message size | `10240` | `20480` |
| `platform.drive.tenderdash.p2p.sendRate` | P2P send rate limit | `5120000` | `10240000` |
| `platform.drive.tenderdash.p2p.recvRate` | P2P receive rate limit | `5120000` | `10240000` |
| `platform.drive.tenderdash.p2p.maxConnections` | Maximum P2P connections | `64` | `128` |
| `platform.drive.tenderdash.p2p.maxOutgoingConnections` | Maximum outgoing P2P connections | `30` | `60` |

Example of P2P peers configuration:
```json
{
  "persistentPeers": [
    {
      "id": "8c379d4d3b9995c712665dc9a9414dbde5b30483",
      "host": "172.16.0.10",
      "port": 26656
    }
  ],
  "seeds": [
    {
      "id": "7b1c1c5409ac7b8fd88d53f0f3c7cac3c7fdb8e5",
      "host": "seed.dash.org",
      "port": 26656
    }
  ]
}
```

These settings affect how Tenderdash nodes communicate with each other:
- Host and port settings determine the network endpoint for P2P communication
- Rate limits control bandwidth usage
- Connection limits determine network resilience and resource usage
- Seeds and persistent peers help with network discovery and stability

## RPC

These settings control the RPC interface for Tenderdash:

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.drive.tenderdash.rpc.port` | RPC port for tenderdash | `26657` | `26658` |
| `platform.drive.tenderdash.rpc.host` | Host binding for RPC | `127.0.0.1` | `0.0.0.0` |
| `platform.drive.tenderdash.rpc.maxOpenConnections` | Maximum RPC connections | `900` | `1800` |
| `platform.drive.tenderdash.rpc.timeoutBroadcastTx` | Timeout for broadcasting transactions | `0` | `30s` |

The RPC interface is used for:
- Querying blockchain state
- Submitting transactions
- Fetching network status

## Metrics and Profiling

These settings control monitoring and profiling tools:

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.drive.tenderdash.metrics.enabled` | Enable metrics | `false` | `true` |
| `platform.drive.tenderdash.metrics.host` | Host binding for metrics | `127.0.0.1` | `0.0.0.0` |
| `platform.drive.tenderdash.metrics.port` | Metrics port | `26660` | `26661` |
| `platform.drive.tenderdash.pprof.enabled` | Enable profiling server | `false` | `true` |
| `platform.drive.tenderdash.pprof.port` | Profiling server port | `6060` | `6061` |

- Metrics: Provides performance and health data in Prometheus format
- pprof: Go profiling tool for identifying performance bottlenecks

## Mempool

The mempool handles pending transactions before they are added to blocks:

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.drive.tenderdash.mempool.size` | Maximum number of transactions in mempool | `5000` | `10000` |
| `platform.drive.tenderdash.mempool.cacheSize` | Size of mempool cache | `10000` | `20000` |
| `platform.drive.tenderdash.mempool.maxTxsBytes` | Maximum transaction size in bytes | `1048576` | `2097152` |
| `platform.drive.tenderdash.mempool.timeoutCheckTx` | Timeout for checking transactions | `1s` | `2s` |
| `platform.drive.tenderdash.mempool.txEnqueueTimeout` | Timeout for enqueueing transactions | `1s` | `2s` |
| `platform.drive.tenderdash.mempool.txSendRateLimit` | Rate limit for sending transactions | `0` | `1000` |
| `platform.drive.tenderdash.mempool.txRecvRateLimit` | Rate limit for receiving transactions | `0` | `1000` |
| `platform.drive.tenderdash.mempool.maxConcurrentCheckTx` | Maximum number of concurrent tx checks | `0` | `10` |
| `platform.drive.tenderdash.mempool.ttlDuration` | Time-to-live duration for transactions | `0s` | `60s` |
| `platform.drive.tenderdash.mempool.ttlNumBlocks` | Time-to-live in number of blocks | `0` | `10` |

Mempool configuration example:
```json
{
  "mempool": {
    "size": 5000,
    "cacheSize": 10000,
    "maxTxsBytes": 1048576,
    "timeoutCheckTx": "1s",
    "txEnqueueTimeout": "1s",
    "txSendRateLimit": 0,
    "txRecvRateLimit": 0,
    "maxConcurrentCheckTx": 0,
    "ttlDuration": "0s",
    "ttlNumBlocks": 0
  }
}
```

## Consensus

These settings control the consensus mechanism:

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.drive.tenderdash.consensus.createEmptyBlocks` | Create blocks even without transactions | `true` | `false` |
| `platform.drive.tenderdash.consensus.createEmptyBlocksInterval` | Empty block creation interval | `0s` | `10s` |
| `platform.drive.tenderdash.consensus.peer.gossipSleepDuration` | Sleep time between gossip broadcast messages | `1s` | `2s` |
| `platform.drive.tenderdash.consensus.peer.queryMaj23SleepDuration` | Sleep time between each query for vote messages | `2s` | `4s` |
| `platform.drive.tenderdash.consensus.unsafeOverride.propose.timeout` | Timeout for block proposal phase | `3s` | `5s` |
| `platform.drive.tenderdash.consensus.unsafeOverride.propose.delta` | Delta for propose phase | `500ms` | `1s` |
| `platform.drive.tenderdash.consensus.unsafeOverride.vote.timeout` | Timeout for vote phase | `1s` | `2s` |
| `platform.drive.tenderdash.consensus.unsafeOverride.vote.delta` | Delta for vote phase | `500ms` | `1s` |
| `platform.drive.tenderdash.consensus.unsafeOverride.commit.timeout` | Timeout for commit phase | `1s` | `2s` |
| `platform.drive.tenderdash.consensus.unsafeOverride.commit.bypass` | Whether to bypass commit phase | `false` | `true` |

Consensus configuration example:
```json
{
  "consensus": {
    "createEmptyBlocks": true,
    "createEmptyBlocksInterval": "0s",
    "peer": {
      "gossipSleepDuration": "100ms",
      "queryMaj23SleepDuration": "2s"
    },
    "unsafeOverride": {
      "propose": {
        "timeout": "3s",
        "delta": "500ms"
      },
      "vote": {
        "timeout": "1s",
        "delta": "500ms"
      },
      "commit": {
        "timeout": "1s",
        "bypass": false
      }
    }
  }
}
```

The `unsafeOverride` timeouts can be set to `null` to use default values.

## Logging

Configure how Tenderdash logs are captured and formatted:

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.drive.tenderdash.log.level` | Log verbosity level | `info` | `debug`, `trace`, `warn`, `error` |
| `platform.drive.tenderdash.log.format` | Log output format | `plain` | `json` |
| `platform.drive.tenderdash.log.path` | Path to log file (null for stdout only) | `null` | `/var/log/tenderdash.log` |

Example log configuration:
```json
{
  "log": {
    "level": "info",
    "format": "plain",
    "path": null
  }
}
```

## Node Configuration

These settings control node identity:

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.drive.tenderdash.node.id` | Node ID (derived from node key) | `null` | `"8c379d4d3b9995c712665dc9a9414dbde5b30483"` |
| `platform.drive.tenderdash.node.key` | Private key for node identity | `null` | Base64-encoded string |
| `platform.drive.tenderdash.moniker` | Human-readable node name | `null` | `"my-validator-node"` |

Node configuration is typically auto-generated during setup.

## Tenderdash Genesis Configuration

The genesis configuration defines the initial state of the blockchain:

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.drive.tenderdash.genesis.genesis_time` | Block chain start time | ISO date string | `"2023-01-01T00:00:00Z"` |
| `platform.drive.tenderdash.genesis.chain_id` | Unique chain identifier | String | `"dash-platform-testnet"` |
| `platform.drive.tenderdash.genesis.validators` | Initial validator set | Array | See advanced docs |

Genesis configuration is typically auto-generated based on other settings.
