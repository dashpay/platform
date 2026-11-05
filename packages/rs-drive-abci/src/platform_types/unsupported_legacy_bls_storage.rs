//! Rejection of database formats that require an intermediate Platform upgrade.
//!
//! Each storage generation retired in Platform v5.1.0 by #5323 keeps a placeholder at its
//! historical path, and they all answer with this one error.

use bincode::error::DecodeError;

/// The error a retired storage generation's decoder returns, before reading its payload.
pub(crate) fn unsupported_legacy_bls_storage_error() -> DecodeError {
    DecodeError::Other(
        "Unsupported legacy BLS state format. Run Platform v4.0.0 on this database and let it successfully commit at least one block before upgrading.",
    )
}
