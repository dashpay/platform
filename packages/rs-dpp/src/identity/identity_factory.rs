use crate::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
#[cfg(all(feature = "state-transitions", feature = "client"))]
use crate::identity::state_transition::asset_lock_proof::AssetLockProof;
use crate::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
#[cfg(all(feature = "state-transitions", feature = "client"))]
use crate::identity::state_transition::AssetLockProved;
#[cfg(all(feature = "state-transitions", feature = "client"))]
use crate::identity::IdentityV0;

use crate::identity::{Identity, IdentityPublicKey, KeyID};

use crate::ProtocolError;

use dashcore::{InstantLock, Transaction};
use platform_value::Identifier;
use std::collections::BTreeMap;

#[cfg(all(feature = "identity-serialization", feature = "client"))]
use crate::consensus::basic::decode::SerializedObjectParsingError;
#[cfg(all(feature = "identity-serialization", feature = "client"))]
use crate::consensus::basic::BasicError;
#[cfg(all(feature = "identity-serialization", feature = "client"))]
use crate::consensus::ConsensusError;
#[cfg(all(feature = "state-transitions", feature = "client"))]
use crate::identity::accessors::IdentityGettersV0;
#[cfg(all(feature = "state-transitions", feature = "client"))]
use crate::identity::core_script::CoreScript;
#[cfg(all(feature = "state-transitions", feature = "client"))]
use crate::prelude::IdentityNonce;
#[cfg(all(feature = "identity-serialization", feature = "client"))]
use crate::serialization::PlatformDeserializable;
#[cfg(all(feature = "state-transitions", feature = "client"))]
use crate::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
#[cfg(all(feature = "state-transitions", feature = "client"))]
use crate::state_transition::identity_create_transition::IdentityCreateTransition;
#[cfg(all(feature = "state-transitions", feature = "client"))]
use crate::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
#[cfg(all(feature = "state-transitions", feature = "client"))]
use crate::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
#[cfg(all(feature = "state-transitions", feature = "client"))]
use crate::state_transition::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0;
#[cfg(all(feature = "state-transitions", feature = "client"))]
use crate::state_transition::identity_credit_withdrawal_transition::v1::IdentityCreditWithdrawalTransitionV1;
#[cfg(all(feature = "state-transitions", feature = "client"))]
use crate::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
#[cfg(all(feature = "state-transitions", feature = "client"))]
use crate::state_transition::identity_topup_transition::accessors::IdentityTopUpTransitionAccessorsV0;
#[cfg(all(feature = "state-transitions", feature = "client"))]
use crate::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0;
#[cfg(all(feature = "state-transitions", feature = "client"))]
use crate::state_transition::identity_topup_transition::IdentityTopUpTransition;
#[cfg(all(feature = "state-transitions", feature = "client"))]
use crate::state_transition::identity_update_transition::accessors::IdentityUpdateTransitionAccessorsV0;
#[cfg(all(feature = "state-transitions", feature = "client"))]
use crate::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
#[cfg(all(feature = "state-transitions", feature = "client"))]
use crate::state_transition::identity_update_transition::IdentityUpdateTransition;
#[cfg(all(feature = "state-transitions", feature = "client"))]
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::version::PlatformVersion;
#[cfg(all(feature = "state-transitions", feature = "client"))]
use crate::withdrawal::Pooling;

pub const IDENTITY_PROTOCOL_VERSION: u32 = 1;

#[derive(Clone)]
pub struct IdentityFactory {
    protocol_version: u32,
}

impl IdentityFactory {
    pub fn new(protocol_version: u32) -> Self {
        IdentityFactory { protocol_version }
    }

    pub fn create(
        &self,
        id: Identifier,
        public_keys: BTreeMap<KeyID, IdentityPublicKey>,
    ) -> Result<Identity, ProtocolError> {
        Identity::new_with_id_and_keys(
            id,
            public_keys,
            PlatformVersion::get(self.protocol_version)?,
        )
    }

    #[cfg(all(feature = "identity-serialization", feature = "client"))]
    pub fn create_from_buffer(
        &self,
        buffer: Vec<u8>,
        #[cfg(feature = "validation")] skip_validation: bool,
    ) -> Result<Identity, ProtocolError> {
        let identity: Identity =
            Identity::deserialize_from_bytes_no_limit(&buffer).map_err(|e| {
                ConsensusError::BasicError(BasicError::SerializedObjectParsingError(
                    SerializedObjectParsingError::new(format!("Decode protocol entity: {:#?}", e)),
                ))
            })?;

        #[cfg(feature = "validation")]
        if !skip_validation {
            // todo: validate identity
        }

        Ok(identity)
    }

    pub fn create_instant_lock_proof(
        instant_lock: InstantLock,
        asset_lock_transaction: Transaction,
        output_index: u32,
    ) -> InstantAssetLockProof {
        InstantAssetLockProof::new(instant_lock, asset_lock_transaction, output_index)
    }

    pub fn create_chain_asset_lock_proof(
        core_chain_locked_height: u32,
        out_point: [u8; 36],
    ) -> ChainAssetLockProof {
        ChainAssetLockProof::new(core_chain_locked_height, out_point)
    }

    #[cfg(all(feature = "state-transitions", feature = "client"))]
    pub fn create_identity_create_transition(
        &self,
        identity: &Identity,
        asset_lock_proof: AssetLockProof,
    ) -> Result<IdentityCreateTransition, ProtocolError> {
        let transition =
            IdentityCreateTransitionV0::try_from_identity_v0(identity, asset_lock_proof)?;

        Ok(IdentityCreateTransition::V0(transition))
    }

    #[cfg(all(feature = "state-transitions", feature = "client"))]
    pub fn create_identity_with_create_transition(
        &self,
        public_keys: BTreeMap<KeyID, IdentityPublicKey>,
        asset_lock_proof: AssetLockProof,
    ) -> Result<(Identity, IdentityCreateTransition), ProtocolError> {
        let identifier = asset_lock_proof.create_identifier()?;
        let identity = Identity::V0(IdentityV0 {
            id: identifier,
            public_keys: public_keys.clone(),
            balance: 0,
            revision: 0,
        });

        let identity_create_transition = IdentityCreateTransition::V0(
            IdentityCreateTransitionV0::try_from_identity_v0(&identity, asset_lock_proof)?,
        );
        Ok((identity, identity_create_transition))
    }

    #[cfg(all(feature = "state-transitions", feature = "client"))]
    pub fn create_identity_topup_transition(
        &self,
        identity_id: Identifier,
        asset_lock_proof: AssetLockProof,
    ) -> Result<IdentityTopUpTransition, ProtocolError> {
        let mut identity_topup_transition = IdentityTopUpTransitionV0::default();

        identity_topup_transition.set_identity_id(identity_id);
        identity_topup_transition.set_asset_lock_proof(asset_lock_proof)?;

        Ok(IdentityTopUpTransition::V0(identity_topup_transition))
    }

    #[cfg(all(feature = "state-transitions", feature = "client"))]
    pub fn create_identity_credit_transfer_transition(
        &self,
        identity: &Identity,
        recipient_id: Identifier,
        amount: u64,
        identity_nonce: IdentityNonce,
    ) -> Result<IdentityCreditTransferTransition, ProtocolError> {
        let identity_credit_transfer_transition = IdentityCreditTransferTransitionV0 {
            identity_id: identity.id(),
            recipient_id,
            amount,
            nonce: identity_nonce,
            ..Default::default()
        };

        Ok(IdentityCreditTransferTransition::from(
            identity_credit_transfer_transition,
        ))
    }

    #[cfg(all(feature = "state-transitions", feature = "client"))]
    pub fn create_identity_credit_withdrawal_transition(
        &self,
        identity_id: Identifier,
        amount: u64,
        core_fee_per_byte: u32,
        pooling: Pooling,
        output_script: Option<CoreScript>,
        identity_nonce: IdentityNonce,
    ) -> Result<IdentityCreditWithdrawalTransition, ProtocolError> {
        let platform_version = PlatformVersion::get(self.protocol_version)?;

        let identity_credit_withdrawal_transition = match platform_version
            .dpp
            .state_transitions
            .identities
            .credit_withdrawal
            .default_constructor
        {
            0 => {
                let output_script = output_script.ok_or_else(|| {
                    ProtocolError::Generic(
                        "Output script is required for IdentityCreditWithdrawalTransitionV0"
                            .to_string(),
                    )
                })?;

                let transition = IdentityCreditWithdrawalTransitionV0 {
                    identity_id,
                    amount,
                    core_fee_per_byte,
                    pooling,
                    output_script,
                    nonce: identity_nonce,
                    ..Default::default()
                };

                IdentityCreditWithdrawalTransition::from(transition)
            }
            1 => {
                let transition = IdentityCreditWithdrawalTransitionV1 {
                    identity_id,
                    amount,
                    core_fee_per_byte,
                    pooling,
                    output_script,
                    nonce: identity_nonce,
                    ..Default::default()
                };

                IdentityCreditWithdrawalTransition::from(transition)
            }
            version => {
                return Err(ProtocolError::UnknownVersionMismatch {
                    method: "create_identity_credit_withdrawal_transition".to_string(),
                    known_versions: vec![0, 1],
                    received: version,
                });
            }
        };

        Ok(identity_credit_withdrawal_transition)
    }

    #[cfg(all(feature = "state-transitions", feature = "client"))]
    pub fn create_identity_update_transition(
        &self,
        identity: Identity,
        identity_nonce: u64,
        add_public_keys: Option<Vec<IdentityPublicKeyInCreation>>,
        public_key_ids_to_disable: Option<Vec<KeyID>>,
    ) -> Result<IdentityUpdateTransition, ProtocolError> {
        let mut identity_update_transition = IdentityUpdateTransitionV0::default();
        identity_update_transition.set_identity_id(identity.id().to_owned());
        identity_update_transition.set_revision(identity.revision() + 1);
        identity_update_transition.set_nonce(identity_nonce);

        if let Some(add_public_keys) = add_public_keys {
            identity_update_transition.set_public_keys_to_add(add_public_keys);
        }

        if let Some(public_key_ids_to_disable) = public_key_ids_to_disable {
            identity_update_transition.set_public_key_ids_to_disable(public_key_ids_to_disable);
        }

        Ok(IdentityUpdateTransition::V0(identity_update_transition))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::accessors::IdentityGettersV0;
    use platform_value::Identifier;
    use std::collections::BTreeMap;

    #[test]
    fn factory_new_sets_protocol_version() {
        let factory = IdentityFactory::new(1);
        assert_eq!(factory.protocol_version, 1);
    }

    #[test]
    fn factory_clone() {
        let factory = IdentityFactory::new(1);
        let cloned = factory.clone();
        assert_eq!(cloned.protocol_version, 1);
    }

    #[test]
    fn create_identity_with_empty_keys() {
        let factory = IdentityFactory::new(1);
        let id = Identifier::random();
        let public_keys = BTreeMap::new();

        let identity = factory.create(id, public_keys).unwrap();

        assert_eq!(identity.id(), id);
    }

    #[test]
    fn create_identity_with_invalid_version() {
        let factory = IdentityFactory::new(u32::MAX);
        let id = Identifier::random();
        let public_keys = BTreeMap::new();

        let result = factory.create(id, public_keys);
        assert!(result.is_err());
    }

    #[test]
    fn create_identity_preserves_id() {
        let factory = IdentityFactory::new(1);
        let id = Identifier::random();
        let public_keys = BTreeMap::new();

        let identity = factory.create(id, public_keys).unwrap();
        assert_eq!(identity.id(), id);
    }

    #[test]
    fn create_identity_has_zero_balance() {
        let factory = IdentityFactory::new(1);
        let id = Identifier::random();
        let public_keys = BTreeMap::new();

        let identity = factory.create(id, public_keys).unwrap();
        assert_eq!(identity.balance(), 0);
    }

    #[test]
    fn create_identity_has_zero_revision() {
        let factory = IdentityFactory::new(1);
        let id = Identifier::random();
        let public_keys = BTreeMap::new();

        let identity = factory.create(id, public_keys).unwrap();
        assert_eq!(identity.revision(), 0);
    }

    #[test]
    fn create_identity_with_multiple_versions() {
        // Version 1 should work
        let factory_v1 = IdentityFactory::new(1);
        let id = Identifier::random();
        let result = factory_v1.create(id, BTreeMap::new());
        assert!(result.is_ok());
    }

    #[test]
    fn create_chain_asset_lock_proof_test() {
        let out_point = [0u8; 36];
        let proof = IdentityFactory::create_chain_asset_lock_proof(100, out_point);

        assert_eq!(proof.core_chain_locked_height, 100);
    }

    #[test]
    fn create_chain_asset_lock_proof_different_heights() {
        let out_point = [1u8; 36];
        let proof = IdentityFactory::create_chain_asset_lock_proof(500, out_point);
        assert_eq!(proof.core_chain_locked_height, 500);

        let proof2 = IdentityFactory::create_chain_asset_lock_proof(0, out_point);
        assert_eq!(proof2.core_chain_locked_height, 0);

        let proof3 = IdentityFactory::create_chain_asset_lock_proof(u32::MAX, out_point);
        assert_eq!(proof3.core_chain_locked_height, u32::MAX);
    }

    #[test]
    fn create_two_identities_have_different_ids() {
        let factory = IdentityFactory::new(1);
        let id1 = Identifier::random();
        let id2 = Identifier::random();

        let identity1 = factory.create(id1, BTreeMap::new()).unwrap();
        let identity2 = factory.create(id2, BTreeMap::new()).unwrap();

        assert_ne!(identity1.id(), identity2.id());
    }

    #[test]
    fn create_identity_with_public_keys() {
        use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use crate::identity::{KeyType, Purpose, SecurityLevel};

        let factory = IdentityFactory::new(1);
        let id = Identifier::random();

        let key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 0,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::MASTER,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: vec![0u8; 33].into(),
            disabled_at: None,
        });

        let mut public_keys = BTreeMap::new();
        public_keys.insert(0u32, key);

        let identity = factory.create(id, public_keys).unwrap();
        assert_eq!(identity.public_keys().len(), 1);
    }
}
