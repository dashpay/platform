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
