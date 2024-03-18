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
    let disabling_a_master_key = public_keys_to_disable
        .iter()
        .any(|key| key.security_level() == SecurityLevel::MASTER);

    let adding_a_master_key = public_keys_being_added
        .iter()
        .any(|key| key.security_level() == SecurityLevel::MASTER);

    // let's check that if we are disabling a master key that we are also enabling one
    if disabling_a_master_key ^ adding_a_master_key {
        Ok(SimpleConsensusValidationResult::new_with_error(
            BasicError::MasterPublicKeyUpdateError(MasterPublicKeyUpdateError::new(
                adding_a_master_key,
                disabling_a_master_key,
            ))
            .into(),
        ))
    } else {
        Ok(SimpleConsensusValidationResult::default())
    }
}
