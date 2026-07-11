mod v0;

pub use v0::*;

use crate::address_funds::PlatformAddress;
use crate::shielded::SerializedAction;
use crate::state_transition::identity_create_from_shielded_pool_transition::IdentityCreateFromShieldedPoolTransition;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use platform_value::Identifier;

impl IdentityCreateFromShieldedPoolTransitionAccessorsV0
    for IdentityCreateFromShieldedPoolTransition
{
    fn actions(&self) -> &[SerializedAction] {
        match self {
            IdentityCreateFromShieldedPoolTransition::V0(v0) => &v0.actions,
        }
    }

    fn set_actions(&mut self, actions: Vec<SerializedAction>) {
        match self {
            IdentityCreateFromShieldedPoolTransition::V0(v0) => v0.actions = actions,
        }
    }

    fn public_keys(&self) -> &[IdentityPublicKeyInCreation] {
        match self {
            IdentityCreateFromShieldedPoolTransition::V0(v0) => &v0.public_keys,
        }
    }

    fn set_public_keys(&mut self, public_keys: Vec<IdentityPublicKeyInCreation>) {
        match self {
            IdentityCreateFromShieldedPoolTransition::V0(v0) => v0.public_keys = public_keys,
        }
    }

    fn denomination(&self) -> u64 {
        match self {
            IdentityCreateFromShieldedPoolTransition::V0(v0) => v0.denomination,
        }
    }

    fn set_denomination(&mut self, denomination: u64) {
        match self {
            IdentityCreateFromShieldedPoolTransition::V0(v0) => v0.denomination = denomination,
        }
    }

    fn anchor(&self) -> [u8; 32] {
        match self {
            IdentityCreateFromShieldedPoolTransition::V0(v0) => v0.anchor,
        }
    }

    fn set_anchor(&mut self, anchor: [u8; 32]) {
        match self {
            IdentityCreateFromShieldedPoolTransition::V0(v0) => v0.anchor = anchor,
        }
    }

    fn proof(&self) -> &[u8] {
        match self {
            IdentityCreateFromShieldedPoolTransition::V0(v0) => &v0.proof,
        }
    }

    fn set_proof(&mut self, proof: Vec<u8>) {
        match self {
            IdentityCreateFromShieldedPoolTransition::V0(v0) => v0.proof = proof,
        }
    }

    fn binding_signature(&self) -> [u8; 64] {
        match self {
            IdentityCreateFromShieldedPoolTransition::V0(v0) => v0.binding_signature,
        }
    }

    fn set_binding_signature(&mut self, binding_signature: [u8; 64]) {
        match self {
            IdentityCreateFromShieldedPoolTransition::V0(v0) => {
                v0.binding_signature = binding_signature
            }
        }
    }

    fn send_to_address_on_creation_failure(&self) -> &PlatformAddress {
        match self {
            IdentityCreateFromShieldedPoolTransition::V0(v0) => {
                &v0.send_to_address_on_creation_failure
            }
        }
    }

    fn set_send_to_address_on_creation_failure(
        &mut self,
        send_to_address_on_creation_failure: PlatformAddress,
    ) {
        match self {
            IdentityCreateFromShieldedPoolTransition::V0(v0) => {
                v0.send_to_address_on_creation_failure = send_to_address_on_creation_failure
            }
        }
    }

    fn identity_id(&self) -> Identifier {
        match self {
            IdentityCreateFromShieldedPoolTransition::V0(v0) => v0.identity_id,
        }
    }

    fn set_identity_id(&mut self, identity_id: Identifier) {
        match self {
            IdentityCreateFromShieldedPoolTransition::V0(v0) => v0.identity_id = identity_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::{KeyType, Purpose, SecurityLevel};
    use crate::state_transition::identity_create_from_shielded_pool_transition::v0::IdentityCreateFromShieldedPoolTransitionV0;
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

    fn make_transition() -> IdentityCreateFromShieldedPoolTransition {
        IdentityCreateFromShieldedPoolTransitionV0 {
            public_keys: vec![mk_key(0, 0xAA)],
            denomination: 10_000_000_000,
            actions: vec![mk_action(0x11)],
            anchor: [7u8; 32],
            proof: vec![8u8; 10],
            binding_signature: [9u8; 64],
            send_to_address_on_creation_failure: PlatformAddress::P2pkh([0u8; 20]),
            identity_id: Identifier::new([1u8; 32]),
        }
        .into()
    }

    #[test]
    fn test_getters_and_setters() {
        let mut t = make_transition();

        assert_eq!(t.actions(), &[mk_action(0x11)]);
        assert_eq!(t.public_keys(), &[mk_key(0, 0xAA)]);
        assert_eq!(t.denomination(), 10_000_000_000);
        assert_eq!(t.anchor(), [7u8; 32]);
        assert_eq!(t.proof(), &[8u8; 10]);
        assert_eq!(t.binding_signature(), [9u8; 64]);
        assert_eq!(
            t.send_to_address_on_creation_failure(),
            &PlatformAddress::P2pkh([0u8; 20])
        );
        assert_eq!(t.identity_id(), Identifier::new([1u8; 32]));

        t.set_actions(vec![mk_action(0x22)]);
        assert_eq!(t.actions(), &[mk_action(0x22)]);

        t.set_public_keys(vec![mk_key(1, 0xBB)]);
        assert_eq!(t.public_keys(), &[mk_key(1, 0xBB)]);

        t.set_denomination(20_000_000_000);
        assert_eq!(t.denomination(), 20_000_000_000);

        t.set_anchor([1u8; 32]);
        assert_eq!(t.anchor(), [1u8; 32]);

        t.set_proof(vec![0xAu8; 5]);
        assert_eq!(t.proof(), &[0xAu8; 5]);

        t.set_binding_signature([0xBu8; 64]);
        assert_eq!(t.binding_signature(), [0xBu8; 64]);

        t.set_send_to_address_on_creation_failure(PlatformAddress::P2pkh([2u8; 20]));
        assert_eq!(
            t.send_to_address_on_creation_failure(),
            &PlatformAddress::P2pkh([2u8; 20])
        );

        t.set_identity_id(Identifier::new([3u8; 32]));
        assert_eq!(t.identity_id(), Identifier::new([3u8; 32]));
    }
}
