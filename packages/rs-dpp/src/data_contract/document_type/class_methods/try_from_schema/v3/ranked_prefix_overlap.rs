//! The generation-3 [`super::common::RankedIndexStructureCheck`]: the cross-index
//! structural rule the ranked grammar adds on top of the shared parse core.
//!
//! Its own file for the same reason the mode-detection versions have their
//! own files: the rule is one frozen unit of generation-3 grammar, and a
//! later generation that needs a different structural rule supplies its own
//! callback instead of editing this one.

use crate::data_contract::document_type::class_methods::consensus_or_protocol_data_contract_error;
use crate::data_contract::document_type::index::Index;
use crate::data_contract::errors::DataContractError;
use crate::ProtocolError;
use std::collections::BTreeMap;

/// Rejects the one compound-ranked shape the storage layer cannot lay
/// out: a compound ranked index whose **full leading prefix** also
/// terminates a separate countable and/or summable index.
///
/// A ranked flag on a compound index `[p1, …, pn]` puts an indexed tree
/// at each prefix's terminal `pn` property-name level — inside the value
/// trees of the `[p1, …, pn-1]` level. When another countable/summable
/// index terminates at exactly that prefix, those value trees are
/// aggregating (`CountTree` / `SumTree` / …), and every continuation
/// subtree inside them must be wrapped in a `NonCounted` / `NotSummed`
/// shell so its contents don't pollute the prefix index's aggregates.
/// grovedb structurally rejects that shell around an indexed tree — the
/// wrapper would neutralize the very aggregates the ranking indexes —
/// so the write path fails closed at document insert. Rejecting the
/// contract here surfaces the conflict at registration instead.
///
/// Only the **exact** `n-1` prefix conflicts. An aggregating index
/// terminating at a shorter prefix wraps a plain intermediate
/// property-name tree (fine), and one extending *past* the ranked
/// terminal lives inside the indexed tree's value trees, which the
/// storage layer supports (see rs-drive's
/// `ranked_terminator_with_a_compound_continuation_gets_both_treatments`).
///
/// Property comparison is by name, positionally: the merged index-level
/// tree keys sub-levels by property name in declaration order, so
/// `[a, b]` and `[b, a]` never share a level and cannot conflict.
///
/// Unconditional (not gated on `full_validation`): the same structural
/// impossibility must reject the contract on every parse path — a
/// contract admitted through a non-validating parse would brick the
/// first document insert under the ranked index.
pub(super) fn validate_no_ranked_prefix_overlap(
    indices: &BTreeMap<String, Index>,
) -> Result<(), ProtocolError> {
    for ranked in indices.values() {
        let is_ranked =
            ranked.ranked_countable || ranked.ranked_summable || ranked.ranked_averageable;
        if !is_ranked || ranked.properties.len() < 2 {
            continue;
        }
        let prefix = &ranked.properties[..ranked.properties.len() - 1];
        for other in indices.values() {
            if other.name == ranked.name {
                continue;
            }
            let terminates_at_prefix = other.properties.len() == prefix.len()
                && other
                    .properties
                    .iter()
                    .zip(prefix.iter())
                    .all(|(a, b)| a.name == b.name);
            let aggregates = other.countable.is_countable() || other.summable.is_some();
            if terminates_at_prefix && aggregates {
                return Err(consensus_or_protocol_data_contract_error(
                    DataContractError::InvalidContractStructure(format!(
                        "compound ranked index `{}` conflicts with index `{}`: the ranked \
                         index's leading prefix [{}] also terminates a countable/summable \
                         index, so the ranked terminal tree would sit inside aggregating \
                         value trees and need a NonCounted/NotSummed shell — which the \
                         storage layer rejects for indexed trees because the wrapper would \
                         neutralize the aggregates the ranking indexes. Drop the ranked \
                         flags, or drop the aggregate flags from the prefix index",
                        ranked.name,
                        other.name,
                        prefix
                            .iter()
                            .map(|p| p.name.as_str())
                            .collect::<Vec<_>>()
                            .join(", "),
                    )),
                ));
            }
        }
    }
    Ok(())
}
