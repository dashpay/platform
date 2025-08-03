#[cfg(feature = "state-transition-signing")]
use crate::{
    identity::{
        accessors::IdentityGettersV0, 
        identity_public_key::accessors::v0::IdentityPublicKeyGettersV0,
        signer::Signer, Identity, IdentityPublicKey, KeyType,
        Purpose, SecurityLevel,
    },
    prelude::{IdentityNonce, UserFeeIncrease},
    state_transition::StateTransition,
    ProtocolError,
};
#[cfg(feature = "state-transition-signing")]
use platform_value::Identifier;

use crate::state_transition::identity_credit_transfer_transition::methods::IdentityCreditTransferTransitionMethodsV0;
use crate::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::GetDataContractSecurityLevelRequirementFn;
#[cfg(feature = "state-transition-signing")]
use platform_version::version::{FeatureVersion, PlatformVersion};

impl IdentityCreditTransferTransitionMethodsV0 for IdentityCreditTransferTransitionV0 {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_identity<S: Signer>(
        identity: &Identity,
        to_identity_with_identifier: Identifier,
        amount: u64,
        user_fee_increase: UserFeeIncrease,
        signer: S,
        signing_withdrawal_key_to_use: Option<&IdentityPublicKey>,
        nonce: IdentityNonce,
        _platform_version: &PlatformVersion,
        _version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        eprintln!("🔵 try_from_identity: Started");
        eprintln!("🔵 try_from_identity: identity_id = {:?}", identity.id());
        eprintln!("🔵 try_from_identity: to_identity_with_identifier = {:?}", to_identity_with_identifier);
        eprintln!("🔵 try_from_identity: amount = {}", amount);
        eprintln!("🔵 try_from_identity: signing_withdrawal_key_to_use present = {}", signing_withdrawal_key_to_use.is_some());
        
        let mut transition: StateTransition = IdentityCreditTransferTransitionV0 {
            identity_id: identity.id(),
            recipient_id: to_identity_with_identifier,
            amount,
            nonce,
            user_fee_increase,
            signature_public_key_id: 0,
            signature: Default::default(),
        }
        .into();

        let identity_public_key = match signing_withdrawal_key_to_use {
            Some(key) => {
                if signer.can_sign_with(key) {
                    key
                } else {
                    eprintln!("❌ try_from_identity ERROR: Specified transfer key cannot be used for signing");
                    eprintln!("❌ try_from_identity: key.id() = {}", key.id());
                    eprintln!("❌ try_from_identity: signer.can_sign_with(key) = false");
                    return Err(
                        ProtocolError::DesiredKeyWithTypePurposeSecurityLevelMissing(
                            "specified transfer public key cannot be used for signing".to_string(),
                        ),
                    );
                }
            }
            None => {
                eprintln!("🔵 try_from_identity: No signing key specified, looking for TRANSFER key");
                eprintln!("🔵 try_from_identity: About to call get_first_public_key_matching");
                eprintln!("🔵 try_from_identity: Purpose = TRANSFER");
                eprintln!("🔵 try_from_identity: SecurityLevel = full_range");
                eprintln!("🔵 try_from_identity: KeyType = all_key_types");
                eprintln!("🔵 try_from_identity: allow_disabled = true");
                
                let key_result = identity
                    .get_first_public_key_matching(
                        Purpose::TRANSFER,
                        SecurityLevel::full_range().into(),
                        KeyType::all_key_types().into(),
                        true,
                    );
                    
                eprintln!("🔵 try_from_identity: get_first_public_key_matching returned: {}", key_result.is_some());
                
                key_result.ok_or_else(|| {
                    eprintln!("❌ try_from_identity ERROR: No transfer public key found in identity");
                    eprintln!("❌ try_from_identity: Total keys in identity: {}", identity.public_keys().len());
                    for (key_id, key) in identity.public_keys() {
                        eprintln!("❌ try_from_identity: Key {}: purpose = {:?}", key_id, key.purpose());
                    }
                    ProtocolError::DesiredKeyWithTypePurposeSecurityLevelMissing(
                        "no transfer public key".to_string(),
                    )
                })?
            }
        };

        eprintln!("🔵 try_from_identity: Found identity_public_key with ID: {}", identity_public_key.id());
        eprintln!("🔵 try_from_identity: About to call transition.sign_external");
        
        match transition.sign_external(
            identity_public_key,
            &signer,
            None::<GetDataContractSecurityLevelRequirementFn>,
        ) {
            Ok(_) => {
                eprintln!("🔵 try_from_identity: sign_external succeeded");
            },
            Err(e) => {
                eprintln!("❌ try_from_identity ERROR: sign_external failed: {:?}", e);
                return Err(e);
            }
        }

        eprintln!("✅ try_from_identity: Successfully created and signed transition");
        Ok(transition)
    }
}
