# getBlockHash

## Client API

Returns the hash of the block at the specified height in the local best blockchain.

**Request Format**:
```json
{
  "jsonrpc": "2.0",
  "method": "getBlockHash",
  "params": [1000],
  "id": 1
}
```

**Response Format**:
```json
{
  "jsonrpc": "2.0",
  "result": "00000000000000437b4c6fa42c9e5095844c1ed847417cead17612d7b153643e",
  "id": 1
}
```

**Example Usage**:
```javascript
// Using fetch API
const response = await fetch('http://localhost:2501', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    jsonrpc: '2.0',
    method: 'getBlockHash',
    params: [1000],
    id: 1
  })
});
const data = await response.json();
console.log(`Block hash at height 1000: ${data.result}`);
```

## Internal Implementation

The `getBlockHash` endpoint is implemented in the `getBlockHash.js` file.

### Implementation Details

1. **Input Validation**
   - Validates the input parameters against a schema
   - Requires a single height parameter that must be a non-negative integer
   - Returns appropriate error responses for invalid inputs

2. **Core Integration**
   - Interfaces with Dash Core via RPC to get the block hash
   - Calls Core's `getBlockHash` method with the provided height

3. **Direct Delegation**
   - This endpoint is a simple pass-through to the Core RPC
   - After validation, it directly calls the corresponding Core method with the same parameters

4. **Error Handling**
   - Maps Core RPC errors to JSON-RPC compatible error responses
   - Handles "block height out of range" errors specifically
   - Provides descriptive error messages to help diagnose issues

5. **Dependencies**
   - Dash Core RPC interface
   - JSON schema validator for parameter validation

### Code Flow

```
Client Request 
  → JSON-RPC Server 
    → getBlockHash Handler 
      → Validate Height Parameter
        → If Invalid: Return Error Response
      → Call Core RPC getBlockHash with height
        → If Successful: Return Block Hash
        → If Error: Map to JSON-RPC Error Response
```

### Schema Validation

The endpoint uses a JSON schema to validate the input:

```json
{
  "type": "array",
  "items": {
    "type": "number",
    "minimum": 0
  },
  "minItems": 1,
  "maxItems": 1
}
```

This ensures that:
- The params field is an array
- It contains exactly one item
- The item is a number (block height)
- The height is non-negative

### Endpoint Categorization

This endpoint is categorized as a Layer 1 (L1) endpoint in the API documentation, indicating that it interacts directly with the Dash Core blockchain functionality.