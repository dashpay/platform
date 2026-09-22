//! Core's per-output dust threshold, mirrored so Platform can tell when an asset unlock
//! payout can never enter a Core mempool.

use dashcore::Script;

/// Serialized size Core assumes for the input that would later spend a non-witness output:
/// 32 (previous txid) + 4 (previous index) + 1 (script length) + 107 (P2PKH scriptSig)
/// + 4 (sequence).
const SPEND_INPUT_SIZE: u64 = 148;

/// Serialized size of a `TxOut`'s value field.
const TX_OUT_VALUE_SIZE: u64 = 8;

/// The minimum value in duffs an output paying `output_script` must carry for Core's mempool
/// to accept it, at a dust relay fee of `dust_relay_fee_per_kb` duffs per kilobyte.
///
/// Mirrors Core's `GetDustThreshold`: the fee, at the dust relay rate, of the serialized
/// output plus the input that would spend it. Unspendable (`OP_RETURN`) outputs are never
/// dust. At Core's default 3000 duffs/kB a P2PKH output needs 546 duffs and a P2SH output
/// 540. Dust is a mempool policy, not a consensus rule: a payout below it is refused by every
/// relaying node, but a miner could still include a signed transaction directly.
pub fn core_dust_threshold_duffs(output_script: &Script, dust_relay_fee_per_kb: u64) -> u64 {
    if output_script.is_op_return() {
        return 0;
    }

    let script_len = output_script.len() as u64;
    let serialized_size =
        SPEND_INPUT_SIZE + TX_OUT_VALUE_SIZE + var_int_size(script_len) + script_len;

    // Core's `CFeeRate::GetFee`: rate * size / 1000, never rounding a positive rate down to 0.
    let fee = dust_relay_fee_per_kb.saturating_mul(serialized_size) / 1000;
    if fee == 0 && dust_relay_fee_per_kb > 0 {
        1
    } else {
        fee
    }
}

/// Serialized size of a Bitcoin-style compact size integer.
fn var_int_size(value: u64) -> u64 {
    match value {
        0..=0xFC => 1,
        0xFD..=0xFFFF => 3,
        0x1_0000..=0xFFFF_FFFF => 5,
        _ => 9,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashcore::ScriptBuf;

    const CORE_DEFAULT_DUST_RELAY_FEE_PER_KB: u64 = 3000;

    fn p2pkh() -> ScriptBuf {
        let mut bytes = vec![0x76, 0xa9, 0x14];
        bytes.extend_from_slice(&[0x11; 20]);
        bytes.extend_from_slice(&[0x88, 0xac]);
        ScriptBuf::from_bytes(bytes)
    }

    fn p2sh() -> ScriptBuf {
        let mut bytes = vec![0xa9, 0x14];
        bytes.extend_from_slice(&[0x22; 20]);
        bytes.push(0x87);
        ScriptBuf::from_bytes(bytes)
    }

    #[test]
    fn should_match_core_default_thresholds() {
        assert_eq!(
            core_dust_threshold_duffs(&p2pkh(), CORE_DEFAULT_DUST_RELAY_FEE_PER_KB),
            546
        );
        assert_eq!(
            core_dust_threshold_duffs(&p2sh(), CORE_DEFAULT_DUST_RELAY_FEE_PER_KB),
            540
        );
    }

    #[test]
    fn should_agree_with_dashcore_dust_value_at_the_default_rate() {
        for script in [p2pkh(), p2sh()] {
            assert_eq!(
                core_dust_threshold_duffs(&script, CORE_DEFAULT_DUST_RELAY_FEE_PER_KB),
                script.dust_value().to_sat()
            );
        }
    }

    #[test]
    fn should_never_treat_unspendable_outputs_as_dust() {
        let op_return = ScriptBuf::from_bytes(vec![0x6a, 0x03, 0x01, 0x02, 0x03]);
        assert_eq!(
            core_dust_threshold_duffs(&op_return, CORE_DEFAULT_DUST_RELAY_FEE_PER_KB),
            0
        );
    }

    #[test]
    fn should_scale_with_the_relay_fee_and_floor_a_positive_rate_at_one_duff() {
        assert_eq!(core_dust_threshold_duffs(&p2pkh(), 0), 0);
        assert_eq!(core_dust_threshold_duffs(&p2pkh(), 1), 1);
        assert_eq!(core_dust_threshold_duffs(&p2pkh(), 6000), 1092);
    }
}
