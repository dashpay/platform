use crate::address_funds::OrchardAddress;
use crate::identity::signer::Signer;
use crate::identity::{Identity, IdentityPublicKey};
use crate::prelude::{IdentityNonce, UserFeeIncrease};
use crate::state_transition::shield_from_identity_transition::methods::ShieldFromIdentityTransitionMethodsV0;
use crate::state_transition::shield_from_identity_transition::ShieldFromIdentityTransition;
use crate::state_transition::StateTransition;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

use super::{build_output_only_bundle, serialize_authorized_bundle, OrchardProver};

/// Build a `ShieldFromIdentity` transition: prove an outputs-only Orchard bundle
/// paying `shield_amount` to `recipient`, then identity-sign it with a TRANSFER key.
///
/// The identity nonce must already be the next nonce for `identity` (the SDK
/// fetches it); `amount + fee` is debited from the identity balance at execution.
#[allow(clippy::too_many_arguments)]
pub async fn build_shield_from_identity_transition<
    S: Signer<IdentityPublicKey>,
    P: OrchardProver,
>(
    identity: &Identity,
    recipient: &OrchardAddress,
    shield_amount: u64,
    nonce: IdentityNonce,
    signer: &S,
    signing_transfer_key_to_use: Option<&IdentityPublicKey>,
    user_fee_increase: UserFeeIncrease,
    prover: &P,
    memo: [u8; 36],
    sender_ovk: Option<grovedb_commitment_tree::OutgoingViewingKey>,
    platform_version: &PlatformVersion,
) -> Result<StateTransition, ProtocolError> {
    if shield_amount == 0 {
        return Err(ProtocolError::ShieldedBuildError(
            "shield amount must be greater than zero".to_string(),
        ));
    }

    let bundle = build_output_only_bundle(recipient, shield_amount, memo, sender_ovk, 0, prover)?;
    let sb = serialize_authorized_bundle(&bundle);

    ShieldFromIdentityTransition::try_from_bundle_with_identity_signer(
        identity,
        sb.value_balance.unsigned_abs(),
        sb.actions,
        sb.anchor,
        sb.proof,
        sb.binding_signature,
        user_fee_increase,
        signer,
        signing_transfer_key_to_use,
        nonce,
        platform_version,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address_funds::AddressWitness;
    use crate::identity::accessors::IdentityGettersV0;
    use crate::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use crate::identity::{KeyType, Purpose, SecurityLevel};
    use crate::shielded::builder::test_helpers::{test_orchard_address, TestProver};
    use crate::state_transition::shield_from_identity_transition::accessors::ShieldFromIdentityTransitionAccessorsV0;
    use crate::state_transition::StateTransitionIdentitySigned;
    use platform_value::BinaryData;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[derive(Debug)]
    struct DummySigner;

    #[async_trait::async_trait]
    impl Signer<IdentityPublicKey> for DummySigner {
        async fn sign(
            &self,
            _key: &IdentityPublicKey,
            _data: &[u8],
        ) -> Result<BinaryData, ProtocolError> {
            Ok(BinaryData::new(vec![0u8; 65]))
        }

        async fn sign_create_witness(
            &self,
            _key: &IdentityPublicKey,
            _data: &[u8],
        ) -> Result<AddressWitness, ProtocolError> {
            Err(ProtocolError::ShieldedBuildError(
                "identity signer never creates address witnesses".to_string(),
            ))
        }

        fn can_sign_with(&self, _key: &IdentityPublicKey) -> bool {
            true
        }
    }

    fn identity_with_transfer_key() -> Identity {
        let platform_version = PlatformVersion::latest();
        let mut identity =
            Identity::random_identity(0, Some(1), platform_version).expect("identity");
        let mut rng = StdRng::seed_from_u64(42);
        let (key, _) = IdentityPublicKey::random_key_with_known_attributes(
            0,
            &mut rng,
            Purpose::TRANSFER,
            SecurityLevel::CRITICAL,
            KeyType::ECDSA_SECP256K1,
            None,
            platform_version,
        )
        .expect("transfer key");
        identity.add_public_key(key);
        identity
    }

    #[tokio::test]
    async fn test_build_shield_from_identity_rejects_zero_amount() {
        let identity = identity_with_transfer_key();
        let result = build_shield_from_identity_transition(
            &identity,
            &test_orchard_address(),
            0,
            1,
            &DummySigner,
            None,
            0,
            &TestProver,
            [0u8; 36],
            None,
            PlatformVersion::latest(),
        )
        .await;
        let err = result
            .expect_err("zero amount must be rejected")
            .to_string();
        assert!(err.contains("greater than zero"), "unexpected error: {err}");
    }

    #[tokio::test]
    async fn test_build_shield_from_identity_transition_valid() {
        let identity = identity_with_transfer_key();
        let result = build_shield_from_identity_transition(
            &identity,
            &test_orchard_address(),
            50_000,
            7,
            &DummySigner,
            None,
            3,
            &TestProver,
            [0u8; 36],
            None,
            PlatformVersion::latest(),
        )
        .await;
        let st = result.expect("expected a built transition");
        let StateTransition::ShieldFromIdentity(t) = st else {
            panic!("expected ShieldFromIdentity, got {st:?}");
        };
        assert_eq!(t.identity_id(), identity.id());
        assert_eq!(t.amount(), 50_000);
        assert_eq!(t.nonce(), 7);
        assert!(!t.actions().is_empty());
        let key = identity
            .get_first_public_key_matching(
                Purpose::TRANSFER,
                SecurityLevel::full_range().into(),
                KeyType::all_key_types().into(),
                true,
            )
            .expect("transfer key");
        assert_eq!(t.signature_public_key_id(), key.id());
    }
}
