/// Functions related to withdrawal documents
pub mod document;
#[cfg(all(feature = "server", any(test, feature = "structure")))]
pub(crate) mod structure;

mod calculate_current_withdrawal_limit;

/// How long an entry counts against (a withdrawal reservation) or toward (a credit inflow) the
/// daily withdrawal limit: 25 hours, a day plus an hour of slack. The two must expire on the
/// same schedule so a deposit and the withdrawal it funds cancel exactly over the whole window.
// Best to use a constant here and not a versioned item as this most likely will not change
pub const DAY_AND_A_HOUR_IN_MS: dpp::prelude::TimestampMillis = 90_000_000; //25 hours
/// Functions related to the Core credit pool balances the Core-anchored withdrawal limit reads
pub mod fetch_core_credit_pool_balances;
/// Functions related to what pooled withdrawals take out of Core's credit pool once mined
pub mod fetch_in_flight_withdrawal_amount;
/// Functions related to the Core credit pool balances the Core-anchored withdrawal limit reads
pub mod fetch_last_recorded_core_credit_pool_height;
/// Functions related to the per-block record of total credits the daily withdrawal limit reads
pub mod fetch_total_credits_in_platform_a_day_ago;
/// Functions and constants related to GroveDB paths
pub mod paths;
/// Functions related to the credit inflows of asset locks, dated by the Core block that mined them
pub mod record_asset_lock_credit_inflow;
/// Functions related to the Core blocks the Core-anchored withdrawal limit reads
pub mod record_core_credit_pool_block;
/// Functions related to the per-block record of credit inflows the daily withdrawal limit adds
pub mod record_credit_inflow;
/// Functions related to the per-block record of total credits the daily withdrawal limit reads
pub mod record_total_credits_history;
/// Functions related to withdrawal transactions
pub mod transaction;

use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::fee::Credits;
use dpp::prelude::TimestampMillis;

/// An asset lock Platform consumed before Core mined it, stored under its transaction id until
/// a Core block that holds it is read. Its credits count as a credit inflow only from then,
/// dated by that Core block, like those of an asset lock Core had already mined.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingAssetLockCreditInflow {
    /// The credits the asset lock minted into Platform.
    pub amount: Credits,
    /// The block time the credits were minted at, in milliseconds. Of several mints of one
    /// asset lock, the earliest: the daily withdrawal limit counts an inflow only when it was
    /// recorded after its day-old base snapshot, so the earliest is the strictest.
    pub recorded_at_time_ms: TimestampMillis,
    /// The Core chain locked height of the block the credits were minted in; the entry is
    /// dropped once Core is `core_credit_pool_window_max_blocks` past it.
    pub recorded_at_core_height: u32,
}

impl PendingAssetLockCreditInflow {
    /// The encoded size: the amount, the time and the Core height, big-endian.
    pub const ENCODED_LEN: usize = 8 + 8 + 4;

    /// Encodes the entry as the value of its item.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::ENCODED_LEN);
        bytes.extend_from_slice(&self.amount.to_be_bytes());
        bytes.extend_from_slice(&self.recorded_at_time_ms.to_be_bytes());
        bytes.extend_from_slice(&self.recorded_at_core_height.to_be_bytes());
        bytes
    }

    /// Decodes the value of an item written by [`Self::to_bytes`].
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let corrupted = || {
            Error::Drive(DriveError::CorruptedSerialization(
                "pending asset lock credit inflow is not 20 bytes".to_string(),
            ))
        };
        let bytes: &[u8; Self::ENCODED_LEN] = bytes.try_into().map_err(|_| corrupted())?;
        let (amount, rest) = bytes.split_at(8);
        let (recorded_at_time_ms, recorded_at_core_height) = rest.split_at(8);
        Ok(Self {
            amount: u64::from_be_bytes(amount.try_into().map_err(|_| corrupted())?),
            recorded_at_time_ms: u64::from_be_bytes(
                recorded_at_time_ms.try_into().map_err(|_| corrupted())?,
            ),
            recorded_at_core_height: u32::from_be_bytes(
                recorded_at_core_height
                    .try_into()
                    .map_err(|_| corrupted())?,
            ),
        })
    }
}

/// The key of a Core-dated credit inflow entry: the Core height it stops counting at, then the
/// block time it was recorded at, both big-endian, so a range from a height selects every entry
/// still counting there.
pub fn core_dated_credit_inflow_key(
    expires_at_core_height: u32,
    recorded_at_time_ms: TimestampMillis,
) -> Vec<u8> {
    let mut key = Vec::with_capacity(12);
    key.extend_from_slice(&expires_at_core_height.to_be_bytes());
    key.extend_from_slice(&recorded_at_time_ms.to_be_bytes());
    key
}

/// Splits a key written by [`core_dated_credit_inflow_key`] into the Core height the entry
/// stops counting at and the block time it was recorded at.
pub fn decode_core_dated_credit_inflow_key(key: &[u8]) -> Result<(u32, TimestampMillis), Error> {
    let corrupted = || {
        Error::Drive(DriveError::CorruptedSerialization(
            "core-dated credit inflow key is not 12 bytes".to_string(),
        ))
    };
    if key.len() != 12 {
        return Err(corrupted());
    }
    let (expires_at_core_height, recorded_at_time_ms) = key.split_at(4);
    Ok((
        u32::from_be_bytes(expires_at_core_height.try_into().map_err(|_| corrupted())?),
        u64::from_be_bytes(recorded_at_time_ms.try_into().map_err(|_| corrupted())?),
    ))
}
