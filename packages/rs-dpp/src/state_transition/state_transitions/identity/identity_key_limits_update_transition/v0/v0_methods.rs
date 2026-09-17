#[cfg(feature = "state-transition-signing")]
use crate::consensus::signature::{MissingPublicKeyError, SignatureError};
#[cfg(feature = "state-transition-signing")]
use crate::consensus::ConsensusError;
use crate::fee::Credits;
#[cfg(feature = "state-transition-signing")]
use crate::identity::accessors::IdentityGettersV0;
#[cfg(feature = "state-transition-signing")]
use crate::identity::signer::Signer;
#[cfg(feature = "state-transition-signing")]
use crate::identity::{Identity, IdentityPublicKey};
use crate::identity::{KeyID, TimestampMillis};
#[cfg(feature = "state-transition-signing")]
use crate::prelude::UserFeeIncrease;
use crate::prelude::{Identifier, IdentityNonce, Revision};
use crate::state_transition::identity_key_limits_update_transition::accessors::IdentityKeyLimitsUpdateTransitionAccessorsV0;
use crate::state_transition::identity_key_limits_update_transition::methods::IdentityKeyLimitsUpdateTransitionMethodsV0;
use crate::state_transition::identity_key_limits_update_transition::v0::IdentityKeyLimitsUpdateTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::{GetDataContractSecurityLevelRequirementFn, StateTransition};
#[cfg(feature = "state-transition-signing")]
use crate::version::FeatureVersion;
#[cfg(feature = "state-transition-signing")]
use crate::ProtocolError;
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

impl IdentityKeyLimitsUpdateTransitionMethodsV0 for IdentityKeyLimitsUpdateTransitionV0 {
    #[cfg(feature = "state-transition-signing")]
    async fn try_from_identity_with_signer<S: Signer<IdentityPublicKey>>(
        identity: &Identity,
        signing_key_id: &KeyID,
        key_id: KeyID,
        total_budget: Option<Credits>,
        expires_at: Option<TimestampMillis>,
        nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        _platform_version: &PlatformVersion,
        _version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        let signing_key = identity
            .public_keys()
            .get(signing_key_id)
            .ok_or::<ConsensusError>(
                SignatureError::MissingPublicKeyError(MissingPublicKeyError::new(*signing_key_id))
                    .into(),
            )?;

        let mut state_transition: StateTransition = IdentityKeyLimitsUpdateTransitionV0 {
            identity_id: identity.id(),
            revision: identity.revision() + 1,
            nonce,
            key_id,
            total_budget,
            expires_at,
            user_fee_increase,
            signature_public_key_id: 0,
            signature: Default::default(),
        }
        .into();

        // Refuses a key that is neither MASTER nor CRITICAL, or is disabled; a CRITICAL key with
        // limits is refused by the identity signature validation against state.
        state_transition
            .sign_external(
                signing_key,
                signer,
                None::<GetDataContractSecurityLevelRequirementFn>,
            )
            .await?;

        Ok(state_transition)
    }
}

impl IdentityKeyLimitsUpdateTransitionAccessorsV0 for IdentityKeyLimitsUpdateTransitionV0 {
    fn set_identity_id(&mut self, id: Identifier) {
        self.identity_id = id;
    }

    fn identity_id(&self) -> Identifier {
        self.identity_id
    }

    fn set_revision(&mut self, revision: Revision) {
        self.revision = revision;
    }

    fn revision(&self) -> Revision {
        self.revision
    }

    fn set_nonce(&mut self, nonce: IdentityNonce) {
        self.nonce = nonce;
    }

    fn nonce(&self) -> IdentityNonce {
        self.nonce
    }

    fn set_key_id(&mut self, key_id: KeyID) {
        self.key_id = key_id;
    }

    fn key_id(&self) -> KeyID {
        self.key_id
    }

    fn set_total_budget(&mut self, total_budget: Option<Credits>) {
        self.total_budget = total_budget;
    }

    fn total_budget(&self) -> Option<Credits> {
        self.total_budget
    }

    fn set_expires_at(&mut self, expires_at: Option<TimestampMillis>) {
        self.expires_at = expires_at;
    }

    fn expires_at(&self) -> Option<TimestampMillis> {
        self.expires_at
    }
}
