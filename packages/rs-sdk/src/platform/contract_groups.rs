//! Contract group queries: a group's stored information (`getContractGroupInfo`), one page of
//! its members of one kind (`getContractGroupMembers`) and the groups a contract belongs to
//! (`getContractGroupsForContract`).
//!
//! A contract group is an identity-owned set of contracts, contract document types and
//! contract tokens; see `dpp::contract_group`. Its id is not on the wire: it derives from the
//! registering identity and the identity nonce of the create transition with
//! [`generate_contract_group_id`](dpp::contract_group::generate_contract_group_id).
//!
//! * [`ContractGroupInfo::fetch`] with a group id returns the owner, admins, name and
//!   description, or `None` when no such group exists.
//! * [`ContractGroupMembersPage::fetch`] with a [`ContractGroupMembersPageQuery`] returns one
//!   page of members of one kind in key order; the page's
//!   [`next_query`](ContractGroupMembersPage::next_query) is the cursor of the next page. An
//!   absent group and a group with no members of that kind both answer with an empty page:
//!   fetch the info to tell them apart.
//! * [`ContractGroupMembershipsForContract::fetch`] with a contract id returns the groups the
//!   contract belongs to, as a whole, through its document types and through its tokens;
//!   every set is empty when it belongs to none.
//!
//! Each type also implements [`FetchUnproved`] for the unverified fast path.

use crate::platform::{Fetch, FetchUnproved, Identifier, Query, QuerySettings};
use crate::Error;
use dapi_grpc::platform::v0::get_contract_group_info_request::GetContractGroupInfoRequestV0;
use dapi_grpc::platform::v0::get_contract_group_members_request::get_contract_group_members_request_v0::Members;
use dapi_grpc::platform::v0::get_contract_group_members_request::{
    ContractMembersQuery, DocumentTypeMembersQuery, GetContractGroupMembersRequestV0,
    TokenMembersQuery,
};
use dapi_grpc::platform::v0::get_contract_groups_for_contract_request::GetContractGroupsForContractRequestV0;
use dapi_grpc::platform::v0::{
    get_contract_group_info_request, get_contract_group_members_request,
    get_contract_groups_for_contract_request, ContractGroupDocumentTypeMember,
    ContractGroupTokenMember, GetContractGroupInfoRequest, GetContractGroupMembersRequest,
    GetContractGroupsForContractRequest,
};
pub use drive_proof_verifier::types::contract_groups::{
    ContractGroupInfo, ContractGroupMembersPage, ContractGroupMembersQuery,
    ContractGroupMembershipsForContract, ContractGroupOwner,
};

/// Query for one page of a contract group's members of one kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractGroupMembersPageQuery {
    /// The group whose members to read.
    pub contract_group_id: Identifier,
    /// Which kind of member to read, and the cursor to continue after.
    pub members: ContractGroupMembersQuery,
    /// At most this many members, between 1 and the protocol page cap (100). `None` means
    /// the cap.
    pub limit: Option<u16>,
}

impl ContractGroupMembersPageQuery {
    /// The first page of a group's members of one kind, up to the protocol page cap.
    pub fn new(contract_group_id: Identifier, members: ContractGroupMembersQuery) -> Self {
        Self {
            contract_group_id,
            members,
            limit: None,
        }
    }

    /// The first page of the contracts that belong to the group as a whole.
    pub fn contracts(contract_group_id: Identifier) -> Self {
        Self::new(
            contract_group_id,
            ContractGroupMembersQuery::Contracts { start_after: None },
        )
    }

    /// The first page of the document types that belong to the group.
    pub fn document_types(contract_group_id: Identifier) -> Self {
        Self::new(
            contract_group_id,
            ContractGroupMembersQuery::DocumentTypes { start_after: None },
        )
    }

    /// The first page of the tokens that belong to the group.
    pub fn tokens(contract_group_id: Identifier) -> Self {
        Self::new(
            contract_group_id,
            ContractGroupMembersQuery::Tokens { start_after: None },
        )
    }

    /// Bounds the page to `limit` members.
    pub fn with_limit(mut self, limit: u16) -> Self {
        self.limit = Some(limit);
        self
    }

    /// The query for the page after `page`, with the same group and limit, or `None` when
    /// `page` is empty and so was the last one.
    pub fn after(&self, page: &ContractGroupMembersPage) -> Option<Self> {
        page.next_query().map(|members| Self {
            contract_group_id: self.contract_group_id,
            members,
            limit: self.limit,
        })
    }
}

impl Query<GetContractGroupMembersRequest> for ContractGroupMembersPageQuery {
    fn query(&self, settings: &QuerySettings<'_>) -> Result<GetContractGroupMembersRequest, Error> {
        let members = match &self.members {
            ContractGroupMembersQuery::Contracts { start_after } => {
                Members::Contracts(ContractMembersQuery {
                    start_after: start_after.map(|id| id.to_vec()),
                })
            }
            ContractGroupMembersQuery::DocumentTypes { start_after } => {
                Members::DocumentTypes(DocumentTypeMembersQuery {
                    start_after: start_after.as_ref().map(|(contract_id, name)| {
                        ContractGroupDocumentTypeMember {
                            contract_id: contract_id.to_vec(),
                            document_type_name: name.clone(),
                        }
                    }),
                })
            }
            ContractGroupMembersQuery::Tokens { start_after } => {
                Members::Tokens(TokenMembersQuery {
                    start_after: start_after.map(|(contract_id, position)| {
                        ContractGroupTokenMember {
                            contract_id: contract_id.to_vec(),
                            token_position: position as u32,
                        }
                    }),
                })
            }
        };
        Ok(GetContractGroupMembersRequest {
            version: Some(get_contract_group_members_request::Version::V0(
                GetContractGroupMembersRequestV0 {
                    contract_group_id: self.contract_group_id.to_vec(),
                    members: Some(members),
                    limit: self.limit.map(u32::from),
                    prove: settings.prove,
                },
            )),
        })
    }
}

impl Query<GetContractGroupInfoRequest> for Identifier {
    fn query(&self, settings: &QuerySettings<'_>) -> Result<GetContractGroupInfoRequest, Error> {
        Ok(GetContractGroupInfoRequest {
            version: Some(get_contract_group_info_request::Version::V0(
                GetContractGroupInfoRequestV0 {
                    contract_group_id: self.to_vec(),
                    prove: settings.prove,
                },
            )),
        })
    }
}

impl Query<GetContractGroupsForContractRequest> for Identifier {
    fn query(
        &self,
        settings: &QuerySettings<'_>,
    ) -> Result<GetContractGroupsForContractRequest, Error> {
        Ok(GetContractGroupsForContractRequest {
            version: Some(get_contract_groups_for_contract_request::Version::V0(
                GetContractGroupsForContractRequestV0 {
                    contract_id: self.to_vec(),
                    prove: settings.prove,
                },
            )),
        })
    }
}

impl Fetch for ContractGroupInfo {
    type Query = GetContractGroupInfoRequest;
    type Request = GetContractGroupInfoRequest;
}

impl FetchUnproved for ContractGroupInfo {
    type Request = GetContractGroupInfoRequest;
}

impl Fetch for ContractGroupMembersPage {
    type Query = GetContractGroupMembersRequest;
    type Request = GetContractGroupMembersRequest;
}

impl FetchUnproved for ContractGroupMembersPage {
    type Request = GetContractGroupMembersRequest;
}

impl Fetch for ContractGroupMembershipsForContract {
    type Query = GetContractGroupsForContractRequest;
    type Request = GetContractGroupsForContractRequest;
}

impl FetchUnproved for ContractGroupMembershipsForContract {
    type Request = GetContractGroupsForContractRequest;
}
