# JSON / Value Conversion Inventory

Consolidated inventory for the rs-dpp `JsonConvertible` / `ValueConvertible` unification effort.
Source traits: `packages/rs-dpp/src/serialization/serialization_traits.rs:141-185`.

**Convention** — by project pattern, trait impls live on the **versioned outer enum** (e.g. `Identity`, `BlockInfo`), *not* on inner `V0/V1` structs. V0/V1 inner structs are intentionally excluded from the candidate list.

**Macro flavors** (`packages/wasm-dpp2/src/serialization/conversions.rs`):
- `impl_wasm_conversions_inner!` — preferred path; assumes the inner rs-dpp type already implements the traits.
- `impl_wasm_conversions_serde!` — fallback path; goes through serde directly.

This file is generated from a 4-agent parallel inventory. A 5th verification agent will cross-check it; corrections land back here as a follow-up.

> **Pass-1 status (2026-04-30, commit `9f23d675af`)**: ~80 of the types catalogued below now have canonical impls. The Section 1 (already-covered) and Section 5 (missing-impls) tables are *historical* — refer to `docs/json-value-unification-plan.md` §7 Phase B and Phase C for current coverage. Pass 2 (tests + bug fixes) is in progress.

---

## Section 1 — rs-dpp types **with trait impls**

Total: **58 types** (50 with both, 7 JsonConvertible-only, 1 ValueConvertible-only).

### Identity

| Type | Kind | Versioned? | Json | Value | File:line |
|---|---|---|:---:|:---:|---|
| `Identity` | enum | V0 | ❌ | ✅ | `src/identity/identity.rs:43` |
| `IdentityV0` | struct | V0 | ❌ | ✅ | `src/identity/v0/mod.rs:36` |
| `IdentityPublicKey` | enum | — | ❌ | ✅ | `src/identity/identity_public_key/mod.rs:55` |
| `ContractBoundSpecification` | enum | — | ✅ | ✅ | `src/identity/identity_public_key/contract_bounds/mod.rs:~35` |

### Asset Lock Proof

| Type | Kind | Versioned? | Json | Value | File:line |
|---|---|---|:---:|:---:|---|
| `InstantAssetLockProof` | struct | — | ✅ | ✅ | `src/identity/state_transition/asset_lock_proof/instant/instant_asset_lock_proof.rs:~25` |
| `ChainAssetLockProof` | struct | — | ✅ | ✅ | `src/identity/state_transition/asset_lock_proof/chain/chain_asset_lock_proof.rs:~25` |

### Data Contract

| Type | Kind | Versioned? | Json | Value | File:line |
|---|---|---|:---:|:---:|---|
| `DataContractConfig` | enum | V0/V1 | ✅ | ❌ | `src/data_contract/config/mod.rs:22` |
| `Group` | enum | V0 | ✅ | ✅ | `src/data_contract/group/mod.rs:~45` |
| `TokenKeepsHistoryRules` | enum | V0 | ✅ | ❌ | `src/data_contract/associated_token/token_keeps_history_rules/mod.rs:~30` |
| `TokenConfigurationConvention` | enum | V0 | ✅ | ✅ | `src/data_contract/associated_token/token_configuration_convention/mod.rs:~40` |
| `TokenPreProgrammedDistribution` | enum | V0 | ✅ | ❌ | `src/data_contract/associated_token/token_pre_programmed_distribution/mod.rs:~30` |
| `TokenPerpetualDistribution` | enum | V0 | ✅ | ❌ | `src/data_contract/associated_token/token_perpetual_distribution/mod.rs:~35` |
| `TokenConfigurationLocalization` | enum | V0 | ✅ | ✅ | `src/data_contract/associated_token/token_configuration_localization/mod.rs:~40` |
| `TokenMarketplaceRules` | enum | V0 | ✅ | ❌ | `src/data_contract/associated_token/token_marketplace_rules/mod.rs:~30` |
| `TokenConfiguration` | enum | V0 | ✅ | ✅ | `src/data_contract/associated_token/token_configuration/mod.rs:~45` |
| `TokenDistributionRules` | enum | V0 | ✅ | ❌ | `src/data_contract/associated_token/token_distribution_rules/mod.rs:~30` |
| `DataContractInSerializationFormat` | enum | — | ✅ | ❌ | `src/data_contract/serialized_version/mod.rs:106` |
| `ChangeControlRules` | enum | V0 | ✅ | ✅ | `src/data_contract/change_control_rules/mod.rs:~25` |

### State Transitions

| Type | Kind | Versioned? | Json | Value | File:line |
|---|---|---|:---:|:---:|---|
| `DataContractCreateTransition` | enum | V0 | ✅ | ❌ | `src/state_transition/state_transitions/contract/data_contract_create_transition/mod.rs:40` |
| `DataContractUpdateTransition` | enum | V0 | ✅ | ❌ | `src/state_transition/state_transitions/contract/data_contract_update_transition/mod.rs:~40` |
| `IdentityCreateTransition` | enum | V0 | ✅ | ❌ | `src/state_transition/state_transitions/identity/identity_create_transition/mod.rs:34` |
| `IdentityUpdateTransition` | enum | V0 | ✅ | ❌ | `src/state_transition/state_transitions/identity/identity_update_transition/mod.rs:35` |
| `IdentityTopUpTransition` | enum | V0 | ✅ | ❌ | `src/state_transition/state_transitions/identity/identity_topup_transition/mod.rs:~40` |
| `IdentityTopUpFromAddressesTransition` | enum | V0 | ❌ | ✅ | `…/identity_topup_from_addresses_transition/mod.rs:49` |
| `IdentityCreditWithdrawalTransition` | enum | V0 | ✅ | ❌ | `…/identity_credit_withdrawal_transition/mod.rs:~40` |
| `IdentityCreditTransferTransition` | enum | V0 | ✅ | ❌ | `…/identity_credit_transfer_transition/mod.rs:~40` |
| `IdentityCreditTransferToAddressesTransition` | enum | V0 | ❌ | ✅ | `…/identity_credit_transfer_to_addresses_transition/mod.rs:53` |
| `IdentityPublicKeyInCreation` | enum | V0 | ✅ | ❌ | `…/public_key_in_creation/mod.rs:~30` |
| `MasternodeVoteTransition` | enum | V0 | ✅ | ❌ | `…/masternode_vote_transition/mod.rs:~40` |
| `IdentityCreateFromAddressesTransition` | enum | V0 | ❌ | ✅ | `…/identity_create_from_addresses_transition/mod.rs:51` |
| `AddressFundingFromAssetLockTransition` | enum | V0 | ❌ | ✅ | `…/address_funds/address_funding_from_asset_lock_transition/mod.rs:51` |
| `AddressFundsTransferTransition` | enum | V0 | ❌ | ✅ | `…/address_funds/address_funds_transfer_transition/mod.rs:51` |

> ✅ **Discrepancy resolved**: the 5 address-related transitions above have `ValueConvertible` *only*. They need `JsonConvertible` added before their WASM wrappers can move to `_inner!`.

### Voting

| Type | Kind | Versioned? | Json | Value | File:line |
|---|---|---|:---:|:---:|---|
| `ContenderWithSerializedDocument` | enum | V0 | ✅ | ✅ | `src/voting/contender_structs/contender/mod.rs:35` |
| `ResourceVote` | enum | V0 | ✅ | ✅ | `src/voting/votes/resource_vote/mod.rs:~30` |
| `Vote` | enum | flat | ✅ | ✅ | `src/voting/votes/mod.rs:25,31` |
| `VotePoll` | enum | flat | ✅ | ✅ | `src/voting/vote_polls/mod.rs:17` |
| `ContestedDocumentResourceVotePoll` | struct | — | ✅ | ✅ | `src/voting/vote_polls/contested_document_resource_vote_poll/mod.rs:20,28` |
| `ContestedDocumentVotePollWinnerInfo` | enum | flat | ✅ | ✅ | `src/voting/vote_info_storage/contested_document_vote_poll_winner_info/mod.rs:~25` |
| `ResourceVoteChoice` | enum | flat | ✅ | ✅ | `src/voting/vote_choices/resource_vote_choice/mod.rs:~40` |

> Note: `Contender` (no serde derives) is **not** in this section — moved to §5b.

### Tokens

| Type | Kind | Versioned? | Json | Value | File:line |
|---|---|---|:---:|:---:|---|
| `TokenStatus` | enum | V0 | ✅ | ✅ | `src/tokens/status/mod.rs:16` |
| `IdentityTokenInfo` | enum | V0 | ✅ | ✅ | `src/tokens/info/mod.rs:19` |
| `TokenEvent` | enum | flat | ✅ | ✅ | `src/tokens/token_event.rs:~160` |

### Group

| Type | Kind | Versioned? | Json | Value | File:line |
|---|---|---|:---:|:---:|---|
| `GroupActionEvent` | enum | flat | ✅ | ✅ | `src/group/action_event.rs:29` |
| `GroupAction` | enum | V0 | ✅ | ✅ | `src/group/group_action/mod.rs:~20` |

### Block / Epoch

| Type | Kind | Versioned? | Json | Value | File:line |
|---|---|---|:---:|:---:|---|
| `BlockInfo` | struct | — | ✅ | ✅ | `src/block/block_info/mod.rs:29` |
| `ExtendedBlockInfo` | enum | V0 | ✅ | ✅ | `src/block/extended_block_info/mod.rs:19` |
| `ExtendedEpochInfo` | enum | V0 | ✅ | ✅ | `src/block/extended_epoch_info/mod.rs:~30` |
| `FinalizedEpochInfo` | enum | V0 | ✅ | ✅ | `src/block/finalized_epoch_info/mod.rs:~30` |

---

## Section 2 — rs-dpp types with **trait impls but no rs-dpp round-trip tests**

Verification (Agent E) confirmed Agent D's "5 types" estimate was a dramatic undercount. The full list of rs-dpp types whose trait impls are derive-only with no direct rs-dpp `#[test]` round-trip:

**Identity / state transitions**: `IdentityCreditTransferTransition`, `IdentityCreditWithdrawalTransition`, `IdentityTopUpTransition`, `IdentityUpdateTransition`, `MasternodeVoteTransition`, `IdentityPublicKeyInCreation`, `DataContractCreateTransition` *(only `to_json` direction)*, `DataContractUpdateTransition`, `IdentityCreateTransition`.

**Block / epoch**: `BlockInfo`, `ExtendedBlockInfo`, `ExtendedEpochInfo`, `FinalizedEpochInfo`.

**Asset lock proofs**: `InstantAssetLockProof`, `ChainAssetLockProof`.

**Voting**: `Vote`, `VotePoll`, `ResourceVote`, `ContenderWithSerializedDocument`, `ContestedDocumentResourceVotePoll`, `ContestedDocumentVotePollWinnerInfo`, `ResourceVoteChoice` *(some inline coverage)*.

**Tokens**: `TokenEvent`, `IdentityTokenInfo`, `TokenStatus`.

**Group**: `GroupAction`, `GroupActionEvent`.

**Data contract**: `DataContractInSerializationFormat`, `DataContractConfig`, `Group`, `TokenConfiguration`, `TokenConfigurationConvention`, `TokenConfigurationLocalization`, `TokenKeepsHistoryRules`, `TokenMarketplaceRules`, `TokenDistributionRules`, `TokenPerpetualDistribution`, `TokenPreProgrammedDistribution`, `ChangeControlRules`, `ContractBoundSpecification`.

Total: **~35 types** lack rs-dpp-side round-trip tests but have impls. Indirect coverage from wasm-dpp2 spec files exists for ~20 of them; the rest are unproven.

**Tagged-enum round-trip tests** (verify variant tag preservation across `to_json`→`from_json`) exist for: `IdentityCreateTransition::V0`, `DataContractCreateTransition::V0`, `DataContract` (V0/V1 dispatch). The Identity wasm test notes a "tagged enum serde limitation" worth re-reading.

---

## Section 3 — WASM wrappers using `impl_wasm_conversions_inner!` (already on traits)

Total: **24 wrappers**. These are healthy.

| WASM Wrapper | Inner Type | File:line |
|---|---|---|
| `BlockInfoWasm` | `BlockInfo` | `block.rs:126` |
| `ContractBoundsWasm` | `ContractBounds` | `data_contract/contract_bounds.rs:160` |
| `DataContractCreateTransitionWasm` | `DataContractCreateTransition` | `data_contract/transitions/create.rs:216` |
| `DataContractUpdateTransitionWasm` | `DataContractUpdateTransition` | `data_contract/transitions/update.rs:203` |
| `MasternodeVoteTransitionWasm` | `MasternodeVoteTransition` | `identity/transitions/masternode_vote_transition.rs:302` |
| `IdentityCreditWithdrawalTransitionWasm` | `IdentityCreditWithdrawalTransition` | `identity/transitions/credit_withdrawal_transition.rs:359` |
| `FinalizedEpochInfoWasm` | `FinalizedEpochInfo` | `epoch/finalized_epoch_info.rs:376` |
| `IdentityTopUpTransitionWasm` | `IdentityTopUpTransition` | `identity/transitions/top_up_transition.rs:242` |
| `IdentityPublicKeyInCreationWasm` | `IdentityPublicKeyInCreation` | `identity/transitions/public_key_in_creation.rs:293` |
| `IdentityUpdateTransitionWasm` | `IdentityUpdateTransition` | `identity/transitions/update_transition.rs:348` |
| `ExtendedEpochInfoWasm` | `ExtendedEpochInfo` | `epoch/extended_epoch_info.rs:205` |
| `IdentityCreditTransferWasm` | `IdentityCreditTransferTransition` | `identity/transitions/identity_credit_transfer_transition.rs:272` |
| `TokenEventWasm` | `TokenEvent` | `group/token_event.rs:90` |
| `IdentityCreateTransitionWasm` | `IdentityCreateTransition` | `identity/transitions/create_transition.rs:258` |
| `GroupActionWasm` | `GroupAction` | `group/action.rs:83` |
| `GroupActionEventWasm` | `GroupActionEvent` | `group/action_event.rs:86` |
| `ResourceVoteChoiceWasm` | `ResourceVoteChoice` | `voting/resource_vote_choice.rs:94` |
| `ContestedDocumentVotePollWinnerInfoWasm` | `ContestedDocumentVotePollWinnerInfo` | `voting/winner_info.rs:122` |
| `ResourceVoteWasm` | `ResourceVote` | `voting/resource_vote.rs:101` |
| `VoteWasm` | `Vote` | `voting/vote.rs:115` |
| `ChainAssetLockProofWasm` | `ChainAssetLockProof` | `asset_lock_proof/chain.rs:114` |
| `VotePollWasm` | `VotePoll` | `voting/vote_poll.rs:240` |
| `ContenderWithSerializedDocumentWasm` | `ContenderWithSerializedDocument` | `voting/contender.rs:109` |
| `InstantAssetLockProofWasm` | `InstantAssetLockProof` | `asset_lock_proof/instant/instant_asset_lock_proof.rs:132` |

---

## Section 4 — WASM wrappers using `impl_wasm_conversions_serde!` (migration targets)

Total: **24 wrappers**. These still bypass the rs-dpp traits.

### 4a — Inner type has `ValueConvertible` only — needs `JsonConvertible` added

Verification corrected the original §4a hypothesis: these inner types have `V` only, not `J+V`. They are **not** "swap-the-macro" easy wins — `JsonConvertible` must be derived on the inner rs-dpp enum first, then the wasm wrapper migrated.

| WASM Wrapper | Inner Type | File:line |
|---|---|---|
| `IdentityCreateFromAddressesTransitionWasm` | `IdentityCreateFromAddressesTransition` | `platform_address/transitions/identity_create_from_addresses_transition.rs:260` |
| `AddressFundingFromAssetLockTransitionWasm` | `AddressFundingFromAssetLockTransition` | `platform_address/transitions/address_funding_from_asset_lock_transition.rs:237` |
| `IdentityTopUpFromAddressesTransitionWasm` | `IdentityTopUpFromAddressesTransition` | `platform_address/transitions/identity_top_up_from_addresses_transition.rs:245` |
| `AddressFundsTransferTransitionWasm` | `AddressFundsTransferTransition` | `platform_address/transitions/address_funds_transfer_transition.rs:219` |
| `IdentityCreditTransferToAddressesTransitionWasm` | `IdentityCreditTransferToAddressesTransition` | `platform_address/transitions/identity_credit_transfer_to_addresses_transition.rs:265` |

### 4b — Inner type missing trait impl entirely (full migration)

| WASM Wrapper | Inner Type | File:line |
|---|---|---|
| `AddressCreditWithdrawalTransitionWasm` | `AddressCreditWithdrawalTransition` | `platform_address/transitions/address_credit_withdrawal_transition.rs:294` |
| `TokenPaymentInfoWasm` | `TokenPaymentInfo` | `state_transitions/batch/token_payment_info.rs:190` |
| `TokenContractInfoWasm` | `TokenContractInfo` | `tokens/contract_info.rs:70` |

### 4c — Verified* result types (proof_result.rs) — domain-specific fallback

These are proof-result wrappers (drive-proof-verifier outputs). May or may not warrant migration depending on whether the inner types live in rs-dpp.

| WASM Wrapper | Inner Type | File:line |
|---|---|---|
| `VerifiedIdentityWasm` | `VerifiedIdentity` | `state_transitions/proof_result.rs:158` |
| `VerifiedTokenBalanceAbsenceWasm` | `VerifiedTokenBalanceAbsence` | `…proof_result.rs:171` |
| `VerifiedTokenBalanceWasm` | `VerifiedTokenBalance` | `…proof_result.rs:194` |
| `VerifiedTokenIdentityInfoWasm` | `VerifiedTokenIdentityInfo` | `…proof_result.rs:209` |
| `VerifiedTokenPricingScheduleWasm` | `VerifiedTokenPricingSchedule` | `…proof_result.rs:227` |
| `VerifiedTokenStatusWasm` | `VerifiedTokenStatus` | `…proof_result.rs:243` |
| `VerifiedPartialIdentityWasm` | `VerifiedPartialIdentity` | `…proof_result.rs:301` |
| `VerifiedBalanceTransferWasm` | `VerifiedBalanceTransfer` | `…proof_result.rs:316` |
| `VerifiedTokenActionWithDocumentWasm` | `VerifiedTokenActionWithDocument` | `…proof_result.rs:374` |
| `VerifiedTokenGroupActionWithDocumentWasm` | `VerifiedTokenGroupActionWithDocument` | `…proof_result.rs:395` |
| `VerifiedTokenGroupActionWithTokenBalanceWasm` | `VerifiedTokenGroupActionWithTokenBalance` | `…proof_result.rs:428` |
| `VerifiedTokenGroupActionWithTokenIdentityInfoWasm` | `VerifiedTokenGroupActionWithTokenIdentityInfo` | `…proof_result.rs:451` |
| `VerifiedTokenGroupActionWithTokenPricingScheduleWasm` | `VerifiedTokenGroupActionWithTokenPricingSchedule` | `…proof_result.rs:474` |
| `VerifiedMasternodeVoteWasm` | `VerifiedMasternodeVote` | `…proof_result.rs:490` |
| `VerifiedNextDistributionWasm` | `VerifiedNextDistribution` | `…proof_result.rs:503` |
| `VerifiedShieldedPoolStateWasm` | `VerifiedShieldedPoolState` | `…proof_result.rs:748` |

(`packages/wasm-dpp/` — the legacy crate — has no usages of either macro.)

---

## Section 5 — rs-dpp domain types **missing trait impls**

Source: Agent C deep scan. Total: **42 missing both J+V**, **9 missing JSON only**, **51 candidates**.

### 5a — Top-priority short list (11)

1. **`DataContract`** — `src/data_contract/mod.rs:107` — **core domain entity** missed by the initial scan. Tagged-enum (V0/V1) routed via `$formatVersion`. *(Note: serialization currently goes through `DataContractInSerializationFormat` which has J only; adding the traits to `DataContract` itself would unify the path.)*
2. **`StateTransition`** — `src/state_transition/mod.rs:431` — top-level union; touches every signing/dispatch path.
3. **`BatchTransition`** — `…/document/batch_transition/mod.rs:~87` — primary document/token mutation entry.
4. **`Document`** — `src/document/mod.rs:54` — core domain object.
5. **`Identity`** *(JSON only)* — `src/identity/identity.rs:43`.
6. **`IdentityPublicKey`** *(JSON only)* — `src/identity/identity_public_key/mod.rs:55`.
7. **`AssetLockProof`** — `src/identity/state_transition/asset_lock_proof/mod.rs:30` — needed by every identity-create/topup flow.
8. **`DocumentTransition`** — `…/batched_transition/document_transition.rs:22`.
9. **`TokenTransition`** — `…/batched_transition/token_transition.rs:50`.
10. **`DocumentBaseTransition`** — `…/document_base_transition/mod.rs:31`.
11. **`PlatformAddress`** — `src/address_funds/platform_address.rs:44`.

### 5b — Identity

| Type | Kind | Missing | File:line |
|---|---|---|---|
| `Identity` | enum | J | `src/identity/identity.rs:43` |
| `IdentityV0` | struct | J | `src/identity/v0/mod.rs:36` *(V0 inner — optional per convention)* |
| `IdentityPublicKey` | enum | J | `src/identity/identity_public_key/mod.rs:55` |
| `PartialIdentity` | struct | J+V | `src/identity/identity.rs:59` |
| `Contender` | enum | J+V | `src/voting/contender_structs/contender/mod.rs:25` *(no serde derives — needs `Serialize`/`Deserialize` first)* |

### 5c — Document / Data Contract

| Type | Kind | Missing | File:line |
|---|---|---|---|
| `DataContract` | enum (V0/V1) | J+V | `src/data_contract/mod.rs:107` — **major omission**, top-priority |
| `Document` | enum | J+V | `src/document/mod.rs:54` |
| `DocumentPatch` | struct | J+V | `src/document/document_patch/mod.rs:9` |
| `ExtendedDocument` | enum | J+V | `src/document/extended_document/mod.rs:30` — has manual serde impls in `serde_serialize.rs:10,94` (gated on `serde-conversion`) |

### 5d — State Transition umbrella

| Type | Kind | Missing | File:line |
|---|---|---|---|
| `StateTransition` | enum (`untagged`) | J+V | `src/state_transition/mod.rs:431` |

### 5e — Address-funds / from-addresses transitions

| Type | Kind | Missing | File:line |
|---|---|---|---|
| `IdentityCreateFromAddressesTransition` | enum | J | `…/identity_create_from_addresses_transition/mod.rs:56` |
| `IdentityTopUpFromAddressesTransition` | enum | J | `…/identity_topup_from_addresses_transition/mod.rs:54` |
| `IdentityCreditTransferToAddressesTransition` | enum | J | `…/identity_credit_transfer_to_addresses_transition/mod.rs:58` |
| `AddressFundingFromAssetLockTransition` | enum | J | `…/address_funds/address_funding_from_asset_lock_transition/mod.rs:56` |
| `AddressFundsTransferTransition` | enum | J | `…/address_funds/address_funds_transfer_transition/mod.rs:56` |
| `AddressCreditWithdrawalTransition` | enum | J+V | `…/address_funds/address_credit_withdrawal_transition/mod.rs:60` |

### 5f — Batch (document/token) transitions

All missing **J+V**. Files all under `src/state_transition/state_transitions/document/batch_transition/`.

| Type | Kind |
|---|---|
| `BatchTransition` | enum V0+V1 |
| `BatchedTransition` | enum (union) |
| `DocumentTransition` | enum (union) |
| `TokenTransition` | enum (union) |
| `DocumentBaseTransition` | enum V0+V1 |
| `DocumentCreateTransition` | enum V0 |
| `DocumentReplaceTransition` | enum V0 |
| `DocumentDeleteTransition` | enum V0 |
| `DocumentTransferTransition` | enum V0 |
| `DocumentPurchaseTransition` | enum V0 |
| `DocumentUpdatePriceTransition` | enum V0 |
| `TokenBaseTransition` | enum V0 |
| `TokenBurnTransition` | enum V0 |
| `TokenMintTransition` | enum V0 |
| `TokenTransferTransition` | enum V0 |
| `TokenFreezeTransition` | enum V0 |
| `TokenUnfreezeTransition` | enum V0 |
| `TokenDestroyFrozenFundsTransition` | enum V0 |
| `TokenEmergencyActionTransition` | enum V0 |
| `TokenConfigUpdateTransition` | enum V0 |
| `TokenClaimTransition` | enum V0 |
| `TokenDirectPurchaseTransition` | enum V0 |
| `TokenSetPriceForDirectPurchaseTransition` | enum V0 |

### 5g — Shielded transitions

All missing **J+V**. Files under `src/state_transition/state_transitions/shielded/`.

| Type | Kind |
|---|---|
| `ShieldTransition` | enum V0 |
| `UnshieldTransition` | enum V0 |
| `ShieldedTransferTransition` | enum V0 |
| `ShieldFromAssetLockTransition` | enum V0 |
| `ShieldedWithdrawalTransition` | enum V0 |

### 5h — Asset-lock-proof / asset-lock-value

| Type | Kind | Missing | File:line |
|---|---|---|---|
| `AssetLockProof` | enum (union) | J+V | `src/identity/state_transition/asset_lock_proof/mod.rs:30` |
| `AssetLockProofType` | enum | J+V | `src/identity/state_transition/asset_lock_proof/mod.rs:135` (no derives — needs `Serialize`/`Deserialize` first) |
| `AssetLockValue` | enum V0 | J+V | `src/asset_lock/reduced_asset_lock_value/mod.rs` |
| `StoredAssetLockInfo` | enum | J+V | `src/asset_lock/mod.rs:9` (unconditional `derive(Serialize, Deserialize)`) |

### 5i — Voting

*(All voting types listed in Section 1's Voting table have trait impls. `ContestedDocumentResourceVotePoll` was previously flagged here in error — verified to have J+V at `…/contested_document_resource_vote_poll/mod.rs:20,28`. `Contender` is in §5b.)*

### 5j — Tokens

| Type | Kind | Missing | File:line |
|---|---|---|---|
| `TokenContractInfo` | enum V0 | J+V | `src/tokens/contract_info/mod.rs:32` |
| `TokenPaymentInfo` | enum V0 | J+V | `src/tokens/token_payment_info/mod.rs:98` |
| `TokenPricingSchedule` | enum | J+V | `src/tokens/token_pricing_schedule.rs:29` |
| `TokenEmergencyAction` | enum | J+V | `src/tokens/emergency_action.rs:14` |
| `GasFeesPaidBy` | enum | J+V | `src/tokens/gas_fees_paid_by.rs:21` |

### 5k — Group

| Type | Kind | Missing | File:line |
|---|---|---|---|
| `GroupStateTransitionInfo` | struct | J+V | `src/group/mod.rs:42` |
| `GroupActionStatus` | enum | J+V | `src/group/group_action_status.rs:9` |
| `GroupStateTransitionInfoStatus` | enum | uncertain | `src/group/mod.rs:15` (no serde derives — needs them first) |
| `GroupStateTransitionResolvedInfo` | struct | J+V | `src/group/mod.rs:53` (has serde derives) |

### 5l — Withdrawal

| Type | Kind | Missing | File:line |
|---|---|---|---|
| `Pooling` | enum (serde_repr) | J+V | `src/withdrawal/mod.rs:12` *(uses `serde_repr::Serialize_repr`/`Deserialize_repr` — should satisfy `Serialize + DeserializeOwned`, but worth a unit test)* |

### 5m — Address

| Type | Kind | Missing | File:line |
|---|---|---|---|
| `PlatformAddress` | enum | J+V | `src/address_funds/platform_address.rs:44` |

### 5n — Validator

| Type | Kind | Missing | File:line |
|---|---|---|---|
| `Validator` | enum V0 | J+V | `src/core_types/validator/mod.rs:12` (gated on `serde-conversion`) |
| `ValidatorSet` | enum V0 | J+V | `src/core_types/validator_set/mod.rs:24` (gated on `serde-conversion`) |

---

## Section 6 — Discrepancies (resolved)

All resolved by the verification agent. Summary:

1. ✅ **Address transitions** (5 types) — Agent C correct: have **V only**. §1 corrected. WASM wrappers cannot trivially migrate to `_inner!` until `JsonConvertible` is added.
2. ✅ **`AddressCreditWithdrawalTransition`** — Agent C correct: missing **J+V**. Listed in §5e.
3. ✅ **`ContestedDocumentResourceVotePoll`** — Agent A correct: has **J+V** at `…/contested_document_resource_vote_poll/mod.rs:20,28`. Removed from §5i.
4. ✅ **`Identity` `JsonConvertible`** — Agent A correct: missing. The "9 enum types migrated" note from memory referred to `ChainAssetLockProof` etc., not `Identity` itself.
5. ✅ **Uncertain serde status** —
   - `ExtendedDocument`: has manual serde impls in `serde_serialize.rs:10,94` (gated on `serde-conversion`) → **eligible**.
   - `StoredAssetLockInfo`: unconditional `derive(Serialize, Deserialize)` at `src/asset_lock/mod.rs:9` → **eligible**.
   - `Validator` / `ValidatorSet`: serde-derived (gated on `serde-conversion`) → **eligible**.
   - `ContestedDocumentVotePollStoredInfo`: only `Encode/Decode` visible → **excluded** until serde added.

### Section 1 corrections applied

- `DataContractMismatch` row → renamed to `DataContractInSerializationFormat` at `src/data_contract/serialized_version/mod.rs:106`.
- `Contender` row removed from §1 voting table (no serde derives — moved to §5b).
- 5 address-transition rows: `J=❌, V=✅`.
- Various line-number refinements.

### Major omission detected

- **`DataContract`** (`src/data_contract/mod.rs:107`) — the core domain entity — was completely missing from §5. Added to §5a (top priority #1) and §5c. Currently relies on `DataContractInSerializationFormat` (which has J only) for serialization.

---

## Section 7 — Counts (post-verification)

| Bucket | Count |
|---|---:|
| rs-dpp types with both J+V | 45 |
| rs-dpp types with only Json | 7 |
| rs-dpp types with only Value | 11 |
| rs-dpp types with impls but **no rs-dpp tests** | ~35 |
| WASM wrappers on `_inner!` | 24 |
| WASM wrappers on `_serde!` | 24 |
| rs-dpp candidates missing J+V | ~46 |
| rs-dpp candidates missing J only | 4 |
| **Total candidate types to review** | **~110** |

Confidence in the merged inventory after verification: **85%** (per Agent E).

## Section 8 — Suggested execution order

1. ✅ **Resolve discrepancies** (Section 6) — done via the verification agent.
2. **Add `JsonConvertible` to address transitions** (§4a inner types) — 5 types in `state_transitions/identity/identity_*_from_addresses_transition/`, `address_funds/address_*` (already V, just need J). Then migrate WASM wrappers `_serde!` → `_inner!`.
3. **High-impact missing impls** — top-11 in §5a, in order: `DataContract`, `StateTransition`, `BatchTransition`, `Document`, `Identity` (J), `IdentityPublicKey` (J), `AssetLockProof`, `DocumentTransition`, `TokenTransition`, `DocumentBaseTransition`, `PlatformAddress`.
4. **Bulk migration** — batch-add impls to remaining document/token transitions (§5f) and shielded transitions (§5g).
5. **Add rs-dpp-side round-trip tests** for the ~35 types in §2 + every newly-added impl.
6. **WASM serde→inner migrations** — sweep §4a/4b/4c after underlying rs-dpp impls land.

### Per-step deliverable

Each impl-adding step should be one PR containing:
- `derive(JsonConvertible)` and/or `derive(ValueConvertible)` on the versioned outer enum.
- Round-trip rs-dpp unit test exercising both directions.
- For tagged-enum types: a test verifying variant-tag preservation.
- WASM wrapper migration `_serde!` → `_inner!` (separate or same PR).
- Updated wasm-dpp2 spec coverage if any new behaviour is exposed.
