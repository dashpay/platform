# getTransaction

## Client API

Retrieves a transaction by ID.

**Request Parameters**:
- `id`: Transaction ID (string, required)

**Response Parameters**:
- `transaction`: Binary buffer containing serialized transaction
- `blockHash`: Hash of the block containing the transaction
- `height`: Height of the block containing the transaction
- `confirmations`: Number of confirmations
- `isInstantLocked`: Whether transaction is instant locked
- `isChainLocked`: Whether transaction is chain locked

**Example Usage**:
```javascript
const txId = '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef';
const tx = await dapiClient.core.getTransaction(txId);
console.log(`Transaction is in block: ${tx.height}`);
```

## Internal Implementation

The `getTransaction` endpoint is implemented in the `getTransactionHandlerFactory.js` file.

### Implementation Details

1. **Input Validation**
   - Validates that the transaction ID is a valid 64-character hex string
   - Returns appropriate error responses for invalid inputs

2. **Core RPC Interaction**
   - Calls Core's `getrawtransaction` RPC method with `verbose=1` flag
   - This returns both the raw transaction data and metadata about its inclusion in the blockchain

3. **Response Transformation**
   - Converts the Core RPC response to the DAPI gRPC format
   - Maps fields from Core's response to DAPI's response structure
   - Adds additional fields like InstantLock and ChainLock status

4. **Error Handling**
   - Maps Core RPC error codes to appropriate gRPC error codes
   - Handles "transaction not found" (-5) errors specifically
   - Provides descriptive error messages for debugging

5. **Dependencies**
   - Dash Core RPC interface for transaction data
   - InsightAPI (optional fallback) if transaction is not found in Core

### Code Flow

```
Client Request 
  → gRPC Server 
    → getTransactionHandler 
      → Validate Transaction ID Format
      → Call getrawtransaction RPC with verbose=1
        → If Found: Process Response
          → Extract Transaction Data
          → Extract Block Information
          → Check Lock Statuses
          → Build Response
        → If Not Found: Return Not Found Error
      → Return Transaction Response
```

### Transaction Lock Status

The handler checks two important security features of Dash transactions:

1. **InstantLock Status**
   - Indicates if the transaction has been locked by InstantSend
   - Provides immediate transaction security without waiting for confirmations

2. **ChainLock Status**
   - Indicates if the block containing the transaction is protected by ChainLocks
   - Provides protection against 51% attacks and chain reorganizations

These statuses help clients determine the security level of a transaction.