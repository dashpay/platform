# wasm-drive-verify

WASM bindings for Drive verification functions with ES module support.

## Overview

This package provides WebAssembly bindings for all the verification functions available in the rs-drive crate. It enables JavaScript/TypeScript applications to verify proofs from the Dash Platform.

### Key Features

- **ES Modules Support**: Import only what you need, reducing bundle size by up to 84%
- **Tree-Shaking**: Modern bundlers can eliminate unused code
- **TypeScript Support**: Full type definitions for all functions
- **Modular Architecture**: Organized into logical verification categories

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

## Installation

```bash
npm install wasm-drive-verify
```

## Usage

### ES Modules (Recommended)

Import only the functions you need for optimal bundle size:

```javascript
// Import specific functions from specific modules
import { verifyFullIdentityByIdentityId } from 'wasm-drive-verify/identity';
import { verifyContract } from 'wasm-drive-verify/contract';

// Use the functions
const identityResult = await verifyFullIdentityByIdentityId(proof, identityId, platformVersion);
const contractResult = await verifyContract(proof, contractId, platformVersion);
```

### Non-unique public key hash lookup

```javascript
// Use the generated web binding at the path where your build serves pkg/.
import init, { verifyFullIdentityByNonUniquePublicKeyHash } from './pkg/wasm_drive_verify.js';
await init();
const result = verifyFullIdentityByNonUniquePublicKeyHash(
  proofBytes, publicKeyHashBytes, startAfterBytes, producerProtocolVersion, verificationProtocolVersion,
);
```

This direct verifier accepts one complete GroveDB proof blob: the standard `Proof`
of a V1 answer. It proves the first matching identity after the exclusive cursor,
including its keys, balance and revision, or complete absence. It does not accept
a V0 answer, which carries separate index and identity proofs. The SDK sends V0
until it has verified a signed answer from protocol 14 or later. For a V0 answer,
the proof that `getIdentityByNonUniquePublicKeyHashWithProofInfo` returns is the
index proof alone, and this function cannot verify it.
The producer version is the authenticated response metadata protocol version.
The verification version is the caller's security table, normally latest supported
and never below its verification floor. Authenticate the returned root and
metadata against a trusted signed block separately; this
standalone function does not verify the quorum signature. High-level SDK proof
verification also authenticates the root and metadata.

The package identity facade still has unrelated imports that do not match the
generated binding names. Use the directly built binding for this API; the
package exports do not expose a `pkg` subpath.

The targeted identity-wrapper test runs locally with
`node --test packages/wasm-drive-verify/tests/identity-wrapper.test.mjs` from the
repository root, or `node --test tests/identity-wrapper.test.mjs` from this package.
It mocks unrelated bindings and does not validate the whole facade.

### Dynamic Imports

Load modules on-demand for code splitting:

```javascript
// Load module only when needed
const { verifyProof } = await import('wasm-drive-verify/document');
const result = await verifyProof(proof, contractId, documentType, query, platformVersion);
```

### Legacy Usage (Imports Everything)

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

## Building from Source

```bash
# Build with ES modules support
npm run build:modules

# Build standard WASM
npm run build
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

## Bundle Size Benefits

Using ES modules can significantly reduce your bundle size:

| Import Method | Bundle Size | Reduction |
|--------------|------------|-----------|
| Full Import | ~2.5MB | Baseline |
| Identity Only | ~400KB | 84% |
| Document Only | ~350KB | 86% |
| Multiple Modules | ~600KB | 76% |

## Bundler Configuration

### Webpack
```javascript
module.exports = {
  experiments: {
    asyncWebAssembly: true,
  },
  module: {
    rules: [
      {
        test: /\.wasm$/,
        type: 'webassembly/async',
      },
    ],
  },
};
```

### Vite
```javascript
export default {
  optimizeDeps: {
    exclude: ['wasm-drive-verify'],
  },
};
```

### Next.js
```javascript
module.exports = {
  webpack: (config) => {
    config.experiments = {
      ...config.experiments,
      asyncWebAssembly: true,
    };
    return config;
  },
};
```

## Documentation

- [Migration Guide](./MIGRATION_GUIDE.md) - Migrate from monolithic to modular imports
- [Bundle Size Analysis](./BUNDLE_SIZE_ANALYSIS.md) - Detailed analysis of bundle size improvements
- [ES Modules Plan](./ES_MODULES_PLAN.md) - Implementation strategy and technical details

## License

MIT
