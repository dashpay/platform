use crate::data_contract::{contract_config, property_names, DataContract, DataContractV0Methods};
use crate::prelude::Identifier;
use crate::util::cbor_value::CborCanonicalMap;
use crate::util::deserializer;
use crate::util::deserializer::SplitProtocolVersionOutcome;
use crate::{data_contract, ProtocolError};
use ciborium::Value as CborValue;

use crate::data_contract::data_contract::DataContractV0;
use crate::version::PlatformVersion;
use integer_encoding::VarInt;
use platform_value::btreemap_extensions::BTreeValueMapHelper;
use platform_value::Value;
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;

impl DataContractV0 {

}
