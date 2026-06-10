use crate::consensus::basic::identity::MissingMasterPublicKeyError;
use crate::consensus::basic::invalid_identifier_error::InvalidIdentifierError;
use crate::consensus::basic::state_transition::ShieldedInvalidDenominationError;
use crate::consensus::basic::BasicError;
use crate::consensus::state::identity::max_identity_public_key_limit_reached_error::MaxIdentityPublicKeyLimitReachedError;
use crate::consensus::state::state_error::StateError;
use crate::state_transition::identity_create_from_shielded_pool_transition::derive_identity_id_from_actions;
use crate::state_transition::identity_create_from_shielded_pool_transition::v0::IdentityCreateFromShieldedPoolTransitionV0;
use crate::state_transition::state_transitions::shielded::common_validation::{
    validate_actions_count, validate_anchor_not_zero, validate_encrypted_note_sizes,
    validate_proof_not_empty,
};
use crate::state_transition::StateTransitionStructureValidation;
use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;

impl StateTransitionStructureValidation for IdentityCreateFromShieldedPoolTransitionV0 {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        // Actions count must be in [1, max]
        let result = validate_actions_count(
            &self.actions,
            platform_version
                .system_limits
                .max_shielded_transition_actions,
        );
        if !result.is_valid() {
            return result;
        }

        // Each action's encrypted_note must be exactly ENCRYPTED_NOTE_SIZE bytes
        let result = validate_encrypted_note_sizes(&self.actions);
        if !result.is_valid() {
            return result;
        }

        // The wire `identity_id` MUST equal the value derived from the spend nullifiers. It is
        // excluded from the platform sighash and is NOT what the Orchard bundle binds (the bundle's
        // `extra_sighash_data` commits to the *derived* id). Without this check a relayer/proposer
        // could overwrite the field with arbitrary bytes: consensus would still create the identity
        // at the derived id, but every downstream consumer that trusts the wire field —
        // `modified_data_ids` (block events / indexers) and the SDK prove/verify path (which build
        // their merged path-query from `identity_id`) — would desync from the canonical state.
        // Rejecting a mismatch here makes the wire id authoritative consensus-wide, exactly as
        // `IdentityCreate` re-derives and checks the id from its asset-lock outpoint.
        if self.identity_id != derive_identity_id_from_actions(&self.actions) {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::InvalidIdentifierError(InvalidIdentifierError::new(
                    "identity_id".to_string(),
                    "does not match the value derived from the spend nullifiers".to_string(),
                ))
                .into(),
            );
        }

        // The denomination MUST be a member of the versioned exit-denomination set. Restricting the
        // exit to a small fixed set is what makes every identity-creation exit of a given size
        // indistinguishable on-chain (maximizing the anonymity set). An empty set (pre-v12) rejects
        // every denomination, but the transition is already gated off pre-v12 by `is_allowed`.
        let denominations = platform_version
            .drive_abci
            .validation_and_processing
            .event_constants
            .shielded_identity_create_denominations;
        if !denominations.contains(&self.denomination) {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::ShieldedInvalidDenominationError(
                    ShieldedInvalidDenominationError::new(self.denomination),
                )
                .into(),
            );
        }

        // Proof must not be empty
        let result = validate_proof_not_empty(&self.proof);
        if !result.is_valid() {
            return result;
        }

        // Anchor must not be all zeros
        let result = validate_anchor_not_zero(&self.anchor);
        if !result.is_valid() {
            return result;
        }

        // At least one public key (the master key requirement and full key-structure validation —
        // duplicates, security levels, proofs-of-possession — run in drive-abci, mirroring
        // `IdentityCreate`).
        if self.public_keys.is_empty() {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::MissingMasterPublicKeyError(MissingMasterPublicKeyError::new()).into(),
            );
        }

        // At most `max_public_keys_in_creation` public keys.
        let max_keys = platform_version
            .dpp
            .state_transitions
            .identities
            .max_public_keys_in_creation as usize;
        if self.public_keys.len() > max_keys {
            return SimpleConsensusValidationResult::new_with_error(
                StateError::MaxIdentityPublicKeyLimitReachedError(
                    MaxIdentityPublicKeyLimitReachedError::new(max_keys),
                )
                .into(),
            );
        }

        SimpleConsensusValidationResult::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::ConsensusError;
    use crate::identity::{KeyType, Purpose, SecurityLevel};
    use crate::shielded::SerializedAction;
    use crate::state_transition::identity_create_from_shielded_pool_transition::derive_identity_id_from_actions;
    use crate::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
    use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
    use assert_matches::assert_matches;
    use platform_value::BinaryData;

    fn dummy_action() -> SerializedAction {
        SerializedAction {
            nullifier: [1u8; 32],
            rk: [2u8; 32],
            cmx: [3u8; 32],
            encrypted_note: vec![4u8; 216],
            cv_net: [5u8; 32],
            spend_auth_sig: [6u8; 64],
        }
    }

    fn master_key() -> IdentityPublicKeyInCreation {
        IdentityPublicKeyInCreation::V0(IdentityPublicKeyInCreationV0 {
            id: 0,
            key_type: KeyType::ECDSA_SECP256K1,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::MASTER,
            contract_bounds: None,
            read_only: false,
            data: BinaryData::new(vec![0u8; 33]),
            signature: BinaryData::new(vec![0u8; 65]),
        })
    }

    fn valid_transition() -> IdentityCreateFromShieldedPoolTransitionV0 {
        let actions = vec![dummy_action()];
        let identity_id = derive_identity_id_from_actions(&actions);
        IdentityCreateFromShieldedPoolTransitionV0 {
            public_keys: vec![master_key()],
            denomination: 10_000_000_000,
            actions,
            anchor: [7u8; 32],
            proof: vec![8u8; 100],
            binding_signature: [9u8; 64],
            send_to_address_on_creation_failure: crate::address_funds::PlatformAddress::P2pkh(
                [0u8; 20],
            ),
            identity_id,
        }
    }

    #[test]
    fn should_validate_a_valid_transition() {
        let platform_version = PlatformVersion::latest();
        let result = valid_transition().validate_structure(platform_version);
        assert!(
            result.is_valid(),
            "expected valid result, got: {:?}",
            result.errors
        );
    }

    #[test]
    fn should_reject_mismatched_wire_identity_id() {
        // A relayer-mutated `identity_id` (not matching the value derived from the spend nullifiers)
        // must be rejected so the wire field stays authoritative for prove/verify/modified_data_ids.
        let platform_version = PlatformVersion::latest();
        let mut t = valid_transition();
        t.identity_id = platform_value::Identifier::new([0xFF; 32]);
        let result = t.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::InvalidIdentifierError(_)
            )]
        );
    }

    #[test]
    fn should_reject_non_member_denomination() {
        let platform_version = PlatformVersion::latest();
        let mut t = valid_transition();
        t.denomination = 12_345; // not a member of the set
        let result = t.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedInvalidDenominationError(_)
            )]
        );
    }

    #[test]
    fn should_accept_each_member_denomination() {
        let platform_version = PlatformVersion::latest();
        for denomination in [
            10_000_000_000u64,
            30_000_000_000,
            50_000_000_000,
            100_000_000_000,
        ] {
            let mut t = valid_transition();
            t.denomination = denomination;
            assert!(
                t.validate_structure(platform_version).is_valid(),
                "denomination {denomination} should be accepted"
            );
        }
    }

    #[test]
    fn should_reject_empty_actions() {
        let platform_version = PlatformVersion::latest();
        let mut t = valid_transition();
        t.actions.clear();
        let result = t.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedNoActionsError(_)
            )]
        );
    }

    #[test]
    fn should_reject_empty_public_keys() {
        let platform_version = PlatformVersion::latest();
        let mut t = valid_transition();
        t.public_keys.clear();
        let result = t.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::MissingMasterPublicKeyError(_)
            )]
        );
    }

    #[test]
    fn should_reject_zero_anchor() {
        let platform_version = PlatformVersion::latest();
        let mut t = valid_transition();
        t.anchor = [0u8; 32];
        let result = t.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedZeroAnchorError(_)
            )]
        );
    }

    #[test]
    fn should_reject_empty_proof() {
        let platform_version = PlatformVersion::latest();
        let mut t = valid_transition();
        t.proof.clear();
        let result = t.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedEmptyProofError(_)
            )]
        );
    }
}
