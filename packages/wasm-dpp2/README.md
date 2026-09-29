# @dashevo/wasm-dpp2

Internal build of the Dash Platform Protocol v2 WebAssembly bindings.

## Scripts

- `yarn build` – run the unified WASM build and bundle artifacts into `dist/`
- `yarn build:release` – same as `build` but forces full release optimisations
- `yarn test` – execute unit + browser (Karma) tests
- `yarn lint` – lint test sources

The build scripts defer to `packages/scripts/build-wasm.sh` to keep behaviour
consistent with other WASM packages such as `@dashevo/wasm-sdk`.

## Document ids

From protocol version 14 the id of a new document commits to the identity
contract nonce of its create transition (see the book, *Data Model →
Documents → Document ID Generation*), so a `Document` built with
`new Document({...})` and no `identityContractNonce` carries a **placeholder**
id: the entropy-only derivation of earlier versions, which consensus no longer
accepts.

The id becomes final where the nonce is known, and the bindings derive it for
you:

```ts
const document = new Document({ properties, documentTypeName, dataContractId, ownerId });

// Derives the id from the document's entropy and the nonce, writes it onto
// the transition and back onto `document`: document.id equals transition.base.id.
const transition = new DocumentCreateTransition({ document, identityContractNonce: nonce });

// To know the id before the transition exists (another document in the same
// batch references it):
document.setIdForCreation(nonce);
// or derive it without a Document:
const idBytes = Document.generateId(documentTypeName, ownerId, dataContractId, entropy, nonce);
// or build the document with its final id from the start:
const ready = new Document({ properties, documentTypeName, dataContractId, ownerId, identityContractNonce: nonce });
```

Each of these takes an optional `platformVersion` (latest by default); before
protocol version 14 the derivation ignores the nonce. Whatever id a `Document`
carried before it is passed to `DocumentCreateTransition` is replaced: the
transition can only carry the id consensus recomputes. For the same reason
`new Document({...})` refuses an explicit `id` that disagrees with the one its
`identityContractNonce` derives. No app needs to reimplement the hash.

## Checking a contract update

A data contract update is refused when it breaks an update rule: the version
must rise by exactly one, the indexes of an existing document type are fixed,
a property may be added but not removed, a new required property needs
`requiredSince`, and so on for every keyword (see the book, *Contract
Keywords*, the *On update* row of each keyword). `DataContract.validateUpdate`
runs those rules with the code consensus runs on the update transition
(`DataContract::validate_update`), so an app, a CI job or a review tool can
check a proposed contract before paying for a transition the platform would
refuse:

```ts
const version = new PlatformVersion(14);
const current = DataContract.fromJSON(currentJson, true, version);
const proposed = DataContract.fromJSON(proposedJson, true, version);

const errors = current.validateUpdate(proposed, undefined, version);
// [] when the update is valid; otherwise ConsensusError objects, e.g.
// errors[0].code === 10217 when an index of an existing type changed
```

Each error is a `ConsensusError` with the `code` a refused transition reaches
JS with. The second argument is the `BlockInfo` of the block the update would
be executed in, of which only the time is read (an added token's
pre-programmed distributions may not start before it); `undefined` uses the
device clock. The two contracts are compared whatever their ids, as the
platform compares the stored contract with the proposed one. Build the
proposed contract with full validation, as above, since the transition's
structural checks run there. No state is read, so what the platform checks
against state on top is not covered: that the contract exists, that group
members, identities named by token configurations and appointed moderators
exist, that references into other contracts resolve, and the signature, nonce
and fees.

This package is built with the crate's `validation` feature, which the method
needs. It also makes full validation (`fromJSON(…, true, …)`, `fromObject`,
`fromBytes`, `new DataContract({ fullValidation: true })`) run the document
meta-schema, and contract parse errors come back as consensus errors with
codes. `@dashevo/wasm-sdk` and `@dashevo/evo-sdk` build these bindings without
it, to keep their size, so they have no `validateUpdate`.
