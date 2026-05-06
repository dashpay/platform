# wasm-dpp2 Cleanup Plan — Post rs-dpp Convention Sweep

**Status**: planning. Source PR is `feat/json-convertible-address-transitions` (#3573); this is the next cleanup PR.
**Scope**: `packages/wasm-dpp2/`.
**Goal**: delegate JSON/Object serialization to rs-dpp's now-canonical `JsonConvertible` / `ValueConvertible` traits everywhere it's safe; identify generic helpers that should move down to rs-dpp; update JS tests to match the rs-dpp wire-shape changes from PR #3573.

## Inventory

### Macro usage (current)

- `impl_wasm_conversions_inner!`: **27 invocations** across 25 files. Already-correct pattern — delegates to rs-dpp `JsonConvertible`/`ValueConvertible`.
- `impl_wasm_conversions_serde!`: **35 invocations** across 20 files. Generic-serde fallback. Some can migrate to `_inner!`, some genuinely can't.

### `_serde!` callers — categorized

**Group A — CAN migrate to `_inner!`** (wraps a rs-dpp domain type that has `JsonConvertible` + `ValueConvertible`, via either derive or manual impl):

| File | Wasm wrapper | rs-dpp inner | Why migratable |
|---|---|---|---|
| `shielded/shield_transition.rs` | `ShieldTransitionWasm` | `ShieldTransition` | derive(JsonConvertible/ValueConvertible) |
| `shielded/unshield_transition.rs` | `UnshieldTransitionWasm` | `UnshieldTransition` | same |
| `shielded/shielded_transfer_transition.rs` | `ShieldedTransferTransitionWasm` | `ShieldedTransferTransition` | same |
| `shielded/shielded_withdrawal_transition.rs` | `ShieldedWithdrawalTransitionWasm` | `ShieldedWithdrawalTransition` | same |
| `shielded/shield_from_asset_lock_transition.rs` | `ShieldFromAssetLockTransitionWasm` | `ShieldFromAssetLockTransition` | same |
| `shielded/orchard_action.rs` | `SerializedOrchardActionWasm` | `SerializedAction` | manual JsonConvertible impl |
| `asset_lock_proof/proof.rs` | `AssetLockProofWasm` | `AssetLockProof` | manual JsonConvertible impl |
| `tokens/contract_info.rs` | `TokenContractInfoWasm` | `TokenContractInfo` | manual JsonConvertible impl |
| `state_transitions/batch/token_payment_info.rs` | `TokenPaymentInfoWasm` | `TokenPaymentInfo` | manual JsonConvertible impl |
| `platform_address/transitions/identity_create_from_addresses_transition.rs` | wrapper | `IdentityCreateFromAddressesTransition` | rs-dpp has trait |
| `platform_address/transitions/identity_credit_transfer_to_addresses_transition.rs` | wrapper | `IdentityCreditTransferToAddressesTransition` | same |
| `platform_address/transitions/identity_top_up_from_addresses_transition.rs` | wrapper | `IdentityTopUpFromAddressesTransition` | same |
| `platform_address/transitions/address_funding_from_asset_lock_transition.rs` | wrapper | `AddressFundingFromAssetLockTransition` | same |
| `platform_address/transitions/address_credit_withdrawal_transition.rs` | wrapper | `AddressCreditWithdrawalTransition` | same |
| `platform_address/transitions/address_funds_transfer_transition.rs` | wrapper | `AddressFundsTransferTransition` | same |

**~15 callers** in this group → migrate to `_inner!`.

**Group B — CANNOT migrate** (wasm-only struct with no rs-dpp domain inner; the `Verified*` types are wasm wrappers around `StateTransitionProofResult` ENUM VARIANTS, not standalone rs-dpp types):

| File | Type | Reason |
|---|---|---|
| `state_transitions/proof_result/voting.rs` | `VerifiedMasternodeVoteWasm`, `VerifiedNextDistributionWasm` | wasm-specific result wrappers; data drawn from `StateTransitionProofResult::*` variants but the wasm struct holds extracted fields directly |
| `state_transitions/proof_result/shielded.rs` | `VerifiedShieldedPoolStateWasm`, `VerifiedAssetLockConsumedWasm` | same |
| `state_transitions/proof_result/identity.rs` | `VerifiedIdentityWasm`, `VerifiedPartialIdentityWasm`, `VerifiedBalanceTransferWasm` | same |
| `state_transitions/proof_result/token.rs` | 8 `Verified*Wasm` types | same |

**~15 callers** in this group → keep `_serde!` (they're wasm-DTOs, not rs-dpp wrappers; generic serde is the right path).

**Group C — verify case-by-case (~5 callers):** the 3-arg form of `impl_wasm_conversions_serde!` that includes JS class type names — check whether the typed JS class can also be passed through `_inner!`'s 4-arg form.

### Manual `Reflect::set` audit

CONVENTIONS.md forbids `Reflect::set` for building wire shapes. Audited `wasm-dpp2/src/`:

- `state_transitions/proof_result/address_funds.rs:80–93`: composes `{identity, addressInfos}` from already-serialized rs-dpp pieces. **Acceptable** — wraps, doesn't reshape.
- `state_transitions/proof_result/helpers.rs`: `js_obj!` macro for building plain JS objects from key-value pairs. **Acceptable** — utility for assembling result types from pre-serialized parts.
- `serialization/conversions.rs:159–161`: `Map` → plain object normalization for JS ergonomics. **Acceptable** — JS-side concern, not wire-shape change.

No violations found. **No removal here.**

## Wire-shape changes from PR #3573 that JS tests may hardcode

The tag-shape sweep in PR #3573 changed wire shapes for these types. Any JS test asserting the old shape will fail when run against the migrated rs-dpp build:

| rs-dpp type | Old wire shape | New wire shape |
|---|---|---|
| `BatchedTransition` | `{type:"document", data:{...}}` adjacent | `{$transition:"document", $action:"create", $formatVersion:"0", ...}` flat |
| `DocumentTransition` / `TokenTransition` | `{type:"create", ...}` plain `type` | `{$action:"create", ...}` `$action` discriminator |
| `StateTransition` | `{type:"batch", ...}` plain `type` | `{$type:"batch", ...}` `$type` discriminator |
| 17 leaf transition wrappers (`DocumentCreateTransition`, `TokenBurnTransition`, ...) | `{V0:{...}}` externally-tagged | `{$formatVersion:"0", ...}` internal-tagged |
| `TokenBaseTransition` / `DocumentBaseTransition` (flattened into leaves) | flat | adds `$baseFormatVersion:"0"` |
| `TokenEvent` | `{type:"mint", data:[5000, "<base58>", "note"]}` | `{type:"mint", amount:5000, recipient:"<base58>", publicNote:"note"}` |
| `Vote` | `{type:"resourceVote", data:{...}}` | `{$type:"resourceVote", $formatVersion:"0", ...}` |
| `VotePoll` | `{type:"contestedDocumentResourceVotePoll", data:{...}}` | `{type:"contestedDocumentResourceVotePoll", contractId:..., ...}` flat |
| `GroupActionEvent` | `{type:"tokenEvent", data:{...}}` | `{kind:"tokenEvent", type:"mint", amount:..., ...}` |
| `ResourceVoteChoice` | `{type:"towardsIdentity", data:"<base58>"}` | `{type:"towardsIdentity", identity:"<base58>"}` |
| `ContestedDocumentVotePollWinnerInfo` | `{type:"wonByIdentity", data:"<base58>"}` | `{type:"wonByIdentity", identity:"<base58>"}` |
| All u64 fields (Credits/TokenAmount/IdentityNonce/Revision/etc.) | bare number | number ≤ MAX_SAFE_INTEGER, otherwise string |
| `[u8; N]` and `Vec<u8>` byte fields (entropy, encrypted notes, etc.) | array of numbers | base64 string in JSON |

**Tests dir:** `packages/wasm-dpp2/tests/unit/` (78 spec files). Need to grep across these for the old shapes after migration.

## Improvements in wasm-dpp2 worth porting BACK to rs-dpp

These are utility patterns useful beyond the wasm boundary. Each is a candidate for a small, separate rs-dpp PR:

1. **Field-aware integer conversion errors** (`utils.rs:315–586`)
   - `try_to_u64` / `try_to_u32` / `try_to_u8` / `try_to_u16` accept BigInt | Number | String with messages like `"'transactionFee' BigInt value is out of u64 range"` (vs. plain `"invalid bigint"`).
   - **rs-dpp benefit:** SDK / CLI consumers parsing JSON often hit string-encoded integers. Centralized error messages with field context would speed up debugging.
   - **Port target:** `packages/rs-dpp/src/serialization/parse_helpers.rs` (new module).

2. **Fixed-size byte array conversion with diagnostics** (`utils.rs:486–512`)
   - `try_to_fixed_bytes::<N>()` with errors showing expected vs. actual length per field name.
   - **Port target:** same module as (1).

3. **Map-key helpers** (`state_transitions/proof_result/helpers.rs:33–80`)
   - `build_address_infos_map(Vec<(addr, opt)>) → JsMap` and `build_nullifier_map(Vec<bytes>) → JsMap`. Pattern (Rust map → wire-friendly map with hex/base58 keys) appears in multiple rs-dpp consumers.
   - **Port target:** `packages/rs-dpp/src/serialization/map_helpers.rs` (new) — Rust-side implementations, plus thin re-export layer in wasm-dpp2.

4. **Self-describing type metadata** (`utils.rs:775–791` — `impl_wasm_type_info!`)
   - Generates `__type` and `__struct` getters for runtime type identification.
   - **Less obvious port:** rs-dpp consumers don't typically need runtime reflection, but the underlying pattern (each domain type knows its own name + version) could become a small `TypeInfo` trait. Lower priority.

## Plan / phases

### Phase 1 — Migrate the 15 `_serde!` callers in Group A to `_inner!`
- Mechanical change: 1 macro name + (in some cases) extra args for the JS class type name.
- Smoke-build the wasm-dpp2 crate.
- Run TS tests. Capture failures (will surface mainly from Phase 2 wire-shape changes).

### Phase 2 — Update JS tests to match new rs-dpp wire shapes
- Grep `tests/unit/` for `"data":` patterns in mock JSON, hardcoded `type: "..."` strings, byte arrays in entropy/encrypted-note fields, plain-number u64 assertions (large values).
- Update mocks per the table above.
- Re-run tests.

### Phase 3 — Verify Group C (the 3-arg / 4-arg `_serde!` callers)
- Check whether the typed JS class names work with `_inner!`. If yes, migrate; if no, document why.

### Phase 4 — Consensus / persistence smoke check
- Round-trip one representative state-transition through `toBytes` / `fromBytes` (bincode path) before and after to confirm no consensus drift. Bincode is independent of serde per CONVENTIONS.md, so this should be a no-op verification.

### Phase 5 — Port back utilities (separate follow-up PR)
- Lift `try_to_u64` / `try_to_u32` / `try_to_fixed_bytes` family to rs-dpp.
- Lift map-key helpers to rs-dpp.
- Update wasm-dpp2 to re-export.

## Risks

- **Test churn**: many of the 78 specs likely hardcode old wire shapes. Expect 10–20 spec files to need updates. Most should be mechanical.
- **External consumers** of wasm-dpp2 NPM package: this is an internal artifact, but if any external SDK depends on the JSON shape, they'll see breaking changes from PR #3573.
- **Group C edge cases**: the 3-arg/4-arg form of `_serde!` includes typed JS class names — `_inner!` may need a 4-arg variant added to support these without losing JS-side type info.

## Out of scope

- The ~46 specs missing `fromObject` round-trip coverage and ~37 missing `fromJSON` (per CONVENTIONS.md migration backlog) — separate follow-up PR.
- StateTransition wrapper toObject/toJSON gap (only has toBytes/toHex/toBase64) — separate follow-up PR.
