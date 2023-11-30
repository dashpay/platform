//! Withdraw funds from an identity
use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::dashcore::Address;

use dpp::identity::core_script::CoreScript;
use dpp::identity::signer::Signer;
use dpp::identity::Identity;

use dpp::state_transition::identity_credit_withdrawal_transition::methods::IdentityCreditWithdrawalTransitionMethodsV0;
use dpp::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;

use crate::{Error, Sdk};
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::withdrawal::Pooling;
use drive::drive::Drive;
use rs_dapi_client::{DapiRequest, RequestSettings};

use super::{
    broadcast::BroadcastStateTransition, broadcast_request::BroadcastRequestForStateTransition,
};

#[async_trait::async_trait]
pub trait WithdrawFromIdentity {
    async fn withdraw<S: Signer + Send>(
        &self,
        sdk: &Sdk,
        address: Address,
        amount: u64,
        core_fee_per_byte: Option<u32>,
        signer: S,
    ) -> Result<u64, Error>;
}

#[async_trait::async_trait]
impl WithdrawFromIdentity for Identity {
    async fn withdraw<S: Signer + Send>(
        &self,
        sdk: &Sdk,
        address: Address,
        amount: u64,
        core_fee_per_byte: Option<u32>,
        signer: S,
    ) -> Result<u64, Error> {
        let state_transition = IdentityCreditWithdrawalTransition::try_from_identity(
            self,
            CoreScript::new(address.script_pubkey()),
            amount,
            Pooling::Never,
            core_fee_per_byte.unwrap_or(1),
            signer,
            sdk.version(),
            None,
        )?;
        let result = state_transition.broadcast_and_wait(sdk, None).await?;

        match result {
            StateTransitionProofResult::VerifiedPartialIdentity(identity) => {
                identity.balance.ok_or(Error::DapiClientError(
                    "expected an identity balance".to_string(),
                ))
            }
            _ => Err(Error::DapiClientError("proved a non identity".to_string())),
        }
    }
}
