use crate::execution::validation::state_transition::documents_batch::data_triggers::triggers::dashpay::create_contact_request_data_trigger;
use crate::execution::validation::state_transition::documents_batch::data_triggers::triggers::dpns::create_domain_data_trigger;
use crate::execution::validation::state_transition::documents_batch::data_triggers::triggers::feature_flags::create_feature_flag_data_trigger;
use crate::execution::validation::state_transition::documents_batch::data_triggers::triggers::reject::reject_data_trigger;
use crate::execution::validation::state_transition::documents_batch::data_triggers::triggers::reward_share::create_masternode_reward_shares_data_trigger;
use crate::execution::validation::state_transition::documents_batch::data_triggers::triggers::withdrawals::delete_withdrawal_data_trigger;
use crate::execution::validation::state_transition::documents_batch::data_triggers::bindings::data_trigger_binding::DataTriggerBindingV0;

use dpp::errors::ProtocolError;
use dpp::system_data_contracts::feature_flags_contract::v1::document_types::update_consensus_params;
use dpp::system_data_contracts::withdrawals_contract::v1::document_types::withdrawal;
use dpp::system_data_contracts::{dashpay_contract, dpns_contract, SystemDataContract};
use drive::state_transition_action::document::documents_batch::document_transition::DocumentTransitionActionType;

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
            transition_action_type: DocumentTransitionActionType::Create,
            data_trigger: create_domain_data_trigger,
        },
        DataTriggerBindingV0 {
            data_contract_id: dpns_contract::ID,
            document_type: "domain".to_string(),
            transition_action_type: DocumentTransitionActionType::Replace,
            data_trigger: reject_data_trigger,
        },
        DataTriggerBindingV0 {
            data_contract_id: dpns_contract::ID,
            document_type: "domain".to_string(),
            transition_action_type: DocumentTransitionActionType::Delete,
            data_trigger: reject_data_trigger,
        },
        DataTriggerBindingV0 {
            data_contract_id: dpns_contract::ID,
            document_type: "preorder".to_string(),
            transition_action_type: DocumentTransitionActionType::Delete,
            data_trigger: reject_data_trigger,
        },
        DataTriggerBindingV0 {
            data_contract_id: dpns_contract::ID,
            document_type: "preorder".to_string(),
            transition_action_type: DocumentTransitionActionType::Delete,
            data_trigger: reject_data_trigger,
        },
        DataTriggerBindingV0 {
            data_contract_id: dashpay_contract::ID,
            document_type: "contactRequest".to_string(),
            transition_action_type: DocumentTransitionActionType::Create,
            data_trigger: create_contact_request_data_trigger,
        },
        DataTriggerBindingV0 {
            data_contract_id: dashpay_contract::ID,
            document_type: "contactRequest".to_string(),
            transition_action_type: DocumentTransitionActionType::Replace,
            data_trigger: reject_data_trigger,
        },
        DataTriggerBindingV0 {
            data_contract_id: dashpay_contract::ID,
            document_type: "contactRequest".to_string(),
            transition_action_type: DocumentTransitionActionType::Delete,
            data_trigger: reject_data_trigger,
        },
        DataTriggerBindingV0 {
            data_contract_id: dashpay_contract::ID,
            document_type: update_consensus_params::NAME.to_string(),
            transition_action_type: DocumentTransitionActionType::Create,
            data_trigger: create_feature_flag_data_trigger,
        },
        DataTriggerBindingV0 {
            data_contract_id: SystemDataContract::FeatureFlags.id(),
            document_type: update_consensus_params::NAME.to_string(),
            transition_action_type: DocumentTransitionActionType::Replace,
            data_trigger: reject_data_trigger,
        },
        DataTriggerBindingV0 {
            data_contract_id: SystemDataContract::FeatureFlags.id(),
            document_type: update_consensus_params::NAME.to_string(),
            transition_action_type: DocumentTransitionActionType::Delete,
            data_trigger: reject_data_trigger,
        },
        DataTriggerBindingV0 {
            data_contract_id: SystemDataContract::MasternodeRewards.id(),
            document_type: update_consensus_params::NAME.to_string(),
            transition_action_type: DocumentTransitionActionType::Create,
            data_trigger: create_masternode_reward_shares_data_trigger,
        },
        DataTriggerBindingV0 {
            data_contract_id: SystemDataContract::MasternodeRewards.id(),
            document_type: "rewardShare".to_string(),
            transition_action_type: DocumentTransitionActionType::Replace,
            data_trigger: create_masternode_reward_shares_data_trigger,
        },
        DataTriggerBindingV0 {
            data_contract_id: SystemDataContract::Withdrawals.id(),
            document_type: withdrawal::NAME.to_string(),
            transition_action_type: DocumentTransitionActionType::Create,
            data_trigger: reject_data_trigger,
        },
        DataTriggerBindingV0 {
            data_contract_id: SystemDataContract::Withdrawals.id(),
            document_type: withdrawal::NAME.to_string(),
            transition_action_type: DocumentTransitionActionType::Replace,
            data_trigger: reject_data_trigger,
        },
        DataTriggerBindingV0 {
            data_contract_id: SystemDataContract::Withdrawals.id(),
            document_type: withdrawal::NAME.to_string(),
            transition_action_type: DocumentTransitionActionType::Delete,
            data_trigger: delete_withdrawal_data_trigger,
        },
    ];

    Ok(data_triggers)
}
