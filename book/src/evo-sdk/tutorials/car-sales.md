# Tutorial: Car Sales Management

Build a decentralised car listing and sales application on Dash Platform. By
the end you will have a data contract for vehicle listings, the ability to
create/query/update listings, and a purchase flow using document transfers.

## What you will learn

- Designing a data contract with multiple document types
- Publishing a contract to testnet
- Creating, querying, and updating documents
- Using document pricing and purchase for a sales flow

## Prerequisites

```sh
npm install @dashevo/evo-sdk
```

You need a funded testnet identity. See the
[Getting Started](../getting-started.md) chapter for setup.

## Step 1: Design the data contract

A car sales contract needs two document types: **listings** (vehicles for sale)
and **reviews** (buyer reviews of sellers).

```typescript
const carSalesSchema = {
  listing: {
    type: 'object',
    properties: {
      make:        { type: 'string', maxLength: 64 },
      model:       { type: 'string', maxLength: 64 },
      year:        { type: 'integer', minimum: 1900, maximum: 2100 },
      mileageKm:   { type: 'integer', minimum: 0 },
      priceUsd:    { type: 'integer', minimum: 0 },
      description: { type: 'string', maxLength: 1024 },
      imageUrl:    { type: 'string', maxLength: 512, format: 'uri' },
      status:      { type: 'string', enum: ['available', 'pending', 'sold'] },
    },
    required: ['make', 'model', 'year', 'priceUsd', 'status'],
    additionalProperties: false,
  },
  review: {
    type: 'object',
    properties: {
      sellerId:  { type: 'string', maxLength: 44 },
      listingId: { type: 'string', maxLength: 44 },
      rating:    { type: 'integer', minimum: 1, maximum: 5 },
      comment:   { type: 'string', maxLength: 512 },
    },
    required: ['sellerId', 'rating'],
    additionalProperties: false,
  },
};
```

## Step 2: Connect and publish the contract

```typescript
import { EvoSDK, wallet } from '@dashevo/evo-sdk';

const sdk = EvoSDK.testnetTrusted();
await sdk.connect();

// Your identity credentials
const identityId = 'YOUR_IDENTITY_ID';
const privateKeyWif = 'YOUR_PRIVATE_KEY_WIF';
const signingKeyIndex = 0;

// Publish the data contract
const contract = await sdk.contracts.publish({
  identityId,
  documentSchemas: carSalesSchema,
  privateKeyWif,
  signingKeyIndex,
  nonce: await sdk.identities.nonce(identityId),
});

const contractId = contract.getId().toString();
console.log('Contract published:', contractId);
```

Save the `contractId` — you will need it for all subsequent operations.

## Step 3: Create a listing

```typescript
const nonce = await sdk.identities.contractNonce(identityId, contractId);

await sdk.documents.create({
  contractId,
  documentType: 'listing',
  document: {
    make: 'Toyota',
    model: 'Camry',
    year: 2021,
    mileageKm: 45000,
    priceUsd: 22500,
    description: 'Well-maintained, single owner, full service history.',
    status: 'available',
  },
  identityId,
  privateKeyWif,
  signingKeyIndex,
  nonce,
});

console.log('Listing created!');
```

## Step 4: Query listings

```typescript
// Fetch all available listings
const results = await sdk.documents.query({
  contractId,
  documentType: 'listing',
  where: [['status', '==', 'available']],
  orderBy: [['priceUsd', 'asc']],
  limit: 20,
});

for (const [id, doc] of results) {
  if (!doc) continue;
  const data = doc.getData();
  console.log(`${data.year} ${data.make} ${data.model} — $${data.priceUsd}`);
  console.log(`  ID: ${id}`);
}
```

### Search by make

```typescript
const toyotas = await sdk.documents.query({
  contractId,
  documentType: 'listing',
  where: [
    ['make', '==', 'Toyota'],
    ['status', '==', 'available'],
  ],
  limit: 10,
});
```

## Step 5: Update a listing

Mark a listing as sold:

```typescript
const listingId = 'THE_LISTING_DOCUMENT_ID';

await sdk.documents.replace({
  contractId,
  documentType: 'listing',
  documentId: listingId,
  document: {
    make: 'Toyota',
    model: 'Camry',
    year: 2021,
    mileageKm: 45000,
    priceUsd: 22500,
    description: 'Well-maintained, single owner, full service history.',
    status: 'sold',
  },
  identityId,
  privateKeyWif,
  signingKeyIndex,
  nonce: await sdk.identities.contractNonce(identityId, contractId),
});

console.log('Listing marked as sold');
```

## Step 6: Leave a review

```typescript
await sdk.documents.create({
  contractId,
  documentType: 'review',
  document: {
    sellerId: 'SELLER_IDENTITY_ID',
    listingId: 'THE_LISTING_DOCUMENT_ID',
    rating: 5,
    comment: 'Great seller, car was exactly as described!',
  },
  identityId: buyerIdentityId,
  privateKeyWif: buyerKeyWif,
  signingKeyIndex: 0,
  nonce: await sdk.identities.contractNonce(buyerIdentityId, contractId),
});
```

### Query reviews for a seller

```typescript
const reviews = await sdk.documents.query({
  contractId,
  documentType: 'review',
  where: [['sellerId', '==', 'SELLER_IDENTITY_ID']],
  orderBy: [['rating', 'desc']],
  limit: 50,
});

let totalRating = 0;
let count = 0;
for (const [, doc] of reviews) {
  if (!doc) continue;
  totalRating += doc.getData().rating;
  count++;
}
console.log(`Average rating: ${(totalRating / count).toFixed(1)} (${count} reviews)`);
```

## Next steps

- Add **indexes** to the contract schema for efficient queries on `make`,
  `year`, and `priceUsd`
- Add a `location` field and query by region
- Use **document pricing** (`sdk.documents.setPrice` / `sdk.documents.purchase`)
  to let buyers pay for premium listing details
- Integrate with a frontend framework (React, Vue, etc.) for a full web app
