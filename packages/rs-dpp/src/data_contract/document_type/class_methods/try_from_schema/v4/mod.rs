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
//! does not read. Like every generation, it stores an object's members in the
//! schema's map order.
//!
//! The generation reuses the complete current parser body with a generation-owned
//! admission flag. Earlier generations retain their refusal of deletable history.
//!
//! Under full validation it also refuses a summed property the type lists as transient
//! ([`validate_no_transient_summed_property`]), after the generation-3 body has parsed the
//! type.

use crate::data_contract::config::DataContractConfig;
#[cfg(feature = "validation")]
use crate::data_contract::document_type::class_methods::consensus_or_protocol_data_contract_error;
#[cfg(feature = "validation")]
use crate::data_contract::document_type::property::is_transient;
use crate::data_contract::document_type::v2::DocumentTypeV2;
use crate::data_contract::document_type::DocumentType;
#[cfg(feature = "validation")]
use crate::data_contract::document_type::DocumentTypeRef;
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
/// contested index. Under full validation it refuses a type summing a property it lists as
/// transient.
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
    let document_type = super::v3::try_from_schema_generation_3(
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
        validate_no_transient_summed_property(&document_type, name)?;
    }
    Ok(document_type)
}

/// No property the type sums is transient. Drive writes a document's value of the property
/// into the type's sums when it stores the document, from the stored document's own
/// properties, and a transient value is never stored. Every summing declaration of a type
/// (`documentsSummable`, `documentsAverageable`, and an index's `summable` or `averageable`)
/// names one property (`apply_doctype_aggregates`); each is read here.
///
/// Full validation only: a contract stored with such a type still loads. An update re-parses
/// the whole contract under full validation, and neither the `transient` list nor what a type
/// sums can change on update, so a contract stored with such a type can no longer be updated;
/// no contract on mainnet or testnet has one.
#[cfg(feature = "validation")]
fn validate_no_transient_summed_property(
    document_type: &DocumentTypeV2,
    name: &str,
) -> Result<(), ProtocolError> {
    let summed_properties = document_type.documents_summable.iter().chain(
        document_type
            .indices
            .values()
            .filter_map(|index| index.summable.as_ref()),
    );
    for property in summed_properties {
        if is_transient(DocumentTypeRef::V2(document_type), property) {
            return Err(consensus_or_protocol_data_contract_error(
                DataContractError::InvalidContractStructure(format!(
                    "document type \"{name}\" sums \"{property}\", which it lists as \
                     transient: a transient value is never stored, and the type's sums read \
                     the stored value of each of its documents"
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
mod transient_summed_property_tests;
