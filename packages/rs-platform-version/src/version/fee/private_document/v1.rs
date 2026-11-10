use crate::version::fee::private_document::FeePrivateDocumentVersion;

/// Introduced with protocol version 15 (5.1), the first version that admits
/// `PrivateDocumentCreateFromShieldedPool`; every fee schedule references it, and none before
/// 15 reads it.
///
/// An entry pays 10,000,000 credits (0.0001 DASH) on top of its storage, so a million entries
/// a day, which every wallet of the type downloads and scans, cost 100 DASH a day in this fee
/// alone, beside the shielded fee every transition pays.
///
/// GroveDB's append into a store meters the entry, Drive's 16-byte header and 1 to 3 bytes of
/// its own per entry, the same at every position (the compacting append included) for every
/// chunk power; 32 bytes cover them with margin. Drive's
/// `should_charge_a_fixed_fee_covering_the_metered_cost_at_every_chunk_power` holds both to it.
pub const FEE_PRIVATE_DOCUMENT_VERSION1: FeePrivateDocumentVersion = FeePrivateDocumentVersion {
    entry_processing_fee: 10_000_000,
    entry_overhead_bytes: 32,
};
