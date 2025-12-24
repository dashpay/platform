# Test Fixtures

This directory contains real proof data from Dash testnet for integration testing.

## Structure

- `testnet_proofs/` - Real proofs fetched from testnet
  - `identity_proofs.json` - Identity verification proofs
  - `document_proofs.json` - Document query proofs
  - `contract_proofs.json` - Data contract proofs
  
## Fetching New Proofs

To update the test fixtures with fresh testnet data:

1. Install dependencies:
   ```bash
   npm install dash
   ```

2. Run the fetch script:
   ```bash
   node ../integration/fetch_testnet_proofs.js
   ```

## Proof Format

Proofs are stored as JSON with the following structure:

```json
{
  "timestamp": "2024-01-20T10:00:00Z",
  "network": "testnet",
  "platformVersion": 1,
  "proofs": {
    "proofType": {
      "description": "Human-readable description",
      "proof": "base64_encoded_proof_bytes",
      "metadata": {
        // Additional context like IDs, parameters used
      },
      "expectedResult": {
        // What we expect the verification to return
      }
    }
  }
}
```

## Known Testnet Resources

### Contracts
- DPNS: `7133734967411265855288437346261134676850487612170005227449438774554101671041`
- DashPay: `11820826580861527503515256915869415134572226289567404439933090029265983217778`

### Test Identities
- (To be populated with known testnet identities)

### Test Documents
- (To be populated with known testnet documents)