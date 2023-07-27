///! The `withdrawals_data_triggers` module contains data triggers related to withdrawals.
use crate::error::execution::ExecutionError;
use crate::error::Error;

use crate::execution::validation::data_trigger::{
    DataTriggerExecutionContext, DataTriggerExecutionResult,
};
use dpp::consensus::state::data_trigger::data_trigger_error::DataTriggerActionError;

use dpp::data_contract::base::DataContractBaseMethodsV0;
use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use dpp::platform_value::Value;
use dpp::state_transition_action::document::documents_batch::document_transition::DocumentTransitionAction;
use dpp::system_data_contracts::withdrawal::NAME;
use dpp::system_data_contracts::withdrawals_contract;
use dpp::version::PlatformVersion;
use dpp::{get_from_transition_action, ProtocolError};
use drive::query::{DriveQuery, InternalClauses, WhereClause, WhereOperator};
use std::collections::BTreeMap;

/// Creates a data trigger for handling deletion of withdrawal documents.
///
/// The trigger is executed whenever a withdrawal document is deleted from the blockchain.
/// It performs various actions depending on the state of the document and the context in which it was deleted.
///
/// # Arguments
///
/// * `document_transition` - A reference to the document transition that triggered the data trigger.
/// * `context` - A reference to the data trigger execution context.
/// * `platform_version` - A reference to the platform version.
///
/// # Returns
///
/// A `DataTriggerExecutionResult` indicating the success or failure of the trigger execution.
pub fn delete_withdrawal_data_trigger_v0(
    document_transition: &DocumentTransitionAction,
    context: &DataTriggerExecutionContext<'_>,
    platform_version: &PlatformVersion,
) -> Result<DataTriggerExecutionResult, Error> {
    let mut result = DataTriggerExecutionResult::default();

    let DocumentTransitionAction::DeleteAction(dt_delete) = document_transition else {
        return Err(Error::Execution(ExecutionError::DataTriggerExecutionError(format!(
            "the Document Transition {} isn't 'DELETE",
            get_from_transition_action!(document_transition, id)
        ))));
    };

    let document_type = context
        .data_contract
        .document_type_for_name(withdrawal::NAME::NAME)?;

    let drive_query = DriveQuery {
        contract: context.data_contract,
        document_type: &document_type,
        internal_clauses: InternalClauses {
            primary_key_in_clause: None,
            primary_key_equal_clause: Some(WhereClause {
                field: "$id".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Identifier(dt_delete.base.id.to_buffer()),
            }),
            in_clause: None,
            range_clause: None,
            equal_clauses: BTreeMap::default(),
        },
        offset: None,
        limit: Some(100),
        order_by: Default::default(),
        start_at: None,
        start_at_included: false,
        block_time_ms: None,
    };

    let withdrawals = context
        .platform
        .drive
        .query_documents(
            drive_query,
            None,
            false,
            context.transaction,
            Some(platform_version.protocol_version),
        )?
        .documents;

    let Some(withdrawal) = withdrawals.get(0) else {
        let err = DataTriggerActionError::DataTriggerConditionError {
            data_contract_id: context.data_contract.id,
            document_transition_id: dt_delete.base.id,
            message: "Withdrawal document was not found".to_string(),
            owner_id: Some(*context.owner_id),
            document_transition: Some(DocumentTransitionAction::DeleteAction(dt_delete.clone())),
        };

        result.add_error(err);

        return Ok(result);
    };

    let status: u8 = withdrawal
        .properties
        .get_integer("status")
        .map_err(ProtocolError::ValueError)?;

    if status != withdrawals_contract::WithdrawalStatus::COMPLETE as u8
        || status != withdrawals_contract::WithdrawalStatus::EXPIRED as u8
    {
        let err = DataTriggerActionError::DataTriggerConditionError {
            data_contract_id: context.data_contract.id,
            document_transition_id: dt_delete.base.id,
            message: "withdrawal deletion is allowed only for COMPLETE and EXPIRED statuses"
                .to_string(),
            owner_id: Some(*context.owner_id),
            document_transition: Some(DocumentTransitionAction::DeleteAction(dt_delete.clone())),
        };

        result.add_error(err);

        return Ok(result);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform_types::platform::PlatformStateRef;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::extended_block_info::BlockInfo;
    use dpp::document::document_transition::{
        DocumentBaseTransitionAction, DocumentDeleteTransitionAction,
    };
    use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
    use dpp::document::Document;
    use dpp::identity::state_transition::identity_credit_withdrawal_transition::Pooling;
    use dpp::platform_value::{platform_value, Bytes32};
    use dpp::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
    use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dpp::tests::fixtures::{get_data_contract_fixture, get_withdrawal_document_fixture};
    use dpp::version::PlatformVersion;
    use drive::drive::object_size_info::DocumentInfo::DocumentRefInfo;
    use drive::drive::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};

    #[test]
    fn should_throw_error_if_withdrawal_not_found() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let state_read_guard = platform.state.read().unwrap();
        let platform_ref = PlatformStateRef {
            drive: &platform.drive,
            state: &state_read_guard,
            config: &platform.config,
        };

        let transition_execution_context = StateTransitionExecutionContext::default();
        let data_contract = get_data_contract_fixture(None).data_contract;
        let owner_id = &data_contract.owner_id;

        let document_transition = DocumentTransitionAction::DeleteAction(Default::default());
        let data_trigger_context = DataTriggerExecutionContext {
            platform: &platform_ref,
            data_contract: &data_contract,
            owner_id,
            state_transition_execution_context: &transition_execution_context,
            transaction: None,
        };

        let result = delete_withdrawal_data_trigger_v0(
            &document_transition,
            &data_trigger_context,
            PlatformVersion::first(),
        )
        .expect_err("the execution result should be returned");

        assert_eq!(
            result.to_string(),
            "protocol: document type not found: can not get document type from contract"
        );
    }

    #[test]
    fn can_serialize_and_deserialize_withdrawal() {
        let platform_version = PlatformVersion::first();
        let data_contract = load_system_data_contract(
            SystemDataContract::Withdrawals,
            platform_version.protocol_version,
        )
        .expect("to load system data contract");
        let owner_id = data_contract.owner_id;

        let platform_version = PlatformVersion::first();

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
                "outputScript": (0..23).collect::<Vec<u8>>(),
                "status": withdrawals_contract::WithdrawalStatus::BROADCASTED as u8,
                "transactionIndex": 1u64,
                "transactionSignHeight": 93u64,
                "transactionId": Bytes32::new([1;32]),
            }),
            None,
        )
        .expect("expected withdrawal document");

        let serialized = document
            .serialize(&document_type, platform_version)
            .expect("expected to serialize document");
        Document::from_bytes(&serialized, &document_type, platform_version)
            .expect("expected to deserialize document");
    }

    #[test]
    fn should_throw_error_if_withdrawal_has_wrong_status() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        let state_read_guard = platform.state.read().unwrap();

        let platform_ref = PlatformStateRef {
            drive: &platform.drive,
            state: &state_read_guard,
            config: &platform.config,
        };

        let transition_execution_context = StateTransitionExecutionContext::default();

        let platform_version = PlatformVersion::first();

        let data_contract = load_system_data_contract(
            SystemDataContract::Withdrawals,
            platform_version.protocol_version,
        )
        .expect("to load system data contract");
        let owner_id = data_contract.owner_id;

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
                "outputScript": (0..23).collect::<Vec<u8>>(),
                "status": withdrawals_contract::WithdrawalStatus::BROADCASTED as u8,
                "transactionIndex": 1u64,
                "transactionSignHeight": 93u64,
                "transactionId": Bytes32::new([1;32]),
            }),
            None,
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
                    document_type: &document_type,
                },
                false,
                BlockInfo::genesis(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert a document successfully");

        let document_transition =
            DocumentTransitionAction::DeleteAction(DocumentDeleteTransitionAction {
                base: DocumentBaseTransitionAction {
                    id: document.id,
                    ..Default::default()
                },
            });

        let data_trigger_context = DataTriggerExecutionContext {
            platform: &platform_ref,
            data_contract: &data_contract,
            owner_id: &owner_id,
            state_transition_execution_context: &transition_execution_context,
            transaction: None,
        };
        let result = delete_withdrawal_data_trigger_v0(
            &document_transition,
            &data_trigger_context,
            PlatformVersion::first(),
        )
        .expect("the execution result should be returned");

        assert!(!result.is_valid());

        let error = result.get_error(0).unwrap();

        assert_eq!(
            error.to_string(),
            "withdrawal deletion is allowed only for COMPLETE and EXPIRED statuses"
        );
    }
}
