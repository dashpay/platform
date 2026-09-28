//! Contract moderation query results and the wire conversions the proved and unproved paths
//! share: one identity's status on the lists queried ([`ContractModerationListStatuses`]) and one
//! page of a contract's banlist, suspension list or warning list ([`ContractModerationEntries`],
//! read with a [`ContractModerationEntriesQuery`]). The records of the documents its moderators deleted
//! ([`ContractDocumentRemovals`], read with a [`ContractDocumentRemovalsQuery`]) and the fee
//! pots of a contract ([`ContractFeePots`]) are read here too: the pots are what its document
//! action fees pay its owner and its moderators.

use crate::Error;
use dapi_grpc::platform::v0::get_contract_document_removals_request::get_contract_document_removals_request_v0::Selection;
use dapi_grpc::platform::v0::get_contract_document_removals_request::{DocumentIds, Page};
use dapi_grpc::platform::v0::get_contract_document_removals_response::ContractDocumentRemoval as ContractDocumentRemovalProto;
#[cfg(test)]
use dapi_grpc::platform::v0::get_contract_fee_pots_response::ContractFeePotLastClaim as ContractFeePotLastClaimProto;
use dapi_grpc::platform::v0::get_contract_fee_pots_response::{
    ContractFeePot as ContractFeePotProto, ContractFeePots as ContractFeePotsProto,
};
use dapi_grpc::platform::v0::get_contract_moderation_entries_response::ContractModerationEntry as ContractModerationEntryProto;
#[cfg(test)]
use dapi_grpc::platform::v0::ContractModerationDocument as ContractModerationDocumentProto;
use dapi_grpc::platform::v0::ContractModerationList as ContractModerationListProto;
use dapi_grpc::platform::v0::ContractModerationReason as ContractModerationReasonProto;
use dapi_grpc::platform::v0::ContractWarning as ContractWarningProto;
pub use dpp::data_contract::config::moderation::{
    ContractBan, ContractDocumentRemoval, ContractDocumentRestoration, ContractModerationDocument,
    ContractModerationList,
    ContractModerationListStatus, ContractModerationListStatuses, ContractModerationReason,
    ContractModerationStatus, ContractSuspension, ContractWarning,
};
pub use dpp::data_contract::document_type::action_fees::{ContractFeePot, ContractFeePotLastClaim};
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
pub use drive::drive::contract::fee_pots::types::{ContractFeePotState, ContractFeePots};
pub use drive::drive::contract::moderation::types::{
    ContractDocumentRemovalEntry, ContractDocumentRemovalsQuery,
    ContractDocumentRemovalsSelection, ContractModerationEntriesQuery, ContractModerationEntry,
};

/// The page size a request without a limit asks for, which is also the largest page a node
/// returns: the platform version's `max_returned_elements`, the number the node reads too.
pub fn default_contract_moderation_entries_limit(platform_version: &PlatformVersion) -> u16 {
    platform_version.drive_abci.query.max_returned_elements
}

/// One page of a moderated contract's banlist, suspension list or warning list, in identity id
/// order. A page shorter than the limit is the last one.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ContractModerationEntries(pub Vec<ContractModerationEntry>);

impl ContractModerationEntries {
    /// The entries of the page.
    pub fn entries(&self) -> &[ContractModerationEntry] {
        &self.0
    }

    /// The query for the page after this one, or `None` when this page is the last: it holds
    /// fewer entries than `query` asked for.
    pub fn next_query(
        &self,
        query: &ContractModerationEntriesQuery,
    ) -> Option<ContractModerationEntriesQuery> {
        if self.0.len() < usize::from(query.limit) {
            return None;
        }
        self.0.last().map(|entry| ContractModerationEntriesQuery {
            list: query.list,
            start_after: Some(entry.identity_id),
            limit: query.limit,
        })
    }
}

/// The pots a fee pots request reads, in the order its proof is built and verified in: always
/// both.
pub const CONTRACT_FEE_POTS_QUERIED: [ContractFeePot; 2] =
    [ContractFeePot::Owner, ContractFeePot::Moderators];

/// The page size a removals request without a limit asks for, which is also the largest page a
/// node returns and the most document ids one may name: the platform version's
/// `max_returned_elements`, the number the node reads too.
pub fn default_contract_document_removals_limit(platform_version: &PlatformVersion) -> u16 {
    platform_version.drive_abci.query.max_returned_elements
}

/// The records of the documents a contract's moderators deleted within one document type, in
/// document id order. A page shorter than the limit is the last one; a read by ids holds only
/// the ids that have a record.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ContractDocumentRemovals(pub Vec<ContractDocumentRemovalEntry>);

impl ContractDocumentRemovals {
    /// The records read.
    pub fn removals(&self) -> &[ContractDocumentRemovalEntry] {
        &self.0
    }

    /// The query for the page after this one, or `None` when this page is the last: it holds
    /// fewer records than `query` asked for. A read by ids names every record it wants, so
    /// nothing follows it.
    pub fn next_query(
        &self,
        query: &ContractDocumentRemovalsQuery,
    ) -> Option<ContractDocumentRemovalsQuery> {
        let ContractDocumentRemovalsSelection::Page { limit, .. } = &query.selection else {
            return None;
        };
        let limit = *limit;
        if self.0.len() < usize::from(limit) {
            return None;
        }
        self.0.last().map(|entry| ContractDocumentRemovalsQuery {
            document_type_name: query.document_type_name.clone(),
            selection: ContractDocumentRemovalsSelection::Page {
                start_after: Some(entry.document_id),
                limit,
            },
        })
    }
}

/// The 32 byte identifier a request field holds, naming the field in the error.
pub fn identifier_from_request(bytes: &[u8], what: &str) -> Result<Identifier, Error> {
    Identifier::from_bytes(bytes).map_err(|_| Error::RequestError {
        error: format!(
            "{what} must be a 32 byte identifier, got {} bytes",
            bytes.len()
        ),
    })
}

/// The 32 byte identifier a response field holds, naming the field in the error.
fn identifier_from_response(bytes: &[u8], what: &str) -> Result<Identifier, Error> {
    Identifier::from_bytes(bytes).map_err(|_| Error::ProtocolError {
        error: format!(
            "{what} must be a 32 byte identifier, got {} bytes",
            bytes.len()
        ),
    })
}

/// The list a request field names.
pub fn list_from_request(list: i32, what: &str) -> Result<ContractModerationList, Error> {
    match ContractModerationListProto::try_from(list) {
        Ok(ContractModerationListProto::Banlist) => Ok(ContractModerationList::Banlist),
        Ok(ContractModerationListProto::Suspensions) => Ok(ContractModerationList::Suspensions),
        Ok(ContractModerationListProto::Warnings) => Ok(ContractModerationList::Warnings),
        // Zero is what a proto3 client sends when it leaves the field out: not a list.
        Ok(ContractModerationListProto::Unspecified) | Err(_) => Err(Error::RequestError {
            error: format!("{what} {list} is not a moderation list"),
        }),
    }
}

/// The wire number of a list.
pub fn list_to_request(list: ContractModerationList) -> i32 {
    match list {
        ContractModerationList::Banlist => ContractModerationListProto::Banlist as i32,
        ContractModerationList::Suspensions => ContractModerationListProto::Suspensions as i32,
        ContractModerationList::Warnings => ContractModerationListProto::Warnings as i32,
    }
}

/// The lists a status request names: at least one, no repeats.
pub fn lists_from_request(lists: &[i32]) -> Result<Vec<ContractModerationList>, Error> {
    let mut parsed: Vec<ContractModerationList> = Vec::with_capacity(lists.len());
    for list in lists {
        let list = list_from_request(*list, "lists")?;
        if parsed.contains(&list) {
            return Err(Error::RequestError {
                error: format!("lists names {} twice", list),
            });
        }
        parsed.push(list);
    }
    if parsed.is_empty() {
        return Err(Error::RequestError {
            error: "lists must name at least one list".to_string(),
        });
    }
    Ok(parsed)
}

/// The Drive query of an entries request.
pub fn entries_query_from_request(
    list: i32,
    start_after: Option<&[u8]>,
    limit: Option<u32>,
    platform_version: &PlatformVersion,
) -> Result<ContractModerationEntriesQuery, Error> {
    let limit = match limit {
        None => default_contract_moderation_entries_limit(platform_version),
        // The bounds the node enforces: a request outside them never got a proof.
        Some(limit) => u16::try_from(limit)
            .ok()
            .filter(|limit| {
                (1..=default_contract_moderation_entries_limit(platform_version)).contains(limit)
            })
            .ok_or_else(|| Error::RequestError {
                error: format!(
                    "limit {limit} is out of bounds, it must be between 1 and {}",
                    default_contract_moderation_entries_limit(platform_version)
                ),
            })?,
    };
    Ok(ContractModerationEntriesQuery {
        list: list_from_request(list, "list")?,
        start_after: start_after
            .map(|bytes| identifier_from_request(bytes, "start_after"))
            .transpose()?,
        limit,
    })
}

/// The reason of an unproved response. Every ban and every suspension carries one, so a
/// response without it is refused, and so is a code that is not a u16 or a cited document
/// whose id is not 32 bytes, which no entry of any version can hold. The length of the text
/// and the number of documents are not checked: their limits belong to a protocol version and
/// may be raised by a later one, the proved path reads whatever the proof holds, and the two
/// must agree on which stored reasons a client can read.
pub fn reason_from_response(
    reason: Option<ContractModerationReasonProto>,
) -> Result<ContractModerationReason, Error> {
    let ContractModerationReasonProto {
        code,
        text,
        documents,
        reason_document_id,
    } = reason.ok_or(Error::ResponseDecodeError {
        error: "contract moderation entry holds no reason".to_string(),
    })?;
    let code = code
        .map(|code| {
            u16::try_from(code).map_err(|_| Error::ResponseDecodeError {
                error: format!("contract moderation reason code {code} is not a u16"),
            })
        })
        .transpose()?;
    let documents = documents
        .into_iter()
        .map(|document| {
            Ok(ContractModerationDocument {
                document_type_name: document.document_type_name,
                document_id: Identifier::from_bytes(&document.document_id).map_err(|_| {
                    Error::ResponseDecodeError {
                        error: format!(
                            "a document a contract moderation reason cites has an id of {} \
                             bytes, not 32",
                            document.document_id.len()
                        ),
                    }
                })?,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let reason_document_id = reason_document_id
        .map(|id| {
            Identifier::from_bytes(&id).map_err(|_| Error::ResponseDecodeError {
                error: format!(
                    "the reason document a contract moderation reason names has an id of {} \
                     bytes, not 32",
                    id.len()
                ),
            })
        })
        .transpose()?;
    Ok(ContractModerationReason {
        code,
        text,
        documents,
        reason_document_id,
    })
}

/// The warnings of an unproved response, oldest first as the node answers. Every warning
/// carries a reason, as [`reason_from_response`] requires.
pub fn warnings_from_response(
    warnings: Vec<ContractWarningProto>,
) -> Result<Vec<ContractWarning>, Error> {
    warnings
        .into_iter()
        .map(|warning| {
            Ok(ContractWarning {
                warned_at: warning.warned_at,
                reason: reason_from_response(warning.reason)?,
            })
        })
        .collect()
}

/// The entries of an unproved response. A warning list entry carries its warnings and no
/// `reason` of its own: its reason is the latest warning's, read from there, and a response
/// that sends one beside the warnings is refused rather than left with two sources for it.
pub fn entries_from_response(
    entries: Vec<ContractModerationEntryProto>,
) -> Result<ContractModerationEntries, Error> {
    entries
        .into_iter()
        .map(|entry| {
            let warnings = warnings_from_response(entry.warnings)?;
            let reason = match warnings.last() {
                None => reason_from_response(entry.reason)?,
                Some(latest) if entry.reason.is_none() => latest.reason.clone(),
                Some(_) => {
                    return Err(Error::ResponseDecodeError {
                        error: "contract moderation entry carries warnings and a reason of \
                                its own"
                            .to_string(),
                    })
                }
            };
            Ok(ContractModerationEntry {
                identity_id: Identifier::from_bytes(&entry.identity_id).map_err(|_| {
                    Error::ProtocolError {
                        error: format!(
                            "entry identity id must be a 32 byte identifier, got {} bytes",
                            entry.identity_id.len()
                        ),
                    }
                })?,
                until: entry.until,
                reason,
                warnings,
            })
        })
        .collect::<Result<Vec<_>, Error>>()
        .map(ContractModerationEntries)
}

/// The Drive query of a document removals request: the document type and what to read of it,
/// under the bounds the node enforces, so a request outside them never got a proof. A request
/// that selects nothing asks for no records at all and is refused rather than read as a page.
pub fn removals_query_from_request(
    document_type_name: String,
    selection: Option<Selection>,
    platform_version: &PlatformVersion,
) -> Result<ContractDocumentRemovalsQuery, Error> {
    let selection = selection.ok_or_else(|| Error::RequestError {
        error: "either document_ids or page must be set".to_string(),
    })?;
    let selection = match selection {
        Selection::DocumentIds(DocumentIds { document_ids }) => {
            ContractDocumentRemovalsSelection::DocumentIds(
                document_ids
                    .iter()
                    .map(|bytes| identifier_from_request(bytes, "document_ids"))
                    .collect::<Result<Vec<Identifier>, Error>>()?,
            )
        }
        Selection::Page(Page { start_after, limit }) => ContractDocumentRemovalsSelection::Page {
            start_after: start_after
                .as_deref()
                .map(|bytes| identifier_from_request(bytes, "start_after"))
                .transpose()?,
            // The page size when the request names none: the largest page, the number the node
            // read and proved. A limit no u16 holds is past every bound, and is refused below
            // as the largest u16 is.
            limit: limit.map_or(
                default_contract_document_removals_limit(platform_version),
                |limit| u16::try_from(limit).unwrap_or(u16::MAX),
            ),
        },
    };
    let query = ContractDocumentRemovalsQuery {
        document_type_name,
        selection,
    };
    // The bounds the node enforced, from the one place they are written.
    Drive::check_contract_document_removals_query(&query, platform_version).map_err(|error| {
        Error::RequestError {
            error: error.to_string(),
        }
    })?;
    Ok(query)
}

/// The records of an unproved response. Every record names three identities, carries a
/// reason and a 32 byte document hash, and a restored one names a fourth identity, so a
/// response missing any of them is refused, and so is one that answers with more records than
/// the query could hold or with the record of a document it did not name.
pub fn removals_from_response(
    removals: Vec<ContractDocumentRemovalProto>,
    query: &ContractDocumentRemovalsQuery,
) -> Result<ContractDocumentRemovals, Error> {
    let limit = query.limit();
    if removals.len() > usize::from(limit) {
        return Err(Error::ResponseDecodeError {
            error: format!(
                "{} document removals returned, the query asked for at most {limit}",
                removals.len()
            ),
        });
    }
    let entries = removals
        .into_iter()
        .map(|removal| {
            Ok(ContractDocumentRemovalEntry {
                document_id: identifier_from_response(&removal.document_id, "removal document id")?,
                removal: ContractDocumentRemoval {
                    document_owner_id: identifier_from_response(
                        &removal.document_owner_id,
                        "removal document owner id",
                    )?,
                    moderator_id: identifier_from_response(
                        &removal.moderator_id,
                        "removal moderator id",
                    )?,
                    reason: reason_from_response(removal.reason)?,
                    removed_at: removal.removed_at,
                    document_hash: removal.document_hash.as_slice().try_into().map_err(|_| {
                        Error::ResponseDecodeError {
                            error: format!(
                                "removal document hash holds {} bytes, expected 32",
                                removal.document_hash.len()
                            ),
                        }
                    })?,
                    restoration: removal
                        .restoration
                        .map(|restoration| {
                            Ok::<_, Error>(ContractDocumentRestoration {
                                moderator_id: identifier_from_response(
                                    &restoration.moderator_id,
                                    "restoration moderator id",
                                )?,
                                restored_at: restoration.restored_at,
                            })
                        })
                        .transpose()?,
                },
            })
        })
        .collect::<Result<Vec<ContractDocumentRemovalEntry>, Error>>()?;
    // A read by ids asked for those records and no others, so the record of another document
    // is not an answer to it.
    if let ContractDocumentRemovalsSelection::DocumentIds(ids) = &query.selection {
        if let Some(entry) = entries
            .iter()
            .find(|entry| !ids.contains(&entry.document_id))
        {
            return Err(Error::ResponseDecodeError {
                error: format!(
                    "the response holds the removal of document {}, which the query did not name",
                    entry.document_id
                ),
            });
        }
    }
    Ok(ContractDocumentRemovals(entries))
}

/// One pot of an unproved response. The epoch of a last claim is a u16 on the chain and its
/// claimant a 32 byte identifier, so a response naming anything else is refused: no pot of any
/// version can hold it.
fn fee_pot_from_response(
    pot: Option<ContractFeePotProto>,
    what: &str,
) -> Result<ContractFeePotState, Error> {
    let ContractFeePotProto {
        credits,
        last_claim,
    } = pot.ok_or_else(|| Error::ResponseDecodeError {
        error: format!("contract fee pots response holds no {what} pot"),
    })?;
    let last_claim = last_claim
        .map(|last_claim| {
            Ok::<_, Error>(ContractFeePotLastClaim {
                epoch_index: u16::try_from(last_claim.epoch).map_err(|_| {
                    Error::ResponseDecodeError {
                        error: format!(
                            "last claim epoch {} of the {what} pot is not a u16",
                            last_claim.epoch
                        ),
                    }
                })?,
                time_ms: last_claim.time_ms,
                claimant_id: Identifier::from_bytes(&last_claim.claimant_id).map_err(|_| {
                    Error::ResponseDecodeError {
                        error: format!(
                            "last claimant of the {what} pot must be a 32 byte identifier, got {} bytes",
                            last_claim.claimant_id.len()
                        ),
                    }
                })?,
            })
        })
        .transpose()?;
    Ok(ContractFeePotState {
        credits,
        last_claim,
    })
}

/// The fee pots of an unproved response. A node always answers with both pots, an empty one as
/// zero credits, so a response that leaves one out is refused.
pub fn fee_pots_from_response(pots: ContractFeePotsProto) -> Result<ContractFeePots, Error> {
    Ok(ContractFeePots {
        owner: fee_pot_from_response(pots.owner, "owner")?,
        moderators: fee_pot_from_response(pots.moderators, "moderators")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use dapi_grpc::platform::v0::get_contract_document_removals_response::ContractDocumentRestoration as ContractDocumentRestorationProto;

    fn id(seed: u8) -> Identifier {
        Identifier::from([seed; 32])
    }

    #[test]
    fn should_report_only_the_lists_queried() {
        // A banned identity, read on the suspension list alone: not suspended, and nothing
        // said about the banlist.
        let banned = ContractModerationStatus {
            ban: Some(ContractBan {
                reason: ContractModerationReason::from_text("spam"),
            }),
            suspension: None,
            warnings: vec![],
        };
        let suspensions_only = ContractModerationListStatuses::from_status(
            &[ContractModerationList::Suspensions],
            &banned,
        );
        assert_eq!(suspensions_only.banned(), None);
        assert_eq!(suspensions_only.suspended_until(), Some(None));
        assert!(!suspensions_only.is_barred_on_queried_lists_at(0));

        let both = ContractModerationListStatuses::from_status(
            &[
                ContractModerationList::Banlist,
                ContractModerationList::Suspensions,
            ],
            &banned,
        );
        assert_eq!(both.banned(), Some(true));
        assert!(both.is_barred_on_queried_lists_at(0));

        let suspended = ContractModerationListStatuses::from_status(
            &[ContractModerationList::Suspensions],
            &ContractModerationStatus {
                ban: None,
                suspension: Some(ContractSuspension {
                    until: 10,
                    reason: ContractModerationReason::from_text("flooding"),
                }),
                warnings: vec![],
            },
        );
        assert!(suspended.is_barred_on_queried_lists_at(9));
        assert!(!suspended.is_barred_on_queried_lists_at(10));
    }

    #[test]
    fn should_round_trip_every_list_through_its_wire_number() {
        for list in [
            ContractModerationList::Banlist,
            ContractModerationList::Suspensions,
            ContractModerationList::Warnings,
        ] {
            assert_eq!(
                list_from_request(list_to_request(list), "list").expect("expected a list"),
                list
            );
        }
        let err = list_from_request(7, "list").unwrap_err();
        assert!(
            matches!(&err, Error::RequestError { error } if error.contains("list 7")),
            "got: {err:?}"
        );
    }

    #[test]
    fn should_parse_the_lists_of_a_status_request() {
        assert_eq!(
            lists_from_request(&[2, 1, 3]).expect("expected lists"),
            vec![
                ContractModerationList::Suspensions,
                ContractModerationList::Banlist,
                ContractModerationList::Warnings,
            ]
        );
        for (lists, needle) in [
            (&[][..], "at least one"),
            (&[1, 1][..], "twice"),
            (&[1, 9][..], "not a moderation list"),
            (&[0][..], "not a moderation list"),
        ] {
            let err = lists_from_request(lists).unwrap_err();
            assert!(
                matches!(&err, Error::RequestError { error } if error.contains(needle)),
                "{lists:?}: {err:?}"
            );
        }
    }

    #[test]
    fn should_build_the_entries_query_of_a_request() {
        let platform_version = PlatformVersion::latest();
        assert_eq!(
            entries_query_from_request(1, None, None, platform_version).expect("expected a query"),
            ContractModerationEntriesQuery {
                list: ContractModerationList::Banlist,
                start_after: None,
                limit: platform_version.drive_abci.query.max_returned_elements,
            }
        );
        assert_eq!(
            entries_query_from_request(2, Some(id(3).as_slice()), Some(5), platform_version)
                .expect("expected a query"),
            ContractModerationEntriesQuery {
                list: ContractModerationList::Suspensions,
                start_after: Some(id(3)),
                limit: 5,
            }
        );
        for (list, start_after, limit, needle) in [
            (9, None, None, "not a moderation list"),
            (0, None, None, "not a moderation list"),
            (1, Some(&[1u8; 5][..]), None, "start_after"),
            (1, None, Some(u16::MAX as u32 + 1), "out of bounds"),
            (1, None, Some(0), "out of bounds"),
            (1, None, Some(101), "out of bounds"),
        ] {
            let err =
                entries_query_from_request(list, start_after, limit, platform_version).unwrap_err();
            assert!(
                matches!(&err, Error::RequestError { error } if error.contains(needle)),
                "{needle}: {err:?}"
            );
        }
    }

    #[test]
    fn should_read_the_entries_of_an_unproved_response() {
        let page = entries_from_response(vec![
            ContractModerationEntryProto {
                identity_id: id(1).to_vec(),
                until: None,
                reason: Some(ContractModerationReasonProto {
                    code: None,
                    text: "spam".to_string(),
                    documents: vec![],
                    reason_document_id: None,
                }),
                warnings: vec![],
            },
            ContractModerationEntryProto {
                identity_id: id(2).to_vec(),
                until: Some(99),
                reason: Some(ContractModerationReasonProto {
                    code: Some(3),
                    text: String::new(),
                    documents: vec![],
                    reason_document_id: None,
                }),
                warnings: vec![],
            },
            ContractModerationEntryProto {
                identity_id: id(3).to_vec(),
                until: None,
                reason: None,
                warnings: vec![
                    ContractWarningProto {
                        warned_at: 5,
                        reason: Some(ContractModerationReasonProto {
                            code: None,
                            text: "first strike".to_string(),
                            documents: vec![],
                            reason_document_id: None,
                        }),
                    },
                    ContractWarningProto {
                        warned_at: 6,
                        reason: Some(ContractModerationReasonProto {
                            code: None,
                            text: "second strike".to_string(),
                            documents: vec![],
                            reason_document_id: None,
                        }),
                    },
                ],
            },
        ])
        .expect("expected entries");
        assert_eq!(
            page.entries(),
            &[
                ContractModerationEntry {
                    identity_id: id(1),
                    until: None,
                    reason: ContractModerationReason::from_text("spam"),
                    warnings: vec![],
                },
                ContractModerationEntry {
                    identity_id: id(2),
                    until: Some(99),
                    reason: ContractModerationReason {
                        code: Some(3),
                        text: String::new(),
                        documents: vec![],
                        reason_document_id: None,
                    },
                    warnings: vec![],
                },
                ContractModerationEntry {
                    identity_id: id(3),
                    until: None,
                    reason: ContractModerationReason::from_text("second strike"),
                    warnings: vec![
                        ContractWarning {
                            warned_at: 5,
                            reason: ContractModerationReason::from_text("first strike"),
                        },
                        ContractWarning {
                            warned_at: 6,
                            reason: ContractModerationReason::from_text("second strike"),
                        },
                    ],
                },
            ]
        );

        // A warning entry's reason is its latest warning's: one beside the warnings is refused.
        let err = entries_from_response(vec![ContractModerationEntryProto {
            identity_id: id(1).to_vec(),
            until: None,
            reason: Some(ContractModerationReasonProto::default()),
            warnings: vec![ContractWarningProto {
                warned_at: 5,
                reason: Some(ContractModerationReasonProto::default()),
            }],
        }])
        .unwrap_err();
        assert!(
            matches!(err, Error::ResponseDecodeError { .. }),
            "got: {err:?}"
        );

        // Every warning carries a reason
        let err = entries_from_response(vec![ContractModerationEntryProto {
            identity_id: id(1).to_vec(),
            until: None,
            reason: Some(ContractModerationReasonProto::default()),
            warnings: vec![ContractWarningProto {
                warned_at: 5,
                reason: None,
            }],
        }])
        .unwrap_err();
        assert!(
            matches!(err, Error::ResponseDecodeError { .. }),
            "got: {err:?}"
        );

        let err = entries_from_response(vec![ContractModerationEntryProto {
            identity_id: vec![1; 5],
            until: None,
            reason: Some(ContractModerationReasonProto::default()),
            warnings: vec![],
        }])
        .unwrap_err();
        assert!(matches!(err, Error::ProtocolError { .. }), "got: {err:?}");

        // Every entry carries a reason
        let err = entries_from_response(vec![ContractModerationEntryProto {
            identity_id: id(1).to_vec(),
            until: None,
            reason: None,
            warnings: vec![],
        }])
        .unwrap_err();
        assert!(
            matches!(err, Error::ResponseDecodeError { .. }),
            "got: {err:?}"
        );

        // A reason cites documents by type and 32-byte id; another id length is refused.
        let cited_reason = |document_id: Vec<u8>| {
            Some(ContractModerationReasonProto {
                code: None,
                text: "spam".to_string(),
                documents: vec![ContractModerationDocumentProto {
                    document_type_name: "post".to_string(),
                    document_id,
                }],
                reason_document_id: None,
            })
        };
        let cited = entries_from_response(vec![ContractModerationEntryProto {
            identity_id: id(1).to_vec(),
            until: None,
            reason: cited_reason(id(9).to_vec()),
            warnings: vec![],
        }])
        .expect("expected a cited document to decode");
        assert_eq!(
            cited.entries()[0].reason.documents,
            vec![ContractModerationDocument {
                document_type_name: "post".to_string(),
                document_id: id(9),
            }]
        );
        let err = entries_from_response(vec![ContractModerationEntryProto {
            identity_id: id(1).to_vec(),
            until: None,
            reason: cited_reason(vec![9; 5]),
            warnings: vec![],
        }])
        .unwrap_err();
        assert!(
            matches!(err, Error::ResponseDecodeError { .. }),
            "got: {err:?}"
        );

        // The text is not bounded here: its limit is a protocol version's, and the proved path
        // reads whatever the proof holds
        let long = entries_from_response(vec![ContractModerationEntryProto {
            identity_id: id(1).to_vec(),
            until: None,
            reason: Some(ContractModerationReasonProto {
                code: None,
                text: "x".repeat(4096),
                documents: vec![],
                reason_document_id: None,
            }),
            warnings: vec![],
        }])
        .expect("expected a long reason to decode");
        assert_eq!(long.entries()[0].reason.text.len(), 4096);
    }

    #[test]
    fn should_continue_after_a_full_page_and_stop_on_a_short_one() {
        let query = ContractModerationEntriesQuery {
            list: ContractModerationList::Suspensions,
            start_after: None,
            limit: 2,
        };
        let page = ContractModerationEntries(vec![
            ContractModerationEntry {
                identity_id: id(1),
                until: Some(5),
                reason: ContractModerationReason::default(),
                warnings: vec![],
            },
            ContractModerationEntry {
                identity_id: id(2),
                until: Some(6),
                reason: ContractModerationReason::default(),
                warnings: vec![],
            },
        ]);
        assert_eq!(
            page.next_query(&query),
            Some(ContractModerationEntriesQuery {
                list: ContractModerationList::Suspensions,
                start_after: Some(id(2)),
                limit: 2,
            })
        );
        assert_eq!(
            ContractModerationEntries::default().next_query(&query),
            None
        );
        // A page shorter than the limit is the last one: no empty page is fetched after it.
        let short = ContractModerationEntries(vec![ContractModerationEntry {
            identity_id: id(1),
            until: Some(5),
            reason: ContractModerationReason::default(),
            warnings: vec![],
        }]);
        assert_eq!(short.next_query(&query), None);
    }

    const POST: &str = "post";

    fn page_selection(start_after: Option<u8>, limit: Option<u32>) -> Option<Selection> {
        Some(Selection::Page(Page {
            start_after: start_after.map(|seed| id(seed).to_vec()),
            limit,
        }))
    }

    fn ids_selection(seeds: &[u8]) -> Option<Selection> {
        Some(Selection::DocumentIds(DocumentIds {
            document_ids: seeds.iter().map(|seed| id(*seed).to_vec()).collect(),
        }))
    }

    fn removal(seed: u8) -> ContractDocumentRemoval {
        ContractDocumentRemoval {
            document_owner_id: id(seed + 0x10),
            moderator_id: id(0x77),
            reason: ContractModerationReason::from_text("spam"),
            removed_at: 1_000 + u64::from(seed),
            document_hash: [seed + 0x20; 32],
            // Every other record was restored.
            restoration: seed.is_multiple_of(2).then(|| ContractDocumentRestoration {
                moderator_id: id(0x78),
                restored_at: 2_000 + u64::from(seed),
            }),
        }
    }

    fn removal_proto(seed: u8) -> ContractDocumentRemovalProto {
        let removal = removal(seed);
        ContractDocumentRemovalProto {
            document_id: id(seed).to_vec(),
            document_owner_id: removal.document_owner_id.to_vec(),
            moderator_id: removal.moderator_id.to_vec(),
            removed_at: removal.removed_at,
            reason: Some(ContractModerationReasonProto {
                code: None,
                text: "spam".to_string(),
                documents: vec![],
                reason_document_id: None,
            }),
            document_hash: removal.document_hash.to_vec(),
            restoration: removal
                .restoration
                .map(|restoration| ContractDocumentRestorationProto {
                    moderator_id: restoration.moderator_id.to_vec(),
                    restored_at: restoration.restored_at,
                }),
        }
    }

    fn ids_query(seeds: &[u8]) -> ContractDocumentRemovalsQuery {
        ContractDocumentRemovalsQuery {
            document_type_name: POST.to_string(),
            selection: ContractDocumentRemovalsSelection::DocumentIds(
                seeds.iter().map(|seed| id(*seed)).collect(),
            ),
        }
    }

    fn page_query(limit: u16) -> ContractDocumentRemovalsQuery {
        ContractDocumentRemovalsQuery {
            document_type_name: POST.to_string(),
            selection: ContractDocumentRemovalsSelection::Page {
                start_after: None,
                limit,
            },
        }
    }

    #[test]
    fn should_build_the_removals_query_of_a_request() {
        let platform_version = PlatformVersion::latest();
        let max = platform_version.drive_abci.query.max_returned_elements;
        assert_eq!(
            removals_query_from_request(
                POST.to_string(),
                page_selection(None, None),
                platform_version
            )
            .expect("expected a query"),
            page_query(max)
        );
        assert_eq!(
            removals_query_from_request(
                POST.to_string(),
                page_selection(Some(3), Some(5)),
                platform_version
            )
            .expect("expected a query"),
            ContractDocumentRemovalsQuery {
                document_type_name: POST.to_string(),
                selection: ContractDocumentRemovalsSelection::Page {
                    start_after: Some(id(3)),
                    limit: 5,
                },
            }
        );
        assert_eq!(
            removals_query_from_request(POST.to_string(), ids_selection(&[3, 1]), platform_version)
                .expect("expected a query"),
            ids_query(&[3, 1])
        );

        for (selection, needle) in [
            (None, "either document_ids or page must be set"),
            (ids_selection(&[]), "must name between 1 and"),
            (ids_selection(&[1, 1]), "name a document id twice"),
            (page_selection(None, Some(0)), "limit must be between 1 and"),
            (
                page_selection(None, Some(u32::from(max) + 1)),
                "limit must be between 1 and",
            ),
            (
                page_selection(None, Some(u16::MAX as u32 + 1)),
                "limit must be between 1 and",
            ),
            (
                Some(Selection::DocumentIds(DocumentIds {
                    document_ids: vec![vec![1; 31]],
                })),
                "document_ids",
            ),
            (
                Some(Selection::Page(Page {
                    start_after: Some(vec![1; 5]),
                    limit: None,
                })),
                "start_after",
            ),
        ] {
            let err = removals_query_from_request(POST.to_string(), selection, platform_version)
                .unwrap_err();
            assert!(
                matches!(&err, Error::RequestError { error } if error.contains(needle)),
                "{needle}: {err:?}"
            );
        }

        // One id over the cap: the node would not have read them either.
        let err = removals_query_from_request(
            POST.to_string(),
            ids_selection(&(0..=max as u8).collect::<Vec<u8>>()),
            platform_version,
        )
        .unwrap_err();
        assert!(
            matches!(&err, Error::RequestError { error } if error.contains("must name between 1 and")),
            "got: {err:?}"
        );
    }

    #[test]
    fn should_read_the_removals_of_an_unproved_response() {
        let removals = removals_from_response(
            vec![removal_proto(1), removal_proto(2)],
            &ids_query(&[1, 2, 9]),
        )
        .expect("expected removals");
        assert_eq!(
            removals.removals(),
            &[
                ContractDocumentRemovalEntry {
                    document_id: id(1),
                    removal: removal(1),
                },
                ContractDocumentRemovalEntry {
                    document_id: id(2),
                    removal: removal(2),
                }
            ]
        );

        // Every identifier of a record is 32 bytes, the restoring moderator's included.
        for spoil in [
            |proto: &mut ContractDocumentRemovalProto| proto.document_id = vec![1; 5],
            |proto: &mut ContractDocumentRemovalProto| proto.document_owner_id = vec![1; 5],
            |proto: &mut ContractDocumentRemovalProto| proto.moderator_id = vec![1; 5],
            |proto: &mut ContractDocumentRemovalProto| {
                proto.restoration = Some(ContractDocumentRestorationProto {
                    moderator_id: vec![1; 5],
                    restored_at: 5,
                })
            },
        ] {
            let mut proto = removal_proto(1);
            spoil(&mut proto);
            let err = removals_from_response(vec![proto], &ids_query(&[1])).unwrap_err();
            assert!(matches!(err, Error::ProtocolError { .. }), "got: {err:?}");
        }

        // And its document hash 32 bytes.
        let mut proto = removal_proto(1);
        proto.document_hash = vec![1; 31];
        let err = removals_from_response(vec![proto], &ids_query(&[1])).unwrap_err();
        assert!(
            matches!(&err, Error::ResponseDecodeError { error } if error.contains("expected 32")),
            "got: {err:?}"
        );

        // Every record carries a reason, with a code that fits a u16.
        for reason in [
            None,
            Some(ContractModerationReasonProto {
                code: Some(u32::from(u16::MAX) + 1),
                text: String::new(),
                documents: vec![],
                reason_document_id: None,
            }),
        ] {
            let proto = ContractDocumentRemovalProto {
                reason,
                ..removal_proto(1)
            };
            let err = removals_from_response(vec![proto], &ids_query(&[1])).unwrap_err();
            assert!(
                matches!(err, Error::ResponseDecodeError { .. }),
                "got: {err:?}"
            );
        }

        // A record of a document the query did not name is not an answer to it.
        let err = removals_from_response(
            vec![removal_proto(1), removal_proto(2)],
            &ids_query(&[1, 3]),
        )
        .unwrap_err();
        assert!(
            matches!(&err, Error::ResponseDecodeError { error } if error.contains("did not name")),
            "got: {err:?}"
        );

        // More records than the read could hold, by ids and by page alike.
        for query in [ids_query(&[1]), page_query(1)] {
            let err = removals_from_response(vec![removal_proto(1), removal_proto(2)], &query)
                .unwrap_err();
            assert!(
                matches!(&err, Error::ResponseDecodeError { error } if error.contains("at most 1")),
                "got: {err:?}"
            );
        }
    }

    #[test]
    fn should_continue_after_a_full_removals_page_only() {
        let query = page_query(2);
        let page = ContractDocumentRemovals(vec![
            ContractDocumentRemovalEntry {
                document_id: id(1),
                removal: removal(1),
            },
            ContractDocumentRemovalEntry {
                document_id: id(2),
                removal: removal(2),
            },
        ]);
        assert_eq!(
            page.next_query(&query),
            Some(ContractDocumentRemovalsQuery {
                document_type_name: POST.to_string(),
                selection: ContractDocumentRemovalsSelection::Page {
                    start_after: Some(id(2)),
                    limit: 2,
                },
            })
        );
        assert_eq!(ContractDocumentRemovals::default().next_query(&query), None);
        // A page shorter than the limit is the last one, and a read by ids has no page after
        // it whatever it held.
        let short = ContractDocumentRemovals(vec![ContractDocumentRemovalEntry {
            document_id: id(1),
            removal: removal(1),
        }]);
        assert_eq!(short.next_query(&query), None);
        assert_eq!(page.next_query(&ids_query(&[1, 2])), None);
    }

    #[test]
    fn should_read_the_fee_pots_of_an_unproved_response() {
        let pots = fee_pots_from_response(ContractFeePotsProto {
            owner: Some(ContractFeePotProto {
                credits: 10_000_000,
                last_claim: None,
            }),
            moderators: Some(ContractFeePotProto {
                credits: u64::MAX,
                // Epoch 0 is an epoch a pot can have been paid out in, not "never".
                last_claim: Some(ContractFeePotLastClaimProto {
                    epoch: 0,
                    time_ms: 1_700_000_000_000,
                    claimant_id: vec![7; 32],
                }),
            }),
        })
        .expect("expected the pots to be read");
        assert_eq!(
            pots,
            ContractFeePots {
                owner: ContractFeePotState {
                    credits: 10_000_000,
                    last_claim: None,
                },
                moderators: ContractFeePotState {
                    credits: u64::MAX,
                    last_claim: Some(ContractFeePotLastClaim {
                        epoch_index: 0,
                        time_ms: 1_700_000_000_000,
                        claimant_id: id(7),
                    }),
                },
            }
        );
        assert_eq!(pots.pot(ContractFeePot::Moderators).credits, u64::MAX);
        assert_eq!(pots.moderators.last_claim_epoch(), Some(0));
        assert_eq!(pots.owner.last_claim_epoch(), None);
    }

    #[test]
    fn should_refuse_fee_pots_a_node_cannot_have_read() {
        let pot = |last_claim| {
            Some(ContractFeePotProto {
                credits: 1,
                last_claim,
            })
        };
        let claim = |epoch, claimant_id| {
            Some(ContractFeePotLastClaimProto {
                epoch,
                time_ms: 1,
                claimant_id,
            })
        };
        for (pots, needle) in [
            (
                ContractFeePotsProto {
                    owner: None,
                    moderators: pot(None),
                },
                "no owner pot",
            ),
            (
                ContractFeePotsProto {
                    owner: pot(None),
                    moderators: None,
                },
                "no moderators pot",
            ),
            (
                ContractFeePotsProto {
                    owner: pot(None),
                    moderators: pot(claim(u32::from(u16::MAX) + 1, vec![7; 32])),
                },
                "is not a u16",
            ),
            (
                ContractFeePotsProto {
                    owner: pot(claim(3, vec![7; 5])),
                    moderators: pot(None),
                },
                "last claimant of the owner pot",
            ),
        ] {
            let err = fee_pots_from_response(pots).expect_err("expected the pots to be refused");
            assert!(
                matches!(&err, Error::ResponseDecodeError { error } if error.contains(needle)),
                "{needle}: {err:?}"
            );
        }
    }
}
