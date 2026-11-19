use crate::version::system_limits::v4::SYSTEM_LIMITS_V4;
use crate::version::system_limits::SystemLimits;

/// System limits for protocol version 15. Relative to `SYSTEM_LIMITS_V4`, which protocol
/// version 14 runs on:
///
/// * Contracts stored compiled: a contract stored compiled (format version 2) takes at most
///   1 MiB (`max_compiled_contract_size`, `None` before). Registration, update and a token
///   configuration update refuse a contract over it, Drive refuses to store one, and proof
///   verification refuses a compiled contract over it.
/// * Private document types: an entry takes at most 4096 bytes
///   (`max_private_document_entry_size`, `None` before). Registration refuses a private type
///   declaring a larger `entrySize`. A private type's store keeps a compacted chunk near
///   1 MiB (`private_document_store_target_chunk_bytes`) at a chunk power from 4 to 11
///   (`private_document_store_min_chunk_power`, `private_document_store_max_chunk_power`), all
///   `None` before. A `PrivateDocumentCreateFromShieldedPool` appends at most 16 entries
///   (`max_private_document_entries_per_transition`, `None` before).
/// * Schema patterns: protocol version 15 is the first whose consensus reads
///   `max_contract_schema_patterns` (32) and `max_schema_pattern_compiled_size` (128 KiB),
///   through `validate_schema_patterns` 0. The numbers are unchanged from the earlier tables,
///   which hold them for `check_tx` alone.
/// * Retained-history erasure: one transition removes at most 100 revisions
///   (`max_document_revisions_erased_per_transition`, `None` before), as enforced by
///   `erase_document_for_contract_operations` 0.
pub const SYSTEM_LIMITS_V5: SystemLimits = SystemLimits {
    max_document_revisions_erased_per_transition: Some(100), // one bounded lifecycle erasure chunk (note 41)
    max_compiled_contract_size: Some(1_048_576), // changed: contracts stored compiled take at most 1 MiB
    max_private_document_entry_size: Some(4096), // changed: a private document type's entries take at most 4096 bytes
    private_document_store_target_chunk_bytes: Some(1024 * 1024), // changed: a private type's store keeps a compacted chunk near 1 MiB
    private_document_store_min_chunk_power: Some(4), // changed: and its chunk power is at least 4
    private_document_store_max_chunk_power: Some(11), // changed: and at most 11
    max_private_document_entries_per_transition: Some(16), // changed: a PrivateDocumentCreateFromShieldedPool appends at most 16 entries
    ..SYSTEM_LIMITS_V4
};
