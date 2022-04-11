use crate::{
    data_trigger::new_error,
    document::document_transition::DocumentTransition,
    get_from_transition,
    prelude::Identifier,
    state_repository::StateRepositoryLike,
    util::{json_value::JsonValueExt, string_encoding::Encoding},
};

use super::{DataTriggerExecutionContext, DataTriggerExecutionResult};
use anyhow::{anyhow, bail, Context};

const PROPERTY_BLOCK_HEIGHT: &str = "height";
const PROPERTY_ENABLE_AT_HEIGHT: &str = "enableAtHeight";

pub async fn create_features_flag_data_trigger<ST>(
    document_transition: &DocumentTransition,
    context: &DataTriggerExecutionContext<ST>,
    top_level_identity: Option<&Identifier>,
) -> Result<DataTriggerExecutionResult, anyhow::Error>
where
    ST: StateRepositoryLike,
{
    let mut result = DataTriggerExecutionResult::default();
    let top_level_identity = top_level_identity.context("Top Level Identity must be defined")?;

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

    let block_height = context
        .state_repository
        .fetch_latest_platform_block_header()
        .await?
        .get_i64(PROPERTY_BLOCK_HEIGHT)?;
    let enable_at_height = data.get_i64(PROPERTY_ENABLE_AT_HEIGHT)?;

    if enable_at_height < block_height {
        let err = new_error(
            context,
            dt_create,
            "This identity can't activate selected feature flag".to_string(),
        );
        result.add_error(err.into());
        return Ok(result);
    }

    if &context.owner_id != top_level_identity {
        let err = new_error(
            context,
            dt_create,
            "This Identity can't activate selected feature flag".to_string(),
        );
        result.add_error(err.into());
    }

    Ok(result)
}
