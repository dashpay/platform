# Dashmate services

This document provides an overview of all dashmate services, their responsibilities, and how they communicate with each other.
The platform is designed as a collection of microservices that work together to provide a complete blockchain-based application platform.

## Overview

Dashmate runs and orchestrate Dash Platform components:

```
                                 ┌─────────────────┐
                                 │                 │
                                 │    Dashmate     │
                                 │    CLI & Helper │
                                 │                 │
                                 └─────────────────┘
                                         │
                                         │ manages
                                         ▼
┌──────────────────────────────────────────────────────────────────────┐
│                                                                      │
│  ┌─────────────┐                                                     │
│  │             │                                                     │
│  │   Core      │◄──────┐                                             │
│  │  (Dash      │       │                                             │
│  │  Blockchain)│       │                                             │
│  │             │       │                                             │
│  └─────────────┘       │                                             │
│                        │                                             │
│                        │                                             │
│                ┌───────┴──────────────────────────────────┐          │
│                │                                          │          │
│                │              Platform Layer              │          │
│                │                                          │          │
│                │  ┌────────┐  ┌────────┐  ┌────────┐      │          │
│                │  │        │  │        │  │        │      │          │
│                │  │ Drive  │  │ Tender │  │ DAPI   │      │          │
│                │  │ ABCI   │  │ dash   │  │ API/   │      │          │
│                │  │        │  │        │  │ Streams│      │          │
│                │  └────────┘  └────────┘  └────────┘      │          │
│                │                                          │          │
│                └──────────────────────────────────────────┘          │
│                                   ▲                                  │
│                                   │                                  │
│                          ┌────────────────┐                          │
│                          │                │                          │
│                          │    Gateway     │◄─────────────────────────┼── User HTTP
│                          │                │                          │   Requests
│                          └────────────────┘                          │
│                                                                      │
└──────────────────────────────────────────────────────────────────────┘
```

* The Gateway handles external HTTP\HTTPS requests, provides TLS termination, and routes traffic to the correct service.
* Dash Core is the Dash blockchain node, handling Layer 1 operations (consensus, masternodes, payment transactions).
* Platform is layer 2 that provide platform functionality:
  * Drive ABCI is runtime for Platform layer 2 chain. Data contracts, and identities logic is implemented here.
  * Tenderdash is the consensus engine for the platform, providing BFT consensus for state transitions.
  * DAPI (decentralized API) exposes the platform functionalities via gRPC, and streaming interfaces, serving as the main interface for client developers.
* Dashmate orchestrates the services, providing a CLI and helper service for managing the platform.


## Platform Architecture

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

DAPI, Drive and Tenderdash communicate with Core via RPC to access layer 1 blockchain data and LLMQ functionality.

DAPI API service is interacting with Tenderdash to retrieve layer 2 blockchain data and broadcast state transitions.
Also it requests proofs for state transitions from Drive ABCI.

## Core services

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


## Gateway Services Diagram

The Gateway acts as the entry point for client applications and consists of several components:

```
                        ┌─────────────────────┐
                        │                     │
                        │  Client Applications│
                        │                     │
                        └─────────┬───────────┘
                                  │                   ┌────────────────────┐
                                  │ HTTPS             |  Rate Limiter      │
                                  │                   │  Metrics           │
                        ┌─────────▼───────────┐       └─────────┬──────────┘
                        │                     │                 │
                        │  Gateway (Envoy)    │       ┌─────────▼──────────┐
                        │  TLS Termination    │       │                    │
                        │  Request Routing    ├───────►  Rate Limiter      │
                        │                     │       │                    │
                        └─────────┬───────────┘       └─────────┬──────────┘
                                  │                             │
                                  │                    ┌────────▼──────────┐
             ┌───────────────────┬┴─────────────┐      │                   │
             │                   │              │      │  Rate Limiter     │
             │                   │              │      │  Redis Storage    │
      ┌──────▼─────┐     ┌──────▼─────┐  ┌─────▼─────┐ └───────────────────┘
      │            │     │            │  │           │
      │  DAPI API  │     │  DAPI Core │  │ Drive ABCI│
      │            │     │  Streams   │  │           │
      └────────────┘     └────────────┘  └───────────┘
```

Gateway uses Envoy proxy to handle incoming requests, providing TLS termination and routing to the appropriate service.
If rate limiter is enabled (`gateway.rateLimiter.enabled`) additional services are running to enforce rate limits on API calls and store historical request data.
Rate limiter metrics service is running to expose metrics in Prometheus format when it's enabled with the `platform.gateway.rateLimiter.metrics.enabled` configuration option.

| Service                   | Port Purpose         | Default Value | Config Path                                      | Default Host Binding  | Host Config Path |
|---------------------------|----------------------|---------------|--------------------------------------------------|-----------------------|-----------------|
| **Gateway**               | DAPI and Drive API   | 443           | `platform.gateway.listeners.dapiAndDrive.port`   | 0.0.0.0 (all)         | `platform.gateway.listeners.dapiAndDrive.host` |
|                           | Metrics              | 9090          | `platform.gateway.metrics.port`                  | 127.0.0.1 (local)     | `platform.gateway.metrics.host` |
|                           | Admin                | 9901          | `platform.gateway.admin.port`                    | 127.0.0.1 (local)     | `platform.gateway.admin.host` |
| **Gateway Rate Limiter**  | gRPC                 | 8081          | (fixed internal)                                 | (internal)            | -               |
| **Rate Limiter Metrics**  | StatsD               | 9125          | (fixed internal)                                 | (internal)            | -               |
|                           | Prometheus           | 9102          | `platform.gateway.rateLimiter.metrics.port`      | 127.0.0.1 (local)     | `platform.gateway.rateLimiter.metrics.host` |
| **Rate Limiter Redis**    | Redis                | 6379          | (fixed internal)                                 | (internal)            | -               |


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



## Dashmate CLI and Helper

Dashmate CLI and Helper are using to manage a Dash Platform node.

Both CLI and Heler using Docker to manage containers and services
defined in docker-compose files.

Dashmate helper serves JSON RPC HTTP API that replicates CLI commands.
It needs to be enabled with the `dashmate.helper.enabled` configuration option.
Helper also performs some background tasks such as ZeroSSL certificates renewal.

| Service                   | Port Purpose         | Default Value | Config Path                                  | Default Host Binding | Host Config Path |
|---------------------------|----------------------|---------------|----------------------------------------------|---------------------|-----------------|
| **Dashmate Helper**       | API                  | 9100          | `dashmate.helper.api.port`                   | 127.0.0.1 (local)   | (fixed)         |



## Communication Flows

### Client API Request Flow

1. Client sends request to Gateway (port 443)
2. Gateway routes the request to:
   - DAPI API for JSON-RPC/gRPC calls
   - DAPI Core Streams for streaming requests
   - Drive ABCI for direct drive requests (if applicable)
3. DAPI API or DAPI Core Streams processes the request by:
   - Querying Dash Core for blockchain data
   - Querying Drive ABCI for platform state
   - Querying Tenderdash for consensus data
4. Response flows back through the same path to the client

### Platform State Transition Flow

1. Client submits state transition to Gateway
2. Gateway routes to DAPI API
3. DAPI API forwards to Drive ABCI
4. Drive ABCI validates and creates a transaction
5. Transaction is submitted to Tenderdash
6. Tenderdash creates a block through consensus
7. Block is executed by Drive ABCI, updating state
8. Periodically, state is anchored to Core blockchain

### Blockchain Synchronization Flow

1. Core syncs with the Dash network
2. Drive ABCI monitors Core for platform-related transactions
3. When platform transactions are found, they are processed by Drive ABCI
4. Tenderdash ensures all validators have the same state
5. State is updated across the platform

## Port Security Considerations

### Summary of Exposed Ports

| Type                | Ports                                                        | Default Host Binding |
|---------------------|--------------------------------------------------------------|---------------------|
| **Public-facing**   | Core P2P (9999)<br>Tenderdash P2P (26656)<br>Gateway API (443) | 0.0.0.0 (all)       |
| **Localhost-only**  | Core RPC (9998)<br>Insight UI (3001)<br>Dashmate Helper (9100)<br>Drive ABCI Metrics (29090)<br>Drive Debug Tools (6669, 8083)<br>Tenderdash RPC (26657)<br>Tenderdash Metrics (26660)<br>Tenderdash Debug (6060)<br>Gateway Metrics (9090)<br>Gateway Admin (9901)<br>Rate Limiter Metrics (9102) | 127.0.0.1 (local)   |
| **Internal only**   | Core ZMQ (29998)<br>Drive ABCI (26658)<br>Drive gRPC (26670)<br>DAPI JSON-RPC (3004)<br>DAPI gRPC (3005)<br>DAPI Streams (3006)<br>Rate Limiter gRPC (8081)<br>Rate Limiter StatsD (9125)<br>Rate Limiter Redis (6379) | (not exposed)       |

### Port Security Notes

- **Public-facing ports** (0.0.0.0): Only ports that need to be accessible from other machines bind to all interfaces by default.
- **Localhost-only ports** (127.0.0.1): Most ports bind only to localhost by default for security.
- **Internal-only ports**: These ports are not bound to any host interface and are only accessible from other containers in the Docker network.
- **RPC Security**: Core RPC has an explicit allowlist in the configuration: `core.rpc.allowIps` (default: ['127.0.0.1', '172.16.0.0/12', '192.168.0.0/16'])

## Network Configuration

The default Docker network configuration can be customized with the following settings:

- Network Subnet: `docker.network.subnet` (default: "0.0.0.0/0")
- Container addresses are assigned automatically by Docker within the subnet range

The platform uses different Docker Compose profiles to enable specific functionality:
- `core`: Core and optional Insight services
- `platform`: All platform-specific services

## Data Persistence

Several volumes are used for data persistence:
- `core_data`: Stores blockchain data
- `drive_abci_data`: Stores platform state data
- `drive_tenderdash`: Stores consensus data

## Metrics and Monitoring

Most services provide metrics endpoints:
- Drive ABCI: Platform application metrics
- Tenderdash: Consensus metrics
- Gateway: API gateway metrics
- Rate Limiter: Rate limiting metrics

These can be integrated with monitoring systems like Prometheus and Grafana.
