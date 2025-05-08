# broadcastStateTransition

## Client API

Broadcasts a state transition to the Dash Platform.

**Request Parameters**:
- `stateTransition`: Binary buffer containing serialized state transition (required)

**Response Parameters**:
- Empty response on success

**Example Usage**:
```javascript
const stateTransitionHex = '0300...'; // Raw state transition hex
const result = await dapiClient.platform.broadcastStateTransition(stateTransitionHex);
console.log('State transition broadcast successfully');
```

## Internal Implementation

The `broadcastStateTransition` endpoint is implemented in the `broadcastStateTransitionHandlerFactory.js` file.

### Implementation Details

1. **State Transition Processing**
   - Takes a state transition (ST) byte array as input
   - Converts the state transition to base64 format for Tenderdash compatibility
   - Sends the encoded state transition to Tenderdash via the `broadcast_tx` RPC call

2. **Validation Flow**
   - Tenderdash performs initial validation during the `broadcast_tx` call
   - If validation fails, DAPI may perform a `check_tx` call to get detailed error information
   - This provides clients with specific validation error messages

3. **Error Handling**
   - Handles various error scenarios with specialized error responses:
     - State transition already in mempool ("tx already exists in cache")
     - State transition already in blockchain
     - Transaction too large
     - Mempool full
     - Broadcast timeout
   - Maps Tenderdash errors to appropriate gRPC error codes
   - Returns detailed error information to help diagnose issues

4. **Dependencies**
   - Tenderdash RPC interface for transaction broadcasting
   - Drive for state transition validation

### Code Flow

```
Client Request 
  → gRPC Server 
    → broadcastStateTransitionHandler 
      → Validate State Transition Format
      → Convert to Base64 Encoding
      → Call Tenderdash broadcast_tx RPC
        → If Successful: 
          → Return Empty Response
        → If Error:
          → Try check_tx for Detailed Error (optional)
          → Map Tenderdash Error to gRPC Error
          → Return Error Response
```

### Common Error Scenarios

The handler processes several common error conditions:

1. **Already in Mempool**
   - Error message: "tx already exists in cache"
   - Indicates the state transition is already being processed
   - Returns ALREADY_EXISTS gRPC error

2. **Validation Failures**
   - Various error messages depending on the specific validation issue
   - Usually relates to state transition structure or signature problems
   - Returns INVALID_ARGUMENT or FAILED_PRECONDITION errors

3. **System Capacity Issues**
   - "mempool is full" or "tx too large"
   - Indicates system resource constraints
   - Returns RESOURCE_EXHAUSTED error

4. **Timeout Errors**
   - Occurs when the broadcast operation exceeds time limits
   - Returns DEADLINE_EXCEEDED error

These detailed error responses help clients understand exactly why a state transition broadcast failed and how to resolve the issue.