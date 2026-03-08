# dpp-json-convertible-derive

Proc macros for compile-safe JSON serialization of `u64`/`i64` values in Dash Platform Protocol types.

## Problem

JavaScript's `Number` type uses IEEE 754 double-precision floats, which can only safely represent integers up to `2^53 - 1` (`Number.MAX_SAFE_INTEGER = 9007199254740991`). Many DPP types use `u64` for credits, timestamps, block heights, and token amounts — values that routinely exceed this limit. When serialized to JSON without protection, these values silently lose precision in JavaScript clients.

## Solution

This crate provides three proc macros that work together to guarantee at compile time that every `u64`/`i64` field in annotated types is protected with a `#[serde(with = "...")]` annotation that stringifies large values in JSON while keeping native integers in non-JSON formats (platform_value, bincode).

### How It Works

The system uses `serializer.is_human_readable()` to branch behavior:
- **JSON** (human-readable): values > `MAX_SAFE_INTEGER` are serialized as strings
- **platform_value / bincode** (non-human-readable): native `u64`/`i64`, no change

### The Three Macros

#### 1. `#[json_safe_fields]` — Attribute macro for V0 structs and enums

```rust
#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(Serialize, Deserialize)]
pub struct TokenConfigurationV0 {
    pub max_supply: Option<TokenAmount>,  // auto-annotated
    pub name: String,                     // left alone (not u64)
    pub config: DistributionFunction,     // compile-time check
}
```

This macro:
1. **Scans fields** for `u64`, `i64`, `Option<u64>`, `Option<i64>`, and known type aliases (e.g., `Credits`, `TimestampMillis`)
2. **Adds `#[serde(with = "json_safe_u64")]`** (or the appropriate variant) to matching fields
3. **Implements `JsonSafeFields`** marker trait for the type
4. **Generates compile-time assertions** that all other field types also implement `JsonSafeFields`

For `Option` fields, it also adds `#[serde(default)]` to preserve serde's "missing field = None" behavior, which `serde(with)` would otherwise override.

#### 2. `#[derive(JsonConvertible)]` — Derive macro for versioned enums

```rust
#[cfg_attr(feature = "json-conversion", derive(JsonConvertible))]
#[serde(tag = "$formatVersion")]
pub enum TokenConfiguration {
    #[serde(rename = "0")]
    V0(TokenConfigurationV0),
}
```

This macro:
1. **Implements `JsonConvertible`** (provides `to_json()` / `from_json()`)
2. **Implements `JsonSafeFields`** for the enum
3. **Asserts at compile time** that all inner variant types implement `JsonSafeFields`

If `TokenConfigurationV0` doesn't have `#[json_safe_fields]`, compilation fails.

#### 3. `#[derive(ValueConvertible)]` — Derive macro for platform_value conversion

```rust
#[derive(ValueConvertible)]
pub enum TokenConfiguration {
    V0(TokenConfigurationV0),
}
```

Implements `ValueConvertible` (provides `to_object()` / `from_object()`). No safety checks needed since platform_value handles u64 natively.

## Compile-Time Safety Guarantees

The `JsonSafeFields` marker trait forms a recursive proof chain:

1. **Primitives** (bool, u8, u16, u32, String, etc.) implement `JsonSafeFields`
2. **u64 and i64 do NOT** implement `JsonSafeFields` — they are not inherently safe
3. **Collections** (`Vec<T>`, `BTreeMap<K, V>`, etc.) implement `JsonSafeFields` only if their type parameters do
4. **Annotated structs** get `JsonSafeFields` from `#[json_safe_fields]`
5. **Versioned enums** get `JsonSafeFields` from `#[derive(JsonConvertible)]`

This means:

| Field type | What happens |
|---|---|
| `u64`, `Credits`, `TokenAmount` | Auto-annotated with `serde(with)` |
| `String`, `bool`, `Identifier` | Passes `JsonSafeFields` check |
| `Vec<String>` | Passes (String: JsonSafeFields) |
| `Vec<u64>` | **Compile error** — u64 doesn't implement JsonSafeFields |
| `BTreeMap<K, u64>` | **Compile error** — needs manual `serde(with)` |
| `type Foo = u64` (unknown alias) | **Compile error** — macro doesn't recognize the alias, `JsonSafeFields` assertion fails |
| Custom struct without `#[json_safe_fields]` | **Compile error** |
| Field with `#[serde(with = "...")]` | Skipped — both auto-annotation and `JsonSafeFields` assertion are skipped; developer takes full responsibility |
| Field with `#[serde(skip)]` / `skip_serializing` / `skip_deserializing` | Skipped — not serialized |
| Field with `#[serde(flatten)]` | Skipped — special serde handling, not checked |

## Maintenance Guide

### Adding a new `type X = u64` alias

Add it to `U64_ALIASES` in `src/lib.rs`. If you forget, any struct using the alias will fail to compile with a `JsonSafeFields` error.

### Adding a new V0 struct with u64 fields

1. Add `#[cfg_attr(feature = "json-conversion", json_safe_fields)]` before the struct
2. Add `#[cfg(feature = "json-conversion")] use crate::serialization::json_safe_fields;` import
3. The macro handles everything else automatically

### Adding a new versioned enum

Add `#[cfg_attr(feature = "json-conversion", derive(JsonConvertible))]`. The derive will verify that all inner V0 types implement `JsonSafeFields`.

### Adding a `BTreeMap<K, u64>` field

These need manual `#[serde(with = "...")]` because the macro can't auto-annotate container internals. Use one of the modules from `serialization::json::safe_integer_map`:
- `json_safe_u64_u64_map` — for `BTreeMap<u64, u64>`
- `json_safe_identifier_u64_map` — for `BTreeMap<Identifier, u64>`
- `json_safe_generic_u64_value_map` — for `BTreeMap<AnyKey, u64>` (works with any serializable key)
- `json_safe_u64_nested_identifier_u64_map` — for `BTreeMap<u64, BTreeMap<Identifier, u64>>`

For `Vec<u64>` fields, there is no ready-made module — write a custom `serde(with)` module following the same pattern, or restructure the data.

Gate the annotation with a feature that enables both serde and the json module, e.g.: `#[cfg_attr(feature = "state-transition-json-conversion", serde(with = "..."))]`

### Adding a simple enum/struct used as a field

If the type doesn't contain any u64/i64 fields, add `impl JsonSafeFields for MyType {}` in `serialization/json/safe_fields.rs`.

### Feature gating

All json-safe machinery is behind the `json-conversion` feature:
- `#[json_safe_fields]` is always gated with `cfg_attr(feature = "json-conversion", ...)`
- The `json` module in `serialization/` is gated with `#[cfg(feature = "json-conversion")]`
- Imports of `json_safe_fields` are gated with `#[cfg(feature = "json-conversion")]`

### Why not a wrapper serializer?

We considered wrapping the serde `Serializer` to intercept `serialize_u64` globally. This won't work because serde's tagged enum processing (`#[serde(tag)]`, adjacently-tagged) buffers field values into an intermediate `Content` representation and re-serializes them, bypassing the custom serializer's `serialize_u64` method. The per-field `#[serde(with)]` approach intercepts at the field level before serde's enum machinery, so it always works.

## Files

| File | Purpose |
|---|---|
| `rs-dpp-json-convertible-derive/src/lib.rs` | Proc macros: `json_safe_fields`, `JsonConvertible`, `ValueConvertible` |
| `rs-dpp/src/serialization/json/safe_integer.rs` | Serde `with` modules: `json_safe_u64`, `json_safe_i64`, `json_safe_option_u64`, `json_safe_option_i64` |
| `rs-dpp/src/serialization/json/safe_integer_map.rs` | Serde `with` modules for maps: `json_safe_u64_u64_map`, `json_safe_identifier_u64_map`, etc. |
| `rs-dpp/src/serialization/json/safe_fields.rs` | `JsonSafeFields` trait + impls for primitives, collections, and external types |
| `rs-dpp/src/serialization/serialization_traits.rs` | `JsonConvertible` and `ValueConvertible` trait definitions |
