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

    // A parent domain name is a single label (its pattern holds no dot), so the
    // parent is the top-level domain of that label
    let parent_domain_label = normalized_parent_domain_name.as_str();

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
                        value: Value::Text(String::new()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
    use crate::platform_types::platform::PlatformStateRef;
    use crate::platform_types::platform_state::PlatformStateV0Methods;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::block_info::BlockInfo;
    use dpp::platform_value::Bytes32;
    use dpp::prelude::Identifier;
    use dpp::state_transition::batch_transition::resolvers::v0::BatchTransitionResolversV0;
    use dpp::tests::fixtures::{
        get_batched_transitions_fixture, get_dpns_data_contract_fixture,
        get_dpns_parent_document_fixture, ParentDocumentOptions,
    };
    use dpp::tests::utils::generate_random_identifier_struct;
    use dpp::version::DefaultForPlatformVersion;
    use drive::drive::contract::DataContractFetchInfo;
    use drive::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::DocumentCreateTransitionAction;
    use drive::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionActionType;
    use std::sync::Arc;

    /// Runs the trigger on the create of a top-level domain by `owner_id`, an
    /// identity other than the DPNS contract owner.
    fn top_level_domain_by(
        owner_id: Identifier,
        dry_run: bool,
    ) -> (DataTriggerExecutionResult, bool) {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let state = platform.state.load();
        let platform_ref = PlatformStateRef {
            drive: &platform.drive,
            state: &state,
            config: &platform.config,
        };
        let platform_version = state
            .current_platform_version()
            .expect("expected a platform version");
        let protocol_version = state.current_protocol_version_in_consensus();

        let document = get_dpns_parent_document_fixture(
            ParentDocumentOptions {
                owner_id,
                ..Default::default()
            },
            protocol_version,
        );
        let data_contract = get_dpns_data_contract_fixture(Some(owner_id), 0, protocol_version)
            .data_contract_owned();
        let document_type = data_contract
            .document_type_for_name("domain")
            .expect("expected the domain document type");
        let transitions = get_batched_transitions_fixture(
            [(
                DocumentTransitionActionType::Create,
                vec![(document, document_type, Bytes32::default(), None)],
            )],
            &mut BTreeMap::new(),
        );
        let document_create_transition = transitions
            .first()
            .expect("expected a transition")
            .as_transition_create()
            .expect("expected a document create transition");
        let action = DocumentCreateTransitionAction::try_from_document_borrowed_create_transition_with_contract_lookup(
            &platform.drive,
            owner_id,
            None,
            document_create_transition,
            &BlockInfo::default(),
            0,
            |_identifier| {
                Ok(Arc::new(DataContractFetchInfo::dpns_contract_fixture(
                    platform_version.protocol_version,
                )))
            },
            platform_version,
        )
        .expect("expected to create the action")
        .0
        .into_data()
        .expect("expected a valid transition");

        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("expected an execution context");
        if dry_run {
            execution_context.enable_dry_run();
        }
        let block_info = BlockInfo::default();
        let mut context = DataTriggerExecutionContext {
            platform: &platform_ref,
            block_info: &block_info,
            owner_id: &owner_id,
            state_transition_execution_context: &mut execution_context,
            transaction: None,
        };
        let result = create_domain_data_trigger_v2(
            &action
                .as_document_action()
                .expect("expected a document action"),
            &mut context,
            platform_version,
        )
        .expect("expected the trigger to run");
        let billed = !context
            .state_transition_execution_context
            .operations_slice()
            .is_empty();
        (result, billed)
    }

    #[test]
    fn should_refuse_a_top_level_domain_from_another_identity() {
        let (result, billed) = top_level_domain_by(generate_random_identifier_struct(), false);

        assert!(!result.is_valid());
        assert!(result
            .errors
            .first()
            .expect("expected an error")
            .to_string()
            .contains("Can't create top level domain for this identity"));
        assert!(!billed, "a top-level domain runs no parent query");
    }

    #[test]
    fn should_judge_nothing_on_a_dry_run() {
        let (result, billed) = top_level_domain_by(generate_random_identifier_struct(), true);

        assert!(result.is_valid());
        assert!(!billed, "a top-level domain runs no parent query");
    }
}
