//! The contract version item: the value stored at [`CONTRACT_VERSION_KEY`] under a
//! contract's root subtree from protocol version 14, and what `getDataContractsLatestVersions`
//! reads and proves when the contracts themselves are not requested.

use crate::drive::contract::paths::CONTRACT_VERSION_KEY;

/// The size of a contract version item's value: the version as four big-endian bytes.
pub const CONTRACT_VERSION_ITEM_SIZE: usize = 4;

/// Encodes a contract version as the value of its version item.
pub fn encode_contract_version(version: u32) -> Vec<u8> {
    version.to_be_bytes().to_vec()
}

/// Decodes the value of a contract version item, `None` when it is not four bytes long.
pub fn decode_contract_version(bytes: &[u8]) -> Option<u32> {
    let bytes: [u8; CONTRACT_VERSION_ITEM_SIZE] = bytes.try_into().ok()?;
    Some(u32::from_be_bytes(bytes))
}

/// The key of the version item as a one-byte slice, for path queries and raw reads.
pub const fn contract_version_key() -> [u8; 1] {
    [CONTRACT_VERSION_KEY]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_round_trip_a_contract_version() {
        for version in [0, 1, 7, u32::MAX] {
            let encoded = encode_contract_version(version);
            assert_eq!(encoded.len(), CONTRACT_VERSION_ITEM_SIZE);
            assert_eq!(decode_contract_version(&encoded), Some(version));
        }
    }

    #[test]
    fn should_reject_a_value_that_is_not_four_bytes() {
        assert_eq!(decode_contract_version(&[]), None);
        assert_eq!(decode_contract_version(&[0, 0, 1]), None);
        assert_eq!(decode_contract_version(&[0, 0, 0, 0, 1]), None);
    }
}
