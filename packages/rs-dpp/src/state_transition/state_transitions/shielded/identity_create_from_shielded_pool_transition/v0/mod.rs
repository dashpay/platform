mod state_transition_like;
mod state_transition_validation;
mod types;
pub(super) mod v0_methods;
mod version;

use crate::address_funds::PlatformAddress;
#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
use crate::shielded::SerializedAction;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreationSignable;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::PlatformSignable;
use platform_value::Identifier;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(Debug, Clone, PartialEq, Encode, Decode, PlatformSignable)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
// As with `IdentityCreateTransitionV0`, deriving bincode for the `#[platform_signable(into = ...)]`
// borrowed key vector is done manually inside the PlatformSignable proc macro instead of via the
// bincode derive.
#[platform_signable(derive_bincode_with_borrowed_vec)]
pub struct IdentityCreateFromShieldedPoolTransitionV0 {
    /// The public keys of the new identity (VARIABLE: 1..=max_public_keys_in_creation), exactly as
    /// `IdentityCreate` carries them. When signing, the per-key proof-of-possession signatures are
    /// NOT part of the sighash (the `Signable` form excludes them); the keys themselves ARE, and
    /// are additionally bound into the Orchard sighash via `extra_sighash_data`.
    #[platform_signable(into = "Vec<IdentityPublicKeyInCreationSignable>")]
    pub public_keys: Vec<IdentityPublicKeyInCreation>,
    /// The fixed exit denomination (in credits) leaving the shielded pool. MUST equal the Orchard
    /// bundle's `value_balance` EXACTLY and MUST be a member of the versioned denomination set.
    pub denomination: u64,
    /// Orchard actions (spend-output pairs). The spend nullifiers fund the exit; any change
    /// re-enters the pool as an ordinary output note.
    pub actions: Vec<SerializedAction>,
    /// Sinsemilla root of the note commitment tree (Orchard Anchor)
    pub anchor: [u8; 32],
    /// Halo2 proof bytes
    pub proof: Vec<u8>,
    /// RedPallas binding signature
    pub binding_signature: [u8; 64],
    /// Fallback platform address credited if identity creation FAILS a stateful check (a
    /// public-key hash already registered to another identity). On failure the spend is still final
    /// — the denomination leaves the pool — and is credited here minus a penalty, exactly like the
    /// `PartiallyUseAssetLock` / `BumpAddressInputNonces` penalty the asset-lock and address-funded
    /// identity creates use. It IS part of the platform sighash (so each key's proof-of-possession
    /// signs it) and is additionally committed into the Orchard `extra_sighash_data`, so a relayer
    /// cannot redirect the fallback.
    pub send_to_address_on_creation_failure: PlatformAddress,
    /// The id of the new identity, derived as `double_sha256(sorted nullifiers)`. It is committed
    /// into the Orchard `extra_sighash_data` (so the bundle cannot be redirected to a different id)
    /// and re-derived + checked at consensus. Excluded from the platform sighash because it is fully
    /// determined by the nullifiers in `actions`, which are already covered.
    #[platform_signable(exclude_from_sig_hash)]
    pub identity_id: Identifier,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::{KeyType, Purpose, SecurityLevel};
    use crate::serialization::Signable;
    use crate::state_transition::identity_create_from_shielded_pool_transition::derive_identity_id_from_actions;
    use crate::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
    use platform_value::BinaryData;

    fn mk_action(nullifier_byte: u8) -> SerializedAction {
        SerializedAction {
            nullifier: [nullifier_byte; 32],
            rk: [2u8; 32],
            cmx: [3u8; 32],
            encrypted_note: vec![4u8; 216],
            cv_net: [5u8; 32],
            spend_auth_sig: [6u8; 64],
        }
    }

    fn mk_key(id: u32, data_byte: u8) -> IdentityPublicKeyInCreation {
        IdentityPublicKeyInCreation::V0(IdentityPublicKeyInCreationV0 {
            id,
            key_type: KeyType::ECDSA_SECP256K1,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::MASTER,
            contract_bounds: None,
            read_only: false,
            data: BinaryData::new(vec![data_byte; 33]),
            signature: BinaryData::new(vec![]),
        })
    }

    fn make_v0() -> IdentityCreateFromShieldedPoolTransitionV0 {
        let actions = vec![mk_action(0x11)];
        let identity_id = derive_identity_id_from_actions(&actions);
        IdentityCreateFromShieldedPoolTransitionV0 {
            public_keys: vec![mk_key(0, 0xAA)],
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
    fn test_state_transition_type() {
        use crate::state_transition::{StateTransitionLike, StateTransitionType};
        let t = make_v0();
        assert_eq!(
            t.state_transition_type(),
            StateTransitionType::IdentityCreateFromShieldedPool
        );
    }

    #[test]
    fn test_modified_data_ids_is_identity_id() {
        use crate::state_transition::StateTransitionLike;
        let t = make_v0();
        assert_eq!(t.modified_data_ids(), vec![t.identity_id]);
    }

    #[test]
    fn test_unique_identifiers_from_nullifiers() {
        use crate::state_transition::StateTransitionLike;
        let mut t = make_v0();
        t.actions = vec![mk_action(0x11), mk_action(0x22)];
        let ids = t.unique_identifiers();
        assert_eq!(ids.len(), 2);
        assert_eq!(ids[0], hex::encode([0x11u8; 32]));
        assert_eq!(ids[1], hex::encode([0x22u8; 32]));
    }

    #[test]
    fn test_feature_versioned() {
        use crate::state_transition::FeatureVersioned;
        assert_eq!(make_v0().feature_version(), 0);
    }

    /// The per-key proof-of-possession signs the transition's signable bytes, so the signable bytes
    /// MUST change when the public-key set or the denomination changes — otherwise a relayer could
    /// replay valid PoP signatures against a swapped key set / amount. (The Orchard
    /// `extra_sighash_data` binding is the primary defense; this asserts the platform-level sighash
    /// also commits to these fields.)
    #[test]
    fn test_signable_bytes_commit_to_keys_and_denomination() {
        let base = make_v0();
        let base_bytes = base.signable_bytes().expect("signable bytes");

        let mut other_keys = base.clone();
        other_keys.public_keys = vec![mk_key(0, 0xBB)];
        assert_ne!(
            base_bytes,
            other_keys.signable_bytes().expect("signable bytes"),
            "changing the public-key set must change the signable bytes"
        );

        let mut other_denom = base.clone();
        other_denom.denomination = 30_000_000_000;
        assert_ne!(
            base_bytes,
            other_denom.signable_bytes().expect("signable bytes"),
            "changing the denomination must change the signable bytes"
        );

        let mut other_failure_address = base.clone();
        other_failure_address.send_to_address_on_creation_failure =
            PlatformAddress::P2pkh([1u8; 20]);
        assert_ne!(
            base_bytes,
            other_failure_address.signable_bytes().expect("signable bytes"),
            "changing the failure-fallback address must change the signable bytes (non-redirectable)"
        );

        // identity_id is excluded from the sighash (it is derived from the nullifiers), so changing
        // it alone must NOT change the signable bytes.
        let mut other_id = base.clone();
        other_id.identity_id = Identifier::new([0xCC; 32]);
        assert_eq!(
            base_bytes,
            other_id.signable_bytes().expect("signable bytes"),
            "identity_id is excluded from the sighash, so it must not change the signable bytes"
        );
    }
}
