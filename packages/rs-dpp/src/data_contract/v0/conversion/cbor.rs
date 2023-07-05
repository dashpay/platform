use crate::data_contract::data_contract::DataContractV0;
use crate::ProtocolError;

impl DataContractV0 {
    #[cfg(feature = "cbor")]
    pub fn from_cbor_buffer(b: impl AsRef<[u8]>) -> Result<DataContractV0, ProtocolError> {
        Self::from_cbor(b)
    }
}