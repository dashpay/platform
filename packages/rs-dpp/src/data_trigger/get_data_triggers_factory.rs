use std::vec;

use crate::{
    contracts::{
        dashpay_contract, dpns_contract, feature_flags_contract, masternode_reward_shares_contract,
    },
    document::document_transition::Action,
    errors::ProtocolError,
    prelude::Identifier,
    state_repository::StateRepositoryLike,
    util::string_encoding::Encoding,
};

use super::{DataTrigger, DataTriggerCode};

/// returns Date Triggers filtered out by dataContractId, documentType, transactionAction
pub fn get_data_triggers<'a, SR>(
    data_contract_id: &'a Identifier,
    document_type: &'a str,
    transition_action: Action,
) -> Result<Vec<DataTrigger>, ProtocolError>
where
    SR: StateRepositoryLike,
{
    let data_triggers = get_data_triggers_factory()?;
    Ok(data_triggers
        .into_iter()
        .filter(|dt| {
            dt.is_matching_trigger_for_data(data_contract_id, document_type, transition_action)
        })
        .collect())
}

pub fn get_data_triggers_factory() -> Result<Vec<DataTrigger>, ProtocolError> {
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

    let data_triggers = vec![
        DataTrigger {
            data_contract_id: dpns_data_contract_id.clone(),
            document_type: "domain".to_string(),
            transition_action: Action::Create,
            data_trigger_code: DataTriggerCode::DataTriggerCreateDomain,
            top_level_identity: Some(dpns_owner_id),
        },
        DataTrigger {
            data_contract_id: dpns_data_contract_id.clone(),
            document_type: "domain".to_string(),
            transition_action: Action::Replace,
            data_trigger_code: DataTriggerCode::DataTriggerReject,
            top_level_identity: None,
        },
        DataTrigger {
            data_contract_id: dpns_data_contract_id.clone(),
            document_type: "domain".to_string(),
            transition_action: Action::Delete,
            data_trigger_code: DataTriggerCode::DataTriggerReject,
            top_level_identity: None,
        },
        DataTrigger {
            data_contract_id: dpns_data_contract_id.clone(),
            document_type: "preorder".to_string(),
            transition_action: Action::Delete,
            data_trigger_code: DataTriggerCode::DataTriggerReject,
            top_level_identity: None,
        },
        DataTrigger {
            data_contract_id: dpns_data_contract_id,
            document_type: "preorder".to_string(),
            transition_action: Action::Delete,
            data_trigger_code: DataTriggerCode::DataTriggerReject,
            top_level_identity: None,
        },
        DataTrigger {
            data_contract_id: dashpay_data_contract_id.clone(),
            document_type: "contactRequest".to_string(),
            transition_action: Action::Create,
            data_trigger_code: DataTriggerCode::CreateDataContractRequest,
            top_level_identity: None,
        },
        DataTrigger {
            data_contract_id: dashpay_data_contract_id.clone(),
            document_type: "contactRequest".to_string(),
            transition_action: Action::Replace,
            data_trigger_code: DataTriggerCode::DataTriggerReject,
            top_level_identity: None,
        },
        DataTrigger {
            data_contract_id: dashpay_data_contract_id,
            document_type: "contactRequest".to_string(),
            transition_action: Action::Replace,
            data_trigger_code: DataTriggerCode::DataTriggerReject,
            top_level_identity: None,
        },
        DataTrigger {
            data_contract_id: feature_flags_data_contract_id.clone(),
            document_type: feature_flags_contract::types::UPDATE_CONSENSUS_PARAMS.to_string(),
            transition_action: Action::Create,
            data_trigger_code: DataTriggerCode::CrateFeatureFlag,
            top_level_identity: Some(feature_flags_owner_id),
        },
        DataTrigger {
            data_contract_id: feature_flags_data_contract_id.clone(),
            document_type: feature_flags_contract::types::UPDATE_CONSENSUS_PARAMS.to_string(),
            transition_action: Action::Replace,
            data_trigger_code: DataTriggerCode::DataTriggerReject,
            top_level_identity: None,
        },
        DataTrigger {
            data_contract_id: feature_flags_data_contract_id,
            document_type: feature_flags_contract::types::UPDATE_CONSENSUS_PARAMS.to_string(),
            transition_action: Action::Delete,
            data_trigger_code: DataTriggerCode::DataTriggerReject,
            top_level_identity: None,
        },
        DataTrigger {
            data_contract_id: master_node_reward_shares_contract_id.clone(),
            document_type: feature_flags_contract::types::UPDATE_CONSENSUS_PARAMS.to_string(),
            transition_action: Action::Create,
            data_trigger_code: DataTriggerCode::CreateDataContractRequest,
            top_level_identity: None,
        },
        DataTrigger {
            data_contract_id: master_node_reward_shares_contract_id,
            document_type: feature_flags_contract::types::UPDATE_CONSENSUS_PARAMS.to_string(),
            transition_action: Action::Replace,
            data_trigger_code: DataTriggerCode::CreateDataContractRequest,
            top_level_identity: None,
        },
    ];
    Ok(data_triggers)
}
