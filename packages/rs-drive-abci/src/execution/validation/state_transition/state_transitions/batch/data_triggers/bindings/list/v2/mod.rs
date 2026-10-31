use crate::execution::validation::state_transition::batch::data_triggers::bindings::data_trigger_binding::DataTriggerBindingV0;
use crate::execution::validation::state_transition::batch::data_triggers::triggers::dashpay::{create_contact_request_data_trigger, validate_profile_payment_addresses_data_trigger};
use crate::execution::validation::state_transition::batch::data_triggers::triggers::dpns::create_domain_data_trigger;
use crate::execution::validation::state_transition::batch::data_triggers::triggers::reject::reject_data_trigger;
use crate::execution::validation::state_transition::batch::data_triggers::triggers::withdrawals::delete_withdrawal_data_trigger;

use dpp::errors::ProtocolError;
use dpp::system_data_contracts::withdrawals_contract::v1::document_types::withdrawal;
use dpp::system_data_contracts::{dashpay_contract, dpns_contract, SystemDataContract};
use drive::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionActionType;

/// Retrieves a list of data triggers binding with matching params.
///
/// This function gets all known data triggers which are then returned
/// as a vector of `DataTrigger` structs.
///
/// v2 (PROTOCOL_VERSION_14): DashPay `profile` documents gain Create and
/// Replace triggers enforcing the DIP-33 payment-address type byte
/// (`0x00` P2PKH / `0x01` P2SH) that the schema vocabulary cannot express.
/// Everything else is unchanged from v1.
///
/// # Returns
///
/// A `Vec<DataTriggerBinding>` containing all known data triggers.
///
/// # Errors
///
/// Returns a `ProtocolError` if there was an error.
#[inline(always)]
pub(super) fn data_trigger_bindings_list_v2() -> Result<Vec<DataTriggerBindingV0>, ProtocolError> {
    let data_triggers = vec![
        DataTriggerBindingV0 {
            data_contract_id: dpns_contract::ID,
            document_type: "domain".to_string(),
            transition_action_type: DocumentTransitionActionType::Create,
            data_trigger: create_domain_data_trigger,
        },
        // Domain documents can never be modified or deleted, but since
        // protocol version 13 they can be transferred and sold
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
            data_contract_id: dashpay_contract::ID,
            document_type: "contactRequest".to_string(),
            transition_action_type: DocumentTransitionActionType::Create,
            data_trigger: create_contact_request_data_trigger,
        },
        // DIP-33 payment address fields must carry a supported type byte
        DataTriggerBindingV0 {
            data_contract_id: dashpay_contract::ID,
            document_type: "profile".to_string(),
            transition_action_type: DocumentTransitionActionType::Create,
            data_trigger: validate_profile_payment_addresses_data_trigger,
        },
        DataTriggerBindingV0 {
            data_contract_id: dashpay_contract::ID,
            document_type: "profile".to_string(),
            transition_action_type: DocumentTransitionActionType::Replace,
            data_trigger: validate_profile_payment_addresses_data_trigger,
        },
        // Only masternodes will be able to update it
        DataTriggerBindingV0 {
            data_contract_id: SystemDataContract::MasternodeRewards.id(),
            document_type: "rewardShare".to_string(),
            transition_action_type: DocumentTransitionActionType::Create,
            data_trigger: reject_data_trigger,
        },
        // Only masternodes will be able to update it
        DataTriggerBindingV0 {
            data_contract_id: SystemDataContract::MasternodeRewards.id(),
            document_type: "rewardShare".to_string(),
            transition_action_type: DocumentTransitionActionType::Replace,
            data_trigger: reject_data_trigger,
        },
        // Only masternodes will be able to update it
        DataTriggerBindingV0 {
            data_contract_id: SystemDataContract::MasternodeRewards.id(),
            document_type: "rewardShare".to_string(),
            transition_action_type: DocumentTransitionActionType::Delete,
            data_trigger: reject_data_trigger,
        },
        // We can't use mutability flag otherwise documents won't have revision
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
