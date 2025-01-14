use crate::drive::identity::key::fetch::{
    IdentityKeysRequest, KeyIDIdentityPublicKeyPairBTreeMap, KeyRequestType,
};
use crate::drive::Drive;
use crate::error::Error;
use crate::state_transition_action::identity::identity_credit_withdrawal::v0::IdentityCreditWithdrawalTransitionActionV0;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::identity::identity_public_key_is_disabled_error::IdentityPublicKeyIsDisabledError;
use dpp::consensus::state::identity::missing_transfer_key_error::MissingTransferKeyError;
use dpp::consensus::state::identity::no_transfer_key_for_core_withdrawal_available_error::NoTransferKeyForCoreWithdrawalAvailableError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::ConsensusError;
use dpp::data_contracts::withdrawals_contract;
use dpp::data_contracts::withdrawals_contract::v1::document_types::withdrawal;
use dpp::document::{Document, DocumentV0};
use dpp::identity::core_script::CoreScript;
use dpp::identity::hash::IdentityPublicKeyHashMethodsV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{IdentityPublicKey, KeyType};
use dpp::platform_value::platform_value;
use dpp::state_transition::identity_credit_withdrawal_transition::v1::IdentityCreditWithdrawalTransitionV1;
use dpp::state_transition::state_transitions::identity::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0;
use dpp::validation::ConsensusValidationResult;
use dpp::withdrawal::Pooling;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl IdentityCreditWithdrawalTransitionActionV0 {
    /// from identity credit withdrawal
    pub fn from_identity_credit_withdrawal_v0(
        identity_credit_withdrawal: &IdentityCreditWithdrawalTransitionV0,
        creation_time_ms: u64,
    ) -> Self {
        let mut entropy = Vec::new();
        entropy.extend_from_slice(&identity_credit_withdrawal.nonce.to_be_bytes());
        entropy.extend_from_slice(identity_credit_withdrawal.output_script.as_bytes());

        let document_id = Document::generate_document_id_v0(
            &withdrawals_contract::ID,
            &identity_credit_withdrawal.identity_id,
            withdrawal::NAME,
            &entropy,
        );

        let document_data = platform_value!({
            withdrawal::properties::AMOUNT: identity_credit_withdrawal.amount,
            withdrawal::properties::CORE_FEE_PER_BYTE: identity_credit_withdrawal.core_fee_per_byte,
            // TODO(withdrawals): replace with actual value from state transition once pooling is done
            withdrawal::properties::POOLING: Pooling::Never,
            withdrawal::properties::OUTPUT_SCRIPT: identity_credit_withdrawal.output_script.as_bytes(),
            withdrawal::properties::STATUS: withdrawals_contract::WithdrawalStatus::QUEUED,
        });

        let withdrawal_document = DocumentV0 {
            id: document_id,
            owner_id: identity_credit_withdrawal.identity_id,
            properties: document_data.into_btree_string_map().unwrap(),
            revision: Some(1),
            created_at: Some(creation_time_ms),
            updated_at: Some(creation_time_ms),
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
        }
        .into();

        IdentityCreditWithdrawalTransitionActionV0 {
            identity_id: identity_credit_withdrawal.identity_id,
            nonce: identity_credit_withdrawal.nonce,
            prepared_withdrawal_document: withdrawal_document,
            amount: identity_credit_withdrawal.amount,
            user_fee_increase: identity_credit_withdrawal.user_fee_increase,
        }
    }

    /// from identity credit withdrawal v1
    pub fn try_from_identity_credit_withdrawal_v1(
        drive: &Drive,
        tx: TransactionArg,
        identity_credit_withdrawal: &IdentityCreditWithdrawalTransitionV1,
        block_info: &BlockInfo,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<Self>, Error> {
        let output_script_bytes = if let Some(output_script) =
            &identity_credit_withdrawal.output_script
        {
            output_script.to_bytes()
        } else {
            let key_request = IdentityKeysRequest {
                identity_id: identity_credit_withdrawal.identity_id.to_buffer(),
                request_type: KeyRequestType::RecentWithdrawalKeys,
                limit: Some(1),
                offset: None,
            };
            let key: Option<IdentityPublicKey> =
                drive.fetch_identity_keys(key_request, tx, platform_version)?;
            let Some(mut key) = key else {
                return Ok(ConsensusValidationResult::new_with_error(
                    ConsensusError::StateError(StateError::MissingTransferKeyError(
                        MissingTransferKeyError::new(identity_credit_withdrawal.identity_id),
                    )),
                ));
            };
            if key.is_disabled() {
                // The first key is disabled, let's look at some more withdrawal keys to find one that isn't disabled
                let after_first_key_request = IdentityKeysRequest {
                    identity_id: identity_credit_withdrawal.identity_id.to_buffer(),
                    request_type: KeyRequestType::RecentWithdrawalKeys,
                    limit: Some(5),
                    offset: Some(1),
                };
                let other_keys: KeyIDIdentityPublicKeyPairBTreeMap =
                    drive.fetch_identity_keys(after_first_key_request, tx, platform_version)?;

                if let Some(found_non_disabled_key) = other_keys
                    .values()
                    .rev()
                    .find(|identity_public_key| !identity_public_key.is_disabled())
                    .cloned()
                {
                    key = found_non_disabled_key
                } else {
                    return Ok(ConsensusValidationResult::new_with_error(
                        ConsensusError::StateError(StateError::IdentityPublicKeyIsDisabledError(
                            IdentityPublicKeyIsDisabledError::new(key.id()),
                        )),
                    ));
                }
            }
            match key.key_type() {
                KeyType::ECDSA_HASH160 => {
                    // We should get the withdrawal address
                    CoreScript::new_p2pkh(key.public_key_hash()?).to_bytes()
                }
                KeyType::BIP13_SCRIPT_HASH => {
                    // We should get the withdrawal address
                    CoreScript::new_p2sh(key.public_key_hash()?).to_bytes()
                }
                _ => {
                    return Ok(ConsensusValidationResult::new_with_error(
                        ConsensusError::StateError(
                            StateError::NoTransferKeyForCoreWithdrawalAvailableError(
                                NoTransferKeyForCoreWithdrawalAvailableError::new(
                                    identity_credit_withdrawal.identity_id,
                                ),
                            ),
                        ),
                    ));
                }
            }
        };

        let mut entropy = Vec::new();
        entropy.extend_from_slice(&identity_credit_withdrawal.nonce.to_be_bytes());
        entropy.extend_from_slice(output_script_bytes.as_slice());

        let document_id = Document::generate_document_id_v0(
            &withdrawals_contract::ID,
            &identity_credit_withdrawal.identity_id,
            withdrawal::NAME,
            &entropy,
        );

        let document_data = platform_value!({
            withdrawal::properties::AMOUNT: identity_credit_withdrawal.amount,
            withdrawal::properties::CORE_FEE_PER_BYTE: identity_credit_withdrawal.core_fee_per_byte,
            // TODO(withdrawals): replace with actual value from state transition once pooling is done
            withdrawal::properties::POOLING: Pooling::Never,
            withdrawal::properties::OUTPUT_SCRIPT: output_script_bytes,
            withdrawal::properties::STATUS: withdrawals_contract::WithdrawalStatus::QUEUED,
        });

        let withdrawal_document = DocumentV0 {
            id: document_id,
            owner_id: identity_credit_withdrawal.identity_id,
            properties: document_data.into_btree_string_map()?,
            revision: Some(1),
            created_at: Some(block_info.time_ms),
            updated_at: Some(block_info.time_ms),
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
        }
        .into();

        Ok(ConsensusValidationResult::new_with_data(
            IdentityCreditWithdrawalTransitionActionV0 {
                identity_id: identity_credit_withdrawal.identity_id,
                nonce: identity_credit_withdrawal.nonce,
                prepared_withdrawal_document: withdrawal_document,
                amount: identity_credit_withdrawal.amount,
                user_fee_increase: identity_credit_withdrawal.user_fee_increase,
            },
        ))
    }
}
