use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::validation::state_transition::state_transitions::shielded_common::{
    read_pool_total_balance, validate_anchor_exists, validate_minimum_pool_notes,
    validate_nullifiers,
};
use dpp::consensus::basic::document::InvalidDocumentTypeError;
use dpp::consensus::state::data_contract::data_contract_not_found_error::DataContractNotFoundError;
use dpp::consensus::state::document::document_type_not_private_error::DocumentTypeNotPrivateError;
use dpp::consensus::state::document::private_document_entry_size_mismatch_error::PrivateDocumentEntrySizeMismatchError;
use dpp::consensus::state::shielded::invalid_shielded_proof_error::InvalidShieldedProofError;
use dpp::consensus::state::state_error::StateError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::private_document_create_from_shielded_pool_transition::PrivateDocumentCreateFromShieldedPoolTransition;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::shielded::private_document_create_from_shielded_pool::PrivateDocumentCreateFromShieldedPoolTransitionAction;
use drive::state_transition_action::StateTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::private_document_create_from_shielded_pool) trait PrivateDocumentCreateFromShieldedPoolStateTransitionTransformIntoActionValidationV0
{
    fn transform_into_action_v0(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl PrivateDocumentCreateFromShieldedPoolStateTransitionTransformIntoActionValidationV0
    for PrivateDocumentCreateFromShieldedPoolTransition
{
    /// The Orchard proof and the exact fee have already been checked by the processor. Every
    /// failure here is refused without a fee, as every check runs before execution. The pool
    /// checks come first, as `ShieldedTransfer`'s (the pool notes floor, the anchor, unspent
    /// nullifiers, the pool balance), so junk with a made-up anchor costs no contract read.
    /// Then the target: the contract and the type must exist, the type must be private, the
    /// entries must take its `entrySize`, and its store must be there to take them, so the
    /// append in execution cannot fail.
    fn transform_into_action_v0(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let PrivateDocumentCreateFromShieldedPoolTransition::V0(v0) = self;

        let nullifiers: Vec<[u8; 32]> = v0.actions.iter().map(|a| a.nullifier).collect();

        let mut drive_operations = vec![];
        let current_total_balance =
            read_pool_total_balance(drive, transaction, &mut drive_operations, platform_version)?;

        if let Some(consensus_error) = validate_minimum_pool_notes(
            drive,
            transaction,
            &mut drive_operations,
            platform_version,
        )? {
            return Ok(consensus_error);
        }

        if let Some(consensus_error) = validate_anchor_exists(
            drive,
            &v0.anchor,
            transaction,
            &mut drive_operations,
            platform_version,
        )? {
            return Ok(consensus_error);
        }

        if let Some(consensus_error) = validate_nullifiers(
            drive,
            &nullifiers,
            transaction,
            &mut drive_operations,
            platform_version,
        )? {
            return Ok(consensus_error);
        }

        if current_total_balance < v0.value_balance {
            return Ok(ConsensusValidationResult::new_with_error(
                StateError::InvalidShieldedProofError(InvalidShieldedProofError::new(format!(
                    "shielded pool has insufficient balance: pool has {} but the private \
                     document create's fee is {}",
                    current_total_balance, v0.value_balance
                )))
                .into(),
            ));
        }

        let Some(contract_fetch_info) = drive.get_contract_with_fetch_info(
            v0.data_contract_id.to_buffer(),
            false,
            transaction,
            platform_version,
        )?
        else {
            return Ok(ConsensusValidationResult::new_with_error(
                DataContractNotFoundError::new(v0.data_contract_id).into(),
            ));
        };
        let Ok(document_type) = contract_fetch_info
            .contract
            .document_type_for_name(&v0.document_type_name)
        else {
            return Ok(ConsensusValidationResult::new_with_error(
                InvalidDocumentTypeError::new(v0.document_type_name.clone(), v0.data_contract_id)
                    .into(),
            ));
        };
        let Some(private_document) = document_type.private_document() else {
            return Ok(ConsensusValidationResult::new_with_error(
                DocumentTypeNotPrivateError::new(
                    v0.data_contract_id,
                    v0.document_type_name.clone(),
                )
                .into(),
            ));
        };
        let entry_size = private_document.entry_size();
        // Basic structure holds every entry to the first one's size, and to a u32.
        let size = v0
            .entries
            .first()
            .map(|entry| entry.len().min(u32::MAX as usize) as u32)
            .unwrap_or(0);
        if size != entry_size {
            return Ok(ConsensusValidationResult::new_with_error(
                PrivateDocumentEntrySizeMismatchError::new(
                    v0.data_contract_id,
                    v0.document_type_name.clone(),
                    entry_size,
                    size,
                )
                .into(),
            ));
        }

        // Every path that registers a private type creates its store; one missing, or of
        // another entry size, is corrupted state, which no transition is executed against.
        let Some(store_chunk_power) = drive.private_document_store_chunk_power_taking_entries(
            &v0.data_contract_id.to_buffer(),
            &v0.document_type_name,
            entry_size,
            transaction,
            platform_version,
        )?
        else {
            return Err(Error::Execution(ExecutionError::CorruptedDriveResponse(
                format!(
                    "the private document type \"{}\" of contract {} has no store taking entries \
                     of {} bytes",
                    v0.document_type_name, v0.data_contract_id, entry_size
                ),
            )));
        };

        // The minimum fee validation held the value balance to the fee for these entries at
        // the first entry's size, which is the type's `entrySize`, so the whole value balance
        // is the fee.
        let result = PrivateDocumentCreateFromShieldedPoolTransitionAction::try_from_transition(
            self,
            contract_fetch_info.clone(),
            store_chunk_power,
            current_total_balance,
            v0.value_balance,
        );

        Ok(result.map(|action| action.into()))
    }
}
