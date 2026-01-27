use std::collections::BTreeMap;

use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::schema::DataContractSchemaMethodsV0;
use crate::data_contract::DataContract;
use crate::identity::signer::Signer;
use crate::identity::{IdentityPublicKey, KeyID, PartialIdentity};
use crate::serialization::Signable;

use crate::prelude::{IdentityNonce, UserFeeIncrease};
use crate::state_transition::data_contract_update_transition::methods::DataContractUpdateTransitionMethodsV0;
use crate::state_transition::data_contract_update_transition::{
    DataContractUpdateTransition, DataContractUpdateTransitionV1,
};
use crate::state_transition::StateTransition;
use crate::version::FeatureVersion;
use crate::{NonConsensusError, ProtocolError};
use platform_version::version::PlatformVersion;

impl DataContractUpdateTransitionMethodsV0 for DataContractUpdateTransitionV1 {
    fn new_from_data_contract<S: Signer<IdentityPublicKey>>(
        data_contract: DataContract,
        identity: &PartialIdentity,
        key_id: KeyID,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        _platform_version: &PlatformVersion,
        _feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        let transition = DataContractUpdateTransition::V1(DataContractUpdateTransitionV1 {
            update_contract_system_version: None,
            id: data_contract.id(),
            owner_id: data_contract.owner_id(),
            revision: data_contract.version(),
            updated_schema_defs: BTreeMap::new(),
            new_schema_defs: data_contract.schema_defs().cloned().unwrap_or_default(),
            updated_document_schemas: BTreeMap::new(),
            new_document_schemas: BTreeMap::new(),
            new_groups: BTreeMap::new(),
            new_tokens: BTreeMap::new(),
            remove_keywords: Vec::new(),
            add_keywords: Vec::new(),
            update_description: None,
            identity_contract_nonce,
            user_fee_increase,
            signature_public_key_id: key_id,
            signature: Default::default(),
        });

        let mut state_transition: StateTransition = transition.into();
        let value = state_transition.signable_bytes()?;
        let public_key =
            identity
                .loaded_public_keys
                .get(&key_id)
                .ok_or(ProtocolError::NonConsensusError(
                    NonConsensusError::StateTransitionCreationError(
                        "public key did not exist".to_string(),
                    ),
                ))?;
        state_transition.set_signature(signer.sign(public_key, &value)?);
        Ok(state_transition)
    }
}
