use dpp::contracts::{dashpay_contract, dpns_contract, feature_flags_contract, masternode_reward_shares_contract, withdrawals_contract};
use crate::execution::validation::state_transition::documents_batch::data_triggers::triggers::dashpay::create_contact_request_data_trigger;
use crate::execution::validation::state_transition::documents_batch::data_triggers::triggers::dpns::create_domain_data_trigger;
use crate::execution::validation::state_transition::documents_batch::data_triggers::triggers::feature_flags::create_feature_flag_data_trigger;
use crate::execution::validation::state_transition::documents_batch::data_triggers::triggers::reject::reject_data_trigger;
use crate::execution::validation::state_transition::documents_batch::data_triggers::triggers::reward_share::create_masternode_reward_shares_data_trigger;
use crate::execution::validation::state_transition::documents_batch::data_triggers::triggers::withdrawals::delete_withdrawal_data_trigger;

use dpp::system_data_contracts::feature_flags_contract::document_types::update_consensus_params;
use dpp::system_data_contracts::withdrawals_contract::document_types::withdrawal;
use dpp::system_data_contracts::{
    dashpay_contract, dpns_contract, feature_flags_contract, withdrawals_contract,
    SystemDataContract,
};
use dpp::{errors::ProtocolError, prelude::Identifier};

use super::{DataTrigger, DataTriggerKind};

/// Retrieves a list of data triggers binding with matching params.
///
/// This function gets all known data triggers which are then returned
/// as a vector of `DataTrigger` structs.
///
/// # Returns
///
/// A `Vec<DataTriggerBinding>` containing all known data triggers.
///
/// # Errors
///
/// Returns a `ProtocolError` if there was an error.
pub fn data_trigger_bindings_list_v0() -> Result<Vec<DataTriggerBindingV0>, ProtocolError> {
    let data_triggers = vec![
        DataTriggerBindingV0 {
            data_contract_id: dpns_contract::ID,
            document_type: "domain".to_string(),
            transition_action: Action::Create,
            data_trigger: Box::new(create_domain_data_trigger),
        },
        DataTriggerBindingV0 {
            data_contract_id: dpns_contract::ID,
            document_type: "domain".to_string(),
            transition_action: Action::Replace,
            data_trigger: Box::new(reject_data_trigger),
        },
        DataTriggerBindingV0 {
            data_contract_id: dpns_contract::ID,
            document_type: "domain".to_string(),
            transition_action: Action::Delete,
            data_trigger: Box::new(reject_data_trigger),
        },
        DataTriggerBindingV0 {
            data_contract_id: dpns_contract::ID,
            document_type: "preorder".to_string(),
            transition_action: Action::Delete,
            data_trigger: Box::new(reject_data_trigger),
        },
        DataTriggerBindingV0 {
            data_contract_id: dpns_contract::ID,
            document_type: "preorder".to_string(),
            transition_action: Action::Delete,
            data_trigger: Box::new(reject_data_trigger),
        },
        DataTriggerBindingV0 {
            data_contract_id: dashpay_contract::ID,
            document_type: "contactRequest".to_string(),
            transition_action: Action::Create,
            data_trigger: Box::new(create_contact_request_data_trigger),
        },
        DataTriggerBindingV0 {
            data_contract_id: dashpay_contract::ID,
            document_type: "contactRequest".to_string(),
            transition_action: Action::Replace,
            data_trigger: Box::new(reject_data_trigger),
        },
        DataTriggerBindingV0 {
            data_contract_id: dashpay_contract::ID,
            document_type: "contactRequest".to_string(),
            data_trigger_kind: DataTriggerKind::DataTriggerReject,
            data_trigger: Box::new(reject_data_trigger),
        },
        DataTriggerBindingV0 {
            data_contract_id: dashpay_contract::ID,
            document_type: update_consensus_params::NAME.to_string(),
            transition_action: Action::Create,
            data_trigger_kind: DataTriggerKind::CrateFeatureFlag,
            data_trigger: Box::new(create_feature_flag_data_trigger),
        },
        DataTriggerBindingV0 {
            data_contract_id: SystemDataContract::FeatureFlags.id(),
            document_type: update_consensus_params::NAME.to_string(),
            transition_action: Action::Replace,
            data_trigger: Box::new(reject_data_trigger),
        },
        DataTriggerBindingV0 {
            data_contract_id: SystemDataContract::FeatureFlags.id(),
            document_type: update_consensus_params::NAME.to_string(),
            transition_action: Action::Delete,
            data_trigger: Box::new(reject_data_trigger),
        },
        DataTriggerBindingV0 {
            data_contract_id: SystemDataContract::MasternodeRewards.id(),
            document_type: update_consensus_params::NAME.to_string(),
            transition_action: Action::Create,
            data_trigger: Box::new(create_masternode_reward_shares_data_trigger),
        },
        DataTriggerBindingV0 {
            data_contract_id: SystemDataContract::MasternodeRewards.id(),
            document_type: "rewardShare".to_string(),
            transition_action: Action::Replace,
            data_trigger: Box::new(create_masternode_reward_shares_data_trigger),
        },
        DataTriggerBindingV0 {
            data_contract_id: SystemDataContract::Withdrawals.id(),
            document_type: withdrawal::NAME.to_string(),
            transition_action: Action::Create,
            data_trigger: Box::new(reject_data_trigger),
        },
        DataTriggerBindingV0 {
            data_contract_id: SystemDataContract::Withdrawals.id(),
            document_type: withdrawal::NAME.to_string(),
            transition_action: Action::Replace,
            data_trigger: Box::new(reject_data_trigger),
        },
        DataTriggerBindingV0 {
            data_contract_id: SystemDataContract::Withdrawals.id(),
            document_type: withdrawal::NAME.to_string(),
            transition_action: Action::Delete,
            data_trigger: Box::new(delete_withdrawal_data_trigger),
        },
    ];

    Ok(data_triggers)
}
