use lazy_static::__Deref;

use dpp::platform_value::string_encoding::Encoding;
use dpp::{
    contracts::{
        dashpay_contract, dpns_contract, feature_flags_contract, masternode_reward_shares_contract,
        withdrawals_contract,
    },
    document::document_transition::Action,
    errors::ProtocolError,
    prelude::Identifier,
};

use super::{DataTrigger, DataTriggerKind};

/// returns Date Triggers filtered out by dataContractId, documentType, transactionAction
pub fn get_data_triggers<'a>(
    data_contract_id: &'a Identifier,
    document_type_name: &'a str,
    transition_action: Action,
    data_triggers_list: impl IntoIterator<Item = &'a DataTrigger>,
) -> Result<Vec<&'a DataTrigger>, ProtocolError> {
    Ok(data_triggers_list
        .into_iter()
        .filter(|dt| {
            dt.is_matching_trigger_for_data(data_contract_id, document_type_name, transition_action)
        })
        .collect())
}

/// Retrieves a list of all known data triggers.
///
/// This function gets all known data triggers which are then returned
/// as a vector of `DataTrigger` structs.
///
/// # Returns
///
/// A `Vec<DataTrigger>` containing all known data triggers.
///
/// # Errors
///
/// Returns a `ProtocolError` if there was an error.
pub fn data_triggers() -> Result<Vec<DataTrigger>, ProtocolError> {
    let dpns_data_contract_id =
        Identifier::from_string(&dpns_contract::system_ids().contract_id, Encoding::Base58)?;
    let dpns_owner_id =
        Identifier::from_string(&dpns_contract::system_ids().owner_id, Encoding::Base58)?;

    let dashpay_data_contract_id = Identifier::from_string(
        &dashpay_contract::system_ids().contract_id,
        Encoding::Base58,
    )?;
    let feature_flags_data_contract_id = Identifier::from_string(
        &feature_flags_contract::system_ids().contract_id,
        Encoding::Base58,
    )?;
    let feature_flags_owner_id = Identifier::from_string(
        &feature_flags_contract::system_ids().owner_id,
        Encoding::Base58,
    )?;
    let master_node_reward_shares_contract_id = Identifier::from_string(
        &masternode_reward_shares_contract::system_ids().contract_id,
        Encoding::Base58,
    )?;
    let withdrawals_owner_id = withdrawals_contract::OWNER_ID.deref();
    let withdrawals_contract_id = withdrawals_contract::CONTRACT_ID.deref();

    let data_triggers = vec![
        DataTrigger {
            data_contract_id: dpns_data_contract_id,
            document_type: "domain".to_string(),
            transition_action: Action::Create,
            data_trigger_kind: DataTriggerKind::DataTriggerCreateDomain,
            top_level_identity: Some(dpns_owner_id),
        },
        DataTrigger {
            data_contract_id: dpns_data_contract_id,
            document_type: "domain".to_string(),
            transition_action: Action::Replace,
            data_trigger_kind: DataTriggerKind::DataTriggerReject,
            top_level_identity: None,
        },
        DataTrigger {
            data_contract_id: dpns_data_contract_id,
            document_type: "domain".to_string(),
            transition_action: Action::Delete,
            data_trigger_kind: DataTriggerKind::DataTriggerReject,
            top_level_identity: None,
        },
        DataTrigger {
            data_contract_id: dpns_data_contract_id,
            document_type: "preorder".to_string(),
            transition_action: Action::Delete,
            data_trigger_kind: DataTriggerKind::DataTriggerReject,
            top_level_identity: None,
        },
        DataTrigger {
            data_contract_id: dpns_data_contract_id,
            document_type: "preorder".to_string(),
            transition_action: Action::Delete,
            data_trigger_kind: DataTriggerKind::DataTriggerReject,
            top_level_identity: None,
        },
        DataTrigger {
            data_contract_id: dashpay_data_contract_id,
            document_type: "contactRequest".to_string(),
            transition_action: Action::Create,
            data_trigger_kind: DataTriggerKind::CreateDataContractRequest,
            top_level_identity: None,
        },
        DataTrigger {
            data_contract_id: dashpay_data_contract_id,
            document_type: "contactRequest".to_string(),
            transition_action: Action::Replace,
            data_trigger_kind: DataTriggerKind::DataTriggerReject,
            top_level_identity: None,
        },
        DataTrigger {
            data_contract_id: dashpay_data_contract_id,
            document_type: "contactRequest".to_string(),
            transition_action: Action::Delete,
            data_trigger_kind: DataTriggerKind::DataTriggerReject,
            top_level_identity: None,
        },
        DataTrigger {
            data_contract_id: feature_flags_data_contract_id,
            document_type: feature_flags_contract::types::UPDATE_CONSENSUS_PARAMS.to_string(),
            transition_action: Action::Create,
            data_trigger_kind: DataTriggerKind::CrateFeatureFlag,
            top_level_identity: Some(feature_flags_owner_id),
        },
        DataTrigger {
            data_contract_id: feature_flags_data_contract_id,
            document_type: feature_flags_contract::types::UPDATE_CONSENSUS_PARAMS.to_string(),
            transition_action: Action::Replace,
            data_trigger_kind: DataTriggerKind::DataTriggerReject,
            top_level_identity: None,
        },
        DataTrigger {
            data_contract_id: feature_flags_data_contract_id,
            document_type: feature_flags_contract::types::UPDATE_CONSENSUS_PARAMS.to_string(),
            transition_action: Action::Delete,
            data_trigger_kind: DataTriggerKind::DataTriggerReject,
            top_level_identity: None,
        },
        DataTrigger {
            data_contract_id: master_node_reward_shares_contract_id,
            document_type: feature_flags_contract::types::UPDATE_CONSENSUS_PARAMS.to_string(),
            transition_action: Action::Create,
            data_trigger_kind: DataTriggerKind::DataTriggerRewardShare,
            top_level_identity: None,
        },
        DataTrigger {
            data_contract_id: master_node_reward_shares_contract_id,
            document_type: "rewardShare".to_string(),
            transition_action: Action::Replace,
            data_trigger_kind: DataTriggerKind::DataTriggerRewardShare,
            top_level_identity: None,
        },
        DataTrigger {
            data_contract_id: *withdrawals_contract_id,
            document_type: withdrawals_contract::document_types::WITHDRAWAL.to_string(),
            transition_action: Action::Create,
            data_trigger_kind: DataTriggerKind::DataTriggerReject,
            top_level_identity: None,
        },
        DataTrigger {
            data_contract_id: *withdrawals_contract_id,
            document_type: withdrawals_contract::document_types::WITHDRAWAL.to_string(),
            transition_action: Action::Replace,
            data_trigger_kind: DataTriggerKind::DataTriggerReject,
            top_level_identity: None,
        },
        DataTrigger {
            data_contract_id: *withdrawals_contract_id,
            document_type: withdrawals_contract::document_types::WITHDRAWAL.to_string(),
            transition_action: Action::Delete,
            data_trigger_kind: DataTriggerKind::DeleteWithdrawal,
            top_level_identity: Some(*withdrawals_owner_id),
        },
    ];
    Ok(data_triggers)
}
