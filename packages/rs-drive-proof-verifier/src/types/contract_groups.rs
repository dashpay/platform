//! Contract group query results and the wire conversions the proved and unproved paths share.
//!
//! The result types are the ones Drive reads and verifies: a group's stored information
//! ([`ContractGroupInfo`]), one page of its members of one kind ([`ContractGroupMembersPage`],
//! read with a [`ContractGroupMembersQuery`]), and the groups one contract belongs to
//! ([`ContractGroupMembershipsForContract`]).

use crate::Error;
use dapi_grpc::platform::v0::get_contract_group_info_response::ContractGroupInfo as ContractGroupInfoProto;
use dapi_grpc::platform::v0::get_contract_group_members_request::get_contract_group_members_request_v0::Members;
use dapi_grpc::platform::v0::get_contract_group_members_response::get_contract_group_members_response_v0::Result as MembersResult;
use dapi_grpc::platform::v0::get_contract_groups_for_contract_response::ContractGroupMemberships;
use dapi_grpc::platform::v0::{ContractGroupDocumentTypeMember, ContractGroupTokenMember};
pub use dpp::contract_group::{ContractGroupInfo, ContractGroupInfoV0, ContractGroupOwner};
use dpp::data_contract::TokenContractPosition;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
pub use drive::drive::contract_groups::types::{
    ContractGroupMembersPage, ContractGroupMembersQuery, ContractGroupMembershipsForContract,
};
use std::collections::BTreeSet;

fn identifier_from(bytes: &[u8], what: &str) -> Result<Identifier, Error> {
    Identifier::from_bytes(bytes).map_err(|_| Error::RequestError {
        error: format!(
            "{what} must be a 32 byte identifier, got {} bytes",
            bytes.len()
        ),
    })
}

fn token_position_from(token_position: u32, what: &str) -> Result<TokenContractPosition, Error> {
    TokenContractPosition::try_from(token_position).map_err(|_| Error::RequestError {
        error: format!(
            "{what} {token_position} is out of bounds, it must be at most {}",
            TokenContractPosition::MAX
        ),
    })
}

/// The stored information a response carries, as the protocol type. Empty admins mean a
/// single owner.
pub(crate) fn contract_group_info_from_proto(
    info: ContractGroupInfoProto,
) -> Result<ContractGroupInfo, Error> {
    let ContractGroupInfoProto {
        owner_id,
        admin_ids,
        name,
        description,
    } = info;
    let owner = identifier_from(&owner_id, "owner_id")?;
    let admins = admin_ids
        .iter()
        .map(|admin| identifier_from(admin, "admin_ids entry"))
        .collect::<Result<BTreeSet<Identifier>, Error>>()?;
    let owner = if admins.is_empty() {
        ContractGroupOwner::SingleOwner(owner)
    } else {
        ContractGroupOwner::OwnerAndAdmins { owner, admins }
    };
    Ok(ContractGroupInfo::V0(ContractGroupInfoV0 {
        owner,
        name,
        description,
    }))
}

/// The members kind and cursor of a request as the query Drive reads and verifies pages with.
pub(crate) fn members_query_from_request(
    members: Option<Members>,
) -> Result<ContractGroupMembersQuery, Error> {
    let members = members.ok_or_else(|| Error::RequestError {
        error: "members must select contracts, document types or tokens".to_string(),
    })?;
    Ok(match members {
        Members::Contracts(query) => ContractGroupMembersQuery::Contracts {
            start_after: query
                .start_after
                .map(|contract_id| identifier_from(&contract_id, "start_after"))
                .transpose()?,
        },
        Members::DocumentTypes(query) => ContractGroupMembersQuery::DocumentTypes {
            start_after: query
                .start_after
                .map(|cursor| {
                    let ContractGroupDocumentTypeMember {
                        contract_id,
                        document_type_name,
                    } = cursor;
                    Ok::<_, Error>((
                        identifier_from(&contract_id, "start_after.contract_id")?,
                        document_type_name,
                    ))
                })
                .transpose()?,
        },
        Members::Tokens(query) => ContractGroupMembersQuery::Tokens {
            start_after: query
                .start_after
                .map(|cursor| {
                    let ContractGroupTokenMember {
                        contract_id,
                        token_position,
                    } = cursor;
                    Ok::<_, Error>((
                        identifier_from(&contract_id, "start_after.contract_id")?,
                        token_position_from(token_position, "start_after.token_position")?,
                    ))
                })
                .transpose()?,
        },
    })
}

/// The page limit a request asks for: the protocol page cap (`max_returned_elements`) when
/// absent, which is also the upper bound. The same rule the node applies, so a client
/// verifying the proof rebuilds the page query the node answered.
pub(crate) fn members_limit_from_request(
    limit: Option<u32>,
    platform_version: &PlatformVersion,
) -> Result<u16, Error> {
    let max_returned_elements = platform_version.drive_abci.query.max_returned_elements;
    match limit {
        None => Ok(max_returned_elements),
        Some(requested) => match u16::try_from(requested) {
            Ok(limit) if (1..=max_returned_elements).contains(&limit) => Ok(limit),
            _ => Err(Error::RequestError {
                error: format!(
                    "limit {requested} is out of bounds, it must be between 1 and {max_returned_elements}"
                ),
            }),
        },
    }
}

/// One members page as an unproved response carries it. The page must be of the kind the
/// query asked for.
pub(crate) fn members_page_from_response(
    query: &ContractGroupMembersQuery,
    result: MembersResult,
) -> Result<ContractGroupMembersPage, Error> {
    let page = match result {
        MembersResult::Contracts(members) => ContractGroupMembersPage::Contracts(
            members
                .contract_ids
                .iter()
                .map(|contract_id| identifier_from(contract_id, "contract_ids entry"))
                .collect::<Result<Vec<_>, Error>>()?,
        ),
        MembersResult::DocumentTypes(members) => ContractGroupMembersPage::DocumentTypes(
            members
                .document_types
                .into_iter()
                .map(|member| {
                    Ok((
                        identifier_from(&member.contract_id, "document type contract_id")?,
                        member.document_type_name,
                    ))
                })
                .collect::<Result<Vec<_>, Error>>()?,
        ),
        MembersResult::Tokens(members) => ContractGroupMembersPage::Tokens(
            members
                .tokens
                .into_iter()
                .map(|member| {
                    Ok((
                        identifier_from(&member.contract_id, "token contract_id")?,
                        token_position_from(member.token_position, "token_position")?,
                    ))
                })
                .collect::<Result<Vec<_>, Error>>()?,
        ),
        MembersResult::Proof(_) => {
            return Err(Error::ResponseDecodeError {
                error: "expected unproved contract group members, got a proof".to_string(),
            })
        }
    };
    let same_kind = matches!(
        (query, &page),
        (
            ContractGroupMembersQuery::Contracts { .. },
            ContractGroupMembersPage::Contracts(_)
        ) | (
            ContractGroupMembersQuery::DocumentTypes { .. },
            ContractGroupMembersPage::DocumentTypes(_)
        ) | (
            ContractGroupMembersQuery::Tokens { .. },
            ContractGroupMembersPage::Tokens(_)
        )
    );
    if !same_kind {
        return Err(Error::ResponseDecodeError {
            error: format!("the members page does not match the requested kind {query:?}"),
        });
    }
    Ok(page)
}

/// The memberships of a contract as an unproved response carries them.
pub(crate) fn memberships_from_response(
    memberships: ContractGroupMemberships,
) -> Result<ContractGroupMembershipsForContract, Error> {
    let group_ids = |ids: &[Vec<u8>]| {
        ids.iter()
            .map(|id| identifier_from(id, "contract_group_ids entry"))
            .collect::<Result<BTreeSet<Identifier>, Error>>()
    };
    Ok(ContractGroupMembershipsForContract {
        contract: group_ids(&memberships.contract_group_ids)?,
        document_types: memberships
            .document_types
            .into_iter()
            .map(|entry| {
                Ok((
                    entry.document_type_name,
                    group_ids(&entry.contract_group_ids)?,
                ))
            })
            .collect::<Result<_, Error>>()?,
        tokens: memberships
            .tokens
            .into_iter()
            .map(|entry| {
                Ok((
                    token_position_from(entry.token_position, "token_position")?,
                    group_ids(&entry.contract_group_ids)?,
                ))
            })
            .collect::<Result<_, Error>>()?,
    })
}
