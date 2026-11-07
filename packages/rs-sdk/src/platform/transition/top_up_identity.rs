#[cfg(feature = "core_key_wallet")]
use super::asset_lock_submission::{
    settle_top_up, submit_funding_with_fee_floor, FundingFailure, FundingFailureKind,
};
use super::broadcast::BroadcastStateTransition;
use super::put_settings::PutSettings;
use super::validation::ensure_valid_state_transition_structure;
use super::waitable::Waitable;
use crate::{Error, Sdk};
#[cfg(feature = "core_key_wallet")]
use dapi_grpc::platform::v0::ResponseMetadata;
use dpp::dashcore::PrivateKey;
#[cfg(feature = "core_key_wallet")]
use dpp::fee::Credits;
use dpp::identity::{Identity, PartialIdentity};
use dpp::prelude::AssetLockProof;
#[cfg(feature = "core_key_wallet")]
use dpp::prelude::UserFeeIncrease;
use dpp::state_transition::identity_topup_transition::methods::IdentityTopUpTransitionMethodsV0;
use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;

#[async_trait::async_trait]
pub trait TopUpIdentity: Waitable {
    /// Tops up an existing identity using an asset lock proof whose
    /// private key is held in-process.
    ///
    /// Prefer [`Self::top_up_identity_with_signer`] when the asset-lock
    /// private key lives outside Rust (Swift / hardware wallet / HSM):
    /// the `_with_signer` variant routes asset-lock signing through an
    /// external [`key_wallet::signer::Signer`] so the private key never
    /// crosses the FFI boundary as raw bytes.
    ///
    /// The balance returned is the identity's in the wait's proof. A node
    /// may answer that from a block before this top-up, and it does not
    /// prove this top-up executed: settle the lock with
    /// [`Sdk::asset_lock_outpoint_state`].
    async fn top_up_identity_with_private_key(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &PrivateKey,
        settings: Option<PutSettings>,
    ) -> Result<u64, Error>;

    /// Tops up an existing identity using an asset-lock signer.
    ///
    /// Signer-driven counterpart to
    /// [`Self::top_up_identity_with_private_key`]. `asset_lock_signer`
    /// produces the outer state-transition ECDSA signature for the key
    /// at `asset_lock_proof_path` — atomically deriving, signing, and
    /// zeroising inside the signer's trust boundary.
    ///
    /// The balance returned is the identity's in the wait's proof. A node
    /// may answer that from a block before this top-up, and it does not
    /// prove this top-up executed: use [`Self::top_up_identity_settled`],
    /// or settle the lock with [`Sdk::asset_lock_outpoint_state`].
    #[cfg(feature = "core_key_wallet")]
    async fn top_up_identity_with_signer<AS>(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_path: &dpp::key_wallet::bip32::DerivationPath,
        asset_lock_signer: &AS,
        settings: Option<PutSettings>,
    ) -> Result<u64, Error>
    where
        AS: dpp::key_wallet::signer::Signer + Send + Sync;

    /// Tops up an existing identity like [`Self::top_up_identity_with_signer`],
    /// through [`submit_funding_with_fee_floor`] from `user_fee_increase_floor`
    /// (`on_attempt` gets each attempt's fee before it is built), and settles
    /// the lock. The wait proves only a snapshot of the identity's balance,
    /// which binds neither this top-up nor this lock, so the lock is settled
    /// only when its proved record ([`Sdk::asset_lock_outpoint_state`]) shows
    /// it used up. Otherwise the failure is [`FundingFailureKind::Submitted`],
    /// with [`Error::AssetLockNotUsedUp`] naming the record, or the error that
    /// kept it unread.
    ///
    /// `Ok` carries the wait's snapshot, which need not include this top-up.
    /// It settles the lock only: the record names no transition, so it does
    /// not prove this identity was credited.
    #[cfg(feature = "core_key_wallet")]
    async fn top_up_identity_settled<AS, A>(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_path: &dpp::key_wallet::bip32::DerivationPath,
        asset_lock_signer: &AS,
        user_fee_increase_floor: UserFeeIncrease,
        on_attempt: A,
    ) -> Result<Credits, FundingFailure>
    where
        AS: dpp::key_wallet::signer::Signer + Send + Sync,
        A: Fn(UserFeeIncrease) + Send + Sync,
    {
        let out_point = asset_lock_proof.out_point().ok_or_else(|| FundingFailure {
            kind: FundingFailureKind::Unavailable,
            error: Error::InvalidRequest("the asset lock proof names no outpoint".to_string()),
        })?;
        let balance = submit_funding_with_fee_floor(
            &asset_lock_proof,
            user_fee_increase_floor,
            on_attempt,
            |settings| {
                self.top_up_identity_with_signer(
                    sdk,
                    asset_lock_proof.clone(),
                    asset_lock_proof_path,
                    asset_lock_signer,
                    Some(settings),
                )
            },
        )
        .await?;
        settle_top_up(
            out_point,
            balance,
            sdk.asset_lock_outpoint_state(&out_point).await,
        )
    }
}

/// Balance operations that also expose the committed proof metadata.
#[async_trait::async_trait]
pub trait TopUpIdentityWithMetadata: Waitable {
    /// Returns the balance in the wait's proof and that proof's metadata.
    /// The balance is a snapshot, which a node may answer from a block
    /// before this top-up: it does not prove this top-up executed.
    #[cfg(feature = "core_key_wallet")]
    async fn top_up_identity_with_signer_with_metadata<AS>(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_path: &dpp::key_wallet::bip32::DerivationPath,
        asset_lock_signer: &AS,
        settings: Option<PutSettings>,
    ) -> Result<(u64, ResponseMetadata), Error>
    where
        AS: dpp::key_wallet::signer::Signer + Send + Sync;
}

#[async_trait::async_trait]
impl TopUpIdentity for Identity {
    async fn top_up_identity_with_private_key(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &PrivateKey,
        settings: Option<PutSettings>,
    ) -> Result<u64, Error> {
        let user_fee_increase = settings
            .and_then(|s| s.user_fee_increase)
            .unwrap_or_default();
        let state_transition = IdentityTopUpTransition::try_from_identity_with_private_key(
            self,
            asset_lock_proof,
            &asset_lock_proof_private_key.inner.to_bytes()[..],
            user_fee_increase,
            sdk.version(),
            None,
        )?;
        ensure_valid_state_transition_structure(&state_transition, sdk.version())?;
        let identity: PartialIdentity = state_transition
            .broadcast_and_wait_for_affected_state(sdk, settings)
            .await?;

        identity
            .balance
            .ok_or(Error::Generic("expected an identity balance".to_string()))
    }

    #[cfg(feature = "core_key_wallet")]
    async fn top_up_identity_with_signer<AS>(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_path: &dpp::key_wallet::bip32::DerivationPath,
        asset_lock_signer: &AS,
        settings: Option<PutSettings>,
    ) -> Result<u64, Error>
    where
        AS: dpp::key_wallet::signer::Signer + Send + Sync,
    {
        self.top_up_identity_with_signer_with_metadata(
            sdk,
            asset_lock_proof,
            asset_lock_proof_path,
            asset_lock_signer,
            settings,
        )
        .await
        .map(|(balance, _)| balance)
    }
}

#[async_trait::async_trait]
impl TopUpIdentityWithMetadata for Identity {
    #[cfg(feature = "core_key_wallet")]
    async fn top_up_identity_with_signer_with_metadata<AS>(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_path: &dpp::key_wallet::bip32::DerivationPath,
        asset_lock_signer: &AS,
        settings: Option<PutSettings>,
    ) -> Result<(u64, ResponseMetadata), Error>
    where
        AS: dpp::key_wallet::signer::Signer + Send + Sync,
    {
        let user_fee_increase = settings
            .and_then(|s| s.user_fee_increase)
            .unwrap_or_default();
        let state_transition = IdentityTopUpTransition::try_from_identity_with_signer(
            self,
            asset_lock_proof,
            asset_lock_proof_path,
            asset_lock_signer,
            user_fee_increase,
            sdk.version(),
            None,
        )
        .await?;
        ensure_valid_state_transition_structure(&state_transition, sdk.version())?;
        let (identity, metadata): (PartialIdentity, _) = state_transition
            .broadcast_and_wait_for_affected_state_with_metadata(sdk, settings)
            .await?;

        identity
            .balance
            .ok_or(Error::Generic("expected an identity balance".to_string()))
            .map(|balance| (balance, metadata))
    }
}
