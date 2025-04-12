# DAPI Platform Endpoints

These endpoints provide access to Dash Platform (Evolution) functionality through the gRPC interface.

## Available Endpoints

### `broadcastStateTransition`

Broadcasts a state transition to the Dash Platform.

**Request Parameters**:
- `stateTransition`: Binary buffer containing serialized state transition (required)

**Response Parameters**: 
- Empty response on success

**Example Usage**:
```javascript
const { broadcastStateTransition } = dapiClient.platform;
const stateTransitionHex = '0200000001...'; // State transition hex
const result = await broadcastStateTransition(stateTransitionHex);
```

### `getConsensusParams`

Retrieves consensus parameters from the Dash Platform.

**Request Parameters**:
- `height`: Height at which to fetch parameters (optional)
- `prove`: Whether to include proofs (not implemented)

**Response Parameters**:
- `blockMaxBytes`: Maximum block size in bytes
- `blockMaxGas`: Maximum gas allowed per block
- `blockTime`: Target block time in milliseconds
- `evidenceMaxAgeNumBlocks`: Maximum age of evidence in blocks
- `evidenceMaxAgeDuration`: Maximum age of evidence in nanoseconds
- `evidenceMaxBytes`: Maximum evidence size in bytes

**Example Usage**:
```javascript
const { getConsensusParams } = dapiClient.platform;
const params = await getConsensusParams();
console.log(`Block time: ${params.blockTime}ms`);
```

### `getStatus`

Gets the status of the Dash Platform.

**Request Parameters**: None

**Response Parameters**:
- `version`: Version information
  - `protocol`: Protocol version
  - `software`: Software version
  - `agent`: User agent string
- `time`: Various time data
  - `now`: Current server time
  - `block`: Latest block time
  - `sync`: Sync status time information
- `status`: General status information
  - `sync`: Sync status
  - `chain`: Chain information
  - `network`: Network information

**Example Usage**:
```javascript
const { getStatus } = dapiClient.platform;
const status = await getStatus();
console.log(`Platform version: ${status.version.software}`);
```

### `waitForStateTransitionResult`

Waits for a state transition to be processed and returns the result.

**Request Parameters**:
- `stateTransitionHash`: Hash of the state transition to wait for (string, required)
- `prove`: Whether to include proof in response (boolean, optional)

**Response Parameters**:
If successful:
- `status`: Status of the state transition (number)
- `proof`: Proof data if requested

If failure:
- `status`: Status of the state transition (number)
- `errorCode`: Error code number
- `errorMessage`: Detailed error message

**Example Usage**:
```javascript
const { waitForStateTransitionResult } = dapiClient.platform;
const hash = '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef';
try {
  const result = await waitForStateTransitionResult(hash);
  console.log(`State transition processed with status: ${result.status}`);
} catch (error) {
  console.error(`State transition failed: ${error.message}`);
}
```

## Unimplemented Endpoints

The following endpoints are defined in the API but are currently unimplemented:

- `getIdentity`
- `getIdentitiesContractKeys`
- `getIdentityBalance`
- `getIdentityBalanceAndRevision`
- `getIdentityKeys`
- `getDocuments`
- `getDataContract`
- `getDataContracts`
- `getDataContractHistory`
- `getIdentityByPublicKeyHash`
- `getIdentitiesByPublicKeyHashes`
- `getProofs`
- `getEpochsInfo`
- `getProtocolVersionUpgradeVoteStatus`
- `getProtocolVersionUpgradeState`
- `getIdentityContractNonce`
- `getIdentityNonce`

These endpoints may be implemented in future releases of DAPI as the Platform functionality expands.