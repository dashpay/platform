mod v0;

pub use v0::*;

use crate::data_contract::DataContract;
use crate::identity::signer::Signer;
use crate::identity::{KeyID, PartialIdentity};
use crate::state_transition::data_contract_create_transition::{
    DataContractCreateTransition, DataContractCreateTransitionV0,
};
use crate::version::FeatureVersion;
use crate::ProtocolError;
use platform_value::{Bytes32, Identifier};

impl DataContractCreateTransitionMethodsV0 for DataContractCreateTransition {
    fn new_from_data_contract<S: Signer>(
        data_contract: DataContract,
        entropy: Bytes32,
        identity: &PartialIdentity,
        key_id: KeyID,
        signer: &S,
        version: FeatureVersion,
    ) -> Result<DataContractCreateTransition, ProtocolError> {
        match version {
            0 => DataContractCreateTransitionV0::new_from_data_contract(
                data_contract,
                entropy,
                identity,
                key_id,
                signer,
                version,
            ),
            v => Err(ProtocolError::UnknownVersionError(format!(
                "Unknown DataContractCreateTransition version for new_from_data_contract {v}"
            ))),
        }
    }

    fn modified_data_ids(&self) -> Vec<Identifier> {
        match self {
            DataContractCreateTransition::V0(transition) => transition.modified_data_ids(),
        }
    }
}
