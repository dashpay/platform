# Evo SDK

[![NPM Version](https://img.shields.io/npm/v/@dashevo/evo-sdk)](https://www.npmjs.com/package/@dashevo/evo-sdk)
[![Build Status](https://github.com/dashpay/platform/actions/workflows/release.yml/badge.svg)](https://github.com/dashpay/platform/actions/workflows/release.yml)
[![Release Date](https://img.shields.io/github/release-date/dashpay/platform)](https://github.com/dashpay/platform/releases/latest)
[![standard-readme compliant](https://img.shields.io/badge/readme%20style-standard-brightgreen)](https://github.com/RichardLitt/standard-readme)

TypeScript SDK for building applications on Dash Platform

Evo SDK provides a high-level, strongly-typed interface for interacting with [Dash Platform](https://dashplatform.readme.io/docs/introduction-what-is-dash-platform/) on [supported networks](https://github.com/dashpay/platform/#supported-networks). It wraps the WebAssembly-based [@dashevo/wasm-sdk](../wasm-sdk/) in ergonomic facades covering identities, documents, data contracts, tokens, DPNS, and more. The SDK works in both Node.js and modern browsers.

## Table of Contents

- [Install](#install)
- [Usage](#usage)
- [Facades](#facades)
- [Ranked queries](#ranked-queries)
- [Document references (`refersTo`)](#document-references-refersto)
- [Building a document create transition by hand](#building-a-document-create-transition-by-hand)
- [Immutable properties (`immutable`)](#immutable-properties-immutable)
- [Property constraints (`propertyConstraints`)](#property-constraints-propertyconstraints)
- [Chained queries (provable semi-join)](#chained-queries-provable-semi-join)
- [Composite queries (a page plus its sub-queries)](#composite-queries-a-page-plus-its-sub-queries)
- [Contributing](#contributing)
- [License](#license)

## Install

```sh
npm install @dashevo/evo-sdk
```

The package is ESM-only (`"type": "module"`). In CommonJS projects, use dynamic `import()`. Requires Node.js >= 18.18.

## Usage

Trusted mode is required for all queries. It pre-fetches quorum public keys so the SDK can verify Platform proofs.

```typescript
import { EvoSDK } from '@dashevo/evo-sdk';

const sdk = EvoSDK.testnetTrusted(); // or mainnetTrusted()
await sdk.connect();

const epoch = await sdk.epoch.current();
console.log('Current epoch:', epoch.index);
```

### Configuration

`EvoSDK` accepts the following options:

| Option | Type | Default | Notes |
|--------|------|---------|-------|
| `network` | `'testnet' \| 'mainnet' \| 'local' \| 'devnet'` | `'testnet'` | Target network. |
| `trusted` | `boolean` | `false` | When `true`, pre-fetches quorum keys for proof verification. Required for default query methods. |
| `addresses` | `string[]` | — | Seed masternode addresses. Required for non-trusted devnet; optional for other networks (replaces built-in defaults). |
| `devnetName` | `string` | — | Short name of the devnet (e.g. `'paloma'`). Required when `network: 'devnet'` and `trusted: true` (used to derive the quorum URL); ignored otherwise — only valid when `network === 'devnet'`. |
| `quorumUrl` | `string` | — | Override the trusted-context quorum base URL. Only meaningful when `trusted: true`. Useful for staging endpoints or devnets where the public DNS isn't deployed yet. |
| `proofs` | `boolean` | `true` | Setting to `false` disables proof requests where supported, but unproved mode is limited — several query paths (e.g. document fetches) force proofs regardless, and some query builders reject the unproved path. Mainly intended for mock/offline replay. |
| `version` | `number` | latest | Platform protocol version. |
| `logs` | `string` | — | Tracing/log filter for the underlying Wasm SDK. Accepts simple levels (`'info'`, `'debug'`, …) or a full `EnvFilter` string. |
| `settings` | `{ connectTimeoutMs?, timeoutMs?, retries?, banFailedAddress? }` | — | DAPI client transport settings. |

Preset factories are available as convenience: `EvoSDK.testnet()`, `EvoSDK.mainnet()`, `EvoSDK.testnetTrusted()`, `EvoSDK.mainnetTrusted()`, `EvoSDK.local()`, `EvoSDK.localTrusted()` (the last two target a dashmate local node), and the devnet factories `EvoSDK.devnet(name, options)` / `EvoSDK.devnetTrusted(name, options)`.

```typescript
// Trusted devnet — quorum URL auto-derived from the devnet name.
const sdk = EvoSDK.devnetTrusted('paloma');
await sdk.connect();

// Non-trusted devnet — explicit addresses required (no quorum context).
const local = EvoSDK.devnet('paloma', {
  addresses: ['https://10.0.0.5:1443'],
});
await local.connect();
```

Static helpers are also exported:

- `await EvoSDK.setLogLevel(filter)` — configure the underlying Wasm SDK's tracing globally.
- `await EvoSDK.getLatestVersionNumber()` — return the latest Platform protocol version supported by the bundled Wasm SDK.
- `sdk.version()` — the protocol version this SDK currently uses. Unpinned SDKs seed at a per-network floor (13 on mainnet, testnet and local; 14 on devnets) and ratchet upward from verified response metadata. On mainnet and testnet the Wasm SDK persists the learned version in `localStorage` under `dash-sdk.protocol-version.<network>` and seeds the next SDK with it, so the first proved request of a later page load already runs at the network's version. Passing `version` pins the SDK and disables both the ratchet and the persistence.
- Data contracts fetched through the SDK are cached in the trusted context and, on mainnet and testnet, persisted in `localStorage` under `dash-sdk.contracts.<network>`. The next SDK seeds its cache from that store, so the first proved document query of a later page load needs no contract round trip. A document stamped with a newer `$contractVersion` than the cached contract drops the entry and refetches the contract; `removeCachedContract` clears both the cache and the stored copy.
- `await EvoSDK.maxRankedLimit()` — the hard ceiling on a [ranked / having-range](#ranked-queries) `limit`.
- `await EvoSDK.rankedAverageScale()` — the fixed-point divisor for the `avg` axis of a ranked / having-range result.
- `await EvoSDK.maxPrefixInBranches()` — the hard ceiling on the element count of a branching `in` [prefix pin](#ranked-queries).

## Facades

The SDK organises its API into domain-specific facades, each accessible as a property on the `EvoSDK` instance:

| Facade | Description |
|--------|-------------|
| [`sdk.addresses`](src/addresses/facade.ts) | Query balances, transfer credits, withdraw to L1 |
| [`sdk.identities`](src/identities/facade.ts) | Fetch, create, update, and top up identities |
| [`sdk.documents`](src/documents/facade.ts) | Query, create, replace, delete, and transfer documents; aggregate `count` / `sum` / `average` over indexed fields; `ranked` top-K and `having` range queries over ranked indexes |
| [`sdk.contracts`](src/contracts/facade.ts) | Fetch, publish, and update data contracts |
| [`sdk.tokens`](src/tokens/facade.ts) | Mint, burn, transfer, freeze tokens and query balances |
| [`sdk.dpns`](src/dpns/facade.ts) | Register and resolve Dash Platform names |
| [`sdk.epoch`](src/epoch/facade.ts) | Query epoch information and evonode proposed blocks |
| [`sdk.protocol`](src/protocol/facade.ts) | Protocol version upgrade state and voting |
| [`sdk.stateTransitions`](src/state-transitions/facade.ts) | Broadcast and wait for state transitions |
| [`sdk.system`](src/system/facade.ts) | System status, quorum info, and total credits |
| [`sdk.group`](src/group/facade.ts) | Group membership, actions, and contested resources |
| [`sdk.voting`](src/voting/facade.ts) | Contested resource vote states and polls |
| [`sdk.shielded`](src/shielded/facade.ts) | Query shielded pool state, encrypted notes, anchors, and nullifier status |
| [`sdk.encryptedFor`](src/encrypted-for/facade.ts) | Encrypt and decrypt the byte properties a document type declares `encryptedFor`, for any contract |
| [`sdk.moderationCharters`](src/moderation-charters/facade.ts) | Read a contract's seated charter, its team, proposals and join requests; build join and resignation requests |

A `wallet` namespace is also exported with utilities for BIP39 mnemonic generation and validation, BIP44/DIP9/DIP13 key derivation (path helpers included), extended-key conversion (`xprvToXpub`, `deriveChildPublicKey`), key-pair generation and import (`generateKeyPair`, `keyPairFromWif`, `keyPairFromHex`), public-key-to-address conversion, address validation, message signing, and Dashpay contact-key derivation. See [`src/wallet/functions.ts`](src/wallet/functions.ts) for the full list.

## Ranked queries

From protocol version 14, a contract index can declare `rankedCountable`, `rankedSummable` or `rankedAverageable`. Against such an index the SDK can answer "which groups score highest?" with a proof, in `O(log n + k)`, without walking every group:

```ts
// The three best restaurants by average grade.
const page = await sdk.documents.ranked({
  dataContractId: RESTAURANTS,
  documentTypeName: 'review',
  groupBy: 'restaurantId',
  aggregate: { type: 'avg', property: 'grade' },
  limit: 3,
});

for (const entry of page.entries) {
  // `value` is exact fixed point for the avg axis — divide by `page.valueScale`,
  // never by a hardcoded constant. `valueAsNumber` is a lossy display helper.
  console.log(entry.rank, entry.groupValue, Number(entry.value) / Number(page.valueScale));
}
```

`limit` is required and capped at `await EvoSDK.maxRankedLimit()` (a hard reject, not a clamp). `offset` skips ranks — `{ limit: 1, offset: 4 }` is "the 5th best" — and has no ceiling, because the skipped region is attested rather than walked.

### Pinning a compound index

A compound ranked index keeps one ordered secondary per prefix value, with no ordering across prefixes, so a ranked read has to name the prefixes it descends into. `where` pins each leading index property:

```ts
// The best-rated restaurants in either of two cities.
const page = await sdk.documents.ranked({
  dataContractId: RESTAURANTS,
  documentTypeName: 'review',
  groupBy: 'restaurantId',
  aggregate: { type: 'avg', property: 'grade' },
  limit: 3,
  where: [['city', 'in', ['Berlin', 'Hamburg']]],
});

for (const entry of page.entries) {
  // Only set on a merged page: the same `groupKeyHex` can appear under
  // two pinned prefixes, and this says which branch the entry came from.
  console.log(entry.branchKeyHex, entry.groupValue);
}
```

Each pin is a `==`, except that at most **one** may be a branching `in` carrying 2..=`await EvoSDK.maxPrefixInBranches()` elements — one secondary walk per element, merged into a single proved page. Several `in`s would multiply into a cartesian product of walks inside one proof, so they are rejected. A single-element `in` normalizes to `==` and never spends that budget. Range operators cannot pin a prefix at all.

A branching `in` cannot combine with a non-zero `offset`: rank-skip is attested from one secondary's counted commitments, and there is no counted structure over a branch union. Page one prefix at a time (`==` plus `offset`), or drop the offset.

`sdk.documents.having()` bounds the same axis by value instead of by position (`{ operator: '>', value: 100 }`), and `rankedWithProof` / `havingWithProof` return the proof and block metadata alongside the result.

## Contracts the app already holds

An app knows its contracts at build time. Instead of fetching them on every load, bundle a snapshot (`contract.toBase64(platformVersion)`), seed the SDK with it, and confirm off the critical path that the snapshot is still current:

```ts
import { DataContract, PlatformVersion } from '@dashevo/evo-sdk';

// Seed: no round trip before the first document query, and the seeded
// contracts are persisted like fetched ones.
for (const { bytes } of bundledContracts) {
  await sdk.contracts.addKnown(DataContract.fromBase64(bytes, true, PlatformVersion.latest()));
}

// Revalidate after first paint, proved. Versions only: the contracts come
// back only for the ids that changed.
const latest = await sdk.contracts.getLatestVersions({ contractIds: bundledContracts.map((c) => c.id) });
const stale = bundledContracts.filter((c) => latest.get(c.id)?.version !== c.version).map((c) => c.id);
if (stale.length > 0) {
  await sdk.contracts.getMany(stale); // replaces the seeded entries in the cache
}
```

A seeded contract that the network has since updated is also caught without the check: the SDK drops a cached contract on the first document stamped with a newer `$contractVersion` (see the contract cache note above), and the next query fetches the current one.

## Document references (`refersTo`)

Also from protocol version 14, an identifier property can declare what it points at. This is a write-time consensus constraint — nothing resolves a reference for a reader — but a fetched contract can be asked what it declares:

```ts
const contract = await sdk.contracts.fetch(contractId);

for (const ref of contract.documentTypeReferences('note')) {
  // { path: 'author', type: 'identityPublicKey', keyIdProperty: 'authorKeyId' }
  // A permanentDocument reference may additionally carry a write-time
  // equality binding between the two documents' properties:
  // { path: 'postId', type: 'permanentDocument', contractId, documentType: 'post',
  //   propertyAgreement: { hashtag: 'hashtag' } }
  // The referenced side may also name the referenced document's `$ownerId`
  // or `$creatorId`, e.g. `propertyAgreement: { authorId: '$ownerId' }`, and
  // the referring side may be the writer's own `$ownerId`: a write gate such
  // as `{ '$ownerId': '$ownerId' }` lets only the referenced document's
  // current owner create or replace the referring document.
  console.log(ref.path, ref.type);
}

// Every document type that declares at least one reference.
contract.documentReferences;
```

A typed array of identifiers may declare `refersTo` on its `items`, which every element then carries. Such a declaration is listed at the list path of its elements, `path: 'reasons[]'`, which is not a property path: read the list at `reasons` and treat each element as a reference. The same declaration is on the typed array's item, `contract.documentTypeTypedArrays('charter')[0].items.refersTo`. Consensus checks every element when the document is written, and a rejection names the failing element by its index, as in `reasons[2]` for the third.

A document type may also declare `ownerRefersTo`, a reference whose value is the document's owner, the writer, instead of a property's value. It is listed first, at `path: '$ownerId'`, which is not a property path: the value it constrains is the document's `ownerId`. Its `type` is `identity` or a `permanentDocument` with a `lookup`, in which `'.'` is the writer:

```ts
// { path: '$ownerId', type: 'permanentDocument', contractId, documentType: 'addedModerator',
//   lookup: { index: 'byElectedCharterMember', keys: { electedCharterId: 'electedCharterId', memberId: '.' } } }
```

reads: the writer must be the `memberId` of an `addedModerator` for the document's `electedCharterId`. Consensus checks it when a document is created and when a replace changes a property the lookup or a `propertyAgreement` reads, and a rejection names it `$ownerId`. Only a document type whose documents can be neither transferred nor traded may declare it, so the owner is always the writer that was checked. A transferable or tradeable type declares `creatorRefersTo` instead, listed first at `path: '$creatorId'`: the same declaration, whose value is the document's creator, which never changes, so a transfer or a purchase leaves it true.

A document reference comes in two strengths. `permanentDocument` requires the referenced document type to declare `canBeDeleted: false`, so a reference that was accepted keeps resolving. `deletableDocument` takes the same declaration (`contractId`, `documentType`, `propertyAgreement`) and is its disjoint counterpart: the referenced type must allow deletion (`ReferencedDocumentTypeNotDeletable`, 40131, otherwise). The referenced document must exist, and the agreement must hold, when the referring document is written, but it may be deleted afterwards. Nothing blocks that deletion and nothing cleans up after it, so a reader must expect such a reference to resolve to nothing. It can never start resolving to different content: a document id commits to the nonce of its create transition, so a deleted id can not be created again. A writer may not leave it that way: every replace of the referring document re-validates the reference, touched or not, so once the target is gone the replace has to repoint it at a document that exists or clear it (`ReferencedEntityNotFound` otherwise). A writer gate is then checked against the new target, never against a missing one. On an `immutable` property clearing is the only move, and the immutable check lets that one change through. The referring document can always be deleted. A property cannot switch between the two on a contract update, and `preallocated` indexes are only available through `permanentDocument`.

Declarations are only parsed from protocol version 14 onward; a contract deserialized against an earlier version reports none even when its raw schema carries the keyword.

When a write is rejected because a reference does not resolve, the consensus code reaches JS as `error.code`:

```ts
import { DocumentReferenceErrorCode } from '@dashevo/evo-sdk';

try {
  await sdk.documents.create({ document, identityKey, signer });
} catch (e) {
  if (e.code === DocumentReferenceErrorCode.ReferencedIdentityKeyDisabled) {
    // the referenced key exists but was disabled
  }
}
```

## Building a document create transition by hand

`sdk.documents.create` fetches the nonce, builds, signs, broadcasts and returns the confirmed document, whose `id` is the one Platform stored. An app that needs the signed transition itself (to broadcast later, or to cache the signed bytes) builds it from the re-exported `wasm-dpp2` classes:

```ts
import { Document, DocumentCreateTransition, BatchTransition } from '@dashevo/evo-sdk';

const document = new Document({ properties, documentTypeName, dataContractId, ownerId });
const transition = new DocumentCreateTransition({ document, identityContractNonce: nonce });
const batch = BatchTransition.fromBatchedTransitions([transition.toDocumentTransition()], ownerId, 0); // userFeeIncrease
const stateTransition = batch.toStateTransition();
// sign, then sdk.stateTransitions.broadcast(stateTransition)
```

From protocol version 14 the id of a new document commits to the identity contract nonce of its create transition. `new DocumentCreateTransition(...)` derives that id from the document's entropy and `identityContractNonce`, puts it on the transition and writes it back onto `document`, so `document.id` is final once the transition exists and equals `transition.base.id`. Before that the `Document` carries a placeholder. To know the id earlier, `document.setIdForCreation(nonce)` or `Document.generateId(type, owner, contract, entropy, nonce)`, or pass `identityContractNonce` to the `Document` constructor. Pass `platformVersion` (defaults to latest) to any of them for a network on an earlier protocol version. No app needs to reimplement the hash.

## Encrypted properties (`encryptedFor`)

From protocol version 14 a byte array property can declare how its ciphertext was produced, so a wallet reads the recipe from the contract instead of a side channel: the recipient (an identifier property of the same document type, or `$ownerId` for a message the writer encrypts to themself), the integer properties carrying the recipient's and the sender's key ids, and the scheme. The one scheme today, `ecdh-secp256k1-aes256-cbc`, is the dashpay contact request's: a random 16-byte IV followed by AES-256-CBC with PKCS7 padding under the libsecp256k1 ECDH shared key of the two identities' keys. A fetched contract can be asked what it declares:

```ts
const contract = await sdk.contracts.fetch(contractId);

contract.documentTypeEncryptedProperties('joinRequest');
// [{
//   path: 'encryptedMessage',
//   recipient: 'recipientId',
//   recipientKey: 'recipientKeyId',
//   senderKey: 'senderKeyId',
//   scheme: 'ecdh-secp256k1-aes256-cbc',
// }]

// Every document type that declares at least one encrypted property.
contract.documentEncryptedProperties;
```

The keyword is only parsed from protocol version 14 onward; a contract deserialized against an earlier version reports none even when its raw schema carries it. Consensus checks only the shape of the bytes on every create and replace (at least 32 bytes and a multiple of 16 for AES-CBC) and nothing about who can decrypt them. A value of the wrong shape is rejected, and the code reaches JS as `error.code`:

```ts
import { DocumentEncryptionErrorCode } from '@dashevo/evo-sdk';

try {
  await sdk.documents.create({ document, identityKey, signer });
} catch (e) {
  if (e.code === DocumentEncryptionErrorCode.InvalidEncryptedPropertyShape) {
    // the bytes are not a ciphertext of the declared scheme (code 10420)
  }
}
```

`sdk.encryptedFor` encrypts and decrypts such a property, reading the declaration from the contract, so the same calls work for every contract that declares one. They run locally and need no connection.

```ts
import { PrivateKey } from '@dashevo/evo-sdk';

// The writer: the fields to set on the document, the ciphertext and both key ids
const fields = await sdk.encryptedFor.encrypt({
  dataContract: contract,
  documentTypeName: 'joinRequest',
  property: 'encryptedMessage',
  plaintext: 'I would like to help moderate',
  senderKey: writerIdentity.getPublicKeyById(4),       // its id goes into senderKeyId
  senderPrivateKey: PrivateKey.fromWIF(writerKeyWif),
  recipientKey: leaderIdentity.getPublicKeyById(2),    // its id goes into recipientKeyId
});
// { encryptedMessage: Uint8Array(48), recipientKeyId: 2, senderKeyId: 4 }

// The reader: whose keys the stored document names, then decrypt
const envelope = await sdk.encryptedFor.envelope({ dataContract: contract, document, property: 'encryptedMessage' });
const sender = await sdk.identities.fetch(envelope.senderId);
const message = await sdk.encryptedFor.decrypt({
  dataContract: contract,
  document,
  property: 'encryptedMessage',
  recipientPrivateKey: PrivateKey.fromWIF(leaderDecryptionKeyWif), // the key recipientKeyId names
  senderKey: sender.getPublicKeyById(envelope.senderKeyId),
});
```

The IV is fresh randomness on every call. The scheme carries no authentication tag: a wrong key is caught only by the padding check, which it passes about once in 256 attempts and then returns garbage, so an app that must tell the two apart has to recognise its plaintext. ECDH is symmetric, so the writer can read its own message back with its private key and the recipient's key.

## Moderation charters

A contract that declares elected moderation is moderated by the team of its seated charter in the moderation charters system contract (protocol version 14, `EG7RGfV8fDTayC2FyVr8HwdpJh3fXDbVztcfE94UmN88`). `sdk.moderationCharters` reads it with ordinary proved document queries:

```ts
// The seated charter: the one electedCharter for the contract, or undefined
const charter = await sdk.moderationCharters.seatedCharter(contractId);

// Its proposal, the submittedCharter it runs on
const proposal = await sdk.moderationCharters.submittedCharter(charter.properties.submittedCharterId);

// The team: the leader plus the elected members and the additions, less the removals
const team = await sdk.moderationCharters.team(contractId);
team.leaderId; team.members; team.contains(identityId);

// Proposals for a contract in filing order, and the join requests for one, a page at a time
const proposals = await sdk.moderationCharters.submittedCharters({ targetContractId: contractId, limit: 20 });
const requests = await sdk.moderationCharters.joinRequests({ submittedCharterId: proposalId });

// Resignation requests whose writer is still on the team (the leader has not acted on them)
const pending = await sdk.moderationCharters.pendingResignationRequests(charter.id);
```

A join request and a resignation request carry a message only the leader can read. The builders fetch the proposal (or the charter) and the leader, pick the leader's decryption key bound to `submittedCharter` and the writer's encryption key bound to `joinRequest`, the keys the schema's `keyRequirements` demand, encrypt the message and set `recipientId`, `recipientKeyId` and `senderKeyId`:

```ts
const joinRequest = await sdk.moderationCharters.buildJoinRequest({
  submittedCharterId: proposalId,
  message: 'Five years moderating a forum; happy to help',
  writer: identity,                                    // or its id
  writerEncryptionKey: PrivateKey.fromWIF(encryptionKeyWif),
});
await sdk.documents.create({ document: joinRequest, identityKey, signer });

const resignation = await sdk.moderationCharters.buildResignationRequest({
  electedCharterId: charter.id,
  message: 'Stepping down at the end of the month',
  writer: identity,
  writerEncryptionKey: PrivateKey.fromWIF(encryptionKeyWif),
});
await sdk.documents.create({ document: resignation, identityKey, signer });
```

The leader reads either with `sdk.encryptedFor.decrypt`.

## Immutable properties (`immutable`)

From protocol version 14 a mutable document type can freeze some of its top-level properties at creation with the doctype-level `immutable` list, while the rest of the document stays replaceable. A second list, `immutableAllowSetting`, names the frozen properties a replace may still set while the stored document has no value for them; once present they are frozen too. Both are consensus-enforced on every replace, and a fetched contract can be asked what it declares:

```ts
const contract = await sdk.contracts.fetch(contractId);

contract.documentTypeImmutableProperties('post');
// { immutable: ['author', 'mood'], immutableAllowSetting: ['mood'] }
// Both arrays hold top-level property names, sorted. Listing an object
// property freezes it whole, nested values included.

// Every document type that freezes at least one property.
contract.documentImmutableProperties;
```

The lists are only parsed from protocol version 14 onward; a contract deserialized against an earlier version reports empty lists even when its raw schema carries the keywords.

A replace that changes, adds or removes a frozen property is rejected, and the consensus code reaches JS as `error.code`:

```ts
import { DocumentImmutabilityErrorCode } from '@dashevo/evo-sdk';

try {
  await sdk.documents.replace({ document, identityKey, signer });
} catch (e) {
  if (e.code === DocumentImmutabilityErrorCode.DocumentImmutablePropertyChanged) {
    // the replace touched a property the document type freezes (code 40128)
  }
}
```

## Property constraints (`propertyConstraints`)

From protocol version 14 a document type can declare rules its documents' integer properties must meet, each a comparison of two integer expressions built from property paths and integer values:

```json
"propertyConstraints": {
  "depositCoversOrder": {
    "lessThanOrEqual": [
      { "multiply": [{ "add": ["price", "fee"] }, "quantity"] },
      "deposit"
    ]
  },
  "minimumOrder": {
    "greaterThanOrEqual": [{ "multiply": ["price", { "ifAbsent": ["quantity", 1] }] }, 100]
  }
}
```

The comparisons are `equal`, `notEqual`, `lessThan`, `lessThanOrEqual`, `greaterThan` and `greaterThanOrEqual`, and the operators `add` and `multiply` (two or more operands) and `subtract`, `divide`, `modulo` and `power` (exactly two). A property the document leaves out counts as 0, or as the value of an `ifAbsent` operand naming it. The arithmetic is exact over 128-bit integers, and `divide` and `modulo` are Euclidean, so a remainder is never negative. The rules are fixed when the document type is created.

Consensus checks every rule on each create and replace, and rejects a document that breaks one, or whose rule overflows, divides by zero or raises to a negative power. The code reaches JS as `error.code`, and the message names the rule:

```ts
import { DocumentPropertyConstraintErrorCode } from '@dashevo/evo-sdk';

try {
  await sdk.documents.create({ document, identityKey, signer });
} catch (e) {
  if (e.code === DocumentPropertyConstraintErrorCode.DocumentPropertyConstraintViolated) {
    // the document breaks one of its type's rules (code 10422)
  }
}
```

## Chained queries (provable semi-join)

A `refersTo: permanentDocument` declaration also lights up the read side: a **chained query** answers `SELECT * FROM post WHERE $id IN (SELECT postId FROM like WHERE $ownerId = me)` in one verified round trip. The node returns the inner indexOnly page and the referenced documents under ONE merged proof — a single quorum-signed state root by construction — and the SDK re-derives the outer query itself and checks it against the *proven* inner values — the node cannot substitute, omit, or inject joined documents. For a `permanentDocument` join property a missing referenced document fails verification outright, since such a reference cannot dangle. For a `deletableDocument` join property a referenced document that was deleted since is proven absent: it has no entry in `outerDocuments` (so match the two halves by id, not by position) and its id is listed in `missingOuterIds`, in first-appearance order. The node still cannot pass an existing document off as deleted.

```ts
// The posts I liked, newest page first by postId.
const page = await sdk.documents.chained({
  dataContractId: YAPPR,
  innerDocumentType: 'like',
  where: [['$ownerId', '==', me]],
  innerLimit: 25,
  joinProperty: 'postId',
  outerDocumentType: 'post',
});

for (const post of page.outerDocuments) {
  console.log(post.properties.message);
}

// Next page: continue past the last proven join value.
const cursor = page.innerDocuments.at(-1)?.properties.postId;
const next = await sdk.documents.chained({
  dataContractId: YAPPR,
  innerDocumentType: 'like',
  where: [['$ownerId', '==', me], ['postId', '>', cursor]],
  orderBy: [['postId', 'asc']],
  innerLimit: 25,
  joinProperty: 'postId',
  outerDocumentType: 'post',
});
```

The inner query must target an indexOnly document type and resolve to an index carrying `joinProperty`, and `joinProperty` must declare a same-contract `refersTo: permanentDocument` or `refersTo: deletableDocument` targeting `outerDocumentType`. `innerLimit` is required — it bounds the derived outer fetch, so there is no server-default fallback. There are no outer-side clauses by design; filter `outerDocuments` locally. `sdk.documents.chainedWithProof(...)` returns the same result with the metadata and proof envelope attached.

## Composite queries (a page plus its sub-queries)

A **composite query** answers a page and everything a UI needs to render it in ONE verified round trip: the page documents, plus one to ten sub-queries whose `IN` clause the node derives from the proven page (or from an earlier `documents` sub-query). The request never names the derived values. Four sub-query shapes exist:

- a **by-id join** (`bind.field: '$id'`): the documents a page property refers to (the property must declare `refersTo: permanentDocument` or `refersTo: deletableDocument` targeting the sub-query's type; a missing document fails verification for the former; for the latter it is proven absent, left out of `documents` and listed in that sub-result's `missingIds`);
- an **indexed lookup** (`bind.field` an indexed property or `$ownerId`): documents keyed by a page value, in this or any other contract, with a `limit` on the rows it returns in total unless the index already bounds them (a unique index, or an indexOnly terminal with every prefix fixed);
- a **count** (`kind: 'counts'`): one count per page value from a `countable` index covering the fixed clauses plus the bound field;
- a **sibling** (no `bind`): an independent documents query proven under the same root.

The node returns everything under ONE merged proof, a single quorum-signed state root by construction, and the SDK bootstraps the page from the proof, re-derives every sub-query itself and verifies the whole composition: the node cannot substitute, omit, or inject a sub-result.

```ts
// A feed page: the dash posts, their like counts, the posts they quote,
// their authors' profiles, and which of them I liked.
const page = await sdk.documents.composite({
  dataContractId: YAPPR,
  documentType: 'post',
  where: [['hashtag', '==', 'dash']],
  orderBy: [['$createdAt', 'desc']],
  limit: 20,
  subQueries: [
    { documentType: 'like', kind: 'counts', where: [['hashtag', '==', 'dash']], bind: { sourceProperty: '$id', field: 'postId' } },
    { documentType: 'post', bind: { sourceProperty: 'quotedPostId', field: '$id' } },
    { dataContractId: DASHPAY, documentType: 'profile', bind: { sourceProperty: '$ownerId', field: '$ownerId' } },
    { documentType: 'like', where: [['$ownerId', '==', me]], bind: { sourceProperty: '$id', field: 'postId' } },
  ],
});

const [likeCounts, quotedPosts, profiles, myLikes] = page.subResults;
for (const post of page.pageDocuments) {
  const likes = likeCounts.kind === 'counts' ? likeCounts.counts.get(post.id.toBase58()) ?? 0n : 0n;
  console.log(post.properties.message, likes);
}

// Next page: continue past the last proven page document.
const cursor = page.pageDocuments.at(-1)?.createdAt;
const next = await sdk.documents.composite({
  dataContractId: YAPPR,
  documentType: 'post',
  where: [['hashtag', '==', 'dash'], ['$createdAt', '<', cursor]],
  orderBy: [['$createdAt', 'desc']],
  limit: 20,
  subQueries: [/* the same */],
});
```

`limit` on the page is required (1–100) and bounds every derived clause (at most 100 values reach a sub-query). A sub-query may bind the page (`bind.source: 'page'`, the default) or an earlier `documents` sub-query by index (`bind.source: 1`), so quoted posts can in turn pull their authors' profiles. Every sub-query walks in the page's direction: leave a lookup's ordering out and it inherits that direction, while an ordering that disagrees with the page is refused. Sub-results come back in request order as `{ kind: 'documents', documents }` (a join in first-appearance order of the page's ids, a lookup or sibling in query order) or `{ kind: 'counts', counts }` (a `Map` keyed by the bound value's base58 identifier; a value with no entry counts zero). There is no cursor on this surface; paginate with a range clause on the page's ordering property. `sdk.documents.compositeWithProof(...)` returns the same result with the metadata and proof envelope attached.

## Contributing

Feel free to dive in! [Open an issue](https://github.com/dashpay/platform/issues/new/choose) or submit PRs.

## License

[MIT](LICENSE) &copy; Dash Core Group, Inc.
