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
///   `max_contract_schema_patterns` (32), `max_schema_pattern_compiled_size` (128 KiB),
///   `max_schema_pattern_bytes` (256), `max_contract_schema_pattern_bytes` (4 KiB) and
///   `max_schema_pattern_case_folded_code_points` (4096), through `validate_schema_patterns`
///   0. The numbers are unchanged from the earlier tables, which hold them for `check_tx`
///   alone.
/// * Retained-history erasure: one transition removes at most 100 revisions
///   (`max_document_revisions_erased_per_transition`, `None` before), as enforced by
///   `erase_document_for_contract_operations` 0.
/// * Enum violations: a document's `enum` violation lists at most 16 of the allowed values
///   (`max_enum_violation_listed_values`, `None` before), whose JSON takes at most 256 bytes
///   together (`max_enum_violation_listed_bytes`, `None` before), and counts the others, so
///   the error stays small, and readable by a client, whatever the schema's enum holds.
/// * Summed values: a summed property admits values past ±2^27
///   (`max_summed_value_magnitude_without_type_total`, `None` before) only when it declares a
///   `minimum` of at least 0 on a type that keeps the total of the property over all its
///   documents. Registration and update refuse any other; document create state validation 3
///   and replace state validation 2 hold that total in the range the type's sums hold.
/// * Sum width: a document type whose summed property is not declared within ±2^27
///   (`max_summed_value_magnitude_in_64_bit_sums`, `None` before) keeps every sum in a tree
///   with a 128-bit total, where earlier versions keep every sum in 64 bits.
/// * Document history layout: the first block of the version moves retained document histories
///   to the composite-key layout in batches of about 500 GroveDB operations
///   (`max_history_migration_batch_operations`, `None` before).
pub const SYSTEM_LIMITS_V5: SystemLimits = SystemLimits {
    max_summed_value_magnitude_without_type_total: Some(1 << 27), // changed: larger summed values need a minimum of 0 and the type's total
    max_summed_value_magnitude_in_64_bit_sums: Some(1 << 27), // changed: a type summing larger values keeps its sums in 128 bits
    max_document_revisions_erased_per_transition: Some(100), // one bounded lifecycle erasure chunk (note 41)
    max_enum_violation_listed_values: Some(16), // changed: an enum violation lists at most 16 allowed values
    max_enum_violation_listed_bytes: Some(256), // changed: whose JSON takes at most 256 bytes together
    max_compiled_contract_size: Some(1_048_576), // changed: contracts stored compiled take at most 1 MiB
    max_private_document_entry_size: Some(4096), // changed: a private document type's entries take at most 4096 bytes
    private_document_store_target_chunk_bytes: Some(1024 * 1024), // changed: a private type's store keeps a compacted chunk near 1 MiB
    private_document_store_min_chunk_power: Some(4), // changed: and its chunk power is at least 4
    private_document_store_max_chunk_power: Some(11), // changed: and at most 11
    max_private_document_entries_per_transition: Some(16), // changed: a PrivateDocumentCreateFromShieldedPool appends at most 16 entries
    max_history_migration_batch_operations: Some(500), // changed: the protocol version 15 move of retained document histories applies its writes in batches of about this many operations
    ..SYSTEM_LIMITS_V4
};
