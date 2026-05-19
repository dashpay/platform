# Tutorial: Creating a Basic Token

> **Environment:** This tutorial uses **Node.js** scripts to deploy a contract
> and perform token operations. For browser-based applications, deploy the
> contract from a Node.js script first, then use the contract ID in your
> frontend code.

Create a fungible token on Dash Platform with minting, transferring, and
balance queries. This tutorial walks through the full lifecycle from contract
deployment to token operations.

## What you will learn

- Defining a data contract with a token configuration
- Minting tokens to an identity
- Transferring tokens between identities
- Querying balances and supply

## Prerequisites

```sh
npm install @dashevo/evo-sdk
```

You need a funded testnet identity with enough credits to deploy a contract and
perform token operations.

## Step 1: Define the token contract

A token is defined as part of a data contract. The contract schema includes a
`tokens` section alongside the usual document schemas.

```typescript
import {
  EvoSDK, DataContract, Identifier, IdentitySigner,
  TokenConfigurationConvention, TokenConfigurationLocalization, TokenConfiguration,
  ChangeControlRules, AuthorizedActionTakers, TokenDistributionRules,
  TokenKeepsHistoryRules, TokenMarketplaceRules, TokenTradeMode,
} from '@dashevo/evo-sdk';

const sdk = EvoSDK.testnetTrusted();
await sdk.connect();

const identityId = 'YOUR_IDENTITY_ID';
const privateKeyWif = 'YOUR_PRIVATE_KEY_WIF';
const signingKeyIndex = 0;

// Define a contract with a token
const contractSchema = {
  // Document types (optional — a token-only contract can have none)
  tokenMetadata: {
    type: 'object',
    properties: {
      tokenName: { type: 'string', maxLength: 63, position: 0 },
      description: { type: 'string', maxLength: 256, position: 1 },
    },
    additionalProperties: false,
  },
};

// Build the token configuration using SDK classes
const localization = new TokenConfigurationLocalization(true, 'CoffeeCoin', 'CoffeeCoins');
const conventions = new TokenConfigurationConvention({ en: localization }, 2);

const ownerOnly = new ChangeControlRules({
  authorizedToMakeChange: AuthorizedActionTakers.ContractOwner(),
  adminActionTakers: AuthorizedActionTakers.ContractOwner(),
});
const noOne = new ChangeControlRules({
  authorizedToMakeChange: AuthorizedActionTakers.NoOne(),
  adminActionTakers: AuthorizedActionTakers.NoOne(),
});

const tokenConfig = new TokenConfiguration({
  conventions,
  conventionsChangeRules: noOne,
  baseSupply: 0n,
  maxSupply: 1_000_000_00n,  // 1,000,000.00 with 2 decimals
  maxSupplyChangeRules: noOne,
  keepsHistory: new TokenKeepsHistoryRules({
    isKeepingMintingHistory: true,
    isKeepingBurningHistory: true,
    isKeepingTransferHistory: true,
  }),
  distributionRules: new TokenDistributionRules({
    perpetualDistributionRules: noOne,
    newTokensDestinationIdentityRules: noOne,
    mintingAllowChoosingDestination: true,
    mintingAllowChoosingDestinationRules: noOne,
    changeDirectPurchasePricingRules: noOne,
  }),
  marketplaceRules: new TokenMarketplaceRules(TokenTradeMode.NotTradeable(), noOne),
  manualMintingRules: ownerOnly,
  manualBurningRules: ownerOnly,
  freezeRules: noOne,
  unfreezeRules: noOne,
  destroyFrozenFundsRules: noOne,
  emergencyActionRules: noOne,
  mainControlGroupCanBeModified: AuthorizedActionTakers.NoOne(),
});
```

## Step 2: Publish the contract

```typescript
// Set up signing
const identity = await sdk.identities.fetch(identityId);
const identityKey = identity.publicKeys[signingKeyIndex];
const signer = new IdentitySigner();
signer.addKeyFromWif(privateKeyWif);

const nonce = await sdk.identities.nonce(identityId);
const dataContract = new DataContract({
  ownerId: new Identifier(identityId),
  identityNonce: nonce + 1n,
  schemas: contractSchema,
  tokens: { 0: tokenConfig },
});
const contract = await sdk.contracts.publish({ dataContract, identityKey, signer });

const contractId = contract.id.toString();
console.log('Contract published:', contractId);
```

## Step 3: Mint tokens

The contract owner can mint tokens to any identity:

```typescript
// Token operations require a CRITICAL security level key.
// Fetch a key with the appropriate security level from the identity.
const criticalKey = identity.publicKeys[signingKeyIndex];
const criticalSigner = new IdentitySigner();
criticalSigner.addKeyFromWif(privateKeyWif);

// Mint 10,000.00 CoffeeCoins to yourself
await sdk.tokens.mint({
  dataContractId: new Identifier(contractId),
  tokenPosition: 0,
  amount: 10_000_00n,          // 10,000.00 (2 decimal places) — must be bigint
  recipientId: new Identifier(identityId),
  identityId: new Identifier(identityId),
  identityKey: criticalKey,
  signer: criticalSigner,
});

console.log('Minted 10,000 CoffeeCoins');
```

### Mint to another identity

```typescript
await sdk.tokens.mint({
  dataContractId: new Identifier(contractId),
  tokenPosition: 0,
  amount: 500_00n,             // 500.00 CoffeeCoins
  recipientId: new Identifier('RECIPIENT_IDENTITY_ID'),
  identityId: new Identifier(identityId),
  identityKey: criticalKey,
  signer: criticalSigner,
});
```

## Step 4: Check balances

```typescript
// Check your own balance
const myBalances = await sdk.tokens.identityBalances(identityId, [contractId]);
let myBalance = 0n;
for (const [id, balance] of myBalances) {
  if (id.toString() === contractId) myBalance = balance;
}
console.log('My balance:', Number(myBalance) / 100, 'CoffeeCoins');

// Check multiple identities at once
const balances = await sdk.tokens.balances(
  [identityId, 'OTHER_IDENTITY_ID'],
  contractId,
);

for (const [id, balance] of balances) {
  console.log(`${id}: ${Number(balance) / 100} CoffeeCoins`);
}
```

### Check total supply

```typescript
const tokenId = await sdk.tokens.calculateId(contractId, 0);
const supply = await sdk.tokens.totalSupply(tokenId);
if (supply) {
  console.log('Total supply:', Number(supply.totalSupply) / 100, 'CoffeeCoins');
}
```

## Step 5: Transfer tokens

```typescript
await sdk.tokens.transfer({
  dataContractId: new Identifier(contractId),
  tokenPosition: 0,
  amount: 25_00n,              // 25.00 CoffeeCoins
  recipientId: new Identifier('RECIPIENT_IDENTITY_ID'),
  senderId: new Identifier(identityId),
  identityKey: criticalKey,
  signer: criticalSigner,
});

console.log('Transferred 25 CoffeeCoins');
```

## Step 6: Burn tokens

Reduce the supply by burning tokens you own:

```typescript
await sdk.tokens.burn({
  dataContractId: new Identifier(contractId),
  tokenPosition: 0,
  amount: 100_00n,             // 100.00 CoffeeCoins
  identityId: new Identifier(identityId),
  identityKey: criticalKey,
  signer: criticalSigner,
});

console.log('Burned 100 CoffeeCoins');
```

## Full example

Putting it all together as a complete script:

```typescript
import { EvoSDK, Identifier, IdentitySigner } from '@dashevo/evo-sdk';

async function main() {
  const sdk = EvoSDK.testnetTrusted();
  await sdk.connect();

  const identityId = 'YOUR_IDENTITY_ID';
  const privateKeyWif = 'YOUR_PRIVATE_KEY_WIF';
  const contractId = 'YOUR_CONTRACT_ID';  // from step 2

  // Set up signing (token ops require a CRITICAL security level key)
  const identity = await sdk.identities.fetch(identityId);
  const identityKey = identity.publicKeys[0];
  const signer = new IdentitySigner();
  signer.addKeyFromWif(privateKeyWif);

  // Check balance
  const balances = await sdk.tokens.identityBalances(identityId, [contractId]);
  for (const [id, balance] of balances) {
    if (id.toString() === contractId) console.log('Balance:', balance);
  }

  // Transfer
  await sdk.tokens.transfer({
    dataContractId: new Identifier(contractId),
    tokenPosition: 0,
    amount: 10_00n,
    recipientId: new Identifier('FRIEND_IDENTITY_ID'),
    senderId: new Identifier(identityId),
    identityKey,
    signer,
  });

  console.log('Transfer complete!');
}

main().catch(console.error);
```

## Next steps

- Add **freeze/unfreeze** capabilities for compliance scenarios
- Set up a **direct purchase price** so anyone can buy tokens with credits
- Create a **distribution schedule** for automatic token rewards
- Use the `tokenMetadata` document type to store on-chain metadata
