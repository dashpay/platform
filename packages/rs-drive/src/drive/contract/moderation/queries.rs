use crate::drive::contract::moderation::types::{
    ContractDocumentRemovalsQuery, ContractDocumentRemovalsSelection,
    ContractModerationEntriesQuery, ContractTeamActionsQuery,
};
use crate::drive::contract::paths::{
    contract_document_type_removals_path_vec, contract_moderation_action_counts_path_vec,
    contract_moderation_list_path_vec, contract_team_action_path_vec,
    contract_team_action_signers_path_vec, contract_team_action_status_path_vec,
    contract_team_actions_path_vec, CONTRACT_TEAM_ACTION_INFO_KEY,
    CONTRACT_TEAM_ACTION_SIGNERS_KEY, CONTRACT_TEAM_ACTIVE_ACTIONS_KEY,
    CONTRACT_TEAM_CLOSED_ACTIONS_KEY,
};
use crate::drive::Drive;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::{Query, QueryItem};
use dpp::data_contract::config::moderation::ContractModerationList;
use dpp::group::group_action_status::GroupActionStatus;
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
        let mut query = Query::new_with_direction(true);
        match &removals_query.selection {
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
        PathQuery {
            path: contract_document_type_removals_path_vec(
                &contract_id,
                &removals_query.document_type_name,
            ),
            query: SizedQuery {
                query,
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
        let max_limit = platform_version.drive_abci.query.max_returned_elements;
        match &query.selection {
            ContractDocumentRemovalsSelection::DocumentIds(ids) => {
                if ids.is_empty() || ids.len() > max_limit as usize {
                    return Err(Error::Query(QuerySyntaxError::InvalidLimit(format!(
                        "contract document removals must name between 1 and {} document ids, got {}",
                        max_limit,
                        ids.len()
                    ))));
                }
                let mut sorted = ids.clone();
                sorted.sort_unstable();
                sorted.dedup();
                if sorted.len() != ids.len() {
                    return Err(Error::Query(QuerySyntaxError::InvalidParameter(
                        "contract document removals name a document id twice".to_string(),
                    )));
                }
            }
            ContractDocumentRemovalsSelection::Page { limit, .. } => {
                if *limit == 0 || *limit > max_limit {
                    return Err(Error::Query(QuerySyntaxError::InvalidLimit(format!(
                        "contract document removals limit must be between 1 and {}, got {}",
                        max_limit, limit
                    ))));
                }
            }
        }
        Ok(())
    }

    /// The query for the moderation action counts of an elected contract, in identity id
    /// order: at most `limit` of them, or all when `None`. The proof reads them all, like the
    /// approvals of a team action: a count exists only for a member of the seated team (every
    /// settle of the moderators pot deletes them all, and a change of the team settles first),
    /// so the tree never holds more than the team, bounded when its contract registered. A
    /// limit from today's limits could cut a team registered under larger ones short, with a
    /// proof that still verifies.
    pub fn contract_moderation_action_counts_query(
        contract_id: [u8; 32],
        limit: Option<u16>,
    ) -> PathQuery {
        let mut query = Query::new_with_direction(true);
        query.insert_item(QueryItem::RangeFull(RangeFull));
        PathQuery {
            path: contract_moderation_action_counts_path_vec(&contract_id),
            query: SizedQuery {
                query,
                limit,
                offset: None,
            },
        }
    }

    /// The query for a page of a contract's team actions, active or closed: each action's info
    /// (`I`) and its approvals tree (`S`), whose sum is how many approvals it holds, in action id
    /// order, from the start the query gives. Every action holds both, so the limit is twice the
    /// page's, and the prover and the verifier bound the proof alike. It read `I` alone before
    /// it was given `S`, in place at method version 0: no release carried that shape (the query
    /// is protocol version 14's, unreleased), so every released prover and verifier agree.
    pub fn contract_team_actions_query(
        contract_id: [u8; 32],
        actions_query: &ContractTeamActionsQuery,
    ) -> PathQuery {
        let mut query = Query::new_with_direction(true);
        match actions_query.start_at {
            None => query.insert_item(QueryItem::RangeFull(RangeFull)),
            Some((action_id, true)) => {
                query.insert_item(QueryItem::RangeFrom(action_id.to_vec()..))
            }
            Some((action_id, false)) => {
                query.insert_item(QueryItem::RangeAfter(action_id.to_vec()..))
            }
        }
        let mut action_query = Query::new_with_direction(true);
        action_query.insert_keys(vec![
            CONTRACT_TEAM_ACTION_INFO_KEY.to_vec(),
            CONTRACT_TEAM_ACTION_SIGNERS_KEY.to_vec(),
        ]);
        query.set_subquery(action_query);
        PathQuery {
            path: contract_team_action_status_path_vec(&contract_id, actions_query.status),
            query: SizedQuery {
                query,
                limit: Some(actions_query.limit.saturating_mul(2)),
                offset: None,
            },
        }
    }

    /// The query for one team action's info (`I`), active or closed as `status` says: proved
    /// present with its value, or absent.
    pub fn contract_team_action_query(
        contract_id: [u8; 32],
        status: GroupActionStatus,
        action_id: [u8; 32],
    ) -> PathQuery {
        PathQuery::new_single_key(
            contract_team_action_path_vec(&contract_id, status, &action_id),
            CONTRACT_TEAM_ACTION_INFO_KEY.to_vec(),
        )
    }

    /// The query for every approval of one team action, active or closed as `status` says. A
    /// team action holds at most the members of the team, so the query needs no limit.
    pub fn contract_team_action_signers_query(
        contract_id: [u8; 32],
        status: GroupActionStatus,
        action_id: [u8; 32],
    ) -> PathQuery {
        PathQuery::new_unsized(
            contract_team_action_signers_path_vec(&contract_id, status, &action_id),
            Query::new_range_full(),
        )
    }

    /// The query for one member's approval of one team action wherever it is, active or closed:
    /// what a proposal's or an approval's execution proves. An approval never moves but with its
    /// action, when the action closes, so the proof holds while the approval stands: one deleted
    /// because its member left the team no longer proves.
    pub fn contract_team_action_signer_query(
        contract_id: [u8; 32],
        action_id: [u8; 32],
        signer_id: [u8; 32],
    ) -> PathQuery {
        let mut query = Query::new_with_direction(true);
        query.insert_keys(vec![
            CONTRACT_TEAM_ACTIVE_ACTIONS_KEY.to_vec(),
            CONTRACT_TEAM_CLOSED_ACTIONS_KEY.to_vec(),
        ]);
        query.set_subquery_path(vec![
            action_id.to_vec(),
            CONTRACT_TEAM_ACTION_SIGNERS_KEY.to_vec(),
            signer_id.to_vec(),
        ]);
        PathQuery::new_unsized(contract_team_actions_path_vec(&contract_id), query)
    }

    /// A page of team actions returns at least one and at most `max_returned_elements`: what
    /// bounds the proof the verifier accepts. Drive, the node's query handler and the proof
    /// verifier all call it.
    pub fn check_contract_team_actions_query(
        query: &ContractTeamActionsQuery,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let max_limit = platform_version.drive_abci.query.max_returned_elements;
        if query.limit == 0 || query.limit > max_limit {
            return Err(Error::Query(QuerySyntaxError::InvalidLimit(format!(
                "contract team actions limit must be between 1 and {}, got {}",
                max_limit, query.limit
            ))));
        }
        Ok(())
    }
}
