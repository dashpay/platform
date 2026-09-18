use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContextMethodsV0;
use crate::execution::validation::state_transition::batch::data_triggers::{
    DataTriggerExecutionContext, DataTriggerExecutionResult,
};
use dpp::consensus::state::data_trigger::data_trigger_condition_error::DataTriggerConditionError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::document::DocumentV0Getters;
use dpp::fee::fee_result::FeeResult;
use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use dpp::platform_value::Value;
use dpp::system_data_contracts::withdrawals_contract::v1::document_types::withdrawal;
use dpp::system_data_contracts::withdrawals_contract::WithdrawalStatus;
use dpp::version::PlatformVersion;
use dpp::{document, ProtocolError};
use drive::drive::document::query::QueryDocumentsOutcomeV0Methods;
use drive::query::{DriveDocumentQuery, InternalClauses, WhereClause, WhereOperator};
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_delete_transition_action::v0::DocumentDeleteTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;
use std::collections::BTreeMap;

/// Statuses a withdrawal document may be deleted in: both are terminal, Platform never
/// touches the document again.
const DELETABLE_STATUSES: [WithdrawalStatus; 2] =
    [WithdrawalStatus::COMPLETE, WithdrawalStatus::FAILED];

/// Version 2 changes on version 1 by also allowing the deletion of withdrawals in the
/// terminal `FAILED` status the withdrawals contract v2 admits (a withdrawal Core could
/// never mine, given up on by `rebroadcast_expired_withdrawal_documents` v2). The lookup
/// and its fee accounting are those of v1.
#[inline(always)]
pub(super) fn delete_withdrawal_data_trigger_v2(
    document_transition: &DocumentTransitionAction,
    context: &mut DataTriggerExecutionContext<'_>,
    platform_version: &PlatformVersion,
) -> Result<DataTriggerExecutionResult, Error> {
    let data_contract_fetch_info = document_transition.base().data_contract_fetch_info();
    let data_contract = &data_contract_fetch_info.contract;
    let mut result = DataTriggerExecutionResult::default();

    let DocumentTransitionAction::DeleteAction(dt_delete) = document_transition else {
        return Err(Error::Execution(ExecutionError::DataTriggerExecutionError(
            format!(
                "the Document Transition {} isn't 'DELETE",
                document_transition.base().id()
            ),
        )));
    };

    let document_type = data_contract.document_type_for_name(withdrawal::NAME)?;

    let drive_query = DriveDocumentQuery {
        contract: data_contract,
        document_type,
        internal_clauses: InternalClauses {
            primary_key_in_clause: None,
            primary_key_equal_clause: Some(WhereClause {
                field: document::property_names::ID.to_string(),
                operator: WhereOperator::Equal,
                value: Value::Identifier(dt_delete.base().id().to_buffer()),
            }),
            in_clauses: Vec::new(),
            range_clause: None,
            equal_clauses: BTreeMap::default(),
        },
        offset: None,
        limit: Some(100),
        order_by: Default::default(),
        start_at: None,
        start_at_included: false,
        block_time_ms: None,
        resolved_time_ranges: vec![],
        sub_queries: vec![],
    };

    let withdrawals_outcome = context.platform.drive.query_documents(
        drive_query,
        Some(&context.block_info.epoch),
        false,
        context.transaction,
        Some(platform_version.protocol_version),
    )?;
    let query_fee = FeeResult {
        processing_fee: withdrawals_outcome.cost(),
        ..Default::default()
    };
    context
        .state_transition_execution_context
        .add_operation(ValidationOperation::PrecalculatedOperation(query_fee));
    let withdrawals = withdrawals_outcome.documents_owned();

    let Some(withdrawal) = withdrawals.first() else {
        let err = DataTriggerConditionError::new(
            data_contract.id(),
            dt_delete.base().id(),
            "Withdrawal document was not found".to_string(),
        );

        result.add_error(err);

        return Ok(result);
    };

    let status: u8 = withdrawal
        .properties()
        .get_integer(withdrawal::properties::STATUS)
        .map_err(ProtocolError::ValueError)?;

    if !DELETABLE_STATUSES
        .iter()
        .any(|deletable| *deletable as u8 == status)
    {
        let err = DataTriggerConditionError::new(
            data_contract.id(),
            dt_delete.base().id(),
            "withdrawal deletion is allowed only for COMPLETE or FAILED statuses".to_string(),
        );

        result.add_error(err);

        return Ok(result);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::types::state_transition_execution_context::v0::StateTransitionExecutionContextV0;
    use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
    use crate::platform_types::platform::PlatformStateRef;
    use crate::platform_types::platform_state::PlatformStateV0Methods;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::block_info::BlockInfo;
    use dpp::identity::core_script::CoreScript;
    use dpp::platform_value::platform_value;
    use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dpp::tests::fixtures::get_withdrawal_document_fixture;
    use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
    use dpp::withdrawal::Pooling;
    use drive::drive::contract::DataContractFetchInfo;
    use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionV0};
    use drive::state_transition_action::batch::batched_transition::document_transition::document_delete_transition_action::v0::DocumentDeleteTransitionActionV0;
    use drive::state_transition_action::batch::batched_transition::document_transition::document_delete_transition_action::DocumentDeleteTransitionAction;
    use drive::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use drive::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
    use std::sync::Arc;

    /// Stores a withdrawal document in `status` and runs the v2 delete trigger against it.
    fn run_delete_trigger_on_withdrawal_with_status(
        status: WithdrawalStatus,
    ) -> DataTriggerExecutionResult {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        let state_read_guard = platform.state.load();

        let platform_ref = PlatformStateRef {
            drive: &platform.drive,
            state: &state_read_guard,
            config: &platform.config,
        };

        let mut transition_execution_context =
            StateTransitionExecutionContext::V0(StateTransitionExecutionContextV0::default());

        let platform_version = state_read_guard
            .current_platform_version()
            .expect("should return a platform version");

        let data_contract =
            load_system_data_contract(SystemDataContract::Withdrawals, platform_version)
                .expect("to load system data contract");
        let owner_id = data_contract.owner_id();

        let document_type = data_contract
            .document_type_for_name(withdrawal::NAME)
            .expect("expected to get withdrawal document type");

        let document = get_withdrawal_document_fixture(
            &data_contract,
            owner_id,
            platform_value!({
                "amount": 1000u64,
                "coreFeePerByte": 1u32,
                "pooling": Pooling::Never as u8,
                "outputScript": CoreScript::from_bytes((0..23).collect::<Vec<u8>>()),
                "status": status as u8,
                "transactionIndex": 1u64,
                "transactionSignHeight": 93u64,
            }),
            None,
            platform_version.protocol_version,
        )
        .expect("expected withdrawal document");

        platform
            .drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&document, None)),
                        owner_id: Some(owner_id.to_buffer()),
                    },
                    contract: &data_contract,
                    document_type,
                },
                false,
                BlockInfo::genesis(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("expected to insert a document successfully");

        let document_transition = DocumentTransitionAction::DeleteAction(
            DocumentDeleteTransitionAction::V0(DocumentDeleteTransitionActionV0 {
                base: DocumentBaseTransitionAction::V0(DocumentBaseTransitionActionV0 {
                    id: document.id(),
                    identity_contract_nonce: 1,
                    document_type_name: withdrawal::NAME.to_string(),
                    data_contract: Arc::new(DataContractFetchInfo::withdrawals_contract_fixture(
                        platform_version.protocol_version,
                    )),
                    token_cost: None,
                    gas_fees_paid_by: GasFeesPaidBy::DocumentOwner,
                    contract_gas_fees_paid_by: GasFeesPaidBy::default(),
                }),
            }),
        );

        let trigger_block_info = BlockInfo::default();
        let mut data_trigger_context = DataTriggerExecutionContext {
            platform: &platform_ref,
            block_info: &trigger_block_info,
            owner_id: &owner_id,
            state_transition_execution_context: &mut transition_execution_context,
            transaction: None,
        };

        let result = delete_withdrawal_data_trigger_v2(
            &document_transition,
            &mut data_trigger_context,
            platform_version,
        )
        .expect("the execution result should be returned");

        assert!(
            !data_trigger_context
                .state_transition_execution_context
                .operations_slice()
                .is_empty(),
            "delete_withdrawal_data_trigger_v2 must account for its lookup like v1"
        );

        result
    }

    #[test]
    fn should_allow_deleting_a_failed_withdrawal() {
        let result = run_delete_trigger_on_withdrawal_with_status(WithdrawalStatus::FAILED);

        assert!(result.is_valid(), "{:?}", result.errors);
    }

    #[test]
    fn should_allow_deleting_a_complete_withdrawal() {
        let result = run_delete_trigger_on_withdrawal_with_status(WithdrawalStatus::COMPLETE);

        assert!(result.is_valid(), "{:?}", result.errors);
    }

    #[test]
    fn should_reject_deleting_a_withdrawal_that_is_still_in_flight() {
        for status in [
            WithdrawalStatus::QUEUED,
            WithdrawalStatus::POOLED,
            WithdrawalStatus::BROADCASTED,
            WithdrawalStatus::EXPIRED,
        ] {
            let result = run_delete_trigger_on_withdrawal_with_status(status);

            assert!(!result.is_valid(), "{status} must not be deletable");

            let error = result.get_error(0).expect("expected a trigger error");

            assert_eq!(
                error.to_string(),
                "withdrawal deletion is allowed only for COMPLETE or FAILED statuses"
            );
        }
    }
}
