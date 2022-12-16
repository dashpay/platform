use std::convert::TryInto;

use anyhow::{anyhow, Result};
use serde_json::{json, Value as JsonValue};

use crate::{
    contracts::withdrawals_contract,
    data_contract::DataContract,
    document::Document,
    prelude::Identifier,
    prelude::Identity,
    state_repository::StateRepositoryLike,
    state_transition::StateTransitionConvert,
    state_transition::StateTransitionLike,
    util::{entropy_generator::generate, json_value::JsonValueExt, string_encoding::Encoding},
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
        let data_contract_id = Identifier::from_string(
            &withdrawals_contract::system_ids().contract_id,
            Encoding::Base58,
        )?;
        let data_contract_owner_id = Identifier::from_string(
            &withdrawals_contract::system_ids().owner_id,
            Encoding::Base58,
        )?;

        let maybe_withdrawals_data_contract: Option<DataContract> = self
            .state_repository
            .fetch_data_contract(&data_contract_id, state_transition.get_execution_context())
            .await?;

        let withdrawals_data_contract = maybe_withdrawals_data_contract
            .ok_or_else(|| anyhow!("Withdrawals data contract not found"))?;

        let latest_platform_block_header: JsonValue = self
            .state_repository
            .fetch_latest_platform_block_header()
            .await?;

        let document_type = String::from(withdrawals_contract::types::WITHDRAWAL);
        let document_entropy = generate();
        let document_created_at_millis = latest_platform_block_header
            .get(PLATFORM_BLOCK_HEADER_TIME_PROPERTY)
            .ok_or_else(|| anyhow!("time property is not set in block header"))?
            .get_i64(PLATFORM_BLOCK_HEADER_TIME_SECONDS_PROPERTY)?
            * 1000;

        let document_data = json!({
            "amount": state_transition.amount,
            "coreFeePerByte": state_transition.core_fee_per_byte,
            "pooling": 0,
            "outputScript": state_transition.output_script.as_bytes(),
            "status": withdrawals_contract::statuses::QUEUED,
        });

        let document_id_bytes: [u8; 32] = state_transition
            .hash(true)?
            .try_into()
            .map_err(|_| anyhow!("Can't convert state transition hash to a document id"))?;

        // TODO: use DocumentFactory once it is complete
        let withdrawal_document = Document {
            protocol_version: state_transition.protocol_version,
            id: Identifier::new(document_id_bytes),
            document_type,
            revision: 0,
            data_contract_id,
            owner_id: data_contract_owner_id.clone(),
            created_at: Some(document_created_at_millis),
            updated_at: Some(document_created_at_millis),
            data: document_data,
            data_contract: withdrawals_data_contract,
            metadata: None,
            entropy: document_entropy,
        };

        self.state_repository
            .create_document(
                &withdrawal_document,
                state_transition.get_execution_context(),
            )
            .await?;

        let maybe_existing_identity: Option<Identity> = self
            .state_repository
            .fetch_identity(
                &state_transition.identity_id,
                state_transition.get_execution_context(),
            )
            .await?;

        let mut existing_identity =
            maybe_existing_identity.ok_or_else(|| anyhow!("Identity not found"))?;

        existing_identity.reduce_balance(state_transition.amount);

        let updated_identity_revision = existing_identity.get_revision() + 1;

        existing_identity.set_revision(updated_identity_revision);

        // TODO: we need to be able to batch state repository operations
        self.state_repository
            .update_identity(&existing_identity, state_transition.get_execution_context())
            .await
    }
}
