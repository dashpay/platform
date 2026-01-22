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
| ~~B3~~ | ~~wasm-sdk: Uses serde-based deserialization for complex options objects~~ | ~~`state_transitions/*.rs`~~ | ~~N/A~~ (correct pattern for structs) |

---

## C. PARAMETER TYPE INCONSISTENCIES

| # | Issue | Files | Priority |
|---|-------|-------|----------|
| ~~C1~~ | ~~Raw `JsValue` used where `PlatformVersionLikeJs` should be~~ | ~~`data_contract/model.rs:653`~~ | ~~Fixed~~ ✓ |
| ~~C2~~ | ~~Methods returning `JsValue` - fixed optional returns, union types acceptable~~ | ~~3 transition files fixed~~ | ~~Partial~~ ✓ |
| ~~C3~~ | ~~wasm-sdk: Uses serde-based deserialization (correct pattern for complex objects)~~ | ~~`state_transitions/*.rs`~~ | ~~N/A~~ |

---

## D. RETURN TYPE INCONSISTENCIES

| # | Issue | Files | Priority |
|---|-------|-------|----------|
| ~~D1~~ | ~~Bare `JsValue` returns - added `unchecked_return_type` for unions~~ | ~~5 files~~ | ~~Done~~ ✓ |
| ~~D2~~ | ~~Mixed typed return (`DocumentObjectJs`) vs generic (`JsValue`)~~ | ~~Various files~~ | ~~Done~~ ✓ |

**Note on `unchecked_return_type`**: The wasm-sdk uses `unchecked_return_type` for TypeScript generics like `ProofMetadataResponseTyped<T>`, `Map<K, V>`, and `Array<T>`. This is the correct pattern when:

- TypeScript generics don't have Rust equivalents
- JavaScript built-in types need specific type parameters
For concrete WASM struct returns (like `DpnsUsernameInfoWasm`), we return the typed Rust struct directly.

---

## E. NAMING CONVENTIONS

| # | Issue | Files | Priority |
|---|-------|-------|----------|
| E1 | Enum naming: mix of ALL_CAPS vs PascalCase | `version.rs` (ALL_CAPS), `network.rs` (PascalCase), `enums/keys/` (ALL_CAPS) | Low |
| E2 | Inconsistent `js_name` attribute formatting (quoted vs unquoted, spacing) | Various files | Low |
| ~~E3~~ | ~~wasm-sdk: Missing `Wasm` suffix on exposed types~~ | ~~`RegisterDpnsNameResult`, `DpnsUsernameInfo`~~ | ~~Done~~ ✓ |
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

### High Priority (should fix)

1. ~~**A1**: Add `impl_try_from_options!` to `ProTxHashWasm`~~ ✓
2. ~~**B1/B2**: Standardize `TryFrom` pattern - `&JsValue` has logic, `JsValue` delegates~~ ✓
3. ~~**A4/G2**: Refactor wasm-sdk state transitions to follow `system.rs` macro pattern~~ ✓

### Medium Priority (should address)

1. ~~**A2**~~/~~**A3**~~: Add missing macros to types without them ✓
2. ~~**E3**~~: Add `Wasm` suffix to wasm-sdk exposed types ✓
3. ~~**C2**~~: Use typed returns (optional returns fixed, union types N/A) ✓ / **D1**: Wrap in `Result`
4. **F2**: Ensure `is_human_readable()` checks in Serialize impls

### Low Priority (nice to have)

1. **E1/E2**: Standardize naming conventions
2. **G3**: Create macro for enum `TryFrom` implementations
3. **H1**: Document and enforce import ordering

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
- [x] B1/B2: Standardized `TryFrom` pattern across 14 enum files:
  - Pattern: `TryFrom<&JsValue>` contains conversion logic, `TryFrom<JsValue>` delegates via `Self::try_from(&value)` (avoids clone)
  - Files updated:
    - `enums/keys/security_level.rs`
    - `enums/keys/key_type.rs`
    - `enums/keys/purpose.rs`
    - `enums/token/emergency_action.rs`
    - `enums/token/distribution_type.rs`
    - `enums/token/action_goal.rs`
    - `enums/contested/vote_state_result_type.rs`
    - `enums/batch/batch_enum.rs`
    - `enums/batch/gas_fees_paid_by.rs`
    - `enums/lock_types.rs`
    - `identity/transitions/pooling.rs`
    - `identity/transitions/public_key_in_creation.rs`
    - `asset_lock_proof/outpoint.rs`
    - `version.rs`
- [x] B3: Reviewed - **N/A (different use case)**:
  - wasm-sdk uses serde-based deserialization (`serde_wasm_bindgen::from_value`) for complex options objects
  - This is the **correct pattern** for structs with multiple fields (vs `TryFrom` for simple enums)
  - 19 `deserialize_*` functions are thin wrappers providing context-specific error messages
  - No changes needed - already using idiomatic approach
- [x] C2: Fixed methods returning `JsValue` where typed alternatives exist:
  - Changed `optional_asset_lock_proof` to return `Option<AssetLockProofWasm>`:
    - `identity/transitions/credit_withdrawal_transition.rs`
    - `identity/transitions/top_up_transition.rs`
    - `identity/transitions/update_transition.rs`
  - Changed `resource_vote_choice.value()` to return `Option<IdentifierWasm>`
  - Changed `contender.serialized_document()` to return `Option<Uint8Array>`
  - Added TypeScript union types for documentation:
    - `TokenConfigurationChangeItemValue` in `tokens/configuration_change_item/token_configuration_change_item.rs`
    - `RewardDistributionValue` in `tokens/configuration/reward_distribution_type.rs`
  - Note: Remaining `-> JsValue` returns are for union types that already have TypeScript definitions (e.g., `BatchedTransitionLike`, `TokenTransitionLike`)
- [x] C3: Reviewed - **N/A (same as B3)** - serde is correct pattern for complex options objects
- [x] D1: Added extern types for union type returns with `From` implementations:
  - `batched_transition.rs:to_transition()` → `BatchedTransitionLikeJs` (via `From<DocumentTransitionWasm>`, `From<TokenTransitionWasm>`)
  - `token_transition.rs:transition()` → `TokenTransitionLikeJs` (via macro for all 11 token transition types)
  - `action_taker.rs:get_value()` → `ActionTakerValueJs` (via `From<IdentifierWasm>`, `From<Array>`)
  - `reward_distribution_type.rs:distribution()` → `RewardDistributionValueJs` (via `From` for 3 distribution types)
  - `token_configuration_change_item.rs:item()` → `TokenConfigurationChangeItemValueJs` (uses `unchecked_into()` for heterogeneous primitives)
- [x] E3: Added `Wasm` suffix to wasm-sdk exposed WASM types:
  - `RegisterDpnsNameResult` → `RegisterDpnsNameResultWasm`
  - `DpnsUsernameInfo` → `DpnsUsernameInfoWasm`
  - Note: `Dip14ExtendedPrivKey` and `HDKeyInfo` were NOT exposed to WASM (no `#[wasm_bindgen]`) so they correctly do NOT have the `Wasm` suffix - they are internal Rust types
- [x] D2: Replaced generic `JsValue` returns with typed extern types:
  - `state_transition.rs`: `toBytes()` → `Vec<u8>` (becomes `Uint8Array`), `toHex()`/`toBase64()` → `String`
  - `document/model.rs`: Added `DocumentPropertiesJs` for `properties()` return
  - `data_contract/model.rs`: Added `DataContractSchemasJs`, `DataContractGroupsJs` for typed returns
  - `create.rs`, `replace.rs`: Added `DocumentTransitionDataJs` for `get_data()` return
  - `token_pricing_schedule.rs`: Added `TokenPricingScheduleValueJs` for `value()` return
  - `configuration_convention.rs`: Added `TokenConfigurationLocalizationsJs` for `localizations()` return
  - Replaced all `unchecked_into()` calls with `.into()` in `serialization/conversions.rs`
  - All `impl_wasm_conversions!` macro usages now use 4-argument form (typed returns)
