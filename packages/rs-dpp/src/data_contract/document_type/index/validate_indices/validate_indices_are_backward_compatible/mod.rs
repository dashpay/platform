mod v0;

use std::collections::HashSet;
use std::collections::{BTreeMap, HashMap};
use crate::consensus::basic::data_contract::{
    DataContractHaveNewUniqueIndexError, DataContractInvalidIndexDefinitionUpdateError,
    DataContractUniqueIndicesChangedError,
};
use crate::consensus::basic::BasicError;
use crate::data_contract::document_type::{Index, IndexProperty};
use crate::util::json_schema::JsonSchemaExt;
use crate::util::json_value::JsonValueExt;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use anyhow::anyhow;
use platform_value::Value;
use platform_version::version::PlatformVersion;

type IndexName = String;
type DocumentType = String;
type JsonSchema = serde_json::Value;

impl Index {

    /// Validates if the indices of the document are backward compatible.
    ///
    /// # Arguments
    ///
    /// * `existing_documents` - An iterator over the existing document types and their JSON schemas.
    /// * `new_documents` - An iterator over the new document types and their JSON schemas.
    /// * `platform_version` - The platform version.
    ///
    /// # Returns
    ///
    /// * `Result<SimpleConsensusValidationResult, ProtocolError>` - A validation result which indicates if the indices are backward compatible.
    pub(in crate::data_contract::document_type) fn validate_indices_are_backward_compatible<'a>(
        existing_documents: impl IntoIterator<Item=(&'a DocumentType, &'a Value)>,
        new_documents: impl IntoIterator<Item=(&'a DocumentType, &'a Value)>,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version.dpp.contract_versions.index_versions.validation.validate_indices_are_backward_compatible {
            0 => Index::validate_indices_are_backward_compatible_v0(existing_documents, new_documents, platform_version),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Index::validate_indices_are_backward_compatible".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

