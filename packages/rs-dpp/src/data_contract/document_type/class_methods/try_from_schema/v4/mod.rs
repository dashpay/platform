//! Document-type parser **generation 4** — protocol version 15 and later.
//!
//! Generation 4 is generation 3 plus the keep-history document lifecycle: a
//! keep-history type may allow deletion, may additionally declare
//! `canBeErased`, and may not carry a contested index. Its index grammar, the
//! ranked index-key ceilings and the prefix-overlap rule are generation 3's,
//! reused from that module rather than copied.
//!
//! It exists as its own generation — rather than as a version gate inside the
//! shipped ones — because that is what keeps a historical block from ever
//! picking up grammar that did not exist when it was produced: the dispatcher
//! only routes here from `try_from_schema: 4`, and the grammar this module
//! admits is spelled out below as literals rather than looked up in a version
//! table.
//!
//! The parsing steps themselves are shared with the earlier generations in
//! [`super::common`].

use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::index::IndexGrammarAdmissions;
use crate::data_contract::document_type::v2::DocumentTypeV2;
use crate::data_contract::document_type::v3::DocumentTypeV3;
use crate::data_contract::document_type::DocumentType;
use crate::data_contract::{TokenConfiguration, TokenContractPosition};
use crate::validation::operations::ProtocolValidationOperation;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::{Identifier, Value};
use std::collections::BTreeMap;

use super::common;
use super::v3::ranked_prefix_overlap::validate_no_ranked_prefix_overlap;
use super::v3::RANKED_INDEX_KEY_LENGTH_CHECK;

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
) -> Result<DocumentTypeV3, ProtocolError> {
    // Read the aggregate and indexOnly keywords before the core parser
    // consumes `schema`.
    let aggregates = common::parse_doctype_aggregate_keywords(&schema, name)?;
    let index_only = common::parse_index_only_keyword(&schema)?;
    let can_be_erased = common::parse_can_be_erased_keyword(&schema)?;

    let v1 = common::parse_document_type_core(
        data_contract_id,
        data_contract_system_version,
        contract_config_version,
        name,
        schema,
        schema_defs,
        token_configurations,
        data_contact_config,
        full_validation,
        // Lets the core default omitted index terminals to `$ownerId` before
        // it builds the index structure, so the structure's level info is
        // born normalized (`apply_index_only` below validates the
        // already-normalized set).
        index_only,
        validation_operations,
        &common::ParserGeneration {
            // Generation 4 exists if and only if `document_type_schema` is 4:
            // CONTRACT_VERSIONS_V7 is the only table that selects this parser,
            // and it is the only table naming meta-schema v4. So every constant
            // here is a property of the generation, not of a platform version,
            // and none of them is read out of a table.
            document_type_schema_version: 4,
            // Meta-schema v4 carries the `keeps*History` flags forward, so they
            // are unconditionally part of this generation's grammar.
            admit_history: true,
            // Count indexes arrived at PV12; every version selecting this
            // generation is far past that boundary.
            admit_count_indexes: true,
            meta_schema_method_name: "DocumentType::try_from_schema_v4 (document_type_schema)",
            // The index grammar is generation 3's, read from the shared
            // generation → admission mapping so the registration-cost re-parse
            // can never drift from what this parser accepts. The ranked key
            // ceilings and the prefix-overlap rule are generation 3's as well.
            admit_ranked: IndexGrammarAdmissions::for_schema_generation(4).ranked,
            ranked_index_key_length_check: RANKED_INDEX_KEY_LENGTH_CHECK,
            ranked_index_structure_check: validate_no_ranked_prefix_overlap,
            admit_time_range: IndexGrammarAdmissions::for_schema_generation(4).time_range,
            admit_index_terminal: IndexGrammarAdmissions::for_schema_generation(4).terminal,
            admit_index_preallocated: IndexGrammarAdmissions::for_schema_generation(4).preallocated,
            admit_index_skip_if_absent: IndexGrammarAdmissions::for_schema_generation(4)
                .skip_if_absent,
        },
        platform_version,
    )?;

    let mut v2: DocumentTypeV2 = v1.into();
    common::apply_doctype_aggregates(&mut v2, aggregates, name)?;
    // After the aggregates: `apply_index_only` rejects the doctype-level
    // aggregate flags (they describe the primary-key tree, which an
    // indexOnly type does not have), so it has to see them already applied.
    common::apply_index_only(&mut v2, index_only, name)?;
    // The lifecycle keyword lives on the document type version this
    // generation produces.
    let mut v3: DocumentTypeV3 = v2.into();
    // Reads `canBeDeleted` off the parsed result rather than the raw schema, so
    // it sees the value resolved against the contract config default (`true`
    // when the key is omitted).
    common::apply_can_be_erased(&mut v3, can_be_erased, name)?;
    // A registration-time rule only: a contract that already carries this
    // combination was registered under an earlier protocol, and the structural
    // parse that loads stored contracts must keep reading it.
    if full_validation {
        common::reject_contested_keep_history(&v3, name)?;
    }

    Ok(v3)
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
        .map(DocumentType::V3)
    }
}

#[cfg(test)]
mod keep_history_tests;
