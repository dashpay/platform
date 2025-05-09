# getConsensusParams

## Client API

Retrieves the platform consensus parameters.

**Request Parameters**:
- `height`: Block height to retrieve parameters for (optional, defaults to latest)

**Response Parameters**:
- `blockMaxBytes`: Maximum block size in bytes
- `blockMaxGas`: Maximum block gas limit
- `blockTimeIotaMs`: Block time parameter in milliseconds
- `evidenceMaxAgeDuration`: Maximum age of evidence in nanoseconds
- `evidenceMaxAgeNumBlocks`: Maximum age of evidence in blocks
- `evidenceMaxBytes`: Maximum evidence size in bytes
- `validatorSlashAmount`: Amount to slash validators by

**Example Usage**:
```javascript
// Get latest consensus parameters
const latestParams = await dapiClient.platform.getConsensusParams();
console.log(`Max block size: ${latestParams.blockMaxBytes} bytes`);

// Get consensus parameters at a specific height
const heightParams = await dapiClient.platform.getConsensusParams(1000);
console.log(`Max block size at height 1000: ${heightParams.blockMaxBytes} bytes`);
```

## Internal Implementation

The `getConsensusParams` endpoint is implemented in the `getConsensusParamsFactory.js` file.

### Implementation Details

1. **Parameter Retrieval**
   - Calls the `getConsensusParams` function which interfaces with Tenderdash
   - Can retrieve parameters at the latest height or at a specific historical height
   - Makes a `consensus_params` RPC call to Tenderdash

2. **Parameter Processing**
   - Converts raw Tenderdash parameters to the DAPI gRPC format
   - Handles duration formats and converts them to standard time units
   - Organizes parameters into logical categories

3. **Input Validation**
   - Validates that the height parameter (if provided) is a positive integer
   - Returns appropriate error responses for invalid inputs

4. **Error Handling**
   - Maps Tenderdash RPC errors to appropriate gRPC error codes
   - Handles connection issues with detailed error messages
   - Returns NOT_FOUND errors for requests for non-existent heights

5. **Dependencies**
   - Tenderdash RPC interface for consensus parameter data

### Code Flow

```
Client Request 
  → gRPC Server 
    → getConsensusParamsHandler 
      → Validate Height Parameter (if provided)
      → Call getConsensusParams Function
        → Call Tenderdash consensus_params RPC
          → With height if specified
          → Without height for latest params
        → Process Response
        → Format Parameters
      → Build Response Object
      → Return Consensus Parameters
```

### Consensus Parameter Categories

The parameters are organized into several categories:

1. **Block Parameters**
   - `blockMaxBytes`: Maximum allowed block size
   - `blockMaxGas`: Maximum gas allowed per block
   - `blockTimeIotaMs`: Minimum time increment between blocks

2. **Evidence Parameters**
   - `evidenceMaxAgeDuration`: Maximum age of evidence in time
   - `evidenceMaxAgeNumBlocks`: Maximum age of evidence in blocks
   - `evidenceMaxBytes`: Maximum size of evidence

3. **Validator Parameters**
   - `validatorSlashAmount`: Amount to slash validators for misbehavior

These parameters govern the fundamental rules of the blockchain, controlling aspects like performance, security, and economic incentives. They can change over time through governance processes, which is why the API allows retrieving parameters at specific heights.