use crate::error::Error;
use crate::execution::validation::data_trigger::triggers::dashpay::create_contact_request_data_trigger;
use crate::execution::validation::data_trigger::triggers::dpns::create_domain_data_trigger;
use crate::execution::validation::data_trigger::triggers::feature_flags::create_feature_flag_data_trigger;
use crate::execution::validation::data_trigger::triggers::reject_data_trigger::reject_data_trigger;
use crate::execution::validation::data_trigger::triggers::reward_share::create_masternode_reward_shares_data_trigger;
use crate::execution::validation::data_trigger::triggers::withdrawals::delete_withdrawal_data_trigger;
use crate::execution::validation::data_trigger::{
    DataTriggerExecutionContext, DataTriggerExecutionResult,
};
use dpp::consensus::state::data_trigger::data_trigger_error::DataTriggerActionError;
use dpp::contracts::{
    dashpay_contract, dpns_contract, feature_flags_contract, masternode_reward_shares_contract,
    withdrawals_contract,
};
use dpp::identifier::Identifier;
use dpp::platform_value::string_encoding::Encoding;
use dpp::state_transition_action::document::documents_batch::document_transition::DocumentTransitionAction;
use dpp::version::PlatformVersion;
use dpp::{get_from_transition_action, ProtocolError};
use std::ops::Deref;

type DataTrigger = Box<
    dyn Fn(
        &DocumentTransitionAction,
        &DataTriggerExecutionContext<'_>,
        &PlatformVersion,
    ) -> Result<DataTriggerExecutionResult, Error>,
>;

/// A struct representing a data trigger on the blockchain.
///
/// The `DataTrigger` struct contains information about a data trigger, including the data contract ID, the document
/// type that the trigger handles, the kind of trigger, the action that triggered the trigger, and an optional
/// identifier for the top-level identity associated with the document.
#[derive(Default, Clone)]
pub struct DataTriggerBinding {
    /// The identifier of the data contract associated with the trigger.
    pub data_contract_id: Identifier,
    /// The type of document that the trigger handles.
    pub document_type: String,
    /// The kind of data trigger.
    pub data_trigger: DataTrigger,
    /// The action that triggered the trigger.
    pub transition_action: Action,
}

impl DataTriggerBinding {
    /// Checks whether the data trigger matches the specified data contract ID, document type, and action.
    ///
    /// This function compares the fields of the `DataTrigger` struct with the specified data contract ID, document type,
    /// and action to determine whether the trigger matches. It returns `true` if the trigger matches and `false` otherwise.
    ///
    /// # Arguments
    ///
    /// * `data_contract_id` - A reference to the data contract ID to match.
    /// * `document_type` - A reference to the document type to match.
    /// * `transition_action` - The action to match.
    ///
    /// # Returns
    ///
    /// A boolean value indicating whether the trigger matches the specified data contract ID, document type, and action.
    pub fn is_matching(
        &self,
        data_contract_id: &Identifier,
        document_type: &str,
        transition_action: Action,
    ) -> bool {
        &self.data_contract_id == data_contract_id
            && self.document_type == document_type
            && self.transition_action == transition_action
    }

    /// Executes the data trigger using the specified document transition and execution context.
    ///
    /// This function executes the data trigger using the specified `DocumentTransitionAction` and
    /// `DataTriggerExecutionContext`. It calls the `execute_trigger` function to perform the trigger
    /// execution, passing in the trigger kind, document transition, execution context, and top-level
    /// identity. It then returns a `DataTriggerExecutionResult` containing either a successful result or
    /// a `DataTriggerActionError`, indicating the failure of the trigger.
    ///
    /// # Arguments
    ///
    /// * `document_transition` - A reference to the document transition that triggered the data trigger.
    /// * `context` - A reference to the data trigger execution context.
    ///
    /// # Returns
    ///
    /// A `DataTriggerExecutionResult` containing either a successful result or a `DataTriggerActionError`,
    /// indicating the failure of the trigger.
    pub fn execute(
        &self,
        document_transition: &DocumentTransitionAction,
        context: &DataTriggerExecutionContext<'_>,
        platform_version: &PlatformVersion,
    ) -> DataTriggerExecutionResult {
        let mut result = DataTriggerExecutionResult::default();

        match self.data_trigger(document_transition, context, platform_version) {
            Err(err) => {
                let consensus_error = DataTriggerActionError::DataTriggerExecutionError {
                    // TODO remove the clone
                    data_contract_id: context.data_contract.id.to_owned(),
                    document_transition_id: *get_from_transition_action!(document_transition, id),
                    message: err.to_string(),
                    execution_error: err.to_string(),
                    document_transition: None,
                    owner_id: None,
                };

                result.add_error(consensus_error);

                result
            }

            Ok(execution_result) => execution_result,
        }
    }
}
/// Retrieves a list of all known data triggers with matching params.
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
pub fn data_trigger_bindings() -> Result<Vec<DataTriggerBinding>, ProtocolError> {
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

    let reject_data_trigger_box = Box::new(reject_data_trigger);

    let data_triggers = vec![
        DataTriggerBinding {
            data_contract_id: dpns_data_contract_id,
            document_type: "domain".to_string(),
            transition_action: Action::Create,
            data_trigger: Box::new(create_domain_data_trigger),
        },
        DataTriggerBinding {
            data_contract_id: dpns_data_contract_id,
            document_type: "domain".to_string(),
            transition_action: Action::Replace,
            data_trigger: reject_data_trigger_box,
        },
        DataTriggerBinding {
            data_contract_id: dpns_data_contract_id,
            document_type: "domain".to_string(),
            transition_action: Action::Delete,
            data_trigger: reject_data_trigger_box,
        },
        DataTriggerBinding {
            data_contract_id: dpns_data_contract_id,
            document_type: "preorder".to_string(),
            transition_action: Action::Delete,
            data_trigger: reject_data_trigger_box,
        },
        DataTriggerBinding {
            data_contract_id: dpns_data_contract_id,
            document_type: "preorder".to_string(),
            transition_action: Action::Delete,
            data_trigger: reject_data_trigger_box,
        },
        DataTriggerBinding {
            data_contract_id: dashpay_data_contract_id,
            document_type: "contactRequest".to_string(),
            transition_action: Action::Create,
            data_trigger: Box::new(create_contact_request_data_trigger),
        },
        DataTriggerBinding {
            data_contract_id: dashpay_data_contract_id,
            document_type: "contactRequest".to_string(),
            transition_action: Action::Replace,
            data_trigger: reject_data_trigger_box,
        },
        DataTriggerBinding {
            data_contract_id: dashpay_data_contract_id,
            document_type: "contactRequest".to_string(),
            transition_action: Action::Delete,
            data_trigger: reject_data_trigger_box,
        },
        DataTriggerBinding {
            data_contract_id: feature_flags_data_contract_id,
            document_type: feature_flags_contract::types::UPDATE_CONSENSUS_PARAMS.to_string(),
            transition_action: Action::Create,
            data_trigger: Box::new(create_feature_flag_data_trigger),
        },
        DataTriggerBinding {
            data_contract_id: feature_flags_data_contract_id,
            document_type: feature_flags_contract::types::UPDATE_CONSENSUS_PARAMS.to_string(),
            transition_action: Action::Replace,
            data_trigger: reject_data_trigger_box,
        },
        DataTriggerBinding {
            data_contract_id: feature_flags_data_contract_id,
            document_type: feature_flags_contract::types::UPDATE_CONSENSUS_PARAMS.to_string(),
            transition_action: Action::Delete,
            data_trigger: reject_data_trigger_box,
        },
        DataTriggerBinding {
            data_contract_id: master_node_reward_shares_contract_id,
            document_type: feature_flags_contract::types::UPDATE_CONSENSUS_PARAMS.to_string(),
            transition_action: Action::Create,
            data_trigger: Box::new(create_masternode_reward_shares_data_trigger),
        },
        DataTriggerBinding {
            data_contract_id: master_node_reward_shares_contract_id,
            document_type: "rewardShare".to_string(),
            transition_action: Action::Replace,
            data_trigger: Box::new(create_masternode_reward_shares_data_trigger),
        },
        DataTriggerBinding {
            data_contract_id: *withdrawals_contract_id,
            document_type: withdrawals_contract::document_types::WITHDRAWAL.to_string(),
            transition_action: Action::Create,
            data_trigger: reject_data_trigger_box,
        },
        DataTriggerBinding {
            data_contract_id: *withdrawals_contract_id,
            document_type: withdrawals_contract::document_types::WITHDRAWAL.to_string(),
            transition_action: Action::Replace,
            data_trigger: reject_data_trigger_box,
        },
        DataTriggerBinding {
            data_contract_id: *withdrawals_contract_id,
            document_type: withdrawals_contract::document_types::WITHDRAWAL.to_string(),
            transition_action: Action::Delete,
            data_trigger: Box::new(delete_withdrawal_data_trigger),
        },
    ];

    Ok(data_triggers)
}
