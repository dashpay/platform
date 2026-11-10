#[cfg(feature = "state-transition-signing")]
use crate::shielded::SerializedAction;
use crate::state_transition::private_document_create_from_shielded_pool_transition::methods::PrivateDocumentCreateFromShieldedPoolTransitionMethodsV0;
use crate::state_transition::private_document_create_from_shielded_pool_transition::v0::PrivateDocumentCreateFromShieldedPoolTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::{state_transition::StateTransition, ProtocolError};
#[cfg(feature = "state-transition-signing")]
use platform_value::{BinaryData, Identifier};
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

impl PrivateDocumentCreateFromShieldedPoolTransitionMethodsV0
    for PrivateDocumentCreateFromShieldedPoolTransitionV0
{
    #[cfg(feature = "state-transition-signing")]
    fn try_from_bundle(
        data_contract_id: Identifier,
        document_type_name: String,
        entries: Vec<BinaryData>,
        actions: Vec<SerializedAction>,
        value_balance: u64,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
        _platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError> {
        let transition = PrivateDocumentCreateFromShieldedPoolTransitionV0 {
            data_contract_id,
            document_type_name,
            entries,
            actions,
            value_balance,
            anchor,
            proof,
            binding_signature,
        };
        Ok(transition.into())
    }
}
