//! Signer-aware identity key selection shared by wallet operations.

use super::key_selection::{
    check_signing_key, usable_signing_key, SigningScope, PLATFORM_SIGNING_KEY_TYPES,
    WALLET_SIGNING_KEY_TYPES,
};
use crate::error::SIGNER_KEY_UNAVAILABLE_PREFIX;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityPublicKey, Purpose, SecurityLevel};
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

/// The security levels [`contract_signing_key`] accepts.
pub(super) const CONTRACT_SIGNING_LEVELS: [SecurityLevel; 1] = [SecurityLevel::CRITICAL];

/// The security levels [`master_signing_key`] accepts.
pub(super) const MASTER_SIGNING_LEVELS: [SecurityLevel; 1] = [SecurityLevel::MASTER];

/// The key that signs a contract create or update: a CRITICAL AUTHENTICATION ECDSA key.
/// These are not batches, so a contract-bound key cannot sign them.
pub(super) fn contract_signing_key<'a>(
    identity: &'a Identity,
    signer: &impl Signer<IdentityPublicKey>,
) -> Result<Option<&'a IdentityPublicKey>, dash_sdk::Error> {
    usable_signing_key(
        identity,
        signer,
        Purpose::AUTHENTICATION,
        &CONTRACT_SIGNING_LEVELS,
        &WALLET_SIGNING_KEY_TYPES,
        SigningScope::NonBatch,
    )
}

/// The MASTER key that authorizes an identity update. Platform refuses a disabled signing key.
pub(super) fn master_signing_key<'a>(
    identity: &'a Identity,
    signer: &impl Signer<IdentityPublicKey>,
) -> Result<Option<&'a IdentityPublicKey>, dash_sdk::Error> {
    usable_signing_key(
        identity,
        signer,
        Purpose::AUTHENTICATION,
        &MASTER_SIGNING_LEVELS,
        &WALLET_SIGNING_KEY_TYPES,
        SigningScope::NonBatch,
    )
}

/// The security levels a credit transfer or withdrawal accepts, whatever the key's purpose.
const CREDIT_SIGNING_LEVELS: [SecurityLevel; 1] = [SecurityLevel::CRITICAL];

/// The key that signs a credit transfer or withdrawal: a CRITICAL TRANSFER key, or a CRITICAL
/// OWNER key when `allow_owner`, which only a withdrawal to the identity's payout address sets.
/// An explicit key is never substituted; it is checked by the rules automatic selection
/// applies, and refused with Platform's typed error. Automatic withdrawals exhaust TRANSFER
/// keys before OWNER.
pub(super) fn credit_signing_key<'a>(
    identity: &'a Identity,
    explicit: Option<&'a IdentityPublicKey>,
    signer: &impl Signer<IdentityPublicKey>,
    allow_owner: bool,
) -> Result<&'a IdentityPublicKey, dash_sdk::Error> {
    let purposes: Vec<Purpose> = [Purpose::TRANSFER]
        .into_iter()
        .chain(allow_owner.then_some(Purpose::OWNER))
        .collect();
    if let Some(key) = explicit {
        check_signing_key(
            key,
            signer,
            &purposes,
            &CREDIT_SIGNING_LEVELS,
            &PLATFORM_SIGNING_KEY_TYPES,
            SigningScope::NonBatch,
        )?;
        return Ok(key);
    }
    let mut unavailable = None;
    for purpose in purposes {
        match usable_signing_key(
            identity,
            signer,
            purpose,
            &CREDIT_SIGNING_LEVELS,
            &PLATFORM_SIGNING_KEY_TYPES,
            SigningScope::NonBatch,
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
    use super::super::key_selection::tests::refused_purposes;
    use super::*;
    use crate::{PlatformWallet, PlatformWalletError};
    use async_trait::async_trait;
    use dpp::address_funds::AddressWitness;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeySettersV0;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::KeyType;
    use dpp::platform_value::BinaryData;
    use dpp::prelude::Identifier;
    use dpp::version::PlatformVersion;
    use std::sync::Arc;

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
            usable_signing_key(
                identity,
                &available(ids),
                Purpose::AUTHENTICATION,
                &[SecurityLevel::HIGH],
                &[KeyType::ECDSA_HASH160],
                SigningScope::NonBatch,
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
            let key = usable_signing_key(
                &identity,
                &signer,
                purpose,
                &[level],
                &[key_type],
                SigningScope::NonBatch,
            );
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
    fn should_sign_credit_operations_only_with_a_critical_key() {
        let mut identity = identity(
            Purpose::TRANSFER,
            SecurityLevel::HIGH,
            KeyType::ECDSA_HASH160,
        );
        let mut critical = identity.public_keys().get(&2).unwrap().clone();
        critical.set_id(3);
        critical.set_security_level(SecurityLevel::CRITICAL);
        identity.add_public_key(critical);
        let mut owner = identity.public_keys().get(&2).unwrap().clone();
        owner.set_id(4);
        owner.set_purpose(Purpose::OWNER);
        identity.add_public_key(owner);
        let signer = available(&[1, 2, 3, 4]);
        assert_eq!(
            credit_signing_key(&identity, None, &signer, true)
                .unwrap()
                .id(),
            3,
            "the HIGH TRANSFER keys 1 and 2 are skipped"
        );
        for id in [1, 4] {
            let refusal =
                credit_signing_key(&identity, identity.public_keys().get(&id), &signer, true);
            assert!(
                matches!(
                    refusal,
                    Err(dash_sdk::Error::Protocol(
                        ProtocolError::InvalidSignaturePublicKeySecurityLevelError(_)
                    ))
                ),
                "credit transitions take only a CRITICAL key: {refusal:?}"
            );
        }
    }

    #[test]
    fn should_pick_only_a_credit_key_of_a_type_platform_verifies() {
        let mut identity = identity(
            Purpose::TRANSFER,
            SecurityLevel::CRITICAL,
            KeyType::ECDSA_HASH160,
        );
        for (id, key_type) in [
            (1, KeyType::BIP13_SCRIPT_HASH),
            (2, KeyType::EDDSA_25519_HASH160),
        ] {
            identity
                .public_keys_mut()
                .get_mut(&id)
                .unwrap()
                .set_key_type(key_type);
        }
        let mut verifiable = identity.public_keys().get(&2).unwrap().clone();
        verifiable.set_id(3);
        verifiable.set_key_type(KeyType::ECDSA_HASH160);
        identity.add_public_key(verifiable);
        assert_eq!(
            credit_signing_key(&identity, None, &available(&[1, 2, 3]), false)
                .unwrap()
                .id(),
            3,
            "Drive cannot verify a signature by keys 1 and 2"
        );
    }

    #[test]
    fn should_skip_disabled_transfer_keys_before_owner_fallback() {
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
        assert_eq!(
            select(&[0, 1, 2], true).unwrap(),
            2,
            "Platform refuses a disabled signing key, so key 1 is never chosen"
        );
        assert_eq!(
            select(&[0, 1], true).unwrap(),
            0,
            "only the disabled TRANSFER key is available, so OWNER signs"
        );
        assert_eq!(select(&[0], true).unwrap(), 0);
        assert_unavailable(select(&[1], false).unwrap_err());
    }

    #[test]
    fn should_sign_a_contract_transition_only_with_a_key_free_of_contract_bounds() {
        use super::super::key_selection::tests::{bound_key, identity as identity_of, key};
        use dpp::identity::identity_public_key::contract_bounds::ContractBounds;

        let identity = identity_of(vec![
            bound_key(
                1,
                SecurityLevel::CRITICAL,
                Some(ContractBounds::SingleContract {
                    id: Identifier::from([0x0C; 32]),
                }),
            ),
            key(2, SecurityLevel::CRITICAL).with_limits(Some(1_000), None),
            key(3, SecurityLevel::CRITICAL),
        ]);
        let chosen = contract_signing_key(&identity, &KeyFilter(|_| true)).unwrap();
        assert_eq!(
            chosen.map(|key| key.id()),
            Some(3),
            "Platform refuses a contract-bound key outside a batch, and an unlimited key is \
             taken before a budgeted one"
        );
    }

    #[test]
    fn should_skip_a_disabled_master_key() {
        let mut identity = identity(
            Purpose::AUTHENTICATION,
            SecurityLevel::MASTER,
            KeyType::ECDSA_SECP256K1,
        );
        identity
            .public_keys_mut()
            .get_mut(&1)
            .unwrap()
            .set_disabled_at(1);
        let select = |identity: &Identity, ids: &[u32]| {
            master_signing_key(identity, &available(ids)).map(|key| key.map(|key| key.id()))
        };
        assert_eq!(select(&identity, &[1, 2]).unwrap(), Some(2));
        assert_unavailable(select(&identity, &[1]).unwrap_err());
        identity.public_keys_mut().retain(|id, _| *id == 1);
        assert_eq!(
            select(&identity, &[1]).unwrap(),
            None,
            "a disabled key is not eligible at all"
        );
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

    #[test]
    fn should_refuse_an_explicit_credit_key_platform_would_refuse() {
        let mut identity = identity(
            Purpose::TRANSFER,
            SecurityLevel::CRITICAL,
            KeyType::ECDSA_HASH160,
        );
        let mut owner = identity.public_keys().get(&2).unwrap().clone();
        owner.set_id(3);
        owner.set_purpose(Purpose::OWNER);
        let mut authentication = owner.clone();
        identity.add_public_key(owner);
        let mut script_hash = authentication.clone();
        authentication.set_id(4);
        authentication.set_purpose(Purpose::AUTHENTICATION);
        identity.add_public_key(authentication);
        script_hash.set_id(5);
        script_hash.set_purpose(Purpose::TRANSFER);
        script_hash.set_key_type(KeyType::BIP13_SCRIPT_HASH);
        identity.add_public_key(script_hash);
        identity
            .public_keys_mut()
            .get_mut(&1)
            .unwrap()
            .set_disabled_at(1);
        let explicit = |id: u32, allow_owner| {
            credit_signing_key(
                &identity,
                identity.public_keys().get(&id),
                &available(&[1, 2, 3]),
                allow_owner,
            )
            .map(|key| key.id())
        };
        assert!(
            matches!(
                explicit(1, true),
                Err(dash_sdk::Error::Protocol(
                    ProtocolError::PublicKeyIsDisabledError(_)
                ))
            ),
            "Platform refuses a disabled signing key"
        );
        assert_eq!(
            explicit(3, false).as_ref().err().and_then(refused_purposes),
            Some(vec![Purpose::TRANSFER]),
            "a credit transfer takes only a TRANSFER key"
        );
        assert_eq!(
            explicit(4, true).as_ref().err().and_then(refused_purposes),
            Some(vec![Purpose::TRANSFER, Purpose::OWNER]),
            "a withdrawal names both purposes it takes"
        );
        assert!(
            matches!(
                explicit(5, false),
                Err(dash_sdk::Error::Protocol(
                    ProtocolError::InvalidIdentityPublicKeyTypeError(_)
                ))
            ),
            "Platform cannot verify an identity signature by a script hash"
        );
        assert_eq!(explicit(3, true).unwrap(), 3, "a withdrawal may take OWNER");
        assert_eq!(explicit(2, false).unwrap(), 2);
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

    #[tokio::test]
    async fn should_not_sign_a_withdrawal_to_an_address_with_an_owner_key() {
        use dashcore::{Address as DashAddress, Network};

        let wallet = wallet_with_signing_keys().await;
        let identity = identity(
            Purpose::OWNER,
            SecurityLevel::CRITICAL,
            KeyType::ECDSA_HASH160,
        );
        let to_address = Some(DashAddress::dummy(Network::Testnet, 1));
        let automatic = wallet
            .identity()
            .withdraw_credits_with_signer(
                &identity,
                to_address.clone(),
                1,
                None,
                available(&[1, 2]),
                None,
            )
            .await;
        assert!(
            matches!(
                automatic,
                Err(dash_sdk::Error::Protocol(
                    ProtocolError::DesiredKeyWithTypePurposeSecurityLevelMissing(_)
                ))
            ),
            "only an OWNER key is held, and it cannot sign this withdrawal: {automatic:?}"
        );
        let explicit = wallet
            .identity()
            .withdraw_credits_with_signer(
                &identity,
                to_address,
                1,
                identity.public_keys().get(&1),
                available(&[1, 2]),
                None,
            )
            .await;
        assert_eq!(
            explicit.as_ref().err().and_then(refused_purposes),
            Some(vec![Purpose::TRANSFER]),
            "Platform refuses an OWNER key for a withdrawal to a chosen address: {explicit:?}"
        );
    }

    pub(crate) async fn wallet_with_signing_keys() -> Arc<PlatformWallet> {
        wallet_with_identity(identity(
            Purpose::AUTHENTICATION,
            SecurityLevel::CRITICAL,
            KeyType::ECDSA_SECP256K1,
        ))
        .await
    }

    /// A mock-SDK wallet that owns `identity`.
    pub(crate) async fn wallet_with_identity(identity: Identity) -> Arc<PlatformWallet> {
        use crate::events::{EventHandler, PlatformEventHandler};
        use crate::wallet::persister::NoPlatformPersistence;
        use crate::PlatformWalletManager;
        use key_wallet::wallet::initialization::WalletAccountCreationOptions;

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
                identity,
                0,
                wallet.wallet_id(),
                &wallet.identity().persister,
            )
            .unwrap();
        wallet
    }

    /// A wallet identity holding only HIGH AUTHENTICATION keys: none can sign a contract
    /// create or an identity update.
    async fn wallet_without_critical_or_master_keys() -> (Arc<PlatformWallet>, Identifier) {
        let identity = identity(
            Purpose::AUTHENTICATION,
            SecurityLevel::HIGH,
            KeyType::ECDSA_SECP256K1,
        );
        let identity_id = identity.id();
        (wallet_with_identity(identity).await, identity_id)
    }

    pub(crate) fn assert_no_usable_key(
        error: PlatformWalletError,
        identity: Identifier,
        level: SecurityLevel,
    ) {
        assert!(
            matches!(
                &error,
                PlatformWalletError::NoUsableSigningKey {
                    identity_id,
                    purpose: Purpose::AUTHENTICATION,
                    security_levels,
                    key_types,
                    ..
                } if *identity_id == identity
                    && security_levels == &[level]
                    && key_types == &[KeyType::ECDSA_SECP256K1]
            ),
            "{error:?}"
        );
    }

    #[tokio::test]
    async fn should_report_no_usable_contract_key_as_a_typed_error() {
        let (wallet, identity_id) = wallet_without_critical_or_master_keys().await;
        let error = wallet
            .identity()
            .create_data_contract_with_signer(
                &identity_id,
                "{}",
                None,
                None,
                None,
                None,
                None,
                &KeyFilter(|_| true),
            )
            .await
            .unwrap_err();
        assert_no_usable_key(error, identity_id, SecurityLevel::CRITICAL);
    }

    #[tokio::test]
    async fn should_report_no_usable_master_key_as_a_typed_error() {
        let (wallet, identity_id) = wallet_without_critical_or_master_keys().await;
        let error = wallet
            .identity()
            .update_identity_with_external_signer(
                &identity_id,
                vec![],
                vec![],
                &KeyFilter(|_| true),
                None,
            )
            .await
            .unwrap_err();
        assert_no_usable_key(error, identity_id, SecurityLevel::MASTER);
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
