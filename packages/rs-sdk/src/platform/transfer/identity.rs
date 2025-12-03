use super::types::{IdentityTransferConfig, TransferInput, TransferOutput};
use crate::platform::transition::put_settings::PutSettings;
use crate::platform::transition::transfer::TransferToIdentity;
use crate::{Error, Sdk};
use dpp::fee::Credits;
use dpp::identifier::Identifier;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::prelude::UserFeeIncrease;
use dpp::state_transition::identity_credit_transfer_transition::methods::IdentityCreditTransferTransitionMethodsV0;
use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
use dpp::state_transition::StateTransition;
use std::collections::BTreeMap;

/// Minimal plan describing an identity-to-identity transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct IdentityTransferPlan {
    /// Destination identity receiving the credits.
    pub(crate) recipient_id: Identifier,
    /// Number of credits to transfer.
    pub(crate) amount: Credits,
}

/// Fully classified identity transfer with config and plan.
#[derive(Debug, Clone)]
pub(crate) struct IdentityTransferSelection {
    /// Funding identity configuration.
    pub(crate) config: IdentityTransferConfig,
    /// Planned transfer amount and destination.
    pub(crate) plan: IdentityTransferPlan,
}

/// Build identity transfer context after validating inputs and outputs.
pub(crate) fn classify_identity_transfer(
    inputs: &[TransferInput],
    outputs: &BTreeMap<TransferOutput, Credits>,
) -> Result<IdentityTransferSelection, Error> {
    if inputs.len() != 1 {
        return Err(Error::InvalidCreditTransfer(
            "identity transfer expects exactly one funding input".to_string(),
        ));
    }

    let config = match inputs.first() {
        Some(TransferInput::Identity(config)) => config.clone(),
        Some(_) => {
            return Err(Error::InvalidCreditTransfer(
                "identity transfer requires the funding input to be an identity".to_string(),
            ))
        }
        None => unreachable!(),
    };

    if outputs.len() != 1 {
        return Err(Error::InvalidCreditTransfer(
            "identity transfer expects exactly one output".to_string(),
        ));
    }

    let (recipient_id, amount) = match outputs.iter().next() {
        Some((TransferOutput::Identity(identity_id), amount)) => (*identity_id, *amount),
        Some(_) => {
            return Err(Error::InvalidCreditTransfer(
                "identity transfer output must be another identity".to_string(),
            ))
        }
        None => unreachable!(),
    };

    let plan = IdentityTransferPlan {
        recipient_id,
        amount,
    };

    Ok(IdentityTransferSelection { config, plan })
}

impl IdentityTransferConfig {
    /// Execute a transfer immediately, returning balance changes.
    pub async fn execute(
        &self,
        sdk: &Sdk,
        recipient_id: Identifier,
        amount: Credits,
        settings: Option<PutSettings>,
    ) -> Result<(u64, u64), Error> {
        self.identity
            .transfer_credits(
                sdk,
                recipient_id,
                amount,
                self.signing_key(),
                self.signer(),
                settings,
            )
            .await
    }

    /// Build a state transition for the given recipient and amount.
    pub async fn state_transition(
        &self,
        sdk: &Sdk,
        recipient_id: Identifier,
        amount: Credits,
        user_fee_increase: UserFeeIncrease,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error> {
        let nonce = sdk
            .get_identity_nonce(self.identity().id(), true, settings)
            .await?;

        let transition = IdentityCreditTransferTransition::try_from_identity(
            self.identity(),
            recipient_id,
            amount,
            user_fee_increase,
            self.signer(),
            self.signing_key(),
            nonce,
            sdk.version(),
            None,
        )?;

        Ok(transition)
    }
}

impl std::fmt::Debug for IdentityTransferConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IdentityTransferConfig")
            .field("identity", &self.identity.id())
            .finish()
    }
}
