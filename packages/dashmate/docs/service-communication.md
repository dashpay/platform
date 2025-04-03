# Dash Platform Service Communication

This document provides a detailed overview of how different services in the Dash Platform communicate with each other, including port configurations and network settings.

## Communication Protocols

Dash Platform uses several communication protocols between its services:

1. **HTTP/JSON-RPC**: Used for traditional API calls
2. **gRPC**: Used for efficient service-to-service communication
3. **ABCI (Application Blockchain Interface)**: Used between Tenderdash and Drive ABCI
4. **ZMQ (ZeroMQ)**: Used for event notifications from Core
5. **Redis Protocol**: Used for rate limiting data storage

## Service Communication Matrix

The table below shows the communication patterns between services:

| Service              | Communicates With                     | Protocol(s)                   | Purpose                                      |
|----------------------|---------------------------------------|-------------------------------|----------------------------------------------|
| Gateway              | DAPI API                              | HTTP/gRPC                     | Route client API requests                    |
|                      | DAPI Core Streams                     | gRPC                          | Route client streaming requests              |
|                      | Drive ABCI                            | gRPC                          | Direct drive access                          |
|                      | Gateway Rate Limiter                  | gRPC                          | Enforce rate limits                          |
| DAPI API             | Core                                  | JSON-RPC                      | Access blockchain data                       |
|                      | Drive ABCI                            | gRPC                          | Access platform state                        |
|                      | Tenderdash                            | JSON-RPC                      | Access consensus data                        |
| DAPI Core Streams    | Core                                  | JSON-RPC, ZMQ                 | Stream blockchain events                     |
|                      | Drive ABCI                            | gRPC                          | Access platform state                        |
|                      | Tenderdash                            | JSON-RPC                      | Access consensus data                        |
| Drive ABCI           | Core                                  | JSON-RPC                      | Access blockchain data for consensus and data |
|                      | Tenderdash                            | ABCI                          | Provide application logic for consensus      |
| Tenderdash           | Drive ABCI                            | ABCI                          | Execute application logic                    |
|                      | Other Tenderdash Nodes                | P2P (TCP)                     | Consensus communication                      |
| Gateway Rate Limiter | Gateway Rate Limiter Redis            | Redis Protocol                | Store rate limiting data                     |
|                      | Gateway Rate Limiter Metrics          | StatsD                        | Report metrics                               |
| Dashmate Helper      | Docker Daemon                         | Docker API                    | Manage containers                            |
| Insight API         | Core                                  | JSON-RPC, Direct file access  | Access blockchain data for API queries       |
| Insight UI          | Core                                  | Via Insight API              | Provide blockchain explorer web interface     |

## Exposed Ports and Configuration

The following table details all ports used by Dash Platform services, including their default values and configuration paths:

| Service                   | Port Type           | Default Value | Config Path                                  | Description                                    |
|---------------------------|---------------------|---------------|----------------------------------------------|------------------------------------------------|
| **Core**                  | P2P                 | 9999          | `core.p2p.port`                              | Peer-to-peer communication with other nodes    |
|                           | RPC                 | 9998          | `core.rpc.port`                              | JSON-RPC API for blockchain interaction        |
|                           | ZMQ                 | 29998         | (fixed internal)                             | Event notifications to DAPI Core Streams       |
| **Core Insight API**      | HTTP API            | 3001          | `core.insight.port`                          | Blockchain explorer API                        |
| **Core Insight UI**       | HTTP Web Interface   | 3001          | `core.insight.port`                          | Blockchain explorer web user interface        |
| **Dashmate Helper**       | API                 | 9100          | `dashmate.helper.api.port`                   | Helper API for dashmate operations             |
| **Drive ABCI**            | ABCI                | 26658         | (fixed internal)                             | Communication with Tenderdash                  |
|                           | gRPC                | 26670         | (fixed internal)                             | API for DAPI services                          |
|                           | Metrics             | 29090         | (fixed internal, exposed via config)         | Prometheus metrics endpoint                    |
|                           | Tokio Console       | 6669          | `platform.drive.abci.tokioConsole.port`      | Debug console for Drive internals              |
|                           | GroveDB Visualizer  | 8083          | `platform.drive.abci.grovedbVisualizer.port` | Tool for GroveDB visualization                 |
| **Drive Tenderdash**      | P2P                 | 26656         | `platform.drive.tenderdash.p2p.port`         | Peer-to-peer communication between validators  |
|                           | RPC                 | 26657         | `platform.drive.tenderdash.rpc.port`         | JSON-RPC API for consensus queries             |
|                           | Metrics             | 26660         | `platform.drive.tenderdash.metrics.port`     | Prometheus metrics endpoint                    |
|                           | pprof Debug         | 6060          | `platform.drive.tenderdash.pprof.port`       | Go profiling endpoint                          |
| **DAPI API**              | JSON-RPC            | 3004          | (fixed internal)                             | JSON-RPC API to platform                       |
|                           | gRPC                | 3005          | (fixed internal)                             | gRPC API to platform                           |
| **DAPI Core Streams**     | gRPC Streaming      | 3006          | (fixed internal)                             | Streaming updates from platform                |
| **Gateway**               | DAPI and Drive      | 443           | `platform.gateway.listeners.dapiAndDrive.port` | Main entry point for platform APIs            |
|                           | Metrics             | 9090          | `platform.gateway.metrics.port`              | Gateway metrics endpoint                       |
|                           | Admin               | 9901          | `platform.gateway.admin.port`                | Admin interface for Gateway                    |
| **Gateway Rate Limiter**  | gRPC                | 8081          | (fixed internal)                             | Rate limiting service                          |
|                           | Metrics             | 9102          | `platform.gateway.rateLimiter.metrics.port`  | Rate limiter metrics endpoint                  |
| **Rate Limiter Redis**    | Redis               | 6379          | (fixed internal)                             | Rate limit data storage                        |

## Default Hosts Configuration

Most services bind their ports to specific interfaces, controlled by configuration:

| Service               | Port Type           | Default Host    | Config Path                                    | Purpose                              |
|-----------------------|---------------------|-----------------|-----------------------------------------------|--------------------------------------|
| Core                  | P2P                 | 0.0.0.0         | `core.p2p.host`                               | Accept connections from any address  |
|                       | RPC                 | 127.0.0.1       | `core.rpc.host`                               | Accept only local connections        |
| Drive Tenderdash      | P2P                 | 0.0.0.0         | `platform.drive.tenderdash.p2p.host`          | Accept connections from any address  |
|                       | RPC                 | 127.0.0.1       | `platform.drive.tenderdash.rpc.host`          | Accept only local connections        |
| Gateway               | DAPI and Drive      | 0.0.0.0         | `platform.gateway.listeners.dapiAndDrive.host`| Accept connections from any address  |
|                       | Metrics             | 127.0.0.1       | `platform.gateway.metrics.host`               | Accept only local connections        |
|                       | Admin               | 127.0.0.1       | `platform.gateway.admin.host`                 | Accept only local connections        |
| Drive ABCI            | Tokio Console       | 127.0.0.1       | `platform.drive.abci.tokioConsole.host`       | Accept only local connections        |
|                       | GroveDB Visualizer  | 127.0.0.1       | `platform.drive.abci.grovedbVisualizer.host`  | Accept only local connections        |
|                       | Metrics             | 127.0.0.1       | (exposed via PLATFORM_DRIVE_ABCI_METRICS_HOST)| Accept only local connections        |

## Port Mappings and Service Discovery

Services communicate with each other using Docker network DNS names and port numbers:

| Service                      | Internal Name           | Internal Ports       | External Ports (if mapped)            |
|------------------------------|-------------------------|----------------------|---------------------------------------|
| Core                         | core                    | 29998 (ZMQ)          | CORE_P2P_PORT, CORE_RPC_PORT         |
|                              |                         | CORE_RPC_PORT        |                                       |
|                              |                         | CORE_P2P_PORT        |                                       |
| Drive ABCI                   | drive_abci              | 26658 (ABCI)         | PLATFORM_DRIVE_ABCI_METRICS_PORT     |
|                              |                         | 26670 (gRPC)         | PLATFORM_DRIVE_ABCI_TOKIO_CONSOLE_PORT|
|                              |                         | 29090 (Metrics)      | PLATFORM_DRIVE_ABCI_GROVEDB_VISUALIZER_PORT |
| Tenderdash                   | drive_tenderdash        | PLATFORM_DRIVE_TENDERDASH_P2P_PORT | PLATFORM_DRIVE_TENDERDASH_P2P_PORT |
|                              |                         | PLATFORM_DRIVE_TENDERDASH_RPC_PORT | PLATFORM_DRIVE_TENDERDASH_RPC_PORT |
|                              |                         | PLATFORM_DRIVE_TENDERDASH_METRICS_PORT | PLATFORM_DRIVE_TENDERDASH_METRICS_PORT |
| DAPI API                     | dapi_api                | 3004 (JSON-RPC)      | Not directly exposed                  |
|                              |                         | 3005 (gRPC)          |                                       |
| DAPI Core Streams            | dapi_core_streams       | 3006 (gRPC stream)   | Not directly exposed                  |
| Gateway                      | gateway                 | 10000 (Main API)     | PLATFORM_GATEWAY_LISTENERS_DAPI_AND_DRIVE_PORT |
|                              |                         | 9090 (Metrics)       | PLATFORM_GATEWAY_METRICS_PORT        |
|                              |                         | 9901 (Admin)         | PLATFORM_GATEWAY_ADMIN_PORT          |
| Gateway Rate Limiter         | gateway_rate_limiter    | 8081 (gRPC)          | Not directly exposed                  |
| Gateway Rate Limiter Redis   | gateway_rate_limiter_redis | 6379 (Redis)      | Not directly exposed                  |
| Gateway Rate Limiter Metrics | gateway_rate_limiter_metrics | 9125 (StatsD)   | PLATFORM_GATEWAY_RATE_LIMITER_METRICS_PORT |
|                              |                         | 9102 (Prometheus)    |                                       |
| Dashmate Helper              | dashmate_helper         | DASHMATE_HELPER_API_PORT | DASHMATE_HELPER_API_PORT         |
| Insight API                  | core_insight            | CORE_INSIGHT_PORT    | CORE_INSIGHT_PORT                    |
| Insight UI                   | core_insight (UI)        | CORE_INSIGHT_PORT    | CORE_INSIGHT_PORT                    |

## Network Configuration

The default Docker network configuration can be customized with the following settings:

- Network Subnet: `docker.network.subnet` (default: "0.0.0.0/0")
- Container addresses are assigned automatically by Docker within the subnet range

## Authentication and Security

Services in the Dash Platform secure their communications in several ways:

1. **RPC Authentication**: Core RPC uses username/password authentication:
   - Drive ABCI uses `drive_consensus` and `drive_check_tx` users
   - DAPI API uses the `dapi` user
   - Each RPC user has access to a specific whitelist of commands

2. **Network Isolation**: 
   - Gateway and Rate Limiter services use a separate Docker network (`gateway_rate_limiter`)
   - Other services use the default network

3. **TLS/SSL**: 
   - Gateway handles TLS termination for external client connections
   - SSL can be enabled via `platform.gateway.ssl.enabled` configuration
   - Supports both ZeroSSL and self-signed certificates

4. **IP Restrictions**:
   - Most internal services only bind to localhost by default
   - Core RPC has an explicit allowlist: `core.rpc.allowIps` (default: ['127.0.0.1', '172.16.0.0/12', '192.168.0.0/16'])

## Service Configuration Locations

All service configuration is managed through Dashmate config files. The main configuration paths for each service are:

- Core: `core.*`
- Drive ABCI: `platform.drive.abci.*`
- Tenderdash: `platform.drive.tenderdash.*`
- DAPI API: `platform.dapi.api.*`
- Gateway: `platform.gateway.*`
- Dashmate Helper: `dashmate.helper.*`

## Data Flow Examples

### Platform State Transition (Document Creation)

1. Client sends a document creation state transition to Gateway (port 443)
2. Gateway routes to DAPI API (internal port 3004/3005)
3. DAPI API validates the request
4. DAPI API sends the state transition to Drive ABCI via gRPC (port 26670)
5. Drive ABCI validates the state transition
6. Drive ABCI sends the state transition to Tenderdash via ABCI (port 26658)
7. Tenderdash reaches consensus with other nodes
8. Tenderdash executes the state transition via Drive ABCI
9. Drive ABCI updates its state database
10. Response flows back to the client through DAPI API and Gateway

### Blockchain Query (Transaction Lookup)

1. Client sends a transaction query to Gateway (port 443)
2. Gateway routes to DAPI API (internal port 3004/3005)
3. DAPI API sends an RPC request to Core (port 9998)
4. Core responds with transaction data
5. Response flows back to the client through DAPI API and Gateway

### Real-time Transaction Updates

1. Client establishes a streaming connection to Gateway (port 443)
2. Gateway routes to DAPI Core Streams (internal port 3006)
3. DAPI Core Streams subscribes to Core ZMQ notifications (port 29998)
4. When Core processes a new transaction, it sends a notification via ZMQ
5. DAPI Core Streams processes the notification
6. Update is streamed to the client through Gateway