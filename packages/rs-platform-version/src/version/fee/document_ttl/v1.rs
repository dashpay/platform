use crate::version::fee::document_ttl::{DocumentTtlFeeTier, FeeDocumentTtlVersion};

/// Introduced with protocol version 14 (4.2), the first version that parses the document
/// type `ttl` keyword; every fee schedule references it, and none before 14 reads it.
///
/// The per-byte prices are pro rata of the first year of perpetual storage: 27,000 credits
/// per byte (`storage_disk_usage_credit_per_byte`), of which the distribution table pays 5%
/// out in the first year (40 epochs of about 9.125 days), so about 3.7 credits per byte per
/// day and 33.75 per byte per epoch, rounded up. A document of a one-year `ttl` therefore
/// pays about what a permanent document deleted after a year keeps paying net of its
/// refund.
pub const FEE_DOCUMENT_TTL_VERSION1: FeeDocumentTtlVersion = FeeDocumentTtlVersion {
    tiers: [
        DocumentTtlFeeTier {
            max_ttl_seconds: 3_600, // up to one hour
            credit_per_byte: 1,
        },
        DocumentTtlFeeTier {
            max_ttl_seconds: 86_400, // up to one day
            credit_per_byte: 4,
        },
        DocumentTtlFeeTier {
            max_ttl_seconds: 172_800, // up to two days
            credit_per_byte: 8,
        },
        DocumentTtlFeeTier {
            max_ttl_seconds: 345_600, // up to four days
            credit_per_byte: 15,
        },
        DocumentTtlFeeTier {
            max_ttl_seconds: 604_800, // up to seven days
            credit_per_byte: 26,
        },
    ],
    credit_per_byte_per_epoch: 34,
    processing_route_below_epochs: 2,
    // Measured on protocol version 14 (drive `delete_document_for_contract` of a document of
    // a type with a `ttl`, averaged over ten documents): about 1.53M credits of processing with
    // one index level, 1.94M with two and 2.39M with four. Base plus per level covers each.
    cleanup_base_processing_cost: 1_200_000,
    cleanup_processing_cost_per_index_level: 400_000,
};
