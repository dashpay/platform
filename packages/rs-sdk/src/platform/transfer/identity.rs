use super::types::{
    DynIdentitySigner, IdentityTransferConfig, IdentityTransferSigner, TransferInput,
    TransferOutput,
};
use crate::platform::transition::put_settings::PutSettings;
use crate::platform::transition::transfer::TransferToIdentity;
use crate::{Error, Sdk};
use dpp::fee::Credits;
use dpp::identifier::Identifier;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::IdentityPublicKey;
use dpp::platform_value::BinaryData;
use dpp::prelude::UserFeeIncrease;
use dpp::state_transition::identity_credit_transfer_transition::methods::IdentityCreditTransferTransitionMethodsV0;
use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
use dpp::state_transition::StateTransition;
use dpp::ProtocolError;
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct IdentityTransferPlan {
    pub(crate) recipient_id: Identifier,
    pub(crate) amount: Credits,
}

#[derive(Debug)]
pub(crate) struct IdentityTransferContext<'a> {
    pub(crate) config: &'a IdentityTransferConfig,
    pub(crate) plan: IdentityTransferPlan,
}

pub(crate) fn classify_identity_transfer<'a>(
    inputs: &'a [TransferInput],
    outputs: &BTreeMap<TransferOutput, Credits>,
) -> Result<IdentityTransferContext<'a>, Error> {
    if inputs.len() != 1 {
        return Err(Error::InvalidCreditTransfer(
            "identity transfer expects exactly one funding input".to_string(),
        ));
    }

    let config = match inputs.first() {
        Some(TransferInput::Identity(config)) => config,
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

    Ok(IdentityTransferContext { config, plan })
}

impl IdentityTransferConfig {
    pub fn signer_arc(&self) -> Arc<DynIdentitySigner> {
        self.signer.inner()
    }

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
                self.signer().clone(),
                settings,
            )
            .await
    }

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
            self.signer().clone(),
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

impl dpp::identity::signer::Signer<IdentityPublicKey> for IdentityTransferSigner {
    fn sign(&self, key: &IdentityPublicKey, data: &[u8]) -> Result<BinaryData, ProtocolError> {
        self.inner().sign(key, data)
    }

    fn sign_create_witness(
        &self,
        key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<dpp::address_funds::AddressWitness, ProtocolError> {
        self.inner().sign_create_witness(key, data)
    }

    fn can_sign_with(&self, key: &IdentityPublicKey) -> bool {
        self.inner().can_sign_with(key)
    }
}
