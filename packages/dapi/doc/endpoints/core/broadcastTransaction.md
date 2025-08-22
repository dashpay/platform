# broadcastTransaction

## Client API

Broadcasts a transaction to the Dash network.

**Request Parameters**:
- `transaction`: Binary buffer containing serialized transaction (required)

**Response Parameters**:
- `transactionId`: ID of the broadcast transaction (string)

**Example Usage**:
```javascript
const txHex = '0200000001...'; // Raw transaction hex
const result = await dapiClient.core.broadcastTransaction(txHex);
console.log(`Transaction broadcast with ID: ${result.transactionId}`);
```

## Internal Implementation

The `broadcastTransaction` endpoint is implemented in the `broadcastTransactionHandlerFactory.js` file.

### Implementation Details

1. **Transaction Validation**
   - Validates that the transaction is properly formatted and serialized
   - Performs basic structure checks before attempting to broadcast

2. **Core RPC Interaction**
   - Calls Core's `sendrawtransaction` RPC method
   - Passes the raw transaction data to the Core node for propagation to the network

3. **Error Handling**
   - Maps Core RPC error codes to appropriate gRPC error codes
   - Handles common error scenarios:
     - Invalid transaction format (RPC error code -22)
     - Transaction rejected (RPC error code -26)
     - Transaction already in blockchain (RPC error code -27)
     - Transaction validation failures (RPC error code -25)
   - Provides descriptive error messages to help diagnose transaction issues

4. **Dependencies**
   - Dash Core RPC interface for transaction broadcasting

### Code Flow

```
Client Request 
  → gRPC Server 
    → broadcastTransactionHandler 
      → Validate Transaction Format
      → Convert Binary to Hex (if needed)
      → Call sendrawtransaction RPC
        → If Successful: 
          → Return Transaction ID
        → If Error:
          → Map RPC Error to gRPC Error
          → Return Error Response
```

### Common Error Scenarios

The handler maps Core RPC errors to meaningful client responses:

1. **Invalid Transaction Format (-22)**
   - Occurs when the transaction data is malformed
   - Returns INVALID_ARGUMENT error with details

2. **Transaction Rejected (-26)**
   - Occurs when the transaction is valid but rejected by the network
   - Common causes include double spends, insufficient fees, or script verification failures
   - Returns FAILED_PRECONDITION error with specific rejection reason

3. **Already in Blockchain (-27)**
   - Occurs when attempting to broadcast a transaction that's already confirmed
   - Returns ALREADY_EXISTS error

4. **Transaction Validation Error (-25)**
   - Occurs when the transaction fails internal validation checks
   - Returns INVALID_ARGUMENT error with validation details

This error mapping helps clients understand exactly why a transaction broadcast failed.