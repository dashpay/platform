//! Safe integer conversions for the SQLite `INTEGER` column boundary.
//!
//! SQLite's `INTEGER` affinity is `i64`, but wallet types are `u64`; a
//! naive `as i64` wraps values ≥ `i64::MAX` to negatives and sign-extends
//! them back on read. Every durable boundary cast routes through one of
//! these helpers, which return a typed
//! [`WalletStorageError::IntegerOverflow`] on out-of-range input.

use crate::sqlite::error::WalletStorageError;

/// The target type whose range was exceeded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SafeCastTarget {
    #[error("i64")]
    I64,
    #[error("u64")]
    U64,
    #[error("u32")]
    U32,
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
/// [`WalletStorageError::IntegerOverflow`] when the database stored a
/// negative value.
pub fn i64_to_u64(field: &'static str, value: i64) -> Result<u64, WalletStorageError> {
    u64::try_from(value).map_err(|_| WalletStorageError::IntegerOverflow {
        field,
        // Surface the original bit pattern, not post-cast garbage.
        value: value as u64,
        target: SafeCastTarget::U64,
    })
}

/// Cast a stored `i64` column to `u32`, surfacing
/// [`WalletStorageError::IntegerOverflow`] when the value is negative or
/// exceeds `u32::MAX`. The single boundary helper for the readers that
/// map `INTEGER` columns (heights, account/address indices, nonces) back
/// to their `u32` Rust types.
///
/// `field` is a compile-time identifier (e.g.
/// `"core_sync_state.synced_height"`) naming the column so the resulting
/// error is actionable.
pub fn i64_to_u32(field: &'static str, value: i64) -> Result<u32, WalletStorageError> {
    u32::try_from(value).map_err(|_| WalletStorageError::IntegerOverflow {
        field,
        value: value as u64,
        target: SafeCastTarget::U32,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn i64_to_u32_happy_path() {
        assert_eq!(i64_to_u32("x", 0).unwrap(), 0);
        assert_eq!(i64_to_u32("x", u32::MAX as i64).unwrap(), u32::MAX);
    }

    #[test]
    fn i64_to_u32_overflow_high_and_negative() {
        assert!(matches!(
            i64_to_u32("h", i64::from(u32::MAX) + 1).unwrap_err(),
            WalletStorageError::IntegerOverflow {
                target: SafeCastTarget::U32,
                ..
            }
        ));
        assert!(matches!(
            i64_to_u32("h", -1).unwrap_err(),
            WalletStorageError::IntegerOverflow {
                target: SafeCastTarget::U32,
                ..
            }
        ));
    }

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
