use crate::drive::RootTree;
use dpp::prelude::TimestampMillis;

/// The key of the documents expirations tree under `Misc`. Protocol version 14.
pub const DOCUMENTS_EXPIRATIONS_KEY: &[u8; 1] = b"E";

/// The path of the documents expirations tree
pub fn documents_expirations_path() -> [&'static [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::Misc),
        DOCUMENTS_EXPIRATIONS_KEY,
    ]
}

/// The path of the documents expirations tree, as a vector
pub fn documents_expirations_path_vec() -> Vec<Vec<u8>> {
    vec![
        Into::<&[u8; 1]>::into(RootTree::Misc).to_vec(),
        DOCUMENTS_EXPIRATIONS_KEY.to_vec(),
    ]
}

/// The key of the tree holding the documents that expire at `expires_at_ms`: the time as a
/// big endian u64, so the trees sort by time.
pub fn encode_expiration_time(expires_at_ms: TimestampMillis) -> [u8; 8] {
    expires_at_ms.to_be_bytes()
}

/// The time a key of the documents expirations tree stands for, `None` if it is not one.
pub fn decode_expiration_time(key: &[u8]) -> Option<TimestampMillis> {
    Some(TimestampMillis::from_be_bytes(key.try_into().ok()?))
}

/// The path of the tree holding the documents that expire at `expires_at_ms`
pub fn documents_expirations_at_time_path_vec(expires_at_ms: TimestampMillis) -> Vec<Vec<u8>> {
    vec![
        Into::<&[u8; 1]>::into(RootTree::Misc).to_vec(),
        DOCUMENTS_EXPIRATIONS_KEY.to_vec(),
        encode_expiration_time(expires_at_ms).to_vec(),
    ]
}
