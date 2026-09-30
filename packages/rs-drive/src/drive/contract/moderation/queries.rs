use crate::drive::contract::moderation::types::{
    ContractDocumentRemovalsQuery, ContractDocumentRemovalsSelection,
    ContractModerationEntriesQuery, ContractSettledDeletionsQuery,
};
use crate::drive::contract::paths::{
    contract_document_type_removals_path_vec, contract_document_type_settled_deletions_path_vec,
    contract_moderation_list_path_vec,
};
use crate::drive::Drive;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::{Query, QueryItem};
use dpp::data_contract::config::moderation::ContractModerationList;
use dpp::version::PlatformVersion;
use grovedb::{PathQuery, SizedQuery};
use grovedb_version::version::GroveVersion;
use std::ops::RangeFull;

impl Drive {
    /// The query for one identity's entry in one moderation list of a contract: the key is
    /// proved present with its value, or absent.
    pub fn contract_moderation_entry_query(
        contract_id: [u8; 32],
        list: ContractModerationList,
        identity_id: [u8; 32],
    ) -> PathQuery {
        let mut query = Query::new_with_direction(true);
        query.insert_item(QueryItem::Key(identity_id.to_vec()));
        PathQuery {
            path: contract_moderation_list_path_vec(&contract_id, list),
            query: SizedQuery {
                query,
                limit: None,
                offset: None,
            },
        }
    }

    /// The query for one identity's status on a contract: its entry in every list of `lists`,
    /// merged into one proof. `lists` are the lists the contract's config declares; an
    /// undeclared list has no tree and cannot be queried.
    pub fn contract_moderation_status_query(
        contract_id: [u8; 32],
        identity_id: [u8; 32],
        lists: &[ContractModerationList],
        grove_version: &GroveVersion,
    ) -> Result<PathQuery, Error> {
        let queries: Vec<PathQuery> = lists
            .iter()
            .map(|list| Self::contract_moderation_entry_query(contract_id, *list, identity_id))
            .collect();
        match queries.as_slice() {
            [single] => Ok(single.clone()),
            _ => Ok(PathQuery::merge(queries.iter().collect(), grove_version)?),
        }
    }

    /// The query for one page of one moderation list of a contract: at most `limit` entries
    /// in identity id order, continuing after the cursor.
    pub fn contract_moderation_entries_query(
        contract_id: [u8; 32],
        entries_query: &ContractModerationEntriesQuery,
    ) -> PathQuery {
        let mut query = Query::new_with_direction(true);
        match entries_query.start_after {
            None => query.insert_item(QueryItem::RangeFull(RangeFull)),
            Some(identity_id) => query.insert_item(QueryItem::RangeAfter(identity_id.to_vec()..)),
        }
        PathQuery {
            path: contract_moderation_list_path_vec(&contract_id, entries_query.list),
            query: SizedQuery {
                query,
                limit: Some(entries_query.limit),
                offset: None,
            },
        }
    }

    /// The query for the records of the documents a contract's moderators deleted, within one
    /// document type: the ids named, each proved present with its record or absent, or one
    /// page in document id order continuing after the cursor. The limit of an id read is the
    /// number of ids, so the prover and the verifier bound the proof alike.
    pub fn contract_document_removals_query(
        contract_id: [u8; 32],
        removals_query: &ContractDocumentRemovalsQuery,
    ) -> PathQuery {
        PathQuery {
            path: contract_document_type_removals_path_vec(
                &contract_id,
                &removals_query.document_type_name,
            ),
            query: SizedQuery {
                query: document_ids_selection_query(&removals_query.selection),
                limit: Some(removals_query.limit()),
                offset: None,
            },
        }
    }

    /// A read names at least one and at most `max_returned_elements` records, whether by id or
    /// as a page, and no id twice: what bounds the proof the verifier accepts. The one place the
    /// bounds are written: Drive, the node's query handler and the proof verifier all call it,
    /// so a request the node refuses is one the verifier refuses.
    pub fn check_contract_document_removals_query(
        query: &ContractDocumentRemovalsQuery,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        check_document_ids_selection(
            &query.selection,
            "contract document removals",
            platform_version,
        )
    }

    /// The query for the approvals a seated moderation team gave the deletion of settled
    /// documents, within one document type, the documents selected as for
    /// [`Self::contract_document_removals_query`].
    pub fn contract_settled_deletions_query(
        contract_id: [u8; 32],
        settled_deletions_query: &ContractSettledDeletionsQuery,
    ) -> PathQuery {
        PathQuery {
            path: contract_document_type_settled_deletions_path_vec(
                &contract_id,
                &settled_deletions_query.document_type_name,
            ),
            query: SizedQuery {
                query: document_ids_selection_query(&settled_deletions_query.selection),
                limit: Some(settled_deletions_query.limit()),
                offset: None,
            },
        }
    }

    /// The bounds of a read of settled-deletion approvals, the same as a read of removal
    /// records ([`Self::check_contract_document_removals_query`]).
    pub fn check_contract_settled_deletions_query(
        query: &ContractSettledDeletionsQuery,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        check_document_ids_selection(
            &query.selection,
            "contract settled deletions",
            platform_version,
        )
    }
}

/// The ids named, each a key, or one page in document id order continuing after the cursor.
fn document_ids_selection_query(selection: &ContractDocumentRemovalsSelection) -> Query {
    let mut query = Query::new_with_direction(true);
    match selection {
        ContractDocumentRemovalsSelection::DocumentIds(ids) => {
            for id in ids {
                query.insert_item(QueryItem::Key(id.to_vec()));
            }
        }
        ContractDocumentRemovalsSelection::Page {
            start_after: None, ..
        } => query.insert_item(QueryItem::RangeFull(RangeFull)),
        ContractDocumentRemovalsSelection::Page {
            start_after: Some(document_id),
            ..
        } => query.insert_item(QueryItem::RangeAfter(document_id.to_vec()..)),
    }
    query
}

/// At least one and at most `max_returned_elements` records, whether by id or as a page, and no
/// id twice; `what` names the records in the refusal.
fn check_document_ids_selection(
    selection: &ContractDocumentRemovalsSelection,
    what: &str,
    platform_version: &PlatformVersion,
) -> Result<(), Error> {
    let max_limit = platform_version.drive_abci.query.max_returned_elements;
    match selection {
        ContractDocumentRemovalsSelection::DocumentIds(ids) => {
            if ids.is_empty() || ids.len() > max_limit as usize {
                return Err(Error::Query(QuerySyntaxError::InvalidLimit(format!(
                    "{what} must name between 1 and {} document ids, got {}",
                    max_limit,
                    ids.len()
                ))));
            }
            let mut sorted = ids.clone();
            sorted.sort_unstable();
            sorted.dedup();
            if sorted.len() != ids.len() {
                return Err(Error::Query(QuerySyntaxError::InvalidParameter(format!(
                    "{what} name a document id twice"
                ))));
            }
        }
        ContractDocumentRemovalsSelection::Page { limit, .. } => {
            if *limit == 0 || *limit > max_limit {
                return Err(Error::Query(QuerySyntaxError::InvalidLimit(format!(
                    "{what} limit must be between 1 and {}, got {}",
                    max_limit, limit
                ))));
            }
        }
    }
    Ok(())
}
