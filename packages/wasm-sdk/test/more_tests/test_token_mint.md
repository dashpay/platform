# Token Mint Test Instructions

## Prerequisites
1. The WASM SDK server should be running on http://localhost:8080
2. You need a valid testnet identity ID and private key in WIF format

## Test Steps

1. Open http://localhost:8080 in your browser
2. Select "State Transitions" from the Operation Type dropdown
3. Select "Token Transitions" from the Category dropdown
4. Select "Token Mint" from the Type dropdown

## Fill in the following test values:

### Authentication
- **Identity ID**: Your testnet identity ID (base58 encoded)
- **Private Key (WIF)**: Your private key in WIF format

### Token Mint Parameters
- **Data Contract ID**: The ID of a data contract that has tokens defined
- **Token Contract Position**: 0 (or the position of the token in the contract)
- **Amount to Mint**: 1000000 (or any amount)
- **Key ID**: 0 (or the ID of your authentication key)
- **Issue To Identity ID**: (Optional) Leave empty to mint to yourself
- **Public Note**: (Optional) "Test mint from WASM SDK"

## Expected Result

If successful, you should see a JSON response with one of these formats:

### Standard Token Balance Result
```json
{
  "type": "VerifiedTokenBalance",
  "recipientId": "base58-encoded-identity-id",
  "newBalance": "1000000"
}
```

### Token with History Tracking
```json
{
  "type": "VerifiedTokenActionWithDocument",
  "document": "Document tracking mint history"
}
```

### Group-managed Token Results
```json
{
  "type": "VerifiedTokenGroupActionWithDocument",
  "groupPower": 100,
  "document": true
}
```

or

```json
{
  "type": "VerifiedTokenGroupActionWithTokenBalance",
  "groupPower": 100,
  "status": "Pending",
  "balance": "1000000"
}
```

## Troubleshooting

If you get an error:
1. Check that your identity ID and private key are valid
2. Ensure the data contract ID exists and has tokens defined
3. Verify the token position is correct (starts at 0)
4. Check that your identity has sufficient credits for the transaction
5. Ensure your key ID matches an authentication key in your identity

## Token Burn Test

You can also test token burning by:
1. Select "Token Burn" from the Type dropdown
2. Use the same contract ID and token position
3. Enter an amount to burn (must be less than or equal to your balance)
4. The result should show your new reduced balance