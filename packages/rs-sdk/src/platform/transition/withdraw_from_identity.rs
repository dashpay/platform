use dpp::dashcore::Address;
use dpp::identity::accessors::IdentityGettersV0;

use dpp::identity::core_script::CoreScript;
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityPublicKey};

use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::platform::transition::put_settings::PutSettings;
use crate::platform::transition::validation::ensure_valid_state_transition_structure;
use crate::{Error, Sdk};
use dpp::state_transition::identity_credit_withdrawal_transition::methods::{
    IdentityCreditWithdrawalTransitionMethodsV0, PreferredKeyPurposeForSigningWithdrawal,
};
use dpp::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::withdrawal::Pooling;
use std::future::Future;
use std::pin::Pin;

pub trait WithdrawFromIdentity {
    /// Function to withdraw credits from an identity. Returns the final identity balance.
    /// If signing_withdrawal_key_to_use is not set, we will try to use one in the signer that is
    /// available for withdrawal
    #[allow(clippy::too_many_arguments)]
    fn withdraw<'a, S: Signer<IdentityPublicKey> + Send + 'a>(
        &'a self,
        sdk: &'a Sdk,
        address: Option<Address>,
        amount: u64,
        core_fee_per_byte: Option<u32>,
        signing_withdrawal_key_to_use: Option<&'a IdentityPublicKey>,
        signer: S,
        settings: Option<PutSettings>,
    ) -> Pin<Box<dyn Future<Output = Result<u64, Error>> + Send + 'a>>;
}

impl WithdrawFromIdentity for Identity {
    fn withdraw<'a, S: Signer<IdentityPublicKey> + Send + 'a>(
        &'a self,
        sdk: &'a Sdk,
        address: Option<Address>,
        amount: u64,
        core_fee_per_byte: Option<u32>,
        signing_withdrawal_key_to_use: Option<&'a IdentityPublicKey>,
        signer: S,
        settings: Option<PutSettings>,
    ) -> Pin<Box<dyn Future<Output = Result<u64, Error>> + Send + 'a>> {
        Box::pin(async move {
            let new_identity_nonce = sdk.get_identity_nonce(self.id(), true, settings).await?;
            let script = address.map(|address| CoreScript::new(address.script_pubkey()));
            let user_fee_increase = settings.and_then(|settings| settings.user_fee_increase);
            let state_transition = IdentityCreditWithdrawalTransition::try_from_identity(
                self,
                script,
                amount,
                Pooling::Never,
                core_fee_per_byte.unwrap_or(1),
                user_fee_increase.unwrap_or_default(),
                signer,
                signing_withdrawal_key_to_use,
                PreferredKeyPurposeForSigningWithdrawal::TransferPreferred,
                new_identity_nonce,
                sdk.version(),
                None,
            )?;
            ensure_valid_state_transition_structure(&state_transition, sdk.version())?;

            let result = state_transition.broadcast_and_wait(sdk, settings).await?;

            match result {
                StateTransitionProofResult::VerifiedPartialIdentity(identity) => {
                    identity.balance.ok_or(Error::Generic(
                        "expected an identity balance after withdrawal".to_string(),
                    ))
                }
                _ => Err(Error::Generic("proved a non identity".to_string())),
            }
        })
    }
}
