//! Shield credits from an identity balance straight into the Orchard pool.

use super::broadcast::BroadcastStateTransition;
use super::put_settings::PutSettings;
use super::validation::ensure_valid_state_transition_structure;
use crate::platform::transition::waitable::Waitable;
use crate::{Error, Sdk};
use dpp::fee::Credits;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityPublicKey};
use dpp::shielded::OrchardBundleParams;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::shield_from_identity_transition::methods::ShieldFromIdentityTransitionMethodsV0;
use dpp::state_transition::shield_from_identity_transition::ShieldFromIdentityTransition;

/// Move `amount` credits from this identity's balance into the shielded pool using an
/// already proven outputs-only Orchard bundle (see
/// `dpp::shielded::builder::build_shield_from_identity_transition` for the full
/// build-and-sign path).
///
/// Returns the identity's proven post-debit balance and the proof height.
#[async_trait::async_trait]
pub trait ShieldFromIdentity: Waitable {
    async fn shield_from_identity<S: Signer<IdentityPublicKey> + Send>(
        &self,
        sdk: &Sdk,
        amount: u64,
        bundle: OrchardBundleParams,
        signing_transfer_key_to_use: Option<&IdentityPublicKey>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<(Credits, u64), Error>;
}

#[async_trait::async_trait]
impl ShieldFromIdentity for Identity {
    async fn shield_from_identity<S: Signer<IdentityPublicKey> + Send>(
        &self,
        sdk: &Sdk,
        amount: u64,
        bundle: OrchardBundleParams,
        signing_transfer_key_to_use: Option<&IdentityPublicKey>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<(Credits, u64), Error> {
        if amount == 0 {
            return Err(Error::Generic(
                "shield amount must be greater than zero".to_string(),
            ));
        }

        let new_identity_nonce = sdk.get_identity_nonce(self.id(), true, settings).await?;
        let user_fee_increase = settings
            .as_ref()
            .and_then(|settings| settings.user_fee_increase)
            .unwrap_or_default();

        let OrchardBundleParams {
            actions,
            anchor,
            proof,
            binding_signature,
        } = bundle;

        let state_transition = ShieldFromIdentityTransition::try_from_bundle_with_identity_signer(
            self,
            amount,
            actions,
            anchor,
            proof,
            binding_signature,
            user_fee_increase,
            signer,
            signing_transfer_key_to_use,
            new_identity_nonce,
            sdk.version(),
        )
        .await?;
        ensure_valid_state_transition_structure(&state_transition, sdk.version())?;

        let (st_result, metadata) = state_transition
            .broadcast_and_wait_for_affected_state_with_metadata::<StateTransitionProofResult>(
                sdk, settings,
            )
            .await?;
        match st_result {
            StateTransitionProofResult::VerifiedPartialIdentity(identity) => {
                if identity.id != self.id() {
                    return Err(Error::InvalidProvedResponse(format!(
                        "proof returned identity {} but {} initiated the shield",
                        identity.id,
                        self.id()
                    )));
                }
                let balance = identity.balance.ok_or_else(|| {
                    Error::InvalidProvedResponse(
                        "identity proof did not include updated balance".to_string(),
                    )
                })?;
                Ok((balance, metadata.height))
            }
            other => Err(Error::InvalidProvedResponse(format!(
                "identity balance proof was expected for {:?}, but received {:?}",
                state_transition, other
            ))),
        }
    }
}
