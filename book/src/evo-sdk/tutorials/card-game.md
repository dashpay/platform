# Tutorial: Card Game with Tokens

Build a collectible card game on Dash Platform where cards are documents that
can be traded, and an in-game currency token is used for purchases. This
tutorial combines data contracts, documents, and tokens into a cohesive
application.

> **Environment:** Steps 1-2 (contract design and deployment) are run from a
> **Node.js script** using a developer/operator identity. Steps 3 onward
> (minting, trading, querying) can run in either Node.js or a **browser app**
> using the published contract ID.

## What you will learn

- Designing a contract with both document types and tokens
- Using documents as game items (cards) owned by identities
- Token-based in-game economy (minting rewards, spending on packs)
- Document transfers for card trading between players
- Querying collections and leaderboards

## Prerequisites

```sh
npm install @dashevo/evo-sdk
```

You need a funded testnet identity. This tutorial uses two identities to
demonstrate trading.

## Step 1: Design the game contract

The contract defines three document types and one token:

- **card** — A collectible card with rarity, power, and element
- **deck** — A player's active deck configuration
- **match** — Match result history
- **GemToken** — In-game currency for buying card packs

```typescript
import {
  TokenConfigurationConvention, TokenConfigurationLocalization, TokenConfiguration,
  ChangeControlRules, AuthorizedActionTakers, TokenDistributionRules,
  TokenKeepsHistoryRules, TokenMarketplaceRules, TokenTradeMode,
} from '@dashevo/evo-sdk';

const gameSchema = {
  card: {
    type: 'object',
    properties: {
      name:    { type: 'string', maxLength: 63, position: 0 },
      element: { type: 'string', enum: ['fire', 'water', 'earth', 'air', 'shadow'], position: 1 },
      rarity:  { type: 'string', enum: ['common', 'uncommon', 'rare', 'legendary'], position: 2 },
      power:   { type: 'integer', minimum: 1, maximum: 100, position: 3 },
      defense: { type: 'integer', minimum: 1, maximum: 100, position: 4 },
      ability: { type: 'string', maxLength: 128, position: 5 },
      edition: { type: 'integer', minimum: 1, position: 6 },
    },
    required: ['name', 'element', 'rarity', 'power', 'defense', 'edition'],
    additionalProperties: false,
  },
  deck: {
    type: 'object',
    properties: {
      name:    { type: 'string', maxLength: 63, position: 0 },
      cardIds: {
        type: 'array',
        items: { type: 'string', maxLength: 44 },
        minItems: 5,
        maxItems: 10,
        position: 1,
      },
    },
    required: ['name', 'cardIds'],
    additionalProperties: false,
  },
  match: {
    type: 'object',
    properties: {
      player1Id:    { type: 'string', maxLength: 44, position: 0 },
      player2Id:    { type: 'string', maxLength: 44, position: 1 },
      winnerId:     { type: 'string', maxLength: 44, position: 2 },
      player1Score: { type: 'integer', minimum: 0, position: 3 },
      player2Score: { type: 'integer', minimum: 0, position: 4 },
      timestamp:    { type: 'integer', position: 5 },
    },
    required: ['player1Id', 'player2Id', 'winnerId', 'timestamp'],
    additionalProperties: false,
  },
};

// Build the token configuration using SDK classes
const localization = new TokenConfigurationLocalization(true, 'Gem', 'Gems');
const conventions = new TokenConfigurationConvention({ en: localization }, 0);

const ownerOnly = new ChangeControlRules({
  authorizedToMakeChange: AuthorizedActionTakers.ContractOwner(),
  adminActionTakers: AuthorizedActionTakers.ContractOwner(),
});
const noOne = new ChangeControlRules({
  authorizedToMakeChange: AuthorizedActionTakers.NoOne(),
  adminActionTakers: AuthorizedActionTakers.NoOne(),
});

const gemTokenConfig = new TokenConfiguration({
  conventions,
  conventionsChangeRules: noOne,
  baseSupply: 0n,
  maxSupply: 10_000_000n,  // 10 million Gems total
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

## Step 2: Deploy the contract

```typescript
import { EvoSDK, DataContract, Document, Identifier, IdentitySigner } from '@dashevo/evo-sdk';

const sdk = EvoSDK.testnetTrusted();
await sdk.connect();

// Game operator identity
const operatorId = 'OPERATOR_IDENTITY_ID';
const operatorKey = 'OPERATOR_PRIVATE_KEY_WIF';

// Set up signing
const operatorIdentity = await sdk.identities.fetch(operatorId);
const operatorIdentityKey = operatorIdentity.publicKeys[0];
const operatorSigner = new IdentitySigner();
operatorSigner.addKeyFromWif(operatorKey);

const nonce = await sdk.identities.nonce(operatorId);
const dataContract = new DataContract({
  ownerId: new Identifier(operatorId),
  identityNonce: nonce + 1n,
  schemas: gameSchema,
  tokens: { 0: gemTokenConfig },
});
const contract = await sdk.contracts.publish({
  dataContract,
  identityKey: operatorIdentityKey,
  signer: operatorSigner,
});

const contractId = contract.id.toString();

console.log('Game contract:', contractId);
```

## Step 3: Mint starter Gems for a new player

When a player joins, give them starter Gems:

```typescript
async function onboardPlayer(playerId: string) {
  // Token operations require a CRITICAL security level key
  // Gift 100 Gems to the new player
  await sdk.tokens.mint({
    dataContractId: new Identifier(contractId),
    tokenPosition: 0,
    amount: 100n,
    recipientId: new Identifier(playerId),
    identityId: new Identifier(operatorId),
    identityKey: operatorIdentityKey,
    signer: operatorSigner,
  });

  console.log(`Welcomed ${playerId} with 100 Gems`);
}
```

## Step 4: Create a card pack (operator mints cards)

The operator creates cards as documents. Each card is owned by the operator
initially, then transferred to players when purchased.

```typescript
// Define a set of cards for a pack
const starterPack = [
  { name: 'Flame Sprite',    element: 'fire',   rarity: 'common',   power: 15, defense: 10, edition: 1 },
  { name: 'Tidal Guardian',  element: 'water',  rarity: 'common',   power: 10, defense: 20, edition: 1 },
  { name: 'Stone Golem',     element: 'earth',  rarity: 'uncommon', power: 25, defense: 30, edition: 1 },
  { name: 'Wind Dancer',     element: 'air',    rarity: 'common',   power: 20, defense: 12, edition: 1 },
  { name: 'Shadow Wraith',   element: 'shadow', rarity: 'rare',     power: 40, defense: 15, edition: 1 },
];

async function createCards(cards: typeof starterPack) {
  for (const card of cards) {
    const cardDoc = new Document({
      documentTypeName: 'card',
      dataContractId: new Identifier(contractId),
      ownerId: new Identifier(operatorId),
      properties: card,
    });
    await sdk.documents.create({
      document: cardDoc,
      identityKey: operatorIdentityKey,
      signer: operatorSigner,
    });
    console.log(`Created: ${card.name} (${card.rarity})`);
  }
}

await createCards(starterPack);
```

## Step 5: Player buys a card pack

The purchase flow:
1. Player spends Gems (transfer to operator)
2. Operator transfers card documents to the player

```typescript
const PACK_PRICE = 50n; // 50 Gems per pack

async function buyPack(playerId: string, playerKey: string) {
  // Set up player signing (token ops require CRITICAL security level key)
  const playerIdentity = await sdk.identities.fetch(playerId);
  const playerIdentityKey = playerIdentity.publicKeys[0];
  const playerSigner = new IdentitySigner();
  playerSigner.addKeyFromWif(playerKey);

  // Player pays Gems to the operator
  await sdk.tokens.transfer({
    dataContractId: new Identifier(contractId),
    tokenPosition: 0,
    amount: PACK_PRICE,
    recipientId: new Identifier(operatorId),
    senderId: new Identifier(playerId),
    identityKey: playerIdentityKey,
    signer: playerSigner,
  });
  console.log(`Player paid ${PACK_PRICE} Gems`);

  // Operator transfers cards to the player
  // (In production, select random cards from available pool)
  const availableCards = await sdk.documents.query({
    dataContractId: contractId,
    documentTypeName: 'card',
    where: [['$ownerId', '==', operatorId]],
    limit: 5,
  });

  for (const [cardId, card] of availableCards) {
    if (!card) continue;
    await sdk.documents.transfer({
      document: card,
      recipientId: new Identifier(playerId),
      identityKey: operatorIdentityKey,
      signer: operatorSigner,
    });
    const props = card.properties as Record<string, unknown>;
    console.log(`Transferred ${props.name} to player`);
  }
}
```

## Step 6: Query a player's collection

```typescript
async function getCollection(playerId: string) {
  const cards = await sdk.documents.query({
    dataContractId: contractId,
    documentTypeName: 'card',
    where: [['$ownerId', '==', playerId]],
    orderBy: [['power', 'desc']],
    limit: 100,
  });

  console.log(`\n${playerId}'s collection:`);
  for (const [id, card] of cards) {
    if (!card) continue;
    const d = card.properties as Record<string, unknown>;
    console.log(`  [${d.rarity}] ${d.name} — ${d.element} — ATK:${d.power} DEF:${d.defense}`);
  }

  return cards;
}
```

### Filter by rarity

```typescript
const legendaries = await sdk.documents.query({
  dataContractId: contractId,
  documentTypeName: 'card',
  where: [
    ['$ownerId', '==', playerId],
    ['rarity', '==', 'legendary'],
  ],
  limit: 50,
});
```

## Step 7: Trade cards between players

Player-to-player trading using document transfers:

```typescript
async function tradeCards(
  fromId: string, fromKey: string, fromCardId: string,
  toId: string, toKey: string, toCardId: string,
) {
  // Set up signers for both players
  const fromIdentity = await sdk.identities.fetch(fromId);
  const fromIdentityKey = fromIdentity.publicKeys[0];
  const fromSigner = new IdentitySigner();
  fromSigner.addKeyFromWif(fromKey);

  const toIdentity = await sdk.identities.fetch(toId);
  const toIdentityKey = toIdentity.publicKeys[0];
  const toSigner = new IdentitySigner();
  toSigner.addKeyFromWif(toKey);

  // Fetch both card documents
  const fromCard = await sdk.documents.get(contractId, 'card', fromCardId);
  const toCard = await sdk.documents.get(contractId, 'card', toCardId);

  // Player A sends their card to Player B
  await sdk.documents.transfer({
    document: fromCard,
    recipientId: new Identifier(toId),
    identityKey: fromIdentityKey,
    signer: fromSigner,
  });

  // Player B sends their card to Player A
  await sdk.documents.transfer({
    document: toCard,
    recipientId: new Identifier(fromId),
    identityKey: toIdentityKey,
    signer: toSigner,
  });

  console.log('Trade complete!');
}
```

## Step 8: Record a match result

```typescript
async function recordMatch(
  player1Id: string, player2Id: string,
  winnerId: string,
  p1Score: number, p2Score: number,
) {
  const matchDoc = new Document({
    documentTypeName: 'match',
    dataContractId: new Identifier(contractId),
    ownerId: new Identifier(operatorId),
    properties: {
      player1Id,
      player2Id,
      winnerId,
      player1Score: p1Score,
      player2Score: p2Score,
      timestamp: Date.now(),
    },
  });
  await sdk.documents.create({
    document: matchDoc,
    identityKey: operatorIdentityKey,
    signer: operatorSigner,
  });

  // Reward the winner with Gems (token ops require CRITICAL security level key)
  await sdk.tokens.mint({
    dataContractId: new Identifier(contractId),
    tokenPosition: 0,
    amount: 10n,
    recipientId: new Identifier(winnerId),
    identityId: new Identifier(operatorId),
    identityKey: operatorIdentityKey,
    signer: operatorSigner,
  });

  console.log(`Match recorded. ${winnerId} wins and earns 10 Gems!`);
}
```

## Step 9: Leaderboard

Query match history to build a win count:

```typescript
async function getWinCounts() {
  const matches = await sdk.documents.query({
    dataContractId: contractId,
    documentTypeName: 'match',
    orderBy: [['timestamp', 'desc']],
    limit: 100,
  });

  const wins = new Map<string, number>();
  for (const [, doc] of matches) {
    if (!doc) continue;
    const props = doc.properties as Record<string, unknown>;
    const winner = props.winnerId as string;
    wins.set(winner, (wins.get(winner) ?? 0) + 1);
  }

  // Sort by wins descending
  const sorted = [...wins.entries()].sort((a, b) => b[1] - a[1]);
  console.log('\nLeaderboard:');
  sorted.forEach(([id, count], i) => {
    console.log(`  ${i + 1}. ${id.slice(0, 8)}... — ${count} wins`);
  });
}
```

## Architecture recap

```text
┌──────────────────────────────────────────────────┐
│                  Game Contract                    │
├──────────────────┬───────────────┬───────────────┤
│  card (document) │ deck (doc)    │ match (doc)   │
│  - name, element │ - cardIds[]   │ - players     │
│  - rarity, power │               │ - winner      │
│  - transferable  │               │ - scores      │
├──────────────────┴───────────────┴───────────────┤
│  GemToken (token position 0)                     │
│  - in-game currency                              │
│  - minted as rewards, spent on packs             │
└──────────────────────────────────────────────────┘
```

## Next steps

- Add **deck validation** — check that a deck only contains cards the player owns
- Implement **card pricing** with `sdk.documents.setPrice()` for a marketplace
- Add **seasonal editions** with different `edition` numbers
- Build a real-time game client that listens for match results
- Use **groups** for guild/clan systems with shared card pools
