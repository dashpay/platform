//! What a document whose type declares a `ttl` pays, from the fee schedule's `document_ttl`
//! group (see `FeeDocumentTtlVersion` for the schedule itself).

use crate::error::drive::DriveError;
use crate::error::fee::FeeError;
use crate::error::Error;
use crate::fees::op::EphemeralPricing;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::fee::Credits;
use dpp::prelude::TimestampMillis;
use platform_version::version::fee::FeeVersion;

/// How the bytes a document writes are priced when it has `remaining_lifetime_ms` left to
/// live: the first tier covering that lifetime, or past the last tier the schedule's price
/// per pricing period times the periods it spans, rounded up. A document of a type whose
/// `ttl_seconds` is shorter than the schedule's `processing_route_below_epochs` epochs of
/// `epoch_time_length_s` pays into the processing fees, one of a longer `ttl` into the
/// storage fee pool.
///
/// The price never decreases with the lifetime, which keeps an estimate made at an earlier
/// block time (a longer remaining lifetime) an upper bound of the price at execution. The
/// route depends on the declared `ttl` alone, so the estimate and the execution take the
/// same one: the fee increase a writer offers multiplies processing only.
pub fn document_ttl_pricing(
    remaining_lifetime_ms: u64,
    ttl_seconds: u32,
    epoch_time_length_s: u64,
    fee_version: &FeeVersion,
) -> Result<EphemeralPricing, Error> {
    let schedule = &fee_version.document_ttl;
    let tier_price = schedule
        .tiers
        .iter()
        .find(|tier| remaining_lifetime_ms <= u64::from(tier.max_ttl_seconds) * 1000)
        .map(|tier| tier.credit_per_byte);
    let credit_per_byte = match tier_price {
        Some(credit_per_byte) => credit_per_byte,
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
                )))?
        }
    };
    let storage_pool_from_seconds = u64::from(schedule.processing_route_below_epochs)
        .checked_mul(epoch_time_length_s)
        .ok_or(Error::Fee(FeeError::Overflow(
            "overflow computing the storage pool route of a document with a time to live",
        )))?;
    Ok(EphemeralPricing::DocumentTtl {
        credit_per_byte,
        storage_pool: u64::from(ttl_seconds) >= storage_pool_from_seconds,
    })
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
/// counts its properties, and an index bucketing time into overlapping windows counts them
/// once per window holding the document.
pub fn document_type_weighted_index_levels(document_type: DocumentTypeRef) -> u64 {
    document_type
        .indexes()
        .values()
        .map(|index| {
            let windows = index
                .time_range
                .as_ref()
                .map_or(1, |transform| transform.overlap_factor().max(1));
            (index.properties.len() as u64).saturating_mul(windows)
        })
        .fold(0u64, u64::saturating_add)
}

/// The processing a document of this type prepays for its deletion when it is created: the
/// schedule's base cost plus its cost per index level of the type.
pub fn document_expiration_cleanup_fee(
    document_type: DocumentTypeRef,
    fee_version: &FeeVersion,
) -> Result<Credits, Error> {
    let schedule = &fee_version.document_ttl;
    schedule
        .cleanup_processing_cost_per_index_level
        .checked_mul(document_type_weighted_index_levels(document_type))
        .and_then(|levels_cost| levels_cost.checked_add(schedule.cleanup_base_processing_cost))
        .ok_or(Error::Fee(FeeError::Overflow(
            "overflow pricing the deletion of a document with a time to live",
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
    const WEEK_S: u32 = 604_800;

    fn price_on(lifetime_ms: u64, ttl_seconds: u32, epoch_s: u64) -> (Credits, bool) {
        match document_ttl_pricing(
            lifetime_ms,
            ttl_seconds,
            epoch_s,
            &PlatformVersion::latest().fee_version,
        )
        .expect("prices")
        {
            EphemeralPricing::DocumentTtl {
                credit_per_byte,
                storage_pool,
            } => (credit_per_byte, storage_pool),
            other => panic!("expected a document ttl price, got {other:?}"),
        }
    }

    /// The price of a document created now: its whole time to live is left.
    fn price(lifetime_ms: u64) -> (Credits, bool) {
        let ttl_seconds = u32::try_from(lifetime_ms.div_ceil(1000)).expect("fits");
        price_on(lifetime_ms, ttl_seconds, MAINNET_EPOCH_S)
    }

    #[test]
    fn should_price_each_short_lifetime_by_its_tier() {
        let tiers = PlatformVersion::latest().fee_version.document_ttl.tiers;
        assert_eq!(price(1), (tiers[0].credit_per_byte, false));
        assert_eq!(price(HOUR_MS), (tiers[0].credit_per_byte, false));
        assert_eq!(price(HOUR_MS + 1), (tiers[1].credit_per_byte, false));
        assert_eq!(price(DAY_MS), (tiers[1].credit_per_byte, false));
        assert_eq!(price(DAY_MS + 1), (tiers[2].credit_per_byte, false));
        assert_eq!(price(2 * DAY_MS), (tiers[2].credit_per_byte, false));
        assert_eq!(price(2 * DAY_MS + 1), (tiers[3].credit_per_byte, false));
        assert_eq!(price(4 * DAY_MS), (tiers[3].credit_per_byte, false));
        assert_eq!(price(4 * DAY_MS + 1), (tiers[4].credit_per_byte, false));
        assert_eq!(price(7 * DAY_MS), (tiers[4].credit_per_byte, false));
    }

    #[test]
    fn should_price_longer_lifetimes_by_the_periods_they_span_rounded_up() {
        let schedule = &PlatformVersion::latest().fee_version.document_ttl;
        let per_period = schedule.credit_per_byte_per_period;
        let period_ms = u64::from(schedule.pricing_period_seconds) * 1000;
        // Past seven days but inside the first period: one period.
        assert_eq!(price(7 * DAY_MS + 1), (per_period, false));
        assert_eq!(price(period_ms), (per_period, false));
        assert_eq!(price(period_ms + 1), (2 * per_period, false));
        assert_eq!(price(365 * DAY_MS), (40 * per_period, true));
    }

    #[test]
    fn should_route_by_the_declared_time_to_live_in_epochs() {
        let epoch_s = u32::try_from(MAINNET_EPOCH_S).expect("fits");
        // A type of two epochs or more pays into the storage fee pool, whatever is left.
        assert!(price_on(1, 2 * epoch_s, MAINNET_EPOCH_S).1);
        assert!(price_on(u64::from(2 * epoch_s) * 1000, 2 * epoch_s, MAINNET_EPOCH_S).1);
        // A shorter type pays into the processing fees.
        assert!(!price_on(1, 2 * epoch_s - 1, MAINNET_EPOCH_S).1);
        // The network's epochs decide the route: two hours spans two testnet epochs.
        assert!(price_on(HOUR_MS, 7_200, TESTNET_EPOCH_S).1);
        assert!(!price_on(HOUR_MS, 7_200, MAINNET_EPOCH_S).1);
    }

    #[test]
    fn should_price_a_lifetime_alike_whatever_the_epoch_length() {
        // The price per byte comes from the schedule's own period: a network of one-hour
        // epochs prices a year like mainnet does.
        for lifetime_ms in [HOUR_MS, 3 * DAY_MS, 8 * DAY_MS, 30 * DAY_MS, 365 * DAY_MS] {
            let ttl_seconds = u32::try_from(lifetime_ms / 1000).expect("fits");
            assert_eq!(
                price_on(lifetime_ms, ttl_seconds, TESTNET_EPOCH_S).0,
                price_on(lifetime_ms, ttl_seconds, MAINNET_EPOCH_S).0,
                "{lifetime_ms} ms"
            );
        }
    }

    #[test]
    fn should_never_price_a_longer_lifetime_below_a_shorter_one() {
        let mut previous = 0;
        for lifetime_ms in (0..=400 * DAY_MS).step_by((HOUR_MS / 2) as usize) {
            let (credit_per_byte, _) = price_on(lifetime_ms, WEEK_S, MAINNET_EPOCH_S);
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
