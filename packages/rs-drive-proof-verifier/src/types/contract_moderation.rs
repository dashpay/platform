//! Contract moderation query results and the wire conversions the proved and unproved paths
//! share: one identity's status on a moderated contract ([`ContractModerationStatus`]) and one
//! page of a contract's banlist or suspension list ([`ContractModerationEntries`], read with a
//! [`ContractModerationEntriesQuery`]).

use crate::Error;
use dapi_grpc::platform::v0::get_contract_moderation_entries_response::ContractModerationEntry as ContractModerationEntryProto;
use dapi_grpc::platform::v0::ContractModerationList as ContractModerationListProto;
pub use dpp::data_contract::config::moderation::{
    ContractModerationList, ContractModerationStatus,
};
use dpp::identifier::Identifier;
pub use drive::drive::contract::moderation::types::{
    ContractModerationEntriesQuery, ContractModerationEntry,
};

/// The page size a request without a limit asks for.
pub const DEFAULT_CONTRACT_MODERATION_ENTRIES_LIMIT: u16 = 100;

/// One page of a moderated contract's banlist or suspension list, in identity id order. A page
/// shorter than the limit is the last one.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ContractModerationEntries(pub Vec<ContractModerationEntry>);

impl ContractModerationEntries {
    /// The entries of the page.
    pub fn entries(&self) -> &[ContractModerationEntry] {
        &self.0
    }

    /// The query for the page after this one, or `None` when this page is empty.
    pub fn next_query(
        &self,
        query: &ContractModerationEntriesQuery,
    ) -> Option<ContractModerationEntriesQuery> {
        self.0.last().map(|entry| ContractModerationEntriesQuery {
            list: query.list,
            start_after: Some(entry.identity_id),
            limit: query.limit,
        })
    }
}

fn identifier_from(bytes: &[u8], what: &str) -> Result<Identifier, Error> {
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
        Err(_) => Err(Error::RequestError {
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
) -> Result<ContractModerationEntriesQuery, Error> {
    let limit = match limit {
        None => DEFAULT_CONTRACT_MODERATION_ENTRIES_LIMIT,
        Some(limit) => u16::try_from(limit).map_err(|_| Error::RequestError {
            error: format!("limit {limit} is out of bounds"),
        })?,
    };
    Ok(ContractModerationEntriesQuery {
        list: list_from_request(list, "list")?,
        start_after: start_after
            .map(|bytes| identifier_from(bytes, "start_after"))
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
            lists_from_request(&[1, 0]).expect("expected lists"),
            vec![
                ContractModerationList::Suspensions,
                ContractModerationList::Banlist
            ]
        );
        for (lists, needle) in [
            (&[][..], "at least one"),
            (&[0, 0][..], "twice"),
            (&[0, 9][..], "not a moderation list"),
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
        assert_eq!(
            entries_query_from_request(0, None, None).expect("expected a query"),
            ContractModerationEntriesQuery {
                list: ContractModerationList::Banlist,
                start_after: None,
                limit: DEFAULT_CONTRACT_MODERATION_ENTRIES_LIMIT,
            }
        );
        assert_eq!(
            entries_query_from_request(1, Some(id(3).as_slice()), Some(5))
                .expect("expected a query"),
            ContractModerationEntriesQuery {
                list: ContractModerationList::Suspensions,
                start_after: Some(id(3)),
                limit: 5,
            }
        );
        for (list, start_after, limit, needle) in [
            (9, None, None, "not a moderation list"),
            (0, Some(&[1u8; 5][..]), None, "start_after"),
            (0, None, Some(u16::MAX as u32 + 1), "out of bounds"),
        ] {
            let err = entries_query_from_request(list, start_after, limit).unwrap_err();
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
    fn should_continue_after_the_last_entry_and_stop_on_an_empty_page() {
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
    }
}
