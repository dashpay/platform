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

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::{KeyType, Purpose};
    use dpp::platform_value::BinaryData;
    use dpp::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;

    fn make_identity_public_key(id: u32, security_level: SecurityLevel) -> IdentityPublicKey {
        IdentityPublicKeyV0 {
            id: id.into(),
            purpose: Purpose::AUTHENTICATION,
            security_level,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(vec![0u8; 33]),
            disabled_at: None,
        }
        .into()
    }

    fn make_key_in_creation(id: u32, security_level: SecurityLevel) -> IdentityPublicKeyInCreation {
        IdentityPublicKeyInCreationV0 {
            id: id.into(),
            key_type: KeyType::ECDSA_SECP256K1,
            purpose: Purpose::AUTHENTICATION,
            security_level,
            contract_bounds: None,
            read_only: false,
            data: BinaryData::new(vec![0u8; 33]),
            signature: BinaryData::default(),
        }
        .into()
    }

    #[test]
    fn should_pass_when_no_master_keys_involved() {
        let result = validate_master_key_uniqueness_v0(&[], &[]).expect("should succeed");
        assert!(result.is_valid(), "should be valid with empty inputs");
    }

    #[test]
    fn should_pass_when_only_non_master_keys_added() {
        let keys_added = vec![
            make_key_in_creation(10, SecurityLevel::HIGH),
            make_key_in_creation(11, SecurityLevel::MEDIUM),
        ];
        let result = validate_master_key_uniqueness_v0(&keys_added, &[]).expect("should succeed");
        assert!(
            result.is_valid(),
            "should be valid when only non-master keys are added"
        );
    }

    #[test]
    fn should_pass_when_only_non_master_keys_disabled() {
        let keys_to_disable = vec![
            make_identity_public_key(1, SecurityLevel::HIGH),
            make_identity_public_key(2, SecurityLevel::MEDIUM),
        ];
        let result =
            validate_master_key_uniqueness_v0(&[], &keys_to_disable).expect("should succeed");
        assert!(
            result.is_valid(),
            "should be valid when only non-master keys are disabled"
        );
    }

    #[test]
    fn should_pass_when_one_master_key_disabled_and_one_added() {
        let keys_added = vec![make_key_in_creation(10, SecurityLevel::MASTER)];
        let keys_to_disable = vec![make_identity_public_key(1, SecurityLevel::MASTER)];
        let result = validate_master_key_uniqueness_v0(&keys_added, &keys_to_disable)
            .expect("should succeed");
        assert!(
            result.is_valid(),
            "should be valid when exactly one master key is rotated"
        );
    }

    #[test]
    fn should_fail_when_adding_master_key_without_disabling_one() {
        let keys_added = vec![make_key_in_creation(10, SecurityLevel::MASTER)];
        let result = validate_master_key_uniqueness_v0(&keys_added, &[]).expect("should succeed");
        assert!(
            !result.is_valid(),
            "should be invalid when adding master key without disabling one"
        );
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn should_fail_when_disabling_master_key_without_adding_one() {
        let keys_to_disable = vec![make_identity_public_key(1, SecurityLevel::MASTER)];
        let result =
            validate_master_key_uniqueness_v0(&[], &keys_to_disable).expect("should succeed");
        assert!(
            !result.is_valid(),
            "should be invalid when disabling master key without adding one"
        );
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn should_fail_when_disabling_two_master_keys() {
        let keys_added = vec![
            make_key_in_creation(10, SecurityLevel::MASTER),
            make_key_in_creation(11, SecurityLevel::MASTER),
        ];
        let keys_to_disable = vec![
            make_identity_public_key(1, SecurityLevel::MASTER),
            make_identity_public_key(2, SecurityLevel::MASTER),
        ];
        let result = validate_master_key_uniqueness_v0(&keys_added, &keys_to_disable)
            .expect("should succeed");
        assert!(
            !result.is_valid(),
            "should be invalid when disabling more than one master key"
        );
    }

    #[test]
    fn should_fail_when_adding_two_master_keys_and_disabling_one() {
        let keys_added = vec![
            make_key_in_creation(10, SecurityLevel::MASTER),
            make_key_in_creation(11, SecurityLevel::MASTER),
        ];
        let keys_to_disable = vec![make_identity_public_key(1, SecurityLevel::MASTER)];
        let result = validate_master_key_uniqueness_v0(&keys_added, &keys_to_disable)
            .expect("should succeed");
        assert!(
            !result.is_valid(),
            "should be invalid when adding more than one master key"
        );
    }

    #[test]
    fn should_pass_with_mixed_master_and_non_master_keys_in_valid_rotation() {
        let keys_added = vec![
            make_key_in_creation(10, SecurityLevel::MASTER),
            make_key_in_creation(11, SecurityLevel::HIGH),
            make_key_in_creation(12, SecurityLevel::MEDIUM),
        ];
        let keys_to_disable = vec![
            make_identity_public_key(1, SecurityLevel::MASTER),
            make_identity_public_key(2, SecurityLevel::HIGH),
        ];
        let result = validate_master_key_uniqueness_v0(&keys_added, &keys_to_disable)
            .expect("should succeed");
        assert!(
            result.is_valid(),
            "should be valid when rotating exactly one master key among other non-master keys"
        );
    }
}
