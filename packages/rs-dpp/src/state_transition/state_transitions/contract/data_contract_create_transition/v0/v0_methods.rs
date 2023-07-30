use crate::state_transition::data_contract_create_transition::DataContractCreateTransitionV0;

use crate::serialization::PlatformSerializable;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};

use platform_value::{BinaryData, Bytes32, IntegerReplacementType, ReplacementType, Value};
use serde::{Deserialize, Serialize};

use crate::{
    data_contract::DataContract,
    identity::KeyID,
    prelude::Identifier,
    state_transition::{StateTransitionLike, StateTransitionType},
    Convertible, NonConsensusError, ProtocolError,
};

use crate::serialization::{PlatformDeserializable, Signable};
use bincode::{config, Decode, Encode};
use platform_version::TryIntoPlatformVersioned;
use platform_version::version::PlatformVersion;
use crate::identity::PartialIdentity;
use crate::identity::signer::Signer;
use crate::state_transition::data_contract_create_transition::DataContractCreateTransition;
use crate::state_transition::data_contract_create_transition::methods::DataContractCreateTransitionMethodsV0;
use crate::state_transition::state_transitions::contract::data_contract_create_transition::fields::{BINARY_FIELDS, IDENTIFIER_FIELDS, U32_FIELDS};

use crate::state_transition::StateTransition;
use crate::version::FeatureVersion;

impl DataContractCreateTransitionMethodsV0 for DataContractCreateTransitionV0 {
    fn new_from_data_contract<S: Signer>(
        mut data_contract: DataContract,
        entropy: Bytes32,
        identity: &PartialIdentity,
        key_id: KeyID,
        signer: &S,
        platform_version: &PlatformVersion,
        _feature_version: Option<FeatureVersion>,
    ) -> Result<DataContractCreateTransition, ProtocolError> {
        data_contract.set_id(DataContract::generate_data_contract_id_v0(
            identity.id,
            entropy,
        ));
        data_contract.set_owner_id(identity.id);
        let mut transition = DataContractCreateTransition::V0(DataContractCreateTransitionV0 {
            data_contract: data_contract.try_into_platform_versioned(platform_version)?,
            entropy: Default::default(),
            signature_public_key_id: key_id,
            signature: Default::default(),
        });
        let value = transition.signable_bytes()?;
        let public_key =
            identity
                .loaded_public_keys
                .get(&key_id)
                .ok_or(ProtocolError::NonConsensusError(
                    NonConsensusError::StateTransitionCreationError(
                        "public key did not exist".to_string(),
                    ),
                ))?;
        transition.set_signature(signer.sign(public_key, &value)?.into());
        Ok(transition)
    }
}
