# waitForStateTransitionResult

## Client API

Waits for a state transition to be processed by the platform and returns the result.

**Request Parameters**:
- `stateTransitionHash`: Hash of the state transition to wait for (string, required)
- `prove`: Boolean indicating whether to return a proof (optional, default: false)

**Response Parameters**:
- If successful:
  - `proof`: Merkle proof of the state transition (if requested)
- If error:
  - `error`: Error details object
  - `error.code`: Error code
  - `error.message`: Error message
  - `error.data`: Additional error data

**Example Usage**:
```javascript
const stHash = '4bc5547b87323ef4efd9ef3ebfee4aec53a3e31877f6498126318839a01cd943';
const result = await dapiClient.platform.waitForStateTransitionResult(stHash, true);
if (result.error) {
  console.error(`Error processing state transition: ${result.error.message}`);
} else {
  console.log('State transition processed successfully with proof:', result.proof);
}
```

## Internal Implementation

The `waitForStateTransitionResult` endpoint is implemented in the `waitForStateTransitionResultHandlerFactory.js` file.

### Implementation Details

1. **Wait Mechanism**
   - Uses the `waitForTransactionToBeProvable` function to monitor transaction state
   - Polls Tenderdash until the transaction is either confirmed or rejected
   - Has configurable timeout periods to prevent indefinite waiting

2. **Proof Generation**
   - If the `prove` parameter is true, fetches a cryptographic proof of inclusion
   - Calls Drive to generate the state transition proof
   - This proof can be used for verification without trusting DAPI

3. **Result Processing**
   - Processes both successful and unsuccessful state transitions
   - For successful transactions, returns proof if requested
   - For failed transactions, returns detailed error information

4. **Error Handling**
   - Handles timeout errors with `DeadlineExceededGrpcError`
   - Captures and formats transaction execution errors from Tenderdash
   - Handles Drive unavailability for proof generation

5. **Dependencies**
   - Tenderdash for transaction monitoring
   - Drive for proof generation
   - Internal waitForTransactionToBeProvable utility

### Code Flow

```
Client Request 
  → gRPC Server 
    → waitForStateTransitionResultHandler 
      → Verify Tenderdash Availability
      → Call waitForTransactionToBeProvable
        → Poll Transaction Status
          → If Timeout: Return DeadlineExceededError
          → If Error: Process Transaction Error
          → If Success: 
            → If Prove=true: 
              → Call Drive to Generate Proof
              → Return Proof
            → If Prove=false:
              → Return Empty Success Response
      → Return Response
```

### Transaction Status Monitoring

The handler uses sophisticated techniques to monitor transaction status:

1. **Initial Check**
   - Checks if the transaction is already confirmed
   - Avoids unnecessary waiting for already-processed transactions

2. **Subscription**
   - Subscribes to Tenderdash events to receive real-time updates
   - More efficient than polling for transaction status

3. **Timeout Management**
   - Uses configurable timeouts based on network conditions
   - Provides meaningful errors when timeouts occur

4. **Error Classification**
   - Classifies transaction errors into categories:
     - Validation errors (transaction rejected)
     - System errors (node unavailable)
     - Timeout errors (transaction not confirmed in time)

This comprehensive approach ensures reliable tracking of state transition processing and provides clients with clear information about the outcome.