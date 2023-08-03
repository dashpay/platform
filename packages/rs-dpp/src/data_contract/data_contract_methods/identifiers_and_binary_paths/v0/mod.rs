use crate::ProtocolError;
use platform_value::Value;
use std::collections::{BTreeMap, HashSet};

pub trait DataContractIdentifiersAndBinaryPathsMethodsV0 {
    fn get_identifiers_and_binary_paths(
        &self,
        document_type: &str,
    ) -> Result<(HashSet<&str>, HashSet<&str>), ProtocolError>;
    /// Returns the binary properties for the given document type
    /// Comparing to JS version of DPP, the binary_properties are not generated automatically
    /// if they're not present. It is up to the developer to use proper methods like ['DataContractV0::set_document_schema'] which
    /// automatically generates binary properties when setting the Json Schema
    // TODO: Naming is confusing. It's not clear, it sounds like it will return optional document properties
    //   but not None if document type is not present. Rename this
    fn get_optional_binary_properties(&self, doc_type: &str) -> Option<&BTreeMap<String, Value>>;
    /// Returns the binary properties for the given document type
    /// Comparing to JS version of DPP, the binary_properties are not generated automatically
    /// if they're not present. It is up to the developer to use proper methods like ['DataContractV0::set_document_schema'] which
    /// automatically generates binary properties when setting the Json Schema
    fn get_binary_properties(
        &self,
        doc_type: &str,
    ) -> Result<&BTreeMap<String, Value>, ProtocolError>;

    fn get_identifiers_and_binary_paths_owned<
        I: IntoIterator<Item = String> + Extend<String> + Default,
    >(
        &self,
        document_type: &str,
    ) -> Result<(I, I), ProtocolError>;
}
