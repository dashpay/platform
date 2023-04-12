use crate::identity::state_transition::asset_lock_proof::{
    AssetLockProof, AssetLockProofType, ChainAssetLockProofStructureValidator,
    InstantAssetLockProofStructureValidator, PublicKeyHash,
};
use crate::state_repository::StateRepositoryLike;
use crate::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use crate::validation::ConsensusValidationResult;
use crate::NonConsensusError;
use platform_value::Value;

pub struct AssetLockProofValidator<SR: StateRepositoryLike> {
    instant_asset_lock_structure_validator: InstantAssetLockProofStructureValidator<SR>,
    chain_asset_lock_structure_validator: ChainAssetLockProofStructureValidator<SR>,
}

impl<SR: StateRepositoryLike> AssetLockProofValidator<SR> {
    pub fn new(
        instant_asset_lock_structure_validator: InstantAssetLockProofStructureValidator<SR>,
        chain_asset_lock_structure_validator: ChainAssetLockProofStructureValidator<SR>,
    ) -> Self {
        Self {
            instant_asset_lock_structure_validator,
            chain_asset_lock_structure_validator,
        }
    }

    pub async fn validate_structure(
        &self,
        asset_lock_proof_object: &Value,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<ConsensusValidationResult<PublicKeyHash>, NonConsensusError> {
        let asset_lock_type = AssetLockProof::type_from_raw_value(asset_lock_proof_object);
        if let Some(proof_type) = asset_lock_type {
            match proof_type {
                AssetLockProofType::Instant => {
                    self.instant_asset_lock_structure_validator
                        .validate(asset_lock_proof_object, execution_context)
                        .await
                }
                AssetLockProofType::Chain => {
                    self.chain_asset_lock_structure_validator
                        .validate(asset_lock_proof_object, execution_context)
                        .await
                }
            }
        } else {
            Err(NonConsensusError::SerdeJsonError(String::from(
                "Asset lock proof should have type field",
            )))
        }
    }
}
