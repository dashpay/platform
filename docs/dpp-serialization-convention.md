# Persisting dpp types: canonical serialization pattern

Authoritative reference for adding a new persisted or wire-encoded value —
anywhere in this workspace — that includes an `rs-dpp`-originated type. If
you're about to add a new blob column, cache file, or IPC payload that embeds
a dpp type, read this first.

This doc covers **binary persistence / wire encoding** (SQLite blobs, files,
inter-process bytes). For JSON/`platform_value::Value` conversion (API
surfaces, wasm bindings), see
[`json-value-conversion-canonical-pattern.md`](./json-value-conversion-canonical-pattern.md)
— a related but separate concern with its own canonical traits.

## TL;DR

1. Before persisting or wire-encoding a value, check whether it — or any
   transitive field — is a dpp type shaped `#[serde(tag = "...")]`
   (internally tagged), `#[serde(untagged)]`, or `#[serde(flatten)]`. See the
   [risk checklist](#how-to-check-if-your-new-type-is-at-risk).
2. If it is, **do not** route it through a generic, non-self-describing
   serde bridge (`bincode::serde` and friends). Use the dpp type's own
   canonical codec instead: native bincode `Encode`/`Decode` if it derives
   them, or `PlatformSerializable`/`PlatformSerializableWithPlatformVersion`
   if it implements them.
3. Pick [wire-type field wrapping](#pattern-a-wire-type-field-wrapping) or
   [whole-value native serialize](#pattern-b-whole-value-native-serialize)
   depending on shape — see the table below.
4. Write the [mandatory round-trip test](#mandatory-test-requirement): every
   enum variant, populated with real data — never only the empty/default
   case.

## Why this exists

`rs-dpp` deliberately internally-tags several enums (`#[serde(tag = "...")]`)
so their JSON/Value wire shape is self-describing and stable
(`IdentityPublicKey`, `AssetLockProof`, `DataContractConfig`, `StateTransition`,
and others — see the
[tag-key conventions](./json-value-conversion-canonical-pattern.md#tag-key-conventions)
table). That shape is correct and intentional for JSON/Value. It is also, by
construction, incompatible with any deserializer that isn't self-describing:
decoding an internally-tagged (or `untagged`, or `flatten`) enum requires
buffering the input to find the discriminant before knowing which variant to
build — a `deserialize_any` call. `bincode`'s serde bridge (`bincode::serde`)
has no length/type prefix to support that lookahead, so it cannot service
`deserialize_any`. **Encoding never needs lookahead, so it always succeeds;
decoding always fails.** A value written this way is write-once, read-never —
and because the bytes on disk never captured a self-describing shape, no
decoder (serde-based or otherwise) can reconstruct it after the fact.

This has already happened twice in this workspace, discovered by crash both
times rather than by design:

- **`IdentityPublicKey`** (`#[serde(tag = "$formatVersion")]`) — reached
  `rs-platform-wallet-storage`'s shared blob codec through
  `IdentityKeyEntry.public_key`. Mitigated by pre-encoding the key field with
  bincode's native `Encode`/`Decode` before it reaches the generic bridge
  (`IdentityKeyWire`, `packages/rs-platform-wallet-storage/src/sqlite/schema/identity_keys.rs`).
- **`AssetLockProof`** (`#[serde(tag = "$type")]`) — reached the same blob
  codec through `AssetLockEntry.proof`. A row with `proof: Some(_)` encoded
  successfully but could never be decoded back, so every wallet holding one
  failed rehydration on every relaunch
  ([issue #4133](https://github.com/dashpay/platform/issues/4133)). The
  defect escaped the existing test suite because every fixture used
  `proof: None`, which never touches the internally-tagged inner enum.

Both are the same bug class: a dpp type's serde shape is fine for JSON/Value,
unsafe for a non-self-describing binary bridge, and nothing in `rs-dpp` told
the downstream crate that. `DataContractConfig`
(`packages/rs-dpp/src/data_contract/config/mod.rs`, also
`#[serde(tag = "$formatVersion")]`) has the identical shape and is not
currently blob-persisted anywhere — it is a latent third occurrence the
moment someone adds it to a persisted changeset. Treat it as a name to watch
for, not a bug to fix today.

## The rule

> Before persisting or wire-encoding a value that is, or transitively
> contains, an `rs-dpp` type: check whether that type's `Deserialize`
> requires `deserialize_any` (internally-tagged `#[serde(tag = ...)]`,
> `#[serde(untagged)]`, or `#[serde(flatten)]`). If it does, do not route it
> through a generic, non-self-describing serde bridge. Use the type's own
> canonical dpp codec — native bincode `Encode`/`Decode`, or
> `PlatformSerializable`/`PlatformSerializableWithPlatformVersion` — via one
> of the two patterns below.

This is not a new trait bound, lint, or proc-macro. The hazard is a
**transitive field** of the persisted type, invisible to any type-level
mechanism operating on the top-level type alone — a `T: Serialize` blob type
with a hidden internally-tagged field still compiles and still fails only at
decode time. Only an actual round-trip test with realistic, populated data
can catch it; see [Mandatory test requirement](#mandatory-test-requirement).

## How to do it right

Two established patterns, depending on shape:

| Pattern | Use when | Reference |
|---|---|---|
| A — wire-type field wrapping | The offending field is one of several plain-serde-compatible fields in a struct you still want to persist as one blob via the generic bridge. | `IdentityKeyWire` in `packages/rs-platform-wallet-storage/src/sqlite/schema/identity_keys.rs` |
| B — whole-value native serialize | The persisted value *is* the dpp type (or is dominated by it), stored in its own column/file with nothing else riding the same bridge call. | `StateTransition` persistence in `packages/rs-platform-wallet/src/wallet/shielded/operations.rs` (`arm_redrive_record` / redrive-read path), backing the `st_bytes` column defined in `packages/rs-platform-wallet/src/wallet/shielded/file_store.rs` |

### Pattern A: wire-type field wrapping

Define a private on-disk struct mirroring the persisted type field-for-field,
except the offending field becomes a pre-encoded `Vec<u8>` (or
`Option<Vec<u8>>`). Encode that one field with the dpp type's own codec
*before* the struct as a whole goes through the generic bridge; decode it
back out on read. `IdentityKeyWire` is the reference: `IdentityPublicKey` is
pre-encoded via `bincode::encode_to_vec`/`decode_from_slice` (its native
`Encode`/`Decode` derive), while `identity_id`, `key_id`,
`public_key_hash`, and the rest ride the ordinary serde bridge unchanged.
Reject trailing bytes on decode of the inner payload — a valid-prefix-plus-
garbage blob is corruption, not something to silently truncate.

Use this when the persisted row/blob is a composite of the dpp field plus
unrelated plain fields that don't need special handling, and you want to
keep "one blob per row."

### Pattern B: whole-value native serialize

Serialize the entire dpp value via its own
`PlatformSerializable::serialize_to_bytes()` /
`PlatformDeserializable::deserialize_from_bytes()` into one opaque blob
column, with no generic bridge involved anywhere in the path. The shielded
redrive path is the reference: it stores a dpp `StateTransition` (internally
tagged, would break instantly under `bincode::serde`) by calling
`serialize_to_bytes()` before the write and `StateTransition::deserialize_from_bytes()`
on read, treating the column as opaque bytes end to end.

Use this when there's no surrounding composite struct to preserve — the
persisted unit and the dpp type are the same thing.

### Which codec: native bincode vs `PlatformSerializable`

A dpp type may derive native bincode `Encode`/`Decode`, implement
`PlatformSerializable`/`PlatformSerializableWithPlatformVersion`, or both.
`PlatformSerializable` is the more complete codec where it exists — it
carries the type's own declared size limit and versioning behavior (e.g.
`#[platform_serialize(limit = 2000, unversioned)]` on `IdentityPublicKey`).
When a type implements it, prefer it for whole-value blobs (Pattern B) so
the type's own bound applies rather than the containing blob's cap.

For Pattern A, either mechanism the type offers is acceptable, but pick one
deliberately and note the choice in the wire struct's doc comment — don't
default to whichever happens to already be imported nearby. Be aware the two
mechanisms can carry different size bounds for the same type (e.g.
`IdentityPublicKey`'s `PlatformSerializable` caps at 2000 bytes;
encoding it via the raw native derive instead inherits whatever cap the
surrounding blob format uses). If a type derives *only* native bincode
(as `AssetLockProof` does), that is simply the mechanism to use — there is
no ambiguity to resolve.

Never invent a third, ad hoc codec for a dpp type when one of the above
already exists.

## Mandatory test requirement

Every type persisted via Pattern A or B needs a round-trip test that:

- Encodes and decodes a **populated, non-default** instance — real field
  values, not `None`/empty/zero.
- For an enum-shaped field, **exercises every variant**, not just one. The
  `AssetLockProof` bug survived precisely because every existing fixture
  used `proof: None`, which never touches the internally-tagged inner enum
  at all; a `Some(AssetLockProof::Instant(..))` and a
  `Some(AssetLockProof::Chain(..))` fixture would have failed immediately.
- Asserts the round-tripped value equals the original (not just "decode
  didn't error") — a lossy decode is still a bug.
- Rejects trailing-byte corruption of the pre-encoded inner field, mirroring
  `into_entry_rejects_trailing_bytes_in_public_key_bincode` in
  `identity_keys.rs`.

A representative-but-not-exhaustive fixture is not sufficient coverage — it
is exactly the gap that let this recur. If a persisted type has an
enum-shaped dpp field, the test suite must cover each variant explicitly.

## How to check if your new type is at risk

Before adding a field to any persisted/wire-encoded struct, or persisting a
new dpp type directly:

- [ ] Does the type, or any field reachable through it, carry
      `#[serde(tag = "...")]` (internally tagged), `#[serde(untagged)]`, or
      `#[serde(flatten)]`? (Check the type's own definition in `rs-dpp`, not
      just the field's declared type name — tags live on the enum
      definition.)
- [ ] Does that type derive native bincode `Encode`/`Decode`, or implement
      `PlatformSerializable`/`PlatformSerializableWithPlatformVersion`? If
      neither exists, the type has no canonical binary codec yet — raise
      that in `rs-dpp`, don't invent a downstream workaround.
- [ ] Is the persistence/wire path a non-self-describing bridge — anything
      built on `bincode::serde`, or another codec whose deserializer can't
      look ahead for a discriminant? If a bridge like this touches a type
      that answered yes above, apply Pattern A or B before merging.

## Quick references

- Canonical dpp codecs: `packages/rs-dpp/src/serialization/serialization_traits.rs`
  (`PlatformSerializable`, `PlatformDeserializable`, and their
  platform-versioned variants).
- Pattern A reference: `packages/rs-platform-wallet-storage/src/sqlite/schema/identity_keys.rs`
  (`IdentityKeyWire`).
- Pattern B reference: `packages/rs-platform-wallet/src/wallet/shielded/operations.rs`
  (`serialize_to_bytes()` / `deserialize_from_bytes()` around the `st_bytes`
  column declared in `packages/rs-platform-wallet/src/wallet/shielded/file_store.rs`).
- Generic blob codec this convention guards against misusing:
  `packages/rs-platform-wallet-storage/src/sqlite/schema/blob.rs`.
- Named future risk: `DataContractConfig` in
  `packages/rs-dpp/src/data_contract/config/mod.rs` — same internally-tagged
  shape, not yet blob-persisted.
- Incident record: [issue #4133](https://github.com/dashpay/platform/issues/4133).
- JSON/Value conversion (a related, separate convention):
  [`json-value-conversion-canonical-pattern.md`](./json-value-conversion-canonical-pattern.md).
