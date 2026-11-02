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
///   512 KiB (`private_document_store_target_chunk_bytes`) at a chunk power from 4 to 11
///   (`private_document_store_min_chunk_power`, `private_document_store_max_chunk_power`), all
///   `None` before.
pub const SYSTEM_LIMITS_V5: SystemLimits = SystemLimits {
    max_compiled_contract_size: Some(1_048_576), // changed: contracts stored compiled take at most 1 MiB
    max_private_document_entry_size: Some(4096), // changed: a private document type's entries take at most 4096 bytes
    private_document_store_target_chunk_bytes: Some(512 * 1024), // changed: a private type's store keeps a compacted chunk near 512 KiB
    private_document_store_min_chunk_power: Some(4), // changed: and its chunk power is at least 4
    private_document_store_max_chunk_power: Some(11), // changed: and at most 11
    ..SYSTEM_LIMITS_V4
};
