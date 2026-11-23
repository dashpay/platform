//! Document-type parser **generation 4** — protocol version 15 and later.
//!
//! Generation 4 is generation 3 plus the keep-history document lifecycle: a
//! keep-history type may allow deletion, may additionally declare
//! `canBeErased`, and may not carry a contested index. Its index grammar, the
//! ranked index-key ceilings and the prefix-overlap rule are generation 3's,
//! reused from that module rather than copied.
//!
//! Under full validation it also holds the `position`s of an object's members to
//! integers that follow one another from the object's lowest, which generation 3
//! does not read, and refuses typed-array properties whose element encodes to
//! zero bytes (a byte array pinned to `minItems: 0, maxItems: 0`). Like every
//! generation, it stores an object's members in the schema's map order.
//!
//! The generation reuses the complete current parser body with a generation-owned
//! admission flag. Earlier generations retain their refusal of deletable history.

use crate::data_contract::config::DataContractConfig;
#[cfg(feature = "validation")]
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
#[cfg(feature = "validation")]
use crate::data_contract::document_type::class_methods::consensus_or_protocol_data_contract_error;
#[cfg(feature = "validation")]
use crate::data_contract::document_type::property::DocumentPropertyType;
use crate::data_contract::document_type::v2::DocumentTypeV2;
use crate::data_contract::document_type::DocumentType;
#[cfg(feature = "validation")]
use crate::data_contract::errors::DataContractError;
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
    let v2 = super::v3::try_from_schema_generation_3(
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
    )?;

    #[cfg(feature = "validation")]
    if full_validation {
        validate_typed_array_non_zero_element_width(&v2, name)?;
    }

    Ok(v2)
}

/// Every typed array property's element must encode to at least one byte.
/// A fixed-size byte array pinned to zero bytes (`minItems: 0, maxItems: 0`)
/// writes neither a length prefix nor any payload bytes, so a two-byte count
/// decodes into `maxItems` (up to `max_typed_array_items`) separate values.
/// Read off the flattened properties, which reach a typed array nested in an
/// object too.
///
/// Full validation only, like `validate_typed_array_max_items`: a contract
/// registered at protocol version 14 may hold one and must stay readable.
#[cfg(feature = "validation")]
fn validate_typed_array_non_zero_element_width(
    document_type: &DocumentTypeV2,
    name: &str,
) -> Result<(), ProtocolError> {
    for (path, property) in document_type.flattened_properties() {
        let DocumentPropertyType::TypedArray(typed_array) = &property.property_type else {
            continue;
        };
        if typed_array.has_zero_encoded_width_element() {
            return Err(consensus_or_protocol_data_contract_error(
                DataContractError::InvalidContractStructure(format!(
                    "typed array property \"{path}\" of document type \"{name}\" declares a \
                     zero-width byte array element (minItems: 0, maxItems: 0): elements of a \
                     typed array must encode to at least one byte",
                )),
            ));
        }
    }
    Ok(())
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
    check_nested_member_positions: true,
    meta_schema_method_name: "DocumentType::try_from_schema_v4 (document_type_schema)",
    ..super::v3::PARSER_GENERATION
};

#[cfg(test)]
mod keep_history_tests;

#[cfg(test)]
mod nested_member_order_tests;

#[cfg(all(test, feature = "validation"))]
mod typed_array_element_width_tests;
