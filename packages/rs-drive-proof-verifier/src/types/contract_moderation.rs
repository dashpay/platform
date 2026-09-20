//! Contract moderation query results and the wire conversions the proved and unproved paths
//! share: one identity's status on the lists queried ([`ContractModerationListStatuses`]) and one
//! page of a contract's banlist or suspension list ([`ContractModerationEntries`], read with a
//! [`ContractModerationEntriesQuery`]). The fee pots of a contract ([`ContractFeePots`]) are
//! read here too: they are what its document action fees pay its owner and its moderators.

use crate::Error;
use dapi_grpc::platform::v0::get_contract_fee_pots_response::{
    ContractFeePot as ContractFeePotProto, ContractFeePots as ContractFeePotsProto,
};
use dapi_grpc::platform::v0::get_contract_moderation_entries_response::ContractModerationEntry as ContractModerationEntryProto;
use dapi_grpc::platform::v0::ContractModerationList as ContractModerationListProto;
use dapi_grpc::platform::v0::ContractModerationReason as ContractModerationReasonProto;
pub use dpp::data_contract::config::moderation::{
    ContractBan, ContractModerationList, ContractModerationListStatus,
    ContractModerationListStatuses, ContractModerationReason, ContractModerationStatus,
    ContractSuspension,
};
pub use dpp::data_contract::document_type::action_fees::ContractFeePot;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
pub use drive::drive::contract::fee_pots::types::{ContractFeePotState, ContractFeePots};
pub use drive::drive::contract::moderation::types::{
    ContractModerationEntriesQuery, ContractModerationEntry,
};

/// The page size a request without a limit asks for, which is also the largest page a node
/// returns: the platform version's `max_returned_elements`, the number the node reads too.
pub fn default_contract_moderation_entries_limit(platform_version: &PlatformVersion) -> u16 {
    platform_version.drive_abci.query.max_returned_elements
}

/// One page of a moderated contract's banlist or suspension list, in identity id order. A page
/// shorter than the limit is the last one.
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

/// The 32 byte identifier a request field holds, naming the field in the error.
pub fn identifier_from_request(bytes: &[u8], what: &str) -> Result<Identifier, Error> {
    Identifier::from_bytes(bytes).map_err(|_| Error::RequestError {
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
/// response without it is refused, and so is a code that is not a u16, which no entry of any
/// version can hold. The length of the text is not checked: its limit belongs to a protocol
/// version and may be raised by a later one, the proved path reads whatever the proof holds,
/// and the two must agree on which stored reasons a client can read.
pub fn reason_from_response(
    reason: Option<ContractModerationReasonProto>,
) -> Result<ContractModerationReason, Error> {
    let ContractModerationReasonProto { code, text } =
        reason.ok_or(Error::ResponseDecodeError {
            error: "contract moderation entry holds no reason".to_string(),
        })?;
    let code = code
        .map(|code| {
            u16::try_from(code).map_err(|_| Error::ResponseDecodeError {
                error: format!("contract moderation reason code {code} is not a u16"),
            })
        })
        .transpose()?;
    Ok(ContractModerationReason { code, text })
}

/// The entries of an unproved response.
pub fn entries_from_response(
    entries: Vec<ContractModerationEntryProto>,
) -> Result<ContractModerationEntries, Error> {
    entries
        .into_iter()
        .map(|entry| {
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
                reason: reason_from_response(entry.reason)?,
            })
        })
        .collect::<Result<Vec<_>, Error>>()
        .map(ContractModerationEntries)
}

/// One pot of an unproved response. The epoch a pot was last paid out in is a u16 on the chain,
/// so a response naming a larger one is refused: no pot of any version can hold it.
fn fee_pot_from_response(
    pot: Option<ContractFeePotProto>,
    what: &str,
) -> Result<ContractFeePotState, Error> {
    let ContractFeePotProto {
        credits,
        last_claim_epoch,
    } = pot.ok_or_else(|| Error::ResponseDecodeError {
        error: format!("contract fee pots response holds no {what} pot"),
    })?;
    let last_claim_epoch = last_claim_epoch
        .map(|epoch| {
            u16::try_from(epoch).map_err(|_| Error::ResponseDecodeError {
                error: format!("last claim epoch {epoch} of the {what} pot is not a u16"),
            })
        })
        .transpose()?;
    Ok(ContractFeePotState {
        credits,
        last_claim_epoch,
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
            lists_from_request(&[2, 1]).expect("expected lists"),
            vec![
                ContractModerationList::Suspensions,
                ContractModerationList::Banlist
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
                }),
            },
            ContractModerationEntryProto {
                identity_id: id(2).to_vec(),
                until: Some(99),
                reason: Some(ContractModerationReasonProto {
                    code: Some(3),
                    text: String::new(),
                }),
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
                },
                ContractModerationEntry {
                    identity_id: id(2),
                    until: Some(99),
                    reason: ContractModerationReason {
                        code: Some(3),
                        text: String::new(),
                    },
                }
            ]
        );

        let err = entries_from_response(vec![ContractModerationEntryProto {
            identity_id: vec![1; 5],
            until: None,
            reason: Some(ContractModerationReasonProto::default()),
        }])
        .unwrap_err();
        assert!(matches!(err, Error::ProtocolError { .. }), "got: {err:?}");

        // Every entry carries a reason
        let err = entries_from_response(vec![ContractModerationEntryProto {
            identity_id: id(1).to_vec(),
            until: None,
            reason: None,
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
            }),
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
            },
            ContractModerationEntry {
                identity_id: id(2),
                until: Some(6),
                reason: ContractModerationReason::default(),
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
        }]);
        assert_eq!(short.next_query(&query), None);
    }

    #[test]
    fn should_read_the_fee_pots_of_an_unproved_response() {
        let pots = fee_pots_from_response(ContractFeePotsProto {
            owner: Some(ContractFeePotProto {
                credits: 10_000_000,
                last_claim_epoch: None,
            }),
            moderators: Some(ContractFeePotProto {
                credits: u64::MAX,
                // Epoch 0 is an epoch a pot can have been paid out in, not "never".
                last_claim_epoch: Some(0),
            }),
        })
        .expect("expected the pots to be read");
        assert_eq!(
            pots,
            ContractFeePots {
                owner: ContractFeePotState {
                    credits: 10_000_000,
                    last_claim_epoch: None,
                },
                moderators: ContractFeePotState {
                    credits: u64::MAX,
                    last_claim_epoch: Some(0),
                },
            }
        );
        assert_eq!(pots.pot(ContractFeePot::Moderators).credits, u64::MAX);
    }

    #[test]
    fn should_refuse_fee_pots_a_node_cannot_have_read() {
        let pot = |last_claim_epoch| {
            Some(ContractFeePotProto {
                credits: 1,
                last_claim_epoch,
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
                    moderators: pot(Some(u32::from(u16::MAX) + 1)),
                },
                "is not a u16",
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
