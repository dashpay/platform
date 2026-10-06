//! Joins through a `refersTo: moderatedDocument` property: the removal
//! records a join proves beside the documents it joins.
//!
//! A `moderatedDocument` reference promises its target is in state or was
//! removed by the contract's moderators, on the record: the document type
//! keeps a removal record for every document a moderator deletes, and its
//! documents leave state no other way. So a by-id join off such a property
//! (chained or composite) proves, beside the by-ids fetch of the joined
//! documents, the removal records of the same ids, one more component of the
//! one merged proof: [`removals_path_query`]. A joined id with no document is
//! then reported with its proven record ([`pair_missing_with_removals`]), and
//! an id with neither is refused, as a missing `permanentDocument` target is:
//! corrupted state on the server, an invalid proof in the verifier.
//!
//! The component names every joined id, not only the ones missing a
//! document: the verifier builds the merged query before it knows which are
//! missing, and grovedb proves each queried key present or absent, so a
//! prover can pass neither a removed document off as live nor a live one off
//! as removed.

use crate::drive::contract::moderation::types::{
    ContractDocumentRemovalEntry, ContractDocumentRemovalsQuery, ContractDocumentRemovalsSelection,
};
use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
#[cfg(feature = "server")]
use crate::fees::op::LowLevelDriveOperation;
#[cfg(feature = "server")]
use crate::query::is_absent_path;
use dpp::identifier::Identifier;
#[cfg(feature = "server")]
use dpp::version::PlatformVersion;
#[cfg(feature = "server")]
use grovedb::query_result_type::QueryResultType;
#[cfg(feature = "server")]
use grovedb::TransactionArg;
use grovedb::{Element, PathQuery};
use std::collections::BTreeMap;

/// The removal records of `document_ids` within one document type of one
/// contract, each proved present or absent: the removals query by ids
/// ([`Drive::contract_document_removals_query`]), unlimited, as the by-ids
/// fetch of the documents it sits beside is (the ids bound it), and walking in
/// `left_to_right` so it merges with the components it is proven beside (its
/// selection does not depend on the direction). The ids are queried in
/// canonical byte order, so the prover and a verifier that collected them in
/// any order build the same query.
pub fn removals_path_query(
    contract_id: Identifier,
    document_type_name: &str,
    document_ids: &[Identifier],
    left_to_right: bool,
) -> PathQuery {
    let mut ids = document_ids.to_vec();
    ids.sort();
    ids.dedup();
    let mut path_query = Drive::contract_document_removals_query(
        contract_id.to_buffer(),
        &ContractDocumentRemovalsQuery {
            document_type_name: document_type_name.to_string(),
            selection: ContractDocumentRemovalsSelection::DocumentIds(ids),
        },
    );
    path_query.query.limit = None;
    path_query.query.query.left_to_right = left_to_right;
    path_query
}

/// Decodes the proved (or fetched) entries of a [`removals_path_query`], by
/// document id. A malformed record, or one proved twice, is a corrupted
/// proof.
pub fn decode_removals(
    entries: impl IntoIterator<Item = (Vec<u8>, Element)>,
) -> Result<BTreeMap<Identifier, ContractDocumentRemovalEntry>, Error> {
    let mut removals = BTreeMap::new();
    for (key, element) in entries {
        let entry =
            ContractDocumentRemovalEntry::from_key_element(&key, &element).map_err(|reason| {
                Error::Proof(ProofError::CorruptedProof(format!(
                    "a joined document's removal record does not decode: {reason}"
                )))
            })?;
        let document_id = entry.document_id;
        if removals.insert(document_id, entry).is_some() {
            return Err(Error::Proof(ProofError::CorruptedProof(format!(
                "the removal record of joined document {document_id} is carried twice"
            ))));
        }
    }
    Ok(removals)
}

/// The records of the joined ids `missing` that have no document, in their
/// order, from `removals`. Each must have a record that stands, unrestored: a
/// restored record describes a document that is live again. An id without
/// one means the target left state without a moderator's recorded removal,
/// which a `moderatedDocument` target can not, so it is refused.
pub fn pair_missing_with_removals(
    missing: &[Identifier],
    removals: &BTreeMap<Identifier, ContractDocumentRemovalEntry>,
) -> Result<Vec<ContractDocumentRemovalEntry>, Error> {
    missing
        .iter()
        .map(|document_id| match removals.get(document_id) {
            Some(entry) if !entry.removal.is_restored() => Ok(entry.clone()),
            Some(_) => Err(Error::Proof(ProofError::CorruptedProof(format!(
                "joined document {document_id} is missing although its removal record says a \
                 moderator restored it"
            )))),
            None => Err(Error::Proof(ProofError::CorruptedProof(format!(
                "joined document {document_id} is missing without a removal record: a \
                 moderatedDocument reference resolves to its document or to the record of its \
                 removal"
            )))),
        })
        .collect()
}

/// Fetches, without a proof, the entries of a [`removals_path_query`]: the
/// server's materialized join reads the very query the proof covers. A
/// document type without a removal records tree yet answers with none.
#[cfg(feature = "server")]
pub(crate) fn fetch_removals(
    drive: &Drive,
    path_query: &PathQuery,
    transaction: TransactionArg,
    drive_operations: &mut Vec<LowLevelDriveOperation>,
    platform_version: &PlatformVersion,
) -> Result<BTreeMap<Identifier, ContractDocumentRemovalEntry>, Error> {
    let results = match drive.grove_get_path_query(
        path_query,
        transaction,
        QueryResultType::QueryKeyElementPairResultType,
        drive_operations,
        &platform_version.drive,
    ) {
        Err(error) if is_absent_path(&error) => {
            return Ok(BTreeMap::new());
        }
        other => other?.0,
    };
    decode_removals(results.to_key_elements())
}
