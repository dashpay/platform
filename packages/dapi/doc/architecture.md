# DAPI Architecture

This document explains the high-level architecture of DAPI, its components, and how they interact with the Dash ecosystem.

## Overview

DAPI (Decentralized API) serves as the gateway to the Dash network, providing access to both Dash Core blockchain functionality and Dash Platform (Evolution) features.
Unlike traditional centralized APIs, DAPI is designed to run on the Dash masternode network, ensuring high availability and censorship resistance.

## Architecture Diagram

```
┌─────────────────────────────────────────────────────┐
│                    DAPI                             │
│                                                     │
│  ┌───────────────────┐    ┌─────────────────────┐   │
│  │                   │    │                     │   │
│  │    API Process    │    │  Core Streams       │   │
│  │    (api.js)       │    │  Process            │   │
│  │                   │    │  (core-streams.js)  │   │
│  │  - Core endpoints │    │                     │   │
│  │  - Platform       │    │  - Block streaming  │   │
│  │    endpoints      │    │  - TX streaming     │   │
│  │  - JSON-RPC       │    │  - Masternode list  │   │
│  │                   │    │    streaming        │   │
│  └───────┬───────────┘    └─────────┬───────────┘   │
│          │                          │               │
└──────────┼──────────────────────────┼───────────────┘
           │                          │
           ▼                          ▼
┌──────────────────────┐  ┌─────────────────────────┐
│                      │  │                         │
│     Dash Core        │  │     Drive & Tenderdash  │
│                      │  │                         │
│  - Blockchain        │  │  - Platform State       │
│  - Mempool           │  │  - Data Contracts       │
│  - Wallet            │  │  - Identities           │
│  - P2P Network       │  │  - Documents            │
│                      │  │                         │
└──────────────────────┘  └─────────────────────────┘
```

## Key Components

### API Process

The API process is the main entry point for DAPI. It handles the basic gRPC and JSON-RPC endpoints, including both Core and Platform functionality.

#### Responsibilities:

- **JSON-RPC Server**: Serves legacy JSON-RPC endpoints
- **Core gRPC Endpoints**: Serves Core blockchain endpoints
- **Platform gRPC Endpoints**: Serves Platform (Evolution) endpoints

#### Connections:

- **Dash Core**: Connects to Core via RPC and ZMQ
- **Drive**: Connects to Drive via gRPC
- **Tenderdash**: Connects to Tenderdash via RPC and WebSocket

#### Startup Sequence:

1. Load configuration
2. Connect to Dash Core's ZMQ interface
3. Initialize Platform and Drive clients
4. Connect to Tenderdash WebSocket
5. Start JSON-RPC server
6. Start gRPC server with Core and Platform handlers

#### Endpoints Served:

- **Core gRPC Endpoints**:
   - `getBestBlockHeight`
   - `getBlockchainStatus`
   - `getTransaction`
   - `broadcastTransaction`

- **Platform gRPC Endpoints**:
   - `broadcastStateTransition`
   - `waitForStateTransitionResult`
   - `getConsensusParams`
   - `getStatus`
   - And various unimplemented endpoints

- **JSON-RPC Endpoints**:
   - `getBestBlockHash`
   - `getBlockHash`

#### Dependencies:

- Dash Core (via RPC and ZMQ)
- Drive (via gRPC)
- Tenderdash (via RPC and WebSocket)

#### How to run

```bash
node scripts/api.js
```

### Core Streams Process

The Core Streams process handles streaming data from the Dash blockchain, including blocks, transactions, and masternode lists.

#### Responsibilities:

- **Transaction Streaming**: Stream transactions matching bloom filters
- **Block Header Streaming**: Stream block headers and chain locks
- **Masternode List Streaming**: Stream masternode list updates

#### Connections:

- **Dash Core**: Connects to Core via RPC and ZMQ
- **Chain Data Provider**: Maintains a cache of block headers

#### Startup Sequence:

1. Load configuration
2. Connect to Dash Core's ZMQ interface
3. Initialize bloom filter emitter collection
4. Set up event listeners for ZMQ events
5. Initialize chain data provider and block headers cache
6. Initialize masternode list sync
7. Start gRPC server with streaming handlers

#### Endpoints Served:

- **Stream gRPC Endpoints**:
   - `subscribeToTransactionsWithProofs`
   - `subscribeToBlockHeadersWithChainLocks`
   - `subscribeToMasternodeList`

#### Dependencies:
- Dash Core (via RPC and ZMQ)

### Communication

Both API and Core Streams components operate independently and do not directly communicate with each other.
Instead, they both connect to the same underlying services (Dash Core, Drive, Tenderdash) to provide their respective functionality.

## Interfaces

DAPI provides two main interfaces for client interaction:

### gRPC Interface

The primary and recommended interface, using Protocol Buffers for efficient, typed communication. Supports:
- Request/response endpoints
- Server-side streaming endpoints
- Strong typing and versioning

### JSON-RPC Interface

A legacy interface provided for compatibility with existing tools. Features:
- Compatible with the JSON-RPC 2.0 specification
- Limited subset of Dash Core's JSON-RPC functionality
- No streaming capabilities

## Connection to the Dash Ecosystem

DAPI connects to several underlying services:

### Dash Core

DAPI communicates with Dash Core in two ways:
- **RPC Interface**: For direct blockchain queries and commands
- **ZMQ Interface**: For real-time notifications of new blocks, transactions, and chainlocks

### Drive

For Platform functionality, DAPI connects to Drive via gRPC. Drive is responsible for:
- Processing and validating state transitions
- Maintaining Platform state (data contracts, documents, identities)
- Providing proofs for Platform operations

### Tenderdash

DAPI connects to Tenderdash (a modified version of Tendermint) which serves as the consensus engine for Dash Platform. Connections include:
- **RPC Interface**: For querying Platform chain state
- **WebSocket Interface**: For subscribing to real-time Platform events

## Process Flow Examples

### Example 1: Querying a transaction

1. Client sends gRPC request to `getTransaction` endpoint
2. API process receives request
3. API process queries Dash Core via RPC
4. Dash Core returns transaction data
5. API process formats the response and returns it to the client

### Example 2: Subscribing to transactions with a bloom filter

1. Client creates a bloom filter and connects to `subscribeToTransactionsWithProofs` stream
2. Core Streams process receives the request and registers the bloom filter
3. When Dash Core emits a ZMQ notification for a new transaction:
   - Core Streams process tests the transaction against the bloom filter
   - If it matches, the transaction is sent to the client with merkle proofs
4. The stream continues until the client disconnects

## Security

DAPI protects connections by using TLS to encrypt communication between clients and the masternodes.
This encryption safeguards transmitted data from unauthorized access, interception, or tampering.
Platform gRPC endpoints provide an additional level of security by optionally returning cryptographic proofs.
Successful proof verification guarantees that the server responded without modifying the requested data.

## Deployment Considerations

DAPI is designed to be deployed on masternode. The prefered and officaially supported way is to use [dashmate](https://docs.dash.org/en/stable/docs/user/network/dashmate/index.html).

## Monitoring

Both components use the same logging infrastructure, allowing for consistent monitoring of the entire DAPI service.
Logs are output to the console by default and can be redirected to files or log management systems as needed.

Key events that are logged include:
- Process startup and shutdown
- Connection to dependencies
- Server listening status
- Error conditions

## Endpoints

See the [endpoints](./endpoints/index.md) document for details on available endpoints.

## Further Information

- Consult the [Dash Platform Developer Documentation](https://docs.dash.org/projects/platform/en/stable/) for more information about the broader Dash Platform ecosystem
