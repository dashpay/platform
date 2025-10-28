# Core service

The Core service provides Dash blockchain functionality:

```
            ┌─────────────────────────────────────┐
            │                                     │
            │          ┌─────────────┐            │
            │          │             │            │
            │          │  Core       │            │
            │          │  (Dash      │            │
            │          │  Blockchain)│            │
            │          │             │            │
            │          └──────┬──────┘            │
            │                 │                   │
            │                 │                   │
            │                 ▼                   │
            │          ┌─────────────┐            │
            │          │             │            │
            │          │  Insight    │◄───────────┼── User Browser
            │          │  API/UI     │            │   Requests
            │          │             │            │
            │          └─────────────┘            │
            │                                     │
            └─────────────────────────────────────┘
```

Read more about Dash Core: https://docs.dash.org/en/stable/docs/core/index.html

Core exposes P2P and RPC ports for communication with other services. It also provides ZMQ notifications for real-time updates.

| Service              | Port Purpose | Default Value | Config Path          | Default Host Binding | Host Config Path  |
|----------------------|--------------|---------------|---------------------|----------------------|------------------|
| **Core**             | P2P          | 9999          | `core.p2p.port`     | 0.0.0.0 (all)        | `core.p2p.host`  |
|                      | RPC          | 9998          | `core.rpc.port`     | 127.0.0.1 (local)    | `core.rpc.host`  |
|                      | ZMQ          | 29998 (mainnet), 39998 (testnet), 49998 (local) | `core.zmq.port`     | 127.0.0.1 (local)    | `core.zmq.host`  |
| **Insight API/UI**   | HTTP         | 3001          | `core.insight.port` | 127.0.0.1 (local)    | (fixed)           |

To interact with Core RPC use `dashmate core cli` command.

Optionally, you can enable the Insight API and UI to provide a web interface for exploring the blockchain. The `core.insight.enabled` configuration option enables Insight API and `core.insight.ui.enabled` enables block explorer.

**Responsibilities**:
- Process blockchain transactions
- Handle masternode operations
- Manage the Layer 1 consensus
- Provide RPC services to other components

**Communication**:
- Provides RPC to DAPI API, DAPI Core Streams, and Drive ABCI
- Communicates with outside world via P2P port
- Provides notifications via ZMQ
