#[cfg(feature = "state-transition-signing")]
use crate::data_contract::document_type::DocumentTypeRef;
#[cfg(feature = "state-transition-signing")]
use crate::document::Document;
#[cfg(feature = "state-transition-signing")]
use crate::identity::signer::Signer;
#[cfg(feature = "state-transition-signing")]
use crate::identity::IdentityPublicKey;
#[cfg(feature = "state-transition-signing")]
use crate::prelude::{IdentityNonce, UserFeeIncrease};
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::methods::StateTransitionCreationOptions;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::StateTransition;
#[cfg(feature = "state-transition-signing")]
use crate::version::PlatformVersion;
#[cfg(feature = "state-transition-signing")]
use crate::ProtocolError;

/// Constructors introduced with the V2 batch transition wire format.
pub trait DocumentsBatchTransitionMethodsV2 {
    /// Builds a signed erase of one already deleted keep-history document.
    #[cfg(feature = "state-transition-signing")]
    #[allow(clippy::too_many_arguments)]
    async fn new_document_erase_transition_from_document<S: Signer<IdentityPublicKey>>(
        document: Document,
        document_type: DocumentTypeRef<'_>,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        options: Option<StateTransitionCreationOptions>,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError>;
}
