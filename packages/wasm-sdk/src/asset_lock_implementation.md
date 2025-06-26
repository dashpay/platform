# Asset Lock Proof Implementation

## Overview
Successfully implemented asset lock proof deserialization and integration with the dpp crate's native AssetLockProof types.

## Key Changes

### 1. Refactored to Use Native DPP Types
- Replaced custom implementation with wrapper around `dpp::identity::state_transition::asset_lock_proof::AssetLockProof`
- Now supports both `InstantAssetLockProof` and `ChainAssetLockProof` types

### 2. Proper Serialization/Deserialization
- Using `bincode` for binary serialization (compatible with DPP)
- Using `dashcore::consensus::Encodable/Decodable` for transaction and instant lock serialization
- Added JSON serialization support for JavaScript interop

### 3. New API Methods
- `createInstant()` - Create instant asset lock proof from transaction and instant lock
- `createChain()` - Create chain asset lock proof from height and outpoint
- `toBytes()/fromBytes()` - Binary serialization
- `toJSON()/fromJSON()` - JSON serialization
- `getIdentityId()` - Get the identity ID that will be created from this proof
- `calculateCreditsFromProof()` - Calculate platform credits from proof value

### 4. Helper Functions
- `createOutPoint()` - Create outpoint from transaction ID and index
- `createInstantProofFromParts()` - Helper accepting hex strings or Uint8Arrays
- `createChainProofFromParts()` - Helper for creating chain proofs

## Usage Example

```javascript
// Create instant asset lock proof
const transaction = "..."; // hex string or Uint8Array
const instantLock = "..."; // hex string or Uint8Array
const outputIndex = 0;

const proof = AssetLockProof.createInstant(transaction, outputIndex, instantLock);

// Get identity ID that will be created
const identityId = proof.getIdentityId();

// Calculate credits
const credits = calculateCreditsFromProof(proof);

// Serialize for storage/transport
const bytes = proof.toBytes();
const proofRestored = AssetLockProof.fromBytes(bytes);
```

## Integration Points
- Ready to be used in identity creation state transitions
- Compatible with platform's proof verification
- Properly handles both testnet and mainnet configurations