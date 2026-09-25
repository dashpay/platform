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
/// live: the first tier covering that lifetime, or past the last tier the per-epoch price
/// times the epochs it spans, rounded up. A lifetime shorter than the schedule's
/// `processing_route_below_epochs` epochs pays into the processing fees, a longer one into the
/// storage fee pool.
///
/// The price never decreases with the lifetime, which keeps an estimate made at an earlier
/// block time (a longer remaining lifetime) an upper bound of the price at execution.
pub fn document_ttl_pricing(
    remaining_lifetime_ms: u64,
    epoch_time_length_s: u64,
    fee_version: &FeeVersion,
) -> Result<EphemeralPricing, Error> {
    let schedule = &fee_version.document_ttl;
    let epoch_ms = epoch_time_length_s
        .checked_mul(1000)
        .filter(|epoch_ms| *epoch_ms > 0)
        .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
            "the epoch length must be a positive number of milliseconds",
        )))?;
    let tier_price = schedule
        .tiers
        .iter()
        .find(|tier| remaining_lifetime_ms <= u64::from(tier.max_ttl_seconds) * 1000)
        .map(|tier| tier.credit_per_byte);
    let credit_per_byte = match tier_price {
        Some(credit_per_byte) => credit_per_byte,
        None => {
            let epochs = remaining_lifetime_ms.div_ceil(epoch_ms);
            schedule
                .credit_per_byte_per_epoch
                .checked_mul(epochs)
                .ok_or(Error::Fee(FeeError::Overflow(
                    "overflow pricing the epochs a document with a time to live spans",
                )))?
        }
    };
    let storage_pool_from_ms = u64::from(schedule.processing_route_below_epochs)
        .checked_mul(epoch_ms)
        .ok_or(Error::Fee(FeeError::Overflow(
            "overflow computing the storage pool route of a document with a time to live",
        )))?;
    Ok(EphemeralPricing::DocumentTtl {
        credit_per_byte,
        storage_pool: remaining_lifetime_ms >= storage_pool_from_ms,
    })
}

/// The lifetime a document has left at `block_time_ms`: its time to live counted from
/// `created_at`, zero once past. A document written without a known creation time (a
/// worst-case estimate) is priced for the whole time to live, the most it can have left.
pub fn document_remaining_lifetime_ms(
    created_at: Option<TimestampMillis>,
    ttl_seconds: u32,
    block_time_ms: TimestampMillis,
) -> u64 {
    let ttl_ms = u64::from(ttl_seconds) * 1000;
    match created_at {
        Some(created_at) => created_at
            .saturating_add(ttl_ms)
            .saturating_sub(block_time_ms),
        None => ttl_ms,
    }
}

/// When a document created at `created_at` expires under a time to live of `ttl_seconds`.
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

    const EPOCH_S: u64 = 788_400;
    const HOUR_MS: u64 = 3_600_000;
    const DAY_MS: u64 = 86_400_000;

    fn price(lifetime_ms: u64) -> (Credits, bool) {
        match document_ttl_pricing(lifetime_ms, EPOCH_S, &PlatformVersion::latest().fee_version)
            .expect("prices")
        {
            EphemeralPricing::DocumentTtl {
                credit_per_byte,
                storage_pool,
            } => (credit_per_byte, storage_pool),
            other => panic!("expected a document ttl price, got {other:?}"),
        }
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
    fn should_price_longer_lifetimes_by_the_epochs_they_span_rounded_up() {
        let per_epoch = PlatformVersion::latest()
            .fee_version
            .document_ttl
            .credit_per_byte_per_epoch;
        let epoch_ms = EPOCH_S * 1000;
        // Past seven days but inside the first epoch: one epoch.
        assert_eq!(price(7 * DAY_MS + 1), (per_epoch, false));
        assert_eq!(price(epoch_ms), (per_epoch, false));
        assert_eq!(price(epoch_ms + 1), (2 * per_epoch, false));
        // Two epochs and longer pay into the storage fee pool.
        assert_eq!(price(2 * epoch_ms - 1), (2 * per_epoch, false));
        assert_eq!(price(2 * epoch_ms), (2 * per_epoch, true));
        assert_eq!(price(365 * DAY_MS), (40 * per_epoch, true));
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
        assert_eq!(
            document_remaining_lifetime_ms(Some(1_000), 10, 1_000),
            10_000
        );
        assert_eq!(
            document_remaining_lifetime_ms(Some(1_000), 10, 6_000),
            5_000
        );
        assert_eq!(document_remaining_lifetime_ms(Some(1_000), 10, 60_000), 0);
        assert_eq!(document_remaining_lifetime_ms(None, 10, 60_000), 10_000);
    }

    #[test]
    fn should_refuse_an_epoch_of_no_length() {
        assert!(document_ttl_pricing(1, 0, &PlatformVersion::latest().fee_version).is_err());
    }
}
