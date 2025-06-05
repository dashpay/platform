# wasm-drive-verify

WASM bindings for Drive verification functions.

## Overview

This package provides WebAssembly bindings for all the verification functions available in the rs-drive crate. It enables JavaScript/TypeScript applications to verify proofs from the Dash Platform.

## Modules

The package is organized into the following modules:

- **contract** - Verify data contracts and contract history
- **document** - Verify documents and document queries
- **identity** - Verify identities, balances, keys, and related data
- **single_document** - Verify single document proofs
- **system** - Verify system elements, epochs, and upgrade states
- **group** - Verify group actions and signers
- **state_transition** - Verify state transition execution
- **tokens** - Verify token balances, info, and statuses
- **voting** - Verify voting polls, contests, and votes

## Usage

### Building

```bash
npm run build
# or
./build.sh
```

### Example

```javascript
import init, { verifyContract } from './pkg/wasm_drive_verify.js';

await init();

const result = verifyContract(
  proofBytes,
  false, // contract_known_keeps_history
  false, // is_proof_subset
  false, // in_multiple_contract_proof_form
  contractIdBytes,
  1 // platform_version
);

console.log('Root hash:', result.root_hash);
console.log('Contract:', result.contract);
```

## Generic Functions

Many verification functions support generic return types. These functions have two variants:

- **Vec variant** - Returns results as JavaScript arrays of tuples
- **Map variant** - Returns results as JavaScript objects with hex/string keys

Example:
```javascript
// Vec variant - returns [[publicKeyHash, identity], ...]
const vecResult = verifyFullIdentitiesByPublicKeyHashesVec(proof, hashes, version);

// Map variant - returns { "hex_key": identity, ... }
const mapResult = verifyFullIdentitiesByPublicKeyHashesMap(proof, hashes, version);
```

## License

MIT