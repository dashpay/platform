use std::convert::TryInto;

use anyhow::Context;
use anyhow::{anyhow, bail};
use platform_value::btreemap_extensions::BTreeValueMapHelper;
use platform_value::btreemap_extensions::BTreeValueMapPathHelper;
use platform_value::platform_value;

use crate::document::Document;
use crate::util::hash::hash_to_vec;

use crate::ProtocolError;
use crate::{
    document::document_transition::DocumentTransition, get_from_transition, prelude::Identifier,
    state_repository::StateRepositoryLike,
};

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

pub async fn create_domain_data_trigger<'a, SR>(
    document_transition: &DocumentTransition,
    context: &DataTriggerExecutionContext<'a, SR>,
    top_level_identity: Option<&Identifier>,
) -> Result<DataTriggerExecutionResult, anyhow::Error>
where
    SR: StateRepositoryLike,
{
    let is_dry_run = context.state_transition_execution_context.is_dry_run();
    let dt_create = match document_transition {
        DocumentTransition::Create(d) => d,
        _ => bail!(
            "the Document Transition {} isn't 'CREATE'",
            get_from_transition!(document_transition, id)
        ),
    };

    let data = dt_create.data.as_ref().ok_or_else(|| {
        anyhow!(
            "data isn't defined in Data Transition '{}'",
            dt_create.base.id
        )
    })?;

    let top_level_identity = top_level_identity.context("top level identity isn't provided")?;
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
        .ok_or_else(|| anyhow!("property '{}' doesn't exist", PROPERTY_RECORDS))?
        .to_btree_ref_string_map()
        .map_err(ProtocolError::ValueError)?;

    let rule_allow_subdomains = data
        .get_bool_at_path(PROPERTY_ALLOW_SUBDOMAINS)
        .map_err(ProtocolError::ValueError)?;

    let mut result = DataTriggerExecutionResult::default();
    let mut full_domain_name = normalized_label.to_string();

    if !is_dry_run {
        if full_domain_name.len() > MAX_PRINTABLE_DOMAIN_NAME_LENGTH {
            let err = create_error(
                context,
                dt_create.base.id,
                format!(
                    "Full domain name length can not be more than {} characters long but got {}",
                    MAX_PRINTABLE_DOMAIN_NAME_LENGTH,
                    full_domain_name.len()
                ),
            );
            result.add_error(err.into())
        }

        if normalized_label != label.to_lowercase() {
            let err = create_error(
                context,
                dt_create.base.id,
                "Normalized label doesn't match label".to_string(),
            );
            result.add_error(err.into());
        }

        if let Some(id) = records
            .get_optional_identifier(PROPERTY_DASH_UNIQUE_IDENTITY_ID)
            .map_err(ProtocolError::ValueError)?
        {
            if id != owner_id {
                let err = create_error(
                    context,
                    dt_create.base.id,
                    format!(
                        "ownerId {} doesn't match {} {}",
                        owner_id, PROPERTY_DASH_UNIQUE_IDENTITY_ID, id
                    ),
                );
                result.add_error(err.into())
            }
        }

        if let Some(id) = records
            .get_optional_identifier(PROPERTY_DASH_ALIAS_IDENTITY_ID)
            .map_err(ProtocolError::ValueError)?
        {
            if id != owner_id {
                let err = create_error(
                    context,
                    dt_create.base.id,
                    format!(
                        "ownerId {} doesn't match {} {}",
                        owner_id, PROPERTY_DASH_ALIAS_IDENTITY_ID, id
                    ),
                );
                result.add_error(err.into());
            }
        }

        if normalized_parent_domain_name.is_empty() && context.owner_id != top_level_identity {
            let err = create_error(
                context,
                dt_create.base.id,
                "Can't create top level domain for this identity".to_string(),
            );
            result.add_error(err.into())
        }
    }

    if !normalized_parent_domain_name.is_empty() {
        full_domain_name = format!("{full_domain_name}.{normalized_parent_domain_name}");

        //? What is the `normalized_parent_name`. Are we sure the content is a valid dot-separated data
        let mut parent_domain_segments = normalized_parent_domain_name.split('.');
        let parent_domain_label = parent_domain_segments.next().unwrap().to_string();
        let grand_parent_domain_name = parent_domain_segments.collect::<Vec<&str>>().join(".");

        let documents_data = context
            .state_repository
            .fetch_documents(
                &context.data_contract.id(),
                &dt_create.base.document_type_name,
                platform_value!({
                    "where" : [
                        ["normalizedParentDomainName", "==", grand_parent_domain_name],
                        ["normalizedLabel", "==", parent_domain_label]
                    ]
                }),
                Some(context.state_transition_execution_context),
            )
            .await?;
        let documents: Vec<Document> = documents_data
            .into_iter()
            .map(|d| d.try_into().map_err(Into::<ProtocolError>::into))
            .collect::<Result<Vec<Document>, ProtocolError>>()?;

        if !is_dry_run {
            if documents.is_empty() {
                let err = create_error(
                    context,
                    dt_create.base.id,
                    "Parent domain is not present".to_string(),
                );
                result.add_error(err.into());
                return Ok(result);
            }
            let parent_domain = &documents[0];

            if rule_allow_subdomains {
                let err = create_error(
                    context,
                    dt_create.base.id,
                    "Allowing subdomains registration is forbidden for non top level domains"
                        .to_string(),
                );
                result.add_error(err.into());
            }

            if (!parent_domain
                .properties
                .get_bool_at_path(PROPERTY_ALLOW_SUBDOMAINS)?)
                && context.owner_id != &parent_domain.owner_id
            {
                let err = create_error(
                    context,
                    dt_create.base.id,
                    "The subdomain can be created only by the parent domain owner".to_string(),
                );
                result.add_error(err.into());
            }
        }
    }

    let mut salted_domain_buffer: Vec<u8> = vec![];
    salted_domain_buffer.extend(preorder_salt);
    salted_domain_buffer.extend(full_domain_name.to_owned().as_bytes());

    let salted_domain_hash = hash_to_vec(salted_domain_buffer);

    let preorder_documents_data = context
        .state_repository
        .fetch_documents(
            &context.data_contract.id(),
            "preorder",
            platform_value!({
                //? should this be a base64 encoded
                "where" : [["saltedDomainHash", "==", salted_domain_hash]]
            }),
            Some(context.state_transition_execution_context),
        )
        .await?;
    let preorder_documents: Vec<Document> = preorder_documents_data
        .into_iter()
        .map(|d| d.try_into().map_err(Into::<ProtocolError>::into))
        .collect::<Result<Vec<Document>, ProtocolError>>()?;

    if is_dry_run {
        return Ok(result);
    }

    if preorder_documents.is_empty() {
        let err = create_error(
            context,
            dt_create.base.id,
            "preorderDocument was not found".to_string(),
        );
        result.add_error(err.into())
    }

    Ok(result)
}

#[cfg(test)]
mod test {

    use crate::{
        data_trigger::DataTriggerExecutionContext,
        document::document_transition::Action,
        state_repository::MockStateRepositoryLike,
        state_transition::state_transition_execution_context::StateTransitionExecutionContext,
        tests::{
            fixtures::{
                get_document_transitions_fixture, get_dpns_data_contract_fixture,
                get_dpns_parent_document_fixture, ParentDocumentOptions,
            },
            utils::generate_random_identifier_struct,
        },
    };

    use super::create_domain_data_trigger;

    #[tokio::test]
    async fn should_return_execution_result_on_dry_run() {
        let mut state_repository = MockStateRepositoryLike::new();
        let transition_execution_context = StateTransitionExecutionContext::default();
        let owner_id = generate_random_identifier_struct();
        let document = get_dpns_parent_document_fixture(ParentDocumentOptions {
            owner_id,
            ..Default::default()
        });
        let data_contract = get_dpns_data_contract_fixture(Some(owner_id)).data_contract;
        let transitions = get_document_transitions_fixture([(DocumentTransitionActionType::Create, vec![document])]);
        let first_transition = transitions.get(0).expect("transition should be present");

        state_repository
            .expect_fetch_documents()
            .returning(|_, _, _, _| Ok(vec![]));
        transition_execution_context.enable_dry_run();

        let data_trigger_context = DataTriggerExecutionContext {
            data_contract: &data_contract,
            owner_id: &owner_id,
            state_repository: &state_repository,
            state_transition_execution_context: &transition_execution_context,
        };

        let result =
            create_domain_data_trigger(first_transition, &data_trigger_context, Some(&owner_id))
                .await
                .expect("the execution result should be returned");
        assert!(result.is_ok());
    }
}
