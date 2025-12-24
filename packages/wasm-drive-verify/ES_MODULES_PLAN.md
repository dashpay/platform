# ES Modules Implementation Plan for wasm-drive-verify

## Overview
This plan outlines the strategy to reduce package size and improve tree-shaking by implementing ES modules and splitting the wasm-drive-verify package into logical sub-modules.

## Current State Analysis

### Package Size Issues
- The current monolithic WASM bundle includes all verification functions
- Users importing one function still load the entire bundle
- No tree-shaking possible with current structure

### Existing Module Structure (Rust)
We already have logical separation in the Rust code:
- `identity/` - Identity verification functions
- `document/` - Document and query verification
- `contract/` - Contract verification
- `tokens/` - Token-related verification
- `voting/` - Voting and governance verification
- `group/` - Group management verification
- `system/` - System-level verification
- `state_transition/` - State transition verification

## Implementation Strategy

### Phase 1: ES Module Configuration

#### 1.1 Update package.json
```json
{
  "name": "wasm-drive-verify",
  "version": "1.8.0",
  "type": "module",
  "exports": {
    ".": {
      "import": "./dist/index.js",
      "types": "./dist/index.d.ts"
    },
    "./identity": {
      "import": "./dist/identity.js",
      "types": "./dist/identity.d.ts"
    },
    "./document": {
      "import": "./dist/document.js",
      "types": "./dist/document.d.ts"
    },
    "./contract": {
      "import": "./dist/contract.js",
      "types": "./dist/contract.d.ts"
    },
    "./tokens": {
      "import": "./dist/tokens.js",
      "types": "./dist/tokens.d.ts"
    },
    "./governance": {
      "import": "./dist/governance.js",
      "types": "./dist/governance.d.ts"
    },
    "./transitions": {
      "import": "./dist/transitions.js",
      "types": "./dist/transitions.d.ts"
    },
    "./core": {
      "import": "./dist/core.js",
      "types": "./dist/core.d.ts"
    }
  },
  "sideEffects": false
}
```

#### 1.2 Configure wasm-pack for ES modules
Update build.sh to use ES module target:
```bash
wasm-pack build --target web --out-dir pkg --out-name wasm_drive_verify
```

### Phase 2: Create Module Entry Points

#### 2.1 Core Module (Always Loaded)
- Serialization utilities
- Common types and interfaces
- WASM initialization logic

#### 2.2 Identity Module
- All identity verification functions
- Identity key verification
- Identity balance verification

#### 2.3 Document Module
- Document proof verification
- Query verification
- Single document verification

#### 2.4 Contract Module
- Contract verification
- Contract history verification

#### 2.5 Token Module
- Token balance verification
- Token info verification
- Token state verification

#### 2.6 Governance Module
- Voting verification
- Group management verification
- System state verification

#### 2.7 Transitions Module
- State transition verification
- Execution path queries

### Phase 3: JavaScript Wrapper Implementation

Create separate JS entry points that lazy-load WASM chunks:

```javascript
// identity.js
let wasm;

async function initWasm() {
  if (!wasm) {
    wasm = await import('./wasm_drive_verify_identity.js');
    await wasm.default();
  }
  return wasm;
}

export async function verifyFullIdentityByIdentityId(proof, identityId, platformVersion) {
  const { verify_full_identity_by_identity_id } = await initWasm();
  return verify_full_identity_by_identity_id(proof, identityId, platformVersion);
}
// ... other identity functions
```

### Phase 4: Build Process Updates

#### 4.1 Multi-target wasm-pack builds
Create separate WASM builds for each module using conditional compilation:

```toml
# Cargo.toml features
[features]
default = ["full"]
full = ["identity", "document", "contract", "tokens", "governance", "transitions"]
identity = []
document = []
contract = []
tokens = []
governance = ["voting", "group", "system"]
transitions = []
```

#### 4.2 Build script updates
```bash
#!/bin/bash
# Build each module separately
wasm-pack build --target web --out-dir pkg/identity --features identity --no-default-features
wasm-pack build --target web --out-dir pkg/document --features document --no-default-features
# ... etc
```

### Phase 5: Bundle Size Optimization

#### 5.1 Analyze current bundle
- Use webpack-bundle-analyzer or similar
- Identify largest functions/modules
- Find optimization opportunities

#### 5.2 Code splitting strategies
- Lazy load heavy verification functions
- Share common dependencies between modules
- Minimize WASM instantiation overhead

### Phase 6: Migration Guide

#### 6.1 Before (current usage)
```javascript
import { verifyFullIdentityByIdentityId } from 'wasm-drive-verify';
```

#### 6.2 After (ES modules)
```javascript
// Option 1: Direct import (best for tree-shaking)
import { verifyFullIdentityByIdentityId } from 'wasm-drive-verify/identity';

// Option 2: Dynamic import (best for code splitting)
const { verifyFullIdentityByIdentityId } = await import('wasm-drive-verify/identity');
```

## Expected Benefits

1. **Reduced Initial Load**
   - Users only load the verification functions they need
   - ~70-80% reduction in bundle size for typical use cases

2. **Better Tree-Shaking**
   - Bundlers can eliminate unused functions
   - Dead code elimination at module level

3. **Improved Performance**
   - Faster WASM instantiation
   - Lower memory footprint
   - Better caching strategies

4. **Developer Experience**
   - Clear module boundaries
   - Better TypeScript support
   - Easier to understand API surface

## Testing Strategy

1. **Unit Tests**
   - Test each module in isolation
   - Verify lazy loading works correctly
   - Ensure no cross-module dependencies

2. **Bundle Analysis**
   - Measure bundle sizes before/after
   - Verify tree-shaking effectiveness
   - Test with different bundlers (webpack, rollup, vite)

3. **Integration Tests**
   - Test migration path
   - Verify backward compatibility
   - Test in real applications

## Timeline

- Week 1: Implement ES module configuration and build process
- Week 2: Create JavaScript wrappers and entry points
- Week 3: Testing and optimization
- Week 4: Documentation and migration guide

## Future Considerations

### Potential Package Splitting
If modules are still too large, consider splitting into separate npm packages:
- `@dashpay/wasm-drive-verify-core`
- `@dashpay/wasm-drive-verify-identity`
- `@dashpay/wasm-drive-verify-document`
- etc.

### WebAssembly Component Model
When stable, migrate to the Component Model for better modularity and smaller binaries.