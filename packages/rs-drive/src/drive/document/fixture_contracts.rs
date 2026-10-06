//! The test contracts `drive::document::layout` and `drive::document::cost`
//! are held to Drive with, and the adjustments random documents need to
//! insert into them.

use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::document::Document;
use dpp::document::DocumentV0Getters;
use dpp::platform_value::Value;
use std::collections::BTreeSet;

/// Contracts covering the index shapes Drive lays out: plain, unique and
/// compound indexes, history, countable and summable types and indexes,
/// ranked and chained indexes, time windows, indexOnly types with
/// terminals, flat and preallocated indexes, `skipIfAbsent` indexes
/// skipping below their first property on indexOnly and stored types, and
/// sibling indexes where one side misses a value beside a unique or a
/// `nullSearchable: false` index (their null flags follow each index's own
/// path), and `summableOffCountIndex` counters ranked by sum and average at
/// an earlier level.
pub(crate) const CONTRACTS: [&str; 23] = [
    "tests/supporting_files/contract/family/family-contract.json",
    "tests/supporting_files/contract/family/family-contract-fields-optional.json",
    "tests/supporting_files/contract/family/family-contract-countable.json",
    "tests/supporting_files/contract/family/family-contract-with-history.json",
    "tests/supporting_files/contract/dashpay/dashpay-contract.json",
    "tests/supporting_files/contract/references/references_with_contract_history.json",
    "tests/supporting_files/contract/restaurants/restaurants-contract.json",
    "tests/supporting_files/contract/trending/trending-contract.json",
    "tests/supporting_files/contract/trending/trending-sibling-contract.json",
    "tests/supporting_files/contract/yappr-likes/yappr-likes-contract.json",
    "tests/supporting_files/contract/yappr-likes/yappr-likes-preallocated-contract.json",
    "tests/supporting_files/contract/yappr-likes/yappr-likes-author-preallocated-contract.json",
    "tests/supporting_files/contract/yappr-feed/yappr-feed-contract.json",
    "tests/supporting_files/contract/index-only-scalar-terminal/index-only-scalar-terminal-contract.json",
    "tests/supporting_files/contract/tally/tally-contract.json",
    "tests/supporting_files/contract/tip-jar/tip-jar-contract.json",
    "tests/supporting_files/contract/grades/grades-contract.json",
    "tests/supporting_files/contract/grades/grades-ranked-contract.json",
    "tests/supporting_files/contract/grades/grades-compound-ranked-contract.json",
    "tests/supporting_files/contract/skip-if-absent/skip-likes-contract.json",
    "tests/supporting_files/contract/skip-if-absent/skip-posts-contract.json",
    "tests/supporting_files/contract/sibling-nulls/sibling-nulls-contract.json",
    "tests/supporting_files/contract/yappr-likes/yappr-likes-summable-off-count-index-contract.json",
];

/// Leaves out the optional properties of the type's unique indexes, so a
/// unique index gets entries with null values (two such documents share
/// a key, so they go in a tree by id).
pub(crate) fn leave_out_optional_unique_values(
    document: &mut Document,
    document_type: DocumentTypeRef,
) {
    let required = document_type.required_fields();
    for index in document_type
        .indexes()
        .values()
        .filter(|index| index.unique)
    {
        for property in &index.properties {
            if !property.name.starts_with('$') && !required.contains(&property.name) {
                document.properties_mut().remove(&property.name);
            }
        }
    }
}

/// Leaves out every skip property of the type's `skipIfAbsent` indexes, so
/// those indexes skip the document while the others write it.
pub(crate) fn leave_out_skip_properties(document: &mut Document, document_type: DocumentTypeRef) {
    for index in document_type.indexes().values() {
        for skip_property in &index.skip_if_absent_properties {
            document.properties_mut().remove(skip_property);
        }
    }
}

/// Leaves out the optional properties the type's indexes name: all of them,
/// or with `alternate` every other one in name order, so the index-level
/// walkers meet missing values beside present ones in sibling branches (a
/// random document fills every optional property).
pub(crate) fn leave_out_optional_indexed_values(
    document: &mut Document,
    document_type: DocumentTypeRef,
    alternate: bool,
) {
    let required = document_type.required_fields();
    let optional: BTreeSet<&String> = document_type
        .indexes()
        .values()
        .flat_map(|index| &index.properties)
        .map(|property| &property.name)
        .filter(|name| !name.starts_with('$') && !required.contains(*name))
        .collect();
    for (position, name) in optional.into_iter().enumerate() {
        if !alternate || position % 2 == 0 {
            document.properties_mut().remove(name);
        }
    }
}

/// Random integers ignore the schema's bounds, and a few of them overflow
/// a sum tree; give each summed property a small value instead (within
/// the bounds of every fixture: 1 to 7).
pub(crate) fn small_sums(document: &mut Document, document_type: DocumentTypeRef, seed: u64) {
    let summed = document_type
        .documents_summable()
        .map(str::to_string)
        .into_iter()
        .chain(
            document_type
                .indexes()
                .values()
                .filter_map(|index| index.summable.clone()),
        )
        .collect::<BTreeSet<_>>();
    for property in summed {
        if let Some(value) = document.properties_mut().get_mut(&property) {
            let small = match &*value {
                Value::U8(_) => Value::U8(seed as u8),
                Value::I8(_) => Value::I8(seed as i8),
                Value::U16(_) => Value::U16(seed as u16),
                Value::I16(_) => Value::I16(seed as i16),
                Value::U32(_) => Value::U32(seed as u32),
                Value::I32(_) => Value::I32(seed as i32),
                Value::U64(_) => Value::U64(seed),
                Value::I64(_) => Value::I64(seed as i64),
                other => other.clone(),
            };
            *value = small;
        }
    }
}
