//! Signer-aware identity key selection shared by wallet operations.

use crate::error::SIGNER_KEY_UNAVAILABLE_PREFIX;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::ProtocolError;

/// Preserve the signer's unavailable-key discriminator through SDK and FFI errors.
pub(super) fn signing_key_unavailable(key: &IdentityPublicKey) -> ProtocolError {
    ProtocolError::Generic(format!(
        "{SIGNER_KEY_UNAVAILABLE_PREFIX}Signing key {} is unavailable to signer",
        key.id()
    ))
}

/// Select in key-ID order, preserving the operation's eligibility policy.
/// Call with an identity snapshot, outside wallet manager guards.
/// `Ok(None)` means no eligible key; `Err` means eligible keys are unavailable.
pub(super) fn available_signing_key<'a>(
    identity: &'a Identity,
    signer: &impl Signer<IdentityPublicKey>,
    purpose: Purpose,
    security_levels: &[SecurityLevel],
    key_types: &[KeyType],
    allow_disabled: bool,
) -> Result<Option<&'a IdentityPublicKey>, ProtocolError> {
    let eligible = identity.public_keys().values().filter(|key| {
        key.purpose() == purpose
            && security_levels.contains(&key.security_level())
            && key_types.contains(&key.key_type())
            && (allow_disabled || !key.is_disabled())
    });
    let mut unavailable = None;
    for key in eligible {
        if signer.can_sign_with(key) {
            return Ok(Some(key));
        }
        unavailable.get_or_insert(key);
    }
    match unavailable {
        Some(key) => Err(signing_key_unavailable(key)),
        None => Ok(None),
    }
}

/// Respect explicit keys; automatic withdrawals exhaust TRANSFER before OWNER.
pub(super) fn credit_signing_key<'a>(
    identity: &'a Identity,
    explicit: Option<&'a IdentityPublicKey>,
    signer: &impl Signer<IdentityPublicKey>,
    allow_owner: bool,
) -> Result<&'a IdentityPublicKey, ProtocolError> {
    if let Some(key) = explicit {
        return if signer.can_sign_with(key) {
            Ok(key)
        } else {
            Err(signing_key_unavailable(key))
        };
    }
    let mut unavailable = None;
    for purpose in [Purpose::TRANSFER]
        .into_iter()
        .chain(allow_owner.then_some(Purpose::OWNER))
    {
        match available_signing_key(
            identity,
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
    use dpp::version::PlatformVersion;
    use dpp::ProtocolError;

    #[derive(Debug)]
    struct AvailableKeys(Vec<u32>);

    #[async_trait]
    impl Signer<IdentityPublicKey> for AvailableKeys {
        async fn sign(&self, _: &IdentityPublicKey, _: &[u8]) -> Result<BinaryData, ProtocolError> {
            panic!("selection must not sign")
        }
        async fn sign_create_witness(
            &self,
            _: &IdentityPublicKey,
            _: &[u8],
        ) -> Result<AddressWitness, ProtocolError> {
            panic!("selection must not create witnesses")
        }
        fn can_sign_with(&self, key: &IdentityPublicKey) -> bool {
            self.0.contains(&key.id())
        }
    }

    fn identity(purpose: Purpose, level: SecurityLevel, key_type: KeyType) -> Identity {
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

    #[test]
    fn should_skip_unavailable_keys_for_each_signing_policy() {
        for (purpose, level) in [
            (Purpose::AUTHENTICATION, SecurityLevel::HIGH),
            (Purpose::AUTHENTICATION, SecurityLevel::CRITICAL),
            (Purpose::AUTHENTICATION, SecurityLevel::MASTER),
            (Purpose::TRANSFER, SecurityLevel::CRITICAL),
            (Purpose::OWNER, SecurityLevel::CRITICAL),
        ] {
            for key_type in [
                KeyType::ECDSA_SECP256K1,
                KeyType::ECDSA_HASH160,
                KeyType::BLS12_381,
            ] {
                let identity = identity(purpose, level, key_type);
                assert_eq!(
                    available_signing_key(
                        &identity,
                        &AvailableKeys(vec![2]),
                        purpose,
                        &[level],
                        &[key_type],
                        false
                    )
                    .unwrap()
                    .map(|k| k.id()),
                    Some(2)
                );
                assert_eq!(
                    available_signing_key(
                        &identity,
                        &AvailableKeys(vec![1, 2]),
                        purpose,
                        &[level],
                        &[key_type],
                        false
                    )
                    .unwrap()
                    .map(|k| k.id()),
                    Some(1)
                );
                assert_unavailable(
                    available_signing_key(
                        &identity,
                        &AvailableKeys(vec![]),
                        purpose,
                        &[level],
                        &[key_type],
                        false,
                    )
                    .unwrap_err(),
                );
            }
        }
    }

    #[test]
    fn should_not_relax_eligibility_for_available_keys() {
        let mut identity = identity(
            Purpose::AUTHENTICATION,
            SecurityLevel::CRITICAL,
            KeyType::ECDSA_SECP256K1,
        );
        identity
            .public_keys_mut()
            .get_mut(&1)
            .unwrap()
            .set_disabled_at(1);
        let signer = AvailableKeys(vec![1, 2]);
        assert_eq!(
            available_signing_key(
                &identity,
                &signer,
                Purpose::AUTHENTICATION,
                &[SecurityLevel::CRITICAL],
                &[KeyType::ECDSA_SECP256K1],
                false
            )
            .unwrap()
            .map(|k| k.id()),
            Some(2)
        );
        for (purpose, level, key_type) in [
            (
                Purpose::TRANSFER,
                SecurityLevel::CRITICAL,
                KeyType::ECDSA_SECP256K1,
            ),
            (
                Purpose::AUTHENTICATION,
                SecurityLevel::HIGH,
                KeyType::ECDSA_SECP256K1,
            ),
            (
                Purpose::AUTHENTICATION,
                SecurityLevel::CRITICAL,
                KeyType::ECDSA_HASH160,
            ),
        ] {
            assert!(available_signing_key(
                &identity,
                &signer,
                purpose,
                &[level],
                &[key_type],
                false
            )
            .unwrap()
            .is_none());
        }
    }
    #[test]
    fn should_exhaust_transfer_keys_before_owner_fallback() {
        let mut identity = identity(
            Purpose::TRANSFER,
            SecurityLevel::CRITICAL,
            KeyType::ECDSA_HASH160,
        );
        let mut owner = identity.public_keys().get(&1).unwrap().clone();
        owner.set_id(0);
        owner.set_purpose(Purpose::OWNER);
        identity.add_public_key(owner);
        assert_eq!(
            credit_signing_key(&identity, None, &AvailableKeys(vec![0, 2]), true)
                .unwrap()
                .id(),
            2
        );
        assert_eq!(
            credit_signing_key(&identity, None, &AvailableKeys(vec![0]), true)
                .unwrap()
                .id(),
            0
        );
        assert!(credit_signing_key(&identity, None, &AvailableKeys(vec![0]), false).is_err());
        assert!(credit_signing_key(&identity, None, &AvailableKeys(vec![]), true).is_err());
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
            credit_signing_key(&identity, Some(first), &AvailableKeys(vec![2]), true).unwrap_err(),
        );
        assert_eq!(
            credit_signing_key(&identity, Some(second), &AvailableKeys(vec![1, 2]), false).unwrap(),
            second
        );
    }

    fn assert_unavailable(error: ProtocolError) {
        assert!(
            matches!(error, ProtocolError::Generic(ref message)
            if message.starts_with(SIGNER_KEY_UNAVAILABLE_PREFIX)),
            "{error:?}"
        );
    }

    #[test]
    fn should_distinguish_unavailable_keys_from_ineligible_keys() {
        for (purpose, level, allow_disabled) in [
            (Purpose::AUTHENTICATION, SecurityLevel::HIGH, false),
            (Purpose::AUTHENTICATION, SecurityLevel::CRITICAL, false),
            (Purpose::AUTHENTICATION, SecurityLevel::MASTER, true),
            (Purpose::TRANSFER, SecurityLevel::CRITICAL, true),
            (Purpose::OWNER, SecurityLevel::CRITICAL, true),
        ] {
            for key_type in [
                KeyType::ECDSA_SECP256K1,
                KeyType::ECDSA_HASH160,
                KeyType::BLS12_381,
            ] {
                let mut identity = identity(purpose, level, key_type);
                // The signer can use the second key, but it has the wrong purpose.
                identity
                    .public_keys_mut()
                    .get_mut(&2)
                    .unwrap()
                    .set_purpose(Purpose::ENCRYPTION);
                let select = |identity: &Identity| {
                    available_signing_key(
                        identity,
                        &AvailableKeys(vec![2]),
                        purpose,
                        &[level],
                        &[key_type],
                        allow_disabled,
                    )
                    .map(|key| key.map(|key| key.id()))
                };
                assert_unavailable(select(&identity).unwrap_err());
                identity.public_keys_mut().remove(&1);
                assert_eq!(select(&identity).unwrap(), None);
            }
        }
    }

    #[test]
    fn should_distinguish_unavailable_credit_keys_from_missing_purposes() {
        let mut identity = identity(
            Purpose::OWNER,
            SecurityLevel::CRITICAL,
            KeyType::ECDSA_HASH160,
        );
        assert!(matches!(
            credit_signing_key(&identity, None, &AvailableKeys(vec![1, 2]), false),
            Err(ProtocolError::DesiredKeyWithTypePurposeSecurityLevelMissing(_))
        ));
        assert_unavailable(
            credit_signing_key(&identity, None, &AvailableKeys(vec![]), true).unwrap_err(),
        );
        identity.public_keys_mut().clear();
        for allow_owner in [false, true] {
            assert!(matches!(
                credit_signing_key(&identity, None, &AvailableKeys(vec![]), allow_owner),
                Err(ProtocolError::DesiredKeyWithTypePurposeSecurityLevelMissing(_))
            ));
        }
    }

    #[test]
    fn should_preserve_credit_disabled_key_policy() {
        let mut identity = identity(
            Purpose::TRANSFER,
            SecurityLevel::CRITICAL,
            KeyType::ECDSA_HASH160,
        );
        identity
            .public_keys_mut()
            .get_mut(&1)
            .unwrap()
            .set_disabled_at(1);
        assert_eq!(
            credit_signing_key(&identity, None, &AvailableKeys(vec![1]), false)
                .unwrap()
                .id(),
            1
        );
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

    #[derive(Debug)]
    pub(crate) struct LockCheckingSigner<'a>(pub &'a super::super::IdentityWallet);

    #[async_trait]
    impl Signer<IdentityPublicKey> for LockCheckingSigner<'_> {
        async fn sign(&self, _: &IdentityPublicKey, _: &[u8]) -> Result<BinaryData, ProtocolError> {
            panic!("unavailable signer must not sign")
        }
        async fn sign_create_witness(
            &self,
            _: &IdentityPublicKey,
            _: &[u8],
        ) -> Result<AddressWitness, ProtocolError> {
            panic!("unavailable signer must not create witnesses")
        }
        fn can_sign_with(&self, _: &IdentityPublicKey) -> bool {
            assert!(
                self.0.wallet_manager.try_write().is_ok(),
                "signer callback holds wallet manager lock"
            );
            false
        }
    }

    #[tokio::test]
    async fn should_release_profile_wallet_lock_before_signer_callback() {
        let wallet = wallet_with_signing_keys().await;
        let signer = LockCheckingSigner(wallet.identity());
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
