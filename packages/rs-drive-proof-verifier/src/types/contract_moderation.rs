//! Contract moderation query results and the wire conversions the proved and unproved paths
//! share: one identity's status on the lists queried ([`ContractModerationListStatuses`]) and one
//! page of a contract's banlist or suspension list ([`ContractModerationEntries`], read with a
//! [`ContractModerationEntriesQuery`]).

use crate::Error;
use dapi_grpc::platform::v0::get_contract_moderation_entries_response::ContractModerationEntry as ContractModerationEntryProto;
use dapi_grpc::platform::v0::ContractModerationList as ContractModerationListProto;
pub use dpp::data_contract::config::moderation::{
    ContractModerationList, ContractModerationListStatus, ContractModerationListStatuses,
    ContractModerationStatus,
};
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
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
            })
        })
        .collect::<Result<Vec<_>, Error>>()
        .map(ContractModerationEntries)
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
            banned: true,
            suspended_until: None,
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
                banned: false,
                suspended_until: Some(10),
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
            },
            ContractModerationEntryProto {
                identity_id: id(2).to_vec(),
                until: Some(99),
            },
        ])
        .expect("expected entries");
        assert_eq!(
            page.entries(),
            &[
                ContractModerationEntry {
                    identity_id: id(1),
                    until: None
                },
                ContractModerationEntry {
                    identity_id: id(2),
                    until: Some(99)
                }
            ]
        );

        let err = entries_from_response(vec![ContractModerationEntryProto {
            identity_id: vec![1; 5],
            until: None,
        }])
        .unwrap_err();
        assert!(matches!(err, Error::ProtocolError { .. }), "got: {err:?}");
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
            },
            ContractModerationEntry {
                identity_id: id(2),
                until: Some(6),
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
        }]);
        assert_eq!(short.next_query(&query), None);
    }
}
