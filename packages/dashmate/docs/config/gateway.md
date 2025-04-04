# Platform Gateway Configuration

The `platform.gateway` section configures the Dash Platform Gateway, which serves as the entry point for external clients to access Dash Platform services.

## Docker

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.gateway.docker.image` | Docker image for Gateway | `dashpay/envoy:1.30.2-impr.1` | `dashpay/envoy:latest` |

## Listeners

The listener configuration controls the API endpoints exposed by the Gateway:

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.gateway.listeners.dapiAndDrive.port` | Gateway API port | `443` | `8443` |
| `platform.gateway.listeners.dapiAndDrive.host` | Gateway API host binding | `0.0.0.0` | `127.0.0.1` |
| `platform.gateway.listeners.dapiAndDrive.http2.maxConcurrentStreams` | Max concurrent HTTP/2 streams per connection | `10` | `100` |
| `platform.gateway.listeners.dapiAndDrive.waitForStResultTimeout` | Timeout for state transition results | `125s` | `300s` |

Host binding notes:
- Setting `0.0.0.0` allows connections from any IP address
- Setting `127.0.0.1` restricts connections to localhost only

## Performance

These settings control resource allocation and limits for the Gateway:

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.gateway.maxConnections` | Maximum connections from clients | `1000` | `2000` |
| `platform.gateway.maxHeapSizeInBytes` | Maximum heap size in bytes | `125000000` | `250000000` |

## Upstream

The upstreams configuration controls connections to backend services:

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.gateway.upstreams.driveGrpc.maxRequests` | Maximum parallel requests to Drive gRPC | `100` | `200` |
| `platform.gateway.upstreams.dapiApi.maxRequests` | Maximum parallel requests to DAPI API | `100` | `200` |
| `platform.gateway.upstreams.dapiCoreStreams.maxRequests` | Maximum parallel requests to DAPI Core streams | `100` | `200` |
| `platform.gateway.upstreams.dapiJsonRpc.maxRequests` | Maximum parallel requests to DAPI JSON-RPC | `100` | `200` |

## Metrics

These settings control the metrics endpoint for monitoring the Gateway:

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.gateway.metrics.enabled` | Enable metrics server | `false` | `true` |
| `platform.gateway.metrics.host` | Host binding for metrics server | `127.0.0.1` | `0.0.0.0` |
| `platform.gateway.metrics.port` | Port for metrics server | `9090` | `9091` |

Metrics provide performance and health information about the Gateway service.
Admin must be enabled to access the metrics endpoint.

## Admin

These settings control the admin interface for the Gateway:

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.gateway.admin.enabled` | Enable admin interface | `false` | `true` |
| `platform.gateway.admin.host` | Host binding for admin interface | `127.0.0.1` | `0.0.0.0` |
| `platform.gateway.admin.port` | Port for admin interface | `9901` | `9902` |

The admin interface allows for runtime configuration and statistics retrieval.

## Logging

These settings control logging for the Gateway:

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.gateway.log.level` | Log level for gateway logs | `info` | `debug` |
| `platform.gateway.log.accessLogs` | Access log configuration | `[]` | See example below |

The `accessLogs` array can contain multiple log configurations with different destinations and formats.

### Access Logs Configuration

Each log entry in the `accessLogs` array has the following properties:

**For stdout/stderr outputs:**
- `type`: Output destination - `stdout` or `stderr`
- `format`: Log format - `text` or `json`
- `template`: Template string or object for formatting logs

**For file outputs:**
- `type`: Output destination - `file`
- `format`: Log format - `text` or `json`
- `path`: Absolute path to log file on host machine
- `template`: Template string or object for formatting logs

Access logs example for text and JSON formats:
```json
"platform.gateway.log.accessLogs": [
  {
    "type": "stdout",
    "format": "text",
    "template": "[%START_TIME%] '%REQ(:METHOD)% %REQ(X-ENVOY-ORIGINAL-PATH?:PATH)% %PROTOCOL%' %RESPONSE_CODE% %BYTES_RECEIVED% %BYTES_SENT% %DURATION% %RESP(X-ENVOY-UPSTREAM-SERVICE-TIME)% '%REQ(X-FORWARDED-FOR)%' '%REQ(USER-AGENT)%'"
  },
  {
    "type": "file",
    "format": "json",
    "path": "/var/log/envoy/access.log",
    "template": {
      "timestamp": "%START_TIME%",
      "method": "%REQ(:METHOD)%",
      "path": "%REQ(X-ENVOY-ORIGINAL-PATH?:PATH)%",
      "protocol": "%PROTOCOL%",
      "responseCode": "%RESPONSE_CODE%",
      "bytesReceived": "%BYTES_RECEIVED%",
      "bytesSent": "%BYTES_SENT%",
      "duration": "%DURATION%",
      "upstream": "%RESP(X-ENVOY-UPSTREAM-SERVICE-TIME)%",
      "client": "%REQ(X-FORWARDED-FOR)%",
      "userAgent": "%REQ(USER-AGENT)%"
    }
  }
]
```

See the [Envoy access log documentation](https://www.envoyproxy.io/docs/envoy/latest/configuration/observability/access_log/usage) for more information on available template variables.

Available log levels: `trace`, `debug`, `info`, `warn`, `error`, `critical`, `off`

## SSL

These settings control SSL/TLS for secure connections:

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.gateway.ssl.enabled` | Enable SSL | `false` | `true` |
| `platform.gateway.ssl.provider` | SSL provider | `zerossl` | `selfSigned` |

### ZeroSSL Provider Configuration

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.gateway.ssl.providerConfigs.zerossl.apiKey` | ZeroSSL API key | `null` | `"your-api-key"` |
| `platform.gateway.ssl.providerConfigs.zerossl.id` | ZeroSSL certificate ID | `null` | `"certificate_id"` |

Available SSL providers:
- `zerossl`: Commercial certificate provider with automated issuance
- `selfSigned`: Self-signed certificates (not trusted by browsers)
- `file`: Use existing certificate files (requires certificate and key files to be manually provided)

## Rate Limiter

The rate limiter protects the Platform from excessive requests:

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.gateway.rateLimiter.enabled` | Enable rate limiter | `true` | `false` |
| `platform.gateway.rateLimiter.docker.image` | Docker image for rate limiter | `envoyproxy/ratelimit:3fcc3609` | `envoyproxy/ratelimit:latest` |
| `platform.gateway.rateLimiter.requestsPerUnit` | Requests allowed per time unit | `150` | `300` |
| `platform.gateway.rateLimiter.unit` | Time unit for rate limiting | `minute` | `hour` |
| `platform.gateway.rateLimiter.whitelist` | IPs exempt from rate limiting | `[]` | `["192.168.1.1"]` |
| `platform.gateway.rateLimiter.blacklist` | IPs blocked from all requests | `[]` | `["10.0.0.1"]` |

Available time units:
- `second`: Per-second rate limiting
- `minute`: Per-minute rate limiting
- `hour`: Per-hour rate limiting
- `day`: Per-day rate limiting

## Rate Limiter Metrics

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `platform.gateway.rateLimiter.metrics.enabled` | Enable metrics for rate limiter | `false` | `true` |
| `platform.gateway.rateLimiter.metrics.docker.image` | Docker image for rate limiter metrics | `prom/statsd-exporter:v0.26.1` | `prom/statsd-exporter:latest` |
| `platform.gateway.rateLimiter.metrics.host` | Host binding for metrics | `127.0.0.1` | `0.0.0.0` |
| `platform.gateway.rateLimiter.metrics.port` | Port for metrics | `9102` | `9103` |
