use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb_merk::tree_type::TreeType;

use crate::drive::document::index_level_tree_types::tree_type_for_sum_width;
use crate::error::drive::DriveError;
use crate::error::Error;

/// Extension trait for `DocumentTypeRef` that provides the tree type used
/// for primary key storage in Drive.
pub trait DocumentTypePrimaryKeyTreeType {
    /// Returns the `TreeType` used for the primary key storage tree.
    ///
    /// The primary key tree (key `[0]` under the document type path) stores
    /// document references keyed by document ID. The tree type depends on the
    /// document type's configuration, with the count and sum flag families
    /// composing orthogonally:
    ///
    /// | `range_summable` | `documents_summable` | `range_countable` | `documents_countable` | → TreeType |
    /// |---|---|---|---|---|
    /// | – | – | – | – | `NormalTree` |
    /// | – | – | – | ✓ | `CountTree` |
    /// | – | – | ✓ | (✓) | `ProvableCountTree` |
    /// | – | ✓ | – | – | `SumTree` |
    /// | ✓ | (✓) | – | – | `ProvableSumTree` |
    /// | – | ✓ | – | ✓ | `CountSumTree` |
    /// | – | ✓ | ✓ | (✓) | `ProvableCountSumTree` (per-node count, root-only sum) |
    /// | ✓ | (✓) | ✓ | (✓) | `ProvableCountProvableSumTree` (per-node BOTH) |
    /// | ✓ | (✓) | – | ✓ | `ProvableCountProvableSumTree` (upgrades count to per-node) |
    ///
    /// `ProvableCountSumTree` and `ProvableCountProvableSumTree` are
    /// distinct: the former commits per-node counts but only a
    /// root-level sum; the latter commits both per-node. The full
    /// dispatch matrix in the v1 arm makes the distinction explicit
    /// per-flag-combination.
    ///
    /// From protocol version 15 (generation 2) a type that keeps its sums in 128 bits
    /// (`DocumentTypeV2Getters::uses_big_sums`) gets the 128-bit counterpart of the tree in
    /// the table: `BigSumTree`, `ProvableBigSumTree`, `CountBigSumTree` and, for both
    /// `ProvableCountSumTree` and `ProvableCountProvableSumTree`,
    /// `ProvableCountProvableBigSumTree`.
    fn primary_key_tree_type(&self, platform_version: &PlatformVersion) -> Result<TreeType, Error>;
}

impl DocumentTypePrimaryKeyTreeType for DocumentTypeRef<'_> {
    fn primary_key_tree_type(&self, platform_version: &PlatformVersion) -> Result<TreeType, Error> {
        match platform_version
            .drive
            .methods
            .document
            .primary_key_tree_type
        {
            0 => {
                // v0: count-only dispatch (pre-sum). Preserved verbatim so
                // older platform versions return the exact same tree
                // variant they did before — sum flags are ignored here.
                if self.range_countable() {
                    Ok(TreeType::ProvableCountTree)
                } else if self.documents_countable() {
                    Ok(TreeType::CountTree)
                } else {
                    Ok(TreeType::NormalTree)
                }
            }
            1 => Ok(primary_key_tree_type_v1(*self)),
            // v2: v1's tree at the width the type keeps its sums in.
            2 => Ok(tree_type_for_sum_width(
                primary_key_tree_type_v1(*self),
                self.uses_big_sums(),
            )),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "DocumentTypeRef::primary_key_tree_type".to_string(),
                known_versions: vec![0, 1, 2],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::accessors::DocumentTypeV2Setters;
    use dpp::data_contract::document_type::DocumentType;
    use dpp::tests::json_document::json_document_to_contract_with_ids;
    use dpp::version::PlatformVersion;

    fn make_doc_type() -> DocumentType {
        let pv = PlatformVersion::latest();
        let contract = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/family/family-contract.json",
            None,
            None,
            false,
            pv,
        )
        .expect("contract");
        let dt = contract
            .document_type_for_name("person")
            .expect("person type");
        dt.to_owned_document_type()
    }

    #[test]
    fn default_is_normal_tree() {
        let dt = make_doc_type();
        let pv = PlatformVersion::latest();
        let result = dt.as_ref().primary_key_tree_type(pv).unwrap();
        assert_eq!(result, TreeType::NormalTree);
    }

    #[test]
    fn countable_is_count_tree() {
        let mut dt = make_doc_type();
        dt.set_documents_countable(true);
        let pv = PlatformVersion::latest();
        let result = dt.as_ref().primary_key_tree_type(pv).unwrap();
        assert_eq!(result, TreeType::CountTree);
    }

    #[test]
    fn blast_is_provable_count_tree() {
        let mut dt = make_doc_type();
        dt.set_range_countable(true);
        let pv = PlatformVersion::latest();
        let result = dt.as_ref().primary_key_tree_type(pv).unwrap();
        assert_eq!(result, TreeType::ProvableCountTree);
    }

    #[test]
    fn blast_takes_priority_over_countable() {
        let mut dt = make_doc_type();
        dt.set_documents_countable(true);
        dt.set_range_countable(true);
        let pv = PlatformVersion::latest();
        let result = dt.as_ref().primary_key_tree_type(pv).unwrap();
        assert_eq!(result, TreeType::ProvableCountTree);
    }
}

/// Generation 1 of the primary key tree's type, which generation 2 builds on.
fn primary_key_tree_type_v1(document_type: DocumentTypeRef) -> TreeType {
    // v1: count × sum composition over the expanded
    // grovedb TreeType set. The four flags map to nine
    // distinct cases per the dispatch table below — note
    // the **per-axis** distinction between provable
    // (per-node aggregation, range-queryable) and root-only
    // aggregation:
    //
    // | rc | dc | rs | ds | TreeType                          |
    // |----|----|----|----|-----------------------------------|
    // | F  | F  | F  | F  | NormalTree                        |
    // | F  | T  | F  | F  | CountTree                         |
    // | T  | _  | F  | F  | ProvableCountTree                 |
    // | F  | F  | F  | T  | SumTree                           |
    // | F  | F  | T  | _  | ProvableSumTree                   |
    // | F  | T  | F  | T  | CountSumTree                      |
    // | T  | _  | F  | T  | ProvableCountSumTree              |
    // | F  | T  | T  | _  | ProvableCountProvableSumTree (*)  |
    // | T  | _  | T  | _  | ProvableCountProvableSumTree      |
    //
    // (*) "count root-only + sum provable" has no
    // dedicated grovedb variant; we upgrade the count
    // side to per-node too. Same storage cost as
    // ProvableCountSumTree's count-half (per-node counts)
    // because ProvableCountProvableSumTree is the only
    // way to have a per-node sum aggregate.
    //
    // `ProvableCountProvableSumTree` is distinct from
    // `ProvableCountSumTree`: the latter carries per-node
    // counts but only a *root-level* sum.
    let rc = document_type.range_countable();
    let dc = document_type.documents_countable();
    let rs = document_type.range_summable();
    let ds = document_type.documents_summable().is_some();

    let count_provable = rc;
    let count_root_only = dc && !rc;
    let sum_provable = rs;
    let sum_root_only = ds && !rs;

    match (count_provable, count_root_only, sum_provable, sum_root_only) {
        // No flags
        (false, false, false, false) => TreeType::NormalTree,
        // Pure count
        (false, true, false, false) => TreeType::CountTree,
        (true, _, false, false) => TreeType::ProvableCountTree,
        // Pure sum
        (false, false, false, true) => TreeType::SumTree,
        (false, false, true, _) => TreeType::ProvableSumTree,
        // Combined
        (false, true, false, true) => TreeType::CountSumTree,
        (true, _, false, true) => TreeType::ProvableCountSumTree,
        (true, _, true, _) => TreeType::ProvableCountProvableSumTree,
        (false, true, true, _) => TreeType::ProvableCountProvableSumTree,
    }
}
