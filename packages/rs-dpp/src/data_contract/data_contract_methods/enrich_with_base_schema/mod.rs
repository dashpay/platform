use crate::data_contract::DataContract;
use crate::ProtocolError;
use serde_json::Value as JsonValue;

pub const PREFIX_BYTE_0: u8 = 0;
pub const PREFIX_BYTE_1: u8 = 1;
pub const PREFIX_BYTE_2: u8 = 2;
pub const PREFIX_BYTE_3: u8 = 3;

impl DataContract {
    pub fn enrich_with_base_schema(
        &self,
        base_schema: &JsonValue,
        schema_id_byte_prefix: u8,
        exclude_properties: &[&str],
    ) -> Result<Self, ProtocolError> {
        match self {
            DataContract::V0(v0) => Ok(v0
                .enrich_with_base_schema(base_schema, schema_id_byte_prefix, exclude_properties)?
                .into()),
        }
    }
}
