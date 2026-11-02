#[cfg(feature = "state-transition-signing")]
use crate::serialization::Signable;

#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

#[cfg(feature = "state-transition-signing")]
use crate::consensus::signature::{
    InvalidSignaturePublicKeySecurityLevelError, MissingPublicKeyError, SignatureError,
};
#[cfg(feature = "state-transition-signing")]
use crate::consensus::ConsensusError;
#[cfg(feature = "state-transition-signing")]
use crate::identity::signer::Signer;
#[cfg(feature = "state-transition-signing")]
use crate::identity::{Identity, IdentityPublicKey};

#[cfg(feature = "state-transition-signing")]
use crate::identity::accessors::IdentityGettersV0;
#[cfg(feature = "state-transition-signing")]
use crate::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
#[cfg(feature = "state-transition-signing")]
use crate::identity::key_disable_rule::check_keys_can_be_disabled;
use crate::prelude::IdentityNonce;
#[cfg(feature = "state-transition-signing")]
use crate::prelude::UserFeeIncrease;
use crate::state_transition::identity_update_transition::accessors::IdentityUpdateTransitionAccessorsV0;
use crate::state_transition::identity_update_transition::methods::IdentityUpdateTransitionMethodsV0;
use crate::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Setters;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::{GetDataContractSecurityLevelRequirementFn, StateTransition};
#[cfg(feature = "state-transition-signing")]
use crate::version::FeatureVersion;
use crate::{
    identity::KeyID,
    prelude::{Identifier, Revision},
};
#[cfg(feature = "state-transition-signing")]
use crate::{identity::SecurityLevel, ProtocolError};

impl IdentityUpdateTransitionMethodsV0 for IdentityUpdateTransitionV0 {
    #[cfg(feature = "state-transition-signing")]
    async fn try_from_identity_with_signer<S: Signer<IdentityPublicKey>>(
        identity: &Identity,
        master_public_key_id: &KeyID,
        add_public_keys: Vec<IdentityPublicKey>,
        disable_public_keys: Vec<KeyID>,
        nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        _platform_version: &PlatformVersion,
        _version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        // DIP-11 signing-side policy, checked before anything is signed. This is an in-place edit
        // of a generation every protocol version selects, and it is inert for consensus at all of
        // them: only clients build transitions here (no Drive or drive-abci code path calls this
        // builder), and a request the rule accepts yields the same transition as before. dpp
        // reads no clock, so a key with any expiry does not count as remaining (`None`).
        check_keys_can_be_disabled(identity, &add_public_keys, &disable_public_keys, None)?;

        let add_public_keys_in_creation = add_public_keys
            .iter()
            .map(|public_key| public_key.into())
            .collect();

        let mut identity_update_transition = IdentityUpdateTransitionV0 {
            signature: Default::default(),
            signature_public_key_id: 0,
            identity_id: identity.id(),
            revision: identity.revision(),
            nonce,
            add_public_keys: add_public_keys_in_creation,
            disable_public_keys,
            user_fee_increase,
        };

        let state_transition: StateTransition = identity_update_transition.clone().into();

        let key_signable_bytes = state_transition.signable_bytes()?;

        // Sign all the keys
        for (public_key_with_witness, public_key) in identity_update_transition
            .add_public_keys
            .iter_mut()
            .zip(add_public_keys.iter())
        {
            if public_key.key_type().is_unique_key_type() {
                let signature = signer.sign(public_key, &key_signable_bytes).await?;
                public_key_with_witness.set_signature(signature);
            }
        }

        let master_public_key = identity
            .public_keys()
            .get(master_public_key_id)
            .ok_or::<ConsensusError>(
                SignatureError::MissingPublicKeyError(MissingPublicKeyError::new(
                    *master_public_key_id,
                ))
                .into(),
            )?;
        if master_public_key.security_level() != SecurityLevel::MASTER {
            Err(ProtocolError::InvalidSignaturePublicKeySecurityLevelError(
                InvalidSignaturePublicKeySecurityLevelError::new(
                    master_public_key.security_level(),
                    vec![SecurityLevel::MASTER],
                ),
            ))
        } else {
            let mut state_transition: StateTransition = identity_update_transition.into();
            state_transition
                .sign_external(
                    master_public_key,
                    signer,
                    None::<GetDataContractSecurityLevelRequirementFn>,
                )
                .await?;
            Ok(state_transition)
        }
    }
}

impl IdentityUpdateTransitionAccessorsV0 for IdentityUpdateTransitionV0 {
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

    fn set_public_keys_to_add(&mut self, add_public_keys: Vec<IdentityPublicKeyInCreation>) {
        self.add_public_keys = add_public_keys;
    }

    fn public_keys_to_add(&self) -> &[IdentityPublicKeyInCreation] {
        &self.add_public_keys
    }

    fn public_keys_to_add_mut(&mut self) -> &mut [IdentityPublicKeyInCreation] {
        &mut self.add_public_keys
    }

    fn set_public_key_ids_to_disable(&mut self, disable_public_keys: Vec<KeyID>) {
        self.disable_public_keys = disable_public_keys;
    }

    fn public_key_ids_to_disable(&self) -> &[KeyID] {
        &self.disable_public_keys
    }
}

#[cfg(all(test, feature = "state-transition-signing"))]
mod tests {
    use crate::address_funds::AddressWitness;
    use crate::identity::accessors::IdentityGettersV0;
    use crate::identity::key_disable_rule::tests::{authentication_key, key, one_key_per_level};
    use crate::identity::key_disable_rule::LastAuthenticationKeyError;
    use crate::identity::signer::Signer;
    use crate::identity::{Identity, IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel};
    use crate::state_transition::identity_update_transition::methods::IdentityUpdateTransitionMethodsV0;
    use crate::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
    use crate::state_transition::identity_update_transition::IdentityUpdateTransition;
    use crate::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Setters;
    use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
    use crate::state_transition::StateTransition;
    use crate::ProtocolError;
    use assert_matches::assert_matches;
    use async_trait::async_trait;
    use platform_value::BinaryData;
    use platform_version::version::{PlatformVersion, PLATFORM_VERSIONS};
    use std::sync::atomic::{AtomicUsize, Ordering};

    const SIGNATURE: [u8; 65] = [7; 65];

    /// Returns [`SIGNATURE`] for every request and counts them.
    #[derive(Debug, Default)]
    struct CountingSigner {
        signatures: AtomicUsize,
    }

    impl CountingSigner {
        fn signatures(&self) -> usize {
            self.signatures.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl Signer<IdentityPublicKey> for CountingSigner {
        async fn sign(
            &self,
            _key: &IdentityPublicKey,
            _data: &[u8],
        ) -> Result<BinaryData, ProtocolError> {
            self.signatures.fetch_add(1, Ordering::SeqCst);
            Ok(SIGNATURE.to_vec().into())
        }

        async fn sign_create_witness(
            &self,
            _key: &IdentityPublicKey,
            _data: &[u8],
        ) -> Result<AddressWitness, ProtocolError> {
            Err(ProtocolError::Generic(
                "identity updates create no address witnesses".to_string(),
            ))
        }

        fn can_sign_with(&self, _key: &IdentityPublicKey) -> bool {
            true
        }
    }

    async fn build(
        identity: &Identity,
        add_public_keys: Vec<IdentityPublicKey>,
        disable_public_keys: Vec<KeyID>,
        signer: &CountingSigner,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError> {
        IdentityUpdateTransition::try_from_identity_with_signer(
            identity,
            &0,
            add_public_keys,
            disable_public_keys,
            5,
            0,
            signer,
            platform_version,
            None,
        )
        .await
    }

    #[tokio::test]
    async fn should_refuse_disabling_the_last_critical_key_before_signing_anything() {
        let signer = CountingSigner::default();
        // The added key is a unique key type, so a build that got past the rule would sign it.
        let added = authentication_key(4, SecurityLevel::MEDIUM);
        let result = build(
            &one_key_per_level(),
            vec![added],
            vec![1],
            &signer,
            PlatformVersion::latest(),
        )
        .await;
        assert_matches!(
            result,
            Err(ProtocolError::LastAuthenticationKey(
                LastAuthenticationKeyError {
                    key_id: 1,
                    security_level: SecurityLevel::CRITICAL,
                }
            ))
        );
        assert_eq!(signer.signatures(), 0, "nothing may be signed");
    }

    #[tokio::test]
    async fn should_not_count_an_expiring_key_as_remaining_in_the_builder() {
        // The builder has no clock: even an expiry far in the future does not count.
        let mut identity = one_key_per_level();
        identity.add_public_key(
            authentication_key(4, SecurityLevel::CRITICAL).with_limits(None, Some(u64::MAX)),
        );
        let signer = CountingSigner::default();
        let result = build(
            &identity,
            vec![],
            vec![1],
            &signer,
            PlatformVersion::latest(),
        )
        .await;
        assert_matches!(
            result,
            Err(ProtocolError::LastAuthenticationKey(
                LastAuthenticationKeyError { key_id: 1, .. }
            ))
        );
        assert_eq!(signer.signatures(), 0);

        // The same key without the expiry keeps key 1 disableable.
        identity.add_public_key(authentication_key(4, SecurityLevel::CRITICAL));
        assert!(build(
            &identity,
            vec![],
            vec![1],
            &signer,
            PlatformVersion::latest()
        )
        .await
        .is_ok());
    }

    #[tokio::test]
    async fn should_count_a_hash160_key_as_remaining_in_the_builder() {
        let mut identity = one_key_per_level();
        identity.add_public_key(key(
            4,
            Purpose::AUTHENTICATION,
            SecurityLevel::CRITICAL,
            KeyType::ECDSA_HASH160,
        ));
        let signer = CountingSigner::default();
        let transition = build(
            &identity,
            vec![],
            vec![1],
            &signer,
            PlatformVersion::latest(),
        )
        .await
        .expect("an ECDSA_HASH160 CRITICAL key keeps the CRITICAL level valid");
        assert_matches!(transition, StateTransition::IdentityUpdate(_));
    }

    /// The in-place edit of this shipped generation is inert for a request the rule accepts:
    /// at every protocol version, the last shipped one included, the builder produces exactly
    /// the transition it produced before the rule existed.
    #[tokio::test]
    async fn should_build_an_unchanged_transition_for_a_compliant_update_at_every_version() {
        let identity = one_key_per_level();
        let added = authentication_key(4, SecurityLevel::HIGH);
        let mut added_in_creation: IdentityPublicKeyInCreation = (&added).into();
        added_in_creation.set_signature(SIGNATURE.to_vec().into());
        let expected: StateTransition = IdentityUpdateTransitionV0 {
            identity_id: identity.id(),
            revision: identity.revision(),
            nonce: 5,
            add_public_keys: vec![added_in_creation],
            // CRITICAL 1 stands in for HIGH 2.
            disable_public_keys: vec![2],
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: SIGNATURE.to_vec().into(),
        }
        .into();

        for platform_version in PLATFORM_VERSIONS {
            let signer = CountingSigner::default();
            let transition = build(
                &identity,
                vec![added.clone()],
                vec![2],
                &signer,
                platform_version,
            )
            .await
            .unwrap_or_else(|e| {
                panic!(
                    "protocol version {}: {e}",
                    platform_version.protocol_version
                )
            });
            assert_eq!(
                transition, expected,
                "protocol version {}",
                platform_version.protocol_version
            );
        }
    }
}
