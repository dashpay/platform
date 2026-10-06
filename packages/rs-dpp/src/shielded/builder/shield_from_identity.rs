use crate::address_funds::OrchardAddress;
use crate::identity::accessors::IdentityGettersV0;
use crate::identity::signer::Signer;
use crate::identity::{Identity, IdentityPublicKey};
use crate::prelude::{Identifier, IdentityNonce, UserFeeIncrease};
use crate::shielded::shield_from_identity_extra_sighash_data;
use crate::state_transition::shield_from_identity_transition::methods::ShieldFromIdentityTransitionMethodsV0;
use crate::state_transition::shield_from_identity_transition::ShieldFromIdentityTransition;
use crate::state_transition::StateTransition;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

use super::{
    build_output_only_bundle, serialize_authorized_bundle, OrchardProver, SerializedBundle,
};

/// A `ShieldFromIdentity` bundle that has been proved and bound to its funding identity, but not
/// yet wrapped in an identity-signed transition. Produced by
/// [`prove_shield_from_identity_bundle`], consumed by
/// [`build_shield_from_identity_transition_from_proved_bundle`].
pub struct ProvedShieldFromIdentityBundle {
    bundle: SerializedBundle,
    /// The funding binding the bundle's sighash committed to (see
    /// [`shield_from_identity_extra_sighash_data`]); empty at protocol versions that bind nothing.
    extra_sighash_data: Vec<u8>,
}

impl ProvedShieldFromIdentityBundle {
    /// Whether this bundle binds exactly what consensus will re-derive for `identity_id` at
    /// `platform_version`. A bundle proved at a protocol version with a different binding (e.g.
    /// across the activation of `credit_pool_bundle_binding`) must be proved again.
    pub fn is_bound_for(
        &self,
        identity_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<bool, ProtocolError> {
        Ok(
            shield_from_identity_extra_sighash_data(&identity_id.to_buffer(), platform_version)?
                == self.extra_sighash_data,
        )
    }
}

/// Proves the outputs-only Orchard bundle of a `ShieldFromIdentity` transition.
///
/// This is the CPU-heavy half of [`build_shield_from_identity_transition`] and does no I/O and no
/// signing, so it can run on a blocking thread while the caller fetches the identity nonce. The
/// bundle's binding signature commits to the funding identity id (see
/// [`shield_from_identity_extra_sighash_data`]), never to the nonce; the nonce is committed by
/// the identity signature over the whole transition in
/// [`build_shield_from_identity_transition_from_proved_bundle`].
pub fn prove_shield_from_identity_bundle<P: OrchardProver>(
    identity_id: Identifier,
    recipient: &OrchardAddress,
    shield_amount: u64,
    prover: &P,
    memo: [u8; 36],
    sender_ovk: Option<grovedb_commitment_tree::OutgoingViewingKey>,
    platform_version: &PlatformVersion,
) -> Result<ProvedShieldFromIdentityBundle, ProtocolError> {
    if shield_amount == 0 {
        return Err(ProtocolError::ShieldedBuildError(
            "shield amount must be greater than zero".to_string(),
        ));
    }

    // Bound to the funding identity, so no other identity can sign the proved bundle into a
    // transition of its own.
    let extra_sighash_data =
        shield_from_identity_extra_sighash_data(&identity_id.to_buffer(), platform_version)?;
    let bundle = build_output_only_bundle(
        recipient,
        shield_amount,
        memo,
        sender_ovk,
        0,
        &extra_sighash_data,
        prover,
    )?;
    Ok(ProvedShieldFromIdentityBundle {
        bundle: serialize_authorized_bundle(&bundle),
        extra_sighash_data,
    })
}

/// Wraps a [`ProvedShieldFromIdentityBundle`] into a `ShieldFromIdentity` transition signed by
/// `identity`'s TRANSFER key with `nonce`.
///
/// The binding consensus will re-derive for `identity` at `platform_version` must equal the one
/// the bundle was proved with (see [`ProvedShieldFromIdentityBundle::is_bound_for`]); otherwise
/// this errors instead of producing a transition consensus would reject.
pub async fn build_shield_from_identity_transition_from_proved_bundle<
    S: Signer<IdentityPublicKey>,
>(
    proved: ProvedShieldFromIdentityBundle,
    identity: &Identity,
    nonce: IdentityNonce,
    signer: &S,
    signing_transfer_key_to_use: Option<&IdentityPublicKey>,
    user_fee_increase: UserFeeIncrease,
    platform_version: &PlatformVersion,
) -> Result<StateTransition, ProtocolError> {
    if !proved.is_bound_for(identity.id(), platform_version)? {
        return Err(ProtocolError::ShieldedBuildError(
            "identity does not match the binding the shield bundle was proved with (different \
             identity or protocol version)"
                .to_string(),
        ));
    }
    let sb = proved.bundle;

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

/// Build a `ShieldFromIdentity` transition: prove an outputs-only Orchard bundle
/// paying `shield_amount` to `recipient`, then identity-sign it with a TRANSFER key.
///
/// The identity nonce must already be the next nonce for `identity` (the SDK
/// fetches it); `amount + fee` is debited from the identity balance at execution.
/// Equivalent to [`prove_shield_from_identity_bundle`] followed by
/// [`build_shield_from_identity_transition_from_proved_bundle`]; use those two directly to prove
/// while the nonce is still being fetched.
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
    let proved = prove_shield_from_identity_bundle(
        identity.id(),
        recipient,
        shield_amount,
        prover,
        memo,
        sender_ovk,
        platform_version,
    )?;

    build_shield_from_identity_transition_from_proved_bundle(
        proved,
        identity,
        nonce,
        signer,
        signing_transfer_key_to_use,
        user_fee_increase,
        platform_version,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address_funds::AddressWitness;
    use crate::identity::accessors::{IdentityGettersV0, IdentitySettersV0};
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

    #[tokio::test]
    async fn test_proved_bundle_binding_rejects_other_identity_or_version() {
        let identity = identity_with_transfer_key();
        let mut other = identity_with_transfer_key();
        other.set_id(Identifier::from([9u8; 32]));
        assert_ne!(identity.id(), other.id());

        let proved = prove_shield_from_identity_bundle(
            identity.id(),
            &test_orchard_address(),
            50_000,
            &TestProver,
            [0u8; 36],
            None,
            PlatformVersion::latest(),
        )
        .expect("prove");
        assert!(proved
            .is_bound_for(identity.id(), PlatformVersion::latest())
            .unwrap());
        assert!(!proved
            .is_bound_for(other.id(), PlatformVersion::latest())
            .unwrap());
        assert!(!proved
            .is_bound_for(identity.id(), PlatformVersion::first())
            .unwrap());
        let err = build_shield_from_identity_transition_from_proved_bundle(
            proved,
            &other,
            7,
            &DummySigner,
            None,
            0,
            PlatformVersion::latest(),
        )
        .await
        .expect_err("a bundle proved for one identity must not be signed by another");
        assert!(err.to_string().contains("does not match"), "{err}");
    }
}
