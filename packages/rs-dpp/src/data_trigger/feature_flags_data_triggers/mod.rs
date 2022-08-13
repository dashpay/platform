use anyhow::{anyhow, bail, Context};
use serde_json::Value as JsonValue;

use crate::{
    data_trigger::create_error, document::document_transition::DocumentTransition,
    get_from_transition, prelude::Identifier, state_repository::StateRepositoryLike,
    util::json_value::JsonValueExt,
};

use super::{DataTriggerExecutionContext, DataTriggerExecutionResult};

const PROPERTY_BLOCK_HEIGHT: &str = "height";
const PROPERTY_ENABLE_AT_HEIGHT: &str = "enableAtHeight";

pub async fn create_feature_flag_data_trigger<'a, SR>(
    document_transition: &DocumentTransition,
    context: &DataTriggerExecutionContext<'a, SR>,
    top_level_identity: Option<&Identifier>,
) -> Result<DataTriggerExecutionResult, anyhow::Error>
where
    SR: StateRepositoryLike,
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

    let latest_block_header: JsonValue = context
        .state_repository
        .fetch_latest_platform_block_header()
        .await?;

    let block_height = latest_block_header.get_i64(PROPERTY_BLOCK_HEIGHT)?;
    let enable_at_height = data.get_i64(PROPERTY_ENABLE_AT_HEIGHT)?;

    if enable_at_height < block_height {
        let err = create_error(
            context,
            dt_create,
            "This identity can't activate selected feature flag".to_string(),
        );
        result.add_error(err.into());
        return Ok(result);
    }

    if context.owner_id != top_level_identity {
        let err = create_error(
            context,
            dt_create,
            "This Identity can't activate selected feature flag".to_string(),
        );
        result.add_error(err.into());
    }

    Ok(result)
}
