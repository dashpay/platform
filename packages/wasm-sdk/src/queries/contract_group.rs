//! Contract group queries: a group's stored information, one page of its members of one
//! kind, and the groups a contract belongs to.

use crate::error::WasmSdkError;
use crate::queries::utils::{convert_optional_limit, deserialize_required_query};
use crate::queries::ProofMetadataResponseWasm;
use crate::sdk::WasmSdk;
use dash_sdk::platform::contract_groups::{
    ContractGroupInfo, ContractGroupMembersPage, ContractGroupMembersPageQuery,
    ContractGroupMembersQuery, ContractGroupMembershipsForContract,
};
use dash_sdk::platform::{Fetch, Identifier};
use js_sys::Array;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wasm_dpp2::identifier::{IdentifierLikeJs, IdentifierWasm};
use wasm_dpp2::serialization::conversions as serialization;

#[wasm_bindgen(typescript_custom_section)]
const CONTRACT_GROUP_TS: &'static str = r#"
/**
 * Which kind of member of a contract group to read: contracts that joined as a whole,
 * document types, or tokens.
 */
export type ContractGroupMemberKind = 'contracts' | 'documentTypes' | 'tokens';

/**
 * A members page cursor: the last entry of the page before. `documentTypeName` goes with
 * `kind: 'documentTypes'`, `tokenPosition` with `kind: 'tokens'`; neither with `'contracts'`.
 */
export interface ContractGroupMemberCursor {
  contractId: IdentifierLike;
  documentTypeName?: string;
  tokenPosition?: number;
}

/**
 * Query parameters for one page of a contract group's members (`getContractGroupMembers`).
 */
export interface ContractGroupMembersQuery {
  /** The contract group to read. */
  contractGroupId: IdentifierLike;
  /** Which kind of member to read. */
  kind: ContractGroupMemberKind;
  /** Continue after this entry; omit for the first page. Use the page's `nextStartAfter`. */
  startAfter?: ContractGroupMemberCursor;
  /**
   * Maximum number of members to return, 1 to 100.
   * @default 100
   */
  limit?: number;
}

/** A document type that belongs to a contract group. */
export interface ContractGroupDocumentTypeMember {
  contractId: string;
  documentTypeName: string;
}

/** A token that belongs to a contract group. */
export interface ContractGroupTokenMember {
  contractId: string;
  tokenPosition: number;
}

/**
 * One page of a contract group's members of one kind, in key order. Only the list named by
 * `kind` is set. `nextStartAfter` is the cursor of the next page and is absent when this
 * page is empty, which makes it the last one. An absent group and a group with no members
 * of that kind both answer with an empty page; use `getContractGroupInfo` to tell them apart.
 */
export interface ContractGroupMembersPage {
  kind: ContractGroupMemberKind;
  contracts?: string[];
  documentTypes?: ContractGroupDocumentTypeMember[];
  tokens?: ContractGroupTokenMember[];
  nextStartAfter?: {
    contractId: string;
    documentTypeName?: string;
    tokenPosition?: number;
  };
}

/**
 * The contract groups a contract belongs to (`getContractGroupsForContract`): as a whole,
 * through its document types (keyed by name) and through its tokens (keyed by position).
 * Every entry is empty when the contract belongs to no group.
 */
export interface ContractGroupMemberships {
  contract: string[];
  documentTypes: Record<string, string[]>;
  tokens: Record<string, string[]>;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ContractGroupMembersQuery")]
    pub type ContractGroupMembersQueryJs;
}

/// A contract group's stored information: who owns it and who may add members, and its
/// optional name and description.
#[wasm_bindgen(js_name = "ContractGroupInfo")]
#[derive(Clone)]
pub struct ContractGroupInfoWasm {
    owner_id: String,
    admin_ids: Vec<String>,
    name: Option<String>,
    description: Option<String>,
}

impl From<ContractGroupInfo> for ContractGroupInfoWasm {
    fn from(info: ContractGroupInfo) -> Self {
        let owner = info.owner();
        ContractGroupInfoWasm {
            owner_id: IdentifierWasm::from(*owner.owner_id()).to_base58(),
            admin_ids: owner
                .admin_ids()
                .map(|admins| {
                    admins
                        .iter()
                        .map(|admin| IdentifierWasm::from(*admin).to_base58())
                        .collect()
                })
                .unwrap_or_default(),
            name: info.name().map(str::to_string),
            description: info.description().map(str::to_string),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ContractGroupInfoJson {
    owner_id: String,
    admin_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

#[wasm_bindgen(js_class = ContractGroupInfo)]
impl ContractGroupInfoWasm {
    /// The identity that registered the group and owns it, as a base58 string.
    #[wasm_bindgen(getter = "ownerId")]
    pub fn owner_id(&self) -> String {
        self.owner_id.clone()
    }

    /// The identities that may add members besides the owner, as base58 strings. Empty for
    /// a single owner.
    #[wasm_bindgen(getter = "adminIds", unchecked_return_type = "string[]")]
    pub fn admin_ids(&self) -> Array {
        self.admin_ids
            .iter()
            .map(|admin| JsValue::from_str(admin))
            .collect()
    }

    /// The group's name, when it has one.
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> Option<String> {
        self.name.clone()
    }

    /// The group's description, when it has one.
    #[wasm_bindgen(getter)]
    pub fn description(&self) -> Option<String> {
        self.description.clone()
    }

    #[wasm_bindgen(js_name = "toJSON")]
    pub fn to_json(&self) -> Result<JsValue, WasmSdkError> {
        serialization::to_json(&ContractGroupInfoJson {
            owner_id: self.owner_id.clone(),
            admin_ids: self.admin_ids.clone(),
            name: self.name.clone(),
            description: self.description.clone(),
        })
        .map_err(WasmSdkError::from)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
enum ContractGroupMemberKindInput {
    Contracts,
    DocumentTypes,
    Tokens,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContractGroupMemberCursorInput {
    contract_id: IdentifierWasm,
    #[serde(default)]
    document_type_name: Option<String>,
    #[serde(default)]
    token_position: Option<u32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContractGroupMembersQueryInput {
    contract_group_id: IdentifierWasm,
    kind: ContractGroupMemberKindInput,
    #[serde(default)]
    start_after: Option<ContractGroupMemberCursorInput>,
    #[serde(default)]
    limit: Option<u32>,
}

fn parse_contract_group_members_query(
    query: ContractGroupMembersQueryJs,
) -> Result<ContractGroupMembersPageQuery, WasmSdkError> {
    let input: ContractGroupMembersQueryInput = deserialize_required_query(
        query,
        "Query object is required",
        "contract group members query",
    )?;

    let cursor_contract_id = input
        .start_after
        .as_ref()
        .map(|cursor| Identifier::from(cursor.contract_id.clone()));
    let members = match input.kind {
        ContractGroupMemberKindInput::Contracts => ContractGroupMembersQuery::Contracts {
            start_after: cursor_contract_id,
        },
        ContractGroupMemberKindInput::DocumentTypes => {
            let start_after = match (&input.start_after, cursor_contract_id) {
                (Some(cursor), Some(contract_id)) => {
                    let name = cursor.document_type_name.clone().ok_or_else(|| {
                        WasmSdkError::invalid_argument(
                            "startAfter.documentTypeName is required for kind 'documentTypes'"
                                .to_string(),
                        )
                    })?;
                    Some((contract_id, name))
                }
                _ => None,
            };
            ContractGroupMembersQuery::DocumentTypes { start_after }
        }
        ContractGroupMemberKindInput::Tokens => {
            let start_after = match (&input.start_after, cursor_contract_id) {
                (Some(cursor), Some(contract_id)) => {
                    let position = cursor.token_position.ok_or_else(|| {
                        WasmSdkError::invalid_argument(
                            "startAfter.tokenPosition is required for kind 'tokens'".to_string(),
                        )
                    })?;
                    let position = u16::try_from(position).map_err(|_| {
                        WasmSdkError::invalid_argument(format!(
                            "startAfter.tokenPosition {position} exceeds maximum of {}",
                            u16::MAX
                        ))
                    })?;
                    Some((contract_id, position))
                }
                _ => None,
            };
            ContractGroupMembersQuery::Tokens { start_after }
        }
    };

    Ok(ContractGroupMembersPageQuery {
        contract_group_id: input.contract_group_id.into(),
        members,
        limit: convert_optional_limit(input.limit, "limit")?,
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DocumentTypeMemberJson {
    contract_id: String,
    document_type_name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TokenMemberJson {
    contract_id: String,
    token_position: u16,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MemberCursorJson {
    contract_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    document_type_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    token_position: Option<u16>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ContractGroupMembersPageJson {
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    contracts: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    document_types: Option<Vec<DocumentTypeMemberJson>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tokens: Option<Vec<TokenMemberJson>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    next_start_after: Option<MemberCursorJson>,
}

fn base58(id: &Identifier) -> String {
    IdentifierWasm::from(*id).to_base58()
}

fn members_page_to_js(page: ContractGroupMembersPage) -> Result<JsValue, WasmSdkError> {
    // The next query of a non-empty page always carries a cursor: its last entry.
    let next_start_after = page.next_query().and_then(|next| match next {
        ContractGroupMembersQuery::Contracts { start_after } => {
            start_after.map(|contract_id| MemberCursorJson {
                contract_id: base58(&contract_id),
                document_type_name: None,
                token_position: None,
            })
        }
        ContractGroupMembersQuery::DocumentTypes { start_after } => {
            start_after.map(|(contract_id, name)| MemberCursorJson {
                contract_id: base58(&contract_id),
                document_type_name: Some(name),
                token_position: None,
            })
        }
        ContractGroupMembersQuery::Tokens { start_after } => {
            start_after.map(|(contract_id, position)| MemberCursorJson {
                contract_id: base58(&contract_id),
                document_type_name: None,
                token_position: Some(position),
            })
        }
    });
    let json = match page {
        ContractGroupMembersPage::Contracts(contracts) => ContractGroupMembersPageJson {
            kind: "contracts",
            contracts: Some(contracts.iter().map(base58).collect()),
            document_types: None,
            tokens: None,
            next_start_after,
        },
        ContractGroupMembersPage::DocumentTypes(document_types) => ContractGroupMembersPageJson {
            kind: "documentTypes",
            contracts: None,
            document_types: Some(
                document_types
                    .into_iter()
                    .map(|(contract_id, document_type_name)| DocumentTypeMemberJson {
                        contract_id: base58(&contract_id),
                        document_type_name,
                    })
                    .collect(),
            ),
            tokens: None,
            next_start_after,
        },
        ContractGroupMembersPage::Tokens(tokens) => ContractGroupMembersPageJson {
            kind: "tokens",
            contracts: None,
            document_types: None,
            tokens: Some(
                tokens
                    .into_iter()
                    .map(|(contract_id, token_position)| TokenMemberJson {
                        contract_id: base58(&contract_id),
                        token_position,
                    })
                    .collect(),
            ),
            next_start_after,
        },
    };
    serialization::to_object(&json).map_err(WasmSdkError::from)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ContractGroupMembershipsJson {
    contract: Vec<String>,
    document_types: BTreeMap<String, Vec<String>>,
    tokens: BTreeMap<String, Vec<String>>,
}

fn memberships_to_js(
    memberships: ContractGroupMembershipsForContract,
) -> Result<JsValue, WasmSdkError> {
    let ids = |groups: &std::collections::BTreeSet<Identifier>| -> Vec<String> {
        groups.iter().map(base58).collect()
    };
    let json = ContractGroupMembershipsJson {
        contract: ids(&memberships.contract),
        document_types: memberships
            .document_types
            .iter()
            .map(|(name, groups)| (name.clone(), ids(groups)))
            .collect(),
        tokens: memberships
            .tokens
            .iter()
            .map(|(position, groups)| (position.to_string(), ids(groups)))
            .collect(),
    };
    serialization::to_object(&json).map_err(WasmSdkError::from)
}

fn parse_identifier(id: IdentifierLikeJs, what: &str) -> Result<Identifier, WasmSdkError> {
    id.try_into()
        .map_err(|err| WasmSdkError::invalid_argument(format!("Invalid {what}: {err}")))
}

#[wasm_bindgen]
impl WasmSdk {
    /// A contract group's stored information: owner, admins, name and description, or
    /// `undefined` when no group has the id.
    ///
    /// # Example
    /// ```javascript
    /// const info = await sdk.getContractGroupInfo(contractGroupId);
    /// if (info) console.log(info.ownerId, info.adminIds, info.name);
    /// ```
    #[wasm_bindgen(js_name = "getContractGroupInfo")]
    pub async fn get_contract_group_info(
        &self,
        #[wasm_bindgen(js_name = "contractGroupId")] contract_group_id: IdentifierLikeJs,
    ) -> Result<Option<ContractGroupInfoWasm>, WasmSdkError> {
        let contract_group_id = parse_identifier(contract_group_id, "contract group ID")?;

        let info = ContractGroupInfo::fetch(self.as_ref(), contract_group_id).await?;

        Ok(info.map(ContractGroupInfoWasm::from))
    }

    /// A contract group's stored information together with its proof and metadata.
    #[wasm_bindgen(
        js_name = "getContractGroupInfoWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<ContractGroupInfo | undefined>"
    )]
    pub async fn get_contract_group_info_with_proof_info(
        &self,
        #[wasm_bindgen(js_name = "contractGroupId")] contract_group_id: IdentifierLikeJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let contract_group_id = parse_identifier(contract_group_id, "contract group ID")?;

        let (info, metadata, proof) =
            ContractGroupInfo::fetch_with_metadata_and_proof(self.as_ref(), contract_group_id, None)
                .await?;

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            info.map(|info| JsValue::from(ContractGroupInfoWasm::from(info)))
                .unwrap_or(JsValue::UNDEFINED),
            metadata,
            proof,
        ))
    }

    /// One page of a contract group's members of one kind, in key order. Pass the page's
    /// `nextStartAfter` as the next query's `startAfter` to read the following page; a page
    /// without one is the last.
    ///
    /// # Example
    /// ```javascript
    /// let page = await sdk.getContractGroupMembers({ contractGroupId, kind: 'contracts', limit: 50 });
    /// while (page.nextStartAfter) {
    ///   page = await sdk.getContractGroupMembers({ contractGroupId, kind: 'contracts', limit: 50, startAfter: page.nextStartAfter });
    /// }
    /// ```
    #[wasm_bindgen(
        js_name = "getContractGroupMembers",
        unchecked_return_type = "ContractGroupMembersPage"
    )]
    pub async fn get_contract_group_members(
        &self,
        query: ContractGroupMembersQueryJs,
    ) -> Result<JsValue, WasmSdkError> {
        let query = parse_contract_group_members_query(query)?;

        let page = ContractGroupMembersPage::fetch(self.as_ref(), query)
            .await?
            .unwrap_or(ContractGroupMembersPage::Contracts(vec![]));

        members_page_to_js(page)
    }

    /// One page of a contract group's members together with its proof and metadata.
    #[wasm_bindgen(
        js_name = "getContractGroupMembersWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<ContractGroupMembersPage>"
    )]
    pub async fn get_contract_group_members_with_proof_info(
        &self,
        query: ContractGroupMembersQueryJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let query = parse_contract_group_members_query(query)?;

        let (page, metadata, proof) =
            ContractGroupMembersPage::fetch_with_metadata_and_proof(self.as_ref(), query, None)
                .await?;
        let page = page.unwrap_or(ContractGroupMembersPage::Contracts(vec![]));

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            members_page_to_js(page)?,
            metadata,
            proof,
        ))
    }

    /// The contract groups a contract belongs to: as a whole, through its document types and
    /// through its tokens. Every entry is empty when it belongs to no group.
    ///
    /// # Example
    /// ```javascript
    /// const memberships = await sdk.getContractGroupsForContract(contractId);
    /// console.log(memberships.contract, memberships.documentTypes, memberships.tokens);
    /// ```
    #[wasm_bindgen(
        js_name = "getContractGroupsForContract",
        unchecked_return_type = "ContractGroupMemberships"
    )]
    pub async fn get_contract_groups_for_contract(
        &self,
        #[wasm_bindgen(js_name = "contractId")] contract_id: IdentifierLikeJs,
    ) -> Result<JsValue, WasmSdkError> {
        let contract_id = parse_identifier(contract_id, "contract ID")?;

        let memberships = ContractGroupMembershipsForContract::fetch(self.as_ref(), contract_id)
            .await?
            .unwrap_or_default();

        memberships_to_js(memberships)
    }

    /// The contract groups a contract belongs to together with their proof and metadata.
    #[wasm_bindgen(
        js_name = "getContractGroupsForContractWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<ContractGroupMemberships>"
    )]
    pub async fn get_contract_groups_for_contract_with_proof_info(
        &self,
        #[wasm_bindgen(js_name = "contractId")] contract_id: IdentifierLikeJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let contract_id = parse_identifier(contract_id, "contract ID")?;

        let (memberships, metadata, proof) =
            ContractGroupMembershipsForContract::fetch_with_metadata_and_proof(
                self.as_ref(),
                contract_id,
                None,
            )
            .await?;

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            memberships_to_js(memberships.unwrap_or_default())?,
            metadata,
            proof,
        ))
    }
}
