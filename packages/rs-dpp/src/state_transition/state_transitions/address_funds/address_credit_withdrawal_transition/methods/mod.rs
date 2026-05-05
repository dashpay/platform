mod v0;

#[cfg(feature = "state-transition-signing")]
use std::collections::BTreeMap;
pub use v0::*;

#[cfg(feature = "state-transition-signing")]
use crate::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
#[cfg(feature = "state-transition-signing")]
use crate::fee::Credits;
#[cfg(feature = "state-transition-signing")]
use crate::identity::core_script::CoreScript;
#[cfg(feature = "state-transition-signing")]
use crate::identity::signer::Signer;
use crate::state_transition::address_credit_withdrawal_transition::AddressCreditWithdrawalTransition;
#[cfg(feature = "state-transition-signing")]
use crate::withdrawal::Pooling;
#[cfg(feature = "state-transition-signing")]
use crate::{
    prelude::{AddressNonce, UserFeeIncrease},
    state_transition::{
        address_credit_withdrawal_transition::v0::AddressCreditWithdrawalTransitionV0,
        StateTransition,
    },
    ProtocolError,
};
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

impl AddressCreditWithdrawalTransitionMethodsV0 for AddressCreditWithdrawalTransition {
    #[cfg(feature = "state-transition-signing")]
    async fn try_from_inputs_with_signer<S: Signer<PlatformAddress>>(
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        output: Option<(PlatformAddress, Credits)>,
        fee_strategy: AddressFundsFeeStrategy,
        core_fee_per_byte: u32,
        pooling: Pooling,
        output_script: CoreScript,
        signer: &S,
        user_fee_increase: UserFeeIncrease,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError> {
        match platform_version
            .dpp
            .state_transitions
            .address_funds
            .credit_withdrawal
        {
            0 => Ok(
                AddressCreditWithdrawalTransitionV0::try_from_inputs_with_signer::<S>(
                    inputs,
                    output,
                    fee_strategy,
                    core_fee_per_byte,
                    pooling,
                    output_script,
                    signer,
                    user_fee_increase,
                    platform_version,
                )
                .await?,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "AddressCreditWithdrawalTransition::try_from_inputs_with_signer"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
