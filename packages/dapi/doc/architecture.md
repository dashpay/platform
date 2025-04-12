# DAPI Architecture

This document explains the high-level architecture of DAPI, its components, and how they interact with the Dash ecosystem.

## Overview

DAPI (Decentralized API) serves as the gateway to the Dash network, providing access to both Dash Core blockchain functionality and Dash Platform (Evolution) features. Unlike traditional centralized APIs, DAPI is designed to run on the Dash masternode network, ensuring high availability and censorship resistance.

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

### 1. API Process (`api.js`)

The API process is the main entry point for DAPI. It handles non-streaming API requests through both gRPC and JSON-RPC interfaces.

**Responsibilities:**
- Serve Core blockchain endpoints via gRPC
- Serve Platform (Evolution) endpoints via gRPC
- Provide legacy JSON-RPC endpoints

**Dependencies:**
- Dash Core (via RPC and ZMQ)
- Drive (via gRPC)
- Tenderdash (via RPC and WebSocket)

### 2. Core Streams Process (`core-streams.js`)

The Core Streams process handles long-running streaming connections that provide real-time updates about the Dash network.

**Responsibilities:**
- Stream block headers and chain locks
- Stream transactions matching bloom filters
- Stream masternode list updates

**Dependencies:**
- Dash Core (via RPC and ZMQ)
- Chain Data Provider (internal)
- Bloom Filter Emitter (internal)

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

## Deployment Considerations

DAPI is designed to be deployed:

1. **On Masternodes**: As part of the standard masternode software stack
2. **Standalone**: For development or private deployment

For production deployments, both the API and Core Streams processes should be running with proper process management (e.g., PM2) to ensure continued operation.

## Further Information

- For more details about the processes, see [Processes](./processes.md)
- For configuration options, see [Configuration](./configuration.md)
- For available endpoints, see [Endpoints](./endpoints/index.md)