# Core Configuration

The `core` section contains options for configuring the Dash Core node. The configuration is organized into several subsections, each controlling specific aspects of the Core node operation.

## Docker

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `core.docker.image` | Docker image for Dash Core | `dashpay/dashd:22` | `dashpay/dashd:latest` |
| `core.docker.commandArgs` | Additional command arguments for Core | `[]` | `["--reindex"]` |

With `core.docker.commandArgs` you can pass additional command-line arguments to the Dash Core Docker container. This is useful for customizing the behavior of the Core node.

## P2P

The `core.p2p` section controls the peer-to-peer network settings for Dash Core, which are essential for node communication within the Dash network.

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `core.p2p.port` | Port for peer-to-peer connections | `9999` | `19999` |
| `core.p2p.host` | Host binding for P2P connections (0.0.0.0 allows connections from any IP) | `0.0.0.0` | `127.0.0.1` |
| `core.p2p.seeds` | List of seed nodes for initial P2P connections | `[]` | `["1.2.3.4:9999"]` |

These settings control how your Dash Core node connects to the network:
- The P2P port is used for communication with other Dash nodes
- Setting the host to 0.0.0.0 makes your node accessible from any network interface
- Seed nodes help your node discover other peers on the network during initial startup

## RPC

The `core.rpc` section configures the Remote Procedure Call interface, which allows other applications to interact with your Dash Core node.

| Option | Description                                                     | Default         | Example              |
|--------|-----------------------------------------------------------------|-----------------|----------------------|
| `core.rpc.port` | Port for RPC server                                             | `9998`          | `19998`              |
| `core.rpc.host` | Host binding for RPC (127.0.0.1 restricts to local connections) | `127.0.0.1`     | `0.0.0.0`            |
| `core.rpc.allowIps` | IP addresses allowed to connect to RPC                          | `['127.0.0.1']` | `['192.168.0.0/16']` |
| `core.rpc.users` | Core RPC Users                                                  | `{}`            |                      |

RPC settings are crucial for security and functionality:
- The default settings only allow local connections to RPC for security
- To allow remote access, set host to 0.0.0.0 and add remote IPs to allowIps
- Remote RPC access should be configured carefully to prevent unauthorized access

The `core.rpc.users` section defines RPC users and their permissions:

```json
{
  "users": {
    "dashmate": {
      "password": "rpcpassword",
      "whitelist": null,
      "lowPriority": false
    },
    "dapi": {
      "password": "rpcpassword",
      "whitelist": ["getbestblockhash", "getblockhash", "sendrawtransaction", ...],
      "lowPriority": true
    },
    // More users...
  }
}
```

Each user has:
- `password`: RPC password for authentication
- `whitelist`: List of allowed RPC methods (null means all methods)
- `lowPriority`: Whether the user's requests have low priority and processed in a separate low priority RPC queue

## ZMQ

The `core.zmq` section configures the ZeroMQ notification interface, which provides real-time notifications for blockchain events.

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `core.zmq.port` | Port for ZMQ notifications | `29998` (mainnet),`39998` (testnet), `49998` (local) | `30998` |
| `core.zmq.host` | Host binding for Docker port mapping | `127.0.0.1` | `0.0.0.0` |

ZMQ settings control real-time blockchain event notifications:
- ZMQ provides low-latency notifications for blocks, transactions, and other blockchain events
- **host**: Controls Docker port exposure:
  - `127.0.0.1`: ZMQ port exposed only on localhost (local machine access)
  - `0.0.0.0`: ZMQ port exposed on all interfaces (public internet access - use with caution)
- **port**: The port number for ZMQ notifications. Dashmate offsets the default to prevent clashes between environments (`29998` mainnet, `39998` testnet, `49998` local presets).
- DAPI uses ZMQ to receive real-time blockchain data for streaming to clients
- ZMQ notifications include raw transactions, blocks, instantlocks, and chainlocks
- ZMQ is always enabled in Dash Core as it's used by internal components

**Security Note**: Be cautious when setting `host` to `0.0.0.0` as it makes ZMQ publicly accessible.

## Sporks

The `core.spork` section configures spork functionality. Sporks are a governance mechanism in Dash that allow network parameters to be changed without requiring a node software update.

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `core.spork.address` | Spork signing address | `null` | `"XYZ..."` |
| `core.spork.privateKey` | Spork signing private key | `null` | `"abc..."` |

Spork configuration notes:
- Only masternodes with designated spork keys can activate sporks
- The spork address and private key must be kept secure
- For most users running regular nodes, these fields should remain null

## Indexes

The `core.indexes` setting controls which blockchain indexes are maintained by the node, enabling various lookup functionalities.

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `core.indexes` | List of enabled indexes | `[]` | `["tx", "address", "spent", "timestamp"]` |

Available indexes include:
- `tx`: Maintains an index of all transactions, allowing lookup by TXID
- `address`: Maintains an index of all addresses and their transactions
- `spent`: Tracks spent transaction outputs
- `timestamp`: Indexes blocks by timestamp

Enabling indexes:
- Increases disk space usage and initial sync time
- Enables or improves query performance for specific operations
- Is required for certain applications like block explorers

Note: Some features like `platform.enable`, `core.masternode.enable`, and `core.insight.enabled` automatically add required indexes.

## Masternode

The `core.masternode` section configures masternode settings, which are essential for running a masternode on the Dash network.

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `core.masternode.enable` | Enable masternode functionality | `false` | `true` |
| `core.masternode.operator.privateKey` | BLS private key for the masternode operator | `null` | `"6789abcdef..."` |

Masternode configuration example:

```json
{
  "masternode": {
    "enable": true,
    "operator": {
      "privateKey": "6789abcdef..."
    }
  }
}
```

## Miner

The `core.miner` section configures mining settings for Dash Core. This is primarily used in development and testing environments.

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `core.miner.enable` | Enable internal miner | `false` | `true` |
| `core.miner.address` | Address to receive mining rewards | `null` | `"XYZ..."` |
| `core.miner.interval` | Mining interval in milliseconds | `500` | `1000` |

Mining configuration example:

```json
{
  "miner": {
    "enable": true,
    "address": "XYZ...",
    "interval": 1000
  }
}
```

## Logging

The `core.log` section controls how Dash Core logs its activities, which is essential for monitoring and troubleshooting.

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `core.log.filePath` | Path to log file (null for stdout) | `null` | `"/var/log/dashd.log"` |
| `core.log.debug` | Extensive debug logging configuration | `{ }` | See below |

Debug logging can be configured with the following options:

```json
{
  "debug": {
    "enabled": true,
    "ips": false,
    "sourceLocations": false,
    "threadNames": false,
    "timeMicros": false,
    "includeOnly": [
      "net",
      "mempool",
      "governance",
      "masternode"
    ],
    "exclude": []
  }
}
```

The debug logging options include:

- `enabled`: Enables or disables debug logging (true/false)
- `ips`: When true, includes IP addresses in debug logs (disabled by default for privacy)
- `sourceLocations`: When true, shows source code file and line information in logs
- `threadNames`: When true, includes thread names in log entries for multi-threading analysis
- `timeMicros`: When true, shows microsecond precision in timestamps for detailed timing analysis
- `includeOnly`: Array of categories to include in logging (leave empty to log all except excluded)
- `exclude`: Array of categories to exclude from logging

These options allow you to fine-tune logging output based on your needs:
- For privacy-sensitive environments, keep `ips` disabled
- For development and debugging, enable `sourceLocations` to pinpoint issues
- For performance analysis, enable `timeMicros` and `threadNames`
- Use `includeOnly` and `exclude` to focus debugging on specific components

### Available Debug Categories

You can select specific debugging categories to focus on particular aspects of Dash Core by adding them to the `includeOnly` array:

**Available debug categories include:**

- `net`: Network activity and peer connections
- `tor`: Tor connection and routing
- `mempool`: Memory pool operations and transaction validation
- `http`: HTTP server activity for RPC over HTTP
- `bench`: Benchmarking information
- `zmq`: ZeroMQ notification interface
- `walletdb`: Wallet database operations
- `rpc`: Remote procedure call activity
- `estimatefee`: Fee estimation mechanisms
- `addrman`: Address manager operations and peer management
- `selectcoins`: Coin selection algorithm
- `reindex`: Block reindexing process
- `cmpctblock` (or `cmpct`): Compact block relay
- `rand`: Random number generation
- `prune`: Block and data pruning operations
- `proxy`: Network proxy operations
- `mempoolrej`: Memory pool rejection reasons
- `libevent`: LibEvent library operations
- `coindb`: Coin database activity (UTXO set)
- `qt`: Qt GUI related operations (when applicable)
- `leveldb`: LevelDB database operations
- `chainlocks`: ChainLocks validation and processing
- `gobject`: Governance object handling
- `instantsend`: InstantSend transaction processing
- `llmq`: Long-Living Masternode Quorum activities
- `llmq-dkg`: Distributed Key Generation in LLMQ
- `llmq-sigs`: LLMQ signature operations
- `mnpayments`: Masternode payment verification
- `mnsync`: Masternode synchronization process
- `coinjoin`: CoinJoin mixing process
- `spork`: Spork activity and updates
- `netconn`: Network connection details

**Special debugging options:**
- You can use `includeOnly` to specify only the categories you want to debug
- Leave `includeOnly` empty and use `exclude` to log everything except specific categories

For complex debugging scenarios, you might want to:
- Start with a few key categories related to your issue
- Add more specific categories if needed to narrow down the problem
- Be aware that some categories (like `net`) can be extremely verbose

## Insight

Insight provides a block explorer for Dash Core:

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `core.insight.enabled` | Enable Insight API | `false` | `true` |
| `core.insight.ui.enabled` | Enable Insight UI | `false` | `true` |
| `core.insight.port` | Port for Insight API/UI | `3001` | `3002` |


## Devnet

For custom devnets (custom blockchain networks for development):

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `core.devnet.name` | Custom devnet name | `null` | `"devnet-1"` |
| `core.devnet.minimumDifficultyBlocks` | Number of blocks with minimum difficulty | `0` | `10` |
| `core.devnet.powTargetSpacing` | Block time target in seconds | `150` | `60` |
| `core.devnet.llmq` | LLMQ configurations for different purposes | Object | See below |

LLMQ devnet configuration example:

```json
{
  "llmq": {
    "chainLocks": "llmq_50_60",
    "instantSend": "llmq_50_60",
    "platform": "llmq_50_60",
    "mnhf": "llmq_50_60"
  }
}
```

Available LLMQ types:
 - llmq_devnet
 - llmq_devnet_dip0024
 - llmq_devnet_platform 
 - llmq_50_60
 - llmq_60_75
 - llmq_400_60
 - llmq_400_85
 - llmq_100_67
 - llmq_25_67
