use std::ops::Deref;
use dpp::contracts::{dashpay_contract, dpns_contract, feature_flags_contract, masternode_reward_shares_contract, withdrawals_contract};
use dpp::identifier::Identifier;
use dpp::platform_value::string_encoding::Encoding;
use dpp::ProtocolError;
use crate::execution::validation::state_transition::documents_batch::data_triggers::triggers::dashpay::create_contact_request_data_trigger;
use crate::execution::validation::state_transition::documents_batch::data_triggers::triggers::dpns::create_domain_data_trigger;
use crate::execution::validation::state_transition::documents_batch::data_triggers::triggers::feature_flags::create_feature_flag_data_trigger;
use crate::execution::validation::state_transition::documents_batch::data_triggers::triggers::reject::reject_data_trigger;
use crate::execution::validation::state_transition::documents_batch::data_triggers::triggers::reward_share::create_masternode_reward_shares_data_trigger;
use crate::execution::validation::state_transition::documents_batch::data_triggers::triggers::withdrawals::delete_withdrawal_data_trigger;

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
    let dpns_data_contract_id =
        Identifier::from_string(&dpns_contract::system_ids().contract_id, Encoding::Base58)?;

    let dashpay_data_contract_id = Identifier::from_string(
        &dashpay_contract::system_ids().contract_id,
        Encoding::Base58,
    )?;
    let feature_flags_data_contract_id = Identifier::from_string(
        &feature_flags_contract::system_ids().contract_id,
        Encoding::Base58,
    )?;
    let master_node_reward_shares_contract_id = Identifier::from_string(
        &masternode_reward_shares_contract::system_ids().contract_id,
        Encoding::Base58,
    )?;
    let withdrawals_contract_id = withdrawals_contract::CONTRACT_ID.deref();

    let data_triggers = vec![
        DataTriggerBindingV0 {
            data_contract_id: dpns_data_contract_id,
            document_type: "domain".to_string(),
            transition_action: Action::Create,
            data_trigger: Box::new(create_domain_data_trigger),
        },
        DataTriggerBindingV0 {
            data_contract_id: dpns_data_contract_id,
            document_type: "domain".to_string(),
            transition_action: Action::Replace,
            data_trigger: Box::new(reject_data_trigger),
        },
        DataTriggerBindingV0 {
            data_contract_id: dpns_data_contract_id,
            document_type: "domain".to_string(),
            transition_action: Action::Delete,
            data_trigger: Box::new(reject_data_trigger),
        },
        DataTriggerBindingV0 {
            data_contract_id: dpns_data_contract_id,
            document_type: "preorder".to_string(),
            transition_action: Action::Delete,
            data_trigger: Box::new(reject_data_trigger),
        },
        DataTriggerBindingV0 {
            data_contract_id: dpns_data_contract_id,
            document_type: "preorder".to_string(),
            transition_action: Action::Delete,
            data_trigger: Box::new(reject_data_trigger),
        },
        DataTriggerBindingV0 {
            data_contract_id: dashpay_data_contract_id,
            document_type: "contactRequest".to_string(),
            transition_action: Action::Create,
            data_trigger: Box::new(create_contact_request_data_trigger),
        },
        DataTriggerBindingV0 {
            data_contract_id: dashpay_data_contract_id,
            document_type: "contactRequest".to_string(),
            transition_action: Action::Replace,
            data_trigger: Box::new(reject_data_trigger),
        },
        DataTriggerBindingV0 {
            data_contract_id: dashpay_data_contract_id,
            document_type: "contactRequest".to_string(),
            transition_action: Action::Delete,
            data_trigger: Box::new(reject_data_trigger),
        },
        DataTriggerBindingV0 {
            data_contract_id: feature_flags_data_contract_id,
            document_type: feature_flags_contract::types::UPDATE_CONSENSUS_PARAMS.to_string(),
            transition_action: Action::Create,
            data_trigger: Box::new(create_feature_flag_data_trigger),
        },
        DataTriggerBindingV0 {
            data_contract_id: feature_flags_data_contract_id,
            document_type: feature_flags_contract::types::UPDATE_CONSENSUS_PARAMS.to_string(),
            transition_action: Action::Replace,
            data_trigger: Box::new(reject_data_trigger),
        },
        DataTriggerBindingV0 {
            data_contract_id: feature_flags_data_contract_id,
            document_type: feature_flags_contract::types::UPDATE_CONSENSUS_PARAMS.to_string(),
            transition_action: Action::Delete,
            data_trigger: Box::new(reject_data_trigger),
        },
        DataTriggerBindingV0 {
            data_contract_id: master_node_reward_shares_contract_id,
            document_type: feature_flags_contract::types::UPDATE_CONSENSUS_PARAMS.to_string(),
            transition_action: Action::Create,
            data_trigger: Box::new(create_masternode_reward_shares_data_trigger),
        },
        DataTriggerBindingV0 {
            data_contract_id: master_node_reward_shares_contract_id,
            document_type: "rewardShare".to_string(),
            transition_action: Action::Replace,
            data_trigger: Box::new(create_masternode_reward_shares_data_trigger),
        },
        DataTriggerBindingV0 {
            data_contract_id: *withdrawals_contract_id,
            document_type: withdrawals_contract::document_types::WITHDRAWAL.to_string(),
            transition_action: Action::Create,
            data_trigger: Box::new(reject_data_trigger),
        },
        DataTriggerBindingV0 {
            data_contract_id: *withdrawals_contract_id,
            document_type: withdrawals_contract::document_types::WITHDRAWAL.to_string(),
            transition_action: Action::Replace,
            data_trigger: Box::new(reject_data_trigger),
        },
        DataTriggerBindingV0 {
            data_contract_id: *withdrawals_contract_id,
            document_type: withdrawals_contract::document_types::WITHDRAWAL.to_string(),
            transition_action: Action::Delete,
            data_trigger: Box::new(delete_withdrawal_data_trigger),
        },
    ];

    Ok(data_triggers)
}
