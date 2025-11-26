# wasm-dpp2 entity conversion map

This file is generated on demand. It lists every exported `*Wasm` entity in `packages/wasm-dpp2` and the wasm-bindgen `to*`/`from*` helpers they expose as of the current workspace state.

## Task context

We want a uniform interface across all wasm-dpp2 entities (each `*Wasm` wrapper maps 1:1 to an `rs-dpp` type). The expectations:

- `toObject` returns a `JsValue` containing a plain JS object; binary data should be emitted as `Uint8Array`.
- `fromObject` accepts a plain JS object (not an existing wasm wrapper) and instantiates the entity.
- `toJSON` returns a `JsValue` representing the JSON form, using string encodings for binary fields; `fromJSON` reverses it.
- `toBytes`/`fromBytes` are mandatory alongside the object/JSON helpers so callers can move raw binary data without extra conversions.
- `toBase64`, `toHex`, `toBase58` (and similar conversions) remain optional and can be added per-entity when useful.
- Prefer reusing the underlying `rs-dpp` serialization helpers (e.g., `to_object`, `to_raw_object`,  `from_object`, TryFrom<Value>, serde Platform serialization). Platform Value already round-trips to/from JSON, so lean on those helpers rather than reimplementing logic in wasm.

This catalog helps track which entities already expose these conversions and which still need work.

Implementation plan:
We should go one by one and report current status and what we have in rs-dpp for this structure. then tell me what you planning to do. get conformation and proceed with implementation. when it's ready and I reviewed, you implement tests and I review and make sure they are good. when it's finished add a checkmark here that we finished with it. then we go to the next one.

BatchTransitionWasm now uses a unified `fromBatchedTransitions` constructor (no signature args) and exposes object/JSON converters alongside bytes/base64/hex helpers. As we normalize entities, remove duplicate/legacy conversion helpers (`fromRawObject`, `from_value`, `to_value`, etc.) in favor of the standard `fromObject`/`fromJSON`/`fromBytes` surface.

| Entity | Source file | `to*` methods | `from*` methods |
| --- | --- | --- | --- |
| ✅ `AssetLockProofWasm` | `packages/wasm-dpp2/src/asset_lock_proof/proof.rs` | toBase64, toBytes, toHex, toJSON, toObject | fromBase64, fromBytes, fromHex, fromJSON, fromObject |
| 🔸 `AuthorizedActionTakersWasm` (skipped) | `packages/wasm-dpp2/src/tokens/configuration/authorized_action_takers.rs` | — | — |
| `BatchTransitionWasm` | `packages/wasm-dpp2/src/state_transitions/batch/batch_transition.rs` | toBase64, toBytes, toHex, toObject, toJSON, toStateTransition | fromBase64, fromBatchedTransitions, fromBytes, fromHex, fromJSON, fromObject, fromStateTransition |
| 🔸 `BatchedTransitionWasm` (skipped) | `packages/wasm-dpp2/src/state_transitions/batch/batched_transition.rs` | toTransition | — |
| 🔸 `BlockBasedDistributionWasm` (skipped) | `packages/wasm-dpp2/src/tokens/configuration/reward_distribution_type.rs` | — | — |
| 🔸 `BlockInfoWasm` (skipped) | `packages/wasm-dpp2/src/block.rs` | — | — |
| `ChainAssetLockProofWasm` | `packages/wasm-dpp2/src/asset_lock_proof/chain.rs` | toBytes, toJSON, toObject | fromBytes, fromJSON, fromObject, fromRawObject |
| `ChangeControlRulesWasm` | `packages/wasm-dpp2/src/tokens/configuration/change_control_rules.rs` | — | — |
| `ConsensusErrorWasm` | `packages/wasm-dpp2/src/consensus_error.rs` | — | — |
| `ContenderWithSerializedDocumentWasm` | `packages/wasm-dpp2/src/voting/contender.rs` | — | — |
| `ContestedDocumentVotePollWinnerInfoWasm` | `packages/wasm-dpp2/src/voting/winner_info.rs` | — | — |
| `ContractBoundsWasm` | `packages/wasm-dpp2/src/data_contract/contract_bounds.rs` | — | — |
| `CoreScriptWasm` | `packages/wasm-dpp2/src/core_script.rs` | toAddress, toBase64, toBytes, toHex, toString | fromBytes |
| `DataContractCreateTransitionWasm` | `packages/wasm-dpp2/src/data_contract/transitions/create.rs` | toBase64, toBytes, toHex, toStateTransition | fromBase64, fromBytes, fromHex, fromStateTransition |
| `DataContractUpdateTransitionWasm` | `packages/wasm-dpp2/src/data_contract/transitions/update.rs` | toBase64, toBytes, toHex, toStateTransition | fromBase64, fromBytes, fromHex, fromStateTransition |
| `DataContractWasm` | `packages/wasm-dpp2/src/data_contract/model.rs` | toBase64, toBytes, toHex, toJSON, toObject | fromBase64, fromBytes, fromHex, fromJSON, fromObject |
| `DistributionExponentialWasm` | `packages/wasm-dpp2/src/tokens/configuration/distribution_structs.rs` | — | — |
| `DistributionFixedAmountWasm` | `packages/wasm-dpp2/src/tokens/configuration/distribution_structs.rs` | — | — |
| `DistributionFunctionWasm` | `packages/wasm-dpp2/src/tokens/configuration/distribution_function.rs` | — | — |
| `DistributionInvertedLogarithmicWasm` | `packages/wasm-dpp2/src/tokens/configuration/distribution_structs.rs` | — | — |
| `DistributionLinearWasm` | `packages/wasm-dpp2/src/tokens/configuration/distribution_structs.rs` | — | — |
| `DistributionLogarithmicWasm` | `packages/wasm-dpp2/src/tokens/configuration/distribution_structs.rs` | — | — |
| `DistributionPolynomialWasm` | `packages/wasm-dpp2/src/tokens/configuration/distribution_structs.rs` | — | — |
| `DistributionRandomWasm` | `packages/wasm-dpp2/src/tokens/configuration/distribution_structs.rs` | — | — |
| `DistributionStepDecreasingAmountWasm` | `packages/wasm-dpp2/src/tokens/configuration/distribution_structs.rs` | — | — |
| `DocumentBaseTransitionWasm` | `packages/wasm-dpp2/src/state_transitions/batch/document_base_transition.rs` | — | — |
| `DocumentCreateTransitionWasm` | `packages/wasm-dpp2/src/state_transitions/batch/document_transitions/create.rs` | toDocumentTransition | fromDocumentTransition |
| `DocumentDeleteTransitionWasm` | `packages/wasm-dpp2/src/state_transitions/batch/document_transitions/delete.rs` | toDocumentTransition | fromDocumentTransition |
| `DocumentPurchaseTransitionWasm` | `packages/wasm-dpp2/src/state_transitions/batch/document_transitions/purchase.rs` | toDocumentTransition | fromDocumentTransition |
| `DocumentReplaceTransitionWasm` | `packages/wasm-dpp2/src/state_transitions/batch/document_transitions/replace.rs` | toDocumentTransition | fromDocumentTransition |
| `DocumentTransferTransitionWasm` | `packages/wasm-dpp2/src/state_transitions/batch/document_transitions/transfer.rs` | toDocumentTransition | fromDocumentTransition |
| `DocumentTransitionWasm` | `packages/wasm-dpp2/src/state_transitions/batch/document_transition.rs` | — | — |
| `DocumentUpdatePriceTransitionWasm` | `packages/wasm-dpp2/src/state_transitions/batch/document_transitions/update_price.rs` | toDocumentTransition | fromDocumentTransition |
| `DocumentWasm` | `packages/wasm-dpp2/src/data_contract/document/model.rs` | — | — |
| 🔸 `EpochBasedDistributionWasm` (skipped) | `packages/wasm-dpp2/src/tokens/configuration/reward_distribution_type.rs` | — | — |
| `ExtendedEpochInfoWasm` | `packages/wasm-dpp2/src/epoch/extended_epoch_info.rs` | — | — |
| `FinalizedEpochInfoWasm` | `packages/wasm-dpp2/src/epoch/finalized_epoch_info.rs` | — | — |
| `GroupActionEventWasm` | `packages/wasm-dpp2/src/group/action_event.rs` | — | — |
| `GroupActionWasm` | `packages/wasm-dpp2/src/group/action.rs` | — | — |
| `GroupStateTransitionInfoWasm` | `packages/wasm-dpp2/src/state_transitions/base/group_state_transition_info.rs` | — | — |
| `GroupWasm` | `packages/wasm-dpp2/src/tokens/configuration/group.rs` | — | — |
| `IdentifierWasm` | `packages/wasm-dpp2/src/identifier.rs` | toBase58, toBase64, toBytes, toHex | fromBase58, fromBase64, fromBytes, fromHex |
| `IdentityCreateTransitionWasm` | `packages/wasm-dpp2/src/identity/transitions/create_transition.rs` | toBytes, toStateTransition | fromBase64, fromBytes, fromHex, fromStateTransition |
| `IdentityCreditTransferWasm` | `packages/wasm-dpp2/src/identity/transitions/identity_credit_transfer_transition.rs` | toBase64, toBytes, toHex, toStateTransition | fromBase64, fromBytes, fromHex, fromStateTransition |
| `IdentityCreditWithdrawalTransitionWasm` | `packages/wasm-dpp2/src/identity/transitions/credit_withdrawal_transition.rs` | toBase64, toBytes, toHex, toStateTransition | fromBase64, fromBytes, fromHex, fromStateTransition |
| `IdentityPublicKeyInCreationWasm` | `packages/wasm-dpp2/src/identity/transitions/public_key_in_creation.rs` | — | — |
| `IdentityPublicKeyWasm` | `packages/wasm-dpp2/src/identity/public_key.rs` | toBytes, toJSON, toObject | fromJSON, fromObject |
| `IdentityTokenInfoWasm` | `packages/wasm-dpp2/src/tokens/info.rs` | — | — |
| `IdentityTopUpTransitionWasm` | `packages/wasm-dpp2/src/identity/transitions/top_up_transition.rs` | toBase64, toBytes, toHex, toStateTransition | fromBase64, fromBytes, fromHex, fromStateTransition |
| `IdentityUpdateTransitionWasm` | `packages/wasm-dpp2/src/identity/transitions/update_transition.rs` | toBase64, toBytes, toHex, toStateTransition | fromBase64, fromBytes, fromHex, fromStateTransition |
| `IdentityWasm` | `packages/wasm-dpp2/src/identity/model.rs` | toBase64, toBytes, toHex, toJSON, toObject | fromBase64, fromBytes, fromHex, fromJSON, fromObject |
| `InstantAssetLockProofWasm` | `packages/wasm-dpp2/src/asset_lock_proof/instant/instant_asset_lock_proof.rs` | toObject | fromObject |
| `InstantLockWasm` | `packages/wasm-dpp2/src/asset_lock_proof/instant/instant_lock.rs` | — | — |
| `MasternodeVoteTransitionWasm` | `packages/wasm-dpp2/src/identity/transitions/masternode_vote_transition.rs` | toBytes, toStateTransition | fromBase64, fromBytes, fromHex, fromStateTransition |
| `OutPointWasm` | `packages/wasm-dpp2/src/asset_lock_proof/outpoint.rs` | toBase64, toBytes, toHex | fromBase64, fromBytes, fromHex |
| `PartialIdentityWasm` | `packages/wasm-dpp2/src/identity/partial_identity.rs` | — | — |
| `PrefundedVotingBalanceWasm` | `packages/wasm-dpp2/src/state_transitions/batch/prefunded_voting_balance.rs` | — | — |
| `PrivateEncryptedNoteWasm` | `packages/wasm-dpp2/src/tokens/encrypted_note/private_encrypted_note.rs` | — | — |
| `PrivateKeyWasm` | `packages/wasm-dpp2/src/private_key.rs` | toBytes, toHex | fromBytes, fromHex, fromWIF |
| `PublicKeyWasm` | `packages/wasm-dpp2/src/public_key.rs` | toBytes | fromBytes |
| `ResourceVoteChoiceWasm` | `packages/wasm-dpp2/src/voting/resource_vote_choice.rs` | — | — |
| `RewardDistributionTypeWasm` | `packages/wasm-dpp2/src/tokens/configuration/reward_distribution_type.rs` | — | — |
| `SharedEncryptedNoteWasm` | `packages/wasm-dpp2/src/tokens/encrypted_note/shared_encrypted_note.rs` | — | — |
| `StateTransitionWasm` | `packages/wasm-dpp2/src/state_transitions/base/state_transition.rs` | toBase64, toBytes, toHex | fromBase64, fromBytes, fromHex |
| 🔸 `TimeBasedDistributionWasm` (skipped) | `packages/wasm-dpp2/src/tokens/configuration/reward_distribution_type.rs` | — | — |
| `TokenBaseTransitionWasm` | `packages/wasm-dpp2/src/state_transitions/batch/token_base_transition.rs` | — | — |
| `TokenBurnTransitionWasm` | `packages/wasm-dpp2/src/state_transitions/batch/token_transitions/token_burn.rs` | — | — |
| `TokenClaimTransitionWasm` | `packages/wasm-dpp2/src/state_transitions/batch/token_transitions/token_claim.rs` | — | — |
| `TokenConfigUpdateTransitionWasm` | `packages/wasm-dpp2/src/state_transitions/batch/token_transitions/config_update.rs` | — | — |
| `TokenConfigurationChangeItemWasm` | `packages/wasm-dpp2/src/tokens/configuration_change_item/token_configuration_change_item.rs` | — | — |
| `TokenConfigurationConventionWasm` | `packages/wasm-dpp2/src/tokens/configuration/configuration_convention.rs` | — | — |
| `TokenConfigurationLocalizationWasm` | `packages/wasm-dpp2/src/tokens/configuration/localization.rs` | toJSON | fromJSON, fromObject |
| `TokenConfigurationWasm` | `packages/wasm-dpp2/src/tokens/configuration/token_configuration.rs` | — | — |
| `TokenContractInfoWasm` | `packages/wasm-dpp2/src/tokens/contract_info.rs` | — | — |
| `TokenDestroyFrozenFundsTransitionWasm` | `packages/wasm-dpp2/src/state_transitions/batch/token_transitions/token_destroy_frozen_funds.rs` | — | — |
| `TokenDirectPurchaseTransitionWasm` | `packages/wasm-dpp2/src/state_transitions/batch/token_transitions/direct_purchase.rs` | — | — |
| `TokenDistributionRecipientWasm` | `packages/wasm-dpp2/src/tokens/configuration/distribution_recipient.rs` | — | — |
| `TokenDistributionRulesWasm` | `packages/wasm-dpp2/src/tokens/configuration/distribution_rules.rs` | — | — |
| `TokenEmergencyActionTransitionWasm` | `packages/wasm-dpp2/src/state_transitions/batch/token_transitions/token_emergency_action.rs` | — | — |
| `TokenEventWasm` | `packages/wasm-dpp2/src/group/token_event.rs` | toObject | — |
| `TokenFreezeTransitionWasm` | `packages/wasm-dpp2/src/state_transitions/batch/token_transitions/token_freeze.rs` | — | — |
| `TokenKeepsHistoryRulesWasm` | `packages/wasm-dpp2/src/tokens/configuration/keeps_history_rules.rs` | — | — |
| `TokenMarketplaceRulesWasm` | `packages/wasm-dpp2/src/tokens/configuration/marketplace_rules.rs` | — | — |
| `TokenMintTransitionWasm` | `packages/wasm-dpp2/src/state_transitions/batch/token_transitions/token_mint.rs` | — | — |
| `TokenPaymentInfoWasm` | `packages/wasm-dpp2/src/state_transitions/batch/token_payment_info.rs` | — | — |
| `TokenPerpetualDistributionWasm` | `packages/wasm-dpp2/src/tokens/configuration/perpetual_distribution.rs` | — | — |
| `TokenPreProgrammedDistributionWasm` | `packages/wasm-dpp2/src/tokens/configuration/pre_programmed_distribution.rs` | — | — |
| `TokenPricingScheduleWasm` | `packages/wasm-dpp2/src/state_transitions/batch/token_pricing_schedule.rs` | — | — |
| `TokenSetPriceForDirectPurchaseTransitionWasm` | `packages/wasm-dpp2/src/state_transitions/batch/token_transitions/set_price_for_direct_purchase.rs` | — | — |
| `TokenStatusWasm` | `packages/wasm-dpp2/src/tokens/status.rs` | — | — |
| `TokenTradeModeWasm` | `packages/wasm-dpp2/src/tokens/configuration/trade_mode.rs` | — | — |
| `TokenTransferTransitionWasm` | `packages/wasm-dpp2/src/state_transitions/batch/token_transitions/token_transfer.rs` | — | — |
| `TokenTransitionWasm` | `packages/wasm-dpp2/src/state_transitions/batch/token_transition.rs` | — | — |
| `TokenUnFreezeTransitionWasm` | `packages/wasm-dpp2/src/state_transitions/batch/token_transitions/token_unfreeze.rs` | — | — |
| `TxOutWasm` | `packages/wasm-dpp2/src/asset_lock_proof/tx_out.rs` | — | — |
| `VotePollWasm` | `packages/wasm-dpp2/src/voting/vote_poll.rs` | toString | — |
| `VoteWasm` | `packages/wasm-dpp2/src/voting/vote.rs` | — | — |
