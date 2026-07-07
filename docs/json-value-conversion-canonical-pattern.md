# JSON / Value conversion: canonical pattern

Authoritative reference for adding a new domain type to `rs-dpp` and exposing
it through `wasm-dpp2`. If you're tempted to write a custom `to_json` /
`to_object` inherent method, read this first.

This doc covers **how to do it correctly today**.

## TL;DR

For a new struct or enum:

1. Derive `Serialize`, `Deserialize`, `JsonConvertible`, `ValueConvertible`.
2. If versioned, use `#[serde(tag = "$formatVersion")]` (see [tag conventions](#tag-key-conventions)).
3. Write a round-trip test using the [test template](#test-template) below.
4. In `wasm-dpp2`, expose via `impl_wasm_conversions_inner!` — one line.

If you find yourself writing `pub fn to_json(&self) -> JsonValue` outside a
trait impl, stop and re-read this doc.

## The canonical traits

```rust
// packages/rs-dpp/src/serialization/serialization_traits.rs
pub trait JsonConvertible: Serialize + DeserializeOwned {
    fn to_json(&self) -> Result<JsonValue, ProtocolError>;
    fn from_json(json: JsonValue) -> Result<Self, ProtocolError>;
}

pub trait ValueConvertible: Serialize + DeserializeOwned {
    fn to_object(&self) -> Result<Value, ProtocolError>;
    fn into_object(self) -> Result<Value, ProtocolError>;
    fn from_object(value: Value) -> Result<Self, ProtocolError>;
}
```

Default implementations route through `serde_json::to_value` /
`platform_value::to_value`. For 95% of types, the derive is sufficient and
no custom impl is needed.

The wasm-dpp2 `impl_wasm_conversions_inner!` macro delegates to these traits,
then handles JS-boundary concerns (`platform_value` ↔ `JsValue`, large-number
stringification, Map-key normalization).

## When to derive vs when to hand-roll

| Type shape | Action |
|---|---|
| Plain struct or enum, all fields serde-derive cleanly | `#[derive(JsonConvertible, ValueConvertible)]` — done. |
| Versioned enum (V0, V1, …) | Derive both, add `#[serde(tag = "$formatVersion")]`. See [tag conventions](#tag-key-conventions). |
| Tagged enum with tuple variants whose inners aren't named structs | Custom `Serialize`/`Deserialize` flattening tuple fields to named JSON keys. See [escape hatches](#escape-hatches). |
| Type whose canonical wire shape needs extra context (`platform_version`, `data_contract`) | Define an inherent context-aware method on a versioned-conversion trait (e.g., `try_from_platform_versioned`). Don't try to fit it into `JsonConvertible`/`ValueConvertible`. |

## Tag-key conventions

When you have `#[serde(tag = "...")]`, pick the discriminator key by this
rule: **`$`-prefix iff the same JSON level carries other `$`-prefixed
fields**. Plain key otherwise.

| Tag key | When | Example types |
|---|---|---|
| `$formatVersion` | Versioned enums (V0/V1/…) — versioned structs always sit next to `$`-fields. | `Identity`, `IdentityPublicKey`, `DataContractConfig`, `Group`, `Validator`, `ValidatorSet`, `AssetLockValue`, `TokenContractInfo`, all 17 leaf transition wrappers, ~40 others. |
| `$baseFormatVersion` | Inner-versioned envelope when the outer also carries `$formatVersion`. | inner-base of certain transition wrappers. |
| `$extendedFormatVersion` | Outer envelope when the inner is already `$formatVersion`-tagged via `serde(flatten)`. | `ExtendedDocument`. |
| `$type` | Discriminator at a level that already has `$`-fields and isn't a version dimension. | `StateTransition`, `Vote`. |
| `$transition` | Inner umbrella inside `BatchedTransition` (where `$type` would collide with `document_type_name`). | `BatchedTransition`. |
| `$action` | Inner action discriminator inside `DocumentTransition` / `TokenTransition` umbrellas (same collision). | `DocumentTransition`, `TokenTransition`. |
| `kind` | Plain key chosen instead of `$type` because the inner content has its own `type` discriminator that would collide. | `GroupActionEvent` (carries inner `TokenEvent` whose internal tag is `type`). |
| `type` | Plain `type` key when the level has no `$`-prefixed neighbors and no inner-tag collision. | `VotePoll`, `ResourceVoteChoice`, `ContestedDocumentVotePollWinnerInfo`. |

If you're unsure, follow the rule, run round-trip tests, and document any
collision-avoidance reasoning inline.

## Test template

Every J or V impl gets a unit test using a **non-default fixture** with a
**round-trip + per-property** assertion. Pure round-trip can pass tautologically
when fields silently drop both ways; per-property catches that.

```rust
#[cfg(all(test, feature = "json-conversion", feature = "value-conversion", feature = "serde-conversion"))]
mod json_convertible_tests {
    use super::*;
    use platform_value::platform_value;
    use serde_json::json;

    fn fixture() -> MyType {
        // Non-default values for every field; identifiers non-zero.
        MyType { /* ... */ }
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // Lock the wire shape — catches silent renames / type narrowing /
        // tag-key drift on top of the round-trip check below.
        assert_eq!(json, json!({
            "$formatVersion": "0",
            // ... fields with explicit expected values
        }));
        let recovered = MyType::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // platform_value preserves typed variants (Value::U16, Value::Identifier);
        // assert against the typed shape, not the JSON-erased form.
        assert_eq!(value, platform_value!({
            "$formatVersion": "0",
            // ...
        }));
        let recovered = MyType::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
```

Tagged enums get an additional **tag-preservation** test asserting that
`V0` doesn't silently round-trip back as `V1`.

## Escape hatches

Use **only** when the derived path can't express the shape. Document why
inline.

### Tuple-variant enums needing internal tagging

Serde's auto-derive can't internal-tag a tuple variant — internal tagging
requires struct variants or newtype-of-named-struct. If the variants are
shape-stable tuples that map cleanly to named JSON keys, write a custom
`Serialize` / `Deserialize` that emits the flat shape.

Reference impls:
- `packages/rs-dpp/src/tokens/token_event.rs` — 11 variants, mapped
  positional fields to named keys (`amount`, `recipient`, `publicNote`, …).
- `packages/rs-dpp/src/voting/vote_choices/resource_vote_choice/mod.rs` —
  `TowardsIdentity(Identifier)` flattened to `{type, identity}`.
- `packages/rs-dpp/src/voting/vote_info_storage/contested_document_vote_poll_winner_info/mod.rs` —
  same pattern.

Bincode `Encode` / `Decode` derives are independent of serde and **stay
untouched** — reshaping serde wire is safe for the consensus binary path.

### Field-level shaping

For specific field shapes, prefer existing serde helpers over custom
visitors:

- `#[serde(with = "crate::serialization::serde_bytes")]` — `[u8; N]`,
  base64 in HR / bytes in binary.
- `#[serde(with = "crate::serialization::serde_bytes::option")]` —
  `Option<[u8; N]>`.
- `#[serde(with = "crate::serialization::serde_bytes_var")]` — `Vec<u8>`,
  base64 in HR / bytes in binary.
- `#[serde(with = "crate::serialization::json::safe_integer::json_safe_u64")]` —
  large `u64` stringified above `2^53` for JS safety.
- `#[serde(with = "crate::withdrawal::pooling_serde")]` — `Pooling` enum
  with lenient string + numeric variant acceptance.
- `#[json_safe_fields]` proc macro — auto-injects the bytes / json_safe_u64
  helpers across a whole struct based on field types.

If you find yourself writing `with = "my_module"` for a primitive serde
shape that smells generic (base64 bytes, lenient enum, large u64), check
this list first or extend an existing module.

### Critical findings to be aware of

The unification plan documents 5 critical findings — all are addressed
in current code, but they shape the testing approach:

- **Critical-1**: `is_human_readable` divergence. Serde's
  `ContentDeserializer` (used by internally-tagged enums) reports HR=true
  even when wrapping a non-HR Content. Bytes deserializers must accept
  **both** string and bytes paths. See `dpp::serialization::serde_bytes`'s
  `AnyShapeVisitor` for the canonical pattern.
- **Critical-2**: `From<JsonValue> for Value` array→bytes silent
  coercion. A JSON array of u8 (length ≥ 10, all ≤ 255) is silently
  treated as `Value::Bytes`. Round-trip tests must include arrays of
  small numbers explicitly.
- **Critical-3**: `ExtendedDocument` had a non-round-trippable manual
  serde (resolved). Outer enum uses
  `#[serde(tag = "$extendedFormatVersion")]` so it can coexist with the
  inner Document's `$formatVersion`.
- **Critical-4**: `DataContract::Serialize` is impure — depends on
  `PlatformVersion::get_current()`. Treat DataContract conversion as
  context-aware, route through `from_value(value, full_validation,
  &platform_version)`.
- **Critical-5**: `to_canonical_object` sorts keys — signature-load
  bearing. Don't change ordering in canonical paths.

## wasm-dpp2 wrapper patterns

When exposing an rs-dpp domain type to JavaScript:

```rust
// Preferred — delegates to canonical traits, handles JS-boundary
// concerns (BigInt, Uint8Array, Map-key stringification, base58/base64).
impl_wasm_conversions_inner!(
    MyTypeWasm,         // the wasm wrapper struct
    MyType,             // the inner rs-dpp domain type with the canonical traits
    MyType,             // the JS class name
    MyTypeObjectJs,     // typed return for toObject() / fromObject() (optional)
    MyTypeJSONJs,       // typed return for toJSON() / fromJSON()    (optional)
);
```

For wasm-only DTOs (e.g., decompositions of `StateTransitionProofResult`
tuple variants into named-field JS classes — `VerifiedBalanceTransfer`,
`VerifiedMasternodeVote`, etc.) where there is no rs-dpp domain type to
delegate to:

```rust
// Wasm-only DTO — uses serde derives directly, not a JsonConvertible
// inner type.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MyDtoWasm { /* ... */ }

impl_wasm_conversions_serde!(MyDtoWasm, MyDto);
```

`_serde!` is **not** a fallback awaiting migration — it's the canonical
path for wasm-only structs. Don't try to invent a sibling rs-dpp type just
to convert a `_serde!` site to `_inner!`.

For context-aware wrappers (`Identity` accepting `platform_version`,
`Document` accepting `data_contract`, etc.), write the methods manually
on the wasm struct and call the rs-dpp conversion methods directly. See
`packages/wasm-dpp2/src/identity/model.rs` for a representative example —
`to_object` / `to_json` / `from_json` use the canonical traits, only
`from_object` is manual because of the `platform_version` arg.

## Things to avoid

- **`pub fn to_json(&self) -> JsonValue` inherent methods on rs-dpp
  types.** If `JsonConvertible::to_json` works, use it. If it doesn't,
  the fix is to make the type derive the trait or hand-roll the trait
  impl, not to add a parallel inherent method that diverges.
- **Re-implementing canonical wire shapes in wasm wrappers.** If the
  rs-dpp type has `JsonConvertible` / `ValueConvertible`, the wasm
  wrapper must delegate through them (via `impl_wasm_conversions_inner!`
  or by calling the trait methods directly). Don't go through
  `serialization::to_object` / `to_json` (the generic-serde wasm helper)
  when the canonical trait is available.
- **Custom Serialize impls on top of structs that already have a derived
  `Serialize`.** Two competing impls confuses readers and breaks if
  someone later changes the field set.
- **Mocking the bytes encoding twice.** Use `dpp::serialization::serde_bytes`
  / `serde_bytes_var` / `serde_bytes::option`. Don't fork another
  `bytes_b64` helper.

## Quick references

- Trait definitions: `packages/rs-dpp/src/serialization/serialization_traits.rs`
- Bytes serde helpers: `packages/rs-dpp/src/serialization/serde_bytes.rs`,
  `serde_bytes_var.rs`
- Macros: `packages/wasm-dpp2/src/serialization/conversions.rs`
  (`impl_wasm_conversions_inner!` and `impl_wasm_conversions_serde!`)
- Tag-convention precedent: `packages/rs-dpp/src/state_transition/mod.rs`
  (StateTransition `$type`),
  `packages/rs-dpp/src/voting/votes/mod.rs` (Vote `$type`),
  `packages/rs-dpp/src/group/action_event.rs` (GroupActionEvent `kind`).
