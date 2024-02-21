use crate::identity::accessors::IdentityGettersV0;
use crate::identity::core_script::CoreScript;
use crate::identity::signer::Signer;
use crate::identity::{Identity, KeyType, Purpose, SecurityLevel};
use crate::prelude::IdentityNonce;
use crate::ProtocolError;
use platform_version::version::{FeatureVersion, PlatformVersion};

use crate::state_transition::identity_credit_withdrawal_transition::methods::IdentityCreditWithdrawalTransitionMethodsV0;
use crate::state_transition::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0;
use crate::state_transition::{GetDataContractSecurityLevelRequirementFn, StateTransition};
use crate::withdrawal::Pooling;

impl IdentityCreditWithdrawalTransitionMethodsV0 for IdentityCreditWithdrawalTransitionV0 {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_identity<S: Signer>(
        identity: &Identity,
        output_script: CoreScript,
        amount: u64,
        pooling: Pooling,
        core_fee_per_byte: u32,
        signer: S,
        nonce: IdentityNonce,
        _platform_version: &PlatformVersion,
        _version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        let mut transition: StateTransition = IdentityCreditWithdrawalTransitionV0 {
            identity_id: identity.id(),
            amount,
            core_fee_per_byte,
            pooling,
            output_script,
            nonce,
            signature_public_key_id: 0,
            signature: Default::default(),
        }
        .into();

        let identity_public_key = identity
            .get_first_public_key_matching(
                Purpose::WITHDRAW,
                SecurityLevel::full_range().into(),
                KeyType::all_key_types().into(),
            )
            .ok_or(
                ProtocolError::DesiredKeyWithTypePurposeSecurityLevelMissing(
                    "no withdrawal public key".to_string(),
                ),
            )?;

        transition.sign_external(
            identity_public_key,
            &signer,
            None::<GetDataContractSecurityLevelRequirementFn>,
        )?;

        Ok(transition)
    }
}
