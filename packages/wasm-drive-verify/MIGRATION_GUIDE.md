# Migration Guide: wasm-drive-verify ES Modules

## Overview

The wasm-drive-verify package now supports ES modules with granular imports, allowing you to significantly reduce bundle sizes by importing only the verification functions you need.

## Benefits

- **Reduced Bundle Size**: Import only what you need, reducing bundle size by up to 84%
- **Better Tree-Shaking**: Modern bundlers can eliminate unused code
- **Faster Load Times**: Smaller bundles mean faster initial page loads
- **Code Splitting**: Use dynamic imports to load modules on demand

## Migration Steps

### Step 1: Update Your Imports

#### Before (Monolithic Import)
```javascript
import { 
  verifyFullIdentityByIdentityId,
  verifyProof,
  verifyContract 
} from 'wasm-drive-verify';
```

#### After (Modular Imports)
```javascript
import { verifyFullIdentityByIdentityId } from 'wasm-drive-verify/identity';
import { verifyProof } from 'wasm-drive-verify/document';
import { verifyContract } from 'wasm-drive-verify/contract';
```

### Step 2: Update Your Bundler Configuration

#### Webpack
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

#### Vite
```javascript
export default {
  optimizeDeps: {
    exclude: ['wasm-drive-verify'],
  },
};
```

#### Rollup
```javascript
import { wasm } from '@rollup/plugin-wasm';

export default {
  plugins: [
    wasm(),
  ],
};
```

## Module Structure

### Available Modules

1. **`wasm-drive-verify/identity`**
   - Identity verification functions
   - Identity balance queries
   - Identity key verification

2. **`wasm-drive-verify/document`**
   - Document proof verification
   - Query verification
   - Single document verification

3. **`wasm-drive-verify/contract`**
   - Contract verification
   - Contract history

4. **`wasm-drive-verify/tokens`**
   - Token balance verification
   - Token info queries
   - Token state verification

5. **`wasm-drive-verify/governance`**
   - Voting verification
   - Group management
   - System state verification

6. **`wasm-drive-verify/transitions`**
   - State transition verification

7. **`wasm-drive-verify/core`**
   - Serialization utilities
   - Common types

## Function Mapping

### Identity Module
- `verifyFullIdentityByIdentityId`
- `verifyFullIdentityByUniquePublicKeyHash`
- `verifyFullIdentityByNonUniquePublicKeyHash`
- `verifyFullIdentitiesByPublicKeyHashes`
- `verifyIdentityBalanceForIdentityId`
- `verifyIdentityBalancesForIdentityIds`
- `verifyIdentityBalanceAndRevisionForIdentityId`
- `verifyIdentityRevisionForIdentityId`
- `verifyIdentityNonce`
- `verifyIdentityContractNonce`
- `verifyIdentityKeysByIdentityId`
- `verifyIdentitiesContractKeys`
- `verifyIdentityIdByUniquePublicKeyHash`
- `verifyIdentityIdByNonUniquePublicKeyHash`
- `verifyIdentityIdsByUniquePublicKeyHashes`

### Document Module
- `verifyProof`
- `verifyProofKeepSerialized`
- `verifyStartAtDocumentInProof`
- `verifySingleDocument`

### Contract Module
- `verifyContract`
- `verifyContractHistory`

### Token Module
- `verifyTokenBalanceForIdentityId`
- `verifyTokenBalancesForIdentityId`
- `verifyTokenBalancesForIdentityIds`
- `verifyTokenInfoForIdentityId`
- `verifyTokenInfosForIdentityId`
- `verifyTokenInfosForIdentityIds`
- `verifyTokenContractInfo`
- `verifyTokenStatus`
- `verifyTokenStatuses`
- `verifyTokenDirectSellingPrice`
- `verifyTokenDirectSellingPrices`
- `verifyTokenPreProgrammedDistributions`
- `verifyTokenPerpetualDistributionLastPaidTime`
- `verifyTokenTotalSupplyAndAggregatedIdentityBalance`

### Governance Module
- Group functions:
  - `verifyGroupInfo`
  - `verifyGroupInfosInContract`
  - `verifyActionSigners`
  - `verifyActionSignersTotalPower`
  - `verifyActiveActionInfos`
- Voting functions:
  - `verifyVotePollVoteStateProof`
  - `verifyVotePollVotesProof`
  - `verifyVotePollsEndDateQuery`
  - `verifyContestsProof`
  - `verifyIdentityVotesGivenProof`
  - `verifyMasternodeVote`
  - `verifySpecializedBalance`
- System functions:
  - `verifyTotalCreditsInSystem`
  - `verifyUpgradeState`
  - `verifyUpgradeVoteStatus`
  - `verifyEpochInfos`
  - `verifyEpochProposers`
  - `verifyElements`

### Transitions Module
- `verifyStateTransitionWasExecutedWithProof`

### Core Module
- `serializeToBytes`
- `deserializeFromBytes`

## Advanced Usage

### Dynamic Imports

Use dynamic imports for code splitting:

```javascript
// Load module only when needed
async function verifyOnDemand(data) {
  const { verifyProof } = await import('wasm-drive-verify/document');
  return verifyProof(data.proof, data.contractId, data.documentType, data.query, data.platformVersion);
}
```

### Conditional Loading

Load modules based on user actions:

```javascript
const verificationHandlers = {
  async identity(data) {
    const { verifyFullIdentityByIdentityId } = await import('wasm-drive-verify/identity');
    return verifyFullIdentityByIdentityId(data.proof, data.identityId, data.platformVersion);
  },
  
  async document(data) {
    const { verifyProof } = await import('wasm-drive-verify/document');
    return verifyProof(data.proof, data.contractId, data.documentType, data.query, data.platformVersion);
  },
  
  async token(data) {
    const { verifyTokenBalanceForIdentityId } = await import('wasm-drive-verify/tokens');
    return verifyTokenBalanceForIdentityId(data.proof, data.contractId, data.identityId, data.platformVersion);
  },
};

// Use based on verification type
const result = await verificationHandlers[verificationType](data);
```

## Troubleshooting

### Issue: Module not found
Ensure you're using the correct import path. All modules should be imported from `wasm-drive-verify/[module-name]`.

### Issue: WASM loading errors
Make sure your bundler is configured to handle WebAssembly modules (see bundler configuration section).

### Issue: Types not found
TypeScript declarations are included with each module. Ensure your `tsconfig.json` has `"moduleResolution": "node"`.

## Performance Tips

1. **Import only what you need**: Each module you import adds to your bundle size
2. **Use dynamic imports**: For features used infrequently, load them on demand
3. **Preload critical modules**: Use `<link rel="modulepreload">` for modules needed at startup
4. **Monitor bundle size**: Use tools like webpack-bundle-analyzer to track your bundle size

## Backwards Compatibility

The default export still includes all modules for backwards compatibility:

```javascript
// This still works but imports everything
import * as wasmDriveVerify from 'wasm-drive-verify';
```

However, we strongly recommend migrating to modular imports for better performance.