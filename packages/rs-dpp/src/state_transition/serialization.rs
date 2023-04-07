use crate::state_transition::StateTransition;
use crate::ProtocolError;
use bincode::config;
use platform_value::Value;
use std::fmt::format;

impl StateTransition {
    pub fn serialize(&self) -> Result<Vec<u8>, ProtocolError> {
        let config = config::standard().with_big_endian().with_no_limit();
        bincode::encode_to_vec(self, config).map_err(|e| {
            ProtocolError::EncodingError(format!("unable to serialize state transition {e}"))
        })
    }

    pub fn serialized_size(&self) -> Result<usize, ProtocolError> {
        self.serialize().map(|a| a.len())
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self, ProtocolError> {
        let config = config::standard().with_big_endian().with_limit::<100000>();
        bincode::decode_from_slice(bytes, config)
            .map_err(|e| {
                ProtocolError::EncodingError(format!(
                    "unable to deserialize state transition {}",
                    e
                ))
            })
            .map(|(a, _)| a)
    }

    pub fn deserialize_many(
        raw_state_transitions: &Vec<Vec<u8>>,
    ) -> Result<Vec<Self>, ProtocolError> {
        raw_state_transitions
            .iter()
            .map(|raw_state_transition| Self::deserialize(raw_state_transition))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::data_contract::state_transition::data_contract_create_transition::DataContractCreateTransition;
    use crate::data_contract::state_transition::data_contract_update_transition::DataContractUpdateTransition;
    use crate::document::document_transition::Action;
    use crate::document::DocumentsBatchTransition;
    use crate::identity::core_script::CoreScript;
    use crate::identity::state_transition::identity_create_transition::IdentityCreateTransition;
    use crate::identity::state_transition::identity_credit_withdrawal_transition::{
        IdentityCreditWithdrawalTransition, Pooling,
    };
    use crate::identity::state_transition::identity_public_key_transitions::IdentityPublicKeyInCreationWithWitness;
    use crate::identity::state_transition::identity_topup_transition::IdentityTopUpTransition;
    use crate::identity::state_transition::identity_update_transition::identity_update_transition::IdentityUpdateTransition;
    use crate::identity::Identity;
    use crate::state_transition::{
        StateTransition, StateTransitionConvert, StateTransitionLike, StateTransitionType,
    };
    use crate::tests::fixtures::{
        get_data_contract_fixture, get_document_transitions_fixture,
        get_documents_fixture_with_owner_id_from_contract,
    };
    use crate::version::LATEST_VERSION;
    use crate::{NativeBlsModule, ProtocolError};
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use std::collections::BTreeMap;
    use std::convert::TryInto;

    #[test]
    fn identity_create_transition_ser_de() {
        let identity = Identity::random_identity(5, Some(5));
        let identity_create_transition: IdentityCreateTransition = identity
            .try_into()
            .expect("expected to make an identity create transition");
        let state_transition: StateTransition = identity_create_transition.into();
        let bytes = state_transition.serialize().expect("expected to serialize");
        let recovered_state_transition =
            StateTransition::deserialize(&bytes).expect("expected to deserialize state transition");
        assert_eq!(state_transition, recovered_state_transition);
    }

    #[test]
    fn identity_topup_transition_ser_de() {
        let identity = Identity::random_identity(5, Some(5));
        let identity_topup_transition = IdentityTopUpTransition {
            asset_lock_proof: identity
                .asset_lock_proof
                .expect("expected an asset lock proof on the identity"),
            identity_id: identity.id,
            protocol_version: LATEST_VERSION,
            transition_type: StateTransitionType::IdentityTopUp,
            signature: [1u8; 65].to_vec().into(),
        };
        let state_transition: StateTransition = identity_topup_transition.into();
        let bytes = state_transition.serialize().expect("expected to serialize");
        let recovered_state_transition =
            StateTransition::deserialize(&bytes).expect("expected to deserialize state transition");
        assert_eq!(state_transition, recovered_state_transition);
    }

    #[test]
    fn identity_update_transition_add_keys_ser_de() {
        let mut rng = StdRng::seed_from_u64(5);
        let (identity, mut keys): (Identity, BTreeMap<_, _>) =
            Identity::random_identity_with_main_keys_with_private_key(5, &mut rng)
                .expect("expected to get identity");
        let bls = NativeBlsModule::default();
        let mut identity_update_transition = IdentityUpdateTransition {
            protocol_version: LATEST_VERSION,
            transition_type: StateTransitionType::IdentityUpdate,
            signature: Default::default(),
            signature_public_key_id: 0,
            identity_id: identity.id,
            revision: 1,
            add_public_keys: identity
                .public_keys
                .into_values()
                .map(|public_key| {
                    let private_key = keys
                        .get(&public_key)
                        .expect("expected to have the private key");
                    IdentityPublicKeyInCreationWithWitness::from_public_key_signed_with_private_key(
                        public_key,
                        private_key,
                        &bls,
                    )
                })
                .collect::<Result<Vec<IdentityPublicKeyInCreationWithWitness>, ProtocolError>>()
                .expect("expected to get added public keys"),
            disable_public_keys: vec![],
            public_keys_disabled_at: None,
        };

        let (public_key, private_key) = keys.pop_first().unwrap();
        identity_update_transition
            .sign_by_private_key(private_key.as_slice(), public_key.key_type, &bls)
            .expect("expected to sign IdentityUpdateTransition");

        let state_transition: StateTransition = identity_update_transition.into();
        let bytes = state_transition.serialize().expect("expected to serialize");
        let recovered_state_transition =
            StateTransition::deserialize(&bytes).expect("expected to deserialize state transition");
        assert_eq!(state_transition, recovered_state_transition);
    }

    #[test]
    fn identity_update_transition_disable_keys_ser_de() {
        let mut rng = StdRng::seed_from_u64(5);
        let (identity, mut keys): (Identity, BTreeMap<_, _>) =
            Identity::random_identity_with_main_keys_with_private_key(5, &mut rng)
                .expect("expected to get identity");
        let bls = NativeBlsModule::default();
        let mut identity_update_transition = IdentityUpdateTransition {
            protocol_version: LATEST_VERSION,
            transition_type: StateTransitionType::IdentityUpdate,
            signature: Default::default(),
            signature_public_key_id: 0,
            identity_id: identity.id,
            revision: 1,
            add_public_keys: identity
                .public_keys
                .into_values()
                .map(|public_key| {
                    let private_key = keys
                        .get(&public_key)
                        .expect("expected to have the private key");
                    IdentityPublicKeyInCreationWithWitness::from_public_key_signed_with_private_key(
                        public_key,
                        private_key,
                        &bls,
                    )
                })
                .collect::<Result<Vec<IdentityPublicKeyInCreationWithWitness>, ProtocolError>>()
                .expect("expected to get added public keys"),
            disable_public_keys: vec![3, 4, 5],
            public_keys_disabled_at: Some(15),
        };

        let (public_key, private_key) = keys.pop_first().unwrap();
        identity_update_transition
            .sign_by_private_key(private_key.as_slice(), public_key.key_type, &bls)
            .expect("expected to sign IdentityUpdateTransition");

        let state_transition: StateTransition = identity_update_transition.into();
        let bytes = state_transition.serialize().expect("expected to serialize");
        let recovered_state_transition =
            StateTransition::deserialize(&bytes).expect("expected to deserialize state transition");
        assert_eq!(state_transition, recovered_state_transition);
    }

    #[test]
    fn identity_credit_withdrawal_transition_ser_de() {
        let identity = Identity::random_identity(5, Some(5));
        let mut identity_credit_withdrawal_transition = IdentityCreditWithdrawalTransition {
            protocol_version: LATEST_VERSION,
            transition_type: StateTransitionType::IdentityCreditWithdrawal,
            identity_id: identity.id,
            amount: 5000000,
            core_fee_per_byte: 34,
            pooling: Pooling::Standard,
            output_script: CoreScript::from_bytes((0..23).collect::<Vec<u8>>()),
            revision: 1,
            signature_public_key_id: 0,
            signature: [1u8; 65].to_vec().into(),
        };
        let state_transition: StateTransition = identity_credit_withdrawal_transition.into();
        let bytes = state_transition.serialize().expect("expected to serialize");
        let recovered_state_transition =
            StateTransition::deserialize(&bytes).expect("expected to deserialize state transition");
        assert_eq!(state_transition, recovered_state_transition);
    }

    #[test]
    fn data_contract_create_ser_de() {
        let identity = Identity::random_identity(5, Some(5));
        let mut data_contract = get_data_contract_fixture(Some(identity.id));
        data_contract.entropy = Default::default();
        let data_contract_create_transition = DataContractCreateTransition {
            protocol_version: LATEST_VERSION,
            transition_type: StateTransitionType::DataContractCreate,
            data_contract,
            entropy: Default::default(),
            signature_public_key_id: 0,
            signature: [1u8; 65].to_vec().into(),
        };
        let state_transition: StateTransition = data_contract_create_transition.into();
        let bytes = state_transition.serialize().expect("expected to serialize");
        let recovered_state_transition =
            StateTransition::deserialize(&bytes).expect("expected to deserialize state transition");
        assert_eq!(state_transition, recovered_state_transition);
    }

    #[test]
    fn data_contract_update_ser_de() {
        let identity = Identity::random_identity(5, Some(5));
        let mut data_contract = get_data_contract_fixture(Some(identity.id));
        data_contract.entropy = Default::default();
        let data_contract_update_transition = DataContractUpdateTransition {
            protocol_version: LATEST_VERSION,
            transition_type: StateTransitionType::DataContractCreate,
            data_contract,
            signature_public_key_id: 0,
            signature: [1u8; 65].to_vec().into(),
        };
        let state_transition: StateTransition = data_contract_update_transition.into();
        let bytes = state_transition.serialize().expect("expected to serialize");
        let recovered_state_transition =
            StateTransition::deserialize(&bytes).expect("expected to deserialize state transition");
        assert_eq!(state_transition, recovered_state_transition);
    }

    #[test]
    fn document_batch_transition_10_created_documents_ser_de() {
        let mut data_contract = get_data_contract_fixture(None);
        data_contract.entropy = Default::default();
        let documents =
            get_documents_fixture_with_owner_id_from_contract(data_contract.clone()).unwrap();
        let transitions = get_document_transitions_fixture([(Action::Create, documents)]);
        let documents_batch_transition = DocumentsBatchTransition {
            owner_id: data_contract.owner_id,
            transitions,
            ..Default::default()
        };
        let state_transition: StateTransition = documents_batch_transition.into();
        let bytes = state_transition.serialize().expect("expected to serialize");
        let recovered_state_transition =
            StateTransition::deserialize(&bytes).expect("expected to deserialize state transition");
        assert_eq!(state_transition, recovered_state_transition);
    }
}
