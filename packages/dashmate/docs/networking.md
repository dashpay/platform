# Dash Platform Service Communication

This document provides a detailed overview of how different services in the Dash Platform communicate with each other.

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
| Drive ABCI           | Core                                  | JSON-RPC                      | Access blockchain for consensus and data     |
|                      | Tenderdash                            | ABCI                          | Provide application logic for consensus      |
| Tenderdash           | Drive ABCI                            | ABCI                          | Execute application logic                    |
|                      | Other Tenderdash Nodes                | P2P (TCP)                     | Consensus communication                      |
| Gateway Rate Limiter | Gateway Rate Limiter Redis            | Redis Protocol                | Store rate limiting data                     |
|                      | Gateway Rate Limiter Metrics          | StatsD                        | Report metrics                               |
| Dashmate Helper      | Docker Daemon                         | Docker API                    | Manage containers                            |
| Insight API/UI       | Core                                  | JSON-RPC, Direct file access  | Access blockchain data                       |

## Port Mappings and Service Discovery

Services communicate with each other using Docker network DNS names and port numbers:

| Service                      | Internal Name           | Internal Ports       | External Ports (if mapped)            |
|------------------------------|-------------------------|----------------------|---------------------------------------|
| Core                         | core                    | 9998 (ZMQ)           | CORE_P2P_PORT, CORE_RPC_PORT         |
|                              |                         | CORE_RPC_PORT        |                                       |
|                              |                         | CORE_P2P_PORT        |                                       |
| Drive ABCI                   | drive_abci              | 26658 (ABCI)         | PLATFORM_DRIVE_ABCI_METRICS_PORT     |
|                              |                         | 26670 (gRPC)         | PLATFORM_DRIVE_ABCI_TOKIO_CONSOLE_PORT|
|                              |                         | 29090 (Metrics)      | PLATFORM_DRIVE_ABCI_GROVEDB_VISUALIZER_PORT |
|                              |                         |                      |                                       |
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
| Insight API/UI               | core_insight            | CORE_INSIGHT_PORT    | CORE_INSIGHT_PORT                    |

## Authentication and Security

Services in the Dash Platform secure their communications in several ways:

1. **RPC Authentication**: Core RPC uses username/password authentication:
   - Drive ABCI uses `drive_consensus` and `drive_check_tx` users
   - DAPI API uses the `dapi` user

2. **Network Isolation**: 
   - Gateway and Rate Limiter services use a separate Docker network (`gateway_rate_limiter`)
   - Other services use the default network

3. **TLS/SSL**: 
   - Gateway handles TLS termination for external client connections
   - Internal service communication typically uses unencrypted connections within the Docker network

## Data Flow Examples

### Platform State Transition (Document Creation)

1. Client sends a document creation state transition to Gateway (port 10000)
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

1. Client sends a transaction query to Gateway (port 10000)
2. Gateway routes to DAPI API (internal port 3004/3005)
3. DAPI API sends an RPC request to Core (port CORE_RPC_PORT)
4. Core responds with transaction data
5. Response flows back to the client through DAPI API and Gateway

### Real-time Transaction Updates

1. Client establishes a streaming connection to Gateway (port 10000)
2. Gateway routes to DAPI Core Streams (internal port 3006)
3. DAPI Core Streams subscribes to Core ZMQ notifications (port 9998)
4. When Core processes a new transaction, it sends a notification via ZMQ
5. DAPI Core Streams processes the notification
6. Update is streamed to the client through Gateway

## Network Topology Considerations

The Dash Platform is designed to work in various deployment scenarios:

1. **Single Node**: All services run on a single machine
2. **Distributed**: Services can be distributed across multiple machines
3. **High Availability**: Critical services can be replicated for redundancy

The Docker Compose configuration supports scaling of certain services:
- DAPI API and DAPI Core Streams can be scaled with the `replicas` setting
- Other services like Core, Drive ABCI, and Tenderdash typically run as single instances

## Troubleshooting Communication Issues

When troubleshooting service communication issues:

1. **Check service connectivity**:
   ```bash
   docker exec -it <container_name> ping <service_name>
   ```

2. **Verify port accessibility**:
   ```bash
   docker exec -it <container_name> nc -zv <service_name> <port>
   ```

3. **Check service logs**:
   ```bash
   docker logs <container_name>
   ```

4. **Inspect network configuration**:
   ```bash
   docker network inspect bridge
   ```

5. **Monitor API calls** through the Gateway admin interface (port 9901)