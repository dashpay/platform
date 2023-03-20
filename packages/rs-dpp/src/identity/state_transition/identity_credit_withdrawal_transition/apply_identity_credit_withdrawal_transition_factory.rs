use anyhow::{anyhow, Result};
use dashcore::{consensus, BlockHeader};
use lazy_static::__Deref;
use std::convert::TryInto;

use serde_json::json;

use crate::{
    contracts::withdrawals_contract, data_contract::DataContract, document::generate_document_id,
    document::Document, identity::state_transition::identity_credit_withdrawal_transition::Pooling,
    state_repository::StateRepositoryLike, state_transition::StateTransitionLike,
    util::entropy_generator::generate,
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
        let data_contract_id = withdrawals_contract::CONTRACT_ID.deref();

        let maybe_withdrawals_data_contract: Option<DataContract> = self
            .state_repository
            .fetch_data_contract(data_contract_id, state_transition.get_execution_context())
            .await?
            .map(TryInto::try_into)
            .transpose()
            .map_err(Into::into)?;

        let withdrawals_data_contract = maybe_withdrawals_data_contract
            .ok_or_else(|| anyhow!("Withdrawals data contract not found"))?;

        let latest_platform_block_header_bytes: Vec<u8> = self
            .state_repository
            .fetch_latest_platform_block_header()
            .await?;

        let latest_platform_block_header: BlockHeader =
            consensus::deserialize(&latest_platform_block_header_bytes)?;

        let document_type = String::from(withdrawals_contract::document_types::WITHDRAWAL);
        let document_created_at_millis: i64 = latest_platform_block_header.time as i64 * 1000i64;

        let document_data = json!({
            withdrawals_contract::property_names::AMOUNT: state_transition.amount,
            withdrawals_contract::property_names::CORE_FEE_PER_BYTE: state_transition.core_fee_per_byte,
            withdrawals_contract::property_names::POOLING: Pooling::Never,
            withdrawals_contract::property_names::OUTPUT_SCRIPT: state_transition.output_script.as_bytes(),
            withdrawals_contract::property_names::STATUS: withdrawals_contract::WithdrawalStatus::QUEUED,
        });

        let mut document_id;

        loop {
            let document_entropy = generate()?;

            document_id = generate_document_id::generate_document_id(
                data_contract_id,
                &state_transition.identity_id,
                &document_type,
                &document_entropy,
            );

            let documents = self
                .state_repository
                .fetch_documents(
                    withdrawals_contract::CONTRACT_ID.deref(),
                    withdrawals_contract::document_types::WITHDRAWAL,
                    json!({
                        "where": [
                            ["$id", "==", document_id],
                        ],
                    }),
                    &state_transition.execution_context,
                )
                .await?;

            if documents.is_empty() {
                break;
            }
        }

        // TODO: use DocumentFactory once it is complete
        let withdrawal_document = Document {
            protocol_version: state_transition.protocol_version,
            id: document_id,
            document_type,
            revision: 0,
            data_contract_id: *data_contract_id,
            owner_id: state_transition.identity_id,
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

        // TODO: we need to be able to batch state repository operations
        self.state_repository
            .remove_from_identity_balance(
                &state_transition.identity_id,
                state_transition.amount,
                state_transition.get_execution_context(),
            )
            .await?;

        self.state_repository
            .remove_from_system_credits(
                state_transition.amount,
                state_transition.get_execution_context(),
            )
            .await
    }
}
