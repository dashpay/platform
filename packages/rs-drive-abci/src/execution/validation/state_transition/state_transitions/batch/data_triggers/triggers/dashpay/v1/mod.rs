//! PROTOCOL_VERSION_12+ version of the DashPay contact-request trigger.
//!
//! Mirrors `create_contact_request_data_trigger_v0` but uses
//! `fetch_identity_balance_with_costs` (instead of `fetch_identity_balance`)
//! and bills the returned `FeeResult` via
//! `context.state_transition_execution_context.add_operation(...)`.

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContextMethodsV0;
use crate::execution::validation::state_transition::batch::data_triggers::{
    DataTriggerExecutionContext, DataTriggerExecutionResult,
};
use dpp::consensus::state::data_trigger::data_trigger_condition_error::DataTriggerConditionError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use dpp::system_data_contracts::dashpay_contract::v1::document_types::contact_request::properties::TO_USER_ID;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::DocumentCreateTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;

#[inline(always)]
pub(super) fn create_contact_request_data_trigger_v1(
    document_transition: &DocumentTransitionAction,
    context: &mut DataTriggerExecutionContext<'_>,
    platform_version: &PlatformVersion,
) -> Result<DataTriggerExecutionResult, Error> {
    let data_contract_fetch_info = document_transition.base().data_contract_fetch_info();
    let data_contract = &data_contract_fetch_info.contract;
    let mut result = DataTriggerExecutionResult::default();
    let is_dry_run = context.state_transition_execution_context.in_dry_run();
    let owner_id = context.owner_id;

    let DocumentTransitionAction::CreateAction(document_create_transition) = document_transition
    else {
        return Err(Error::Execution(ExecutionError::DataTriggerExecutionError(
            format!(
                "the Document Transition {} isn't 'CREATE",
                document_transition.base().id()
            ),
        )));
    };

    let data = document_create_transition.data();

    let to_user_id = data
        .get_identifier(TO_USER_ID)
        .map_err(ProtocolError::ValueError)?;

    // You shouldn't create a contract request to yourself
    if !is_dry_run && owner_id == &to_user_id {
        let err = DataTriggerConditionError::new(
            data_contract.id(),
            document_transition.base().id(),
            format!("Identity {to_user_id} must not be equal to owner id"),
        );

        result.add_error(err);

        return Ok(result);
    }

    // Diff vs `_v0` (recipient identity existence check):
    //
    //   _v0: `.fetch_identity_balance(to_user_id, transaction, pv)?`
    //        — returns just the balance (no cost info). No way to
    //        bill the grovedb read; the explicit `TODO: Calculate fee
    //        operations` comment marked the gap.
    //
    //   _v1: `.fetch_identity_balance_with_costs(to_user_id,
    //        block_info, apply=true, transaction, pv)?` — returns
    //        `(Option<Credits>, FeeResult)`. Bill the FeeResult via
    //        `add_operation`.
    //
    // Why the change: contact-request creates would do a grovedb
    // identity-balance lookup for free on PV11. `apply: true` matches
    // `_v0`'s stateful query (not the stateless estimated-cost path)
    // so the balance value returned is byte-identical to `_v0`.
    let (to_identity, balance_fee_result) =
        context.platform.drive.fetch_identity_balance_with_costs(
            to_user_id.to_buffer(),
            context.block_info,
            true,
            context.transaction,
            platform_version,
        )?;
    context.state_transition_execution_context.add_operation(
        ValidationOperation::PrecalculatedOperation(balance_fee_result),
    );

    if !is_dry_run && to_identity.is_none() {
        let err = DataTriggerConditionError::new(
            data_contract.id(),
            document_create_transition.base().id(),
            format!("Identity {to_user_id} doesn't exist"),
        );

        result.add_error(err);

        return Ok(result);
    }

    Ok(result)
}
