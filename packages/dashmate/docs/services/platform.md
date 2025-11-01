# Platform Services

The following diagram shows platform components and their dependencies:

```
                                 ┌───────────────┐
                                 │  Dash Core    │
                                 │  Blockchain   │
                                 └───────┬───────┘
                                         │
                   ┌─────────────────────┼─────────────────────────────┐
                   │                     │                             │
                   │        ┌────────────┼─────────────┐               │
                   │        │            │             │               │
          ┌────────▼────────┐    ┌───────▼───────┐     │      ┌───────▼───────┐
          │                 │    │               │     │      │               │
          │   Drive ABCI    │◄───┤  Tenderdash   │     │      │   DAPI API    │
          │  (Application   │    │  (Consensus   │     │      │  (JSON-RPC/   │
          │   Logic)        │    │   Engine)     │     │      │   gRPC API)   │
          │                 │    │               │     │      │               │
          └────────┬────────┘    └───────┬───────┘     │      └───────┬───────┘
                   │                     │             │              │
                   │                     │             │              │
                   │                     │             │      ┌───────▼───────┐
                   │                     │             │      │               │
                   │                     │             │      │  DAPI Core    │
                   │                     │             │      │   Streams     │
                   │                     │             │      │               │
                   │                     │             │      └───────┬───────┘
                   │                     │             │              │
                   │                     │             │              │
                   └─────────────────────┼─────────────┼──────────────┘
                                         │             │
                                 ┌───────▼─────────────▼───┐
                                 │                         │
                                 │         Gateway         │
                                 │                         │
                                 └─────────────────────────┘
                                           ▲
                                           │
                                           │
                                 ┌─────────┴─────────┐
                                 │                   │
                                 │      Clients      │
                                 │                   │
                                 └───────────────────┘
```

Gateway is the entry point for client applications, handling TLS termination and routing requests to the appropriate service:
* Core-related and some Platform API calls are forwarded to DAPI API
* Core streaming requests such as transactions and block headers are forwarded to DAPI Core Streams
* The most of the queries to Platform state are forwarded directly to Drive ABCI

For detailed information about the Gateway service, see the [Gateway Service documentation](./services/gateway.md).

DAPI, Drive and Tenderdash communicate with Core via RPC to access layer 1 blockchain data and LLMQ functionality.

DAPI API service is interacting with Tenderdash to retrieve layer 2 blockchain data and broadcast state transitions.
Also it requests proofs for state transitions from Drive ABCI.

## Drive (ABCI and Tenderdash)

```
            ┌─────────────────────────────────────┐
            │                                     │
            │          ┌─────────────┐            │
            │          │             │            │
            │          │  Drive      │ ◄──────────┼── User API Requests
            │          │  ABCI       │            │
            │          │             │            │
            │          └──────┬──────┘            │
            │                 ▼  ABCI             │
            │          ┌─────────────┐            │
            │          │             │
            │          │  Tenderdash │            │
            │          │             │◄───────────┼── Consensus Messages
            │          └─────────────┘            │
            └─────────────────────────────────────┘
```

Drive ABCI and Tenderdash works together to execute blocks and build the platform state.
They are using ABCI protocol to communicate with each other.

Drive ABCI is the application logic layer that processes state transitions and manages the platform state.

**Responsibilities**:
- Process platform state transitions
- Implement application logic
- Maintain state database
- Interface with Tenderdash consensus engine

**Communication**:
- Connects to Core for blockchain data via RPC
- Interfaces with Tenderdash for consensus
- Provides gRPC API for DAPI services
- Offers debugging tools (Tokio Console, GroveDB Visualizer)

Tenderdash is the consensus engine that provides Byzantine Fault Tolerant (BFT) consensus for the platform.
* Communicates with other Tenderdash nodes via P2P
* Provides RPC for DAPI API

| Service                   | Port Purpose         | Default Value | Config Path                                      | Default Host Binding | Host Config Path |
|---------------------------|----------------------|---------------|--------------------------------------------------|---------------------|-----------------|
| **Drive ABCI**            | ABCI                 | 26658         | (fixed internal)                                 | (internal)          | -               |
|                           | gRPC                 | 26670         | (fixed internal)                                 | (internal)          | -               |
|                           | Metrics              | 29090         | (exposed via PLATFORM_DRIVE_ABCI_METRICS_PORT)   | 127.0.0.1 (local)   | PLATFORM_DRIVE_ABCI_METRICS_HOST |
|                           | Tokio Console        | 6669          | `platform.drive.abci.tokioConsole.port`          | 127.0.0.1 (local)   | `platform.drive.abci.tokioConsole.host` |
|                           | GroveDB Visualizer   | 8083          | `platform.drive.abci.grovedbVisualizer.port`     | 127.0.0.1 (local)   | `platform.drive.abci.grovedbVisualizer.host` |
| **Drive Tenderdash**      | P2P                  | 26656         | `platform.drive.tenderdash.p2p.port`             | 0.0.0.0 (all)       | `platform.drive.tenderdash.p2p.host` |
|                           | RPC                  | 26657         | `platform.drive.tenderdash.rpc.port`             | 127.0.0.1 (local)   | `platform.drive.tenderdash.rpc.host` |
|                           | Metrics              | 26660         | `platform.drive.tenderdash.metrics.port`         | 127.0.0.1 (local)   | `platform.drive.tenderdash.metrics.host` |
|                           | pprof Debug          | 6060          | `platform.drive.tenderdash.pprof.port`           | 127.0.0.1 (local)   | (fixed)         |

## DAPI Services

### 5. DAPI API

**Service Name**: `dapi_api`

**Description**: The API service for Dash Platform that exposes JSON-RPC and gRPC interfaces.

**Responsibilities**:
- Provide JSON-RPC API
- Provide gRPC API
- Connect clients to platform services

**Communication**:
- Connects to Core via RPC
- Connects to Drive ABCI via gRPC
- Connects to Tenderdash via RPC
- Served to external clients via Gateway

### 6. DAPI Core Streams

**Service Name**: `dapi_core_streams`

**Description**: Provides streaming services for Dash Platform transactions.

**Responsibilities**:
- Stream blockchain events
- Filter transactions
- Provide real-time updates

**Communication**:
- Connects to Core via RPC and ZMQ
- Connects to Drive ABCI via gRPC
- Connects to Tenderdash via RPC
- Served to external clients via Gateway

**DAPI Exposed Ports and Configuration**:

| Service                   | Port Purpose         | Default Value | Config Path                                      | Default Host Binding | Host Config Path |
|---------------------------|----------------------|---------------|--------------------------------------------------|---------------------|-----------------|
| **DAPI API**              | JSON-RPC             | 3004          | (fixed internal)                                 | (internal)          | -               |
|                           | gRPC                 | 3005          | (fixed internal)                                 | (internal)          | -               |
| **DAPI Core Streams**     | gRPC Streaming       | 3006          | (fixed internal)                                 | (internal)          | -               |
| **rs-dapi (Rust)**        | Health + Metrics     | 9091 (mainnet), 19091 (testnet), 29091 (local) | `platform.dapi.rsDapi.metrics.port`              | 127.0.0.1           | `platform.dapi.rsDapi.metrics.host` |

The rs-dapi metrics server exposes health endpoints alongside Prometheus data on `/metrics` from the same port. Dashmate applies network-specific defaults (mainnet 9091, testnet 19091, local 29091) so multiple presets can coexist on a host without conflicts.
