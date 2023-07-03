use crate::state_transition::data_contract_create_transition::DataContractCreateTransitionV0;

use crate::serialization_traits::PlatformSerializable;
use platform_serialization::PlatformSignable;
use platform_serialization::{PlatformDeserialize, PlatformSerialize};

use platform_value::{BinaryData, Bytes32, IntegerReplacementType, ReplacementType, Value};
use serde::{Deserialize, Serialize};

use crate::{
    data_contract::DataContract,
    identity::KeyID,
    prelude::Identifier,
    state_transition::{StateTransitionLike, StateTransitionType},
    Convertible, NonConsensusError, ProtocolError,
};

use crate::serialization_traits::{PlatformDeserializable, Signable};
use bincode::{config, Decode, Encode};
use crate::identity::PartialIdentity;
use crate::identity::signer::Signer;
use crate::state_transition::data_contract_create_transition::DataContractCreateTransition;
use crate::state_transition::state_transitions::contract::data_contract_create_transition::fields::{BINARY_FIELDS, IDENTIFIER_FIELDS, U32_FIELDS};

use crate::state_transition::StateTransition;
use crate::version::FeatureVersion;

pub trait DataContractCreateTransitionV0Methods {
    /// Creates a new instance of the DataContractCreateTransition from the provided data contract.
    ///
    /// # Arguments
    ///
    /// * `data_contract` - A mutable `DataContract` instance, to be used in the transition.
    /// * `entropy` - A `Bytes32` value providing additional randomness.
    /// * `identity` - A reference to a `PartialIdentity` object.
    /// * `key_id` - A `KeyID` identifier for the public key used for signing the transition.
    /// * `signer` - A reference to an object implementing the `Signer` trait.
    ///
    /// # Returns
    ///
    /// If successful, returns a `Result<Self, ProtocolError>` containing a `DataContractCreateTransition`
    /// object. Otherwise, returns `ProtocolError`.
    fn new_from_data_contract<S: Signer>(
        data_contract: DataContract,
        entropy: Bytes32,
        identity: &PartialIdentity,
        key_id: KeyID,
        signer: &S,
        version: FeatureVersion,
    ) -> Result<DataContractCreateTransition, ProtocolError>;

    fn get_data_contract(&self) -> &DataContract;

    fn set_data_contract(&mut self, data_contract: DataContract);

    fn get_modified_data_ids(&self) -> Vec<Identifier>;
}

impl DataContractCreateTransitionV0Methods for DataContractCreateTransitionV0 {
    fn new_from_data_contract<S: Signer>(
        mut data_contract: DataContract,
        entropy: Bytes32,
        identity: &PartialIdentity,
        key_id: KeyID,
        signer: &S,
        _version: FeatureVersion,
    ) -> Result<DataContractCreateTransition, ProtocolError> {
        data_contract.owner_id = identity.id;
        data_contract.id = DataContract::generate_data_contract_id_v0(identity.id, entropy);
        let mut transition = DataContractCreateTransition::V0(DataContractCreateTransitionV0 {
            data_contract,
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

    fn get_data_contract(&self) -> &DataContract {
        &self.data_contract
    }

    fn set_data_contract(&mut self, data_contract: DataContract) {
        self.data_contract = data_contract;
    }

    /// Returns ID of the created contract
    fn get_modified_data_ids(&self) -> Vec<Identifier> {
        vec![self.data_contract.id]
    }
}
