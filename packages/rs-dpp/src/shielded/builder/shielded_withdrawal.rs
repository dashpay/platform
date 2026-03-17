use grovedb_commitment_tree::{Anchor, FullViewingKey, SpendAuthorizingKey};

use crate::address_funds::OrchardAddress;
use crate::fee::Credits;
use crate::identity::core_script::CoreScript;
use crate::shielded::compute_minimum_shielded_fee;
use crate::state_transition::shielded_withdrawal_transition::methods::ShieldedWithdrawalTransitionMethodsV0;
use crate::state_transition::shielded_withdrawal_transition::ShieldedWithdrawalTransition;
use crate::state_transition::StateTransition;
use crate::withdrawal::Pooling;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

use super::{build_spend_bundle, serialize_authorized_bundle, OrchardProver, SpendableNote};

/// Builds a ShieldedWithdrawal state transition (shielded pool -> core L1 address).
///
/// Spends existing notes and withdraws value to a core chain script output.
/// The shielded fee is deducted from the spent notes. Any remaining value is
/// returned to the shielded `change_address`.
///
/// # Parameters
/// - `spends` - Notes to spend with their Merkle paths
/// - `withdrawal_amount` - Amount to withdraw to the core chain
/// - `output_script` - Core chain script to receive the funds
/// - `core_fee_per_byte` - Core chain fee rate
/// - `pooling` - Withdrawal pooling strategy
/// - `change_address` - Orchard address for change output
/// - `fvk` - Full viewing key for spend authorization
/// - `ask` - Spend authorizing key for RedPallas signatures
/// - `anchor` - Sinsemilla root of the note commitment tree (Orchard Anchor)
/// - `prover` - Orchard prover (holds the Halo 2 proving key)
/// - `memo` - 36-byte structured memo for the change output (4-byte type tag + 32-byte payload)
/// - `fee` - Optional fee override; if `None`, the minimum fee is computed automatically.
///   If `Some`, must be >= the minimum fee.
/// - `platform_version` - Protocol version
#[allow(clippy::too_many_arguments)]
pub fn build_shielded_withdrawal_transition<P: OrchardProver>(
    spends: Vec<SpendableNote>,
    withdrawal_amount: u64,
    output_script: CoreScript,
    core_fee_per_byte: u32,
    pooling: Pooling,
    change_address: &OrchardAddress,
    fvk: &FullViewingKey,
    ask: &SpendAuthorizingKey,
    anchor: Anchor,
    prover: &P,
    memo: [u8; 36],
    fee: Option<Credits>,
    platform_version: &PlatformVersion,
) -> Result<StateTransition, ProtocolError> {
    if withdrawal_amount > i64::MAX as u64 {
        return Err(ProtocolError::ShieldedBuildError(format!(
            "withdrawal amount {} exceeds maximum allowed value {}",
            withdrawal_amount,
            i64::MAX as u64
        )));
    }

    let total_spent: u64 = spends.iter().map(|s| s.note.value().inner()).sum();

    // Conservative action count: at least (spends, 1) since we have a change output.
    let num_actions = spends.len().max(1);
    let min_fee = compute_minimum_shielded_fee(num_actions, platform_version);
    let effective_fee = match fee {
        Some(f) if f < min_fee => {
            return Err(ProtocolError::ShieldedBuildError(format!(
                "fee {} is below minimum required fee {}",
                f, min_fee
            )));
        }
        Some(f) => f,
        None => min_fee,
    };

    let required = withdrawal_amount
        .checked_add(effective_fee)
        .ok_or_else(|| {
            ProtocolError::ShieldedBuildError("fee + withdrawal_amount overflows u64".to_string())
        })?;
    if required > total_spent {
        return Err(ProtocolError::ShieldedBuildError(format!(
            "withdrawal amount {} + fee {} = {} exceeds total spendable value {}",
            withdrawal_amount, effective_fee, required, total_spent
        )));
    }

    let change_amount = total_spent - required;

    // ShieldedWithdrawal extra_data = output_script || unshielding_amount (le bytes)
    // Must match server-side sighash in shielded_proof.rs
    let mut extra_sighash_data = output_script.as_bytes().to_vec();
    extra_sighash_data.extend_from_slice(&required.to_le_bytes());

    let bundle = build_spend_bundle(
        spends,
        change_address,
        change_amount,
        memo,
        fvk,
        ask,
        anchor,
        prover,
        &extra_sighash_data,
    )?;

    let sb = serialize_authorized_bundle(&bundle);

    ShieldedWithdrawalTransition::try_from_bundle(
        sb.actions,
        sb.value_balance as u64,
        sb.anchor,
        sb.proof,
        sb.binding_signature,
        core_fee_per_byte,
        pooling,
        output_script,
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
    fn test_shielded_withdrawal_fee_below_minimum() {
        let platform_version = PlatformVersion::latest();
        let change_address = test_orchard_address();

        let note = test_spendable_note(1_000_000);
        let spends = vec![note];

        let sk = grovedb_commitment_tree::SpendingKey::from_bytes([42u8; 32])
            .expect("valid spending key bytes");
        let fvk = FullViewingKey::from(&sk);
        let ask = SpendAuthorizingKey::from(&sk);

        let result = build_shielded_withdrawal_transition(
            spends,
            100,
            CoreScript::new_p2pkh([1u8; 20]), // minimal P2PKH prefix
            1,
            Pooling::Never,
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
    fn test_shielded_withdrawal_insufficient_funds() {
        let platform_version = PlatformVersion::latest();
        let change_address = test_orchard_address();

        let note = test_spendable_note(100);
        let spends = vec![note];

        let sk = grovedb_commitment_tree::SpendingKey::from_bytes([42u8; 32])
            .expect("valid spending key bytes");
        let fvk = FullViewingKey::from(&sk);
        let ask = SpendAuthorizingKey::from(&sk);

        let result = build_shielded_withdrawal_transition(
            spends,
            1_000_000,
            CoreScript::new_p2pkh([1u8; 20]),
            1,
            Pooling::Never,
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
}
