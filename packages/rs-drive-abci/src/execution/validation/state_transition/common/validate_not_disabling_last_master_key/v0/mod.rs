use crate::error::Error;
use dpp::consensus::basic::BasicError;
use dpp::consensus::state::identity::master_public_key_update_error::MasterPublicKeyUpdateError;

use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{IdentityPublicKey, SecurityLevel};
use dpp::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Getters;

use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use dpp::validation::SimpleConsensusValidationResult;

/// This will validate that all keys are valid against the state
pub(super) fn validate_master_key_uniqueness_v0(
    public_keys_being_added: &[IdentityPublicKeyInCreation],
    public_keys_to_disable: &[IdentityPublicKey],
) -> Result<SimpleConsensusValidationResult, Error> {
    let master_keys_to_disable_count = public_keys_to_disable
        .iter()
        .filter(|key| key.security_level() == SecurityLevel::MASTER)
        .count();

    let master_keys_being_added_count = public_keys_being_added
        .iter()
        .filter(|key| key.security_level() == SecurityLevel::MASTER)
        .count();

    // Check that at most one master key is being disabled and at most one is being added
    if master_keys_to_disable_count > 1
        || master_keys_being_added_count > 1
        || ((master_keys_to_disable_count == 1) ^ (master_keys_being_added_count == 1))
    {
        Ok(SimpleConsensusValidationResult::new_with_error(
            BasicError::MasterPublicKeyUpdateError(MasterPublicKeyUpdateError::new(
                master_keys_being_added_count,
                master_keys_to_disable_count,
            ))
            .into(),
        ))
    } else {
        Ok(SimpleConsensusValidationResult::default())
    }
}
