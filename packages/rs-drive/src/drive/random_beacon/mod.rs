//! The values a block's random beacon draws (protocol version 15).
//!
//! The beacon is the threshold signature the validators' prevotes for a block combine into,
//! which no one can compute before the block is accepted. Every random value of the block is
//! drawn from it the same way, under a request key that names what the value is for, so two
//! requests of one block never share a seed:
//!
//! - a `sys.random` document property (`Drive::draw_random_document_properties`):
//!   `0x00 || contract_id || len16(document_type_name) || document_id || len16(property_path)`;
//! - a random token amount (`Drive::settle_random_token_amounts`):
//!   `0x01 || token_id || owner_id || identity_contract_nonce` (a big-endian `u64`).
//!
//! `len16(x)` is `x` after its length as a big-endian `u16`.

use dpp::data_contract::document_type::RandomRange;
use dpp::util::hash::hash_single;

/// The domain of a draw's seed, so a beacon never seeds anything else the same way.
const DRAW_DOMAIN: &[u8] = b"dash-platform-random-v1";

/// The first byte of the request key of a `sys.random` document property.
pub(crate) const DOCUMENT_PROPERTY_DRAW: u8 = 0;

/// The first byte of the request key of a random token amount.
pub(crate) const TOKEN_AMOUNT_DRAW: u8 = 1;

/// The value `beacon` draws for `request_key` from `range`: SHA-256 of the domain, the beacon
/// (prefixed by its length) and the request key, whose first 16 bytes, big-endian, reduced
/// modulo the range's size, offset the range's minimum. A range holds at most 2^64 values, so
/// the reduction favours no value by more than 2^-64 (`RandomRange` is checked when it is built,
/// so its minimum is never above its maximum). `None` only for a beacon longer than a length
/// prefix holds, which no caller passes.
pub(crate) fn draw_from_beacon_v0(
    beacon: &[u8],
    request_key: &[u8],
    range: RandomRange,
) -> Option<i128> {
    let mut preimage = Vec::with_capacity(DRAW_DOMAIN.len() + 2 + beacon.len() + request_key.len());
    preimage.extend_from_slice(DRAW_DOMAIN);
    push_with_length(&mut preimage, beacon)?;
    preimage.extend_from_slice(request_key);
    let seed = hash_single(&preimage);
    let mut bits = [0u8; 16];
    bits.copy_from_slice(seed.get(..16)?);
    range.value_at(u128::from_be_bytes(bits) % range.size())
}

/// Appends `part` after its length as a big-endian `u16`; `None` for a part longer than that
/// holds.
pub(crate) fn push_with_length(preimage: &mut Vec<u8>, part: &[u8]) -> Option<()> {
    let length = u16::try_from(part.len()).ok()?;
    preimage.extend_from_slice(&length.to_be_bytes());
    preimage.extend_from_slice(part);
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_seed_each_request_key_and_beacon_apart() {
        let range = RandomRange::new(0, u64::MAX.into()).expect("a range of 2^64 values");
        let beacon = [7u8; 96];
        assert_eq!(
            draw_from_beacon_v0(&beacon, &[1, 2, 3], range),
            draw_from_beacon_v0(&beacon, &[1, 2, 3], range)
        );
        assert_ne!(
            draw_from_beacon_v0(&beacon, &[1, 2, 3], range),
            draw_from_beacon_v0(&beacon, &[1, 2, 4], range)
        );
        assert_ne!(
            draw_from_beacon_v0(&beacon, &[1, 2, 3], range),
            draw_from_beacon_v0(&[8u8; 96], &[1, 2, 3], range)
        );
    }

    #[test]
    fn should_draw_inside_the_range() {
        let beacon = [7u8; 96];
        for key in 0u8..32 {
            let drawn =
                draw_from_beacon_v0(&beacon, &[key], RandomRange::new(10, 12).expect("a range"))
                    .expect("draws");
            assert!((10..=12).contains(&drawn));
        }
        assert_eq!(
            draw_from_beacon_v0(&beacon, &[0], RandomRange::new(5, 5).expect("a range")),
            Some(5)
        );
    }
}
