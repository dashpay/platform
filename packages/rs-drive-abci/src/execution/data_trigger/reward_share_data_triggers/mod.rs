use dpp::data_contract::DriveContractExt;
use dpp::document::document_transition::{
    DocumentCreateTransitionAction, DocumentTransitionAction,
};
use dpp::document::Document;
use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use dpp::platform_value::{Identifier, Value};
use dpp::prelude::DocumentTransition;
use dpp::{get_from_transition_action, ProtocolError};
use drive::query::{DriveQuery, InternalClauses, WhereClause, WhereOperator};
use std::collections::BTreeMap;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::data_trigger::create_error;

use super::{DataTriggerExecutionContext, DataTriggerExecutionResult};

const MAX_PERCENTAGE: u64 = 10000;
const PROPERTY_PAY_TO_ID: &str = "payToId";
const PROPERTY_PERCENTAGE: &str = "percentage";
const MAX_DOCUMENTS: usize = 16;

/// Creates a data trigger for handling masternode reward share documents.
///
/// The trigger is executed whenever a new masternode reward share document is created on the blockchain.
/// It performs various actions depending on the state of the document and the context in which it was created.
///
/// # Arguments
///
/// * `document_transition` - A reference to the document transition that triggered the data trigger.
/// * `context` - A reference to the data trigger execution context.
/// * `_top_level_identifier` - An unused parameter for the top-level identifier associated with the masternode
///   reward share document (which is not needed for this trigger).
///
/// # Returns
///
/// A `DataTriggerExecutionResult` indicating the success or failure of the trigger execution.
pub fn create_masternode_reward_shares_data_trigger<'a>(
    document_transition: &DocumentTransitionAction,
    context: &DataTriggerExecutionContext<'a>,
    _top_level_identifier: Option<&Identifier>,
) -> Result<DataTriggerExecutionResult, Error> {
    let mut result = DataTriggerExecutionResult::default();
    let is_dry_run = context.state_transition_execution_context.is_dry_run();

    let document_create_transition = match document_transition {
        DocumentTransitionAction::CreateAction(d) => d,
        _ => {
            return Err(Error::Execution(ExecutionError::DataTriggerExecutionError(
                format!(
                    "the Document Transition {} isn't 'CREATE",
                    get_from_transition_action!(document_transition, id)
                ),
            )))
        }
    };

    let properties = &document_create_transition.data;

    let pay_to_id = properties
        .get_hash256_bytes(PROPERTY_PAY_TO_ID)
        .map_err(ProtocolError::ValueError)?;
    let percentage = properties
        .get_integer(PROPERTY_PERCENTAGE)
        .map_err(ProtocolError::ValueError)?;

    if !is_dry_run {
        let valid_masternodes_list = &context.platform.state.hpmn_masternode_list;

        let owner_id_in_sml = valid_masternodes_list
            .get(context.owner_id.as_slice())
            .is_some();

        if !owner_id_in_sml {
            let err = create_error(
                context,
                document_create_transition,
                "Only masternode identities can share rewards".to_string(),
            );
            result.add_error(err);
        }
    }

    let maybe_identity = context
        .platform
        .drive
        .fetch_identity_balance(pay_to_id, context.transaction)?;

    if !is_dry_run && maybe_identity.is_none() {
        let err = create_error(
            context,
            document_create_transition,
            format!(
                "Identity '{}' doesn't exist",
                bs58::encode(pay_to_id).into_string()
            ),
        );
        result.add_error(err);
    }

    let document_type = context
        .data_contract
        .document_type_for_name(&document_create_transition.base.document_type_name)?;

    let drive_query = DriveQuery {
        contract: &context.data_contract,
        document_type,
        internal_clauses: InternalClauses {
            primary_key_in_clause: None,
            primary_key_equal_clause: None,
            in_clause: None,
            range_clause: None,
            equal_clauses: BTreeMap::from([(
                "$ownerId".to_string(),
                WhereClause {
                    field: "$ownerId".to_string(),
                    operator: WhereOperator::Equal,
                    value: Value::Identifier(context.owner_id.to_buffer()),
                },
            )]),
        },
        offset: 0,
        limit: 0,
        order_by: Default::default(),
        start_at: None,
        start_at_included: false,
        block_time: None,
    };

    let documents = context
        .platform
        .drive
        .query_documents(drive_query, None, context.transaction)?
        .documents;

    if is_dry_run {
        return Ok(result);
    }

    if documents.len() >= MAX_DOCUMENTS {
        let err = create_error(
            context,
            document_create_transition,
            format!(
                "Reward shares cannot contain more than {} identities",
                MAX_DOCUMENTS
            ),
        );
        result.add_error(err);
        return Ok(result);
    }

    let mut total_percent: u64 = percentage;
    for d in documents.iter() {
        total_percent += d
            .properties
            .get_integer::<u64>(PROPERTY_PERCENTAGE)
            .map_err(ProtocolError::ValueError)?;
    }

    if total_percent > MAX_PERCENTAGE {
        let err = create_error(
            context,
            document_create_transition,
            format!("Percentage can not be more than {}", MAX_PERCENTAGE),
        );
        result.add_error(err);
    }

    Ok(result)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::platform::PlatformStateRef;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::data_contract::DataContract;
    use dpp::document::document_transition::Action;
    use dpp::document::ExtendedDocument;
    use dpp::identity::Identity;
    use dpp::mocks::{SMLEntry, SMLStore, SimplifiedMNList};
    use dpp::platform_value::Value;
    use dpp::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
    use dpp::tests::fixtures::{
        get_document_transitions_fixture, get_masternode_reward_shares_documents_fixture,
    };
    use dpp::tests::utils::generate_random_identifier_struct;
    use dpp::DataTriggerActionError;
    use drive::drive::block_info::BlockInfo;
    use drive::drive::object_size_info::DocumentInfo::DocumentRefWithoutSerialization;
    use drive::drive::object_size_info::OwnedDocumentInfo;

    struct TestData {
        top_level_identifier: Identifier,
        data_contract: DataContract,
        sml_store: SMLStore,
        extended_documents: Vec<ExtendedDocument>,
        document_create_transition: DocumentCreateTransitionAction,
        identity: Identity,
    }

    fn setup_test() -> TestData {
        let top_level_identifier_hex =
            "c286807d463b06c7aba3b9a60acf64c1fc03da8c1422005cd9b4293f08cf0562";
        let top_level_identifier =
            Identifier::from_bytes(&hex::decode(top_level_identifier_hex).unwrap()).unwrap();

        let sml_entries: Vec<SMLEntry> = vec![
            SMLEntry {
          pro_reg_tx_hash: top_level_identifier_hex.to_string(),
          confirmed_hash: "4eb56228c535db3b234907113fd41d57bcc7cdcb8e0e00e57590af27ee88c119".to_string(),
          service: "192.168.65.2:20101".to_string(),
          pub_key_operator: "809519c5f6f3be1c08782ac42ae9a83b6c7205eba43f9a96a4f032ec7a73f1a7c25fa78cce0d6d9c135f7e2c28527179".to_string(),
          voting_address: "yXmprXYP51uzfMyndtWwxz96MnkCKkFc9x".to_string(),
          is_valid: true,
        },

        SMLEntry {
          pro_reg_tx_hash: "a3e1edc6bd352eeaf0ae58e30781ef4b127854241a3fe7fddf36d5b7e1dc2b3f".to_string(),
          confirmed_hash: "27a0b637b56af038c45e2fd1f06c2401c8dadfa28ca5e0d19ca836cc984a8378".to_string(),
          service: "192.168.65.2:20201".to_string(),
          pub_key_operator: "987a4873caba62cd45a2f7d4aa6d94519ee6753e9bef777c927cb94ade768a542b0ff34a93231d3a92b4e75ffdaa366e".to_string(),
          voting_address: "ycL7L4mhYoaZdm9TH85svvpfeKtdfo249u".to_string(),
          is_valid: true,
         }
        ];

        let (documents, data_contract) = get_masternode_reward_shares_documents_fixture();
        let sml_store = SMLStore {
            sml_list_by_height: SimplifiedMNList {
                masternodes: sml_entries.clone(),
            },
            sml_list_current: SimplifiedMNList {
                masternodes: sml_entries,
            },
        };
        let document_transitions =
            get_document_transitions_fixture([(Action::Create, vec![documents[0].clone()])]);

        let DocumentTransition::Create(document_create_transition) =
            document_transitions[0].clone();
        TestData {
            extended_documents: documents,
            data_contract,
            top_level_identifier,
            sml_store,
            document_create_transition: document_create_transition.into(),
            identity: Identity::default(),
        }
    }

    fn get_data_trigger_error(
        result: &Result<DataTriggerExecutionResult, Error>,
        error_number: usize,
    ) -> &DataTriggerActionError {
        let execution_result = result.as_ref().expect("it should return execution result");
        execution_result
            .get_error(error_number)
            .expect("errors should exist")
    }

    fn should_return_an_error_if_percentage_greater_than_1000() {
        let mut platform = TestPlatformBuilder::new().build_with_mock_rpc();
        let state_read_guard = platform.state.read().unwrap();

        let platform_ref = PlatformStateRef {
            drive: &platform.drive,
            state: &state_read_guard,
            config: &platform.config,
        };

        let TestData {
            mut document_create_transition,
            extended_documents,
            sml_store,
            data_contract,
            top_level_identifier,
            ..
        } = setup_test();

        for document in extended_documents {
            platform_ref
                .drive
                .apply_contract(
                    &document.data_contract,
                    BlockInfo::default(),
                    true,
                    None,
                    None,
                )
                .expect("expected to apply contract");
            platform_ref
                .drive
                .add_document(
                    OwnedDocumentInfo {
                        document_info: DocumentRefWithoutSerialization((&document.document, None)),
                        owner_id: None,
                    },
                    document.data_contract_id,
                    document.document_type_name.as_str(),
                    false,
                    &BlockInfo::default(),
                    true,
                    None,
                )
                .expect("expected to add document");
        }

        let documents: Vec<Document> = extended_documents
            .clone()
            .into_iter()
            .map(|dt| dt.document)
            .collect();

        // documentsFixture contains percentage = 500
        document_create_transition
            .data
            .insert("percentage".to_string(), Value::U64(9501));

        let execution_context = StateTransitionExecutionContext::default();
        let context = DataTriggerExecutionContext {
            platform: &platform_ref,
            data_contract: &data_contract,
            owner_id: &top_level_identifier,
            state_transition_execution_context: &execution_context,
            transaction: None,
        };

        let result = create_masternode_reward_shares_data_trigger(
            &document_create_transition.into(),
            &context,
            None,
        );

        let percentage_error = get_data_trigger_error(&result, 1);
        assert_eq!(
            "Percentage can not be more than 10000",
            percentage_error.to_string()
        );
    }

    fn should_return_an_error_if_pay_to_id_does_not_exists() {
        let mut platform = TestPlatformBuilder::new().build_with_mock_rpc();
        let state_read_guard = platform.state.read().unwrap();

        let platform_ref = PlatformStateRef {
            drive: &platform.drive,
            state: &state_read_guard,
            config: &platform.config,
        };

        let TestData {
            document_create_transition,
            sml_store,
            data_contract,
            top_level_identifier,
            ..
        } = setup_test();

        let execution_context = StateTransitionExecutionContext::default();
        let context = DataTriggerExecutionContext {
            platform: &platform_ref,
            data_contract: &data_contract,
            owner_id: &top_level_identifier,
            state_transition_execution_context: &execution_context,
            transaction: None,
        };
        let result = create_masternode_reward_shares_data_trigger(
            &document_create_transition.into(),
            &context,
            None,
        );

        let error = get_data_trigger_error(&result, 0);
        let pay_to_id_bytes = document_create_transition
            .data
            .get_hash256_bytes(PROPERTY_PAY_TO_ID)
            .expect("expected to be able to get a hash");
        let pay_to_id = Identifier::from(pay_to_id_bytes);

        assert_eq!(
            format!("Identity '{}' doesn't exist", pay_to_id),
            error.to_string()
        );
    }

    fn should_return_an_error_if_owner_id_is_not_a_masternode_identity() {
        let mut platform = TestPlatformBuilder::new().build_with_mock_rpc();
        let state_read_guard = platform.state.read().unwrap();

        let platform_ref = PlatformStateRef {
            drive: &platform.drive,
            state: &state_read_guard,
            config: &platform.config,
        };

        let TestData {
            document_create_transition,
            sml_store,
            data_contract,
            ..
        } = setup_test();

        let execution_context = StateTransitionExecutionContext::default();
        let context = DataTriggerExecutionContext {
            platform: &platform_ref,
            data_contract: &data_contract,
            owner_id: &generate_random_identifier_struct(),
            state_transition_execution_context: &execution_context,
            transaction: None,
        };
        let result = create_masternode_reward_shares_data_trigger(
            &document_create_transition.into(),
            &context,
            None,
        );
        let error = get_data_trigger_error(&result, 0);

        assert_eq!(
            "Only masternode identities can share rewards",
            error.to_string()
        );
    }

    fn should_pass() {
        let mut platform = TestPlatformBuilder::new().build_with_mock_rpc();
        let state_read_guard = platform.state.read().unwrap();
        let platform_ref = PlatformStateRef {
            drive: &platform.drive,
            state: &state_read_guard,
            config: &platform.config,
        };

        let TestData {
            document_create_transition,
            sml_store,
            data_contract,
            top_level_identifier,
            identity,
            ..
        } = setup_test();

        platform_ref
            .drive
            .add_new_identity(identity, &BlockInfo::default(), true, None)
            .expect("should insert identity");

        let execution_context = StateTransitionExecutionContext::default();
        let context = DataTriggerExecutionContext {
            platform: &platform_ref,
            data_contract: &data_contract,
            owner_id: &top_level_identifier,
            state_transition_execution_context: &execution_context,
            transaction: None,
        };
        let result = create_masternode_reward_shares_data_trigger(
            &document_create_transition.into(),
            &context,
            None,
        )
        .expect("the execution result should be returned");
        assert!(result.is_valid())
    }

    fn should_return_error_if_there_are_16_stored_shares() {
        let mut platform = TestPlatformBuilder::new().build_with_mock_rpc();
        let state_read_guard = platform.state.read().unwrap();

        let platform_ref = PlatformStateRef {
            drive: &platform.drive,
            state: &state_read_guard,
            config: &platform.config,
        };

        let TestData {
            document_create_transition,
            sml_store,
            data_contract,
            top_level_identifier,
            identity,
            ..
        } = setup_test();

        platform_ref
            .drive
            .add_new_identity(identity, &BlockInfo::default(), true, None)
            .expect("should insert identity");

        let mut state_repository_mock = MockStateRepositoryLike::new();
        state_repository_mock
            .expect_fetch_sml_store()
            .returning(move || Ok(sml_store.clone()));
        state_repository_mock
            .expect_fetch_identity()
            .returning(move |_, _| Ok(Some(identity.clone())));
        let documents_to_return: Vec<Document> = (0..16).map(|_| Document::default()).collect();
        state_repository_mock
            .expect_fetch_documents()
            .return_once(move |_, _, _, _| Ok(documents_to_return));

        let execution_context = StateTransitionExecutionContext::default();
        let context = DataTriggerExecutionContext {
            platform: &platform_ref,
            data_contract: &data_contract,
            owner_id: &top_level_identifier,
            state_transition_execution_context: &execution_context,
            transaction: None,
        };

        let result = create_masternode_reward_shares_data_trigger(
            &document_create_transition.into(),
            &context,
            None,
        );
        let error = get_data_trigger_error(&result, 0);

        assert_eq!(
            "Reward shares cannot contain more than 16 identities",
            error.to_string()
        );
    }

    fn should_pass_on_dry_run() {
        let mut platform = TestPlatformBuilder::new().build_with_mock_rpc();
        let state_read_guard = platform.state.read().unwrap();

        let platform_ref = PlatformStateRef {
            drive: &platform.drive,
            state: &state_read_guard,
            config: &platform.config,
        };

        let TestData {
            document_create_transition,
            data_contract,
            top_level_identifier,
            ..
        } = setup_test();

        let mut state_repository_mock = MockStateRepositoryLike::new();
        state_repository_mock
            .expect_fetch_identity()
            .returning(move |_, _| Ok(None));
        state_repository_mock
            .expect_fetch_documents()
            .returning(move |_, _, _, _| Ok(vec![]));

        let execution_context = StateTransitionExecutionContext::default();
        execution_context.enable_dry_run();

        let context = DataTriggerExecutionContext {
            platform: &platform_ref,
            data_contract: &data_contract,
            owner_id: &top_level_identifier,
            state_transition_execution_context: &execution_context,
            transaction: None,
        };
        let result = create_masternode_reward_shares_data_trigger(
            &document_create_transition.into(),
            &context,
            None,
        )
        .expect("the execution result should be returned");
        assert!(result.is_valid());
    }
}
