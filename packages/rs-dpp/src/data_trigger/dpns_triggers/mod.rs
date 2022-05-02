use anyhow::Context;
use anyhow::{anyhow, bail};
use serde_json::{json, Value as JsonValue};

use crate::document::Document;
use crate::util::hash::hash;
use crate::util::string_encoding::Encoding;
use crate::{
    document::document_transition::DocumentTransition, get_from_transition, prelude::Identifier,
    state_repository::StateRepositoryLike, util::json_value::JsonValueExt,
};

use super::{new_error, DataTriggerExecutionContext, DataTriggerExecutionResult};

const MAX_PRINTABLE_DOMAIN_NAME_LENGTH: usize = 253;
const PROPERTY_LABEL: &str = "label";
const PROPERTY_NORMALIZED_LABEL: &str = "normalizedLabel";
const PROPERTY_NORMALIZED_PARENT_DOMAIN_NAME: &str = "normalizedParentDomainName";
const PROPERTY_PREORDER_SALT: &str = "prorderSalt";
const PROPERTY_ALLOW_SUBDOMAINS: &str = "subdomainRules.allowSubdomains";
const PROPERTY_RECORDS: &str = "records";
const PROPERTY_DASH_UNIQUE_IDENTITY_ID: &str = "dashUniqueIdentityId";
const PROPERTY_DASH_ALIAS_IDENTITY_ID: &str = "dashAliasIdentityId";

pub async fn create_domain_data_trigger<SR>(
    document_transition: &DocumentTransition,
    context: &DataTriggerExecutionContext<SR>,
    top_level_identity: Option<&Identifier>,
) -> Result<DataTriggerExecutionResult, anyhow::Error>
where
    SR: StateRepositoryLike,
{
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
    let owner_id = context.owner_id.to_string(Encoding::Base58);
    let label = data.get_string(PROPERTY_LABEL)?;
    let normalized_label = data.get_string(PROPERTY_NORMALIZED_LABEL)?;
    let normalized_parent_domain_name = data.get_string(PROPERTY_NORMALIZED_PARENT_DOMAIN_NAME)?;

    let preorder_salt = data.get_bytes(PROPERTY_PREORDER_SALT)?;
    let records = data
        .get(PROPERTY_RECORDS)
        .ok_or_else(|| anyhow!("property '{}' doesn't exist", PROPERTY_RECORDS))?;

    let rule_allow_subdomains = data
        .get_value(PROPERTY_ALLOW_SUBDOMAINS)?
        .as_bool()
        .ok_or_else(|| anyhow!("property '{}' isn't a bool", PROPERTY_ALLOW_SUBDOMAINS))?;

    let mut result = DataTriggerExecutionResult::default();
    let full_domain_name = normalized_label;

    if full_domain_name.len() > MAX_PRINTABLE_DOMAIN_NAME_LENGTH {
        let err = new_error(
            context,
            dt_create,
            format!(
                "Full domain name length can not be more than {} characters long but got {}",
                MAX_PRINTABLE_DOMAIN_NAME_LENGTH,
                full_domain_name.len()
            ),
        );
        result.add_error(err.into())
    }

    if normalized_label != label.to_lowercase() {
        let err = new_error(
            context,
            dt_create,
            "Normalized label doesn't match label".to_string(),
        );
        result.add_error(err.into());
    }

    if let Some(JsonValue::String(ref id)) = records.get(PROPERTY_DASH_UNIQUE_IDENTITY_ID) {
        if id != &owner_id {
            let err = new_error(
                context,
                dt_create,
                format!(
                    "ownerId {} doesn't match {} {}",
                    owner_id, PROPERTY_DASH_UNIQUE_IDENTITY_ID, id
                ),
            );
            result.add_error(err.into())
        }
    }

    if let Some(JsonValue::String(ref id)) = records.get(PROPERTY_DASH_ALIAS_IDENTITY_ID) {
        if id != &owner_id {
            let err = new_error(
                context,
                dt_create,
                format!(
                    "ownerId {} doesn't match {} {}",
                    owner_id, PROPERTY_DASH_ALIAS_IDENTITY_ID, id
                ),
            );
            result.add_error(err.into());
        }
    }

    if normalized_parent_domain_name.is_empty() && &context.owner_id != top_level_identity {
        let err = new_error(
            context,
            dt_create,
            "Can't create top level domain for this identity".to_string(),
        );
        result.add_error(err.into())
    }

    if !normalized_parent_domain_name.is_empty() {
        //? What is the `normalized_parent_name`. Are we sure the content is a valid dot-separated data
        let mut parent_domain_segments = normalized_parent_domain_name.split('.');
        let parent_domain_label = parent_domain_segments.next().unwrap().to_string();
        let grand_parent_domain_name = parent_domain_segments.collect::<Vec<&str>>().join(".");

        let documents: Vec<Document> = context
            .state_repository
            .fetch_documents(
                &context.data_contract.id,
                &dt_create.base.document_type,
                json!({
                    "where" : [
                        ["normalizedParentDomainName", "==", grand_parent_domain_name],
                        ["normalizedLabel", "==", parent_domain_label]
                    ]
                }),
            )
            .await?;

        if documents.is_empty() {
            let err = new_error(
                context,
                dt_create,
                "Parent domain is not present".to_string(),
            );
            result.add_error(err.into());
        }
        let parent_domain = &documents[0];

        if rule_allow_subdomains {
            let err = new_error(
                context,
                dt_create,
                "Allowing subdomains registration is forbidden for non top level domains"
                    .to_string(),
            );
            result.add_error(err.into());
        }

        if (!parent_domain
            .data
            .get_value(PROPERTY_ALLOW_SUBDOMAINS)?
            .as_bool()
            .unwrap())
            && context.owner_id != parent_domain.owner_id
        {
            let err = new_error(
                context,
                dt_create,
                "The subdomain can be created only by the parent domain owner".to_string(),
            );
            result.add_error(err.into());
        }
    }

    let mut salted_domain_buffer: Vec<u8> = vec![];
    salted_domain_buffer.extend(preorder_salt);
    salted_domain_buffer.extend(full_domain_name.to_owned().as_bytes());

    let salted_domain_hash = hash(salted_domain_buffer);

    let preorder_documents: Vec<Document> = context
        .state_repository
        .fetch_documents(
            &context.data_contract.id,
            "preorder",
            json!({
                //? should this be a base64 encoded
                "where" : [["saltedDomainHash", "==", salted_domain_hash]]
            }),
        )
        .await?;

    if preorder_documents.is_empty() {
        let err = new_error(
            context,
            dt_create,
            "preorderDocument was not found".to_string(),
        );
        result.add_error(err.into())
    }

    Ok(result)
}
