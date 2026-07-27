use dashcore::{InstantLock, Transaction};

use std::collections::BTreeMap;

use crate::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use crate::identity::state_transition::asset_lock_proof::{AssetLockProof, InstantAssetLockProof};
use crate::identity::{Identity, IdentityPublicKey, KeyID};
use crate::prelude::{Identifier, IdentityNonce};

use crate::identity::identity_factory::IdentityFactory;
#[cfg(feature = "state-transitions")]
use crate::state_transition::{
    identity_create_transition::IdentityCreateTransition,
    identity_credit_transfer_transition::IdentityCreditTransferTransition,
    identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition,
    identity_topup_transition::IdentityTopUpTransition,
    identity_update_transition::IdentityUpdateTransition,
    public_key_in_creation::IdentityPublicKeyInCreation,
};

use crate::identity::core_script::CoreScript;
use crate::withdrawal::Pooling;
use crate::ProtocolError;

#[derive(Clone)]
pub struct IdentityFacade {
    factory: IdentityFactory,
}

impl IdentityFacade {
    pub fn new(protocol_version: u32) -> Self {
        Self {
            factory: IdentityFactory::new(protocol_version),
        }
    }

    pub fn create(
        &self,
        id: Identifier,
        public_keys: BTreeMap<KeyID, IdentityPublicKey>,
    ) -> Result<Identity, ProtocolError> {
        self.factory.create(id, public_keys)
    }

    // TODO(versioning): not used anymore?
    // pub fn create_from_object(
    //     &self,
    //     raw_identity: Value,
    //     skip_validation: bool,
    // ) -> Result<Identity, ProtocolError> {
    //     self.factory
    //         .create_from_object(raw_identity)
    // }

    #[cfg(all(feature = "identity-serialization", feature = "client"))]
    pub fn create_from_buffer(
        &self,
        buffer: Vec<u8>,
        #[cfg(feature = "validation")] skip_validation: bool,
    ) -> Result<Identity, ProtocolError> {
        self.factory.create_from_buffer(
            buffer,
            #[cfg(feature = "validation")]
            skip_validation,
        )
    }

    pub fn create_instant_lock_proof(
        instant_lock: InstantLock,
        asset_lock_transaction: Transaction,
        output_index: u32,
    ) -> InstantAssetLockProof {
        IdentityFactory::create_instant_lock_proof(
            instant_lock,
            asset_lock_transaction,
            output_index,
        )
    }

    pub fn create_chain_asset_lock_proof(
        core_chain_locked_height: u32,
        out_point: [u8; 36],
    ) -> ChainAssetLockProof {
        IdentityFactory::create_chain_asset_lock_proof(core_chain_locked_height, out_point)
    }

    #[cfg(feature = "state-transitions")]
    pub fn create_identity_create_transition(
        &self,
        identity: &Identity,
        asset_lock_proof: AssetLockProof,
    ) -> Result<IdentityCreateTransition, ProtocolError> {
        self.factory
            .create_identity_create_transition(identity, asset_lock_proof)
    }

    #[cfg(feature = "state-transitions")]
    pub fn create_identity_topup_transition(
        &self,
        identity_id: Identifier,
        asset_lock_proof: AssetLockProof,
    ) -> Result<IdentityTopUpTransition, ProtocolError> {
        self.factory
            .create_identity_topup_transition(identity_id, asset_lock_proof)
    }

    #[cfg(feature = "state-transitions")]
    pub fn create_identity_credit_transfer_transition(
        &self,
        identity: &Identity,
        recipient_id: Identifier,
        amount: u64,
        identity_nonce: IdentityNonce,
    ) -> Result<IdentityCreditTransferTransition, ProtocolError> {
        self.factory.create_identity_credit_transfer_transition(
            identity,
            recipient_id,
            amount,
            identity_nonce,
        )
    }

    #[cfg(feature = "state-transitions")]
    pub fn create_identity_credit_withdrawal_transition(
        &self,
        identity_id: Identifier,
        amount: u64,
        core_fee_per_byte: u32,
        pooling: Pooling,
        output_script: Option<CoreScript>,
        identity_nonce: u64,
    ) -> Result<IdentityCreditWithdrawalTransition, ProtocolError> {
        self.factory.create_identity_credit_withdrawal_transition(
            identity_id,
            amount,
            core_fee_per_byte,
            pooling,
            output_script,
            identity_nonce,
        )
    }

    #[cfg(feature = "state-transitions")]
    pub fn create_identity_update_transition(
        &self,
        identity: Identity,
        identity_nonce: u64,
        add_public_keys: Option<Vec<IdentityPublicKeyInCreation>>,
        public_key_ids_to_disable: Option<Vec<KeyID>>,
    ) -> Result<IdentityUpdateTransition, ProtocolError> {
        self.factory.create_identity_update_transition(
            identity,
            identity_nonce,
            add_public_keys,
            public_key_ids_to_disable,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::accessors::IdentityGettersV0;

    fn facade_v1() -> IdentityFacade {
        IdentityFacade::new(1)
    }

    #[test]
    fn new_constructs_facade() {
        let facade = IdentityFacade::new(1);
        let cloned = facade.clone();
        // Exercising Clone; also re-use to verify the facade is usable after clone.
        let id = Identifier::random();
        let _identity = cloned.create(id, BTreeMap::new()).unwrap();
    }

    #[test]
    fn create_returns_identity_with_given_id() {
        let facade = facade_v1();
        let id = Identifier::from([11u8; 32]);
        let identity = facade.create(id, BTreeMap::new()).expect("create");
        assert_eq!(identity.id(), id);
        assert_eq!(identity.balance(), 0);
        assert_eq!(identity.revision(), 0);
        assert_eq!(identity.public_keys().len(), 0);
    }

    #[test]
    fn create_errors_on_unknown_protocol_version() {
        let facade = IdentityFacade::new(u32::MAX);
        let id = Identifier::from([12u8; 32]);
        let result = facade.create(id, BTreeMap::new());
        assert!(result.is_err());
    }

    #[test]
    fn create_chain_asset_lock_proof_preserves_inputs() {
        let out_point = [7u8; 36];
        let proof = IdentityFacade::create_chain_asset_lock_proof(333, out_point);
        assert_eq!(proof.core_chain_locked_height, 333);
        assert_eq!(proof.out_point, out_point.into());
    }

    #[test]
    fn create_instant_lock_proof_preserves_output_index() {
        use crate::identity::state_transition::asset_lock_proof::AssetLockProof;
        use crate::tests::fixtures::instant_asset_lock_proof_fixture;
        let fixture = instant_asset_lock_proof_fixture(None, None);
        let (il, tx) = match &fixture {
            AssetLockProof::Instant(p) => (p.instant_lock.clone(), p.transaction.clone()),
            _ => panic!("expected instant"),
        };
        let built = IdentityFacade::create_instant_lock_proof(il, tx.clone(), 11);
        assert_eq!(built.output_index, 11);
        assert_eq!(built.transaction.txid(), tx.txid());
    }

    #[cfg(feature = "identity-serialization")]
    #[test]
    fn create_from_buffer_roundtrips_serialized_identity() {
        use crate::serialization::PlatformSerializable;
        let facade = facade_v1();
        let id = Identifier::from([13u8; 32]);
        let identity = facade.create(id, BTreeMap::new()).unwrap();
        let bytes = PlatformSerializable::serialize_to_bytes(&identity).unwrap();
        let restored = facade
            .create_from_buffer(
                bytes,
                #[cfg(feature = "validation")]
                true,
            )
            .expect("roundtrip succeeds");
        assert_eq!(restored.id(), id);
    }

    #[cfg(feature = "identity-serialization")]
    #[test]
    fn create_from_buffer_errors_on_garbage() {
        let facade = facade_v1();
        let result = facade.create_from_buffer(
            vec![0xEE; 10],
            #[cfg(feature = "validation")]
            true,
        );
        assert!(result.is_err());
    }

    #[cfg(feature = "state-transitions")]
    mod state_transitions {
        use super::*;
        use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use crate::identity::{KeyType, Purpose, SecurityLevel};
        use crate::state_transition::identity_credit_transfer_transition::accessors::IdentityCreditTransferTransitionAccessorsV0;
        use crate::state_transition::identity_credit_withdrawal_transition::accessors::IdentityCreditWithdrawalTransitionAccessorsV0;
        use crate::state_transition::identity_topup_transition::accessors::IdentityTopUpTransitionAccessorsV0;
        use crate::state_transition::identity_update_transition::accessors::IdentityUpdateTransitionAccessorsV0;
        use crate::tests::fixtures::instant_asset_lock_proof_fixture;

        fn sample_pk(id: u32) -> IdentityPublicKey {
            IdentityPublicKey::V0(IdentityPublicKeyV0 {
                id,
                purpose: Purpose::AUTHENTICATION,
                security_level: SecurityLevel::MASTER,
                contract_bounds: None,
                key_type: KeyType::ECDSA_SECP256K1,
                read_only: false,
                data: vec![0u8; 33].into(),
                disabled_at: None,
            })
        }

        #[test]
        fn create_identity_create_transition_wraps_factory() {
            let facade = facade_v1();
            let proof = instant_asset_lock_proof_fixture(None, None);
            let expected_id = proof.create_identifier().unwrap();
            let identity = facade.create(expected_id, BTreeMap::new()).unwrap();
            let transition = facade
                .create_identity_create_transition(&identity, proof)
                .expect("transition");
            match transition {
                IdentityCreateTransition::V0(v0) => {
                    assert_eq!(v0.identity_id, expected_id);
                }
            }
        }

        #[test]
        fn create_identity_topup_transition_uses_given_id_and_proof() {
            let facade = facade_v1();
            let identity_id = Identifier::from([21u8; 32]);
            let proof = instant_asset_lock_proof_fixture(None, None);
            let transition = facade
                .create_identity_topup_transition(identity_id, proof)
                .expect("transition");
            assert_eq!(*transition.identity_id(), identity_id);
        }

        #[test]
        fn create_identity_credit_transfer_transition_uses_identity_id() {
            let facade = facade_v1();
            let id = Identifier::from([31u8; 32]);
            let identity = facade.create(id, BTreeMap::new()).unwrap();
            let recipient = Identifier::from([32u8; 32]);
            let transition = facade
                .create_identity_credit_transfer_transition(&identity, recipient, 500, 3)
                .expect("transition");
            assert_eq!(transition.identity_id(), id);
            assert_eq!(transition.recipient_id(), recipient);
            assert_eq!(transition.amount(), 500);
            assert_eq!(transition.nonce(), 3);
        }

        #[test]
        fn create_identity_credit_withdrawal_transition_v0_requires_output_script() {
            let facade = facade_v1();
            let id = Identifier::from([41u8; 32]);
            let result = facade.create_identity_credit_withdrawal_transition(
                id,
                1000,
                1,
                Pooling::Never,
                None,
                5,
            );
            match result {
                Err(ProtocolError::Generic(msg)) => {
                    assert!(msg.contains("Output script is required"));
                }
                other => panic!("expected Generic error, got {:?}", other),
            }
        }

        #[test]
        fn create_identity_credit_withdrawal_transition_v0_with_script_succeeds() {
            let facade = facade_v1();
            let id = Identifier::from([42u8; 32]);
            let script = CoreScript::new_p2pkh([0xFF; 20]);
            let transition = facade
                .create_identity_credit_withdrawal_transition(
                    id,
                    1500,
                    2,
                    Pooling::IfAvailable,
                    Some(script.clone()),
                    7,
                )
                .expect("transition");
            assert_eq!(transition.identity_id(), id);
            assert_eq!(transition.amount(), 1500);
            assert_eq!(transition.core_fee_per_byte(), 2);
            assert_eq!(transition.output_script(), Some(script));
            assert_eq!(transition.nonce(), 7);
        }

        #[test]
        fn create_identity_update_transition_adds_and_disables_keys() {
            let facade = facade_v1();
            let id = Identifier::from([51u8; 32]);
            let identity = facade.create(id, BTreeMap::new()).unwrap();
            let new_key: IdentityPublicKeyInCreation = sample_pk(1).into();
            let transition = facade
                .create_identity_update_transition(
                    identity,
                    9,
                    Some(vec![new_key.clone()]),
                    Some(vec![5u32, 6u32]),
                )
                .expect("transition");
            assert_eq!(transition.identity_id(), id);
            assert_eq!(transition.nonce(), 9);
            // Fresh identity has revision 0, so the update becomes revision 1.
            assert_eq!(transition.revision(), 1);
            assert_eq!(transition.public_keys_to_add().len(), 1);
            assert_eq!(transition.public_key_ids_to_disable(), &[5u32, 6][..]);
        }
    }
}
