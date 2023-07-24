use crate::data_contract::DataContract;
use crate::identity::signer::Signer;
use crate::identity::{KeyID, PartialIdentity};
use crate::state_transition::data_contract_create_transition::v0::v0_methods::DataContractCreateTransitionV0Methods;
use crate::state_transition::data_contract_create_transition::{
    DataContractCreateTransition, DataContractCreateTransitionV0,
};
use crate::version::FeatureVersion;
use crate::ProtocolError;
use platform_value::{Bytes32, Identifier};

impl DataContractCreateTransitionV0Methods for DataContractCreateTransition {
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

    fn data_contract(&self) -> &DataContract {
        match self {
            DataContractCreateTransition::V0(transition) => transition.data_contract(),
        }
    }

    fn entropy(&self) -> &Bytes32 {
        match self {
            DataContractCreateTransition::V0(transition) => transition.entropy(),
        }
    }

    fn set_data_contract(&mut self, data_contract: DataContract) {
        match self {
            DataContractCreateTransition::V0(transition) => {
                transition.set_data_contract(data_contract)
            }
        }
    }

    fn get_modified_data_ids(&self) -> Vec<Identifier> {
        match self {
            DataContractCreateTransition::V0(transition) => transition.get_modified_data_ids(),
        }
    }
}
