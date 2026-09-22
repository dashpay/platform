//! TypeScript discriminated-union declaration and the dispatcher that
//! converts a Rust `StateTransitionProofResult` into the corresponding
//! typed WASM wrapper.

use super::address_funds::{
    VerifiedAddressInfosWasm, VerifiedIdentityFullWithAddressInfosWasm,
    VerifiedIdentityWithAddressInfosWasm,
};
use super::data_contract::{
    VerifiedContractDocumentRemovalWasm, VerifiedContractFeeClaimWasm,
    VerifiedContractModerationListStatusesWasm, VerifiedDataContractWasm,
};
use super::document::VerifiedDocumentsWasm;
use super::helpers::{
    action_status_to_string, build_address_infos_map, build_nullifier_map, doc_to_wasm,
};
use super::identity::{
    VerifiedBalanceTransferWasm, VerifiedIdentityWasm, VerifiedPartialIdentityWasm,
};
use super::shielded::{
    VerifiedAssetLockConsumedWasm, VerifiedAssetLockConsumedWithAddressInfosWasm,
    VerifiedIdentityWithShieldedNullifiersWasm, VerifiedShieldedNullifiersWasm,
    VerifiedShieldedNullifiersWithAddressInfosWasm,
    VerifiedShieldedNullifiersWithWithdrawalDocumentWasm,
};
use super::token::{
    VerifiedTokenActionWithDocumentWasm, VerifiedTokenBalanceAbsenceWasm, VerifiedTokenBalanceWasm,
    VerifiedTokenGroupActionWithDocumentWasm, VerifiedTokenGroupActionWithTokenBalanceWasm,
    VerifiedTokenGroupActionWithTokenIdentityInfoWasm,
    VerifiedTokenGroupActionWithTokenPricingScheduleWasm, VerifiedTokenIdentitiesBalancesWasm,
    VerifiedTokenIdentityInfoWasm, VerifiedTokenPricingScheduleWasm, VerifiedTokenStatusWasm,
};
use super::voting::{VerifiedMasternodeVoteWasm, VerifiedNextDistributionWasm};
use crate::IdentifierWasm;
use crate::error::WasmDppResult;
use crate::utils::JsMapExt;
use dpp::data_contract::config::moderation::ContractModerationListStatus;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use js_sys::{BigInt, Map};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;

// ============================================================================
// TypeScript union type
// ============================================================================

#[wasm_bindgen(typescript_custom_section)]
const TS_PROOF_RESULT_TYPE: &str = r#"
export type StateTransitionProofResultType =
  | VerifiedDataContract
  | VerifiedIdentity
  | VerifiedTokenBalanceAbsence
  | VerifiedTokenBalance
  | VerifiedTokenIdentityInfo
  | VerifiedTokenPricingSchedule
  | VerifiedTokenStatus
  | VerifiedTokenIdentitiesBalances
  | VerifiedPartialIdentity
  | VerifiedBalanceTransfer
  | VerifiedDocuments
  | VerifiedTokenActionWithDocument
  | VerifiedTokenGroupActionWithDocument
  | VerifiedTokenGroupActionWithTokenBalance
  | VerifiedTokenGroupActionWithTokenIdentityInfo
  | VerifiedTokenGroupActionWithTokenPricingSchedule
  | VerifiedMasternodeVote
  | VerifiedNextDistribution
  | VerifiedAddressInfos
  | VerifiedIdentityFullWithAddressInfos
  | VerifiedIdentityWithAddressInfos
  | VerifiedAssetLockConsumed
  | VerifiedAssetLockConsumedWithAddressInfos
  | VerifiedShieldedNullifiers
  | VerifiedShieldedNullifiersWithAddressInfos
  | VerifiedShieldedNullifiersWithWithdrawalDocument
  | VerifiedIdentityWithShieldedNullifiers
  | VerifiedContractModerationListStatuses
  | VerifiedContractFeeClaim
  | VerifiedContractDocumentRemoval;
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "StateTransitionProofResultType")]
    pub type StateTransitionProofResultTypeJs;
}

// ============================================================================
// Conversion function
// ============================================================================

/// Convert a Rust `StateTransitionProofResult` into the corresponding typed
/// WASM wrapper, ready to be returned to JavaScript.
pub fn convert_proof_result(
    result: StateTransitionProofResult,
) -> WasmDppResult<StateTransitionProofResultTypeJs> {
    let js_value: JsValue = match result {
        StateTransitionProofResult::VerifiedDataContract(dc) => VerifiedDataContractWasm {
            data_contract: dc.into(),
        }
        .into(),

        StateTransitionProofResult::VerifiedIdentity(identity) => VerifiedIdentityWasm {
            identity: identity.into(),
        }
        .into(),

        StateTransitionProofResult::VerifiedTokenBalanceAbsence(id) => {
            VerifiedTokenBalanceAbsenceWasm {
                token_id: id.into(),
            }
            .into()
        }

        StateTransitionProofResult::VerifiedTokenBalance(id, amount) => VerifiedTokenBalanceWasm {
            token_id: id.into(),
            balance: amount,
        }
        .into(),

        StateTransitionProofResult::VerifiedTokenIdentityInfo(id, info) => {
            VerifiedTokenIdentityInfoWasm {
                token_id: id.into(),
                token_info: info.into(),
            }
            .into()
        }

        StateTransitionProofResult::VerifiedTokenPricingSchedule(id, schedule) => {
            VerifiedTokenPricingScheduleWasm {
                token_id: id.into(),
                pricing_schedule: schedule.map(Into::into),
            }
            .into()
        }

        StateTransitionProofResult::VerifiedTokenStatus(status) => VerifiedTokenStatusWasm {
            token_status: status.into(),
        }
        .into(),

        StateTransitionProofResult::VerifiedTokenIdentitiesBalances(balances) => {
            let map = Map::from_entries(balances.into_iter().map(|(id, amount)| {
                let key: JsValue = IdentifierWasm::from(id).to_base58().into();
                let val: JsValue = BigInt::from(amount).into();
                (key, val)
            }));
            VerifiedTokenIdentitiesBalancesWasm { balances: map }.into()
        }

        StateTransitionProofResult::VerifiedPartialIdentity(pi) => VerifiedPartialIdentityWasm {
            partial_identity: pi.into(),
        }
        .into(),

        StateTransitionProofResult::VerifiedBalanceTransfer(from, to) => {
            VerifiedBalanceTransferWasm {
                sender: from.into(),
                recipient: to.into(),
            }
            .into()
        }

        StateTransitionProofResult::VerifiedDocuments(docs) => {
            let map = Map::from_entries(docs.into_iter().map(|(id, maybe_doc)| {
                let key: JsValue = IdentifierWasm::from(id).to_base58().into();
                let val: JsValue = match maybe_doc {
                    Some(doc) => doc_to_wasm(doc).into(),
                    None => JsValue::undefined(),
                };
                (key, val)
            }));
            VerifiedDocumentsWasm { documents: map }.into()
        }

        StateTransitionProofResult::VerifiedTokenActionWithDocument(doc) => {
            VerifiedTokenActionWithDocumentWasm {
                document: doc_to_wasm(doc),
            }
            .into()
        }

        StateTransitionProofResult::VerifiedTokenGroupActionWithDocument(power, maybe_doc) => {
            VerifiedTokenGroupActionWithDocumentWasm {
                group_power: power,
                document: maybe_doc.map(doc_to_wasm),
            }
            .into()
        }

        StateTransitionProofResult::VerifiedTokenGroupActionWithTokenBalance(
            power,
            status,
            maybe_balance,
        ) => VerifiedTokenGroupActionWithTokenBalanceWasm {
            group_power: power,
            action_status: action_status_to_string(status),
            balance: maybe_balance,
        }
        .into(),

        StateTransitionProofResult::VerifiedTokenGroupActionWithTokenIdentityInfo(
            power,
            status,
            maybe_info,
        ) => VerifiedTokenGroupActionWithTokenIdentityInfoWasm {
            group_power: power,
            action_status: action_status_to_string(status),
            token_info: maybe_info.map(Into::into),
        }
        .into(),

        StateTransitionProofResult::VerifiedTokenGroupActionWithTokenPricingSchedule(
            power,
            status,
            maybe_schedule,
        ) => VerifiedTokenGroupActionWithTokenPricingScheduleWasm {
            group_power: power,
            action_status: action_status_to_string(status),
            pricing_schedule: maybe_schedule.map(Into::into),
        }
        .into(),

        StateTransitionProofResult::VerifiedMasternodeVote(vote) => {
            VerifiedMasternodeVoteWasm { vote: vote.into() }.into()
        }

        StateTransitionProofResult::VerifiedNextDistribution(vote) => {
            VerifiedNextDistributionWasm { vote: vote.into() }.into()
        }

        StateTransitionProofResult::VerifiedAddressInfos(infos) => VerifiedAddressInfosWasm {
            address_infos: build_address_infos_map(infos),
        }
        .into(),

        StateTransitionProofResult::VerifiedIdentityFullWithAddressInfos(identity, infos) => {
            VerifiedIdentityFullWithAddressInfosWasm {
                identity: identity.into(),
                address_infos: build_address_infos_map(infos),
            }
            .into()
        }

        StateTransitionProofResult::VerifiedIdentityWithAddressInfos(pi, infos) => {
            VerifiedIdentityWithAddressInfosWasm {
                partial_identity: pi.into(),
                address_infos: build_address_infos_map(infos),
            }
            .into()
        }

        StateTransitionProofResult::VerifiedAssetLockConsumed(info) => {
            use dpp::asset_lock::StoredAssetLockInfo;
            use dpp::asset_lock::reduced_asset_lock_value::AssetLockValueGettersV0;
            let (status, initial, remaining) = match info {
                StoredAssetLockInfo::FullyConsumed => ("FullyConsumed".to_string(), None, None),
                StoredAssetLockInfo::PartiallyConsumed(val) => (
                    "PartiallyConsumed".to_string(),
                    Some(val.initial_credit_value()),
                    Some(val.remaining_credit_value()),
                ),
                StoredAssetLockInfo::NotPresent => ("NotPresent".to_string(), None, None),
            };
            VerifiedAssetLockConsumedWasm::new(status, initial, remaining).into()
        }

        StateTransitionProofResult::VerifiedAssetLockConsumedWithAddressInfos(info, infos) => {
            use dpp::asset_lock::StoredAssetLockInfo;
            use dpp::asset_lock::reduced_asset_lock_value::AssetLockValueGettersV0;
            let (status, initial, remaining) = match info {
                StoredAssetLockInfo::FullyConsumed => ("FullyConsumed".to_string(), None, None),
                StoredAssetLockInfo::PartiallyConsumed(val) => (
                    "PartiallyConsumed".to_string(),
                    Some(val.initial_credit_value()),
                    Some(val.remaining_credit_value()),
                ),
                StoredAssetLockInfo::NotPresent => ("NotPresent".to_string(), None, None),
            };
            VerifiedAssetLockConsumedWithAddressInfosWasm::new(
                status,
                initial,
                remaining,
                build_address_infos_map(infos),
            )
            .into()
        }

        StateTransitionProofResult::VerifiedShieldedNullifiers(nullifiers) => {
            VerifiedShieldedNullifiersWasm::from_map(build_nullifier_map(nullifiers)).into()
        }

        StateTransitionProofResult::VerifiedShieldedNullifiersWithAddressInfos(
            nullifiers,
            infos,
        ) => VerifiedShieldedNullifiersWithAddressInfosWasm::new(
            build_nullifier_map(nullifiers),
            build_address_infos_map(infos),
        )
        .into(),

        StateTransitionProofResult::VerifiedShieldedNullifiersWithWithdrawalDocument(
            nullifiers,
            docs,
        ) => {
            let doc_map = Map::from_entries(docs.into_iter().map(|(id, maybe_doc)| {
                let key: JsValue = IdentifierWasm::from(id).to_base58().into();
                let val: JsValue = match maybe_doc {
                    Some(doc) => doc_to_wasm(doc).into(),
                    None => JsValue::undefined(),
                };
                (key, val)
            }));
            VerifiedShieldedNullifiersWithWithdrawalDocumentWasm::new(
                build_nullifier_map(nullifiers),
                doc_map,
            )
            .into()
        }

        StateTransitionProofResult::VerifiedIdentityWithShieldedNullifiers(
            identity,
            nullifiers,
        ) => VerifiedIdentityWithShieldedNullifiersWasm::new(
            identity.into(),
            build_nullifier_map(nullifiers),
        )
        .into(),

        StateTransitionProofResult::VerifiedContractModerationListStatuses(
            contract_id,
            identity_id,
            statuses,
        ) => {
            let mut lists = vec![];
            let mut banned = None;
            let mut ban_reason = None;
            let mut suspended_until = None;
            let mut suspension_reason = None;
            let mut warnings = None;
            for status in statuses.0 {
                match status {
                    ContractModerationListStatus::Banlist { ban } => {
                        lists.push("banlist".to_string());
                        banned = Some(ban.is_some());
                        ban_reason = ban.map(|ban| ban.reason);
                    }
                    ContractModerationListStatus::Suspensions { suspension } => {
                        lists.push("suspensions".to_string());
                        if let Some(suspension) = suspension {
                            suspended_until = Some(suspension.until);
                            suspension_reason = Some(suspension.reason);
                        }
                    }
                    ContractModerationListStatus::Warnings {
                        warnings: proved_warnings,
                    } => {
                        lists.push("warnings".to_string());
                        warnings = Some(proved_warnings);
                    }
                }
            }
            VerifiedContractModerationListStatusesWasm {
                contract_id: contract_id.into(),
                identity_id: identity_id.into(),
                lists,
                banned,
                ban_reason,
                suspended_until,
                suspension_reason,
                warnings,
            }
            .into()
        }

        StateTransitionProofResult::VerifiedContractFeeClaim(
            contract_id,
            pot,
            last_claim,
            remaining_credits,
            balances,
        ) => {
            let balances = Map::from_entries(balances.into_iter().map(|(id, credits)| {
                let key: JsValue = IdentifierWasm::from(id).to_base58().into();
                let val: JsValue = BigInt::from(credits).into();
                (key, val)
            }));
            VerifiedContractFeeClaimWasm {
                contract_id: contract_id.into(),
                pot: pot.to_string(),
                last_claim_epoch: last_claim.epoch_index,
                last_claim_time_ms: last_claim.time_ms,
                last_claimant_id: last_claim.claimant_id.into(),
                remaining_credits,
                balances,
            }
            .into()
        }
        StateTransitionProofResult::VerifiedContractDocumentRemoval(
            contract_id,
            document_type_name,
            document_id,
            removal,
        ) => VerifiedContractDocumentRemovalWasm {
            contract_id: contract_id.into(),
            document_type_name,
            document_id: document_id.into(),
            document_owner_id: removal.document_owner_id.into(),
            moderator_id: removal.moderator_id.into(),
            reason: removal.reason,
            removed_at: removal.removed_at,
            document_hash: removal.document_hash,
            restored_by: removal
                .restoration
                .as_ref()
                .map(|restoration| restoration.moderator_id.into()),
            restored_at: removal
                .restoration
                .as_ref()
                .map(|restoration| restoration.restored_at),
        }
        .into(),
    };

    Ok(js_value.into())
}
