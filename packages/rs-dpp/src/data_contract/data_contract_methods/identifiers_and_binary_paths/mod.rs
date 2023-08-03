mod v0;

use crate::data_contract::DataContract;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::Value;
use serde_json::Value as JsonValue;
use std::collections::{BTreeMap, HashSet};
pub use v0::*;

impl DataContractIdentifiersAndBinaryPathsMethodsV0 for DataContract {
    fn get_identifiers_and_binary_paths(
        &self,
        document_type: &str,
    ) -> Result<(HashSet<&str>, HashSet<&str>), ProtocolError> {
        match self {
            DataContract::V0(v0) => v0.get_identifiers_and_binary_paths(document_type),
        }
    }

    fn get_optional_binary_properties(&self, doc_type: &str) -> Option<&BTreeMap<String, Value>> {
        match self {
            DataContract::V0(v0) => v0.get_optional_binary_properties(doc_type),
        }
    }

    fn get_binary_properties(
        &self,
        doc_type: &str,
    ) -> Result<&BTreeMap<String, Value>, ProtocolError> {
        match self {
            DataContract::V0(v0) => v0.get_binary_properties(doc_type),
        }
    }

    fn get_identifiers_and_binary_paths_owned<
        I: IntoIterator<Item = String> + Extend<String> + Default,
    >(
        &self,
        document_type: &str,
    ) -> Result<(I, I), ProtocolError> {
        match self {
            DataContract::V0(v0) => v0.get_identifiers_and_binary_paths_owned(document_type),
        }
    }
}
