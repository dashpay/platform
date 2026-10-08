use std::collections::BTreeMap;

use dpp::consensus::state::data_trigger::data_trigger_condition_error::DataTriggerConditionError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::document::DocumentV0Getters;
use dpp::fee::fee_result::FeeResult;
use dpp::platform_value::btreemap_extensions::{BTreeValueMapHelper, BTreeValueMapPathHelper};
use dpp::platform_value::Value;
use dpp::system_data_contracts::dpns_contract;
use dpp::system_data_contracts::dpns_contract::v1::document_types::domain::properties::{
    ALLOW_SUBDOMAINS, NORMALIZED_LABEL, NORMALIZED_PARENT_DOMAIN_NAME,
};
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use drive::drive::document::query::QueryDocumentsOutcomeV0Methods;
use drive::query::{DriveDocumentQuery, InternalClauses, WhereClause, WhereOperator};
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::DocumentCreateTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContextMethodsV0;
use crate::execution::validation::state_transition::batch::data_triggers::{
    DataTriggerExecutionContext, DataTriggerExecutionResult,
};

/// The DPNS `domain` create trigger from protocol version 14, when DPNS v3's
/// schema keywords check the rest of a create: the normalized names are
/// generated from their counterparts (`generatedFrom`), the preorder is the
/// writer's own from an earlier block and the create deletes it (`refersTo` on
/// `preorderSalt`), and the identity record is the owner (`propertyConstraints`
/// rule `recordsIdentityIsOwner`).
///
/// What stays here is the domain's place in the name hierarchy:
///
/// * a top-level domain (an empty `normalizedParentDomainName`) only by the
///   DPNS contract owner;
/// * otherwise the parent domain must exist (a billed Drive query, as in v1),
///   the new domain may not allow subdomains, and a parent that does not allow
///   subdomains takes them only from its own owner.
///
/// # Returns
///
/// A `DataTriggerExecutionResult` holding a `DataTriggerConditionError` for the
/// first failed check, or none. A dry run (`check_tx`) runs the parent query to
/// bill it and judges nothing.
#[inline(always)]
pub(super) fn create_domain_data_trigger_v2(
    document_transition: &DocumentTransitionAction,
    context: &mut DataTriggerExecutionContext<'_>,
    platform_version: &PlatformVersion,
) -> Result<DataTriggerExecutionResult, Error> {
    let data_contract_fetch_info = document_transition.base().data_contract_fetch_info();
    let data_contract = &data_contract_fetch_info.contract;
    let is_dry_run = context.state_transition_execution_context.in_dry_run();
    let DocumentTransitionAction::CreateAction(document_create_transition) = document_transition
    else {
        return Err(Error::Execution(ExecutionError::DataTriggerExecutionError(
            format!(
                "the Document Transition {} isn't 'CREATE",
                document_transition.base().id()
            ),
        )));
    };

    // The action transformer has generated `normalizedParentDomainName` when the
    // transition left it out, so it is always here
    let data = document_create_transition.data();
    let normalized_parent_domain_name = data
        .get_string(NORMALIZED_PARENT_DOMAIN_NAME)
        .map_err(ProtocolError::ValueError)?;
    let rule_allow_subdomains = data
        .get_bool_at_path(ALLOW_SUBDOMAINS)
        .map_err(ProtocolError::ValueError)?;

    let mut result = DataTriggerExecutionResult::default();

    if normalized_parent_domain_name.is_empty() {
        if !is_dry_run && context.owner_id != &dpns_contract::OWNER_ID {
            result.add_error(DataTriggerConditionError::new(
                data_contract.id(),
                document_transition.base().id(),
                "Can't create top level domain for this identity".to_string(),
            ));
        }
        return Ok(result);
    }

    // The parent is the domain whose label is the first segment of the parent
    // name, under the rest of it
    let (parent_domain_label, grand_parent_domain_name) = normalized_parent_domain_name
        .split_once('.')
        .unwrap_or((normalized_parent_domain_name.as_str(), ""));

    let document_type = data_contract.document_type_for_name(
        document_create_transition
            .base()
            .document_type_name()
            .as_str(),
    )?;

    let drive_query = DriveDocumentQuery {
        contract: data_contract,
        document_type,
        internal_clauses: InternalClauses {
            primary_key_in_clause: None,
            primary_key_equal_clause: None,
            in_clauses: Vec::new(),
            range_clause: None,
            equal_clauses: BTreeMap::from([
                (
                    NORMALIZED_PARENT_DOMAIN_NAME.to_string(),
                    WhereClause {
                        field: NORMALIZED_PARENT_DOMAIN_NAME.to_string(),
                        operator: WhereOperator::Equal,
                        value: Value::Text(grand_parent_domain_name.to_string()),
                    },
                ),
                (
                    NORMALIZED_LABEL.to_string(),
                    WhereClause {
                        field: NORMALIZED_LABEL.to_string(),
                        operator: WhereOperator::Equal,
                        value: Value::Text(parent_domain_label.to_string()),
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
        resolved_time_ranges: vec![],
        sub_queries: vec![],
    };

    let parent_domain_outcome = context.platform.drive.query_documents(
        drive_query,
        Some(&context.block_info.epoch),
        is_dry_run,
        context.transaction,
        Some(platform_version.protocol_version),
    )?;
    context.state_transition_execution_context.add_operation(
        ValidationOperation::PrecalculatedOperation(FeeResult {
            processing_fee: parent_domain_outcome.cost(),
            ..Default::default()
        }),
    );

    if is_dry_run {
        return Ok(result);
    }

    let documents = parent_domain_outcome.documents_owned();
    let Some(parent_domain) = documents.first() else {
        result.add_error(DataTriggerConditionError::new(
            data_contract.id(),
            document_transition.base().id(),
            "Parent domain is not present".to_string(),
        ));
        return Ok(result);
    };

    if rule_allow_subdomains {
        result.add_error(DataTriggerConditionError::new(
            data_contract.id(),
            document_transition.base().id(),
            "Allowing subdomains registration is forbidden for this domain".to_string(),
        ));
        return Ok(result);
    }

    let parent_allows_subdomains = parent_domain
        .properties()
        .get_bool_at_path(ALLOW_SUBDOMAINS)
        .map_err(ProtocolError::ValueError)?;
    if !parent_allows_subdomains && context.owner_id != &parent_domain.owner_id() {
        result.add_error(DataTriggerConditionError::new(
            data_contract.id(),
            document_transition.base().id(),
            "The subdomain can be created only by the parent domain owner".to_string(),
        ));
    }

    Ok(result)
}
