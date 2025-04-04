# Core Configuration

The `core` section contains options for configuring the Dash Core node. The configuration is organized into several subsections, each controlling specific aspects of the Core node operation.

## Docker Options

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `core.docker.image` | Docker image for Dash Core | `dashpay/dashd:22` | `dashpay/dashd:latest` |
| `core.docker.commandArgs` | Additional command arguments for Core | `[]` | `["--reindex"]` |

## Core P2P Configuration

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

## Core RPC Configuration

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
  "core": {
    "rpc": {
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
  }
}
```

Each user has:
- `password`: RPC password for authentication
- `whitelist`: List of allowed RPC methods (null means all methods)
- `lowPriority`: Whether the user's requests have low priority and processed in a separate low priority RPC queue

## Core Spork Configuration

The `core.spork` section configures spork functionality. Sporks are a governance mechanism in Dash that allow network parameters to be changed without requiring a node software update.

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `core.spork.address` | Spork signing address | `null` | `"XYZ..."` |
| `core.spork.privateKey` | Spork signing private key | `null` | `"abc..."` |

Spork configuration notes:
- Only masternodes with designated spork keys can activate sporks
- The spork address and private key must be kept secure
- For most users running regular nodes, these fields should remain null

## Core Indexes Configuration

The `core.indexes` setting controls which blockchain indexes are maintained by the node, enabling various lookup functionalities.

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `core.indexes` | List of enabled indexes | `[]` | `["txindex", "addressindex", "spentindex"]` |

Available indexes include:
- `txindex`: Maintains an index of all transactions, allowing lookup by TXID
- `addressindex`: Maintains an index of all addresses and their transactions
- `spentindex`: Tracks spent transaction outputs
- `timestampindex`: Indexes blocks by timestamp

Enabling indexes:
- Increases disk space usage and initial sync time
- Improves query performance for specific operations
- Is required for certain applications like block explorers

## Core Masternode Configuration

The `core.masternode` section configures masternode settings, which are essential for running a masternode on the Dash network.

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `core.masternode.enable` | Enable masternode functionality | `false` | `true` |
| `core.masternode.operator.privateKey` | BLS private key for the masternode operator | `null` | `"6789abcdef..."` |

Masternode configuration example:

```json
"core.masternode": {
  "enable": true,
  "operator": {
    "privateKey": "6789abcdef..."
  }
}
```

## Core Miner Configuration

The `core.miner` section configures mining settings for Dash Core. This is primarily used in development and testing environments.

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `core.miner.enable` | Enable internal miner | `false` | `true` |
| `core.miner.address` | Address to receive mining rewards | `null` | `"XYZ..."` |
| `core.miner.interval` | Mining interval in milliseconds | `500` | `1000` |

Mining configuration example:

```json
"core.miner": {
  "enable": true,
  "address": "XYZ...",
  "interval": 1000
}
```

Important mining notes:
- Internal mining is primarily for development and testing
- Mining on mainnet typically requires specialized hardware (ASICs)
- The mining address should be a valid Dash address you control
- Mining interval controls the frequency of block generation attempts
- In local development networks, mining is used to generate blocks for testing

## Core Log Configuration

The `core.log` section controls how Dash Core logs its activities, which is essential for monitoring and troubleshooting.

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `core.log.filePath` | Path to log file (null for stdout) | `null` | `"/var/log/dashd.log"` |
| `core.log.debug` | Extensive debug logging configuration | `{ }` | See below |

Debug logging can be configured with the following options:

```json
"core.log.debug": {
  "enabled": true,
  "ips": false,
  "sourceLocations": false,
  "threadNames": false,
  "timeMicros": false,
  "includeOnly": [],
  "exclude": [],
  "categories": ["net", "mempool", "governance", "masternode"]
}
```

The debug logging options include:

- `enabled`: Enables or disables debug logging (true/false)
- `ips`: When true, includes IP addresses in debug logs (disabled by default for privacy)
- `sourceLocations`: When true, shows source code file and line information in logs
- `threadNames`: When true, includes thread names in log entries for multi-threading analysis
- `timeMicros`: When true, shows microsecond precision in timestamps for detailed timing analysis
- `includeOnly`: If specified, only these categories will be logged (overrides categories)
- `exclude`: Categories to exclude from logging
- `categories`: Array of logging categories to enable (described below)

These options allow you to fine-tune logging output based on your needs:
- For privacy-sensitive environments, keep `ips` disabled
- For development and debugging, enable `sourceLocations` to pinpoint issues
- For performance analysis, enable `timeMicros` and `threadNames`
- Use `includeOnly` and `exclude` to focus logging on specific components

### Available Debug Categories

You can enable specific debugging categories to focus on particular aspects of Dash Core:

**Network-related categories:**
- `net`: Network activity, P2P message handling, and peer connections
- `addrman`: Address manager operations and peer address management
- `cmpct`: Compact block relay and processing
- `http`: HTTP server activity for RPC over HTTP
- `libevent`: LibEvent library operations (low-level event handling)
- `tor`: Tor connection and routing if Tor is enabled
- `zmq`: ZeroMQ notification interface events

**Core functionality categories:**
- `mempool`: Memory pool operations, transaction validation and fee calculation
- `mempoolrej`: Memory pool rejection reasons - why transactions were rejected
- `coindb`: Coin database activity (UTXO set management)
- `db`: General database operations
- `leveldb`: LevelDB database operations (low-level storage)
- `reindex`: Block reindexing process details during reindexing operations
- `rpc`: Remote procedure call activity, methods and parameters
- `selectcoins`: Coin selection algorithm details for transaction creation
- `estimatefee`: Fee estimation mechanisms
- `bench`: Benchmarking information
- `txindex`: Transaction index operations
- `prune`: Block and data pruning operations
- `rand`: Random number generation (useful for security auditing)

**Masternode and governance categories:**
- `masternode`: Masternode operations, payments, and status updates
- `mnpayments`: Masternode payment verification and voting
- `mnbudget`: Budget system operations (legacy governance system)
- `mnsync`: Masternode synchronization process details
- `gobject`: Governance object handling, proposals and votes
- `privsend`: PrivateSend operations and mixing
- `instantsend`: InstantSend transaction processing and locking
- `coinjoin`: CoinJoin mixing process (privacy feature)

**Quorum and consensus categories:**
- `governance`: Governance proposals and voting process
- `spork`: Spork activity and updates (network-wide settings)
- `llmq`: Long-Living Masternode Quorum activities and general operations
- `llmq-dkg`: Distributed Key Generation in LLMQ (key creation phase)
- `llmq-sigs`: LLMQ signature operations (threshold signing)
- `llmq-inst`: LLMQ InstantSend operations
- `llmq-chainlocks`: LLMQ ChainLocks operations
- `chainlocks`: ChainLocks validation and processing
- `quorum`: General quorum-related activities
- `signing`: Signature creation and verification operations

For complex debugging scenarios, you might want to:
- Start with a few key categories related to your issue
- Add more specific categories if needed to narrow down the problem
- Be aware that some categories (like `net`) can be extremely verbose

## Core Insight Configuration

Insight provides a block explorer for Dash Core:

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `core.insight.enabled` | Enable Insight API | `false` | `true` |
| `core.insight.ui.enabled` | Enable Insight UI | `false` | `true` |
| `core.insight.port` | Port for Insight API/UI | `3001` | `3002` |


## Devnet Configuration

For custom devnets (custom blockchain networks for development):

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `core.devnet.name` | Custom devnet name | `null` | `"devnet-1"` |
| `core.devnet.minimumDifficultyBlocks` | Number of blocks with minimum difficulty | `0` | `10` |
| `core.devnet.powTargetSpacing` | Block time target in seconds | `150` | `60` |
| `core.devnet.llmq` | LLMQ configurations for different purposes | Object | See below |

LLMQ devnet configuration example:
```javascript
"core.devnet.llmq": {
  "llmq_50_60": {
    "size": 50,
    "threshold": 30,
    "lifetime": 24
  },
  "chainLocks": "llmq_50_60",
  "instantSend": "llmq_50_60",
  "platform": "llmq_50_60"
}
```
