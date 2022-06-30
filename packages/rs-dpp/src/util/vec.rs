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
