#[cfg(feature = "state-transition-signing")]
use crate::shielded::SerializedAction;
use crate::state_transition::StateTransitionType;
#[cfg(feature = "state-transition-signing")]
use crate::{state_transition::StateTransition, ProtocolError};
#[cfg(feature = "state-transition-signing")]
use platform_value::{BinaryData, Identifier};
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

pub trait PrivateDocumentCreateFromShieldedPoolTransitionMethodsV0 {
    /// Build the transition from an already proven and signed Orchard spend bundle. The
    /// bundle's binding signature must have been computed over the platform sighash that binds
    /// the contract, the type name and the entries (see
    /// `private_document_create_from_shielded_pool_extra_sighash_data`), and its value balance
    /// must be the fee (`compute_private_document_create_from_shielded_pool_fee`).
    #[cfg(feature = "state-transition-signing")]
    #[allow(clippy::too_many_arguments)]
    fn try_from_bundle(
        data_contract_id: Identifier,
        document_type_name: String,
        entries: Vec<BinaryData>,
        actions: Vec<SerializedAction>,
        value_balance: u64,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError>;

    /// Get State Transition Type
    fn get_type() -> StateTransitionType {
        StateTransitionType::PrivateDocumentCreateFromShieldedPool
    }
}
