//! What a document whose type declares a `ttl` pays, from the fee schedule's `document_ttl`
//! group (see `FeeDocumentTtlVersion` for the schedule itself).

use crate::error::drive::DriveError;
use crate::error::fee::FeeError;
use crate::error::Error;
#[cfg(feature = "server")]
use crate::fees::op::EphemeralPricing;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::fee::Credits;
use dpp::prelude::TimestampMillis;
use platform_version::version::fee::FeeVersion;

/// How the bytes a document writes are priced when it has `remaining_lifetime_ms` left to
/// live: the first tier covering that lifetime, or past the last tier the schedule's price
/// per pricing period times the periods it spans, rounded up; paid out over the epochs of
/// `epoch_time_length_s` the lifetime spans, rounded up and at most `epochs_per_era`.
///
/// The price never decreases with the lifetime, which keeps an estimate made at an earlier
/// block time (a longer remaining lifetime) an upper bound of the price at execution. The
/// epochs only decide which epochs the pools pay the amount to.
#[cfg(feature = "server")]
pub fn document_ttl_pricing(
    remaining_lifetime_ms: u64,
    epoch_time_length_s: u64,
    epochs_per_era: u16,
    fee_version: &FeeVersion,
) -> Result<EphemeralPricing, Error> {
    let credit_per_byte = document_ttl_credit_per_byte(remaining_lifetime_ms, fee_version)?;
    let epoch_ms = epoch_time_length_s
        .checked_mul(1000)
        .filter(|epoch_ms| *epoch_ms > 0)
        .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
            "the epoch length must be a positive number of milliseconds",
        )))?;
    let lifetime_epochs = u16::try_from(remaining_lifetime_ms.div_ceil(epoch_ms))
        .unwrap_or(u16::MAX)
        .clamp(1, epochs_per_era.max(1));
    Ok(EphemeralPricing::DocumentTtl {
        credit_per_byte,
        lifetime_epochs,
    })
}

/// What a byte of a document with `remaining_lifetime_ms` left to live costs: the first tier
/// covering that lifetime, or past the last tier the schedule's price per pricing period times
/// the periods it spans, rounded up. Shared by [`document_ttl_pricing`] and
/// `drive::document::cost`.
pub fn document_ttl_credit_per_byte(
    remaining_lifetime_ms: u64,
    fee_version: &FeeVersion,
) -> Result<Credits, Error> {
    let schedule = &fee_version.document_ttl;
    let tier_price = schedule
        .tiers
        .iter()
        .find(|tier| remaining_lifetime_ms <= u64::from(tier.max_ttl_seconds) * 1000)
        .map(|tier| tier.credit_per_byte);
    match tier_price {
        Some(credit_per_byte) => Ok(credit_per_byte),
        None => {
            let period_ms = u64::from(schedule.pricing_period_seconds) * 1000;
            if period_ms == 0 {
                return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                    "the document ttl pricing period must be a positive number of seconds",
                )));
            }
            schedule
                .credit_per_byte_per_period
                .checked_mul(remaining_lifetime_ms.div_ceil(period_ms))
                .ok_or(Error::Fee(FeeError::Overflow(
                    "overflow pricing the periods a document with a time to live spans",
                )))
        }
    }
}

/// When a document created at `created_at` expires under a time to live of `ttl_seconds`:
/// the one definition of a document's expiry, which its entry in the documents expirations
/// tree is keyed by and every expiry check reads.
pub fn document_expires_at(
    created_at: TimestampMillis,
    ttl_seconds: u32,
) -> Result<TimestampMillis, Error> {
    created_at
        .checked_add(u64::from(ttl_seconds) * 1000)
        .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
            "a document's expiry time overflows",
        )))
}

/// The lifetime a document has left at `block_time_ms`: from then to its expiry, zero once
/// past. A document written without a known creation time (a worst-case estimate) is priced
/// for the whole time to live, the most it can have left.
pub fn document_remaining_lifetime_ms(
    created_at: Option<TimestampMillis>,
    ttl_seconds: u32,
    block_time_ms: TimestampMillis,
) -> Result<u64, Error> {
    match created_at {
        Some(created_at) => {
            Ok(document_expires_at(created_at, ttl_seconds)?.saturating_sub(block_time_ms))
        }
        None => Ok(u64::from(ttl_seconds) * 1000),
    }
}

/// The index levels a document of this type writes and its deletion removes: every index
/// counts its properties, and an index bucketing time or integers into overlapping windows
/// counts them once per window holding the document.
pub fn document_type_weighted_index_levels(document_type: DocumentTypeRef) -> u64 {
    document_type
        .indexes()
        .values()
        .map(|index| {
            let windows = index
                .time_range
                .as_ref()
                .map(|transform| transform.overlap_factor())
                .or(index
                    .integer_range
                    .as_ref()
                    .map(|transform| transform.overlap_factor()))
                .map_or(1, |windows| windows.max(1));
            (index.properties.len() as u64).saturating_mul(windows)
        })
        .fold(0u64, u64::saturating_add)
}

/// The processing a document of this type, `document_bytes` long when stored, prepays for its
/// deletion when it is created: the schedule's base cost, its cost per index level of the
/// type, and its cost per document byte.
pub fn document_expiration_cleanup_fee(
    document_type: DocumentTypeRef,
    document_bytes: u64,
    fee_version: &FeeVersion,
) -> Result<Credits, Error> {
    let schedule = &fee_version.document_ttl;
    schedule
        .cleanup_processing_cost_per_index_level
        .checked_mul(document_type_weighted_index_levels(document_type))
        .and_then(|levels_cost| levels_cost.checked_add(schedule.cleanup_base_processing_cost))
        .ok_or(Error::Fee(FeeError::Overflow(
            "overflow pricing the deletion of a document with a time to live",
        )))?
        .checked_add(document_expiration_cleanup_fee_for_bytes(
            document_bytes,
            fee_version,
        )?)
        .ok_or(Error::Fee(FeeError::Overflow(
            "overflow pricing the deletion of a document with a time to live",
        )))
}

/// The processing the deletion of `document_bytes` bytes of a document costs, prepaid for
/// the whole document when it is created and for the bytes a change adds to it.
pub fn document_expiration_cleanup_fee_for_bytes(
    document_bytes: u64,
    fee_version: &FeeVersion,
) -> Result<Credits, Error> {
    fee_version
        .document_ttl
        .cleanup_processing_cost_per_document_byte
        .checked_mul(document_bytes)
        .ok_or(Error::Fee(FeeError::Overflow(
            "overflow pricing the deletion of the bytes of a document with a time to live",
        )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_version::version::PlatformVersion;

    const MAINNET_EPOCH_S: u64 = 788_400;
    const TESTNET_EPOCH_S: u64 = 3_600;
    const HOUR_MS: u64 = 3_600_000;
    const DAY_MS: u64 = 86_400_000;

    const EPOCHS_PER_ERA: u16 = 40;

    fn price_on(lifetime_ms: u64, epoch_s: u64) -> (Credits, u16) {
        match document_ttl_pricing(
            lifetime_ms,
            epoch_s,
            EPOCHS_PER_ERA,
            &PlatformVersion::latest().fee_version,
        )
        .expect("prices")
        {
            EphemeralPricing::DocumentTtl {
                credit_per_byte,
                lifetime_epochs,
            } => (credit_per_byte, lifetime_epochs),
            other => panic!("expected a document ttl price, got {other:?}"),
        }
    }

    fn price(lifetime_ms: u64) -> (Credits, u16) {
        price_on(lifetime_ms, MAINNET_EPOCH_S)
    }

    #[test]
    fn should_price_each_short_lifetime_by_its_tier() {
        let tiers = PlatformVersion::latest().fee_version.document_ttl.tiers;
        assert_eq!(price(1).0, tiers[0].credit_per_byte);
        assert_eq!(price(HOUR_MS).0, tiers[0].credit_per_byte);
        assert_eq!(price(HOUR_MS + 1).0, tiers[1].credit_per_byte);
        assert_eq!(price(DAY_MS).0, tiers[1].credit_per_byte);
        assert_eq!(price(DAY_MS + 1).0, tiers[2].credit_per_byte);
        assert_eq!(price(2 * DAY_MS).0, tiers[2].credit_per_byte);
        assert_eq!(price(2 * DAY_MS + 1).0, tiers[3].credit_per_byte);
        assert_eq!(price(4 * DAY_MS).0, tiers[3].credit_per_byte);
        assert_eq!(price(4 * DAY_MS + 1).0, tiers[4].credit_per_byte);
        assert_eq!(price(7 * DAY_MS).0, tiers[4].credit_per_byte);
    }

    #[test]
    fn should_price_longer_lifetimes_by_the_periods_they_span_rounded_up() {
        let schedule = &PlatformVersion::latest().fee_version.document_ttl;
        let per_period = schedule.credit_per_byte_per_period;
        let period_ms = u64::from(schedule.pricing_period_seconds) * 1000;
        // Past seven days but inside the first period: one period.
        assert_eq!(price(7 * DAY_MS + 1).0, per_period);
        assert_eq!(price(period_ms).0, per_period);
        assert_eq!(price(period_ms + 1).0, 2 * per_period);
        assert_eq!(price(365 * DAY_MS).0, 40 * per_period);
    }

    #[test]
    fn should_pay_out_over_the_epochs_the_lifetime_spans() {
        let epoch_ms = MAINNET_EPOCH_S * 1000;
        // Less than an epoch still pays one epoch.
        assert_eq!(price(1).1, 1);
        assert_eq!(price(epoch_ms).1, 1);
        assert_eq!(price(epoch_ms + 1).1, 2);
        // A year of 365 days is exactly 40 epochs of 9.125 days.
        assert_eq!(price(365 * DAY_MS).1, 40);
        // Never past one era, whatever the network's epochs: testnet's hour-long epochs pay
        // a two-hour lifetime over two epochs and a year over one era.
        assert_eq!(price_on(2 * HOUR_MS, TESTNET_EPOCH_S).1, 2);
        assert_eq!(price_on(365 * DAY_MS, TESTNET_EPOCH_S).1, EPOCHS_PER_ERA);
    }

    #[test]
    fn should_price_a_lifetime_alike_whatever_the_epoch_length() {
        // The price per byte comes from the schedule's own period: a network of one-hour
        // epochs prices a year like mainnet does.
        for lifetime_ms in [HOUR_MS, 3 * DAY_MS, 8 * DAY_MS, 30 * DAY_MS, 365 * DAY_MS] {
            assert_eq!(
                price_on(lifetime_ms, TESTNET_EPOCH_S).0,
                price_on(lifetime_ms, MAINNET_EPOCH_S).0,
                "{lifetime_ms} ms"
            );
        }
    }

    #[test]
    fn should_never_price_a_longer_lifetime_below_a_shorter_one() {
        let mut previous = 0;
        for lifetime_ms in (0..=400 * DAY_MS).step_by((HOUR_MS / 2) as usize) {
            let (credit_per_byte, _) = price(lifetime_ms);
            assert!(
                credit_per_byte >= previous,
                "{lifetime_ms} ms costs {credit_per_byte}, less than {previous}"
            );
            previous = credit_per_byte;
        }
    }

    #[test]
    fn should_count_the_remaining_lifetime_from_creation() {
        let remaining = |created_at, block_time_ms| {
            document_remaining_lifetime_ms(created_at, 10, block_time_ms).expect("fits")
        };
        assert_eq!(remaining(Some(1_000), 1_000), 10_000);
        assert_eq!(remaining(Some(1_000), 6_000), 5_000);
        assert_eq!(remaining(Some(1_000), 60_000), 0);
        assert_eq!(remaining(None, 60_000), 10_000);
    }

    #[test]
    fn should_refuse_an_expiry_past_the_end_of_time() {
        assert!(document_expires_at(u64::MAX, 1).is_err());
        assert!(document_remaining_lifetime_ms(Some(u64::MAX), 1, 0).is_err());
    }
}
