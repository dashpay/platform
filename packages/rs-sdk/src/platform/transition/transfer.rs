use dpp::identifier::Identifier;
use dpp::identity::accessors::IdentityGettersV0;

use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::platform::transition::put_settings::PutSettings;
use crate::{Error, Sdk};
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityPublicKey};
use dpp::state_transition::identity_credit_transfer_transition::methods::IdentityCreditTransferTransitionMethodsV0;
use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
use dpp::state_transition::proof_result::StateTransitionProofResult;

#[async_trait::async_trait]
pub trait TransferToIdentity {
    /// Function to transfer credits from an identity to another identity. Returns the final
    /// identity balance.
    ///
    /// If signing_transfer_key_to_use is not set, we will try to use one in the signer that is
    /// available for the transfer.
    async fn transfer_credits<S: Signer + Send>(
        &self,
        sdk: &Sdk,
        to_identity_id: Identifier,
        amount: u64,
        signing_transfer_key_to_use: Option<&IdentityPublicKey>,
        signer: S,
        settings: Option<PutSettings>,
    ) -> Result<u64, Error>;
}

#[async_trait::async_trait]
impl TransferToIdentity for Identity {
    async fn transfer_credits<S: Signer + Send>(
        &self,
        sdk: &Sdk,
        to_identity_id: Identifier,
        amount: u64,
        signing_transfer_key_to_use: Option<&IdentityPublicKey>,
        signer: S,
        settings: Option<PutSettings>,
    ) -> Result<u64, Error> {
        let new_identity_nonce = sdk.get_identity_nonce(self.id(), true, settings).await?;
        let user_fee_increase = settings.and_then(|settings| settings.user_fee_increase);
        let state_transition = IdentityCreditTransferTransition::try_from_identity(
            self,
            to_identity_id,
            amount,
            user_fee_increase.unwrap_or_default(),
            signer,
            signing_transfer_key_to_use,
            new_identity_nonce,
            sdk.version(),
            None,
        )?;

        let result = state_transition.broadcast_and_wait(sdk, None).await?;

        match result {
            StateTransitionProofResult::VerifiedPartialIdentity(identity) => {
                identity.balance.ok_or(Error::DapiClientError(
                    "expected an identity balance after transfer".to_string(),
                ))
            }
            _ => Err(Error::DapiClientError("proved a non identity".to_string())),
        }
    }
}
