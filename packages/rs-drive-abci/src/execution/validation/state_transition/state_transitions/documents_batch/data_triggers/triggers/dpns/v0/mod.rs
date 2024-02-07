use dpp::consensus::state::data_trigger::data_trigger_condition_error::DataTriggerConditionError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contracts::dpns_contract::v1::document_types::domain::properties::PARENT_DOMAIN_NAME;
///! The `dpns_triggers` module contains data triggers specific to the DPNS data contract.
use dpp::util::hash::hash_double;
use std::collections::BTreeMap;

use crate::error::execution::ExecutionError;
use crate::error::Error;

use crate::execution::validation::state_transition::documents_batch::data_triggers::{
    DataTriggerExecutionContext, DataTriggerExecutionResult,
};
use dpp::document::DocumentV0Getters;
use dpp::platform_value::btreemap_extensions::{BTreeValueMapHelper, BTreeValueMapPathHelper};
use dpp::platform_value::Value;
use dpp::ProtocolError;
use drive::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::document::documents_batch::document_transition::document_create_transition_action::DocumentCreateTransitionActionAccessorsV0;
use drive::state_transition_action::document::documents_batch::document_transition::DocumentTransitionAction;
use dpp::system_data_contracts::dpns_contract;
use dpp::system_data_contracts::dpns_contract::v1::document_types::domain::properties::{ALLOW_SUBDOMAINS,
                                                                                     DASH_ALIAS_IDENTITY_ID, DASH_UNIQUE_IDENTITY_ID, LABEL, NORMALIZED_LABEL, NORMALIZED_PARENT_DOMAIN_NAME, PREORDER_SALT, RECORDS};
use dpp::util::strings::convert_to_homograph_safe_chars;
use dpp::version::PlatformVersion;
use drive::drive::document::query::QueryDocumentsOutcomeV0Methods;
use drive::query::{DriveQuery, InternalClauses, WhereClause, WhereOperator};
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContextMethodsV0;

pub const MAX_PRINTABLE_DOMAIN_NAME_LENGTH: usize = 253;

/// Creates a data trigger for handling domain documents.
///
/// The trigger is executed whenever a new domain document is created on the blockchain.
/// It performs various actions depending on the state of the document and the context in which it was created.
///
/// # Arguments
///
/// * `document_transition` - A reference to the document transition that triggered the data trigger.
/// * `context` - A reference to the data trigger execution context.
/// * `dpns_contract::OWNER_ID` - An optional identifier for the top-level identity associated with the domain
///   document (if one exists).
///
/// # Returns
///
/// A `DataTriggerExecutionResult` indicating the success or failure of the trigger execution.
pub fn create_domain_data_trigger_v0(
    document_transition: &DocumentTransitionAction,
    context: &DataTriggerExecutionContext<'_>,
    platform_version: &PlatformVersion,
) -> Result<DataTriggerExecutionResult, Error> {
    let data_contract_fetch_info = document_transition.base().data_contract_fetch_info();
    let data_contract = &data_contract_fetch_info.contract;
    let is_dry_run = context.state_transition_execution_context.in_dry_run();
    let document_create_transition = match document_transition {
        DocumentTransitionAction::CreateAction(d) => d,
        _ => {
            return Err(Error::Execution(ExecutionError::DataTriggerExecutionError(
                format!(
                    "the Document Transition {} isn't 'CREATE",
                    document_transition.base().id()
                ),
            )))
        }
    };

    let data = document_create_transition.data();

    let owner_id = context.owner_id;
    let label = data.get_string(LABEL).map_err(ProtocolError::ValueError)?;
    let normalized_label = data
        .get_str(NORMALIZED_LABEL)
        .map_err(ProtocolError::ValueError)?;

    let parent_domain_name = data
        .get_string(PARENT_DOMAIN_NAME)
        .map_err(ProtocolError::ValueError)?;
    let normalized_parent_domain_name = data
        .get_string(NORMALIZED_PARENT_DOMAIN_NAME)
        .map_err(ProtocolError::ValueError)?;

    let preorder_salt = data
        .get_hash256_bytes(PREORDER_SALT)
        .map_err(ProtocolError::ValueError)?;
    let records = data
        .get(RECORDS)
        .ok_or(ExecutionError::DataTriggerExecutionError(format!(
            "property '{}' doesn't exist",
            RECORDS
        )))?
        .to_btree_ref_string_map()
        .map_err(ProtocolError::ValueError)?;

    let rule_allow_subdomains = data
        .get_bool_at_path(ALLOW_SUBDOMAINS)
        .map_err(ProtocolError::ValueError)?;

    let full_domain_name = if parent_domain_name.is_empty() {
        label.to_string()
    } else {
        format!("{normalized_label}.{parent_domain_name}")
    };

    let mut result = DataTriggerExecutionResult::default();

    if !is_dry_run {
        if full_domain_name.len() > MAX_PRINTABLE_DOMAIN_NAME_LENGTH {
            let err = DataTriggerConditionError::new(
                data_contract.id(),
                document_transition.base().id(),
                format!(
                    "Full domain name length can not be more than {} characters long but got {}",
                    MAX_PRINTABLE_DOMAIN_NAME_LENGTH,
                    full_domain_name.len()
                ),
            );

            result.add_error(err)
        }

        if normalized_label != convert_to_homograph_safe_chars(label.as_str()) {
            let err = DataTriggerConditionError::new(
                data_contract.id(),
                document_transition.base().id(),
                format!(
                    "Normalized label doesn't match label: {} != {}",
                    normalized_label, label
                ),
            );

            result.add_error(err);
        }

        if normalized_parent_domain_name
            != convert_to_homograph_safe_chars(parent_domain_name.as_str())
        {
            let err = DataTriggerConditionError::new(
                data_contract.id(),
                document_transition.base().id(),
                format!(
                    "Normalized parent domain name doesn't match parent domain name: {} != {}",
                    normalized_parent_domain_name, parent_domain_name
                ),
            );

            result.add_error(err);
        }

        if let Some(id) = records
            .get_optional_identifier(DASH_UNIQUE_IDENTITY_ID)
            .map_err(ProtocolError::ValueError)?
        {
            if id != owner_id {
                let err = DataTriggerConditionError::new(
                    data_contract.id(),
                    document_transition.base().id(),
                    format!(
                        "ownerId {} doesn't match {} {}",
                        owner_id, DASH_UNIQUE_IDENTITY_ID, id
                    ),
                );

                result.add_error(err);
            }
        }

        if let Some(id) = records
            .get_optional_identifier(DASH_ALIAS_IDENTITY_ID)
            .map_err(ProtocolError::ValueError)?
        {
            if id != owner_id {
                let err = DataTriggerConditionError::new(
                    data_contract.id(),
                    document_transition.base().id(),
                    format!(
                        "ownerId {} doesn't match {} {}",
                        owner_id, DASH_ALIAS_IDENTITY_ID, id
                    ),
                );

                result.add_error(err);
            }
        }

        if normalized_parent_domain_name.is_empty() && context.owner_id != &dpns_contract::OWNER_ID
        {
            let err = DataTriggerConditionError::new(
                data_contract.id(),
                document_transition.base().id(),
                "Can't create top level domain for this identity".to_string(),
            );

            result.add_error(err);
        }
    }

    if !normalized_parent_domain_name.is_empty() {
        //? What is the `normalized_parent_name`. Are we sure the content is a valid dot-separated data
        let mut parent_domain_segments = normalized_parent_domain_name.split('.');
        let parent_domain_label = parent_domain_segments.next().unwrap().to_string();
        let grand_parent_domain_name = parent_domain_segments.collect::<Vec<&str>>().join(".");

        let document_type = data_contract.document_type_for_name(
            document_create_transition
                .base()
                .document_type_name()
                .as_str(),
        )?;

        let drive_query = DriveQuery {
            contract: data_contract,
            document_type,
            internal_clauses: InternalClauses {
                primary_key_in_clause: None,
                primary_key_equal_clause: None,
                in_clause: None,
                range_clause: None,
                equal_clauses: BTreeMap::from([
                    (
                        "normalizedParentDomainName".to_string(),
                        WhereClause {
                            field: "normalizedParentDomainName".to_string(),
                            operator: WhereOperator::Equal,
                            value: Value::Text(grand_parent_domain_name),
                        },
                    ),
                    (
                        "normalizedLabel".to_string(),
                        WhereClause {
                            field: "normalizedLabel".to_string(),
                            operator: WhereOperator::Equal,
                            value: Value::Text(parent_domain_label),
                        },
                    ),
                ]),
            },
            offset: None,
            limit: None,
            order_by: Default::default(),
            start_at: None,
            start_at_included: false,
            block_time_ms: None,
        };

        let documents = context
            .platform
            .drive
            .query_documents(
                drive_query,
                None,
                is_dry_run,
                context.transaction,
                Some(platform_version.protocol_version),
            )?
            .documents_owned();

        if !is_dry_run {
            if documents.is_empty() {
                let err = DataTriggerConditionError::new(
                    data_contract.id(),
                    document_transition.base().id(),
                    "Parent domain is not present".to_string(),
                );

                result.add_error(err);

                return Ok(result);
            }
            let parent_domain = &documents[0];

            if rule_allow_subdomains {
                let err = DataTriggerConditionError::new(
                    data_contract.id(),
                    document_transition.base().id(),
                    "Allowing subdomains registration is forbidden for this domain".to_string(),
                );

                result.add_error(err);

                return Ok(result);
            }

            if (!parent_domain
                .properties()
                .get_bool_at_path(ALLOW_SUBDOMAINS)
                .map_err(ProtocolError::ValueError)?)
                && context.owner_id != &parent_domain.owner_id()
            {
                let err = DataTriggerConditionError::new(
                    data_contract.id(),
                    document_transition.base().id(),
                    "The subdomain can be created only by the parent domain owner".to_string(),
                );

                result.add_error(err);

                return Ok(result);
            }
        }
    }

    let mut salted_domain_buffer: Vec<u8> = vec![];
    salted_domain_buffer.extend(preorder_salt);
    salted_domain_buffer.extend(full_domain_name.as_bytes());

    let salted_domain_hash = hash_double(salted_domain_buffer);

    let document_type = data_contract.document_type_for_name("preorder")?;

    let drive_query = DriveQuery {
        contract: data_contract,
        document_type,
        internal_clauses: InternalClauses {
            primary_key_in_clause: None,
            primary_key_equal_clause: None,
            in_clause: None,
            range_clause: None,
            equal_clauses: BTreeMap::from([(
                "saltedDomainHash".to_string(),
                WhereClause {
                    field: "saltedDomainHash".to_string(),
                    operator: WhereOperator::Equal,
                    value: Value::Bytes32(salted_domain_hash),
                },
            )]),
        },
        offset: None,
        limit: None,
        order_by: Default::default(),
        start_at: None,
        start_at_included: false,
        block_time_ms: None,
    };

    let preorder_documents = context
        .platform
        .drive
        .query_documents(
            drive_query,
            None,
            is_dry_run,
            context.transaction,
            Some(platform_version.protocol_version),
        )?
        .documents_owned();

    if is_dry_run {
        return Ok(result);
    }

    if preorder_documents.is_empty() {
        let err = DataTriggerConditionError::new(
            data_contract.id(),
            document_transition.base().id(),
            "preorderDocument was not found".to_string(),
        );
        result.add_error(err)
    }

    Ok(result)
}

#[cfg(test)]
mod test {
    use std::sync::Arc;
    use dpp::platform_value::Bytes32;
    use drive::state_transition_action::document::documents_batch::document_transition::document_create_transition_action::DocumentCreateTransitionAction;
    use drive::state_transition_action::document::documents_batch::document_transition::DocumentTransitionActionType;
    use dpp::tests::fixtures::{get_document_transitions_fixture, get_dpns_data_contract_fixture, get_dpns_parent_document_fixture, ParentDocumentOptions};
    use dpp::tests::utils::generate_random_identifier_struct;
    use dpp::version::{DefaultForPlatformVersion};
    use drive::drive::contract::DataContractFetchInfo;
    use crate::execution::types::state_transition_execution_context::{StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0};
    use crate::platform_types::platform::PlatformStateRef;
    use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use super::*;

    #[test]
    fn should_return_execution_result_on_dry_run() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let state_read_guard = platform.state.read().unwrap();

        let platform_ref = PlatformStateRef {
            drive: &platform.drive,
            state: &state_read_guard,
            config: &platform.config,
        };

        let platform_version = state_read_guard
            .current_platform_version()
            .expect("should return a platform version");

        let mut transition_execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .unwrap();
        let owner_id = generate_random_identifier_struct();
        let document = get_dpns_parent_document_fixture(
            ParentDocumentOptions {
                owner_id,
                ..Default::default()
            },
            state_read_guard.current_protocol_version_in_consensus(),
        );
        let data_contract = get_dpns_data_contract_fixture(
            Some(owner_id),
            state_read_guard.current_protocol_version_in_consensus(),
        )
        .data_contract_owned();
        let document_type = data_contract
            .document_type_for_name("domain")
            .expect("expected to get domain document type");
        let transitions = get_document_transitions_fixture([(
            DocumentTransitionActionType::Create,
            vec![(document, document_type, Bytes32::default())],
        )]);
        let first_transition = transitions.get(0).expect("transition should be present");

        let document_create_transition = first_transition
            .as_transition_create()
            .expect("expected a document create transition");

        transition_execution_context.enable_dry_run();

        let data_trigger_context = DataTriggerExecutionContext {
            platform: &platform_ref,
            owner_id: &owner_id,
            state_transition_execution_context: &transition_execution_context,
            transaction: None,
        };

        let result = create_domain_data_trigger_v0(
            &DocumentCreateTransitionAction::from_document_borrowed_create_transition_with_contract_lookup(
                document_create_transition,|_identifier| {
                    Ok(Arc::new(DataContractFetchInfo::dpns_contract_fixture(platform_version.protocol_version)))
                }).expect("expected to create action").into(),
            &data_trigger_context,
            platform_version,
        )
        .expect("the execution result should be returned");
        assert!(result.is_valid());
    }
}
