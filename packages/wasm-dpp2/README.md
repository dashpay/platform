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
