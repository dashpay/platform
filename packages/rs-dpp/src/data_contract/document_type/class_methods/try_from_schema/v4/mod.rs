//! Document-type parser **generation 4**: protocol version 15 and later.
//!
//! Generation 3's grammar and parse, with full validation checking the schema natively
//! ([`super::native_schema`]) in place of checking it against the document meta-schema
//! through the `jsonschema` crate and compiling it with that crate. Registration then runs
//! no `jsonschema` code, and every document type it admits has a typed validation schema,
//! which documents are checked against natively from protocol version 15.
//!
//! Loading a stored contract runs no schema check, so it reads a document type exactly as
//! generation 3 does.

use super::v3::try_from_schema_generation_3;
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::DocumentType;
use crate::data_contract::{TokenConfiguration, TokenContractPosition};
use crate::validation::operations::ProtocolValidationOperation;
use crate::ProtocolError;
use platform_value::{Identifier, Value};
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

impl DocumentType {
    /// Generation 3's parse, with native schema checks.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::data_contract::document_type::class_methods) fn try_from_schema_v4(
        data_contract_id: Identifier,
        data_contract_system_version: u16,
        contract_config_version: u16,
        name: &str,
        schema: Value,
        schema_defs: Option<&BTreeMap<String, Value>>,
        token_configurations: &BTreeMap<TokenContractPosition, TokenConfiguration>,
        data_contact_config: &DataContractConfig,
        full_validation: bool,
        validation_operations: &mut impl Extend<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        try_from_schema_generation_3(
            data_contract_id,
            data_contract_system_version,
            contract_config_version,
            name,
            schema,
            schema_defs,
            token_configurations,
            data_contact_config,
            full_validation,
            true,
            validation_operations,
            platform_version,
        )
        .map(DocumentType::V2)
    }
}
