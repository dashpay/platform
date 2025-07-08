# Gateway Service

The Gateway service is a critical component of the Dash Platform infrastructure that serves as the entry point for external clients to access Platform services.
It uses Envoy, a high-performance proxy, to route HTTP requests to the appropriate backend services.

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

## Overview

The Gateway service performs several key functions:

1. **Request Routing**: Directs incoming API requests to the appropriate backend services (DAPI API, DAPI Core Streams, Drive gRPC)
2. **Protocol Support**: Handles HTTP/1.1, HTTP/2, and gRPC-Web protocols
3. **Connection Management**: Controls connection timeouts, idle timeouts, and concurrent stream limits
4. **Load Protection**: Implements circuit breaking, rate limiting, and resource monitoring
5. **TLS Termination**: Manages SSL/TLS encryption for secure communications
6. **Cross-Origin Resource Sharing (CORS)**: Enables web applications to safely consume the API
7. **Observability**: Provides metrics, logging, and administrative capabilities

## Architecture

The Gateway service uses Envoy as its proxy technology, which is configured to handle different types of requests through specific routes and clusters.

### Listeners

Listeners define how the Gateway accepts connections:

- **dapi_and_drive**: Main listener that handles all Platform API requests on port 10000 (mapped to 443 by default)
- **prometheus_metrics**: Optional listener for exposing Prometheus metrics when metrics are enabled

### Clusters

Clusters define the backend services the Gateway connects to:

- **dapi_api**: Handles general DAPI requests
- **dapi_core_streams**: Handles streaming Core endpoints
- **dapi_json_rpc**: Handles JSON-RPC requests
- **drive_grpc**: Handles Platform requests
- **ratelimit_service**: Optional service for rate limiting
- **admin**: Internal administrative interface

### Routes

The Gateway routes requests to different backend services based on URL path:

- Core streaming endpoints (`/org.dash.platform.dapi.v0.Core/subscribeTo*`): routed to `dapi_core_streams`
- Other Core endpoints (`/org.dash.platform.dapi.v0.Core*`): routed to `dapi_api`
- Platform waitForStateTransitionResult (`/org.dash.platform.dapi.v0.Platform/waitForStateTransitionResult`): routed to `dapi_api` with extended timeout
- Platform endpoints (`/org.dash.platform.dapi.v0.Platform*`): routed to `drive_grpc`
- JSON-RPC endpoints (`/`): routed to `dapi_json_rpc`

## Configuration Options

### HTTP Connection Management

The Gateway service carefully manages HTTP connections with the following settings (fixed in Envoy configuration):

```yaml
common_http_protocol_options:
  max_connection_duration: 600s  # Maximum duration for a single HTTP connection
  idle_timeout: 300s             # How long to maintain an idle connection
  max_stream_duration: 15s       # Default request timeout (overridden for specific routes)
```

### HTTP/2 Protocol Options

For HTTP/2 connections, which support multiple concurrent requests per connection:

```yaml
http2_protocol_options:
  initial_stream_window_size: 65536     # 64 KiB - Buffer per-stream
  initial_connection_window_size: 1048576  # 1 MiB - Connection-level buffer
  max_concurrent_streams: 10  # Maximum parallel requests per connection
```

**Config option**: `platform.gateway.listeners.dapiAndDrive.http2.maxConcurrentStreams`

### Timeouts

Different endpoints have different timeout requirements:

- Standard API endpoints: 10-15 seconds (fixed in Envoy configuration)
- Core streaming endpoints: 600 seconds (10 minutes) (fixed in Envoy configuration)
- waitForStateTransitionResult endpoint: 125 seconds (default)

**Config option**: `platform.gateway.listeners.dapiAndDrive.waitForStResultTimeout`

### Circuit Breaking

The Gateway implements circuit breaking to prevent service overload:

```yaml
circuit_breakers:
  thresholds:
    - priority: DEFAULT
      max_requests: 100  # Maximum parallel requests to a service
```

Each backend service has its own circuit breaker configuration with a default of 100 max requests.

**Config options**:
- DAPI API: `platform.gateway.upstreams.dapiApi.maxRequests`
- DAPI Core Streams: `platform.gateway.upstreams.dapiCoreStreams.maxRequests`
- DAPI JSON-RPC: `platform.gateway.upstreams.dapiJsonRpc.maxRequests`
- Drive gRPC: `platform.gateway.upstreams.driveGrpc.maxRequests`

### Resource Monitoring and Overload Protection

The Gateway monitors resource usage and takes actions to protect the system under high load:

```yaml
resource_monitors:
  # Monitor heap memory usage
  - name: "envoy.resource_monitors.fixed_heap"
    max_heap_size_bytes: 125000000  # Maximum memory usage (125 MB)

  # Monitor connection count
  - name: "envoy.resource_monitors.global_downstream_max_connections"
    max_active_downstream_connections: 1000  # Maximum concurrent connections
```

**Config options**:
- Maximum heap size: `platform.gateway.maxHeapSizeInBytes`
- Maximum connections: `platform.gateway.maxConnections`

When resource thresholds are exceeded, the Gateway implements progressive protective actions:

1. At 92% heap usage: Release unused memory and disable HTTP keepalives
2. At 95% heap usage: Stop accepting new HTTP requests and TCP connections
3. At 95-100% connection limit: Disable keepalives and stop accepting new connections

## Rate Limiting

When enabled (default is enabled), the Gateway implements IP-based rate limiting to protect the system from abuse:

```yaml
http_filters:
  - name: envoy.filters.http.ratelimit
    typed_config:
      domain: edge_proxy_per_ip
      timeout: 5s
      failure_mode_deny: true
      rate_limited_as_resource_exhausted: true
```

**Config options**:
- Enable/disable: `platform.gateway.rateLimiter.enabled`
- Requests per time unit: `platform.gateway.rateLimiter.requestsPerUnit` (default: 150)
- Time unit: `platform.gateway.rateLimiter.unit` (default: minute, options: second, minute, hour, day)
- IP whitelist: `platform.gateway.rateLimiter.whitelist`
- IP blacklist: `platform.gateway.rateLimiter.blacklist`

## TLS/SSL Configuration

The Gateway supports multiple SSL configuration options:

1. **Self-signed certificates**: Generated locally for testing
2. **ZeroSSL**: Automated certificate issuance and renewal (default provider)
3. **File-based**: Using existing certificate files

**Config options**:
- Enable/disable SSL: `platform.gateway.ssl.enabled`
- Provider: `platform.gateway.ssl.provider` (options: 'self-signed', 'zerossl')
- ZeroSSL configuration: `platform.gateway.ssl.providerConfigs.zerossl.apiKey` and `platform.gateway.ssl.providerConfigs.zerossl.id`

When SSL is enabled, the Gateway configures TLS settings:

```yaml
transport_socket:
  name: envoy.transport_sockets.tls
  typed_config:
    common_tls_context:
      alpn_protocols: [ "h2, http/1.1" ]  # Support both HTTP/2 and HTTP/1.1
      tls_certificates:
        - certificate_chain:
            filename: "/etc/ssl/bundle.crt"
          private_key:
            filename: "/etc/ssl/private.key"
```

## Logging

The Gateway provides flexible logging options for monitoring and debugging:

```yaml
access_log:
  - name: envoy.access_loggers.stdout
    typed_config:
      "@type": type.googleapis.com/envoy.extensions.access_loggers.stream.v3.StdoutAccessLog
      log_format:
        text_format_source:
          inline_string: "[%START_TIME%] \"%REQ(:METHOD)% %REQ(X-ENVOY-ORIGINAL-PATH?:PATH)%\" %RESPONSE_CODE% %GRPC_STATUS% %RESPONSE_FLAGS% R:%BYTES_RECEIVED% S:%BYTES_SENT% D:%DURATION%\n"
```

**Config options**:
- Log level: `platform.gateway.log.level`
- Access logs: `platform.gateway.log.accessLogs` (array of log configurations)
  - Each log entry can specify:
    - Type: 'stdout', 'stderr', or 'file'
    - Format: 'json' or 'text'
    - Template: custom format template

Default logging configuration:
- Destination: stdout
- Format: text
- Level: info

## Metrics

The Gateway can expose Prometheus metrics for monitoring its performance and health status. This feature is disabled by default for security reasons.

When enabled, the Gateway creates a dedicated listener that exposes a `/metrics` endpoint containing detailed performance statistics in Prometheus format.

```yaml
listeners:
  - name: prometheus_metrics
    address:
      socket_address:
        address: "0.0.0.0"  # Bound to all interfaces but should be restricted to trusted networks
        port_value: 9090    # Default port for Prometheus metrics
```

Metrics include:
- Request rates and error counts
- Request and response sizes
- Connection statistics
- Circuit breaker state
- Resource usage (memory, connections)
- HTTP status code distribution

### Configuration Options

**Config options**:
- Enable/disable metrics: `platform.gateway.metrics.enabled` (default: false)
- Host: `platform.gateway.metrics.host` (default: 127.0.0.1)
- Port: `platform.gateway.metrics.port` (default: 9090)

**Note:** admin interface must be enabled too.

### Security Considerations

For security reasons:
- Metrics should be exposed only on trusted networks
- Consider using network-level access controls to restrict access
- Use a reverse proxy with authentication for public exposure
- Never expose metrics on public interfaces without protection

## Admin Interface

The Gateway's admin interface provides powerful runtime inspection and configuration capabilities. This feature is disabled by default for security reasons.

When enabled, the admin interface provides:

```yaml
admin:
  address:
    socket_address:
      address: 0.0.0.0  # Bound to all interfaces but should be restricted to trusted networks
      port_value: 9901  # Default port for admin interface
```

The admin interface offers:
- Real-time configuration inspection and modification
- Runtime statistics and metrics
- Cluster and route information
- Connection and request debugging
- Memory usage and resource statistics
- Log level control

### Configuration Options

**Config options**:
- Enable/disable admin: `platform.gateway.admin.enabled` (default: false)
- Host: `platform.gateway.admin.host` (default: 127.0.0.1)
- Port: `platform.gateway.admin.port` (default: 9901)

### Security Considerations

The admin interface is a powerful tool but presents security risks:
- Only enable in protected environments
- Restrict access to trusted networks using firewalls
- Never expose the admin interface to public networks
- Consider using a reverse proxy with strong authentication if remote access is needed
- Regularly audit access to the admin interface

## Ports

| Service                   | Port Purpose         | Default Value | Config Path                                      | Default Host Binding  | Host Config Path |
|---------------------------|----------------------|---------------|--------------------------------------------------|-----------------------|-----------------|
| **Gateway**               | DAPI and Drive API   | 443           | `platform.gateway.listeners.dapiAndDrive.port`   | 0.0.0.0 (all)         | `platform.gateway.listeners.dapiAndDrive.host` |
|                           | Metrics              | 9090          | `platform.gateway.metrics.port`                  | 127.0.0.1 (local)     | `platform.gateway.metrics.host` |
|                           | Admin                | 9901          | `platform.gateway.admin.port`                    | 127.0.0.1 (local)     | `platform.gateway.admin.host` |
| **Gateway Rate Limiter**  | gRPC                 | 8081          | (fixed internal)                                 | (internal)            | -               |
| **Rate Limiter Metrics**  | StatsD               | 9125          | (fixed internal)                                 | (internal)            | -               |
|                           | Prometheus           | 9102          | `platform.gateway.rateLimiter.metrics.port`      | 127.0.0.1 (local)     | `platform.gateway.rateLimiter.metrics.host` |
| **Rate Limiter Redis**    | Redis                | 6379          | (fixed internal)                                 | (internal)            | -               |


## Best Practices

1. **Security**: Restrict admin and metrics access to trusted networks
2. **Rate Limiting**: Enable rate limiting in production to prevent abuse
3. **Resource Limits**: Configure appropriate memory and connection limits based on hardware
4. **Logging**: Use JSON format for machine parsing, text format for human reading
5. **SSL**: Use a trusted certificate provider (ZeroSSL) for production deployments
