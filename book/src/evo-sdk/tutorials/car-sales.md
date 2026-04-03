# Tutorial: Car Sales Management

Build a decentralised car listing and sales application on Dash Platform. By
the end you will have a data contract for vehicle listings, the ability to
create/query/update listings, and a purchase flow using document transfers.

> **How this works in practice:** Data contracts are deployed once using a
> **Node.js script** with a developer identity. After deployment, your
> **browser app** uses the published contract ID to create, query, and update
> documents. Steps 1-2 below are run from Node.js; steps 3 onward can run
> in either Node.js or the browser.

## What you will learn

- Designing a data contract with multiple document types
- Publishing a contract to testnet from a Node.js deployment script
- Creating, querying, and updating documents (Node.js or browser)
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
      make:        { type: 'string', maxLength: 63, position: 0 },
      model:       { type: 'string', maxLength: 63, position: 1 },
      year:        { type: 'integer', minimum: 1900, maximum: 2100, position: 2 },
      mileageKm:   { type: 'integer', minimum: 0, position: 3 },
      priceUsd:    { type: 'integer', minimum: 0, position: 4 },
      description: { type: 'string', maxLength: 1024, position: 5 },
      imageUrl:    { type: 'string', maxLength: 512, format: 'uri', position: 6 },
      status:      { type: 'string', enum: ['available', 'pending', 'sold'], position: 7 },
    },
    required: ['make', 'model', 'year', 'priceUsd', 'status'],
    additionalProperties: false,
  },
  review: {
    type: 'object',
    properties: {
      sellerId:  { type: 'string', maxLength: 44, position: 0 },
      listingId: { type: 'string', maxLength: 44, position: 1 },
      rating:    { type: 'integer', minimum: 1, maximum: 5, position: 2 },
      comment:   { type: 'string', maxLength: 512, position: 3 },
    },
    required: ['sellerId', 'rating'],
    additionalProperties: false,
  },
};
```

## Step 2: Connect and publish the contract

```typescript
import { EvoSDK, DataContract, Document, Identifier, IdentitySigner } from '@dashevo/evo-sdk';

const sdk = EvoSDK.testnetTrusted();
await sdk.connect();

// Your identity credentials
const identityId = 'YOUR_IDENTITY_ID';
const privateKeyWif = 'YOUR_PRIVATE_KEY_WIF';
const signingKeyIndex = 0;

// Set up signing
const identity = await sdk.identities.fetch(identityId);
const identityKey = identity.publicKeys[signingKeyIndex];
const signer = new IdentitySigner();
signer.addKeyFromWif(privateKeyWif);

// Publish the data contract
const nonce = await sdk.identities.nonce(identityId);
const dataContract = new DataContract({
  ownerId: new Identifier(identityId),
  identityNonce: nonce + 1n,
  schemas: carSalesSchema,
});
const contract = await sdk.contracts.publish({ dataContract, identityKey, signer });

const contractId = contract.id.toString();
console.log('Contract published:', contractId);
```

Save the `contractId` — you will need it for all subsequent operations.

## Step 3: Create a listing

```typescript
const doc = new Document({
  documentTypeName: 'listing',
  dataContractId: new Identifier(contractId),
  ownerId: new Identifier(identityId),
  properties: {
    make: 'Toyota',
    model: 'Camry',
    year: 2021,
    mileageKm: 45000,
    priceUsd: 22500,
    description: 'Well-maintained, single owner, full service history.',
    status: 'available',
  },
});
await sdk.documents.create({ document: doc, identityKey, signer });

console.log('Listing created!');
```

## Step 4: Query listings

```typescript
// Fetch all available listings
const results = await sdk.documents.query({
  dataContractId: contractId,
  documentTypeName: 'listing',
  where: [['status', '==', 'available']],
  orderBy: [['priceUsd', 'asc']],
  limit: 20,
});

for (const [id, doc] of results) {
  if (!doc) continue;
  const data = doc.properties as Record<string, unknown>;
  console.log(`${data.year} ${data.make} ${data.model} — $${data.priceUsd}`);
  console.log(`  ID: ${id}`);
}
```

### Search by make

```typescript
const toyotas = await sdk.documents.query({
  dataContractId: contractId,
  documentTypeName: 'listing',
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

// Fetch the existing document, modify it, and bump the revision
const existing = await sdk.documents.get(contractId, 'listing', listingId);
existing.properties = { ...existing.properties, status: 'sold' };
existing.revision = (existing.revision ?? 0n) + 1n;
await sdk.documents.replace({ document: existing, identityKey, signer });

console.log('Listing marked as sold');
```

## Step 6: Leave a review

```typescript
// Set up buyer signing
const buyerIdentity = await sdk.identities.fetch(buyerIdentityId);
const buyerKey = buyerIdentity.publicKeys[0];
const buyerSigner = new IdentitySigner();
buyerSigner.addKeyFromWif(buyerKeyWif);

const reviewDoc = new Document({
  documentTypeName: 'review',
  dataContractId: new Identifier(contractId),
  ownerId: new Identifier(buyerIdentityId),
  properties: {
    sellerId: 'SELLER_IDENTITY_ID',
    listingId: 'THE_LISTING_DOCUMENT_ID',
    rating: 5,
    comment: 'Great seller, car was exactly as described!',
  },
});
await sdk.documents.create({ document: reviewDoc, identityKey: buyerKey, signer: buyerSigner });
```

### Query reviews for a seller

```typescript
const reviews = await sdk.documents.query({
  dataContractId: contractId,
  documentTypeName: 'review',
  where: [['sellerId', '==', 'SELLER_IDENTITY_ID']],
  orderBy: [['rating', 'desc']],
  limit: 50,
});

let totalRating = 0;
let count = 0;
for (const [, doc] of reviews) {
  if (!doc) continue;
  const props = doc.properties as Record<string, unknown>;
  totalRating += props.rating as number;
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
