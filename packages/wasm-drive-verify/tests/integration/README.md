# Integration Tests

This directory contains integration tests for wasm-drive-verify using real proof data.

## Running Integration Tests

1. Build the WASM module:
   ```bash
   ./build.sh
   ```

2. Run tests:
   ```bash
   # Install wasm-pack if not already installed
   curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
   
   # Run integration tests in browser
   wasm-pack test --chrome --headless -- --test integration_tests
   
   # Or run in Node.js
   wasm-pack test --node -- --test integration_tests
   ```

## Fetching Real Testnet Data

To get real proof data from testnet:

### Option 1: Using Dash SDK (JavaScript)

```javascript
const Dash = require('dash');

const client = new Dash.Client({
  network: 'testnet',
  wallet: { mnemonic: null }
});

// Fetch identity proof
const identity = await client.platform.identities.get('some-identity-id');
const proof = identity.getMetadata().getProof();

// Save proof as base64
const proofBase64 = Buffer.from(proof).toString('base64');
```

### Option 2: Using gRPC directly

You can query testnet nodes directly:
- Testnet seeds: `seed-1.testnet.networks.dash.org:1443`
- Known reliable nodes: See `networkConfigs.js`

### Option 3: Using Dash Platform Explorer

Visit the testnet explorer to find:
- Identity IDs
- Contract IDs  
- Document IDs

Then use the SDK to fetch proofs for specific items.

## Proof Data Format

Proofs are stored in `fixtures/testnet_proofs/` as JSON:

```json
{
  "timestamp": "ISO date",
  "network": "testnet",
  "platformVersion": 1,
  "proofs": {
    "proofName": {
      "description": "What this proof tests",
      "proof": "base64_encoded_proof", 
      "metadata": {
        "identityId": "...",
        "contractId": "..."
      },
      "expectedResult": {
        "hasRootHash": true,
        "hasIdentity": true
      }
    }
  }
}
```

## Known Testnet Resources

### System Contracts
- DPNS: `7133734967411265855288437346261134676850487612170005227449438774554101671041`
- DashPay: `11820826580861527503515256915869415134572226289567404439933090029265983217778`
- Feature Flags: `G7c8S5JDw5FkJEGGeGKCMoHwGbvCNrQrtdbnMirALNV2`

### Test Data
To find test data on testnet:
1. Query DPNS for registered names
2. Look up DashPay profiles
3. Check recent state transitions

## Adding New Test Cases

1. Fetch the proof data from testnet
2. Add it to the appropriate fixture file
3. Create a test case in `integration_tests.rs`
4. Document what the test verifies

## Continuous Updates

Consider setting up a scheduled job to:
1. Fetch fresh proofs from testnet weekly
2. Update fixture files
3. Ensure tests still pass with latest data

This helps catch any breaking changes in proof format or verification logic.