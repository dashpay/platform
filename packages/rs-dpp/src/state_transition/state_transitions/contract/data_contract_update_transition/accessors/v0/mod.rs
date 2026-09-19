use crate::data_contract::serialized_version::DataContractInSerializationFormat;
use crate::prelude::IdentityNonce;
use crate::ProtocolError;
use platform_value::Identifier;

pub trait DataContractUpdateTransitionAccessorsV0 {
    /// The full contract a V0 update embeds. A delta-based V1 update carries
    /// none: the updated contract only exists once the delta is merged onto
    /// the stored one.
    fn data_contract(&self) -> Option<&DataContractInSerializationFormat>;
    /// Replaces the embedded contract of a V0 update. A V1 update has no
    /// embedded contract to replace, and returns an error.
    fn set_data_contract(
        &mut self,
        data_contract: DataContractInSerializationFormat,
    ) -> Result<(), ProtocolError>;

    fn identity_contract_nonce(&self) -> IdentityNonce;

    /// The contract this update targets.
    fn data_contract_id(&self) -> Identifier;
}
