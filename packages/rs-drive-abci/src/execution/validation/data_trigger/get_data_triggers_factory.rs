use lazy_static::__Deref;

use dpp::system_data_contracts::feature_flags_contract::document_types::update_consensus_params;
use dpp::system_data_contracts::withdrawals_contract::document_types::withdrawal;
use dpp::system_data_contracts::{
    dashpay_contract, dpns_contract, feature_flags_contract, withdrawals_contract,
    SystemDataContract,
};
use dpp::{errors::ProtocolError, prelude::Identifier};

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
    let data_triggers = vec![
        DataTrigger {
            data_contract_id: dpns_contract::ID,
            document_type: "domain".to_string(),
            transition_action: Action::Create,
            data_trigger_kind: DataTriggerKind::DataTriggerCreateDomain,
        },
        DataTrigger {
            data_contract_id: dpns_contract::ID,
            document_type: "domain".to_string(),
            transition_action: Action::Replace,
            data_trigger_kind: DataTriggerKind::DataTriggerReject,
        },
        DataTrigger {
            data_contract_id: dpns_contract::ID,
            document_type: "domain".to_string(),
            transition_action: Action::Delete,
            data_trigger_kind: DataTriggerKind::DataTriggerReject,
        },
        DataTrigger {
            data_contract_id: dpns_contract::ID,
            document_type: "preorder".to_string(),
            transition_action: Action::Delete,
            data_trigger_kind: DataTriggerKind::DataTriggerReject,
        },
        DataTrigger {
            data_contract_id: dpns_contract::ID,
            document_type: "preorder".to_string(),
            transition_action: Action::Delete,
            data_trigger_kind: DataTriggerKind::DataTriggerReject,
        },
        DataTrigger {
            data_contract_id: dashpay_contract::ID,
            document_type: "contactRequest".to_string(),
            transition_action: Action::Create,
            data_trigger_kind: DataTriggerKind::CreateDataContractRequest,
        },
        DataTrigger {
            data_contract_id: dashpay_contract::ID,
            document_type: "contactRequest".to_string(),
            transition_action: Action::Replace,
            data_trigger_kind: DataTriggerKind::DataTriggerReject,
        },
        DataTrigger {
            data_contract_id: dashpay_contract::ID,
            document_type: "contactRequest".to_string(),
            transition_action: Action::Delete,
            data_trigger_kind: DataTriggerKind::DataTriggerReject,
        },
        DataTrigger {
            data_contract_id: dashpay_contract::ID,
            document_type: update_consensus_params::NAME.to_string(),
            transition_action: Action::Create,
            data_trigger_kind: DataTriggerKind::CrateFeatureFlag,
        },
        DataTrigger {
            data_contract_id: SystemDataContract::FeatureFlags.id(),
            document_type: update_consensus_params::NAME.to_string(),
            transition_action: Action::Replace,
            data_trigger_kind: DataTriggerKind::DataTriggerReject,
        },
        DataTrigger {
            data_contract_id: SystemDataContract::FeatureFlags.id(),
            document_type: update_consensus_params::NAME.to_string(),
            transition_action: Action::Delete,
            data_trigger_kind: DataTriggerKind::DataTriggerReject,
        },
        DataTrigger {
            data_contract_id: SystemDataContract::MasternodeRewards.id(),
            document_type: update_consensus_params::NAME.to_string(),
            transition_action: Action::Create,
            data_trigger_kind: DataTriggerKind::DataTriggerRewardShare,
        },
        DataTrigger {
            data_contract_id: SystemDataContract::MasternodeRewards.id(),
            document_type: "rewardShare".to_string(),
            transition_action: Action::Replace,
            data_trigger_kind: DataTriggerKind::DataTriggerRewardShare,
        },
        DataTrigger {
            data_contract_id: SystemDataContract::Withdrawals.id(),
            document_type: withdrawal::NAME.to_string(),
            transition_action: Action::Create,
            data_trigger_kind: DataTriggerKind::DataTriggerReject,
        },
        DataTrigger {
            data_contract_id: SystemDataContract::Withdrawals.id(),
            document_type: withdrawal::NAME.to_string(),
            transition_action: Action::Replace,
            data_trigger_kind: DataTriggerKind::DataTriggerReject,
        },
        DataTrigger {
            data_contract_id: SystemDataContract::Withdrawals.id(),
            document_type: withdrawal::NAME.to_string(),
            transition_action: Action::Delete,
            data_trigger_kind: DataTriggerKind::DeleteWithdrawal,
        },
    ];

    Ok(data_triggers)
}
