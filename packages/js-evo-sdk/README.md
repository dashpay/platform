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
- [How a document type is stored (`documentTypeLayout`)](#how-a-document-type-is-stored-documenttypelayout)
- [What a document costs (`documentCreateCost`)](#what-a-document-costs-documentcreatecost)
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
  // A permanentDocument reference may additionally carry `where`, a
  // write-time equality the referenced document must meet, keyed by the
  // referenced document's property and valued by the referring one:
  // { path: 'postId', type: 'permanentDocument', contractId, documentType: 'post',
  //   where: { hashtag: 'hashtag' } }
  // The key may also name the referenced document's `$ownerId`, `$creatorId`
  // or `$id`, e.g. `where: { '$ownerId': 'authorId' }`, and the value may be
  // the writer's own `$ownerId`: a write gate such as
  // `{ '$ownerId': '$ownerId' }` lets only the referenced document's current
  // owner create or replace the referring document.
  console.log(ref.path, ref.type);
}

// Every document type that declares at least one reference.
contract.documentReferences;
```

A typed array of identifiers may declare `refersTo` on its `items`, which every element then carries. Such a declaration is listed at the list path of its elements, `path: 'reasons[]'`, which is not a property path: read the list at `reasons` and treat each element as a reference. The same declaration is on the typed array's item, `contract.documentTypeTypedArrays('charter')[0].items.refersTo`. Consensus checks every element when the document is written, and a rejection names the failing element by its index, as in `reasons[2]` for the third.

A document type may also declare `ownerRefersTo`, a reference whose value is the document's owner, the writer, instead of a property's value. It is listed first, at `path: '$ownerId'`, which is not a property path: the value it constrains is the document's `ownerId`. Its `type` is `identity`, a `permanentDocument` or `deletableDocument` with `findBy` (the properties of the referenced type's unique index that finds the document, each mapped to where its value comes from), or a `permanentDocument` with `inList`. In `findBy`, `'.'` is the writer:

```ts
// { path: '$ownerId', type: 'deletableDocument', contractId, documentType: 'addedModerator',
//   findBy: { electedCharterId: 'electedCharterId', memberId: '.' } }
```

reads: the writer must be the `memberId` of an `addedModerator` for the document's `electedCharterId`, one that exists now. Consensus checks it when a document is created and when a replace changes a property its `findBy` or `where` reads (for a `deletableDocument` like this one, on every replace), and a rejection names it `$ownerId`. Only a document type whose documents can be neither transferred nor traded may declare it, so the owner is always the writer that was checked. A transferable or tradeable type declares `creatorRefersTo` instead, listed first at `path: '$creatorId'`: the same declaration, whose value is the document's creator, which never changes, so a transfer or a purchase leaves it true.

A document reference comes in two strengths. `permanentDocument` requires the referenced document type to declare `canBeDeleted: false`, so a reference that was accepted keeps resolving. `deletableDocument` takes the same declaration (`contractId`, `documentType`, `findBy`, `where`) and is its disjoint counterpart: the referenced type must allow deletion (`ReferencedDocumentTypeNotDeletable`, 40131, otherwise). The referenced document must exist, and its `where` must hold, when the referring document is written, but it may be deleted afterwards. Nothing blocks that deletion and nothing cleans up after it, so a reader must expect such a reference to resolve to nothing. A writer may not leave it that way: every replace of the referring document re-validates the reference, touched or not, so once the target is gone the replace has to repoint it at a document that exists or clear it (`ReferencedEntityNotFound` otherwise). A writer gate is then checked against the new target, never against a missing one. On an `immutable` property clearing is the only move, and the immutable check lets that one change through. The referring document can always be deleted. By id it can never start resolving to different content: a document id commits to the nonce of its create transition, so a deleted id can not be created again. Found by `findBy`, it means a document with that key exists now: once the one it found is deleted, a later document with the same key may be found instead. A property cannot switch between the two on a contract update, and `preallocated` indexes are only available through `permanentDocument`.

A third strength sits between them: `moderatedDocument`, for a document type whose documents leave state only when the contract's moderators remove them, each removal leaving a removal record (`canBeDeleted: false`, no `ttl`, and `moderatorAbilities.delete` keeping records, the default). The three are disjoint: such a type refuses a `deletableDocument` reference (`ReferencedDocumentTypeModerated`, 40144), and a `moderatedDocument` reference to any other type is refused (`ReferencedDocumentTypeNotModerated`, 40143). It takes `contractId`, `documentType` and `where`, by id only. The document must exist when the referring document is written. Once a moderator removes it, the reference is kept and resolves to the removal record: a replace re-validates it only as it would a `permanentDocument` reference, and a `where` entry it asks about again is checked against the record's owner and id, any other property refusing the replace (`ReferencedDocumentRemoved`, 40145). A new value, on a create or a repoint, must name a document in state.

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
// undefined unless the document enters a contest (a DPNS name, a moderation charter)
const prefundedVotingBalance = await sdk.documents.contestFundToJoin(document);
const transition = new DocumentCreateTransition({
  document,
  identityContractNonce: nonce,
  prefundedVotingBalance,
});
const batch = BatchTransition.fromBatchedTransitions([transition.toDocumentTransition()], ownerId, 0); // userFeeIncrease
const stateTransition = batch.toStateTransition();
// sign, then sdk.stateTransitions.broadcast(stateTransition)
```

From protocol version 14 the id of a new document commits to the identity contract nonce of its create transition. `new DocumentCreateTransition(...)` derives that id from the document's entropy and `identityContractNonce`, puts it on the transition and writes it back onto `document`, so `document.id` is final once the transition exists and equals `transition.base.id`. Before that the `Document` carries a placeholder. To know the id earlier, `document.setIdForCreation(nonce)` or `Document.generateId(type, owner, contract, entropy, nonce)`, or pass `identityContractNonce` to the `Document` constructor. Pass `platformVersion` (defaults to latest) to any of them for a network on an earlier protocol version. No app needs to reimplement the hash.

A document whose values fall under a contested index enters a contest, and its create must state the most it pays into the contest's fund: without it, or stating less than the fund to join, Platform refuses the create with error 40114 and still charges its fees. From protocol version 14 that fund doubles once the contest holds 250 contenders and again for every 50 more. `sdk.documents.create` states it itself. For a transition built by hand, `sdk.documents.contestFundToJoin(document)` reads the contest's contenders (one proved query per 100) and returns the `PrefundedVotingBalance` to pass, the contested index's name and the fund to join now, or `undefined` for a document that joins no contest. Without the SDK, pass the document's contract as `dataContract` to `new DocumentCreateTransition(...)`: a contested document then states the contest's fund on its contested index, what joining costs below 250 contenders, and `contestFund` replaces that amount.

From protocol version 14 a create may state more, as headroom for contenders joining before it lands: `new PrefundedVotingBalance({ indexName: prefundedVotingBalance.indexName, credits: 2n * prefundedVotingBalance.credits })`. Platform charges only the fund to join, but the identity must hold what the create states. Before 14 the stated amount must be exactly the contest's fund, and a create stating more is refused.

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

From protocol version 14 a mutable document type can freeze some of its top-level properties with the doctype-level `immutable` list, while the rest of the document stays replaceable. An entry naming a property freezes it at creation. An entry `{ property, when }` freezes it for any replace its condition holds for: the condition takes the grammar of a `propertyConstraints` rule, is judged on the document the replace writes (whose `$updatedAt` is the replace's block time), and reads the stored document through `$old.` paths.

```json
"immutable": [
  "author",
  { "property": "text", "when": { "greaterThan": [{ "subtract": ["$updatedAt", "$createdAt"] }, 300000] } },
  { "property": "mood", "when": { "present": "$old.mood" } }
]
```

Here `author` never changes, `text` can be edited for five minutes after the document is created, and `mood` can be set once. Both kinds are consensus-enforced on every replace, and a fetched contract can be asked what it declares:

```ts
const contract = await sdk.contracts.fetch(contractId);

contract.documentTypeImmutableProperties('post');
// { immutable: ['author'], immutableWhen: { mood: { present: '$old.mood' }, text: { ... } } }
// Both hold top-level property names, sorted, and each condition as the
// contract declares it. Listing an object property freezes it whole,
// nested values included.

// Every document type that freezes at least one property.
contract.documentImmutableProperties;
```

The keyword is only parsed from protocol version 14 onward; a contract deserialized against an earlier version reports nothing frozen even when its raw schema carries it.

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

From protocol version 14 a document type can declare rules its documents' properties must meet, each a comparison of two integer expressions built from property paths and integer values, an `in` list of values, a comparison of a string property with string constants, a `present` or `absent` test, or `anyOf`, `allOf` or `not` over such conditions:

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
  },
  "feeWaivedOrAtLeastTen": {
    "anyOf": [{ "equal": ["fee", 0] }, { "greaterThanOrEqual": ["fee", 10] }]
  },
  "discountGivenAboveZero": {
    "anyOf": [{ "absent": "discount" }, { "greaterThan": ["discount", 0] }]
  },
  "tieredFee": { "in": ["fee", [0, 10, 25, 50]] },
  "closedNeedsClosedAt": {
    "anyOf": [{ "notEqual": ["status", { "const": "closed" }] }, { "present": "closedAt" }]
  }
}
```

The comparisons are `equal`, `notEqual`, `lessThan`, `lessThanOrEqual`, `greaterThan` and `greaterThanOrEqual`, and the operators `add` and `multiply` (two or more operands) and `subtract`, `divide`, `modulo` and `power` (exactly two). `{ "in": [expression, [values]] }` holds if the expression takes one of two or more distinct integer values. A string property (an enum, say) is compared with `{ "equal": ["status", { "const": "closed" }] }` or `notEqual`, with another string property (`{ "notEqual": ["fromCurrency", "toCurrency"] }`), or listed with `{ "in": ["status", ["open", "pending"]] }`; a constant must be one of the property's `enum` values, and a string the document leaves out equals none, unless `{ "ifAbsent": ["status", "open"] }` gives it a default. Identifier properties compare the same way, with base58 constants: `{ "equal": ["paymentToken", { "const": "<base58>" }] }`, `{ "notEqual": ["buyerId", "sellerId"] }`, or `{ "in": ["paymentToken", ["<base58>", "<base58>"]] }`. `$ownerId`, the document's owner, is an identifier operand as well (`{ "equal": ["authorId", "$ownerId"] }`), and a transfer or purchase that would break such a rule is refused. `{ "startsWith": ["url", { "const": "https://" }] }` and `endsWith` test a string property's start or end, byte for byte, against a constant or another string property. `{ "contains": ["participants", "$ownerId"] }` holds when a typed array property has an element equal to the value, looked for as the array's elements are (an integer expression, a string or an identifier), so `{ "not": { "contains": ["labels", { "const": "used" }] } }` refuses a label; the array is reported as a read of kind `elements`. `anyOf` holds if at least one of two or more conditions holds, `allOf` if every one does, `not` if its one condition does not, `{ "ifThen": [a, b] }` if `b` holds whenever `a` does (evaluating `b` only then), and `{ "ifThenElse": [a, b, c] }` if `b` holds when `a` does and `c` when it does not; `{ "notIn": [expression, [values]] }` is an `in` negated, and `min`, `max` (two or more operands) and `abs` (one) join the arithmetic; conditions are checked in order and `anyOf` stops at the first that holds, so `{ "anyOf": [{ "equal": ["b", 0] }, { "equal": [{ "divide": ["a", "b"] }, 2] }] }` never divides by zero. An operand may read an integer or a boolean property (true as 1, false as 0), or a size: `{ "length": path }` and `{ "byteLength": path }` give the characters and UTF-8 bytes of a string property, and `{ "count": path }` the items of an array or the bytes of a byte array, so `{ "lessThanOrEqual": [{ "count": "tags" }, "maxTags"] }` holds a list to its own limit (a size read is reported with kind `length` or `count`). A type that lists `$createdAt`, `$updatedAt` or `$transferredAt` (or any of them with `BlockHeight` or `CoreBlockHeight` appended) in `required` may read it too: `{ "lessThanOrEqual": [{ "subtract": ["endsAt", "$createdAt"] }, 604800000] }` keeps a listing to a week, and since a price update sets `$updatedAt` and a transfer or purchase `$transferredAt`, each is judged against the rules reading those. `{ "countOf": [type, filter] }` and `{ "sumOf": [type, property, filter] }` read a total from state, how many documents of a type of the same contract match the filter or what an integer property adds up to over them, as the type's count or sum trees keep it once the write is done: `{ "lessThanOrEqual": [{ "countOf": ["listing", { "$ownerId": "$ownerId" }] }, 10] }` on `listing` keeps every owner at ten listings or fewer. The filter maps keys of the counted type (or `$ownerId`) to values read from the document being written, and may be left out for a whole-type total; the type needs `documentsCountable` or `documentsSummable` for a whole-type total, and an index whose properties are exactly the filter's keys (countable, or summing the property) otherwise. A property the document leaves out counts as 0, or as the value of an `ifAbsent` operand naming it; `{ "present": path }` and `{ "absent": path }` tell a property left out from one set to 0, and may name a property of any type. The arithmetic is exact over 128-bit integers, and `divide` and `modulo` are Euclidean, so a remainder is never negative. The rules are fixed when the document type is created.

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

To find a broken rule before paying for a refused transition, a contract lists a document type's rules and checks a document against them with the code consensus runs. The check covers the rules alone, not the JSON schema. It reads the document's owner for `$ownerId`, and the device clock for the times the write will record (`readsSystem` lists the ones a rule reads); a rule reading a block height is not checked, since the height is unknown until the block, and neither is a rule reading a `countOf` or `sumOf` total, which only the platform reads from state (`readsTotals` lists the ones a rule reads, each with its `kind`, `documentType`, the summed `property` of a `sumOf` and the `filter` keys):

```ts
contract.documentTypePropertyConstraints('offer');
// [{ name: 'discountBelowPrice', rule: { lessThan: ['discount', 'price'] },
//    reads: [{ path: 'discount', kind: 'value' }, { path: 'price', kind: 'value' }],
//    readsOwner: false, readsSystem: [], readsTotals: [] }, ...]

const broken = contract.checkDocumentPropertyConstraints(document);
if (broken) {
  // { rule: 'discountBelowPrice', violation: 'NotMet', message: 'it does not hold' }
}
```

Rules come back in name order, the order consensus checks them in; `contract.documentPropertyConstraints` maps every document type that declares rules to its list. The `PropertyConstraintCondition`, `PropertyConstraintExpression` and `PropertyConstraintEqualityOperand` types spell out the rule grammar, and `violation` is one of `NotMet`, `Overflow`, `DivisionByZero`, `NegativeExponent` or `NotAnInteger`, the reason consensus would report.

## How a document type is stored (`documentTypeLayout`)

`documentTypeLayout(contract, documentTypeName, platformVersion)` returns the GroveDB layout of a document type as Drive writes it: the document type tree, the documents by id and, for each index, the property and value trees down to where the index ends. Each layer carries the tree or element type Drive writes there (a count or sum tree, a ranked indexed tree, a reference, an indexOnly item), the wrapper a continuation tree gets under an aggregating value tree, the indexes that use it, and conditions such as the tree a unique index falls back to when a value is null. It runs locally with Drive's own rules, those of protocol version 14 on (an earlier version is refused), so it needs no connection:

```ts
import { documentTypeLayout, PlatformVersion } from '@dashevo/evo-sdk';

const { root } = documentTypeLayout(contract, 'review', new PlatformVersion(14));
// root.children: the documents by id ([0]) and one tree per first index property;
// each node: { key, role, element, wrapper?, rankedAxes, indexes, notes, alternative?, children, structureNode }
// (wrapper and alternative are left out when there is none)
```

`structureNode` names the layer of Drive's GroveDB structure description it is an instance of, as the [GroveDB structure viewer](https://dashpay.github.io/grovedb-structure-viewer/) shows it (`#/<structureNode>`).

## What a document costs (`documentCreateCost`)

`documentCreateCost(contract, documentTypeName, options, platformVersion)` returns what creating a document of a type costs, in credits (`creditsPerDash` of them make one Dash), computed locally by Drive from the contract:

- `storage`: the bytes the insert writes and their fee, exact, under two scenarios: `newValues` (the first document with these index values creates their trees) and `knownValues` (a later document with the same values adds only its own entries);
- `indexes`: per index, the bytes of the layers it shares with other indexes and of its own, so its cost on its own is `sharedBytes + ownBytes`;
- `processing`: the signature and identity fetch (exact) and the work of the writes (estimated for `existingDocuments` stored documents);
- `contractCharges`: the create's action fee, token cost and contest fund, when the type has them (a contested create is stored in the vote poll until the contest ends; that storage is not priced);
- `refund`: what a delete refunds, in the same epoch and a year later;
- `fields`: how the priced document was filled.

The document is built from sizes, not values: by default each variable-size field is at the middle of its bounds and each optional field is present. Pass `fields` to change that:

```ts
import { documentCreateCost, PlatformVersion } from '@dashevo/evo-sdk';

const cost = documentCreateCost(contract, 'note', {
  fields: { text: { length: 200 }, mood: { present: false } },
  existingDocuments: 10_000,
}, PlatformVersion.latest());
const dash = cost.totalCredits.newValues / cost.creditsPerDash;
```

It follows protocol version 14 on; an earlier version is refused. A type whose documents have a `ttl` is priced by lifetime, with no refund.

## Chained queries (provable semi-join)

A `refersTo: permanentDocument` declaration also lights up the read side: a **chained query** answers `SELECT * FROM post WHERE $id IN (SELECT postId FROM like WHERE $ownerId = me)` in one verified round trip. The node returns the inner indexOnly page and the referenced documents under ONE merged proof — a single quorum-signed state root by construction — and the SDK re-derives the outer query itself and checks it against the *proven* inner values — the node cannot substitute, omit, or inject joined documents. For a `permanentDocument` join property a missing referenced document fails verification outright, since such a reference cannot dangle. For a `deletableDocument` join property a referenced document that was deleted since is proven absent: it has no entry in `outerDocuments` (so match the two halves by id, not by position) and its id is listed in `missingOuterIds`, in first-appearance order. For a `moderatedDocument` join property a referenced document a moderator removed has no entry in `outerDocuments` either, and is listed in `removedOuterDocuments` with its proven removal record (`documentId`, `documentOwnerId`, `moderatorId`, `reason`, `removedAt`, `documentHash`, and `keptFields`, the values of the fields its type keeps public under `moderatorAbilities.deleteKeepsFields`), in first-appearance order; one gone without a record fails verification. The node still cannot pass an existing document off as deleted, nor a removed one off as missing.

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

The inner query must target an indexOnly document type and resolve to an index carrying `joinProperty`, and `joinProperty` must declare a same-contract `refersTo: permanentDocument`, `refersTo: moderatedDocument` or `refersTo: deletableDocument` targeting `outerDocumentType`. `innerLimit` is required — it bounds the derived outer fetch, so there is no server-default fallback. There are no outer-side clauses by design; filter `outerDocuments` locally. `sdk.documents.chainedWithProof(...)` returns the same result with the metadata and proof envelope attached.

## Composite queries (a page plus its sub-queries)

A **composite query** answers a page and everything a UI needs to render it in ONE verified round trip: the page documents, plus one to ten sub-queries whose `IN` clause the node derives from the proven page (or from an earlier `documents` sub-query). The request never names the derived values. Four sub-query shapes exist:

- a **by-id join** (`bind.field: '$id'`): the documents a page property refers to (the property must declare `refersTo: permanentDocument`, `refersTo: moderatedDocument` or `refersTo: deletableDocument` targeting the sub-query's type; a missing document fails verification for the first; for the second a document a moderator removed is left out of `documents` and listed with its proven removal record in that sub-result's `removed`; for the third it is proven absent, left out of `documents` and listed in that sub-result's `missingIds`);
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
