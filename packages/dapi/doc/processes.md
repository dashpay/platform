# DAPI Processes

DAPI operates using two separate Node.js processes, each handling different aspects of the API functionality. This document explains each process and its responsibilities.

## 1. API Process (`api.js`)

The API process is the main entry point for DAPI. It handles the basic gRPC and JSON-RPC endpoints, including both Core and Platform functionality.

### Responsibilities:

- **JSON-RPC Server**: Serves legacy JSON-RPC endpoints
- **Core gRPC Endpoints**: Serves Core blockchain endpoints
- **Platform gRPC Endpoints**: Serves Platform (Evolution) endpoints

### Connections:

- **Dash Core**: Connects to Core via RPC and ZMQ
- **Drive**: Connects to Drive via gRPC
- **Tenderdash**: Connects to Tenderdash via RPC and WebSocket

### Startup Sequence:

1. Load configuration
2. Connect to Dash Core's ZMQ interface
3. Initialize Platform and Drive clients
4. Connect to Tenderdash WebSocket
5. Start JSON-RPC server
6. Start gRPC server with Core and Platform handlers

### Endpoints Served:

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

## 2. Core Streams Process (`core-streams.js`)

The Core Streams process handles streaming data from the Dash blockchain, including blocks, transactions, and masternode lists.

### Responsibilities:

- **Transaction Streaming**: Stream transactions matching bloom filters
- **Block Header Streaming**: Stream block headers and chain locks
- **Masternode List Streaming**: Stream masternode list updates

### Connections:

- **Dash Core**: Connects to Core via RPC and ZMQ
- **Chain Data Provider**: Maintains a cache of block headers

### Startup Sequence:

1. Load configuration
2. Connect to Dash Core's ZMQ interface
3. Initialize bloom filter emitter collection
4. Set up event listeners for ZMQ events
5. Initialize chain data provider and block headers cache
6. Initialize masternode list sync
7. Start gRPC server with streaming handlers

### Endpoints Served:

- **Stream gRPC Endpoints**:
  - `subscribeToTransactionsWithProofs`
  - `subscribeToBlockHeadersWithChainLocks`
  - `subscribeToMasternodeList`

## Process Communication

The two processes operate independently and do not directly communicate with each other. Instead, they both connect to the same underlying services (Dash Core, Drive, Tenderdash) to provide their respective functionality.

## Running the Processes

To run DAPI, both processes need to be started:

```bash
# Start the API process
node scripts/api.js

# Start the Core Streams process
node scripts/core-streams.js
```

It's recommended to use a process manager like PM2 in production environments to ensure both processes stay running and restart automatically if they crash.

## Monitoring

Both processes use the same logging infrastructure, allowing for consistent monitoring of the entire DAPI service. Logs are output to the console by default and can be redirected to files or log management systems as needed.

Key events that are logged include:
- Process startup and shutdown
- Connection to dependencies
- Server listening status
- Error conditions

## Configuration

Both processes share the same configuration infrastructure, reading environment variables or a `.env` file. See the [Configuration](./configuration.md) document for details on available options.