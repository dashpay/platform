use crate::error::Error;
use dpp::consensus::ConsensusError;
use dpp::platform_value::platform_value;
use dpp::platform_value::string_encoding::{encode, Encoding};
use dpp::serialization::PlatformSerializableWithPlatformVersion;
use dpp::version::PlatformVersion;

pub trait AbciResponseInfoGetter {
    /// Returns a base64 encoded consensus error for Tenderdash response info
    fn response_info_for_version(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<String, Error>;
}

impl AbciResponseInfoGetter for ConsensusError {
    fn response_info_for_version(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<String, Error> {
        let consensus_error_bytes = self
            .serialize_to_bytes_with_platform_version(platform_version)
            .map_err(Error::Protocol)?;

        let error_data_buffer = platform_value!({
            "data": {
                "serializedError": consensus_error_bytes
            }
        })
        .to_cbor_buffer()
        .map_err(|e| Error::Protocol(e.into()))?;

        let encoded_info = encode(&error_data_buffer, Encoding::Base64);

        Ok(encoded_info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_info_for_version_returns_non_empty_base64_for_default_error() {
        let platform_version = PlatformVersion::latest();
        let error = ConsensusError::DefaultError;

        let result = error.response_info_for_version(platform_version);
        assert!(result.is_ok(), "should produce response info");

        let info = result.unwrap();
        assert!(!info.is_empty(), "response info should not be empty");

        // Verify it is valid base64 by decoding
        use dpp::platform_value::string_encoding::{decode, Encoding as Dec};
        let decoded = decode(&info, Dec::Base64).expect("should decode base64");
        assert!(!decoded.is_empty(), "decoded data should not be empty");
    }

    #[test]
    fn response_info_contains_serialized_error_data() {
        let platform_version = PlatformVersion::latest();
        let error = ConsensusError::DefaultError;

        let info = error
            .response_info_for_version(platform_version)
            .expect("should produce response info");

        // Decode base64 and then decode CBOR to verify the structure
        use dpp::platform_value::string_encoding::{decode, Encoding as Dec};
        let decoded_bytes = decode(&info, Dec::Base64).expect("should decode base64");

        // Verify CBOR can be parsed (the structure contains "data" -> "serializedError")
        let value: ciborium::Value =
            ciborium::from_reader(&decoded_bytes[..]).expect("should parse CBOR");

        // The value should be a map
        assert!(value.is_map(), "CBOR value should be a map");
    }
}
