use futures::FutureExt;
use std::vec;

use crate::{
    contracts::{
        dashpay_contract, dpns_contract, feature_flags_contract, masternode_reward_shares_contract,
    },
    data_trigger::{
        dashpay_data_triggers::create_contract_request_data_trigger,
        dpns_triggers::create_domain_data_trigger,
        feature_flags_data_triggers::create_feature_flag_data_trigger, reject_data_trigger,
        DataTriggerExecutionContext,
    },
    document::document_transition::Action,
    errors::ProtocolError,
    prelude::Identifier,
    state_repository::{SMLStoreLike, SimplifiedMNListLike, StateRepositoryLike},
    util::string_encoding::Encoding,
};

use super::DataTrigger;

macro_rules! to_boxed_data_trigger {
    ($e:expr) => {
        Box::new(
            |document_transition,
             data_trigger_exec_context: &DataTriggerExecutionContext<SR, S, L>,
             top_level_identity| {
                $e(
                    document_transition,
                    data_trigger_exec_context,
                    top_level_identity,
                )
                .boxed_local()
            },
        )
    };
}

pub fn get_data_triggers<'a, SR, S, L>(
    data_contract_id: &Identifier,
    document_type: &str,
    transition_action: Action,
) -> Result<Vec<DataTrigger<'a, SR, S, L>>, ProtocolError>
where
    L: SimplifiedMNListLike,
    S: SMLStoreLike<L>,
    SR: StateRepositoryLike<S, L>,
{
    let data_triggers = get_data_triggers_factory()?;
    Ok(data_triggers
        .into_iter()
        .filter(|dt| {
            dt.is_matching_trigger_for_data(data_contract_id, document_type, transition_action)
        })
        .collect())
}

pub fn get_data_triggers_factory<'a, SR, S, L>(
) -> Result<Vec<DataTrigger<'a, SR, S, L>>, ProtocolError>
where
    L: SimplifiedMNListLike,
    S: SMLStoreLike<L>,
    SR: StateRepositoryLike<S, L>,
{
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

    // create_domain_data_trigger(document_transition, context, top_level_identity)

    let data_triggers = vec![
        DataTrigger {
            data_contract_id: dpns_data_contract_id.clone(),
            document_type: "domain".to_string(),
            transition_action: Action::Create,
            trigger: to_boxed_data_trigger!(create_domain_data_trigger),
            top_level_identity: Some(dpns_owner_id),
        },
        DataTrigger {
            data_contract_id: dpns_data_contract_id.clone(),
            document_type: "domain".to_string(),
            transition_action: Action::Replace,
            trigger: to_boxed_data_trigger!(reject_data_trigger),
            top_level_identity: None,
        },
        DataTrigger {
            data_contract_id: dpns_data_contract_id.clone(),
            document_type: "domain".to_string(),
            transition_action: Action::Delete,
            trigger: to_boxed_data_trigger!(reject_data_trigger),
            top_level_identity: None,
        },
        DataTrigger {
            data_contract_id: dpns_data_contract_id.clone(),
            document_type: "preorder".to_string(),
            transition_action: Action::Delete,
            trigger: to_boxed_data_trigger!(reject_data_trigger),
            top_level_identity: None,
        },
        DataTrigger {
            data_contract_id: dpns_data_contract_id,
            document_type: "preorder".to_string(),
            transition_action: Action::Delete,
            trigger: to_boxed_data_trigger!(reject_data_trigger),
            top_level_identity: None,
        },
        DataTrigger {
            data_contract_id: dashpay_data_contract_id.clone(),
            document_type: "contactRequest".to_string(),
            transition_action: Action::Create,
            trigger: to_boxed_data_trigger!(create_contract_request_data_trigger),
            top_level_identity: None,
        },
        DataTrigger {
            data_contract_id: dashpay_data_contract_id.clone(),
            document_type: "contactRequest".to_string(),
            transition_action: Action::Replace,
            trigger: to_boxed_data_trigger!(reject_data_trigger),
            top_level_identity: None,
        },
        DataTrigger {
            data_contract_id: dashpay_data_contract_id,
            document_type: "contactRequest".to_string(),
            transition_action: Action::Replace,
            trigger: to_boxed_data_trigger!(reject_data_trigger),
            top_level_identity: None,
        },
        DataTrigger {
            data_contract_id: feature_flags_data_contract_id.clone(),
            document_type: feature_flags_contract::types::UPDATE_CONSENSUS_PARAMS.to_string(),
            transition_action: Action::Create,
            trigger: to_boxed_data_trigger!(create_feature_flag_data_trigger),
            top_level_identity: Some(feature_flags_owner_id),
        },
        DataTrigger {
            data_contract_id: feature_flags_data_contract_id.clone(),
            document_type: feature_flags_contract::types::UPDATE_CONSENSUS_PARAMS.to_string(),
            transition_action: Action::Replace,
            trigger: to_boxed_data_trigger!(reject_data_trigger),
            top_level_identity: None,
        },
        DataTrigger {
            data_contract_id: feature_flags_data_contract_id,
            document_type: feature_flags_contract::types::UPDATE_CONSENSUS_PARAMS.to_string(),
            transition_action: Action::Delete,
            trigger: to_boxed_data_trigger!(reject_data_trigger),
            top_level_identity: None,
        },
        DataTrigger {
            data_contract_id: master_node_reward_shares_contract_id.clone(),
            document_type: feature_flags_contract::types::UPDATE_CONSENSUS_PARAMS.to_string(),
            transition_action: Action::Create,
            trigger: to_boxed_data_trigger!(create_contract_request_data_trigger),
            top_level_identity: None,
        },
        DataTrigger {
            data_contract_id: master_node_reward_shares_contract_id,
            document_type: feature_flags_contract::types::UPDATE_CONSENSUS_PARAMS.to_string(),
            transition_action: Action::Replace,
            trigger: to_boxed_data_trigger!(create_contract_request_data_trigger),
            top_level_identity: None,
        },
    ];
    Ok(data_triggers)
}

// we need a 3 elements
