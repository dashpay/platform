use std::convert::TryInto;

use anyhow::{anyhow, Result};
use serde_json::{json, Value as JsonValue};

use crate::{
    contracts::withdrawals_contract,
    data_contract::DataContract,
    document::generate_document_id,
    document::Document,
    identity::state_transition::identity_credit_withdrawal_transition::Pooling,
    state_repository::StateRepositoryLike,
    state_transition::StateTransitionLike,
    util::{entropy_generator::generate, json_value::JsonValueExt},
};

use super::IdentityCreditWithdrawalTransition;

const PLATFORM_BLOCK_HEADER_TIME_PROPERTY: &str = "time";
const PLATFORM_BLOCK_HEADER_TIME_SECONDS_PROPERTY: &str = "seconds";

pub struct ApplyIdentityCreditWithdrawalTransition<SR>
where
    SR: StateRepositoryLike,
{
    state_repository: SR,
}

impl<SR> ApplyIdentityCreditWithdrawalTransition<SR>
where
    SR: StateRepositoryLike,
{
    pub fn new(state_repository: SR) -> Self {
        Self { state_repository }
    }

    pub async fn apply_identity_credit_withdrawal_transition(
        &self,
        state_transition: &IdentityCreditWithdrawalTransition,
    ) -> Result<()> {
        let data_contract_id = withdrawals_contract::CONTRACT_ID.clone();
        let data_contract_owner_id = withdrawals_contract::OWNER_ID.clone();

        let maybe_withdrawals_data_contract: Option<DataContract> = self
            .state_repository
            .fetch_data_contract(&data_contract_id, state_transition.get_execution_context())
            .await?
            .map(TryInto::try_into)
            .transpose()
            .map_err(Into::into)?;

        let withdrawals_data_contract = maybe_withdrawals_data_contract
            .ok_or_else(|| anyhow!("Withdrawals data contract not found"))?;

        let latest_platform_block_header: JsonValue = self
            .state_repository
            .fetch_latest_platform_block_header()
            .await?;

        let document_type = String::from(withdrawals_contract::types::WITHDRAWAL);
        let document_entropy = generate()?;
        let document_created_at_millis = latest_platform_block_header
            .get(PLATFORM_BLOCK_HEADER_TIME_PROPERTY)
            .ok_or_else(|| anyhow!("time property is not set in block header"))?
            .get_i64(PLATFORM_BLOCK_HEADER_TIME_SECONDS_PROPERTY)?
            * 1000;

        let document_data = json!({
            withdrawals_contract::property_names::AMOUNT: state_transition.amount,
            withdrawals_contract::property_names::CORE_FEE_PER_BYTE: state_transition.core_fee_per_byte,
            withdrawals_contract::property_names::POOLING: Pooling::Never,
            withdrawals_contract::property_names::OUTPUT_SCRIPT: state_transition.output_script.as_bytes(),
            withdrawals_contract::property_names::STATUS: withdrawals_contract::Status::QUEUED,
        });

        let document_id = generate_document_id::generate_document_id(
            &data_contract_id,
            &data_contract_owner_id,
            &document_type,
            &document_entropy,
        );

        // TODO: use DocumentFactory once it is complete
        let withdrawal_document = Document {
            protocol_version: state_transition.protocol_version,
            id: document_id,
            document_type,
            revision: 0,
            data_contract_id,
            owner_id: data_contract_owner_id.clone(),
            created_at: Some(document_created_at_millis),
            updated_at: Some(document_created_at_millis),
            data: document_data,
            data_contract: withdrawals_data_contract,
            metadata: None,
            entropy: [0; 32],
        };

        self.state_repository
            .create_document(
                &withdrawal_document,
                state_transition.get_execution_context(),
            )
            .await?;

        let maybe_existing_identity = self
            .state_repository
            .fetch_identity(
                &state_transition.identity_id,
                state_transition.get_execution_context(),
            )
            .await?
            .map(TryInto::try_into)
            .transpose()
            .map_err(Into::into)?;

        let mut existing_identity =
            maybe_existing_identity.ok_or_else(|| anyhow!("Identity not found"))?;

        existing_identity.reduce_balance(state_transition.amount);
        existing_identity.increment_revision()?;

        // TODO: we need to be able to batch state repository operations
        self.state_repository
            .update_identity(&existing_identity, state_transition.get_execution_context())
            .await
    }
}
