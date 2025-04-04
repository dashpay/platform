# Tenderdash Configuration

Tenderdash is the consensus engine for Dash Platform. Its configuration is divided into several functional areas.

## Tenderdash Docker Configuration

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.drive.tenderdash.docker.image` | Docker image for Tenderdash | `dashpay/tenderdash:1` | `dashpay/tenderdash:latest` |

## Tenderdash P2P Configuration

These settings control the peer-to-peer network for Tenderdash nodes:

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.drive.tenderdash.p2p.port` | P2P port for tenderdash | `26656` | `26657` |
| `platform.drive.tenderdash.p2p.host` | Host binding for P2P | `0.0.0.0` | `127.0.0.1` |
| `platform.drive.tenderdash.p2p.flushThrottleTimeout` | Throttle timeout for P2P data | `100ms` | `200ms` |
| `platform.drive.tenderdash.p2p.maxPacketMsgPayloadSize` | Maximum P2P message size | `10240` | `20480` |
| `platform.drive.tenderdash.p2p.sendRate` | P2P send rate limit | `5120000` | `10240000` |
| `platform.drive.tenderdash.p2p.recvRate` | P2P receive rate limit | `5120000` | `10240000` |
| `platform.drive.tenderdash.p2p.maxConnections` | Maximum P2P connections | `64` | `128` |
| `platform.drive.tenderdash.p2p.maxOutgoingConnections` | Maximum outgoing P2P connections | `30` | `60` |

These settings affect how Tenderdash nodes communicate with each other:
- Host and port settings determine the network endpoint for P2P communication
- Rate limits control bandwidth usage
- Connection limits determine network resilience and resource usage

## Tenderdash RPC Configuration

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

## Tenderdash Metrics and Profiling Configuration

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

## Tenderdash Mempool Configuration

The mempool handles pending transactions before they are added to blocks:

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.drive.tenderdash.mempool.size` | Maximum number of transactions in mempool | `5000` | `10000` |
| `platform.drive.tenderdash.mempool.cacheSize` | Size of mempool cache | `10000` | `20000` |
| `platform.drive.tenderdash.mempool.maxTxBytes` | Maximum transaction size in bytes | `1048576` | `2097152` |

Mempool configuration example:
```javascript
"platform.drive.tenderdash.mempool": {
  "size": 5000,
  "cacheSize": 10000,
  "maxTxBytes": 1048576
}
```

## Tenderdash Consensus Configuration

These settings control the consensus mechanism:

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.drive.tenderdash.consensus.createEmptyBlocks` | Create blocks even without transactions | `true` | `false` |
| `platform.drive.tenderdash.consensus.createEmptyBlocksInterval` | Empty block creation interval | `0s` | `10s` |
| `platform.drive.tenderdash.consensus.timeoutPropose` | Block proposal timeout | `3s` | `5s` |
| `platform.drive.tenderdash.consensus.timeoutPrevote` | Prevote step timeout | `1s` | `2s` |
| `platform.drive.tenderdash.consensus.timeoutPrecommit` | Precommit step timeout | `1s` | `2s` |
| `platform.drive.tenderdash.consensus.timeoutCommit` | Commit step timeout | `1s` | `2s` |

Consensus configuration example:
```javascript
"platform.drive.tenderdash.consensus": {
  "createEmptyBlocks": true,
  "createEmptyBlocksInterval": "0s",
  "timeoutPropose": "3s",
  "timeoutPrevote": "1s", 
  "timeoutPrecommit": "1s",
  "timeoutCommit": "1s"
}
```

## Tenderdash Genesis Configuration

The genesis configuration defines the initial state of the blockchain:

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.drive.tenderdash.genesis.genesis_time` | Block chain start time | ISO date string | `"2023-01-01T00:00:00Z"` |
| `platform.drive.tenderdash.genesis.chain_id` | Unique chain identifier | String | `"dash-platform-testnet"` |
| `platform.drive.tenderdash.genesis.validators` | Initial validator set | Array | See advanced docs |

Genesis configuration is typically auto-generated based on other settings.