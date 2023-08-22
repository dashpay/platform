use anyhow::{anyhow, Result};
use dashcore::{consensus, BlockHeader};
use lazy_static::__Deref;
use std::collections::BTreeMap;
use std::convert::TryInto;

use platform_value::{platform_value, Bytes32, Value};

use crate::contracts::withdrawals_contract::property_names;
use crate::data_contract::document_type::document_type::PROTOCOL_VERSION;
use crate::document::{DocumentV0, ExtendedDocument};
use crate::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use crate::util::entropy_generator::DefaultEntropyGenerator;
use crate::version::LATEST_PLATFORM_VERSION;
use crate::{
    contracts::withdrawals_contract, data_contract::DataContract, document::generate_document_id,
    document::Document, identity::state_transition::identity_credit_withdrawal_transition::Pooling,
    state_repository::StateRepositoryLike, util::entropy_generator::EntropyGenerator,
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
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<()> {
        let data_contract_id = withdrawals_contract::CONTRACT_ID.deref();

        let maybe_withdrawals_data_contract: Option<DataContract> = self
            .state_repository
            .fetch_data_contract(data_contract_id, Some(execution_context))
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
        let document_created_at_millis: u64 = latest_platform_block_header.time as u64 * 1000u64;

        let document_properties = BTreeMap::from([
            (
                property_names::AMOUNT.to_string(),
                Value::U64(state_transition.amount),
            ),
            (
                property_names::CORE_FEE_PER_BYTE.to_string(),
                Value::U32(state_transition.core_fee_per_byte),
            ),
            (
                property_names::POOLING.to_string(),
                Value::U8(Pooling::Never as u8),
            ),
            (
                property_names::OUTPUT_SCRIPT.to_string(),
                Value::Bytes(state_transition.output_script.as_bytes().to_vec()),
            ),
            (
                property_names::STATUS.to_string(),
                Value::U8(withdrawals_contract::WithdrawalStatus::QUEUED as u8),
            ),
        ]);

        let mut document_id;

        let generator = DefaultEntropyGenerator;
        loop {
            let document_entropy = generator.generate()?;

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
                    platform_value!({
                        "where": [
                            ["$id", "==", document_id.to_buffer()],
                        ],
                    }),
                    Some(execution_context),
                )
                .await?;

            if documents.is_empty() {
                break;
            }
        }

        let withdrawal_document: Document = DocumentV0 {
            id: document_id,
            revision: None,
            owner_id: state_transition.identity_id,
            created_at: Some(document_created_at_millis),
            updated_at: Some(document_created_at_millis),
            properties: document_properties,
        }
        .into();

        let extended_withdrawal_document = ExtendedDocumentV0 {
            document_type_name: document_type,
            data_contract_id: withdrawals_data_contract.id(),
            document: withdrawal_document,
            data_contract: withdrawals_data_contract,
            metadata: None,
            entropy: Bytes32::default(),
        }
        .into();

        self.state_repository
            .create_document(&extended_withdrawal_document, Some(execution_context))
            .await?;

        // TODO: we need to be able to batch state repository operations
        self.state_repository
            .remove_from_identity_balance(
                &state_transition.identity_id,
                state_transition.amount,
                Some(execution_context),
            )
            .await?;

        self.state_repository
            .remove_from_system_credits(state_transition.amount, Some(execution_context))
            .await
    }
}
