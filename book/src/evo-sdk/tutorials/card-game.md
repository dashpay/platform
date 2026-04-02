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
const gameSchema = {
  card: {
    type: 'object',
    properties: {
      name:    { type: 'string', maxLength: 64 },
      element: { type: 'string', enum: ['fire', 'water', 'earth', 'air', 'shadow'] },
      rarity:  { type: 'string', enum: ['common', 'uncommon', 'rare', 'legendary'] },
      power:   { type: 'integer', minimum: 1, maximum: 100 },
      defense: { type: 'integer', minimum: 1, maximum: 100 },
      ability: { type: 'string', maxLength: 128 },
      edition: { type: 'integer', minimum: 1 },
    },
    required: ['name', 'element', 'rarity', 'power', 'defense', 'edition'],
    additionalProperties: false,
  },
  deck: {
    type: 'object',
    properties: {
      name:    { type: 'string', maxLength: 64 },
      cardIds: {
        type: 'array',
        items: { type: 'string', maxLength: 44 },
        minItems: 5,
        maxItems: 10,
      },
    },
    required: ['name', 'cardIds'],
    additionalProperties: false,
  },
  match: {
    type: 'object',
    properties: {
      player1Id:  { type: 'string', maxLength: 44 },
      player2Id:  { type: 'string', maxLength: 44 },
      winnerId:   { type: 'string', maxLength: 44 },
      player1Score: { type: 'integer', minimum: 0 },
      player2Score: { type: 'integer', minimum: 0 },
      timestamp:  { type: 'integer' },
    },
    required: ['player1Id', 'player2Id', 'winnerId', 'timestamp'],
    additionalProperties: false,
  },
};

const gemTokenConfig = {
  conventions: {
    localizations: {
      en: {
        shouldCapitalize: true,
        singularForm: 'Gem',
        pluralForm: 'Gems',
      },
    },
    decimals: 0, // whole numbers only
  },
  manualMinting: {
    rules: { type: 'ownerOnly' },
  },
  manualBurning: {
    rules: { type: 'ownerOnly' },
  },
  maxSupply: 10_000_000, // 10 million Gems total
};
```

## Step 2: Deploy the contract

```typescript
import { EvoSDK } from '@dashevo/evo-sdk';

const sdk = EvoSDK.testnetTrusted();
await sdk.connect();

// Game operator identity
const operatorId = 'OPERATOR_IDENTITY_ID';
const operatorKey = 'OPERATOR_PRIVATE_KEY_WIF';

const contract = await sdk.contracts.publish({
  identityId: operatorId,
  documentSchemas: gameSchema,
  tokens: [gemTokenConfig],
  privateKeyWif: operatorKey,
  signingKeyIndex: 0,
  nonce: await sdk.identities.nonce(operatorId),
});

const contractId = contract.getId().toString();
const gemTokenId = await sdk.tokens.calculateId(contractId, 0);

console.log('Game contract:', contractId);
console.log('Gem token:', gemTokenId);
```

## Step 3: Mint starter Gems for a new player

When a player joins, give them starter Gems:

```typescript
async function onboardPlayer(playerId: string) {
  // Gift 100 Gems to the new player
  await sdk.tokens.mint({
    tokenId: gemTokenId,
    amount: 100,
    recipientId: playerId,
    identityId: operatorId,
    privateKeyWif: operatorKey,
    signingKeyIndex: 0,
    nonce: await sdk.identities.nonce(operatorId),
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
    await sdk.documents.create({
      contractId,
      documentType: 'card',
      document: card,
      identityId: operatorId,
      privateKeyWif: operatorKey,
      signingKeyIndex: 0,
      nonce: await sdk.identities.contractNonce(operatorId, contractId),
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
const PACK_PRICE = 50; // 50 Gems per pack

async function buyPack(playerId: string, playerKey: string) {
  // Player pays Gems to the operator
  await sdk.tokens.transfer({
    tokenId: gemTokenId,
    amount: PACK_PRICE,
    recipientId: operatorId,
    identityId: playerId,
    privateKeyWif: playerKey,
    signingKeyIndex: 0,
    nonce: await sdk.identities.nonce(playerId),
  });
  console.log(`Player paid ${PACK_PRICE} Gems`);

  // Operator transfers cards to the player
  // (In production, select random cards from available pool)
  const availableCards = await sdk.documents.query({
    contractId,
    documentType: 'card',
    where: [['$ownerId', '==', operatorId]],
    limit: 5,
  });

  for (const [cardId, card] of availableCards) {
    if (!card) continue;
    await sdk.documents.transfer({
      contractId,
      documentType: 'card',
      documentId: cardId,
      recipientId: playerId,
      identityId: operatorId,
      privateKeyWif: operatorKey,
      signingKeyIndex: 0,
      nonce: await sdk.identities.contractNonce(operatorId, contractId),
    });
    console.log(`Transferred ${card.getData().name} to player`);
  }
}
```

## Step 6: Query a player's collection

```typescript
async function getCollection(playerId: string) {
  const cards = await sdk.documents.query({
    contractId,
    documentType: 'card',
    where: [['$ownerId', '==', playerId]],
    orderBy: [['power', 'desc']],
    limit: 100,
  });

  console.log(`\n${playerId}'s collection:`);
  for (const [id, card] of cards) {
    if (!card) continue;
    const d = card.getData();
    console.log(`  [${d.rarity}] ${d.name} — ${d.element} — ATK:${d.power} DEF:${d.defense}`);
  }

  return cards;
}
```

### Filter by rarity

```typescript
const legendaries = await sdk.documents.query({
  contractId,
  documentType: 'card',
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
  // Player A sends their card to Player B
  await sdk.documents.transfer({
    contractId,
    documentType: 'card',
    documentId: fromCardId,
    recipientId: toId,
    identityId: fromId,
    privateKeyWif: fromKey,
    signingKeyIndex: 0,
    nonce: await sdk.identities.contractNonce(fromId, contractId),
  });

  // Player B sends their card to Player A
  await sdk.documents.transfer({
    contractId,
    documentType: 'card',
    documentId: toCardId,
    recipientId: fromId,
    identityId: toId,
    privateKeyWif: toKey,
    signingKeyIndex: 0,
    nonce: await sdk.identities.contractNonce(toId, contractId),
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
  await sdk.documents.create({
    contractId,
    documentType: 'match',
    document: {
      player1Id,
      player2Id,
      winnerId,
      player1Score: p1Score,
      player2Score: p2Score,
      timestamp: Date.now(),
    },
    identityId: operatorId,
    privateKeyWif: operatorKey,
    signingKeyIndex: 0,
    nonce: await sdk.identities.contractNonce(operatorId, contractId),
  });

  // Reward the winner with Gems
  await sdk.tokens.mint({
    tokenId: gemTokenId,
    amount: 10,
    recipientId: winnerId,
    identityId: operatorId,
    privateKeyWif: operatorKey,
    signingKeyIndex: 0,
    nonce: await sdk.identities.nonce(operatorId),
  });

  console.log(`Match recorded. ${winnerId} wins and earns 10 Gems!`);
}
```

## Step 9: Leaderboard

Query match history to build a win count:

```typescript
async function getWinCounts() {
  const matches = await sdk.documents.query({
    contractId,
    documentType: 'match',
    orderBy: [['timestamp', 'desc']],
    limit: 100,
  });

  const wins = new Map<string, number>();
  for (const [, doc] of matches) {
    if (!doc) continue;
    const winner = doc.getData().winnerId;
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
