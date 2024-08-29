use crate::data_contract::conversion::cbor::DataContractCborConversionMethodsV0;
use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use crate::data_contract::data_contract::DataContractV0;
use crate::util::cbor_value::CborCanonicalMap;

use crate::version::PlatformVersion;
use crate::ProtocolError;
use ciborium::Value as CborValue;
use platform_value::{Identifier, Value};
use std::convert::TryFrom;

impl DataContractCborConversionMethodsV0 for DataContractV0 {
    // TODO: Do we need to use this?
    fn from_cbor_with_id(
        cbor_bytes: impl AsRef<[u8]>,
        contract_id: Option<Identifier>,
        full_validation: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let mut data_contract = Self::from_cbor(cbor_bytes, full_validation, platform_version)?;
        if let Some(id) = contract_id {
            data_contract.id = id;
        }
        Ok(data_contract)
    }

    fn from_cbor(
        cbor_bytes: impl AsRef<[u8]>,
        full_validation: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let data_contract_cbor_value: CborValue = ciborium::de::from_reader(cbor_bytes.as_ref())
            .map_err(|_| {
                ProtocolError::DecodingError("unable to decode contract from cbor".to_string())
            })?;

        let data_contract_value: Value =
            Value::try_from(data_contract_cbor_value).map_err(ProtocolError::ValueError)?;

        Self::from_value(data_contract_value, full_validation, platform_version)
    }

    fn to_cbor(&self, platform_version: &PlatformVersion) -> Result<Vec<u8>, ProtocolError> {
        let value = self.to_value(platform_version)?;

        let mut buf: Vec<u8> = Vec::new();

        ciborium::ser::into_writer(&value, &mut buf)
            .map_err(|e| ProtocolError::EncodingError(e.to_string()))?;

        Ok(buf)
    }

    // /// Returns Data Contract as a Buffer
    // fn to_cbor_buffer(&self) -> Result<Vec<u8>, ProtocolError> {
    //     let mut object = self.to_object()?;
    //     if self.defs.is_none() {
    //         object.remove(property_names::DEFINITIONS)?;
    //     }
    //     object
    //         .to_map_mut()
    //         .unwrap()
    //         .sort_by_lexicographical_byte_ordering_keys_and_inner_maps();
    //
    //     // we are on version 0 here
    //     cbor_serializer::serializable_value_to_cbor(&object, Some(0))
    // }

    // TODO: Revisit
    fn to_cbor_canonical_map(
        &self,
        _platform_version: &PlatformVersion,
    ) -> Result<CborCanonicalMap, ProtocolError> {
        unimplemented!();

        // let mut contract_cbor_map = CborCanonicalMap::new();
        //
        // contract_cbor_map.insert(property_names::ID, self.id.to_buffer().to_vec());
        // contract_cbor_map.insert(property_names::SCHEMA, self.schema.as_str());
        // contract_cbor_map.insert(property_names::VERSION, self.version);
        // contract_cbor_map.insert(property_names::OWNER_ID, self.owner_id.to_buffer().to_vec());
        //
        // let docs = CborValue::serialized(&self.documents)
        //     .map_err(|e| ProtocolError::EncodingError(e.to_string()))?;
        //
        // contract_cbor_map.insert(property_names::DOCUMENTS, docs);
        //
        // if let Some(_defs) = &self.defs {
        //     contract_cbor_map.insert(
        //         property_names::DEFINITIONS,
        //         CborValue::serialized(&self.defs)
        //             .map_err(|e| ProtocolError::EncodingError(e.to_string()))?,
        //     );
        // };
        //
        // Ok(contract_cbor_map)
    }
}
