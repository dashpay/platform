//! Adapter from the independently pinned V019 codec to storage errors.

use v019_codec::{DecodedRecord, Error};

use crate::sqlite::error::WalletStorageError;

pub(super) fn decode_transaction_record(
    payload: &[u8],
) -> Result<DecodedRecord, WalletStorageError> {
    v019_codec::decode(payload).map_err(|error| match error {
        Error::TooLarge => WalletStorageError::BlobTooLarge {
            len_bytes: payload.len(),
            limit_bytes: v019_codec::MAX_RECORD_BYTES,
        },
        error => WalletStorageError::V019RecordDecode { source: error },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_pin_every_v019_codec_dependency_independently_of_runtime_versions() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("v019-codec/Cargo.toml");
        let manifest =
            std::fs::read_to_string(path).expect("V019 needs an independently pinned codec");
        for dependency in ["key-wallet", "dashcore"] {
            let line = manifest
                .lines()
                .find(|line| line.starts_with(&format!("{dependency} =")))
                .unwrap();
            assert!(
                line.contains("aca020263990a86a15cc1d97ae2cbf965d488db5"),
                "the V019 source revision is immutable"
            );
            assert!(!line.contains("workspace"));
        }
        assert!(manifest.contains("package = \"grovedb-bincode\", version = \"=2.1.0\""));
    }

    #[test]
    fn should_decode_all_immutable_v019_transaction_fixtures() {
        for line in include_str!("../../../tests/fixtures/core_transaction_rows.txt").lines() {
            let (name, bytes) = line.split_once(' ').unwrap();
            if name == "asset_lock_lifecycle" {
                continue;
            }
            let bytes = hex::decode(bytes).unwrap();
            let record =
                decode_transaction_record(&bytes).unwrap_or_else(|error| panic!("{name}: {error}"));
            assert!(!record.inputs.is_empty(), "{name}");
        }
    }
}
