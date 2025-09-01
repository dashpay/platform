DAPI (Distributed API) server for Dash Platform

Provides gRPC, REST, and JSON-RPC endpoints for blockchain and platform data.

CONFIGURATION:
Server configuration is based on environment variables that can be set in the 
environment or saved in a .env file. Use 'rs-dapi config' to see current values.

ENVIRONMENT VARIABLES:
Server Configuration:
  DAPI_GRPC_SERVER_PORT       - gRPC API server port (default: 3005)
  DAPI_GRPC_STREAMS_PORT      - gRPC streams server port (default: 3006)  
  DAPI_JSON_RPC_PORT          - JSON-RPC server port (default: 3004)
  DAPI_REST_GATEWAY_PORT      - REST API server port (default: 8080)
  DAPI_HEALTH_CHECK_PORT      - Health check port (default: 9090)
  DAPI_BIND_ADDRESS           - IP address to bind to (default: 127.0.0.1)

Service Configuration:
  DAPI_ENABLE_REST            - Enable REST API (default: false)
  DAPI_DRIVE_URI              - Drive service URI (default: http://127.0.0.1:6000)
  DAPI_TENDERDASH_URI         - Tenderdash RPC URI (default: http://127.0.0.1:26657)
  DAPI_TENDERDASH_WEBSOCKET_URI - Tenderdash WebSocket URI (default: ws://127.0.0.1:26657/websocket)
  DAPI_CORE_ZMQ_URL           - Dash Core ZMQ URL (default: tcp://127.0.0.1:29998)
  DAPI_CORE_RPC_URL           - Dash Core JSON-RPC URL (default: http://127.0.0.1:9998)
  DAPI_CORE_RPC_USER          - Dash Core RPC username (default: empty)
  DAPI_CORE_RPC_PASS          - Dash Core RPC password (default: empty)
  DAPI_STATE_TRANSITION_WAIT_TIMEOUT - Timeout in ms (default: 30000)

CONFIGURATION LOADING:
1. Command line environment variables (highest priority)
2. .env file variables (specified with --config or .env in current directory)
3. Default values (lowest priority)

EXAMPLES:
  rs-dapi                                    # Start with defaults
  rs-dapi --config /etc/dapi/production.env # Use custom config
  rs-dapi -vv start                          # Start with trace logging
  rs-dapi config                             # Show current configuration
  rs-dapi --help                             # Show this help
