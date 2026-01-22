# WASM Refactoring TODOs

This document tracks inconsistencies found in wasm-dpp2 and wasm-sdk that should be addressed for consistency.

---

## A. MACRO USAGE INCONSISTENCIES

| # | Issue | Files | Priority |
|---|-------|-------|----------|
| ~~A1~~ | ~~`ProTxHashWasm` has manual `try_from_options` instead of macro~~ | ~~`core/pro_tx_hash.rs`~~ | ~~Done~~ ✓ |
| ~~A2~~ | ~~Types missing `impl_wasm_type_info!`~~ | ~~`identity/partial_identity.rs`, token types, `voting/`, `block.rs`, etc.~~ | ~~Done~~ ✓ |
| ~~A3~~ | ~~Types have manual `toObject/toJSON` instead of `impl_wasm_conversions!`~~ | ~~`identity/partial_identity.rs` (fixed), `voting/contender.rs` (already has macro), `batched_transition.rs` (special case)~~ | ~~Done~~ ✓ |
| ~~A4~~ | ~~wasm-sdk: State transition results use manual implementations, not macros~~ | ~~`state_transitions/token.rs` (10 types)~~ | ~~Done~~ ✓ |
| A5 | wasm-sdk: Redundant dual implementations (From + macro) | `queries/mod.rs` (ResponseMetadataWasm, ProofInfoWasm) | Low |

---

## B. TRYFROM INCONSISTENCIES

| # | Issue | Files | Priority |
|---|-------|-------|----------|
| ~~B1~~ | ~~Mixed `TryFrom<JsValue>` vs `TryFrom<&JsValue>` - standardized on `&JsValue` having logic~~ | ~~14 enum files~~ | ~~Done~~ ✓ |
| ~~B2~~ | ~~Both variants now follow pattern: `TryFrom<&JsValue>` has logic, `TryFrom<JsValue>` delegates~~ | ~~Same files~~ | ~~Done~~ ✓ |
| B3 | wasm-sdk: No `TryFrom` pattern used at all, uses ad-hoc deserialize functions | `state_transitions/*.rs` | Medium (out of scope) |

---

## C. PARAMETER TYPE INCONSISTENCIES

| # | Issue | Files | Priority |
|---|-------|-------|----------|
| C1 | ~~Raw `JsValue` used where `PlatformVersionLikeJs` should be~~ | ~~`data_contract/model.rs:653`~~ | ~~Fixed~~ ✓ |
| C2 | Methods returning `JsValue` instead of typed result | `document/model.rs` (properties), transition files | Medium |
| C3 | wasm-sdk: Manual JsValue deserialization instead of typed alternatives | `state_transitions/token.rs` (10+ deserialize functions) | Medium |

---

## D. RETURN TYPE INCONSISTENCIES

| # | Issue | Files | Priority |
|---|-------|-------|----------|
| D1 | Functions returning bare `JsValue` without `Result` wrapper | `batched_transition.rs`, `token_transition.rs`, `voting/resource_vote_choice.rs`, `voting/contender.rs` | Medium |
| D2 | Mixed typed return (`DocumentObjectJs`) vs generic (`JsValue`) | Various files | Low |

---

## E. NAMING CONVENTIONS

| # | Issue | Files | Priority |
|---|-------|-------|----------|
| E1 | Enum naming: mix of ALL_CAPS vs PascalCase | `version.rs` (ALL_CAPS), `network.rs` (PascalCase), `enums/keys/` (ALL_CAPS) | Low |
| E2 | Inconsistent `js_name` attribute formatting (quoted vs unquoted, spacing) | Various files | Low |
| E3 | wasm-sdk: Missing `Wasm` suffix on exposed types | `Dip14ExtendedPrivKey`, `HDKeyInfo`, `RegisterDpnsNameResult`, `DpnsUsernameInfo` | Medium |
| E4 | wasm-sdk: Inconsistent getter attribute patterns | `getter_with_clone` vs `getter` + manual clone | Low |

---

## F. SERIALIZATION INCONSISTENCIES

| # | Issue | Files | Priority |
|---|-------|-------|----------|
| F1 | Mixed serialization approaches (custom impl vs macro-generated) | `document/model.rs`, `identity/transitions/pooling.rs` | Low |
| F2 | Not all `Serialize` implementations check `is_human_readable()` | Various files | Medium |

---

## G. STRUCTURAL INCONSISTENCIES

| # | Issue | Files | Priority |
|---|-------|-------|----------|
| G1 | Mix of newtype wrappers `Foo(Inner)` vs structs with fields | `document/model.rs`, `identity/signer.rs`, `platform_address/input_output.rs` | Low |
| ~~G2~~ | ~~wasm-sdk: `system.rs` pattern (macro-based) not followed in state transitions~~ | ~~`state_transitions/token.rs`~~ | ~~Done~~ ✓ |
| G3 | Enum `TryFrom` implementations have duplicate code pattern (could use macro) | All enum files in `enums/` | Low |

---

## H. IMPORT INCONSISTENCIES

| # | Issue | Files | Priority |
|---|-------|-------|----------|
| H1 | No consistent import ordering/grouping | All files | Low |
| H2 | wasm-sdk: Blanket `pub use wasm_dpp2::*` shadows individual imports | `lib.rs` | Low |

---

## Recommended Priority Order

### High Priority (should fix):
1. ~~**A1**: Add `impl_try_from_options!` to `ProTxHashWasm`~~ ✓
2. ~~**B1/B2**: Standardize `TryFrom` pattern - `&JsValue` has logic, `JsValue` delegates~~ ✓
3. ~~**A4/G2**: Refactor wasm-sdk state transitions to follow `system.rs` macro pattern~~ ✓

### Medium Priority (should address):
4. ~~**A2**~~/~~**A3**~~: Add missing macros to types without them ✓
5. **E3**: Add `Wasm` suffix to wasm-sdk exposed types
6. **C2/D1**: Use typed returns and wrap in `Result`
7. **F2**: Ensure `is_human_readable()` checks in Serialize impls

### Low Priority (nice to have):
8. **E1/E2**: Standardize naming conventions
9. **G3**: Create macro for enum `TryFrom` implementations
10. **H1**: Document and enforce import ordering

---

## Completed Items

- [x] C1: Fixed raw `JsValue` in `data_contract/document/model.rs` - now uses `PlatformVersionLikeJs`
- [x] Updated `impl_try_from_options!` macro to use `TryFrom<&JsValue>`
- [x] Added `impl_try_from_js_value!` macro for types that only accept WASM objects
- [x] Replaced manual `try_from_options` in `NetworkWasm`, `IdentifierWasm`, `PrivateKeyWasm`, `GroupStateTransitionInfoStatusWasm`
- [x] A1: Replaced manual `try_from_options` in `ProTxHashWasm` with macro
- [x] A2: Added `impl_wasm_type_info!` to types missing it:
  - `PartialIdentityWasm`
  - `ProTxHashWasm`
  - `ContenderWithSerializedDocumentWasm`
  - `ContestedDocumentVotePollWinnerInfoWasm`
  - `BlockInfoWasm`
  - `ConsensusErrorWasm`
  - `IdentityTokenInfoWasm`
  - `TokenStatusWasm`
  - `TokenContractInfoWasm`
- [x] A3: Evaluated types for `impl_wasm_conversions!` macro:
  - `PartialIdentityWasm` - **cannot use macro** (serde can't serialize `BTreeMap<KeyID, _>` - integer keys cause "Map key is not a string" error; also `id` serializes as bytes not `IdentifierWasm`). Added unit tests to ensure manual implementation works correctly. Fixed bug in `value_to_loaded_public_keys` where Object.keys() returns strings but code expected numbers.
  - `ContenderWithSerializedDocumentWasm` - already had macro
  - `BatchedTransitionWasm` - special case (has `toTransition()` not serialization)
- [x] A4/G2: Refactored wasm-sdk token result types to use macros:
  - `TokenMintResultWasm` - uses `#[derive(Clone, Serialize, Deserialize)]`, `#[serde(rename_all = "camelCase")]`, `#[wasm_bindgen(getter_with_clone)]`, and `impl_wasm_serde_conversions!`
  - `TokenBurnResultWasm` - same pattern
  - `TokenTransferResultWasm` - same pattern
  - `TokenFreezeResultWasm` - same pattern
  - `TokenUnfreezeResultWasm` - same pattern
  - `TokenDestroyFrozenResultWasm` - same pattern
  - `TokenEmergencyActionResultWasm` - same pattern
  - `TokenClaimResultWasm` - same pattern
  - `TokenSetPriceResultWasm` - same pattern (with `#[serde(skip)]` for `TokenPricingScheduleWasm` field)
  - `TokenDirectPurchaseResultWasm` - same pattern (with manual getter for BigInt conversion)
  - Note: All types use `#[serde(skip)]` for `DocumentWasm` fields and manual getters for `u64` → `BigInt` conversions
