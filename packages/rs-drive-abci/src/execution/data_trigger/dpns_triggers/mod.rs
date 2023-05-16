use dpp::util::hash::hash;
use std::collections::BTreeMap;

use crate::error::execution::ExecutionError;
use crate::error::Error;

use dpp::document::document_transition::DocumentTransitionAction;
use dpp::platform_value::btreemap_extensions::{BTreeValueMapHelper, BTreeValueMapPathHelper};
use dpp::platform_value::Value;
use dpp::prelude::Identifier;
use dpp::{get_from_transition_action, ProtocolError};
use drive::query::{DriveQuery, InternalClauses, WhereClause, WhereOperator};

use super::{create_error, DataTriggerExecutionContext, DataTriggerExecutionResult};

const MAX_PRINTABLE_DOMAIN_NAME_LENGTH: usize = 253;
const PROPERTY_LABEL: &str = "label";
const PROPERTY_NORMALIZED_LABEL: &str = "normalizedLabel";
const PROPERTY_NORMALIZED_PARENT_DOMAIN_NAME: &str = "normalizedParentDomainName";
const PROPERTY_PREORDER_SALT: &str = "preorderSalt";
const PROPERTY_ALLOW_SUBDOMAINS: &str = "subdomainRules.allowSubdomains";
const PROPERTY_RECORDS: &str = "records";
const PROPERTY_DASH_UNIQUE_IDENTITY_ID: &str = "dashUniqueIdentityId";
const PROPERTY_DASH_ALIAS_IDENTITY_ID: &str = "dashAliasIdentityId";

/// Creates a data trigger for handling domain documents.
///
/// The trigger is executed whenever a new domain document is created on the blockchain.
/// It performs various actions depending on the state of the document and the context in which it was created.
///
/// # Arguments
///
/// * `document_transition` - A reference to the document transition that triggered the data trigger.
/// * `context` - A reference to the data trigger execution context.
/// * `top_level_identity` - An optional identifier for the top-level identity associated with the domain
///   document (if one exists).
///
/// # Returns
///
/// A `DataTriggerExecutionResult` indicating the success or failure of the trigger execution.
pub fn create_domain_data_trigger(
    document_transition: &DocumentTransitionAction,
    context: &DataTriggerExecutionContext<'_>,
    top_level_identity: Option<&Identifier>,
) -> Result<DataTriggerExecutionResult, Error> {
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

    let data = &document_create_transition.data;

    let top_level_identity = top_level_identity.ok_or(Error::Execution(
        ExecutionError::DataTriggerExecutionError("top level identity isn't provided".to_string()),
    ))?;
    let owner_id = context.owner_id;
    let label = data
        .get_string(PROPERTY_LABEL)
        .map_err(ProtocolError::ValueError)?;
    let normalized_label = data
        .get_str(PROPERTY_NORMALIZED_LABEL)
        .map_err(ProtocolError::ValueError)?;
    let normalized_parent_domain_name = data
        .get_string(PROPERTY_NORMALIZED_PARENT_DOMAIN_NAME)
        .map_err(ProtocolError::ValueError)?;

    let preorder_salt = data
        .get_hash256_bytes(PROPERTY_PREORDER_SALT)
        .map_err(ProtocolError::ValueError)?;
    let records = data
        .get(PROPERTY_RECORDS)
        .ok_or(ExecutionError::DataTriggerExecutionError(format!(
            "property '{}' doesn't exist",
            PROPERTY_RECORDS
        )))?
        .to_btree_ref_string_map()
        .map_err(ProtocolError::ValueError)?;

    let rule_allow_subdomains = data
        .get_bool_at_path(PROPERTY_ALLOW_SUBDOMAINS)
        .map_err(ProtocolError::ValueError)?;

    let mut result = DataTriggerExecutionResult::default();
    let full_domain_name = normalized_label;

    if !is_dry_run {
        if full_domain_name.len() > MAX_PRINTABLE_DOMAIN_NAME_LENGTH {
            let err = create_error(
                context,
                document_create_transition,
                format!(
                    "Full domain name length can not be more than {} characters long but got {}",
                    MAX_PRINTABLE_DOMAIN_NAME_LENGTH,
                    full_domain_name.len()
                ),
            );
            result.add_error(err)
        }

        if normalized_label != label.to_lowercase() {
            let err = create_error(
                context,
                document_create_transition,
                "Normalized label doesn't match label".to_string(),
            );
            result.add_error(err);
        }

        if let Some(id) = records
            .get_optional_identifier(PROPERTY_DASH_UNIQUE_IDENTITY_ID)
            .map_err(ProtocolError::ValueError)?
        {
            if id != owner_id {
                let err = create_error(
                    context,
                    document_create_transition,
                    format!(
                        "ownerId {} doesn't match {} {}",
                        owner_id, PROPERTY_DASH_UNIQUE_IDENTITY_ID, id
                    ),
                );
                result.add_error(err);
            }
        }

        if let Some(id) = records
            .get_optional_identifier(PROPERTY_DASH_ALIAS_IDENTITY_ID)
            .map_err(ProtocolError::ValueError)?
        {
            if id != owner_id {
                let err = create_error(
                    context,
                    document_create_transition,
                    format!(
                        "ownerId {} doesn't match {} {}",
                        owner_id, PROPERTY_DASH_ALIAS_IDENTITY_ID, id
                    ),
                );
                result.add_error(err);
            }
        }

        if normalized_parent_domain_name.is_empty() && context.owner_id != top_level_identity {
            let err = create_error(
                context,
                document_create_transition,
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

        let document_type = context
            .data_contract
            .document_type_for_name(document_create_transition.base.document_type_name.as_str())?;
        let drive_query = DriveQuery {
            contract: context.data_contract,
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
            .query_documents(drive_query, None, is_dry_run, context.transaction)?
            .documents;

        if !is_dry_run {
            if documents.is_empty() {
                let err = create_error(
                    context,
                    document_create_transition,
                    "Parent domain is not present".to_string(),
                );
                result.add_error(err);
                return Ok(result);
            }
            let parent_domain = &documents[0];

            if rule_allow_subdomains {
                let err = create_error(
                    context,
                    document_create_transition,
                    "Allowing subdomains registration is forbidden for non top level domains"
                        .to_string(),
                );
                result.add_error(err);
            }

            if (!parent_domain
                .properties
                .get_bool_at_path(PROPERTY_ALLOW_SUBDOMAINS)
                .map_err(ProtocolError::ValueError)?)
                && context.owner_id != &parent_domain.owner_id
            {
                let err = create_error(
                    context,
                    document_create_transition,
                    "The subdomain can be created only by the parent domain owner".to_string(),
                );
                result.add_error(err);
            }
        }
    }

    let mut salted_domain_buffer: Vec<u8> = vec![];
    salted_domain_buffer.extend(preorder_salt);
    salted_domain_buffer.extend(full_domain_name.to_owned().as_bytes());

    let salted_domain_hash = hash(salted_domain_buffer);

    let document_type = context.data_contract.document_type_for_name("preorder")?;

    let drive_query = DriveQuery {
        contract: context.data_contract,
        document_type,
        internal_clauses: InternalClauses {
            primary_key_in_clause: None,
            primary_key_equal_clause: None,
            in_clause: None,
            range_clause: None,
            equal_clauses: BTreeMap::from([(
                "saltedDomainHash".to_string(),
                WhereClause {
                    field: "normalizedParentDomainName".to_string(),
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
        .query_documents(drive_query, None, is_dry_run, context.transaction)?
        .documents;

    if is_dry_run {
        return Ok(result);
    }

    if preorder_documents.is_empty() {
        let err = create_error(
            context,
            document_create_transition,
            "preorderDocument was not found".to_string(),
        );
        result.add_error(err)
    }

    Ok(result)
}

#[cfg(test)]
mod test {
    use crate::execution::data_trigger::DataTriggerExecutionContext;
    use crate::platform::PlatformStateRef;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::document::document_transition::{Action, DocumentCreateTransitionAction};
    use dpp::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
    use dpp::tests::fixtures::{
        get_document_transitions_fixture, get_dpns_data_contract_fixture,
        get_dpns_parent_document_fixture, ParentDocumentOptions,
    };
    use dpp::tests::utils::generate_random_identifier_struct;

    use super::create_domain_data_trigger;

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

        let transition_execution_context = StateTransitionExecutionContext::default();
        let owner_id = generate_random_identifier_struct();
        let document = get_dpns_parent_document_fixture(ParentDocumentOptions {
            owner_id,
            ..Default::default()
        });
        let data_contract = get_dpns_data_contract_fixture(Some(owner_id));
        let transitions = get_document_transitions_fixture([(Action::Create, vec![document])]);
        let first_transition = transitions.get(0).expect("transition should be present");

        let document_create_transition = first_transition
            .as_transition_create()
            .expect("expected a document create transition");

        transition_execution_context.enable_dry_run();

        let data_trigger_context = DataTriggerExecutionContext {
            platform: &platform_ref,
            data_contract: &data_contract.data_contract,
            owner_id: &owner_id,
            state_transition_execution_context: &transition_execution_context,
            transaction: None,
        };

        let result = create_domain_data_trigger(
            &DocumentCreateTransitionAction::from(document_create_transition).into(),
            &data_trigger_context,
            Some(&owner_id),
        )
        .expect("the execution result should be returned");
        assert!(result.is_valid());
    }
}
