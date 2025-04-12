# DAPI JSON-RPC Endpoints

DAPI provides a set of JSON-RPC endpoints for backward compatibility with traditional blockchain APIs. These endpoints follow the JSON-RPC 2.0 specification.

## JSON-RPC Specification

All requests must follow the JSON-RPC 2.0 format:

```json
{
  "jsonrpc": "2.0",
  "method": "methodName",
  "params": [],
  "id": 1
}
```

Responses will be in the following format:

```json
{
  "jsonrpc": "2.0",
  "result": {},
  "error": null,
  "id": 1
}
```

## Available Endpoints

### `getBestBlockHash`

Returns the hash of the best (tip) block in the longest blockchain.

**Parameters**: None

**Result**: 
- String - the block hash, hex encoded

**Example Request**:
```json
{
  "jsonrpc": "2.0",
  "method": "getBestBlockHash",
  "params": [],
  "id": 1
}
```

**Example Response**:
```json
{
  "jsonrpc": "2.0",
  "result": "0000000000000000000f2adce67e48b6c9ae0f0507c739db9a55f56d85662bc8",
  "error": null,
  "id": 1
}
```

### `getBlockHash`

Returns hash of block in best-block-chain at height provided.

**Parameters**:
1. `height` (number, required) - The height index

**Result**:
- String - The block hash, hex encoded

**Example Request**:
```json
{
  "jsonrpc": "2.0",
  "method": "getBlockHash",
  "params": [1000],
  "id": 1
}
```

**Example Response**:
```json
{
  "jsonrpc": "2.0",
  "result": "00000000000345089c8a91d47b62ca6a1e4d0e8df7f86df9e2e50e62bcdf2887",
  "error": null,
  "id": 1
}
```

## Error Handling

JSON-RPC endpoints return standard error codes as defined in the JSON-RPC 2.0 specification. In addition, DAPI may return additional error details in the error object:

```json
{
  "jsonrpc": "2.0",
  "result": null,
  "error": {
    "code": -32602,
    "message": "Invalid params",
    "data": "Height parameter out of range"
  },
  "id": 1
}
```

Common error codes:
- `-32600`: Invalid Request - The JSON sent is not a valid Request object
- `-32601`: Method not found - The method does not exist / is not available
- `-32602`: Invalid params - Invalid method parameter(s)
- `-32603`: Internal error - Internal JSON-RPC error
- `-32700`: Parse error - Invalid JSON was received by the server

## Connection Details

The JSON-RPC server listens on port 2501 by default (configurable via `API_JSON_RPC_PORT` environment variable).

## Compatibility Notes

These endpoints are designed to provide a subset of the functionality available in Dash Core's JSON-RPC interface. For comprehensive access to all Dash Core functionality, you should connect directly to a Dash Core node's JSON-RPC interface.