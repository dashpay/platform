use bincode::{Decode, Encode};

pub mod v1;

/// One price tier of short-lived document storage.
#[derive(Clone, Copy, Debug, Encode, Decode, Default, PartialEq, Eq)]
pub struct DocumentTtlFeeTier {
    /// The longest remaining lifetime, in seconds, this tier prices.
    pub max_ttl_seconds: u32,
    /// Credits charged per byte the document writes.
    pub credit_per_byte: u64,
}

/// Fees of documents whose document type declares a `ttl` (protocol version 14).
///
/// Such a document is stored without storage flags, refunds nothing when it goes, and is
/// deleted by the platform once its time to live has passed. Instead of the perpetual
/// storage price it pays, when it is written, for the time it will actually occupy the
/// state, plus the processing its deletion will cost:
///
/// * Every byte the document writes (the document, its index entries and its entry in the
///   expirations tree) costs the `credit_per_byte` of the first tier whose
///   `max_ttl_seconds` covers the remaining lifetime; tiers are ordered by
///   `max_ttl_seconds`. A lifetime longer than the last tier costs
///   `credit_per_byte_per_period` for every `pricing_period_seconds` it spans, rounded up.
///   The period is part of the schedule, not the node's epoch length, so a network with
///   short epochs (testnet, local networks) prices a lifetime like mainnet does.
/// * A document of a type whose `ttl` is shorter than `processing_route_below_epochs`
///   epochs of the network pays that amount into the current epoch's processing fee pool;
///   one of a longer `ttl` pays it into the storage fee distribution pool, like ordinary
///   storage. The route follows the declared `ttl`, not the lifetime left, so every write
///   of a document, and an estimate of it made at an earlier block time, takes one route.
/// * A document created with a `ttl` also prepays its deletion as processing:
///   `cleanup_base_processing_cost` plus `cleanup_processing_cost_per_index_level` for
///   every index level of its document type (each index counts its properties, times the
///   number of overlapping windows of a `timeRange` index).
///
/// Every earlier schedule carries the same group; it is unread there because the `ttl`
/// keyword does not parse before protocol version 14.
#[derive(Clone, Debug, Encode, Decode, Default, PartialEq, Eq)]
pub struct FeeDocumentTtlVersion {
    /// Short lifetimes, priced per byte, ordered by `max_ttl_seconds`.
    pub tiers: [DocumentTtlFeeTier; 5],
    /// Credits per byte per pricing period spanned for lifetimes longer than the last tier.
    pub credit_per_byte_per_period: u64,
    /// The length, in seconds, of the period `credit_per_byte_per_period` prices.
    pub pricing_period_seconds: u32,
    /// Documents of a type whose `ttl` is shorter than this many epochs pay their storage
    /// into the processing pool.
    pub processing_route_below_epochs: u16,
    /// Prepaid processing of a document's deletion, charged once when it is created.
    pub cleanup_base_processing_cost: u64,
    /// Prepaid processing per index level of the document type, charged once on creation.
    pub cleanup_processing_cost_per_index_level: u64,
}

#[cfg(test)]
mod tests {
    use super::v1::FEE_DOCUMENT_TTL_VERSION1;

    #[test]
    fn should_order_tiers_by_lifetime_with_non_decreasing_prices() {
        let group = FEE_DOCUMENT_TTL_VERSION1;
        for pair in group.tiers.windows(2) {
            assert!(pair[0].max_ttl_seconds < pair[1].max_ttl_seconds);
            assert!(pair[0].credit_per_byte <= pair[1].credit_per_byte);
        }
        // The first period past the last tier costs at least the last tier and outlasts
        // it, so a longer lifetime never costs less than a shorter one.
        let last = group.tiers[group.tiers.len() - 1];
        assert!(group.credit_per_byte_per_period >= last.credit_per_byte);
        assert!(group.pricing_period_seconds >= last.max_ttl_seconds);
    }
}
