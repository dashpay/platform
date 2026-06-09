//! Safe integer conversions for the SQLite `INTEGER` column boundary.
//!
//! SQLite's `INTEGER` affinity is `i64`. Rust's wallet types (credits
//! balances, durations cast to milliseconds, monotonic-max heights,
//! token balances) are `u64`. Naively `as i64` casting wraps values
//! ≥ `i64::MAX` to negative numbers and silently sign-extends them
//! back to large `u64` on read.
//!
//! Every cross-boundary cast in the writer / reader paths runs through
//! one of these helpers and produces a typed
//! [`WalletStorageError::IntegerOverflow`] on out-of-range input.
//! `clippy::cast_possible_wrap` and `cast_sign_loss` warnings stay
//! allowed crate-wide because many in-crate casts are bounded (e.g.
//! `u8` tags, `u32` indices ≤ `i32::MAX`); the contract is that
//! *durable boundary casts* go through this module.

use crate::sqlite::error::WalletStorageError;

/// The target type whose range was exceeded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SafeCastTarget {
    #[error("i64")]
    I64,
    #[error("u64")]
    U64,
}

/// Cast `value: u64` to `i64`, surfacing
/// [`WalletStorageError::IntegerOverflow`] when the value exceeds
/// `i64::MAX`.
///
/// `field` is a compile-time identifier (e.g. `"asset_locks.amount_duffs"`)
/// naming the column so the resulting error is actionable.
pub fn u64_to_i64(field: &'static str, value: u64) -> Result<i64, WalletStorageError> {
    i64::try_from(value).map_err(|_| WalletStorageError::IntegerOverflow {
        field,
        value,
        target: SafeCastTarget::I64,
    })
}

/// Cast `value: i64` to `u64`, surfacing
/// [`WalletStorageError::IntegerOverflow`] when the database stored
/// a negative value (possible if a previous build wrote a wrapped
/// value before this helper existed).
pub fn i64_to_u64(field: &'static str, value: i64) -> Result<u64, WalletStorageError> {
    u64::try_from(value).map_err(|_| WalletStorageError::IntegerOverflow {
        field,
        // For negative inputs the wrapped representation is what we
        // surface — the operator looks at the original bits, not the
        // post-cast u64 garbage.
        value: value as u64,
        target: SafeCastTarget::U64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u64_to_i64_happy_path() {
        assert_eq!(u64_to_i64("x", 0).unwrap(), 0);
        assert_eq!(u64_to_i64("x", i64::MAX as u64).unwrap(), i64::MAX);
    }

    #[test]
    fn u64_to_i64_overflow() {
        let err = u64_to_i64("balance", u64::MAX).unwrap_err();
        assert!(matches!(
            err,
            WalletStorageError::IntegerOverflow {
                field: "balance",
                value: u64::MAX,
                target: SafeCastTarget::I64,
            }
        ));
    }

    #[test]
    fn i64_to_u64_happy_path() {
        assert_eq!(i64_to_u64("x", 0).unwrap(), 0);
        assert_eq!(i64_to_u64("x", i64::MAX).unwrap(), i64::MAX as u64);
    }

    #[test]
    fn i64_to_u64_overflow_on_negative() {
        let err = i64_to_u64("balance", -1).unwrap_err();
        assert!(matches!(
            err,
            WalletStorageError::IntegerOverflow {
                field: "balance",
                target: SafeCastTarget::U64,
                ..
            }
        ));
    }
}
