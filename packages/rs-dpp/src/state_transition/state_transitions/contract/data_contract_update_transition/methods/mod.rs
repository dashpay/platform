mod v0;

pub use v0::*;

use crate::data_contract::DataContract;
use crate::identity::signer::Signer;
use crate::identity::{KeyID, PartialIdentity};
use crate::state_transition::data_contract_update_transition::{
    DataContractUpdateTransition, DataContractUpdateTransitionV0,
};
use crate::version::FeatureVersion;
use crate::ProtocolError;
use platform_value::{Bytes32, Identifier};

impl DataContractUpdateTransitionMethodsV0 for DataContractUpdateTransition {
    fn new_from_data_contract<S: Signer>(
        data_contract: DataContract,
        identity: &PartialIdentity,
        key_id: KeyID,
        signer: &S,
        version: FeatureVersion,
    ) -> Result<DataContractUpdateTransition, ProtocolError> {
        match version {
            0 => DataContractUpdateTransitionV0::new_from_data_contract(
                data_contract,
                identity,
                key_id,
                signer,
                version,
            ),
            v => Err(ProtocolError::UnknownVersionError(format!(
                "Unknown DataContractUpdateTransition version for new_from_data_contract {v}"
            ))),
        }
    }
}
