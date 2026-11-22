//! Raising the limits of one of an identity's authentication keys (protocol version 14).
//!
//! A key registered with a budget or an expiry is topped up, or has its expiry moved later,
//! with an `IdentityKeyLimitsUpdate` transition rather than replaced. The wallet computes the
//! absolute values from the key it holds, signs with the identity's MASTER key (or a CRITICAL
//! authentication key without limits and without contract bounds) through the supplied
//! signer, and lays the key as stored after the update over its cache so the client's key
//! row follows through the persister.
//!
//! A form asks [`IdentityWallet::preview_identity_key_limits_update`] first: whether the key
//! can be raised at all, which key would sign, and what the update would set. The update runs
//! the same preview, and when it is handed the preview the user reviewed it refuses to sign
//! anything else, so a cache that moved in between cannot raise a limit further than shown.

use dash_sdk::platform::transition::put_settings::PutSettings;
use dash_sdk::platform::transition::update_identity_key_limits::{
    raised_key_limits, signing_key_for_key_limits_update, UpdateIdentityKeyLimits,
};
use dpp::fee::Credits;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v1::IdentityPublicKeyGettersV1;
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityPublicKey, KeyID, TimestampMillis};
use dpp::prelude::Identifier;

use super::update::SignerRef;
use super::*;
use crate::error::PlatformWalletError;

/// What a key limits update of one key would do, worked out from the identity as the wallet
/// holds it and before anything is signed: see
/// [`IdentityWallet::preview_identity_key_limits_update`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdentityKeyLimitsUpdatePreview {
    /// The key that would sign the update: the identity's MASTER key, else a CRITICAL
    /// authentication key without limits and without contract bounds, that the signer holds.
    pub signing_key_id: KeyID,
    /// The total budget the key carries now. `None` when it has no budget, and then none can
    /// be given to it: a budget is raised, not added.
    pub current_total_budget: Option<Credits>,
    /// The expiry the key carries now, in block time milliseconds. `None` when it has no
    /// expiry, and then none can be given to it.
    pub current_expires_at: Option<TimestampMillis>,
    /// The total budget the update would set; `None` when no budget was asked to be added.
    pub new_total_budget: Option<Credits>,
    /// The expiry the update would set; `None` when no expiry was asked for.
    pub new_expires_at: Option<TimestampMillis>,
}

/// The preview of raising the limits of the key `key_id` of `identity`, signed through
/// `signer`. Refuses a key that is missing or disabled, a key without any limit, a change
/// [`raised_key_limits`] refuses, and a signer that holds no key allowed to sign the update.
fn preview_key_limits_update<S>(
    identity: &Identity,
    key_id: KeyID,
    add_budget: Option<Credits>,
    expires_at: Option<TimestampMillis>,
    signer: &S,
) -> Result<IdentityKeyLimitsUpdatePreview, PlatformWalletError>
where
    S: Signer<IdentityPublicKey>,
{
    let (new_total_budget, new_expires_at) =
        raised_key_limits(identity, key_id, add_budget, expires_at)
            .map_err(|e| PlatformWalletError::InvalidIdentityData(e.to_string()))?;
    // `raised_key_limits` found the key, so `None` here cannot be a missing key; it is
    // answered the same way as a key without limits rather than indexed.
    let key = identity
        .public_keys()
        .get(&key_id)
        .filter(|key| key.has_limits())
        .ok_or_else(|| {
            PlatformWalletError::InvalidIdentityData(format!(
                "key {key_id} has no budget and no expiry: limits can be raised, not added"
            ))
        })?;
    let signing_key_id = signing_key_for_key_limits_update(identity, signer)
        .map_err(|e| PlatformWalletError::InvalidIdentityData(e.to_string()))?;

    Ok(IdentityKeyLimitsUpdatePreview {
        signing_key_id,
        current_total_budget: key.total_budget(),
        current_expires_at: key.expires_at(),
        new_total_budget,
        new_expires_at,
    })
}

/// The preview an update of the key `key_id` of `identity` signs. Unlike the preview a form
/// asks for, it needs a change: an update that raises nothing would be refused by Platform
/// (10539). When `reviewed` is the preview the caller showed, any difference from what would
/// be signed now (another total, another expiry, another signing key, because the held
/// identity moved in between) is refused before a nonce is taken, so the caller can show the
/// new preview instead.
fn preview_to_submit<S>(
    identity: &Identity,
    key_id: KeyID,
    add_budget: Option<Credits>,
    expires_at: Option<TimestampMillis>,
    reviewed: Option<&IdentityKeyLimitsUpdatePreview>,
    signer: &S,
) -> Result<IdentityKeyLimitsUpdatePreview, PlatformWalletError>
where
    S: Signer<IdentityPublicKey>,
{
    if add_budget.is_none() && expires_at.is_none() {
        return Err(PlatformWalletError::InvalidIdentityData(format!(
            "a key limits update of key {key_id} needs a budget to add or a new expiry"
        )));
    }
    let preview = preview_key_limits_update(identity, key_id, add_budget, expires_at, signer)?;
    if let Some(reviewed) = reviewed {
        if *reviewed != preview {
            return Err(PlatformWalletError::InvalidIdentityData(format!(
                "the limits of key {key_id} changed since they were reviewed \
                 (reviewed {reviewed:?}, now {preview:?}); nothing was signed, review the update again"
            )));
        }
    }
    Ok(preview)
}

impl IdentityWallet {
    /// What [`Self::update_identity_key_limits_with_external_signer`] would do for the same
    /// arguments, without signing or sending anything: the key that would sign, the limits
    /// the key carries, and the limits the update would set.
    ///
    /// With neither `add_budget` nor `expires_at` it answers whether the key can be raised at
    /// all and by which key, which is what a client asks before it offers the action. A
    /// refusal carries the reason as text: the key is missing or disabled, it has no limits,
    /// the change is not a raise, or the signer holds no MASTER key and no CRITICAL
    /// authentication key without limits and without contract bounds.
    ///
    /// The answer is computed from the identity as the wallet holds it. Whether the key is
    /// still expired after the update is decided by the block time, so Platform answers that.
    pub async fn preview_identity_key_limits_update<S>(
        &self,
        identity_id: &Identifier,
        key_id: KeyID,
        add_budget: Option<Credits>,
        expires_at: Option<TimestampMillis>,
        signer: &S,
    ) -> Result<IdentityKeyLimitsUpdatePreview, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        let identity = self.held_identity(identity_id).await?;
        preview_key_limits_update(&identity, key_id, add_budget, expires_at, signer)
    }

    /// The identity as the wallet holds it.
    async fn held_identity(
        &self,
        identity_id: &Identifier,
    ) -> Result<Identity, PlatformWalletError> {
        let wm = self.wallet_manager.read().await;
        let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
            PlatformWalletError::WalletNotFound(
                "Wallet info not found in wallet manager".to_string(),
            )
        })?;
        info.identity_manager
            .identity(identity_id)
            .map(|m| m.identity.clone())
            .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))
    }

    /// Raise the limits of the key `key_id` of `identity_id`: add `add_budget` credits to its
    /// total budget (and to what is left of it), and move its expiry to `expires_at`. At least
    /// one must be given. What consensus would refuse and charge for is refused here first, as
    /// [`Self::preview_identity_key_limits_update`] refuses it: a disabled key, a limit the
    /// key does not have, a zero top-up, an expiry that is not later.
    ///
    /// `reviewed` is the preview the caller showed for these arguments. When given, the update
    /// is refused, before a nonce is taken or anything is signed, if what it would sign now
    /// differs from it: the held identity moved in between (a refresh, another update), so the
    /// absolute limits or the signing key are no longer the ones the user saw.
    ///
    /// Signing is routed through the supplied `&S: Signer<IdentityPublicKey>`, which must hold
    /// the identity's MASTER key or a CRITICAL authentication key without limits and without
    /// contract bounds. No identity revision is claimed or bumped, so the cached identity only
    /// needs to hold the signing key.
    ///
    /// Resolves to the key as stored after the update, which is also laid over the cached
    /// identity and upserted through the persister, as
    /// [`Self::update_identity_with_external_signer`] does for an added key.
    #[allow(clippy::too_many_arguments)]
    pub async fn update_identity_key_limits_with_external_signer<S>(
        &self,
        identity_id: &Identifier,
        key_id: KeyID,
        add_budget: Option<Credits>,
        expires_at: Option<TimestampMillis>,
        reviewed: Option<&IdentityKeyLimitsUpdatePreview>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<IdentityPublicKey, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        let identity = self.held_identity(identity_id).await?;

        let preview =
            preview_to_submit(&identity, key_id, add_budget, expires_at, reviewed, signer)?;
        let signing_key = identity.public_keys().get(&preview.signing_key_id);

        let updated_key = identity
            .update_key_limits(
                &self.sdk,
                key_id,
                preview.new_total_budget,
                preview.new_expires_at,
                signing_key,
                SignerRef(signer),
                settings,
            )
            .await?;

        // Post-broadcast local apply: the cached key carries the raised limits and the
        // client's key row follows through the persister.
        {
            let mut wm = self.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&self.wallet_id).ok_or_else(|| {
                PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            if let Some(managed) = info.identity_manager.managed_identity_mut(identity_id) {
                managed
                    .replace_key(updated_key.clone(), &self.persister)
                    .map_err(|e| {
                        PlatformWalletError::Persistence(format!(
                            "identity key not persisted after the key limits update: {e}"
                        ))
                    })?;
            }
        }

        Ok(updated_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::address_funds::AddressWitness;
    use dpp::identity::identity_public_key::accessors::v0::{
        IdentityPublicKeyGettersV0, IdentityPublicKeySettersV0,
    };
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::v0::IdentityV0;
    use dpp::identity::{KeyType, Purpose, SecurityLevel};
    use dpp::platform_value::BinaryData;
    use dpp::ProtocolError;

    const EXPIRES_AT: TimestampMillis = 1_800_000_000_000;

    /// Holds the ids of the keys it can sign with; nothing is signed in these tests.
    #[derive(Debug)]
    struct KeyIdSigner(Vec<KeyID>);

    #[async_trait::async_trait]
    impl Signer<IdentityPublicKey> for KeyIdSigner {
        async fn sign(
            &self,
            _key: &IdentityPublicKey,
            _data: &[u8],
        ) -> Result<BinaryData, ProtocolError> {
            Err(ProtocolError::Generic(
                "not signing in this test".to_string(),
            ))
        }

        async fn sign_create_witness(
            &self,
            _key: &IdentityPublicKey,
            _data: &[u8],
        ) -> Result<AddressWitness, ProtocolError> {
            Err(ProtocolError::Generic(
                "not signing in this test".to_string(),
            ))
        }

        fn can_sign_with(&self, key: &IdentityPublicKey) -> bool {
            self.0.contains(&key.id())
        }
    }

    fn authentication_key(id: KeyID, security_level: SecurityLevel) -> IdentityPublicKey {
        IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id,
            purpose: Purpose::AUTHENTICATION,
            security_level,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(vec![id as u8; 33]),
            disabled_at: None,
        })
    }

    /// MASTER key 0, an unlimited HIGH key 1, a HIGH key 2 with a budget, a HIGH key 3 with an
    /// expiry, and a HIGH key 4 with both.
    fn identity() -> Identity {
        let keys = [
            authentication_key(0, SecurityLevel::MASTER),
            authentication_key(1, SecurityLevel::HIGH),
            authentication_key(2, SecurityLevel::HIGH).with_limits(Some(1_000), None),
            authentication_key(3, SecurityLevel::HIGH).with_limits(None, Some(EXPIRES_AT)),
            authentication_key(4, SecurityLevel::HIGH).with_limits(Some(1_000), Some(EXPIRES_AT)),
        ];
        IdentityV0 {
            id: Identifier::from([1; 32]),
            public_keys: keys.into_iter().map(|key| (key.id(), key)).collect(),
            balance: 0,
            revision: 0,
        }
        .into()
    }

    fn master_signer() -> KeyIdSigner {
        KeyIdSigner(vec![0])
    }

    #[test]
    fn should_report_the_limits_a_key_carries_and_the_signing_key_when_no_change_is_asked() {
        let preview = preview_key_limits_update(&identity(), 2, None, None, &master_signer())
            .expect("expected a budgeted key to be raisable");
        assert_eq!(
            preview,
            IdentityKeyLimitsUpdatePreview {
                signing_key_id: 0,
                current_total_budget: Some(1_000),
                current_expires_at: None,
                new_total_budget: None,
                new_expires_at: None,
            }
        );

        let preview = preview_key_limits_update(&identity(), 3, None, None, &master_signer())
            .expect("expected an expiring key to be raisable");
        assert_eq!(preview.current_total_budget, None);
        assert_eq!(preview.current_expires_at, Some(EXPIRES_AT));
    }

    #[test]
    fn should_report_the_absolute_limits_the_update_would_set() {
        let preview = preview_key_limits_update(
            &identity(),
            4,
            Some(500),
            Some(EXPIRES_AT + 1),
            &master_signer(),
        )
        .expect("expected a raise of both limits");
        assert_eq!(
            preview,
            IdentityKeyLimitsUpdatePreview {
                signing_key_id: 0,
                current_total_budget: Some(1_000),
                current_expires_at: Some(EXPIRES_AT),
                new_total_budget: Some(1_500),
                new_expires_at: Some(EXPIRES_AT + 1),
            }
        );
    }

    #[test]
    fn should_refuse_a_key_that_cannot_be_raised() {
        let mut disabled = identity();
        disabled
            .public_keys_mut()
            .get_mut(&2)
            .expect("expected key 2")
            .set_disabled_at(1);

        for (identity, key_id, reason) in [
            (identity(), 1, "no budget and no expiry"),
            (identity(), 0, "no budget and no expiry"),
            (identity(), 9, "no key 9"),
            (disabled, 2, "disabled"),
        ] {
            let result = preview_key_limits_update(&identity, key_id, None, None, &master_signer());
            assert!(
                matches!(
                    &result,
                    Err(PlatformWalletError::InvalidIdentityData(message)) if message.contains(reason)
                ),
                "key {key_id} must be refused with `{reason}`: {result:?}"
            );
        }
    }

    #[test]
    fn should_refuse_a_change_that_is_not_a_raise() {
        for (key_id, add_budget, expires_at) in [
            (2, Some(0), None),
            (2, None, Some(EXPIRES_AT + 1)),
            (3, Some(10), None),
            (3, None, Some(EXPIRES_AT)),
            (4, Some(10), Some(EXPIRES_AT - 1)),
        ] {
            let result = preview_key_limits_update(
                &identity(),
                key_id,
                add_budget,
                expires_at,
                &master_signer(),
            );
            assert!(
                matches!(result, Err(PlatformWalletError::InvalidIdentityData(_))),
                "key {key_id} {add_budget:?} {expires_at:?} must be refused: {result:?}"
            );
        }
    }

    #[test]
    fn should_refuse_to_submit_an_update_that_raises_nothing() {
        let result = preview_to_submit(&identity(), 2, None, None, None, &master_signer());
        assert!(
            matches!(
                &result,
                Err(PlatformWalletError::InvalidIdentityData(message)) if message.contains("needs a budget to add")
            ),
            "{result:?}"
        );
    }

    #[test]
    fn should_submit_only_the_limits_and_signing_key_that_were_reviewed() {
        let reviewed = preview_key_limits_update(&identity(), 2, Some(500), None, &master_signer())
            .expect("expected a raise");
        assert_eq!(reviewed.new_total_budget, Some(1_500));

        let submitted = preview_to_submit(
            &identity(),
            2,
            Some(500),
            None,
            Some(&reviewed),
            &master_signer(),
        )
        .expect("expected the reviewed preview to be submitted");
        assert_eq!(submitted, reviewed);

        // The held key's total moved from 1_000 to 1_200 after the review: the same
        // increment would now sign 1_700, which the user never saw.
        let mut moved = identity();
        let raised = moved.public_keys()[&2]
            .clone()
            .with_limits(Some(1_200), None);
        moved.public_keys_mut().insert(2, raised);
        let result = preview_to_submit(
            &moved,
            2,
            Some(500),
            None,
            Some(&reviewed),
            &master_signer(),
        );
        assert!(
            matches!(
                &result,
                Err(PlatformWalletError::InvalidIdentityData(message)) if message.contains("changed since they were reviewed")
            ),
            "{result:?}"
        );

        // Same limits, another signing key: refused too.
        let mut critical = identity();
        critical
            .public_keys_mut()
            .insert(5, authentication_key(5, SecurityLevel::CRITICAL));
        let result = preview_to_submit(
            &critical,
            2,
            Some(500),
            None,
            Some(&reviewed),
            &KeyIdSigner(vec![5]),
        );
        assert!(
            matches!(result, Err(PlatformWalletError::InvalidIdentityData(_))),
            "{result:?}"
        );
    }

    #[test]
    fn should_refuse_a_signer_that_holds_no_key_allowed_to_raise_limits() {
        // Key 1 is HIGH and key 4 is limited: neither may sign a key limits update.
        let result =
            preview_key_limits_update(&identity(), 2, Some(10), None, &KeyIdSigner(vec![1, 4]));
        assert!(
            matches!(
                &result,
                Err(PlatformWalletError::InvalidIdentityData(message)) if message.contains("MASTER")
            ),
            "{result:?}"
        );
    }
}
