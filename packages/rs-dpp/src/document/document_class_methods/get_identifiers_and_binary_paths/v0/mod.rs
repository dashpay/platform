use std::collections::HashSet;
use crate::data_contract::DataContract;
use crate::document::{Document, IDENTIFIER_FIELDS};
use crate::ProtocolError;

impl Document {
    pub(super) fn get_identifiers_and_binary_paths_v0<'a>(
        data_contract: &'a DataContract,
        document_type_name: &'a str,
    ) -> Result<(HashSet<&'a str>, HashSet<&'a str>), ProtocolError> {
        let (mut identifiers_paths, binary_paths) =
            data_contract.get_identifiers_and_binary_paths(document_type_name)?;

        identifiers_paths.extend(IDENTIFIER_FIELDS);
        Ok((identifiers_paths, binary_paths))
    }
}