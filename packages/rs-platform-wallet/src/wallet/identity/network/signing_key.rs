//! Signer-aware identity key selection shared by wallet operations.

use crate::error::SIGNER_KEY_UNAVAILABLE_PREFIX;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::ProtocolError;

/// Preserve the signer's unavailable-key discriminator through SDK and FFI errors.
fn signing_key_unavailable(key: &IdentityPublicKey) -> dash_sdk::Error {
    ProtocolError::Generic(format!(
        "{SIGNER_KEY_UNAVAILABLE_PREFIX}Signing key {} is unavailable to signer",
        key.id()
    ))
    .into()
}

/// `key` itself, or the signer-unavailable error when `signer` cannot sign with it.
pub(super) fn require_available<'a>(
    key: &'a IdentityPublicKey,
    signer: &impl Signer<IdentityPublicKey>,
) -> Result<&'a IdentityPublicKey, dash_sdk::Error> {
    if signer.can_sign_with(key) {
        Ok(key)
    } else {
        Err(signing_key_unavailable(key))
    }
}

/// The first of the eligible `keys` that `signer` can sign with.
/// `Ok(None)` means no eligible key; `Err` means eligible keys are unavailable.
pub(super) fn first_available<'a>(
    keys: impl Iterator<Item = &'a IdentityPublicKey>,
    signer: &impl Signer<IdentityPublicKey>,
) -> Result<Option<&'a IdentityPublicKey>, dash_sdk::Error> {
    let mut unavailable = None;
    for key in keys {
        if signer.can_sign_with(key) {
            return Ok(Some(key));
        }
        unavailable.get_or_insert(key);
    }
    unavailable.map_or(Ok(None), |key| Err(signing_key_unavailable(key)))
}

/// Signer-aware `get_first_public_key_matching`: key-ID order, same eligibility policy.
/// Call on an identity snapshot, outside wallet manager guards.
/// `Ok(None)` means no eligible key; `Err` means eligible keys are unavailable.
pub(super) trait AvailableSigningKey {
    fn available_signing_key(
        &self,
        signer: &impl Signer<IdentityPublicKey>,
        purpose: Purpose,
        security_levels: &[SecurityLevel],
        key_types: &[KeyType],
        allow_disabled: bool,
    ) -> Result<Option<&IdentityPublicKey>, dash_sdk::Error>;
}

impl AvailableSigningKey for Identity {
    fn available_signing_key(
        &self,
        signer: &impl Signer<IdentityPublicKey>,
        purpose: Purpose,
        security_levels: &[SecurityLevel],
        key_types: &[KeyType],
        allow_disabled: bool,
    ) -> Result<Option<&IdentityPublicKey>, dash_sdk::Error> {
        let eligible = self.public_keys().values().filter(|key| {
            key.purpose() == purpose
                && security_levels.contains(&key.security_level())
                && key_types.contains(&key.key_type())
                && (allow_disabled || !key.is_disabled())
        });
        first_available(eligible, signer)
    }
}

/// Respect explicit keys; automatic withdrawals exhaust TRANSFER before OWNER.
pub(super) fn credit_signing_key<'a>(
    identity: &'a Identity,
    explicit: Option<&'a IdentityPublicKey>,
    signer: &impl Signer<IdentityPublicKey>,
    allow_owner: bool,
) -> Result<&'a IdentityPublicKey, dash_sdk::Error> {
    if let Some(key) = explicit {
        return require_available(key, signer);
    }
    let mut unavailable = None;
    for purpose in [Purpose::TRANSFER]
        .into_iter()
        .chain(allow_owner.then_some(Purpose::OWNER))
    {
        match identity.available_signing_key(
            signer,
            purpose,
            &SecurityLevel::full_range(),
            &KeyType::all_key_types(),
            true,
        ) {
            Ok(Some(key)) => return Ok(key),
            Ok(None) => {}
            Err(error) => {
                unavailable.get_or_insert(error);
            }
        }
    }
    Err(unavailable.unwrap_or_else(|| {
        ProtocolError::DesiredKeyWithTypePurposeSecurityLevelMissing(
            "No requested credit signing key available to signer".to_string(),
        )
        .into()
    }))
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use async_trait::async_trait;
    use dpp::address_funds::AddressWitness;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeySettersV0;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::platform_value::BinaryData;
    use dpp::prelude::Identifier;
    use dpp::version::PlatformVersion;

    /// Test signer that can sign with the keys `F` accepts; selection must never sign.
    pub(crate) struct KeyFilter<F: Fn(&IdentityPublicKey) -> bool>(pub F);

    impl<F: Fn(&IdentityPublicKey) -> bool> std::fmt::Debug for KeyFilter<F> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("KeyFilter")
        }
    }

    #[async_trait]
    impl<F: Fn(&IdentityPublicKey) -> bool + Send + Sync> Signer<IdentityPublicKey> for KeyFilter<F> {
        async fn sign(&self, _: &IdentityPublicKey, _: &[u8]) -> Result<BinaryData, ProtocolError> {
            panic!("key selection must not sign")
        }
        async fn sign_create_witness(
            &self,
            _: &IdentityPublicKey,
            _: &[u8],
        ) -> Result<AddressWitness, ProtocolError> {
            panic!("key selection must not create a witness")
        }
        fn can_sign_with(&self, key: &IdentityPublicKey) -> bool {
            (self.0)(key)
        }
    }

    /// A signer available only for the listed key ids.
    pub(crate) fn available(
        ids: &[u32],
    ) -> KeyFilter<impl Fn(&IdentityPublicKey) -> bool + Send + Sync + '_> {
        KeyFilter(move |key| ids.contains(&key.id()))
    }

    /// A signer with no available keys that asserts the wallet manager lock is free.
    pub(crate) fn lock_checking_signer(
        wallet: &super::super::IdentityWallet,
    ) -> KeyFilter<impl Fn(&IdentityPublicKey) -> bool + Send + Sync + '_> {
        KeyFilter(move |_| {
            assert!(
                wallet.wallet_manager.try_write().is_ok(),
                "signer callback holds wallet manager lock"
            );
            false
        })
    }

    pub(crate) fn identity(purpose: Purpose, level: SecurityLevel, key_type: KeyType) -> Identity {
        let mut identity = Identity::default_versioned(PlatformVersion::latest()).unwrap();
        for id in [1, 2] {
            identity.add_public_key(
                IdentityPublicKeyV0 {
                    id,
                    purpose,
                    security_level: level,
                    key_type,
                    data: vec![id as u8; key_type.default_size()].into(),
                    ..Default::default()
                }
                .into(),
            );
        }
        identity
    }

    fn assert_unavailable(error: dash_sdk::Error) {
        assert!(
            matches!(error, dash_sdk::Error::Protocol(ProtocolError::Generic(ref message))
            if message.starts_with(SIGNER_KEY_UNAVAILABLE_PREFIX)),
            "{error:?}"
        );
    }

    fn assert_missing(result: Result<&IdentityPublicKey, dash_sdk::Error>) {
        assert!(
            matches!(
                result,
                Err(dash_sdk::Error::Protocol(
                    ProtocolError::DesiredKeyWithTypePurposeSecurityLevelMissing(_)
                ))
            ),
            "{result:?}"
        );
    }

    #[test]
    fn should_skip_unavailable_keys_and_distinguish_them_from_ineligible_keys() {
        let mut identity = identity(
            Purpose::AUTHENTICATION,
            SecurityLevel::HIGH,
            KeyType::ECDSA_HASH160,
        );
        let mut ineligible = identity.public_keys().get(&2).unwrap().clone();
        ineligible.set_id(3);
        ineligible.set_purpose(Purpose::ENCRYPTION);
        identity.add_public_key(ineligible);
        let select = |identity: &Identity, ids: &[u32]| {
            identity
                .available_signing_key(
                    &available(ids),
                    Purpose::AUTHENTICATION,
                    &[SecurityLevel::HIGH],
                    &[KeyType::ECDSA_HASH160],
                    false,
                )
                .map(|key| key.map(|key| key.id()))
        };
        assert_eq!(select(&identity, &[2, 3]).unwrap(), Some(2));
        assert_eq!(select(&identity, &[1, 2]).unwrap(), Some(1));
        assert_unavailable(select(&identity, &[3]).unwrap_err());
        identity.public_keys_mut().retain(|id, _| *id == 3);
        assert_eq!(select(&identity, &[3]).unwrap(), None);
    }

    #[test]
    fn should_not_relax_eligibility_for_available_keys() {
        use KeyType::{ECDSA_HASH160 as HASH160, ECDSA_SECP256K1 as SECP};
        use Purpose::{AUTHENTICATION as AUTH, TRANSFER};
        use SecurityLevel::{CRITICAL, HIGH};
        let mut identity = identity(AUTH, CRITICAL, SECP);
        let key = identity.public_keys_mut().get_mut(&1).unwrap();
        key.set_disabled_at(1);
        let select = |purpose, level, key_type| {
            let signer = available(&[1, 2]);
            let key =
                identity.available_signing_key(&signer, purpose, &[level], &[key_type], false);
            key.unwrap().map(|k| k.id())
        };
        assert_eq!(
            select(AUTH, CRITICAL, SECP),
            Some(2),
            "disabled key 1 is skipped"
        );
        for (purpose, level, key_type) in [
            (TRANSFER, CRITICAL, SECP),
            (AUTH, HIGH, SECP),
            (AUTH, CRITICAL, HASH160),
        ] {
            assert_eq!(select(purpose, level, key_type), None);
        }
    }

    #[test]
    fn should_exhaust_transfer_keys_including_disabled_before_owner_fallback() {
        let mut identity = identity(
            Purpose::TRANSFER,
            SecurityLevel::CRITICAL,
            KeyType::ECDSA_HASH160,
        );
        let mut owner = identity.public_keys().get(&1).unwrap().clone();
        owner.set_id(0);
        owner.set_purpose(Purpose::OWNER);
        identity.add_public_key(owner);
        identity
            .public_keys_mut()
            .get_mut(&1)
            .unwrap()
            .set_disabled_at(1);
        let select = |ids: &[u32], allow_owner| {
            credit_signing_key(&identity, None, &available(ids), allow_owner).map(|k| k.id())
        };
        assert_eq!(select(&[0, 1], true).unwrap(), 1);
        assert_eq!(select(&[0, 2], true).unwrap(), 2);
        assert_eq!(select(&[0], true).unwrap(), 0);
        assert_unavailable(select(&[0], false).unwrap_err());
    }

    #[test]
    fn should_distinguish_unavailable_credit_keys_from_missing_purposes() {
        let mut identity = identity(
            Purpose::OWNER,
            SecurityLevel::CRITICAL,
            KeyType::ECDSA_HASH160,
        );
        assert_missing(credit_signing_key(
            &identity,
            None,
            &available(&[1, 2]),
            false,
        ));
        assert_unavailable(credit_signing_key(&identity, None, &available(&[]), true).unwrap_err());
        identity.public_keys_mut().clear();
        for allow_owner in [false, true] {
            assert_missing(credit_signing_key(
                &identity,
                None,
                &available(&[]),
                allow_owner,
            ));
        }
    }

    #[test]
    fn should_not_substitute_an_explicit_credit_key() {
        let identity = identity(
            Purpose::TRANSFER,
            SecurityLevel::CRITICAL,
            KeyType::ECDSA_HASH160,
        );
        let first = identity.public_keys().get(&1).unwrap();
        let second = identity.public_keys().get(&2).unwrap();
        assert_unavailable(
            credit_signing_key(&identity, Some(first), &available(&[2]), true).unwrap_err(),
        );
        assert_eq!(
            credit_signing_key(&identity, Some(second), &available(&[1, 2]), false).unwrap(),
            second
        );
    }

    #[tokio::test]
    async fn should_surface_unavailable_signer_from_credit_operations() {
        let wallet = wallet_with_signing_keys().await;
        let identity = identity(
            Purpose::TRANSFER,
            SecurityLevel::CRITICAL,
            KeyType::ECDSA_HASH160,
        );
        for explicit in [None, identity.public_keys().get(&1)] {
            let transfer = wallet.identity().transfer_credits_with_signer(
                &identity,
                Identifier::from([2; 32]),
                1,
                explicit,
                available(&[]),
                None,
            );
            assert_unavailable(transfer.await.unwrap_err());
            let withdraw = wallet.identity().withdraw_credits_with_signer(
                &identity,
                None,
                1,
                explicit,
                available(&[]),
                None,
            );
            assert_unavailable(withdraw.await.unwrap_err());
        }
    }

    pub(crate) async fn wallet_with_signing_keys() -> std::sync::Arc<crate::PlatformWallet> {
        use crate::events::{EventHandler, PlatformEventHandler};
        use crate::wallet::persister::NoPlatformPersistence;
        use crate::PlatformWalletManager;
        use key_wallet::wallet::initialization::WalletAccountCreationOptions;
        use std::sync::Arc;

        struct Events;
        impl EventHandler for Events {}
        impl PlatformEventHandler for Events {}
        let manager = PlatformWalletManager::new(
            Arc::new(dash_sdk::SdkBuilder::new_mock().build().unwrap()),
            Arc::new(NoPlatformPersistence),
            Arc::new(Events),
        );
        let wallet = manager
            .create_wallet_from_seed_bytes(
                key_wallet::Network::Testnet,
                &[7; 64],
                WalletAccountCreationOptions::Default,
                Some(0),
            )
            .await
            .unwrap();
        let mut wm = manager.wallet_manager.write().await;
        wm.get_wallet_info_mut(&wallet.wallet_id())
            .unwrap()
            .identity_manager
            .add_identity(
                identity(
                    Purpose::AUTHENTICATION,
                    SecurityLevel::CRITICAL,
                    KeyType::ECDSA_SECP256K1,
                ),
                0,
                wallet.wallet_id(),
                &wallet.identity().persister,
            )
            .unwrap();
        wallet
    }

    #[tokio::test]
    async fn should_release_profile_wallet_lock_before_signer_callback() {
        let wallet = wallet_with_signing_keys().await;
        let signer = lock_checking_signer(wallet.identity());
        let error = wallet
            .identity()
            .dashpay()
            .create_profile_with_external_signer(
                &dpp::prelude::Identifier::default(),
                crate::wallet::identity::ProfileUpdate::default(),
                &signer,
            )
            .await
            .unwrap_err();
        assert!(error.to_string().contains("available to signer"), "{error}");
    }
}
