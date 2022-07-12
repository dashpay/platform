use anyhow::{anyhow, bail};
use serde_json::json;

use crate::{
    data_trigger::new_error,
    document::{document_transition::DocumentTransition, Document},
    get_from_transition,
    mocks::SMLStore,
    prelude::Identifier,
    state_repository::StateRepositoryLike,
    util::{json_value::JsonValueExt, string_encoding::Encoding},
};

use super::{DataTriggerExecutionContext, DataTriggerExecutionResult};

const MAX_PERCENTAGE: u64 = 10000;
const PROPERTY_PAY_TO_ID: &str = "payToId";
const PROPERTY_PERCENTAGE: &str = "percentage";

pub async fn create_masternode_reward_shares_data_trigger<SR>(
    document_transition: &DocumentTransition,
    context: &DataTriggerExecutionContext<SR>,
    _top_level_identity: Option<&Identifier>,
) -> Result<DataTriggerExecutionResult, anyhow::Error>
where
    SR: StateRepositoryLike,
{
    let mut result = DataTriggerExecutionResult::default();
    let owner_id = context.owner_id.to_string(Encoding::Base58);

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

    let pay_to_id = data.get_string(PROPERTY_PAY_TO_ID)?;
    let percentage = data.get_u64(PROPERTY_PERCENTAGE)?;

    // Do not allow creating document if ownerId is not in SML
    let sml_store: SMLStore = context.state_repository.fetch_sml_store().await?;

    let valid_master_nodes_list = sml_store.get_current_sml()?.get_valid_master_nodes();

    let owner_id_in_sml = valid_master_nodes_list.iter().any(|entry| {
        hex::decode(&entry.pro_reg_tx_hash).expect("invalid hex value")
            == context.owner_id.to_buffer()
    });

    if !owner_id_in_sml {
        let err = new_error(
            context,
            dt_create,
            "Only masternode identities can share rewards".to_string(),
        );
        result.add_error(err.into());
    }

    // payToId identity exists
    let pay_to_identifier = Identifier::from_string(pay_to_id, Encoding::Base58)?;
    let maybe_identifier: Option<Vec<u8>> = context
        .state_repository
        .fetch_identity(&pay_to_identifier)
        .await?;

    if maybe_identifier.is_none() {
        let err = new_error(
            context,
            dt_create,
            format!("Identifier '{}' doesn't exists", pay_to_id),
        );
        result.add_error(err.into())
    }

    let documents: Vec<Document> = context
        .state_repository
        .fetch_documents(
            &context.data_contract.id,
            &dt_create.base.document_type,
            json!({
                "where" : [ [ "$owner_id", "==", owner_id ]]
            }),
        )
        .await?;

    let mut total_percent: u64 = percentage;
    for d in documents.iter() {
        total_percent += d.data.get_u64(PROPERTY_PERCENTAGE)?;
    }

    if total_percent > MAX_PERCENTAGE {
        let err = new_error(
            context,
            dt_create,
            format!("Percentage can not be more than {}", MAX_PERCENTAGE),
        );
        result.add_error(err.into());
    }

    Ok(result)
}
