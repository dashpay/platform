use std::num::ParseIntError;

use crate::InvalidVectorSizeError;

fn byte_to_hex(byte: &u8) -> String {
    format!("{:02x}", byte)
}

pub fn encode_hex<T: Clone + Into<Vec<u8>>>(bytes: &T) -> String {
    let hex_vec: Vec<String> = bytes.clone().into().iter().map(byte_to_hex).collect();

    hex_vec.join("")
}

pub fn decode_hex(s: &str) -> Result<Vec<u8>, ParseIntError> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect()
}

#[derive(Debug)]
pub enum DecodeError {
    ParseIntError(ParseIntError),
    InvalidVectorSizeError(InvalidVectorSizeError),
}

impl From<InvalidVectorSizeError> for DecodeError {
    fn from(err: InvalidVectorSizeError) -> Self {
        Self::InvalidVectorSizeError(err)
    }
}

impl From<ParseIntError> for DecodeError {
    fn from(err: ParseIntError) -> Self {
        Self::ParseIntError(err)
    }
}

pub fn decode_hex_bls_sig(s: &str) -> Result<[u8; 96], DecodeError> {
    hex_to_array::<96>(s)
}

pub fn decode_hex_sha256(s: &str) -> Result<[u8; 32], DecodeError> {
    hex_to_array::<32>(s)
}

pub fn hex_to_array<const N: usize>(s: &str) -> Result<[u8; N], DecodeError> {
    let vec = decode_hex(s)?;
    Ok(vec_to_array::<N>(&vec)?)
}

pub fn vec_to_array<const N: usize>(vec: &[u8]) -> Result<[u8; N], InvalidVectorSizeError> {
    let mut v: [u8; N] = [0; N];
    // let mut v: T = T::default();
    if v.len() != vec.len() {
        return Err(InvalidVectorSizeError::new(v.len(), vec.len()));
    }
    for i in 0..vec.len() {
        if let Some(n) = vec.get(i) {
            v[i] = *n;
        } else {
            return Err(InvalidVectorSizeError::new(v.len(), vec.len()));
        }
    }
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- encode_hex --

    #[test]
    fn test_encode_hex_empty() {
        let bytes: Vec<u8> = vec![];
        assert_eq!(encode_hex(&bytes), "");
    }

    #[test]
    fn test_encode_hex_single_byte() {
        let bytes: Vec<u8> = vec![0xff];
        assert_eq!(encode_hex(&bytes), "ff");
    }

    #[test]
    fn test_encode_hex_multiple_bytes() {
        let bytes: Vec<u8> = vec![0xde, 0xad, 0xbe, 0xef];
        assert_eq!(encode_hex(&bytes), "deadbeef");
    }

    #[test]
    fn test_encode_hex_leading_zeros() {
        let bytes: Vec<u8> = vec![0x00, 0x01, 0x0a];
        assert_eq!(encode_hex(&bytes), "00010a");
    }

    #[test]
    fn test_encode_hex_all_zeros() {
        let bytes: Vec<u8> = vec![0x00, 0x00, 0x00];
        assert_eq!(encode_hex(&bytes), "000000");
    }

    // -- decode_hex --

    #[test]
    fn test_decode_hex_empty() {
        let result = decode_hex("").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_decode_hex_valid() {
        let result = decode_hex("deadbeef").unwrap();
        assert_eq!(result, vec![0xde, 0xad, 0xbe, 0xef]);
    }

    #[test]
    fn test_decode_hex_uppercase() {
        let result = decode_hex("DEADBEEF").unwrap();
        assert_eq!(result, vec![0xde, 0xad, 0xbe, 0xef]);
    }

    #[test]
    fn test_decode_hex_mixed_case() {
        let result = decode_hex("DeAdBeEf").unwrap();
        assert_eq!(result, vec![0xde, 0xad, 0xbe, 0xef]);
    }

    #[test]
    fn test_decode_hex_leading_zeros() {
        let result = decode_hex("00010a").unwrap();
        assert_eq!(result, vec![0x00, 0x01, 0x0a]);
    }

    #[test]
    fn test_decode_hex_invalid_chars() {
        let result = decode_hex("zzzz");
        assert!(result.is_err());
    }

    #[test]
    #[should_panic]
    fn test_decode_hex_odd_length_panics() {
        // Known issue: odd-length hex strings panic instead of returning Err
        // because s[i..i+2] goes out of bounds on the last byte.
        let _ = decode_hex("abc");
    }

    // -- round-trip encode/decode --

    #[test]
    fn test_hex_round_trip() {
        let original: Vec<u8> = vec![0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef];
        let hex = encode_hex(&original);
        let decoded = decode_hex(&hex).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_hex_round_trip_empty() {
        let original: Vec<u8> = vec![];
        let hex = encode_hex(&original);
        let decoded = decode_hex(&hex).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_hex_round_trip_all_byte_values() {
        let original: Vec<u8> = (0..=255).collect();
        let hex = encode_hex(&original);
        let decoded = decode_hex(&hex).unwrap();
        assert_eq!(original, decoded);
    }

    // -- hex_to_array --

    #[test]
    fn test_hex_to_array_valid_4_bytes() {
        let result = hex_to_array::<4>("deadbeef").unwrap();
        assert_eq!(result, [0xde, 0xad, 0xbe, 0xef]);
    }

    #[test]
    fn test_hex_to_array_valid_32_bytes() {
        let hex = "a".repeat(64); // 32 bytes encoded as 64 hex chars
        let result = hex_to_array::<32>(&hex).unwrap();
        assert_eq!(result.len(), 32);
        assert!(result.iter().all(|&b| b == 0xaa));
    }

    #[test]
    fn test_hex_to_array_wrong_size() {
        // Provide 4 bytes of hex (8 chars) but expect a 2-byte array
        let result = hex_to_array::<2>("deadbeef");
        assert!(result.is_err());
    }

    #[test]
    fn test_hex_to_array_invalid_hex() {
        let result = hex_to_array::<2>("zzzz");
        assert!(result.is_err());
    }

    // -- vec_to_array --

    #[test]
    fn test_vec_to_array_valid() {
        let vec = vec![1u8, 2, 3, 4];
        let result = vec_to_array::<4>(&vec).unwrap();
        assert_eq!(result, [1, 2, 3, 4]);
    }

    #[test]
    fn test_vec_to_array_too_short() {
        let vec = vec![1u8, 2];
        let result = vec_to_array::<4>(&vec);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.expected_size(), 4);
        assert_eq!(err.actual_size(), 2);
    }

    #[test]
    fn test_vec_to_array_too_long() {
        let vec = vec![1u8, 2, 3, 4, 5];
        let result = vec_to_array::<4>(&vec);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.expected_size(), 4);
        assert_eq!(err.actual_size(), 5);
    }

    #[test]
    fn test_vec_to_array_empty_to_zero() {
        let vec: Vec<u8> = vec![];
        let result = vec_to_array::<0>(&vec).unwrap();
        assert_eq!(result, [0u8; 0]);
    }

    #[test]
    fn test_vec_to_array_single_element() {
        let vec = vec![0xffu8];
        let result = vec_to_array::<1>(&vec).unwrap();
        assert_eq!(result, [0xff]);
    }

    // -- decode_hex_sha256 / decode_hex_bls_sig --

    #[test]
    fn test_decode_hex_sha256_valid() {
        let hex = "ab".repeat(32); // 32 bytes
        let result = decode_hex_sha256(&hex).unwrap();
        assert_eq!(result.len(), 32);
        assert!(result.iter().all(|&b| b == 0xab));
    }

    #[test]
    fn test_decode_hex_sha256_wrong_length() {
        let hex = "ab".repeat(16); // 16 bytes, not 32
        let result = decode_hex_sha256(&hex);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_hex_bls_sig_valid() {
        let hex = "cd".repeat(96); // 96 bytes
        let result = decode_hex_bls_sig(&hex).unwrap();
        assert_eq!(result.len(), 96);
        assert!(result.iter().all(|&b| b == 0xcd));
    }

    #[test]
    fn test_decode_hex_bls_sig_wrong_length() {
        let hex = "cd".repeat(48); // 48 bytes, not 96
        let result = decode_hex_bls_sig(&hex);
        assert!(result.is_err());
    }
}
