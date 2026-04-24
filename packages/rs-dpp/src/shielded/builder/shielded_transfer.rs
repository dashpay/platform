use grovedb_commitment_tree::{
    Anchor, Builder, BundleType, DashMemo, FullViewingKey, NoteValue, PaymentAddress,
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

use super::{prove_and_sign_bundle, serialize_authorized_bundle, OrchardProver, SpendableNote};

/// Builds a ShieldedTransfer state transition (shielded pool -> shielded pool).
///
/// Spends existing notes and creates a new note for the recipient. The shielded
/// fee is deducted from the spent notes. Any remaining change is returned to
/// the `change_address`.
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
/// - `fee` - Optional fee override; if `None`, the minimum fee is computed automatically.
///   If `Some`, must be >= the minimum fee.
/// - `platform_version` - Protocol version
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
    fee: Option<Credits>,
    platform_version: &PlatformVersion,
) -> Result<StateTransition, ProtocolError> {
    let total_spent: u64 = spends.iter().map(|s| s.note.value().inner()).sum();

    // Conservative action count: at least (spends, 2) since we always have
    // a recipient output and likely a change output.
    let num_actions = spends.len().max(2);
    let min_fee = compute_minimum_shielded_fee(num_actions, platform_version);
    let effective_fee = match fee {
        Some(f) if f < min_fee => {
            return Err(ProtocolError::ShieldedBuildError(format!(
                "fee {} is below minimum required fee {}",
                f, min_fee
            )));
        }
        Some(f) if f > min_fee.saturating_mul(1000) => {
            return Err(ProtocolError::ShieldedBuildError(format!(
                "fee {} exceeds 1000x the minimum fee {}",
                f, min_fee
            )));
        }
        Some(f) => f,
        None => min_fee,
    };

    let required = transfer_amount.checked_add(effective_fee).ok_or_else(|| {
        ProtocolError::ShieldedBuildError("fee + transfer_amount overflows u64".to_string())
    })?;
    if required > total_spent {
        return Err(ProtocolError::ShieldedBuildError(format!(
            "transfer amount {} + fee {} = {} exceeds total spendable value {}",
            transfer_amount, effective_fee, required, total_spent
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

    // Primary output to recipient
    builder
        .add_output(
            None,
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
                None,
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

    // value_balance = effective_fee (the amount leaving the shielded pool as fee)
    ShieldedTransferTransition::try_from_bundle(
        sb.actions,
        sb.value_balance as u64,
        sb.anchor,
        sb.proof,
        sb.binding_signature,
        platform_version,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shielded::builder::test_helpers::{
        test_orchard_address, test_spendable_note, TestProver,
    };

    #[test]
    fn test_shielded_transfer_fee_below_minimum() {
        let platform_version = PlatformVersion::latest();
        let recipient = test_orchard_address();
        let change_address = test_orchard_address();

        let note = test_spendable_note(1_000_000);
        let spends = vec![note];

        let sk = grovedb_commitment_tree::SpendingKey::from_bytes([42u8; 32])
            .expect("valid spending key bytes");
        let fvk = FullViewingKey::from(&sk);
        let ask = SpendAuthorizingKey::from(&sk);

        let result = build_shielded_transfer_transition(
            spends,
            &recipient,
            100,
            &change_address,
            &fvk,
            &ask,
            Anchor::empty_tree(),
            &TestProver,
            [0u8; 36],
            Some(1), // fee = 1, should be below minimum
            platform_version,
        );

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("below minimum required fee"),
            "unexpected error: {}",
            err
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
            None,
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
    fn test_shielded_transfer_fee_above_upper_bound() {
        // Fee > 1000x the minimum fee should be rejected.
        let platform_version = PlatformVersion::latest();
        let recipient = test_orchard_address();
        let change_address = test_orchard_address();

        let note = test_spendable_note(u64::MAX);
        let spends = vec![note];

        let sk = grovedb_commitment_tree::SpendingKey::from_bytes([42u8; 32])
            .expect("valid spending key bytes");
        let fvk = FullViewingKey::from(&sk);
        let ask = SpendAuthorizingKey::from(&sk);

        // num_actions is max(spends.len(), 2) = 2.
        let min_fee = crate::shielded::compute_minimum_shielded_fee(2, platform_version);
        let excessive_fee = min_fee.saturating_mul(1000) + 1;

        let result = build_shielded_transfer_transition(
            spends,
            &recipient,
            10,
            &change_address,
            &fvk,
            &ask,
            Anchor::empty_tree(),
            &TestProver,
            [0u8; 36],
            Some(excessive_fee),
            platform_version,
        );

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("exceeds 1000x the minimum fee"),
            "unexpected error: {}",
            err
        );
    }

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

        // Compute min fee, then craft a fee that lies in [min_fee, 1000*min_fee]
        // so we bypass the boundary checks, then pick transfer_amount = u64::MAX
        // so amount + fee overflows.
        let min_fee = crate::shielded::compute_minimum_shielded_fee(2, platform_version);

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
            Some(min_fee), // within [min, 1000*min]
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
            None,
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
    fn test_shielded_transfer_fee_default_is_min_fee() {
        // When fee is None, the default min fee is computed — verify that a
        // note *exactly* equal to `transfer_amount + min_fee` on the default
        // branch does not spuriously fail the "exceeds total" check (it
        // fails later in add_spend due to anchor mismatch).
        let platform_version = PlatformVersion::latest();
        let recipient = test_orchard_address();
        let change_address = test_orchard_address();

        let min_fee = crate::shielded::compute_minimum_shielded_fee(2, platform_version);
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
            None,
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
