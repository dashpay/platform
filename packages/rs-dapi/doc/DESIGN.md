# rs-dapi Technical Design Document

## Overview

rs-dapi is a Rust implementation of the Dash Decentralized API (DAPI) that serves as a drop-in replacement for the existing JavaScript DAPI implementation. It provides gRPC and JSON-RPC endpoints for accessing both Dash Core (Layer 1) and Dash Platform (Layer 2) functionality through the masternode network.

rs-dapi operates behind Envoy as a reverse proxy gateway, which handles SSL termination, external security, protocol translation, and request routing. This architecture allows rs-dapi to focus on business logic while Envoy manages all external security concerns.

## Architecture

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      External Clients                      │
│           (Web browsers, mobile apps, CLI tools)           │
└─────────────────────────┬───────────────────────────────────┘
                          │ HTTPS/WSS/gRPC-Web
                          │ (SSL termination & security)
┌─────────────────────────┼───────────────────────────────────┐
│                         │                                   │
│                    Envoy Gateway                            │
│                 (Managed by Dashmate)                      │
│                                                             │
│  • SSL/TLS termination        • Load balancing             │
│  • Protocol translation       • Rate limiting              │
│  • Authentication/authorization • Request routing          │
│  • CORS handling              • Health checking            │
└─────────────────────────┬───────────────────────────────────┘
                          │ HTTP/gRPC/WebSocket
                          │ (Internal network, trusted)
┌─────────────────────────┼───────────────────────────────────┐
│                         │                                   │
│                    rs-dapi                                  │
│              (Single Binary Process)                       │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                                                     │   │
│  │              Unified Server                         │   │
│  │                                                     │   │
│  │  ┌─────────────┐  ┌─────────────────────────────┐   │   │
│  │  │             │  │                             │   │   │
│  │  │ API Handler │  │ Streams Handler             │   │   │
│  │  │             │  │                             │   │   │
│  │  │ - Core gRPC │  │ - Block streaming           │   │   │
│  │  │ - Platform  │  │ - TX streaming              │   │   │
│  │  │ - JSON-RPC  │  │ - Masternode list streaming │   │   │
│  │  │             │  │                             │   │   │
│  │  └─────────────┘  └─────────────────────────────┘   │   │
│  │                                                     │   │
│  └─────────────────────────────────────────────────────┘   │
│                         │                                  │
└─────────────────────────┼──────────────────────────────────┘
                          │
    ┌─────────────────────┼─────────────────┐
    │                     │                 │
    │          External Services            │
    │                                       │
    │  ┌──────────┐  ┌──────────────┐       │
    │  │   Dash   │  │ Tenderdash/  │       │
    │  │   Core   │  │    Drive     │       │
    │  │          │  │              │       │
    │  │ RPC+ZMQ  │  │ gRPC+WS+RPC  │       │
    │  └──────────┘  └──────────────┘       │
    └───────────────────────────────────────┘
```

## Core Components

### 1. Project Structure

```
packages/rs-dapi/
├── Cargo.toml
├── src/
│   ├── main.rs                    # Entry point and server initialization
│   ├── lib.rs                     # Library exports
│   ├── server.rs                  # Unified server implementation
│   ├── config/                    # Configuration management
│   │   ├── mod.rs
│   │   └── settings.rs
│   ├── protocol/                   # Protocol translation layer
│   │   ├── mod.rs
│   │   ├── grpc_native.rs         # Native gRPC protocol handler
│   │   └── jsonrpc_translator.rs  # JSON-RPC to gRPC translation
│   ├── services/                  # gRPC service implementations (protocol-agnostic)
│   │   ├── mod.rs
│   │   ├── core_service.rs        # Core blockchain endpoints
│   │   ├── platform_service.rs    # Platform endpoints (main service implementation)
│   │   ├── platform_service/      # Modular complex method implementations
│   │   │   ├── get_status.rs      # Complex get_status implementation with status building
│   │   │   └── subscribe_platform_events.rs  # Proxy for multiplexed Platform events
│   │   └── streams_service.rs     # Streaming endpoints
│   ├── server/                    # Network servers and monitoring endpoints
│   │   ├── mod.rs
│   │   ├── grpc.rs                # Unified gRPC server
│   │   ├── jsonrpc.rs             # JSON-RPC server bridge
│   │   └── metrics.rs             # Metrics + health HTTP endpoints (/health, /metrics)
│   ├── clients/                   # External API clients
│   │   ├── mod.rs
│   │   ├── dashcore.rs           # Dash Core RPC + ZMQ
│   │   ├── drive.rs              # Drive gRPC client
│   │   └── tenderdash.rs         # Tenderdash RPC + WebSocket
│   ├── handlers/                  # Business logic handlers (protocol-agnostic)
│   │   ├── mod.rs
│   │   ├── core_handlers.rs       # Core endpoint logic
│   │   ├── platform_handlers.rs   # Platform endpoint logic
│   │   └── stream_handlers.rs     # Streaming logic
│   ├── utils/                     # Shared utilities
│   │   ├── mod.rs
│   │   ├── validation.rs          # Input validation
│   │   ├── hash.rs               # Hash utilities
│   │   └── bloom_filter.rs       # Bloom filter implementation
│   ├── errors/                    # Error types and handling
│   │   ├── mod.rs
│   │   └── grpc_errors.rs        # gRPC error mapping
│   └── jsonrpc/                   # JSON-RPC server (deprecated, uses translation layer)
│       ├── mod.rs
│       └── server.rs
├── proto/                         # Generated protobuf code (if needed)
├── tests/                         # Integration tests
└── doc/                           # Documentation
    └── DESIGN.md                  # This document
```

### 2. Modular Service Architecture

rs-dapi implements a modular service architecture that separates simple proxy operations from complex business logic:

#### Architecture Principles
- **Separation of Concerns**: Complex methods are isolated in dedicated modules
- **Context Sharing**: All modules have access to service context without boilerplate
- **Maintainability**: Each complex operation lives in its own file for easy maintenance
- **Scalability**: New complex methods can be added as separate modules
- **Minimal Macros**: A small `drive_method!` macro is used to generate simple proxy methods with caching to reduce boilerplate; all complex logic remains in regular `impl` blocks

#### Service Organization Pattern
```
services/
├── service_name.rs               # Main service implementation
│   ├── Service struct definition
│   ├── Simple proxy methods (majority of methods)
│   ├── Service initialization
│   └── Delegation calls to complex modules
├── service_name/                 # Directory for complex methods
│   ├── complex_method_1.rs       # First complex method implementation
│   ├── complex_method_2.rs       # Second complex method implementation
│   └── ...                       # Additional complex methods
└── shared_utilities.rs           # Shared helper modules
```

#### Implementation Pattern
Each complex method follows this pattern:

```rust
// Main service file (e.g., platform_service.rs)
mod complex_method;  // Import the complex implementation

impl GrpcTrait for ServiceImpl {
    async fn simple_method(&self, req: Request<Req>) -> Result<Response<Res>, Status> {
        // Simple proxy - direct forwarding
        match self.client.simple_method(req.get_ref()).await {
            Ok(response) => Ok(Response::new(response)),
            Err(e) => Err(Status::internal(format!("Client error: {}", e))),
        }
    }

    async fn complex_method(&self, req: Request<Req>) -> Result<Response<Res>, Status> {
        // Delegate to complex implementation
        self.complex_method_impl(req).await
    }
}

// Complex method file (e.g., service_name/complex_method.rs)
impl ServiceImpl {
    pub async fn complex_method_impl(&self, req: Request<Req>) -> Result<Response<Res>, Status> {
        // Full access to service context:
        // - self.clients (drive_client, tenderdash_client, etc.)
        // - self.cache
        // - self.config
        // Complex business logic here...
    }
}
```


### 3. External Dependencies

The implementation leverages existing Dash Platform crates and external libraries:

#### Platform Crates
- `dapi-grpc` - gRPC service definitions and generated code
- `rs-dpp` - Data Platform Protocol types and validation


## Service Implementations

### 4. Core Service

Implements blockchain-related gRPC endpoints (protocol-agnostic via translation layer):

#### Endpoints
- `getBlockchainStatus` - Network and chain status information
- `getBestBlockHeight` - Current blockchain height
- `getTransaction` - Transaction lookup by hash
- `broadcastTransaction` - Submit transactions to network

#### Key Features
- Direct integration with Dash Core RPC
- ZMQ notifications for real-time updates
- Transaction validation and error handling
- Network status aggregation
- **Protocol-Agnostic**: Works identically for gRPC and JSON-RPC clients

Implementation notes:
- Implemented in `src/services/core_service.rs`, backed by `src/clients/core_client.rs` (dashcore-rpc)
- JSON-RPC minimal parity implemented in `src/server.rs` via translator (see below)

### 5. Platform Service

Implements Dash Platform gRPC endpoints (protocol-agnostic via translation layer) with a modular architecture for complex method implementations:

#### Modular Architecture
The Platform Service uses a modular structure where complex methods are separated into dedicated modules.

#### Main Service (`platform_service.rs`)
- **Service Definition**: Contains `PlatformServiceImpl` struct with all necessary context
- **Simple Methods**: Direct proxy methods that forward requests to Drive client
- **Complex Method Delegation**: Delegates complex operations to specialized modules
- **Shared Context**: All struct fields marked `pub(crate)` for submodule access

#### Complex Method Modules (`platform_service/`)
- **Dedicated Files**: Each complex method gets its own module file
- **Context Access**: Full access to service context via `impl PlatformServiceImpl` blocks
- **Business Logic**: Contains all complex caching, validation, and processing logic
- **Integrated Utilities**: Status building and other utilities included directly in method modules
- **Clean Separation**: Isolated complex logic from simple proxy operations

Implementation notes:
- Simple passthrough methods are generated by `drive_method!` with integrated LRU caching
- `get_status`, `broadcast_state_transition`, `wait_for_state_transition_result`, and `subscribe_platform_events` are implemented as dedicated modules
- Drive client is configured with increased message size limits; compression is disabled at rs-dapi level (Envoy handles wire compression)


#### Endpoints
- `broadcastStateTransition` - Submit state transitions
- `waitForStateTransitionResult` - Wait for processing with proof generation
- `getConsensusParams` - Platform consensus parameters
- `getStatus` - Platform status information

### 6. Protocol Translation

rs-dapi exposes a JSON-RPC gateway alongside gRPC. Axum powers JSON-RPC routing in `src/server.rs`.

- JSON-RPC translator: `src/protocol/jsonrpc_translator.rs`
  - Supported: `getStatus`, `getBestBlockHash`, `getBlockHash(height)`, `sendRawTransaction`
  - Translator converts JSON-RPC requests to internal calls and back; error mapping aligns with JSON-RPC codes
  - Unit tests cover translation and error paths

Operational notes:
- Compression: disabled at rs-dapi; Envoy handles edge compression
- Access logging: HTTP/JSON-RPC and gRPC traffic share the same access logging layer when configured, so all protocols emit uniform access entries

- Platform event streaming is handled via a direct upstream proxy:
  - `subscribePlatformEvents` simply forwards every inbound command stream to a single Drive connection and relays responses back without multiplexing

#### Key Features
- **Modular Organization**: Complex methods separated into dedicated modules for maintainability
- **Context Sharing**: Submodules have full access to service context (clients, cache, config)
- **No Boilerplate**: Uses `impl` blocks rather than wrapper structs
- **Integrated Utilities**: Status building and other helper functions co-located with their usage
- State transition hash validation (64-character SHA256 hex)
- Integration with Drive for proof generation
- Tenderdash WebSocket monitoring for real-time events
- Timeout handling for long-running operations
- Error conversion from Drive responses
- **Protocol-Agnostic**: Identical behavior across all client protocols

##### Platform Events Subscription Proxy

rs-dapi exposes `subscribePlatformEvents` as a server-streaming endpoint and currently performs a straightforward pass-through to rs-drive-abci.

- Public interface:
  - Bi-directional gRPC stream: `subscribePlatformEvents(request stream PlatformEventsCommand) -> (response stream PlatformEventsResponse)`.
  - Commands (`Add`, `Remove`, `Ping`) and responses (`Event`, `Ack`, `Error`) stay in their protobuf `V0` envelopes end-to-end.

- Upstream behavior:
  - Each client stream obtains its own upstream Drive connection; tokio channels forward commands upstream and pipe responses back downstream without pooling.
  - The `EventMux` from `rs-dash-event-bus` is retained for future multiplexing work but does not alter traffic today.

- Observability:
  - Standard `tracing` logging wraps the forwarders, and the proxy participates in the existing `/metrics` exporter via shared counters.

### 6. Streams Service

Implements real-time streaming gRPC endpoints (protocol-agnostic via translation layer):

#### Endpoints
- `subscribeToBlockHeadersWithChainLocks` - Block header streaming
- `subscribeToTransactionsWithProofs` - Transaction filtering with bloom filters
- `subscribeToMasternodeList` - Masternode list updates
 - Note: Platform event streaming is handled by `PlatformService::subscribePlatformEvents` and proxied directly to Drive as described in the Platform Service section.

#### Key Features
- ZMQ event processing for real-time data
- Bloom filter management for transaction filtering
- Merkle proof generation for SPV verification
- Stream lifecycle management
- Connection resilience and reconnection
- **Protocol-Agnostic**: Streaming works consistently across all protocols

### 7. JSON-RPC Service (Legacy)

Provides legacy HTTP endpoints for backward compatibility via protocol translation:

#### Endpoints
- `getBestBlockHash` - Hash of the latest block
- `getBlockHash` - Block hash by height

#### Key Features
- **Translation Layer**: All requests converted to gRPC calls internally
- HTTP server with JSON-RPC 2.0 compliance
- Error format compatibility with existing clients
- Minimal subset focused on essential operations
- **Deprecated**: New clients should use gRPC APIs

### 9. Health and Monitoring Endpoints

Built-in observability and monitoring capabilities:

#### Health Check Endpoints
- `GET /health` - Aggregated health check covering rs-dapi, Drive gRPC status, Tenderdash RPC, and Core RPC. Returns `503` when any dependency is unhealthy.
- Readiness/liveness split removed in favor of the single dependency-aware health probe.

#### Metrics Endpoints
- `GET /metrics` - Prometheus metrics

#### Status Information
- Service uptime and version
- External service connection status
- Request counts and latency statistics
- Error rates and types
- Active stream subscriber counts

## Data Flow and Processing

### 10. Multi-Protocol Server Architecture

rs-dapi implements a unified server with a protocol translation layer that normalizes all incoming requests to gRPC format, operating behind Envoy as a trusted backend service:

#### Protocol Translation Architecture
- **Protocol Translation Layer**: All non-gRPC protocols translated to gRPC format
- **Unified Business Logic**: All handlers work exclusively with gRPC messages
- **Single Code Path**: No protocol-specific logic in business layer
- **Native gRPC**: Direct pass-through for gRPC requests
- **Trusted Environment**: Operates in internal network behind Envoy gateway

#### Request Flow with Protocol Translation
```
External Client → Envoy Gateway → Protocol Translation → gRPC Services → External Services
      ↓              ↓                    ↓                  ↓               ↓
   HTTPS/WSS    SSL termination    ┌─────────────────┐   Core Service   Dash Core
   gRPC-Web  →  Protocol xlat   →  │ JSON→gRPC xlat  │→  Platform Svc →  Drive
   JSON-RPC    Rate limiting      │ Native gRPC     │   Streams Svc    Tenderdash
                Auth/CORS          └─────────────────┘   
                                   Protocol Translation Layer
```

#### Internal Architecture with Translation Layer
```
┌─────────────────────────────────────────────────────────────┐
│               rs-dapi Process (localhost only)             │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Protocol Translation Layer             │   │
│  │                                                     │   │
│  │  ┌─────────────┐ ┌─────────────┐                   │   │
│  │  │  JSON-RPC   │ │    gRPC     │                   │   │
│  │  │ Translator  │ │   Native    │                   │   │
│  │  │             │ │             │                   │   │
│  │  │ JSON→gRPC   │ │ Pass-through│                   │   │
│  │  └─────────────┘ └─────────────┘                   │   │
│  │              │              │                      │   │
│  │              └──────────────┘                      │   │
│  │                             │                      │   │
│  │                             ▼                      │   │
│  │  ┌─────────────────────────────────────────────┐   │   │
│  │  │           gRPC Services Layer               │   │   │
│  │  │         (Protocol-Agnostic)                 │   │   │
│  │  │                                             │   │   │
│  │  │ ┌─────────────┐ ┌─────────────────────────┐ │   │   │
│  │  │ │ Core Service│ │   Platform & Streams    │ │   │   │
│  │  │ │             │ │       Services          │ │   │   │
│  │  │ │ - Blockchain│ │ - State transitions     │ │   │   │
│  │  │ │ - TX broadcast │ - Block streaming     │ │   │   │
│  │  │ │ - Status    │ │ - Masternode updates   │ │   │   │
│  │  │ └─────────────┘ └─────────────────────────┘ │   │   │
│  │  └─────────────────────────────────────────────┘   │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

#### Protocol Translation Details
- **JSON-RPC Translator**: Converts JSON-RPC 2.0 format to corresponding gRPC calls
- **gRPC Native**: Direct pass-through for native gRPC requests (no translation)
- **Response Translation**: Converts gRPC responses back to original protocol format
- **Error Translation**: Maps gRPC status codes to appropriate protocol-specific errors
- **Streaming**: gRPC streaming for real-time data with consistent semantics across protocols

### 11. Protocol Translation Layer

The protocol translation layer is the key architectural component that enables unified business logic while supporting multiple client protocols:

#### Translation Components

##### JSON-RPC to gRPC Translator  
- **RPC Method Mapping**: JSON-RPC method names mapped to gRPC service methods
- **Parameter Conversion**: JSON-RPC params converted to gRPC message fields
- **ID Tracking**: JSON-RPC request IDs preserved for response correlation
- **Batch Request Support**: Multiple JSON-RPC requests in single batch handled
- **Error Format**: gRPC errors converted to JSON-RPC 2.0 error format

##### Native gRPC Handler
- **Direct Pass-through**: No translation required for native gRPC requests
- **Metadata Preservation**: gRPC metadata and headers preserved
- **Streaming Support**: Full bidirectional streaming support
- **Compression**: Native gRPC compression and optimization

### 11. State Transition Processing

The `waitForStateTransitionResult` endpoint follows this flow:

1. **Input Validation**
   - Check Tenderdash connection availability
   - Validate state transition hash format (64-char hex)
   - Parse request parameters (hash, prove flag)

2. **Transaction Monitoring**
   - Wait for transaction to be included in a block
   - Monitor Tenderdash events via WebSocket
   - Handle timeout scenarios with appropriate errors

3. **Proof Generation** (if requested)
   - Fetch proof from Drive for the state transition
   - Include metadata and proof data in response

4. **Error Handling**
   - Convert Drive errors to gRPC status codes
   - Handle timeout with `DEADLINE_EXCEEDED`
   - Map transaction errors to structured responses

### 12. Streaming Data Processing

#### Transaction Filtering
1. Client subscribes with bloom filter
2. ZMQ notifications from Dash Core processed
3. Transactions tested against bloom filters
4. Matching transactions sent with merkle proofs

#### Block Header Streaming
1. ZMQ block notifications from Dash Core
2. Block headers extracted and validated
3. Chain lock information included
4. Streamed to subscribed clients

#### Race-Free Historical + Live Backfill
To avoid gaps between historical fetching and live streaming (race conditions), rs-dapi follows a subscribe-first pattern for continuous streams:
- Subscribe to live events first and attach the forwarder to the client stream.
- Snapshot the current best height from Core RPC.
- If the request includes a starting point (`fromBlockHeight` or `fromBlockHash`) with `count = 0`, backfill historical data from the start to the snapshotted best height and send to the same stream.
- Continue forwarding live events from ZMQ; duplicates are tolerated and handled client-side.

This pattern is applied to:
- `subscribeToBlockHeadersWithChainLocks` (count = 0 with `fromBlock*`): subscribe, snapshot, backfill headers to tip, then stream live block headers and chainlocks.
- `subscribeToTransactionsWithProofs` (count = 0 with `fromBlock*`): subscribe, snapshot, backfill filtered transactions + merkle blocks to tip, then stream live transactions/locks/blocks.

Rationale: If the server performs historical fetch first and subscribes later, any blocks/transactions arriving during the fetch window can be missed. Subscribing first guarantees coverage; backfill up to a captured tip ensures deterministic catch-up without gaps.

### 13. External Service Integration

#### Dash Core Integration
- **RPC Client**: Blockchain queries, transaction broadcasting
- **ZMQ Client**: Real-time notifications (blocks, transactions, chainlocks)
- **Connection Management**: Retry logic, health checks

#### Drive Integration
- **gRPC Client**: State queries, proof generation
- **Error Mapping**: Drive-specific errors to gRPC status codes
- **Connection Pooling**: Efficient resource utilization

#### Tenderdash Integration
- **RPC Client**: Consensus queries, network status
- **WebSocket Client**: Real-time Platform events
- **Event Processing**: State transition monitoring

## Configuration and Deployment

### 14. Configuration Management


#### Process Architecture
- **Single Binary**: One process handles all DAPI functionality behind Envoy
- **Multi-threaded**: Tokio runtime with multiple worker threads
- **Shared State**: Common configuration and client connections
- **Service Isolation**: Logical separation of Core, Platform, and Streams services
- **Internal Network**: All services bind to localhost/internal addresses only
- **Trusted Backend**: No direct external exposure, operates behind Envoy gateway
- 

#### Configuration Files
- .env-based configuration with environment override
- Network-specific default configurations
- Validation and error reporting for invalid configs

### 15. Binary Architecture

The rs-dapi binary is designed as a unified server that handles all DAPI functionality:

#### Single Process Design
- **Unified Server**: Single process serving all endpoints
- **Unified gRPC Services**: Core, Platform, and Streams services on the same port, distinguished by service path
- **Integrated JSON-RPC**: HTTP server embedded within the same process
- **Shared Resources**: Common connection pools and state management

#### Port Configuration (configurable)
- **gRPC Server Port** (default: 3005): Unified port for Core + Platform + streaming endpoints
- **JSON-RPC Port** (default: 3004): Legacy HTTP endpoints
- **Health/Metrics Port** (default: 9090): Monitoring endpoints

All ports bind to internal Docker network. External access is handled by Envoy.

#### Service livecycle management

- **Docker** as primary deployment method
- **Dashmate** as primary deployment and management tool

#### Dashmate Integration
- **Drop-in Replacement**: Direct substitution for JavaScript DAPI processes
- **Same Configuration**: Uses existing environment variables and setup
- **Compatible Deployment**: Works with current dashmate deployment scripts
- **Envoy Gateway**: Integrates with existing Envoy configuration in dashmate
- **Internal Service**: Operates as trusted backend behind Envoy proxy
- **Resource Efficiency**: Single process reduces memory footprint and complexity
- **Automatic Startup**: All services and dependencies start with single command
- **Built-in Monitoring**: Health endpoints accessible to Envoy for health checks

### 16. Error Handling Strategy

#### gRPC Error Mapping
- `INVALID_ARGUMENT` - Input validation failures
- `UNAVAILABLE` - External service connectivity issues
- `DEADLINE_EXCEEDED` - Operation timeouts
- `INTERNAL` - Unexpected internal errors
- `NOT_FOUND` - Resource not found

#### Error Context
- Structured error messages with context
- Request correlation IDs for tracing
- Detailed error metadata for debugging
- Compatible error formats with JavaScript DAPI

## Performance and Scalability

### 17. Performance Characteristics

#### Async Processing
- Tokio runtime with work-stealing scheduler
- Non-blocking I/O for all external communications
- Concurrent request handling

#### Resource Management
- Connection pooling for external services
- Efficient memory usage with zero-copy operations
- Stream backpressure handling

#### Caching Strategy
- Blockchain status caching with TTL
- Connection keep-alive for external services
- Smart invalidation based on ZMQ events

### 18. Monitoring and Observability

#### Logging
- Structured logging with `tracing`
- Request/response logging with correlation IDs
- Performance metrics and timing information
- Protocol-specific logging (gRPC, JSON-RPC)
- Log levels:
  - info - business events, target audience: users, sysops/devops
  - error - errors that break things, need action or posses threat to service, target audience: users, sysops/devops
  - warn - other issues that need attention, target audience: users, sysops/devops
  - debug - non-verbose debugging information adding much value to understanding of system operations; target audience: developers
  - trace - other debugging information that is either quite verbose, or adds little value to understanding of system operations;    
    target audience: developers
  - Prefer logging information about whole logical blocks of code, not individual operations, to limit verbosity (even on trace level)

#### Built-in Metrics
- **Request Metrics**: Counts, latency histograms per protocol
- **Connection Metrics**: External service status and health
- **Stream Metrics**: Active subscribers, message throughput
- **System Metrics**: Memory usage, CPU utilization, goroutine counts
- **Business Metrics**: Transaction success rates, proof generation times

#### Prometheus Integration
- Native Prometheus metrics endpoint
- Custom metrics for DAPI-specific operations
- Grafana-compatible dashboards
- Alerting rules for operational monitoring

#### Health Checks
- Service readiness and liveness endpoints
- External service connectivity validation
- Graceful degradation strategies

## Security Considerations

### 19. Envoy Gateway Security Model

rs-dapi operates in a trusted environment behind Envoy Gateway, which handles all external security concerns:

#### External Security (Handled by Envoy)
- **SSL/TLS Termination**: All external HTTPS/WSS connections terminated at Envoy
- **Certificate Management**: SSL certificates managed by dashmate/Envoy configuration
- **Rate Limiting**: Request rate limiting and DDoS protection at gateway level
- **CORS Handling**: Cross-origin resource sharing policies enforced by Envoy
- **Authentication/Authorization**: Client authentication and authorization at gateway
- **Protocol Translation**: Secure gRPC-Web, WebSocket, and HTTPS to internal HTTP/gRPC

#### Internal Security (rs-dapi Responsibility)
- **Input Validation**: SHA256 hash format validation, buffer overflow prevention
- **Request Sanitization**: Input sanitization for all endpoints and parameters
- **Request Size Limits**: Maximum request size enforcement
- **Connection Limits**: Maximum concurrent connections per internal service
- **Trust Boundary**: Only accepts connections from localhost/internal network

### 20. Network Architecture Security

#### Trust Model
- **Trusted Internal Network**: rs-dapi assumes all requests come from trusted Envoy
- **No Direct External Exposure**: All services bind to localhost (127.0.0.1) by default
- **Network Isolation**: External network access only through Envoy gateway
- **Service Mesh**: Can be integrated with service mesh for additional internal security

#### Internal Communication Security
- **Dash Core Integration**: Secure RPC connections with authentication credentials
- **Drive Integration**: Internal gRPC connections within trusted network
- **Tenderdash Integration**: Authenticated RPC and WebSocket connections
- **Credential Management**: Secure storage and rotation of service credentials

## Testing Strategy

### 21. Test Coverage

#### Unit Tests
- Individual component testing
- Error condition testing
- Input validation testing

#### Integration Tests
- End-to-end service testing
- External service integration
- Stream lifecycle testing
- Error propagation testing

#### Performance Tests
- Load testing under various conditions
- Memory usage profiling
- Connection limit testing
- Concurrent client testing

## Migration and Compatibility

### 22. Compatibility Requirements

#### API Compatibility
- Identical gRPC service definitions
- Same JSON-RPC endpoint behavior
- Compatible error response formats
- Matching timeout behaviors

#### Configuration Compatibility
- Same environment variable names
- Compatible configuration file formats
- Identical default values
- Same network selection logic

### 23. Deployment Strategy

#### Gradual Migration
1. **Dashmate Integration**: Update dashmate to use rs-dapi binary
2. **Feature Flag Deployment**: Deploy with feature flags for rollback capability
3. **Traffic Validation**: Monitor performance and error rates in production
4. **Full Migration**: Complete replacement of JavaScript DAPI once validated

#### Deployment in Dashmate
- **Binary Replacement**: rs-dapi replaces existing JavaScript DAPI processes
- **Envoy Integration**: Works seamlessly with existing Envoy gateway configuration
- **Configuration Compatibility**: Uses same environment variables as current setup
- **Internal Network Binding**: All services bind to localhost, external access via Envoy
- **Process Management**: Single process simplifies service management in dashmate
- **Resource Optimization**: Reduced memory usage and inter-process communication overhead
- **Security Simplification**: No SSL/certificate management needed in rs-dapi

## Future Considerations

### 24. Extensibility

#### Plugin Architecture
- Modular service design
- Configurable middleware
- Extension points for custom logic

#### Performance Optimizations
- Custom allocators for high-frequency operations
- SIMD optimizations for cryptographic operations
- Advanced caching strategies

### 25. Maintenance and Updates

#### Code Organization
- Clear module boundaries
- Comprehensive documentation
- Automated testing and CI/CD
- Regular dependency updates

#### Code Style Guidelines
- Constructor pattern: `new()` methods should create fully operational objects
- Objects should be ready to use immediately after construction
- Use builder pattern for complex configuration instead of multi-step initialization
- Prefer composition over inheritance for extending functionality
- Follow Rust naming conventions and idiomatic patterns
- Document public APIs with short examples
- Use `Result<T, E>` for fallible operations, not panics in constructors

#### Monitoring and Debugging
- Advanced debugging capabilities
- Performance profiling tools
- Memory leak detection
- Crash reporting and analysis

---

This design document serves as the foundation for implementing rs-dapi and will be updated as the implementation progresses and requirements evolve.
