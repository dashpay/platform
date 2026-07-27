# wasm-dpp2 Conventions

This document captures the **single rule** for how protocol structs are exposed
to TypeScript/JavaScript through `wasm-dpp2`, and the small set of sub-rules
that follow from it. The goal is that every wrapper looks the same to a TS
developer — there are no per-type stylistic choices, no "this enum has `data`
because of a Rust mechanic," no manual reshaping in the WASM layer.

## The principle

> **`rs-dpp` serde defines the canonical wire shape. `wasm-dpp2` mirrors it
> 1:1, modulo primitive encoding.**

If `serde_json::to_value(x)` in `rs-dpp` produces shape `S`, then
`xWasm.toJSON()` produces the same `S`, and `xWasm.toObject()` produces a
structurally identical shape with `Uint8Array` instead of base64 strings and
`bigint` instead of safe-number-or-string for large integers.

Wrappers do not invent shapes. Wrappers do not reshape. Wrappers delegate.

## wasm-dpp2 is a thin TypeScript convenience layer

`wasm-dpp2` is not a validation tier. It does not duplicate, pre-empt, or
extend `rs-dpp` logic. Its job is to:

- expose `rs-dpp` types to JavaScript through wasm-bindgen,
- adapt JS-friendly inputs (Options interfaces) into `rs-dpp` types,
- surface `rs-dpp`'s outputs (toObject / toJSON / getters) with TypeScript
  types accurate to the actual wire shape.

It is **not** the place to:

- check field lengths, ranges, or counts that `rs-dpp` validates (e.g.
  `encrypted_note.len() == 216`, `inputs.len() == input_witnesses.len()`,
  `actions.len() <= max_actions`),
- enforce business rules,
- pre-validate state-transition structure.

If `rs-dpp` enforces a constraint at structural validation, `wasm-dpp2` does
not duplicate it at the constructor boundary — even when the duplicate would
give an earlier or clearer error. Two reasons:

1. **Single source of truth.** Constraints live in `rs-dpp` so they apply
   uniformly across all consumers (Rust SDK, drive-abci, wasm-dpp2, …). A
   second copy in `wasm-dpp2` drifts the moment `rs-dpp` changes its rule
   and creates two definitions of "valid" with subtle disagreements.
2. **Layering.** Validation tiers (DPP structural validation, drive-abci
   state validation, consensus) decide when to reject. The wasm boundary's
   job is to construct the type, not gate-keep ahead of those tiers.

The narrow exceptions (things that look like validation but aren't):

- **Structural conversions required by the Rust type system** — e.g.,
  converting `Vec<u8>` to `[u8; N]` for fields that are already
  fixed-size in `rs-dpp` (`anchor`, `bindingSignature`, etc.). The length
  check is a side-effect of the type conversion, not a separate validation.
- **Information-preserving guards** — e.g., `inputs_to_btree_map` rejects
  duplicate addresses because `BTreeMap` would silently drop the second
  entry. The check protects against information loss during the conversion,
  not against an `rs-dpp` constraint.
- **JS-side ergonomic conversions** — e.g., accepting a hex string or
  `Uint8Array` for an address, parsing strings to numbers, etc.

If you find yourself adding `if x.len() < N || x.len() > M` to a
constructor or setter, that's a `rs-dpp` validation function in disguise.
Move it.

Wrappers do not invent shapes. Wrappers do not reshape. Wrappers delegate.

## Sub-rules

### Object vs JSON

Both shapes are **structurally identical** — same fields, same nesting, same
discriminators. They differ **only in primitive encoding**:

|  | Object form (`toObject`) | JSON form (`toJSON`) |
| --- | --- | --- |
| Bytes (`[u8; N]`, `Vec<u8>`) | `Uint8Array` | base64 string |
| Identifiers | `Uint8Array` | base58 string |
| Addresses | `Uint8Array` | hex string |
| `u64` / `i64` (and aliases like `Credits`) | `bigint` | `number` if ≤ `MAX_SAFE_INTEGER`, else `string` |
| Everything else | passthrough | passthrough |

The `#[json_safe_fields]` proc-macro injects the right `#[serde(with = ...)]`
helpers automatically for the integer and byte cases.

### Tagged unions

All sum types use **internal tagging**: `{ type: "variantName", ...fields }`.
No `data` wrapper. No external tagging.

```typescript
// Good — internally tagged, flat.
type AssetLockProofObject =
  | ({ type: "instant" } & InstantAssetLockProofObject)
  | ({ type: "chain" } & ChainAssetLockProofObject);

// Bad — adjacent tag (`data` wrapper).
type AssetLockProofObject =
  | { type: "instant"; data: InstantAssetLockProofObject }   // do NOT do this
  | { type: "chain"; data: ChainAssetLockProofObject };
```

In `rs-dpp`, this means `#[serde(tag = "type", rename_all = "camelCase")]`
without `content`. serde supports this on:

- struct variants (`Variant { a: A, b: B }`)
- newtype variants whose inner type is a named struct
  (`Variant(SomeNamedStruct)`)

If a variant doesn't fit either shape (for example a tuple variant of a
non-struct type), the type needs a custom `Serialize` / `Deserialize` impl that
emits the flat shape manually. `AddressFundsFeeStrategyStep` and
`AddressWitness` are the existing precedents.

### Versioning

Versioned protocol structs use **`tag = "$formatVersion"`** with each variant
renamed to its version string:

```rust
#[serde(tag = "$formatVersion")]
pub enum FooTransition {
    #[serde(rename = "0")]
    V0(FooTransitionV0),
    #[serde(rename = "1")]
    V1(FooTransitionV1),
}
```

`$formatVersion` is the universal key. Do **not** use `$version` for new
versioned enums — that key is reserved for legacy document/state-transition
protocol-version fields in `common_fields.rs`, and reusing it for versioned
serde tagging caused divergence in the past.

### Wrapper plumbing

A WASM wrapper is a transparent newtype over the inner DPP type:

```rust
#[wasm_bindgen(js_name = "Foo")]
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct FooWasm(Foo);
```

Conversions are wired up via one of two macros (defined in
`packages/wasm-dpp2/src/serialization/conversions.rs`):

- **`impl_wasm_conversions_inner!`** — when the inner type implements
  `JsonConvertible` and `ValueConvertible` (the trait-based path). Preferred.
- **`impl_wasm_conversions_serde!`** — when the inner type only has
  `Serialize`/`Deserialize` (no trait impls). Used for newer types and shielded
  wrappers.

Both produce `toObject` / `toJSON` / `fromObject` / `fromJSON` with consistent
behavior. Direct `js_sys::Reflect::set` building of conversion shapes is
**forbidden**: the rs-dpp serde derive defines the shape and the macros
deliver it transparently.

### Getters and setters

The split: **field-like accessors are properties; verbs are methods.**

In ideal JS conventions, properties are cheap and methods imply work. In
wasm-bindgen, **every** accessor crosses the wasm boundary — so there is no
truly "light" property anyway. The codebase reflects this: all field-like
accessors (including ones that clone Vecs, build maps, or wrap inner types)
are exposed as properties — `Identity.publicKeys`, `Document.properties`,
`IdentityCreditWithdrawalTransition.outputScript`, etc. Methods are reserved
for **actions**: things that take parameters, mutate beyond simple set, or
genuinely "do" something (`toBytes`, `toStateTransition`, `createIdentityId`).

```rust
// Good — property style. JS sees `transition.amount`.
#[wasm_bindgen(getter = "amount")]
pub fn amount(&self) -> u64 { ... }

#[wasm_bindgen(setter = "amount")]
pub fn set_amount(&mut self, value: u64) { ... }

// Good — property even though it allocates a Vec (matches Identity.publicKeys).
#[wasm_bindgen(getter = "actions")]
pub fn actions(&self) -> Vec<SerializedOrchardActionWasm> { ... }

// Bad — method style for a field. JS sees `transition.getAmount()`.
#[wasm_bindgen(js_name = getAmount)]
pub fn get_amount(&self) -> u64 { ... }
```

The Rust function name should be the same as the field (no `get_` prefix),
and `getter = "..."` should carry the camelCase JS name. Setters use
`setter = "..."` with `set_` prefixed Rust fn names.

**Return wasm wrapper types** when one exists for the field. Raw types
(`Vec<u8>`, primitives) are reserved for opaque crypto blobs (proofs,
signatures, anchors, encrypted notes) where no wrapper applies:

```rust
// Good — typed wrappers for fields with wrappers.
#[wasm_bindgen(getter = "outputScript")]
pub fn output_script(&self) -> CoreScriptWasm { ... }

#[wasm_bindgen(getter = "assetLockProof")]
pub fn asset_lock_proof(&self) -> AssetLockProofWasm { ... }

// Good — raw bytes for opaque crypto blobs.
#[wasm_bindgen(getter = "proof")]
pub fn proof(&self) -> Vec<u8> { ... } // Halo2 proof bytes

#[wasm_bindgen(getter = "anchor")]
pub fn anchor(&self) -> Vec<u8> { ... } // Sinsemilla root
```

For pooling enums and similar small enums that JS sees as named strings,
wire through the typed wasm enum so the conversion is centralized:

```rust
#[wasm_bindgen(getter = "pooling")]
pub fn pooling(&self) -> String {
    PoolingWasm::from(self.0.pooling()).into()
}
```

Don't expose `getType()` / `state_transition_type()` — that field isn't
exposed on any other transition wrapper, and JS callers can use
`instanceof` against the wasm class instead.

### Constructor inputs

`XxxOptions` constructor input types are **allowed to be looser** than the
output `Object` / `JSON` types:

- optional fields with sensible defaults
- accept either a wasm wrapper instance or its plain-object form
- accept either `Uint8Array` or hex/base64 string for byte fields where
  ergonomic

The output shapes (`Object`, `JSON`) are strict and complete — no missing
fields, no per-call discretion. Constructors do the normalization.

### Test coverage for `Object` / `JSON` round-trips

Every wasm wrapper that exposes `toObject` / `fromObject` / `toJSON` /
`fromJSON` **must** have a unit test covering all four methods, with:

1. **Per-property assertions on `toObject()` output.** Don't just assert the
   call doesn't throw — assert the shape: instance types (`Uint8Array`,
   `bigint`, `Map`), exact lengths for fixed-size byte fields, exact values
   for known fixtures.

2. **Per-property assertions on `toJSON()` output.** Same idea, JSON-side:
   strings (base58 / base64 / hex), `string | number` for safe-integer u64s,
   discriminator fields on tagged enums.

3. **Round-trip via `fromObject(toObject())`.** Catches latent serde bugs in
   the bytes / map deserialization path that pure `toObject()` shape checks
   never exercise (this is how PR #3235 surfaced the
   `serde_bytes_var` / `serde_bytes` `visit_bytes` gap).

4. **Round-trip via `fromJSON(toJSON())`.** Same idea, JSON-side. Matters
   especially for human-readable encodings of non-string types
   (`bigint` ↔ string, `Identifier` ↔ base58, `BinaryData` ↔ base64).

This applies to wasm-dpp2 wrappers and any wasm-sdk result type that
implements the same four methods (`PlatformAddressInfo`,
`ShieldedEncryptedNote`, `ShieldedNullifierStatus`, `ResponseMetadata`,
`ProofInfo`, `IdentityBalanceAndRevision`, etc.).

> **Migration backlog** (see end of file): there are 78 wasm-dpp2 specs
> today; ~46 are missing `fromObject` coverage and ~37 are missing
> `fromJSON` coverage. Tracked separately from PR #3235.

## Bincode is independent of serde

Custom serde impls (used to reshape JSON output, like
`AddressFundsFeeStrategyStep`, `AssetLockProof`, `AddressWitness`, the
`address_funds/serde_helpers/` map reshapers) **do not touch bincode**. The
`Encode` / `Decode` derives drive the consensus binary format; serde drives
JSON / `platform_value`. Reshaping JSON for ergonomics is always safe as long
as bincode derives stay in place.

## Migration backlog

The following types still violate the convention and should be aligned in
follow-up PRs (separate from this one to keep blast radii sensible):

- **Adjacent-tagged enums still emitting `{type, data}`** (8 in rs-dpp), to be
  flattened to internal tagging:
  - `Vote` (`packages/rs-dpp/src/voting/votes/mod.rs`)
  - `VotePoll` (`packages/rs-dpp/src/voting/vote_polls/mod.rs`)
  - `ResourceVoteChoice`
    (`packages/rs-dpp/src/voting/vote_choices/resource_vote_choice/mod.rs`)
  - `TokenEvent` (`packages/rs-dpp/src/tokens/token_event.rs`)
  - `GroupActionEvent` (`packages/rs-dpp/src/group/action_event.rs`)
  - The two `AssetLockProof` derives in
    `packages/rs-dpp/src/identity/state_transition/asset_lock_proof/mod.rs`
    are already flat as of PR #3235.
  - The associated wasm-dpp2 wrappers (`ResourceVoteChoiceObject`,
    `ContestedDocumentVotePollWinnerInfoObject`) drop their `data` slots once
    the rs-dpp side flattens.

  Note: serde's internal tagging works on **struct variants** and on **newtype
  variants whose inner type is a named struct**. If a tuple variant wraps a
  non-struct type (a `Uint8Array`, a primitive, a tuple), the enum needs a
  custom `Serialize`/`Deserialize` impl that flattens manually — same pattern
  `AddressFundsFeeStrategyStep` and `AddressWitness` already use.

- **Parent state transitions have no `toObject` / `toJSON`** — only
  `toBytes`/`toHex`/`toBase64`. Sub-transitions do, but the wrapping
  `StateTransition` does not. Either add the methods following this
  convention, or document the deliberate gap in this file.

- **`tag = "$version"` → `tag = "$formatVersion"` migration** — fixed for the
  5 shielded transitions in PR #3235. Audit anything else still on `$version`
  in newer code.

- **Round-trip + per-property test coverage** — see "Test coverage for
  `Object` / `JSON` round-trips" above. As of PR #3235, ~46 of the 78
  wasm-dpp2 specs are missing `fromObject` coverage and ~37 are missing
  `fromJSON` coverage. Same pattern needed in wasm-sdk for its result
  wrappers. Track as a dedicated cleanup PR — large mechanical sweep,
  expected to surface latent serde bugs along the way (the
  `serde_bytes_var` / `serde_bytes` `visit_bytes` gap is the first known
  example, fixed in PR #3235).
