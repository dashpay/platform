use crate::{
    document::document_transition::DocumentTransition,
    get_from_transition,
    mocks::{SMLStoreLike, SimplifiedMNListLike},
    prelude::Identifier,
    state_repository::StateRepositoryLike,
    util::json_value::JsonValueExt,
};

use super::{DataTriggerExecutionContext, DataTriggerExecutionResult};
use anyhow::{anyhow, bail};

const MAX_PERCENTAGE: i64 = 10000;
const PROPERTY_PAY_TO_ID: &str = "payToId";
const PROPERTY_PERCENTAGE: &str = "percentage";

pub async fn create_masternode_reward_shares_data_trigger<SR, S, L>(
    document_transition: &DocumentTransition,
    context: &DataTriggerExecutionContext<SR, S, L>,
    _top_level_identity: Option<&Identifier>,
) -> Result<DataTriggerExecutionResult, anyhow::Error>
where
    L: SimplifiedMNListLike,
    S: SMLStoreLike<L>,
    SR: StateRepositoryLike<S, L>,
{
    let result = DataTriggerExecutionResult::default();
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
    //TODO? should this be a float or integer
    let percentage = data.get_i64(PROPERTY_PERCENTAGE)?;

    // Do not allow creating document if ownerId is not in SML
    let sml_store = context.state_repository.fetch_sml_store().await?;
    let valid_master_nodes_list = sml_store.get_current_sml()?.get_valid_master_nodes();

    unimplemented!()
}
