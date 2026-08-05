use grovedb_commitment_tree::{
    Anchor, Builder, BundleType, DashMemo, FullViewingKey, NoteValue, PaymentAddress, Scope,
    SpendAuthorizingKey,
};

use crate::address_funds::OrchardAddress;
use crate::fee::Credits;
use crate::shielded::compute_minimum_shielded_fee;
use crate::state_transition::shielded_transfer_transition::methods::ShieldedTransferTransitionMethodsV0;
use crate::state_transition::shielded_transfer_transition::ShieldedTransferTransition;
use crate::state_transition::StateTransition;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

use super::{
    prove_and_sign_bundle, serialize_authorized_bundle, shielded_bundle_action_count,
    OrchardProver, SpendableNote,
};

/// Builds a ShieldedTransfer state transition (shielded pool -> shielded pool).
///
/// Spends existing notes and creates a new note for the recipient. The shielded
/// fee is deducted from the spent notes. Any remaining change is returned to
/// the `change_address`.
///
/// Both real outputs are encrypted with the sender's External-scope OVK
/// (derived from `fvk`), so the sender can recover its own send history
/// (recipient, value, memo) from chain data via OVK recovery.
///
/// # Parameters
/// - `spends` - Notes to spend with their Merkle paths
/// - `recipient` - Orchard address to receive the transferred note
/// - `transfer_amount` - Amount to transfer to the recipient
/// - `change_address` - Orchard address for change output (if any)
/// - `fvk` - Full viewing key for spend authorization
/// - `ask` - Spend authorizing key for RedPallas signatures
/// - `anchor` - Sinsemilla root of the note commitment tree (Orchard Anchor)
/// - `prover` - Orchard prover (holds the Halo 2 proving key)
/// - `memo` - 36-byte structured memo for the recipient (4-byte type tag + 32-byte payload)
/// - `platform_version` - Protocol version
///
/// The fee is not a parameter: a shielded transfer's `value_balance` IS the fee and consensus
/// pins it to exactly `compute_minimum_shielded_fee`, so there is nothing for the caller to
/// choose. Returns the built transition together with the fee (in credits) that was applied.
#[allow(clippy::too_many_arguments)]
pub fn build_shielded_transfer_transition<P: OrchardProver>(
    spends: Vec<SpendableNote>,
    recipient: &OrchardAddress,
    transfer_amount: u64,
    change_address: &OrchardAddress,
    fvk: &FullViewingKey,
    ask: &SpendAuthorizingKey,
    anchor: Anchor,
    prover: &P,
    memo: [u8; 36],
    platform_version: &PlatformVersion,
) -> Result<(StateTransition, Credits), ProtocolError> {
    let total_spent: u64 = spends.iter().map(|s| s.note.value().inner()).sum();

    // Action count = max(spends, outputs), padded to Orchard's 2-action minimum. This bundle
    // publishes at most two outputs (recipient + change), and the no-change case collapses to the
    // same number because of that padding: `max(n, 1).max(2) == max(n, 2).max(2)`. So sizing the
    // fee for the with-change shape is exact in BOTH branches — see the
    // `single_output_transfer_fee_matches_on_wire_action_count` test, which pins the carved fee
    // against the bundle's real `actions.len()`.
    const MAX_OUTPUTS: usize = 2; // recipient + change
    let num_actions = shielded_bundle_action_count(spends.len(), MAX_OUTPUTS)?;
    // The fee is fixed at the minimum: a transfer's `value_balance` IS the fee and consensus
    // pins it to exactly this amount (overpayment buys nothing and would leak a distinguishing
    // fee fingerprint that breaks shielded uniformity).
    let fee = compute_minimum_shielded_fee(num_actions, platform_version)?;

    let required = transfer_amount.checked_add(fee).ok_or_else(|| {
        ProtocolError::ShieldedBuildError("fee + transfer_amount overflows u64".to_string())
    })?;
    if required > total_spent {
        return Err(ProtocolError::ShieldedBuildError(format!(
            "transfer amount {} + fee {} = {} exceeds total spendable value {}",
            transfer_amount, fee, required, total_spent
        )));
    }

    let change_amount = total_spent - required;

    let recipient_payment = PaymentAddress::from(recipient);

    let mut builder = Builder::<DashMemo>::new(BundleType::DEFAULT, anchor);

    for spend in spends {
        builder
            .add_spend(fvk.clone(), spend.note, spend.merkle_path)
            .map_err(|e| {
                ProtocolError::ShieldedBuildError(format!("failed to add spend: {:?}", e))
            })?;
    }

    // Both real outputs carry an `out_ciphertext` encrypted under the sender's
    // External-scope OVK (the Zcash outgoing-transaction-history convention),
    // so the sender can recover its own send history — recipient, value, memo —
    // from chain data alone. Without it, the outgoing cipher key is random and
    // the sent note is unrecoverable by anyone, including the sender.
    let sender_ovk = fvk.to_ovk(Scope::External);

    // Primary output to recipient
    builder
        .add_output(
            Some(sender_ovk.clone()),
            recipient_payment,
            NoteValue::from_raw(transfer_amount),
            memo,
        )
        .map_err(|e| ProtocolError::ShieldedBuildError(format!("failed to add output: {:?}", e)))?;

    // Change output (if any)
    if change_amount > 0 {
        let change_payment = PaymentAddress::from(change_address);
        builder
            .add_output(
                Some(sender_ovk),
                change_payment,
                NoteValue::from_raw(change_amount),
                [0u8; 36],
            )
            .map_err(|e| {
                ProtocolError::ShieldedBuildError(format!("failed to add change output: {:?}", e))
            })?;
    }

    // ShieldedTransfer has no extra_data in sighash
    let bundle = prove_and_sign_bundle(builder, prover, std::slice::from_ref(ask), &[])?;
    let sb = serialize_authorized_bundle(&bundle);

    // value_balance = fee (the amount leaving the shielded pool as fee)
    let state_transition = ShieldedTransferTransition::try_from_bundle(
        sb.actions,
        sb.value_balance as u64,
        sb.anchor,
        sb.proof,
        sb.binding_signature,
        platform_version,
    )?;
    Ok((state_transition, fee))
}

/// One recipient output of a multi-output [`build_shielded_transfer_transition_multi`].
///
/// Each entry becomes its own Orchard output — its own note, with its own randomness and so its
/// own (deterministic) nullifier when later spent. Two entries may name the SAME `recipient`
/// address: Orchard derives independent notes regardless, which is exactly how a single transfer
/// funds one address with several notes.
#[derive(Clone, Copy, Debug)]
pub struct ShieldedTransferOutput {
    /// Orchard address receiving this note.
    pub recipient: OrchardAddress,
    /// Value of this note, in credits.
    pub amount: u64,
    /// 36-byte structured memo (4-byte type tag + 32-byte payload) for this note.
    pub memo: [u8; 36],
}

/// Builds a ShieldedTransfer state transition with **several** recipient outputs in one atomic
/// bundle (shielded pool -> shielded pool).
///
/// This is the multi-output sibling of [`build_shielded_transfer_transition`]. It exists because
/// some flows must land more than one note in a single transition — most importantly, funding an
/// address with two sub-target notes so that a later spend of that address is forced to spend
/// BOTH of them.
///
/// # Why more than one output changes the fee
///
/// An Orchard action is a joined spend/output slot, so the on-wire action count is
/// `max(num_spends, num_outputs)` padded to `MIN_ACTIONS = 2`. A `ShieldedTransfer`'s
/// `value_balance` IS its fee and consensus pins it to `compute_minimum_shielded_fee(actions.len())`
/// **exactly**. With three or more outputs the output side sets the action count, so the fee MUST
/// be sized from it — see [`shielded_bundle_action_count`].
///
/// # Deterministic shape
///
/// This builder ALWAYS emits a change output and therefore requires the spent value to STRICTLY
/// exceed `sum(amounts) + fee`. That makes the output count — and hence the action count and the
/// fee — a pure function of the inputs (`max(spends, recipients + 1, 2)`), with no circular
/// dependency between "is there change?" and "what is the fee?". A caller that spends *exactly*
/// `sum(amounts) + fee` is rejected rather than silently re-shaped into a different action count;
/// note selection always reserves against the same `recipients + 1` floor, so the reserved fee and
/// the carved fee cannot diverge.
///
/// All recipient outputs and the change output are encrypted with the sender's External-scope OVK,
/// so the sender can recover its own send history from chain data (see
/// [`build_shielded_transfer_transition`]).
///
/// # Parameters
/// - `spends` - Notes to spend with their Merkle paths
/// - `outputs` - Recipient outputs; must be non-empty
/// - `change_address` - Orchard address for the (always present) change output
/// - `fvk` / `ask` - Full viewing key and spend authorizing key
/// - `anchor` - Sinsemilla root of the note commitment tree
/// - `prover` - Orchard prover (holds the Halo 2 proving key)
/// - `platform_version` - Protocol version
///
/// Returns the built transition together with the fee (in credits) that was applied.
#[allow(clippy::too_many_arguments)]
pub fn build_shielded_transfer_transition_multi<P: OrchardProver>(
    spends: Vec<SpendableNote>,
    outputs: &[ShieldedTransferOutput],
    change_address: &OrchardAddress,
    fvk: &FullViewingKey,
    ask: &SpendAuthorizingKey,
    anchor: Anchor,
    prover: &P,
    platform_version: &PlatformVersion,
) -> Result<(StateTransition, Credits), ProtocolError> {
    if outputs.is_empty() {
        return Err(ProtocolError::ShieldedBuildError(
            "a multi-output shielded transfer needs at least one recipient output".to_string(),
        ));
    }

    // Checked: a crafted output set could otherwise wrap u64 in release builds.
    let transfer_total = outputs
        .iter()
        .try_fold(0u64, |acc, o| acc.checked_add(o.amount))
        .ok_or_else(|| {
            ProtocolError::ShieldedBuildError(
                "multi-output shielded transfer amounts overflow u64".to_string(),
            )
        })?;
    let total_spent = spends
        .iter()
        .try_fold(0u64, |acc, s| acc.checked_add(s.note.value().inner()))
        .ok_or_else(|| {
            ProtocolError::ShieldedBuildError(
                "multi-output shielded transfer total spent value overflows u64".to_string(),
            )
        })?;

    // A change output is always emitted (see the doc comment), so the output count — and with it
    // the action count and the fee — is fixed before any value arithmetic.
    let num_outputs = outputs.len().checked_add(1).ok_or_else(|| {
        ProtocolError::ShieldedBuildError("output count overflows usize".to_string())
    })?;
    let num_actions = shielded_bundle_action_count(spends.len(), num_outputs)?;
    let fee = compute_minimum_shielded_fee(num_actions, platform_version)?;

    let required = transfer_total.checked_add(fee).ok_or_else(|| {
        ProtocolError::ShieldedBuildError("fee + transfer amounts overflow u64".to_string())
    })?;
    // STRICTLY greater: the change output is unconditional, so it must carry a positive value.
    if required >= total_spent {
        return Err(ProtocolError::ShieldedBuildError(format!(
            "transfer amounts {} + fee {} = {} must be strictly less than the total spendable \
             value {} (a multi-output transfer always emits a change output)",
            transfer_total, fee, required, total_spent
        )));
    }
    let change_amount = total_spent - required;

    let sender_ovk = fvk.to_ovk(Scope::External);
    let mut builder = Builder::<DashMemo>::new(BundleType::DEFAULT, anchor);

    for spend in spends {
        builder
            .add_spend(fvk.clone(), spend.note, spend.merkle_path)
            .map_err(|e| {
                ProtocolError::ShieldedBuildError(format!("failed to add spend: {:?}", e))
            })?;
    }

    for output in outputs {
        builder
            .add_output(
                Some(sender_ovk.clone()),
                PaymentAddress::from(&output.recipient),
                NoteValue::from_raw(output.amount),
                output.memo,
            )
            .map_err(|e| {
                ProtocolError::ShieldedBuildError(format!("failed to add output: {:?}", e))
            })?;
    }

    builder
        .add_output(
            Some(sender_ovk),
            PaymentAddress::from(change_address),
            NoteValue::from_raw(change_amount),
            [0u8; 36],
        )
        .map_err(|e| {
            ProtocolError::ShieldedBuildError(format!("failed to add change output: {:?}", e))
        })?;

    // ShieldedTransfer has no extra_data in sighash
    let bundle = prove_and_sign_bundle(builder, prover, std::slice::from_ref(ask), &[])?;
    let sb = serialize_authorized_bundle(&bundle);

    // The fee was predicted before the bundle existed; consensus recomputes it from the ON-WIRE
    // action count and demands exact equality. Catch any divergence here (cheap) instead of as an
    // opaque rejection after the ~30 s proof.
    if sb.actions.len() != num_actions {
        return Err(ProtocolError::ShieldedBuildError(format!(
            "predicted {} actions but the bundle published {}; the carved fee would not match \
             the consensus minimum",
            num_actions,
            sb.actions.len()
        )));
    }

    let state_transition = ShieldedTransferTransition::try_from_bundle(
        sb.actions,
        sb.value_balance as u64,
        sb.anchor,
        sb.proof,
        sb.binding_signature,
        platform_version,
    )?;
    Ok((state_transition, fee))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shielded::builder::test_helpers::{
        test_orchard_address, test_spendable_note, TestProver,
    };
    use grovedb_commitment_tree::{
        ExtractedNoteCommitment, Hashable, MerkleHashOrchard, MerklePath, SpendingKey,
        NOTE_COMMITMENT_TREE_DEPTH,
    };

    /// Two distinct notes witnessed in one two-leaf commitment tree, plus the shared anchor.
    ///
    /// Each path's level-0 sibling is the other leaf and the upper siblings are shared, so both
    /// witnesses compute the SAME root — a consistent anchor the Orchard circuit accepts. (Same
    /// construction the identity-create builder's two-spend test uses.)
    fn two_spends_in_one_tree(
        value_a: u64,
        value_b: u64,
        fvk: &FullViewingKey,
    ) -> (Vec<SpendableNote>, Anchor, [[u8; 32]; 2]) {
        let note_a = test_spendable_note(value_a).note;
        let note_b = test_spendable_note(value_b).note;
        let cmx_a = ExtractedNoteCommitment::from(note_a.commitment());
        let cmx_b = ExtractedNoteCommitment::from(note_b.commitment());

        let mut auth_path_a = [MerkleHashOrchard::empty_leaf(); NOTE_COMMITMENT_TREE_DEPTH];
        auth_path_a[0] = MerkleHashOrchard::from_cmx(&cmx_b);
        let mut auth_path_b = [MerkleHashOrchard::empty_leaf(); NOTE_COMMITMENT_TREE_DEPTH];
        auth_path_b[0] = MerkleHashOrchard::from_cmx(&cmx_a);
        let path_a = MerklePath::from_parts(0, auth_path_a);
        let path_b = MerklePath::from_parts(1, auth_path_b);

        let anchor = path_a.root(cmx_a);
        assert_eq!(
            anchor.to_bytes(),
            path_b.root(cmx_b).to_bytes(),
            "both witnesses must compute the same anchor"
        );

        let nullifiers = [
            note_a.nullifier(fvk).to_bytes(),
            note_b.nullifier(fvk).to_bytes(),
        ];
        (
            vec![
                SpendableNote {
                    note: note_a,
                    merkle_path: path_a,
                },
                SpendableNote {
                    note: note_b,
                    merkle_path: path_b,
                },
            ],
            anchor,
            nullifiers,
        )
    }

    /// Destructure a built `ShieldedTransfer` into `(actions.len(), value_balance)` — the two
    /// fields consensus reads when it recomputes and pins the fee.
    fn on_wire_actions_and_value_balance(st: &StateTransition) -> (usize, u64) {
        match st {
            StateTransition::ShieldedTransfer(
                crate::state_transition::shielded_transfer_transition::ShieldedTransferTransition::V0(v0),
            ) => (v0.actions.len(), v0.value_balance),
            other => panic!("expected a ShieldedTransfer transition, got {other:?}"),
        }
    }

    /// THE regression pin for the multi-output fee predictor.
    ///
    /// A `ShieldedTransfer`'s `value_balance` IS its fee, and consensus pins it to
    /// `compute_minimum_shielded_fee(actions.len())` EXACTLY (see
    /// `validate_minimum_shielded_fee`: `amount_is_pure_fee` rejects both under- and
    /// over-payment). With two recipient outputs plus change the bundle publishes THREE actions,
    /// so a spends-only predictor (`spends.len().max(2)`) would carve `min_fee(2)` and be
    /// rejected on chain. This asserts the carved fee equals `min_fee(on-wire actions.len())`
    /// and, explicitly, that it is NOT the 2-action fee.
    ///
    /// It also pins the two-notes-to-one-address shape: both outputs name the SAME recipient and
    /// must still become two DISTINCT notes (distinct commitments), which is what makes a later
    /// spend of that address spend two real notes rather than one real note plus a random dummy.
    #[test]
    fn multi_output_transfer_fee_matches_on_wire_action_count() {
        let platform_version = PlatformVersion::latest();
        let sk = SpendingKey::from_bytes([42u8; 32]).expect("valid spending key");
        let fvk = FullViewingKey::from(&sk);
        let ask = SpendAuthorizingKey::from(&sk);
        let change_address = test_orchard_address();
        let recipient = test_orchard_address();

        let (spends, anchor, _) = two_spends_in_one_tree(6_000_000_000, 7_000_000_000, &fvk);

        // The two-note invite funding shape: D split into floor(D/2) + ceil(D/2), both to the
        // SAME one-time address, each strictly below D.
        const D: u64 = 3_000_000_000; // 0.03 DASH in credits
        let outputs = vec![
            ShieldedTransferOutput {
                recipient,
                amount: D / 2,
                memo: [0u8; 36],
            },
            ShieldedTransferOutput {
                recipient,
                amount: D - D / 2,
                memo: [0u8; 36],
            },
        ];

        let (st, fee) = build_shielded_transfer_transition_multi(
            spends,
            &outputs,
            &change_address,
            &fvk,
            &ask,
            anchor,
            &TestProver,
            platform_version,
        )
        .expect("a two-spend, three-output transfer must build");

        let (num_actions, value_balance) = on_wire_actions_and_value_balance(&st);
        assert_eq!(
            num_actions, 3,
            "2 spends + 3 outputs (2 recipients + change) must publish max(2,3) = 3 actions"
        );

        let expected_fee = compute_minimum_shielded_fee(num_actions, platform_version)
            .expect("fee computation should not overflow");
        assert_eq!(
            fee, expected_fee,
            "the carved fee must equal compute_minimum_shielded_fee(on-wire actions.len())"
        );
        assert_eq!(
            value_balance, expected_fee,
            "value_balance IS the fee and consensus pins it to the minimum for the on-wire \
             action count exactly"
        );

        // The bug this fixes: the old spends-only predictor would have carved the 2-action fee.
        let two_action_fee = compute_minimum_shielded_fee(2, platform_version)
            .expect("fee computation should not overflow");
        assert!(
            expected_fee > two_action_fee,
            "a 3-action bundle must cost strictly more than a 2-action one, otherwise this test \
             cannot detect the under-count"
        );
        assert_ne!(
            fee, two_action_fee,
            "a spends-only fee predictor would carve the 2-action fee and be rejected on chain"
        );

        // Two outputs to the SAME address are still two distinct notes.
        let commitments: Vec<[u8; 32]> = match &st {
            StateTransition::ShieldedTransfer(
                crate::state_transition::shielded_transfer_transition::ShieldedTransferTransition::V0(v0),
            ) => v0.actions.iter().map(|a| a.cmx).collect(),
            _ => unreachable!(),
        };
        let unique: std::collections::BTreeSet<[u8; 32]> = commitments.iter().copied().collect();
        assert_eq!(
            unique.len(),
            commitments.len(),
            "every published note commitment must be distinct, including the two notes paid to \
             the same address"
        );
    }

    /// The single-output builder's fee must ALSO equal `min_fee(on-wire actions.len())`. Its
    /// output count (recipient + change = 2) can never exceed Orchard's 2-action minimum, so
    /// routing it through `shielded_bundle_action_count` is numerically a no-op — this pins that
    /// claim against a real bundle so the shared helper cannot regress it.
    #[test]
    fn single_output_transfer_fee_matches_on_wire_action_count() {
        let platform_version = PlatformVersion::latest();
        let sk = SpendingKey::from_bytes([42u8; 32]).expect("valid spending key");
        let fvk = FullViewingKey::from(&sk);
        let ask = SpendAuthorizingKey::from(&sk);
        let change_address = test_orchard_address();
        let recipient = test_orchard_address();

        let (spends, anchor, _) = two_spends_in_one_tree(6_000_000_000, 7_000_000_000, &fvk);

        let (st, fee) = build_shielded_transfer_transition(
            spends,
            &recipient,
            3_000_000_000,
            &change_address,
            &fvk,
            &ask,
            anchor,
            &TestProver,
            [0u8; 36],
            platform_version,
        )
        .expect("a two-spend, two-output transfer must build");

        let (num_actions, value_balance) = on_wire_actions_and_value_balance(&st);
        assert_eq!(
            num_actions, 2,
            "2 spends + 2 outputs must publish 2 actions"
        );
        let expected_fee = compute_minimum_shielded_fee(num_actions, platform_version)
            .expect("fee computation should not overflow");
        assert_eq!(fee, expected_fee);
        assert_eq!(
            value_balance, expected_fee,
            "value_balance must equal the minimum fee for the on-wire action count exactly"
        );
    }

    #[test]
    fn multi_output_transfer_rejects_empty_output_set() {
        let platform_version = PlatformVersion::latest();
        let sk = SpendingKey::from_bytes([42u8; 32]).expect("valid sk");
        let fvk = FullViewingKey::from(&sk);
        let ask = SpendAuthorizingKey::from(&sk);
        let change_address = test_orchard_address();

        let err = build_shielded_transfer_transition_multi(
            vec![test_spendable_note(u64::MAX / 2)],
            &[],
            &change_address,
            &fvk,
            &ask,
            Anchor::empty_tree(),
            &TestProver,
            platform_version,
        )
        .expect_err("an empty output set must be rejected");
        assert!(
            err.to_string().contains("at least one recipient output"),
            "unexpected error: {err}"
        );
    }

    /// The multi-output builder always emits a change output, so it requires the spent value to
    /// STRICTLY exceed `sum(amounts) + fee`. Spending exactly that much is rejected rather than
    /// silently re-shaped into a different (and differently priced) action count.
    #[test]
    fn multi_output_transfer_requires_strictly_positive_change() {
        let platform_version = PlatformVersion::latest();
        let sk = SpendingKey::from_bytes([42u8; 32]).expect("valid sk");
        let fvk = FullViewingKey::from(&sk);
        let ask = SpendAuthorizingKey::from(&sk);
        let change_address = test_orchard_address();
        let recipient = test_orchard_address();

        // One spend + 3 outputs (2 recipients + change) → max(1, 3) = 3 actions.
        let fee = compute_minimum_shielded_fee(3, platform_version).expect("fee");
        let amount = 1_000_000u64;
        // Exactly `sum + fee` — the boundary that must be rejected.
        let note = test_spendable_note(2 * amount + fee);
        let outputs = vec![
            ShieldedTransferOutput {
                recipient,
                amount,
                memo: [0u8; 36],
            },
            ShieldedTransferOutput {
                recipient,
                amount,
                memo: [0u8; 36],
            },
        ];

        let err = build_shielded_transfer_transition_multi(
            vec![note],
            &outputs,
            &change_address,
            &fvk,
            &ask,
            Anchor::empty_tree(),
            &TestProver,
            platform_version,
        )
        .expect_err("spending exactly sum + fee must be rejected");
        assert!(
            err.to_string().contains("strictly less than"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn test_shielded_transfer_insufficient_funds() {
        let platform_version = PlatformVersion::latest();
        let recipient = test_orchard_address();
        let change_address = test_orchard_address();

        // Note with only 100 credits
        let note = test_spendable_note(100);
        let spends = vec![note];

        let sk = grovedb_commitment_tree::SpendingKey::from_bytes([42u8; 32])
            .expect("valid spending key bytes");
        let fvk = FullViewingKey::from(&sk);
        let ask = SpendAuthorizingKey::from(&sk);

        let result = build_shielded_transfer_transition(
            spends,
            &recipient,
            1_000_000,
            &change_address,
            &fvk,
            &ask,
            Anchor::empty_tree(),
            &TestProver,
            [0u8; 36],
            platform_version,
        );

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("exceeds total spendable value"),
            "unexpected error: {}",
            err
        );
    }

    // --------------------------------------------------------------
    // Extra coverage — error/overflow branches
    // --------------------------------------------------------------

    #[test]
    fn test_shielded_transfer_fee_plus_amount_overflow_errors() {
        // transfer_amount + fee overflows u64 → dedicated error branch.
        let platform_version = PlatformVersion::latest();
        let recipient = test_orchard_address();
        let change_address = test_orchard_address();

        let note = test_spendable_note(u64::MAX);
        let spends = vec![note];

        let sk = grovedb_commitment_tree::SpendingKey::from_bytes([42u8; 32])
            .expect("valid spending key bytes");
        let fvk = FullViewingKey::from(&sk);
        let ask = SpendAuthorizingKey::from(&sk);

        // transfer_amount = u64::MAX so amount + the (internally-computed) minimum fee
        // overflows u64, hitting the checked_add error branch.
        let result = build_shielded_transfer_transition(
            spends,
            &recipient,
            u64::MAX,
            &change_address,
            &fvk,
            &ask,
            Anchor::empty_tree(),
            &TestProver,
            [0u8; 36],
            platform_version,
        );

        assert!(result.is_err(), "overflow case should error");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("fee + transfer_amount overflows u64"),
            "expected checked_add overflow branch, got: {}",
            err
        );
    }

    #[test]
    fn test_shielded_transfer_zero_spends_total_is_zero_errors() {
        // Empty spends → total_spent = 0. Any non-zero transfer will fail
        // with "exceeds total spendable value". This exercises the
        // `num_actions = max(0, 2) = 2` branch.
        let platform_version = PlatformVersion::latest();
        let recipient = test_orchard_address();
        let change_address = test_orchard_address();

        let sk = grovedb_commitment_tree::SpendingKey::from_bytes([42u8; 32]).expect("valid sk");
        let fvk = FullViewingKey::from(&sk);
        let ask = SpendAuthorizingKey::from(&sk);

        let result = build_shielded_transfer_transition(
            vec![],
            &recipient,
            1,
            &change_address,
            &fvk,
            &ask,
            Anchor::empty_tree(),
            &TestProver,
            [0u8; 36],
            platform_version,
        );
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("exceeds total spendable value"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn test_shielded_transfer_uses_min_fee() {
        // The fee is always the minimum. Verify that a note *exactly* equal to
        // `transfer_amount + min_fee` proceeds past the "exceeds total" check (it then
        // fails later in add_spend due to anchor mismatch).
        let platform_version = PlatformVersion::latest();
        let recipient = test_orchard_address();
        let change_address = test_orchard_address();

        let min_fee = crate::shielded::compute_minimum_shielded_fee(2, platform_version)
            .expect("fee computation should not overflow");
        let transfer_amount = 10u64;
        let note = test_spendable_note(transfer_amount + min_fee);
        let spends = vec![note];

        let sk = grovedb_commitment_tree::SpendingKey::from_bytes([42u8; 32]).expect("valid sk");
        let fvk = FullViewingKey::from(&sk);
        let ask = SpendAuthorizingKey::from(&sk);

        let result = build_shielded_transfer_transition(
            spends,
            &recipient,
            transfer_amount,
            &change_address,
            &fvk,
            &ask,
            Anchor::empty_tree(),
            &TestProver,
            [0u8; 36],
            platform_version,
        );

        // With a valid fee/amount relationship, the builder proceeds past
        // the amount checks and hits the add_spend AnchorMismatch.
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("failed to add spend")
                || err_msg.contains("anchor")
                || err_msg.contains("AnchorMismatch"),
            "expected downstream add_spend error, got: {}",
            err_msg
        );
    }
}
