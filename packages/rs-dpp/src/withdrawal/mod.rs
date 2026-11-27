pub mod core_credit_pool_unlock_limit;
mod core_dust_threshold;
pub mod daily_withdrawal_limit;
#[cfg(all(feature = "withdrawals-contract", feature = "system_contracts"))]
mod document_try_into_asset_unlock_base_transaction_info;

pub use core_dust_threshold::core_dust_threshold_duffs;

use bincode::{Decode, DecodeUntrusted, Encode};
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::balances::credits::CREDITS_PER_DUFF;
#[cfg(feature = "state-transitions")]
use crate::consensus::basic::identity::{
    InvalidCreditWithdrawalTransitionCoreFeeError,
    InvalidIdentityCreditWithdrawalTransitionAmountError,
};
#[cfg(feature = "state-transitions")]
use crate::consensus::basic::state_transition::WithdrawalBelowMinAmountError;
#[cfg(feature = "state-transitions")]
use crate::consensus::basic::unsupported_version_error::UnsupportedVersionError;
#[cfg(feature = "state-transitions")]
use crate::consensus::basic::BasicError;
#[cfg(feature = "state-transitions")]
use crate::consensus::ConsensusError;
use crate::fee::Credits;
#[cfg(feature = "state-transitions")]
use crate::state_transition::identity_credit_withdrawal_transition::MIN_CORE_FEE_PER_BYTE;
#[cfg(feature = "state-transitions")]
use crate::util::is_non_zero_fibonacci_number::is_non_zero_fibonacci_number;
#[cfg(feature = "state-transitions")]
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use dashcore::transaction::special_transaction::asset_unlock::qualified_asset_unlock::ASSET_UNLOCK_TX_SIZE;
use platform_version::version::PlatformVersion;

#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;

#[repr(u8)]
#[derive(
    Serialize_repr,
    Deserialize_repr,
    PartialEq,
    Eq,
    Clone,
    Copy,
    Debug,
    Encode,
    Decode,
    Default,
    DecodeUntrusted,
)]
pub enum Pooling {
    #[default]
    Never = 0,
    IfAvailable = 1,
    Standard = 2,
}

#[cfg(feature = "json-conversion")]
impl JsonConvertible for Pooling {}

#[cfg(feature = "value-conversion")]
impl ValueConvertible for Pooling {}

/// Transaction index type
pub type WithdrawalTransactionIndex = u64;

/// Simple type alias for withdrawal transaction with it's index
pub type WithdrawalTransactionIndexAndBytes = (WithdrawalTransactionIndex, Vec<u8>);

/// Core fee charged by an asset unlock transaction, expressed in Platform credits.
pub fn core_fee_in_credits(core_fee_per_byte: u32) -> Option<Credits> {
    (ASSET_UNLOCK_TX_SIZE as u64)
        .checked_mul(core_fee_per_byte as u64)?
        .checked_mul(CREDITS_PER_DUFF)
}

/// Rejects a Core fee rate above the protocol version's `max_core_fee_per_byte`.
///
/// Stateless rule from protocol version 14, shared by the identity, address and shielded
/// withdrawal structure validators of that generation. A protocol version without a cap
/// accepts any rate here: `None` preserves the behavior of the protocol versions that predate
/// the limit, per the field's contract.
#[cfg(feature = "state-transitions")]
pub fn validate_core_fee_per_byte_cap(
    core_fee_per_byte: u32,
    platform_version: &PlatformVersion,
) -> SimpleConsensusValidationResult {
    match platform_version.system_limits.max_core_fee_per_byte {
        Some(max_core_fee_per_byte) if core_fee_per_byte > max_core_fee_per_byte => {
            SimpleConsensusValidationResult::new_with_error(
                InvalidCreditWithdrawalTransitionCoreFeeError::new(
                    core_fee_per_byte,
                    MIN_CORE_FEE_PER_BYTE,
                )
                .into(),
            )
        }
        _ => SimpleConsensusValidationResult::new(),
    }
}

/// Minimum amount a withdrawal must reserve for Core from protocol version 14:
/// `min_withdrawal_amount` plus the Core fee of the asset unlock transaction at
/// `core_fee_per_byte`. From that version the fee is carved out of the reserved amount instead
/// of being drawn from the Core credit pool on top of it, so the amount has to leave the
/// protocol floor above the fee.
///
/// The sum cannot overflow: the fee of a 190-byte transaction at a `u32` rate is below 2^50
/// credits and the floor is a table constant. Saturating keeps the rejecting direction should
/// either bound ever change.
pub fn min_withdrawal_amount_with_core_fee(
    core_fee_per_byte: u32,
    platform_version: &PlatformVersion,
) -> Credits {
    let core_fee = core_fee_in_credits(core_fee_per_byte).unwrap_or(Credits::MAX);
    platform_version
        .system_limits
        .min_withdrawal_amount
        .saturating_add(core_fee)
}

/// Rejects a Core fee rate that is not a non-zero Fibonacci number.
///
/// Stateless rule of every shielded withdrawal structure generation, shared with
/// [`preflight_shielded_withdrawal`].
#[cfg(feature = "state-transitions")]
pub fn validate_core_fee_per_byte_is_non_zero_fibonacci(
    core_fee_per_byte: u32,
) -> SimpleConsensusValidationResult {
    if is_non_zero_fibonacci_number(core_fee_per_byte as u64) {
        SimpleConsensusValidationResult::new()
    } else {
        SimpleConsensusValidationResult::new_with_error(
            InvalidCreditWithdrawalTransitionCoreFeeError::new(
                core_fee_per_byte,
                MIN_CORE_FEE_PER_BYTE,
            )
            .into(),
        )
    }
}

/// Rejects a `reserved_amount` (what a withdrawal reserves for Core) outside
/// `[minimum_reserved_amount, maximum_reserved_amount]`.
#[cfg(feature = "state-transitions")]
pub fn validate_withdrawal_reserved_amount_range(
    reserved_amount: Credits,
    minimum_reserved_amount: Credits,
    maximum_reserved_amount: Credits,
) -> SimpleConsensusValidationResult {
    if reserved_amount < minimum_reserved_amount {
        return SimpleConsensusValidationResult::new_with_error(
            BasicError::WithdrawalBelowMinAmountError(WithdrawalBelowMinAmountError::new(
                reserved_amount,
                minimum_reserved_amount,
                maximum_reserved_amount,
            ))
            .into(),
        );
    }

    if reserved_amount > maximum_reserved_amount {
        return SimpleConsensusValidationResult::new_with_error(
            InvalidIdentityCreditWithdrawalTransitionAmountError::new(
                reserved_amount,
                minimum_reserved_amount,
                maximum_reserved_amount,
            )
            .into(),
        );
    }

    SimpleConsensusValidationResult::new()
}

/// The Core fee in duffs an asset unlock transaction requests at `core_fee_per_byte` from
/// protocol version 14: the rate is clamped to the version's `max_core_fee_per_byte`. Errors
/// for a version without a cap, which does not carve the fee out of the withdrawn amount.
pub fn asset_unlock_core_fee_duffs(
    core_fee_per_byte: u32,
    platform_version: &PlatformVersion,
) -> Result<u64, ProtocolError> {
    let max_core_fee_per_byte = platform_version
        .system_limits
        .max_core_fee_per_byte
        .ok_or_else(|| {
            ProtocolError::Generic(
                "asset unlock conversion v1 requires a Core fee limit".to_string(),
            )
        })?;
    (ASSET_UNLOCK_TX_SIZE as u64)
        .checked_mul(core_fee_per_byte.min(max_core_fee_per_byte) as u64)
        .ok_or_else(|| ProtocolError::Generic("asset unlock fee overflow".to_string()))
}

/// What a shielded withdrawal's reserve for Core pays out, as the withdrawal document
/// conversion turns it into the asset unlock output.
#[cfg(feature = "state-transitions")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShieldedWithdrawalReserve {
    /// The Core fee carved out of the reserve, in duffs. 0 before protocol version 14, where
    /// Core draws the fee from its credit pool on top of the reserve.
    pub core_fee_duffs: u64,
    /// The asset unlock output, in duffs: the reserve truncated to whole duffs, minus the fee.
    pub output_duffs: u64,
}

#[cfg(feature = "state-transitions")]
impl ShieldedWithdrawalReserve {
    /// The output in credits: at most the reserve, without its sub-duff remainder and the Core
    /// fee.
    pub fn output_credits(&self) -> Credits {
        self.output_duffs.saturating_mul(CREDITS_PER_DUFF)
    }
}

/// Checks a shielded withdrawal that reserves `reserved_amount` for Core at `core_fee_per_byte`
/// the way consensus does, and returns what it pays out.
///
/// The rules follow the protocol version's `validate_withdrawal_structure` generation: a
/// non-zero Fibonacci rate always; from the generation that caps the rate also the cap, and a
/// reserve above the floor plus the Core fee. Before it the reserve must lie in
/// `[min_withdrawal_amount, max_withdrawal_amount]`, which drive enforces on the net amount
/// there. The accounting follows the generation of `try_into_asset_unlock_base_transaction_info`.
/// The reserve is gross: what the withdrawal debits is not changed by the result.
#[cfg(feature = "state-transitions")]
pub fn preflight_shielded_withdrawal(
    reserved_amount: Credits,
    core_fee_per_byte: u32,
    platform_version: &PlatformVersion,
) -> Result<ShieldedWithdrawalReserve, ConsensusError> {
    let limits = &platform_version.system_limits;
    let mut result = validate_core_fee_per_byte_is_non_zero_fibonacci(core_fee_per_byte);
    match platform_version
        .dpp
        .state_transitions
        .shielded
        .validate_withdrawal_structure
    {
        0 => {
            if result.is_valid() {
                result = validate_withdrawal_reserved_amount_range(
                    reserved_amount,
                    limits.min_withdrawal_amount,
                    limits.max_withdrawal_amount,
                );
            }
        }
        1 => {
            if result.is_valid() {
                result = validate_core_fee_per_byte_cap(core_fee_per_byte, platform_version);
            }
            if result.is_valid() {
                result = validate_withdrawal_reserved_amount_range(
                    reserved_amount,
                    min_withdrawal_amount_with_core_fee(core_fee_per_byte, platform_version),
                    limits.max_withdrawal_amount,
                );
            }
        }
        version => {
            return Err(
                BasicError::UnsupportedVersionError(UnsupportedVersionError::new(version, 0, 1))
                    .into(),
            )
        }
    }
    if let Some(error) = result.errors.into_iter().next() {
        return Err(error);
    }

    // Whole duffs first, then the fee: the order the withdrawal document conversion uses.
    let core_fee_duffs = match platform_version
        .dpp
        .document_versions
        .document_method_versions
        .try_into_asset_unlock_base_transaction_info
    {
        0 => 0,
        // A table that selects the carving generation without a cap cannot account for it.
        1 => asset_unlock_core_fee_duffs(core_fee_per_byte, platform_version).map_err(|_| {
            ConsensusError::from(BasicError::UnsupportedVersionError(
                UnsupportedVersionError::new(1, 0, 0),
            ))
        })?,
        version => {
            return Err(
                BasicError::UnsupportedVersionError(UnsupportedVersionError::new(version, 0, 1))
                    .into(),
            )
        }
    };
    Ok(ShieldedWithdrawalReserve {
        core_fee_duffs,
        // A reserve in range exceeds the fee: the minimum adds the fee to the floor.
        output_duffs: (reserved_amount / CREDITS_PER_DUFF).saturating_sub(core_fee_duffs),
    })
}

/// Serde helper for `Pooling` fields exposed through the JS surface.
///
/// `Pooling` is `#[repr(u8)]` with `Serialize_repr` / `Deserialize_repr`, so the
/// default wire shape is the numeric discriminant (`0`/`1`/`2`). That number
/// leaks into JSON / Object output and makes `XxxJSON.pooling: string`
/// declarations false. The helper switches the **human-readable** path to a
/// camelCase string (`"never"`/`"ifAvailable"`/`"standard"`) while keeping the
/// non-HR path at the original `u8` so bincode (consensus binary format) is
/// untouched.
///
/// Apply via `#[serde(with = "crate::withdrawal::pooling_serde")]` on the
/// `pooling` field of any state transition that surfaces it to JS.
#[cfg(feature = "serde-conversion")]
pub mod pooling_serde {
    use super::Pooling;
    use serde::{Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(pooling: &Pooling, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            let name = match pooling {
                Pooling::Never => "never",
                Pooling::IfAvailable => "ifAvailable",
                Pooling::Standard => "standard",
            };
            serializer.serialize_str(name)
        } else {
            (*pooling as u8).serialize(serializer)
        }
    }

    /// Deserialize accepts both shapes regardless of the deserializer's
    /// human-readable flag — mirrors the `BinaryData` / `Identifier` pattern.
    /// Necessary because `platform_value::to_value` reports HR=false (emits the
    /// numeric discriminant on the way to `JsValue`), but
    /// `platform_value::from_value` reports HR=true on the way back. Without
    /// dual acceptance, the `fromObject(toObject())` round-trip fails on the
    /// `pooling` field.
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Pooling, D::Error> {
        struct PoolingVisitor;

        impl<'de> serde::de::Visitor<'de> for PoolingVisitor {
            type Value = Pooling;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a Pooling variant: 'never'/'ifAvailable'/'standard' or 0/1/2")
            }

            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Pooling, E> {
                match v {
                    "never" | "Never" => Ok(Pooling::Never),
                    "ifAvailable" | "IfAvailable" | "ifavailable" => Ok(Pooling::IfAvailable),
                    "standard" | "Standard" => Ok(Pooling::Standard),
                    other => Err(E::custom(format!(
                        "unknown pooling variant '{}', expected 'never' | 'ifAvailable' | 'standard'",
                        other
                    ))),
                }
            }

            fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Pooling, E> {
                self.visit_str(&v)
            }

            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Pooling, E> {
                match v {
                    0 => Ok(Pooling::Never),
                    1 => Ok(Pooling::IfAvailable),
                    2 => Ok(Pooling::Standard),
                    other => Err(E::custom(format!("unknown pooling discriminant {}", other))),
                }
            }

            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Pooling, E> {
                if v < 0 {
                    return Err(E::custom(format!("negative pooling discriminant {}", v)));
                }
                self.visit_u64(v as u64)
            }

            fn visit_u8<E: serde::de::Error>(self, v: u8) -> Result<Pooling, E> {
                self.visit_u64(v as u64)
            }
        }

        if deserializer.is_human_readable() {
            deserializer.deserialize_any(PoolingVisitor)
        } else {
            deserializer.deserialize_u8(PoolingVisitor)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct Wrap(#[serde(with = "super")] Pooling);

        #[test]
        fn json_emits_camelcase_string() {
            for (variant, expected) in [
                (Pooling::Never, "\"never\""),
                (Pooling::IfAvailable, "\"ifAvailable\""),
                (Pooling::Standard, "\"standard\""),
            ] {
                let json = serde_json::to_string(&Wrap(variant)).expect("serialize");
                assert_eq!(json, expected);
                let restored: Wrap = serde_json::from_str(expected).expect("deserialize");
                assert_eq!(restored, Wrap(variant));
            }
        }

        #[test]
        fn bincode_keeps_u8_discriminant() {
            for (variant, expected_u8) in [
                (Pooling::Never, 0),
                (Pooling::IfAvailable, 1),
                (Pooling::Standard, 2),
            ] {
                let bytes =
                    bincode::serde::encode_to_vec(Wrap(variant), bincode::config::standard())
                        .expect("bincode encode");
                assert_eq!(bytes.last(), Some(&expected_u8));
                let (restored, _): (Wrap, usize) =
                    bincode::serde::decode_from_slice(&bytes, bincode::config::standard())
                        .expect("bincode decode");
                assert_eq!(restored, Wrap(variant));
            }
        }
    }
}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests_pooling {
    use super::*;
    use platform_value::platform_value;
    use serde_json::json;

    // `Pooling` is `#[repr(u8)]` with `Serialize_repr` / `Deserialize_repr`, so
    // the wire shape is the raw `u8` discriminant: `0` / `1` / `2`. JSON has
    // only one number type, so `0u8` is erased to `Number(0)`; the value-path
    // assertion uses explicit `0u8` etc. to lock in `Value::U8`.

    #[test]
    fn json_round_trip_never() {
        use crate::serialization::JsonConvertible;
        let original = Pooling::Never;
        let json = original.to_json().expect("to_json");
        // u8 size erased in JSON.
        assert_eq!(json, json!(0));
        let recovered = Pooling::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_round_trip_if_available() {
        use crate::serialization::JsonConvertible;
        let original = Pooling::IfAvailable;
        let json = original.to_json().expect("to_json");
        assert_eq!(json, json!(1));
        let recovered = Pooling::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_round_trip_standard() {
        use crate::serialization::JsonConvertible;
        let original = Pooling::Standard;
        let json = original.to_json().expect("to_json");
        assert_eq!(json, json!(2));
        let recovered = Pooling::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_never() {
        use crate::serialization::ValueConvertible;
        let original = Pooling::Never;
        let value = original.to_object().expect("to_object");
        // `0u8` locks `Value::U8` (not I32 from a bare `0`).
        assert_eq!(value, platform_value!(0u8));
        let recovered = Pooling::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_if_available() {
        use crate::serialization::ValueConvertible;
        let original = Pooling::IfAvailable;
        let value = original.to_object().expect("to_object");
        assert_eq!(value, platform_value!(1u8));
        let recovered = Pooling::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_standard() {
        use crate::serialization::ValueConvertible;
        let original = Pooling::Standard;
        let value = original.to_object().expect("to_object");
        assert_eq!(value, platform_value!(2u8));
        let recovered = Pooling::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}

#[cfg(all(test, feature = "state-transitions"))]
mod core_fee_tests {
    use super::*;
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;
    use assert_matches::assert_matches;

    #[test]
    fn should_reject_a_core_fee_rate_above_the_cap() {
        let platform_version = PlatformVersion::latest();
        let cap = platform_version
            .system_limits
            .max_core_fee_per_byte
            .expect("the latest protocol version caps the Core fee rate");

        assert!(validate_core_fee_per_byte_cap(cap, platform_version).is_valid());
        assert_matches!(
            validate_core_fee_per_byte_cap(cap + 1, platform_version)
                .errors
                .as_slice(),
            [ConsensusError::BasicError(
                BasicError::InvalidCreditWithdrawalTransitionCoreFeeError(error)
            )] if error.core_fee_per_byte() == cap + 1
        );
    }

    #[test]
    fn should_accept_any_core_fee_rate_without_a_cap() {
        // Protocol version 13 is live without the limit, so its table carries no cap.
        let platform_version = PlatformVersion::get(13).expect("protocol version 13");
        assert_eq!(platform_version.system_limits.max_core_fee_per_byte, None);

        assert!(validate_core_fee_per_byte_cap(u32::MAX, platform_version).is_valid());
    }

    fn rate_error(result: Result<ShieldedWithdrawalReserve, ConsensusError>, rate: u32) {
        assert_matches!(
            result,
            Err(ConsensusError::BasicError(
                BasicError::InvalidCreditWithdrawalTransitionCoreFeeError(error)
            )) if error.core_fee_per_byte() == rate
        );
    }

    #[test]
    fn preflight_truncates_the_reserve_to_whole_duffs_before_the_core_fee() {
        let version_14 = PlatformVersion::get(14).expect("protocol version 14");

        // 1_000_000 duffs and 1 credit: the credit is dropped, then the 190-duff fee.
        let reserve =
            preflight_shielded_withdrawal(1_000_000_001, 1, version_14).expect("in range");
        assert_eq!(reserve.core_fee_duffs, 190);
        assert_eq!(reserve.output_duffs, 999_810);
        assert_eq!(reserve.output_credits(), 999_810_000);

        // A larger reserve at a higher rate, off a duff boundary by 999 credits.
        let reserve =
            preflight_shielded_withdrawal(2_345_678_999, 5, version_14).expect("in range");
        assert_eq!(reserve.core_fee_duffs, 950);
        assert_eq!(
            reserve.output_credits(),
            (2_345_678 - 950) * CREDITS_PER_DUFF
        );
    }

    #[test]
    fn preflight_before_the_core_fee_carve_out_pays_the_whole_duffs_of_the_reserve() {
        let version_13 = PlatformVersion::get(13).expect("protocol version 13");

        let reserve =
            preflight_shielded_withdrawal(1_000_000_999, 1, version_13).expect("in range");
        assert_eq!(reserve.core_fee_duffs, 0);
        assert_eq!(reserve.output_credits(), 1_000_000_000);
    }

    #[test]
    fn preflight_refuses_a_zero_or_non_fibonacci_rate_before_and_after_the_cap() {
        for version in [13, 14] {
            let platform_version = PlatformVersion::get(version).expect("protocol version");
            for rate in [0, 4] {
                rate_error(
                    preflight_shielded_withdrawal(2_000_000_000, rate, platform_version),
                    rate,
                );
            }
        }
    }

    #[test]
    fn preflight_refuses_a_rate_above_the_cap_only_once_the_cap_applies() {
        let version_13 = PlatformVersion::get(13).expect("protocol version 13");
        let version_14 = PlatformVersion::get(14).expect("protocol version 14");

        assert!(preflight_shielded_withdrawal(2_000_000_000, 10_946, version_13).is_ok());
        rate_error(
            preflight_shielded_withdrawal(2_000_000_000, 10_946, version_14),
            10_946,
        );
    }

    #[test]
    fn preflight_keeps_the_reserve_bounds_of_each_generation() {
        let version_13 = PlatformVersion::get(13).expect("protocol version 13");
        let version_14 = PlatformVersion::get(14).expect("protocol version 14");
        let floor = version_14.system_limits.min_withdrawal_amount;
        let max = version_14.system_limits.max_withdrawal_amount;
        let carved_floor = min_withdrawal_amount_with_core_fee(1, version_14);

        assert!(preflight_shielded_withdrawal(floor, 1, version_13).is_ok());
        assert_matches!(
            preflight_shielded_withdrawal(floor, 1, version_14),
            Err(ConsensusError::BasicError(
                BasicError::WithdrawalBelowMinAmountError(_)
            ))
        );
        assert!(preflight_shielded_withdrawal(carved_floor, 1, version_14).is_ok());
        assert_matches!(
            preflight_shielded_withdrawal(max + 1, 1, version_14),
            Err(ConsensusError::BasicError(
                BasicError::InvalidIdentityCreditWithdrawalTransitionAmountError(_)
            ))
        );
    }

    #[test]
    fn should_add_the_core_fee_to_the_withdrawal_floor() {
        let platform_version = PlatformVersion::latest();
        let floor = platform_version.system_limits.min_withdrawal_amount;

        assert_eq!(
            min_withdrawal_amount_with_core_fee(1, platform_version),
            floor + ASSET_UNLOCK_TX_SIZE as u64 * CREDITS_PER_DUFF
        );
        // The largest rate the wire format allows still sums exactly, without saturating.
        assert_eq!(
            min_withdrawal_amount_with_core_fee(u32::MAX, platform_version),
            floor + core_fee_in_credits(u32::MAX).expect("a u32 rate fits in credits")
        );
    }
}
