use crate::error::Error;
use crate::execution::types::validation_operation::{OperationLike, ValidationOperation};
use dpp::fee::Credits;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::state_transition::AssetLockProved;
use dpp::identity::{KeyType, PartialIdentity};
use dpp::prelude::AssetLockProof;
use dpp::state_transition::identity_create_transition::accessors::IdentityCreateTransitionAccessorsV0;
use dpp::state_transition::identity_update_transition::accessors::IdentityUpdateTransitionAccessorsV0;
use dpp::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Getters;
use dpp::state_transition::{StateTransition, StateTransitionIdentitySigned};
use dpp::version::PlatformVersion;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignatureVerificationOperation {
    pub signature_type: KeyType,
}

impl SignatureVerificationOperation {
    pub fn new(signature_type: KeyType) -> Self {
        Self { signature_type }
    }
}

impl OperationLike for SignatureVerificationOperation {
    fn processing_cost(&self, platform_version: &PlatformVersion) -> Result<Credits, Error> {
        Ok(self
            .signature_type
            .signature_verify_cost(platform_version)?)
    }

    fn storage_cost(&self, _platform_version: &PlatformVersion) -> Result<Credits, Error> {
        Ok(0)
    }
}

pub fn signature_verification_operations_from_state_transition(
    state_transition: &StateTransition,
    identity: Option<&PartialIdentity>,
) -> Vec<ValidationOperation> {
    match state_transition {
        StateTransition::IdentityCreate(state_transition) => {
            let mut operations = state_transition
                .public_keys()
                .iter()
                .map(|key| {
                    ValidationOperation::SignatureVerification(SignatureVerificationOperation::new(
                        key.key_type(),
                    ))
                })
                .collect::<Vec<ValidationOperation>>();

            match state_transition.asset_lock_proof() {
                AssetLockProof::Instant(_) => {
                    operations.push(ValidationOperation::SignatureVerification(
                        SignatureVerificationOperation::new(KeyType::ECDSA_HASH160),
                    ))
                }
                _ => {}
            }

            operations
        }
        StateTransition::IdentityUpdate(state_transition) => {
            let mut operations = vec![];

            let identity = identity.unwrap();
            let key = identity
                .loaded_public_keys
                .get(&state_transition.signature_public_key_id())
                .unwrap();

            operations.push(ValidationOperation::SignatureVerification(
                SignatureVerificationOperation::new(key.key_type()),
            ));
            operations.extend(state_transition.public_keys_to_add().iter().map(|key| {
                ValidationOperation::SignatureVerification(SignatureVerificationOperation::new(
                    key.key_type(),
                ))
            }));

            operations
        }
        StateTransition::IdentityTopUp(state_transition) => {
            return match state_transition.asset_lock_proof() {
                AssetLockProof::Instant(_) => {
                    vec![ValidationOperation::SignatureVerification(
                        SignatureVerificationOperation::new(KeyType::ECDSA_HASH160),
                    )]
                }
                _ => vec![],
            };
        }
        state_transition => {
            let mut operations = vec![];

            let identity = identity.unwrap();
            let key = identity
                .loaded_public_keys
                .get(&state_transition.signature_public_key_id().unwrap())
                .unwrap();

            operations.push(ValidationOperation::SignatureVerification(
                SignatureVerificationOperation::new(key.key_type()),
            ));

            operations
        }
    }
}
