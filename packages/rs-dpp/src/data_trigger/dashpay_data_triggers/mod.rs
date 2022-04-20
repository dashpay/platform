use anyhow::{anyhow, bail};
use serde_json::Value as JsonValue;

use crate::{
    document::document_transition::DocumentTransition,
    errors::DataTriggerError,
    get_from_transition,
    prelude::Identifier,
    state_repository::{SMLStoreLike, SimplifiedMNListLike, StateRepositoryLike},
    util::json_value::JsonValueExt,
};

use super::{DataTriggerExecutionContext, DataTriggerExecutionResult};

const BLOCKS_SIZE_WINDOW: i64 = 8;
const PROPERTY_CORE_HEIGHT_CREATED_AT: &str = "coreHeightCreatedAt";
const PROPERTY_CORE_CHAIN_LOCKED_HEIGHT: &str = "coreChainLockedHeight";

pub async fn create_contract_request_data_trigger<SR, S, L>(
    document_transition: &DocumentTransition,
    context: &DataTriggerExecutionContext<SR, S, L>,
    _: Option<&Identifier>,
) -> Result<DataTriggerExecutionResult, anyhow::Error>
where
    L: SimplifiedMNListLike,
    S: SMLStoreLike<L>,
    SR: StateRepositoryLike<S, L>,
{
    let dt_create = match document_transition {
        DocumentTransition::Create(d) => d,
        _ => bail!(
            "the Document Transition {} isn't 'CREATE",
            get_from_transition!(document_transition, id)
        ),
    };
    let data = dt_create.data.as_ref().ok_or_else(|| {
        anyhow!(
            "data isn't defined in Data Transition {}",
            dt_create.base.id
        )
    })?;

    let core_height_created_at = data.get_i64(PROPERTY_CORE_HEIGHT_CREATED_AT)?;

    let latest_block_header: JsonValue = serde_json::from_slice(
        &context
            .state_repository
            .fetch_latest_platform_block_header()
            .await?,
    )?;

    let core_chain_locked_height =
        latest_block_header.get_i64(PROPERTY_CORE_CHAIN_LOCKED_HEIGHT)?;

    let height_window_start = core_chain_locked_height - BLOCKS_SIZE_WINDOW;
    let height_window_end = core_chain_locked_height + BLOCKS_SIZE_WINDOW;

    let mut result = DataTriggerExecutionResult::default();

    if core_height_created_at < height_window_start || core_height_created_at > height_window_end {
        let err = DataTriggerError::DataTriggerConditionError {
            data_contract_id: context.data_contract.id.clone(),
            document_transition_id: dt_create.base.id.clone(),
            message: format!(
                "Core height {} is out of block height window from {} to {}",
                core_height_created_at, height_window_start, height_window_end
            ),
            document_transition: Some(DocumentTransition::Create(dt_create.clone())),
            owner_id: Some(context.owner_id.clone()),
        };
        result.add_error(err.into());
    }

    Ok(result)
}
