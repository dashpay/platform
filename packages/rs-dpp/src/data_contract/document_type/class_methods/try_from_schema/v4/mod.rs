//! Document-type parser **generation 4** — protocol version 15 and later.
//!
//! Generation 4 is generation 3 plus the keep-history document lifecycle: a
//! keep-history type may allow deletion, may additionally declare
//! `canBeErased`, and may not carry a contested index. Its index grammar, the
//! ranked index-key ceilings and the prefix-overlap rule are generation 3's,
//! reused from that module rather than copied.
//!
//! The generation reuses the complete current parser body with a generation-owned
//! admission flag. Earlier generations retain their refusal of deletable history.

use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::v2::DocumentTypeV2;
use crate::data_contract::document_type::DocumentType;
use crate::data_contract::{TokenConfiguration, TokenContractPosition};
use crate::validation::operations::ProtocolValidationOperation;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::{Identifier, Value};
use std::collections::BTreeMap;

use super::common;

/// Parses a document type schema through the generation-4 grammar: the whole
/// generation-3 grammar plus the keep-history document lifecycle keywords.
///
/// This parser is only reachable from protocol version 15+ (via
/// CONTRACT_VERSIONS_V7).
///
/// Generation 4 admits `documentsKeepHistory: true` together with
/// `canBeDeleted: true`: a delete on such a type removes the document from
/// ordinary reads while its retained revisions stay readable. It also admits
/// the `canBeErased` keyword, which additionally allows a deleted document's
/// revisions to be purged, and it refuses a keep-history type that carries a
/// contested index.
#[allow(clippy::too_many_arguments)]
fn try_from_schema_generation_4(
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
) -> Result<DocumentTypeV2, ProtocolError> {
    super::v3::try_from_schema_generation_3(
        data_contract_id,
        data_contract_system_version,
        contract_config_version,
        name,
        schema,
        schema_defs,
        token_configurations,
        data_contact_config,
        full_validation,
        validation_operations,
        &PARSER_GENERATION,
        platform_version,
    )
}

impl DocumentType {
    /// Dispatches to this module's generation-4 parser and wraps the result.
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
        try_from_schema_generation_4(
            data_contract_id,
            data_contract_system_version,
            contract_config_version,
            name,
            schema,
            schema_defs,
            token_configurations,
            data_contact_config,
            full_validation,
            validation_operations,
            platform_version,
        )
        .map(DocumentType::V2)
    }
}

const PARSER_GENERATION: common::ParserGeneration = common::ParserGeneration {
    admit_lifecycle: true,
    meta_schema_method_name: "DocumentType::try_from_schema_v4 (document_type_schema)",
    ..super::v3::PARSER_GENERATION
};

#[cfg(test)]
mod keep_history_tests;
