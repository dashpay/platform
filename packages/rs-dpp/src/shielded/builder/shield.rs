use std::collections::{BTreeMap, BTreeSet};

use crate::address_funds::AddressFundsFeeStrategy;
use crate::address_funds::{OrchardAddress, PlatformAddress};
use crate::fee::Credits;
use crate::identity::signer::Signer;
use crate::prelude::{AddressNonce, UserFeeIncrease};
use crate::shielded::shield_extra_sighash_data;
use crate::state_transition::shield_transition::methods::ShieldTransitionMethodsV0;
use crate::state_transition::shield_transition::ShieldTransition;
use crate::state_transition::StateTransition;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

use super::{
    build_output_only_bundle, serialize_authorized_bundle, OrchardProver, SerializedBundle,
};

/// A Shield bundle that has been proved and bound to its funding addresses, but not yet wrapped
/// in a signed transition. Produced by [`prove_shield_bundle`], consumed by
/// [`build_shield_transition_from_proved_bundle`].
pub struct ProvedShieldBundle {
    bundle: SerializedBundle,
    /// The funding binding the bundle's sighash committed to (see
    /// [`shield_extra_sighash_data`]); empty at protocol versions that bind nothing.
    extra_sighash_data: Vec<u8>,
}

impl ProvedShieldBundle {
    /// Whether this bundle binds exactly what consensus will re-derive for `inputs` at
    /// `platform_version` — the same funding addresses under the same binding rules. A bundle
    /// proved at a protocol version with a different binding (e.g. across the activation of
    /// `credit_pool_bundle_binding`) must be proved again.
    pub fn is_bound_for(
        &self,
        inputs: &BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        platform_version: &PlatformVersion,
    ) -> Result<bool, ProtocolError> {
        Ok(shield_extra_sighash_data(inputs, platform_version)? == self.extra_sighash_data)
    }
}

/// Proves the output-only Orchard bundle of a Shield transition.
///
/// This is the CPU-heavy half of [`build_shield_transition`] (Halo 2 proof generation, seconds of
/// work) and does no I/O and no signing, so it can run on a blocking thread while the caller
/// fetches the input nonces.
///
/// The bundle's binding signature commits to the funding addresses — the SET of input addresses,
/// via [`shield_extra_sighash_data`] — and never to their nonces or contributed amounts. The
/// nonces and amounts are committed later by the input witnesses, which sign the whole
/// transition in [`build_shield_transition_from_proved_bundle`].
pub fn prove_shield_bundle<P: OrchardProver>(
    recipient: &OrchardAddress,
    shield_amount: u64,
    input_addresses: BTreeSet<PlatformAddress>,
    prover: &P,
    memo: [u8; 36],
    sender_ovk: Option<grovedb_commitment_tree::OutgoingViewingKey>,
    platform_version: &PlatformVersion,
) -> Result<ProvedShieldBundle, ProtocolError> {
    // `shield_extra_sighash_data` digests only the map's keys (in key order); the nonce and
    // amount values are deliberately not part of the binding, so placeholders bind exactly the
    // bytes the real inputs map binds (pinned by
    // `test_proved_bundle_binding_ignores_nonces_and_amounts`).
    let binding_inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)> = input_addresses
        .iter()
        .map(|address| (*address, (0, 0)))
        .collect();
    // Bound to the funding addresses, so nobody else can re-wrap the proved bundle; empty at
    // protocol versions whose verifier predates the binding.
    let extra_sighash_data = shield_extra_sighash_data(&binding_inputs, platform_version)?;

    // Shield (Type 15) never pads with anonymity-set fillers — only the
    // Type 18 ShieldFromAssetLock pool-seeding path does (`dummy_outputs`).
    let bundle = build_output_only_bundle(
        recipient,
        shield_amount,
        memo,
        sender_ovk,
        0,
        &extra_sighash_data,
        prover,
    )?;
    Ok(ProvedShieldBundle {
        bundle: serialize_authorized_bundle(&bundle),
        extra_sighash_data,
    })
}

/// Wraps a [`ProvedShieldBundle`] into a signed Shield state transition: attaches the inputs with
/// their nonces and amounts and signs every input witness over the transition's signable bytes.
///
/// The binding consensus will re-derive from `inputs` at `platform_version` must equal the one
/// the bundle was proved with (see [`ProvedShieldBundle::is_bound_for`]); otherwise this errors
/// instead of producing a transition consensus would reject.
pub async fn build_shield_transition_from_proved_bundle<S: Signer<PlatformAddress>>(
    proved: ProvedShieldBundle,
    inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    fee_strategy: AddressFundsFeeStrategy,
    signer: &S,
    user_fee_increase: UserFeeIncrease,
    platform_version: &PlatformVersion,
) -> Result<StateTransition, ProtocolError> {
    if fee_strategy.is_empty() {
        return Err(ProtocolError::ShieldedBuildError(
            "fee_strategy must have at least one step".to_string(),
        ));
    }
    if !proved.is_bound_for(&inputs, platform_version)? {
        return Err(ProtocolError::ShieldedBuildError(
            "shield inputs do not match the binding the bundle was proved with (different \
             funding addresses or protocol version)"
                .to_string(),
        ));
    }
    let sb = proved.bundle;

    ShieldTransition::try_from_bundle_with_signer(
        inputs,
        sb.actions,
        sb.value_balance.unsigned_abs(),
        sb.anchor,
        sb.proof,
        sb.binding_signature,
        fee_strategy,
        signer,
        user_fee_increase,
        platform_version,
    )
    .await
}

/// Builds a Shield state transition (transparent platform addresses -> shielded pool).
///
/// Constructs an output-only Orchard bundle (no spends), proves it, signs the
/// transparent input witnesses, and returns a ready-to-broadcast `StateTransition`.
/// Equivalent to [`prove_shield_bundle`] followed by
/// [`build_shield_transition_from_proved_bundle`]; use those two directly to prove while the
/// input nonces are still being fetched.
///
/// # Parameters
/// - `recipient` - Orchard address to receive the shielded note
/// - `shield_amount` - Amount of credits to shield
/// - `inputs` - Platform address inputs with their nonces and balances
/// - `fee_strategy` - How to deduct fees from the transparent inputs
/// - `signer` - Signs each input address witness (ECDSA)
/// - `user_fee_increase` - Fee multiplier (0 = 100% base fee)
/// - `prover` - Orchard prover (holds the Halo 2 proving key; cache with `OnceLock` — ~30s to build)
/// - `memo` - 36-byte structured memo for the recipient (4-byte type tag + 32-byte payload)
/// - `sender_ovk` - The sender's outgoing viewing key (External scope). With `Some`, the
///   recipient output's `out_ciphertext` is encrypted under it so the sender can later
///   recover the sent note (recipient, value, memo) from chain data via OVK recovery —
///   the Zcash outgoing-transaction-history convention. With `None`, a random outgoing
///   cipher key is used and the sent note is unrecoverable by anyone.
/// - `platform_version` - Protocol version
#[allow(clippy::too_many_arguments)]
pub async fn build_shield_transition<S: Signer<PlatformAddress>, P: OrchardProver>(
    recipient: &OrchardAddress,
    shield_amount: u64,
    inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    fee_strategy: AddressFundsFeeStrategy,
    signer: &S,
    user_fee_increase: UserFeeIncrease,
    prover: &P,
    memo: [u8; 36],
    sender_ovk: Option<grovedb_commitment_tree::OutgoingViewingKey>,
    platform_version: &PlatformVersion,
) -> Result<StateTransition, ProtocolError> {
    // Checked before proving so a malformed request fails without paying for the proof.
    if fee_strategy.is_empty() {
        return Err(ProtocolError::ShieldedBuildError(
            "fee_strategy must have at least one step".to_string(),
        ));
    }

    let proved = prove_shield_bundle(
        recipient,
        shield_amount,
        inputs.keys().copied().collect(),
        prover,
        memo,
        sender_ovk,
        platform_version,
    )?;

    build_shield_transition_from_proved_bundle(
        proved,
        inputs,
        fee_strategy,
        signer,
        user_fee_increase,
        platform_version,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address_funds::AddressFundsFeeStrategyStep;
    use crate::address_funds::AddressWitness;
    use crate::shielded::builder::test_helpers::{test_orchard_address, TestProver};
    use platform_value::BinaryData;

    /// A dummy signer that produces a fake 65-byte signature.
    /// Only used to test the builder pipeline — the signature is not validated here.
    #[derive(Debug)]
    struct DummySigner;

    #[async_trait::async_trait]
    impl Signer<PlatformAddress> for DummySigner {
        async fn sign(
            &self,
            _key: &PlatformAddress,
            _data: &[u8],
        ) -> Result<BinaryData, ProtocolError> {
            Ok(BinaryData::new(vec![0u8; 65]))
        }

        async fn sign_create_witness(
            &self,
            _key: &PlatformAddress,
            _data: &[u8],
        ) -> Result<AddressWitness, ProtocolError> {
            Ok(AddressWitness::P2pkh {
                signature: BinaryData::new(vec![0u8; 65]),
            })
        }

        fn can_sign_with(&self, _key: &PlatformAddress) -> bool {
            true
        }
    }

    #[tokio::test]
    async fn test_build_shield_empty_fee_strategy() {
        let recipient = test_orchard_address();
        let platform_version = PlatformVersion::latest();
        let result = build_shield_transition(
            &recipient,
            1000,
            BTreeMap::new(),
            vec![], // empty fee strategy
            &DummySigner,
            0,
            &TestProver,
            [0u8; 36],
            None,
            platform_version,
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("fee_strategy must have at least one step"),
            "unexpected error: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_build_shield_transition_valid() {
        let recipient = test_orchard_address();
        let platform_version = PlatformVersion::latest();
        // Create a P2PKH address as input
        let input_address = PlatformAddress::P2pkh([1u8; 20]);
        let mut inputs = BTreeMap::new();
        inputs.insert(input_address, (0u32, 100_000u64));

        let fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];

        let result = build_shield_transition(
            &recipient,
            50_000,
            inputs,
            fee_strategy,
            &DummySigner,
            0,
            &TestProver,
            [0u8; 36],
            None,
            platform_version,
        )
        .await;

        assert!(result.is_ok(), "expected Ok, got: {:?}", result.err());
        match result.unwrap() {
            StateTransition::Shield(_) => {} // correct variant
            other => panic!("expected Shield variant, got {:?}", other),
        }
    }

    // ------------------------------------------------------------
    // Extra coverage: error/edge paths not exercised above.
    // ------------------------------------------------------------

    #[tokio::test]
    async fn test_build_shield_multiple_inputs_all_plumbed() {
        // Multiple input addresses should each produce their own witness
        // signature and flow through the downstream Shield transition.
        let recipient = test_orchard_address();
        let platform_version = PlatformVersion::latest();

        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([1u8; 20]), (0u32, 100_000u64));
        inputs.insert(PlatformAddress::P2pkh([2u8; 20]), (0u32, 200_000u64));
        inputs.insert(PlatformAddress::P2pkh([3u8; 20]), (0u32, 300_000u64));

        let fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];

        let result = build_shield_transition(
            &recipient,
            50_000,
            inputs,
            fee_strategy,
            &DummySigner,
            0,
            &TestProver,
            [0u8; 36],
            None,
            platform_version,
        )
        .await;
        assert!(
            result.is_ok(),
            "multi-input shield should succeed: {:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn test_build_shield_user_fee_increase_non_zero_succeeds() {
        // The user_fee_increase param just flows through as metadata.
        // A non-zero value should not fail the bundle build.
        let recipient = test_orchard_address();
        let platform_version = PlatformVersion::latest();
        let input_address = PlatformAddress::P2pkh([5u8; 20]);
        let mut inputs = BTreeMap::new();
        inputs.insert(input_address, (0u32, 500_000u64));

        let fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];

        let result = build_shield_transition(
            &recipient,
            100_000,
            inputs,
            fee_strategy,
            &DummySigner,
            42, // non-zero fee increase
            &TestProver,
            [9u8; 36],
            None,
            platform_version,
        )
        .await;
        assert!(
            result.is_ok(),
            "non-zero user_fee_increase should succeed: {:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn test_build_shield_memo_is_fully_plumbed() {
        // Any 36-byte memo should be accepted — this test is a guard
        // against accidental panics/regressions in memo handling.
        let recipient = test_orchard_address();
        let platform_version = PlatformVersion::latest();
        let input_address = PlatformAddress::P2pkh([9u8; 20]);
        let mut inputs = BTreeMap::new();
        inputs.insert(input_address, (5u32, 200_000u64));

        let fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];
        let mut memo = [0u8; 36];
        for (i, b) in memo.iter_mut().enumerate() {
            *b = i as u8;
        }

        let result = build_shield_transition(
            &recipient,
            80_000,
            inputs,
            fee_strategy,
            &DummySigner,
            0,
            &TestProver,
            memo,
            None,
            platform_version,
        )
        .await;
        assert!(
            result.is_ok(),
            "varied memo should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_proved_bundle_binding_ignores_nonces_and_amounts() {
        // `prove_shield_bundle` binds placeholder nonces/amounts; that is only sound because the
        // Shield binding digests the input address set alone. Pin it at every version that binds.
        let mut real = BTreeMap::new();
        real.insert(PlatformAddress::P2pkh([1u8; 20]), (17u32, 100_000u64));
        real.insert(PlatformAddress::P2sh([2u8; 20]), (u32::MAX, 1u64));
        let placeholders: BTreeMap<PlatformAddress, (AddressNonce, Credits)> =
            real.keys().map(|address| (*address, (0, 0))).collect();
        for platform_version in [PlatformVersion::first(), PlatformVersion::latest()] {
            assert_eq!(
                shield_extra_sighash_data(&real, platform_version).unwrap(),
                shield_extra_sighash_data(&placeholders, platform_version).unwrap(),
            );
        }
        // And at the latest version the binding is not vacuous: it depends on the address set.
        let latest = PlatformVersion::latest();
        assert!(!shield_extra_sighash_data(&real, latest).unwrap().is_empty());
        let mut other = real.clone();
        other.insert(PlatformAddress::P2pkh([3u8; 20]), (0, 0));
        assert_ne!(
            shield_extra_sighash_data(&real, latest).unwrap(),
            shield_extra_sighash_data(&other, latest).unwrap(),
        );
    }

    #[tokio::test]
    async fn test_proved_bundle_split_path_and_binding_mismatch() {
        let recipient = test_orchard_address();
        let latest = PlatformVersion::latest();
        let a = PlatformAddress::P2pkh([1u8; 20]);
        let b = PlatformAddress::P2pkh([2u8; 20]);
        let fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];
        let inputs_for = |addrs: &[PlatformAddress]| -> BTreeMap<_, _> {
            addrs
                .iter()
                .map(|addr| (*addr, (7u32, 100_000u64)))
                .collect()
        };
        let prove = |platform_version| {
            prove_shield_bundle(
                &recipient,
                50_000,
                [a].into_iter().collect(),
                &TestProver,
                [0u8; 36],
                None,
                platform_version,
            )
            .expect("prove")
        };

        // Proved for {a} at the latest version: bound for exactly those inputs at that version.
        let proved = prove(latest);
        assert!(proved.is_bound_for(&inputs_for(&[a]), latest).unwrap());
        assert!(!proved.is_bound_for(&inputs_for(&[a, b]), latest).unwrap());
        assert!(!proved.is_bound_for(&inputs_for(&[b]), latest).unwrap());
        // A version whose binding rules differ (the first version binds nothing) must re-prove.
        assert!(!proved
            .is_bound_for(&inputs_for(&[a]), PlatformVersion::first())
            .unwrap());

        // Wrapping it around other inputs is refused rather than built.
        let err = build_shield_transition_from_proved_bundle(
            proved,
            inputs_for(&[a, b]),
            fee_strategy.clone(),
            &DummySigner,
            0,
            latest,
        )
        .await
        .expect_err("mismatched inputs must be rejected");
        assert!(err.to_string().contains("do not match"), "{err}");

        // Prove first, attach nonces afterwards (the wallet's overlap path).
        let st = build_shield_transition_from_proved_bundle(
            prove(latest),
            inputs_for(&[a]),
            fee_strategy,
            &DummySigner,
            0,
            latest,
        )
        .await
        .expect("assemble");
        assert!(matches!(st, StateTransition::Shield(_)));
    }
}
