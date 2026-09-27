//! The moderation charters system contract: who moderates a contract that declares elected
//! moderation, the proposals and join requests behind it, and the two requests members send the
//! leader. Thin bindings over `dash_sdk::platform::moderation_charters`, where every read is a
//! proved document query on the system contract.

use crate::encrypted_for::message_from_options;
use crate::error::WasmSdkError;
use crate::sdk::WasmSdk;
use dash_sdk::dpp::moderation_charter::{
    ELECTED_CHARTER_DOCUMENT_TYPE_NAME, JOIN_REQUEST_DOCUMENT_TYPE_NAME,
    MODERATION_CHARTERS_CONTRACT_ID, RESIGNATION_REQUEST_DOCUMENT_TYPE_NAME,
    SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME,
};
use dash_sdk::platform::moderation_charters::{
    CharterDocumentsPage, JoinRequestInput, ModerationCharterRequest, ModerationTeam,
    ResignationRequestInput,
};
use dash_sdk::platform::{Document, Fetch, Identifier, Identity};
use drive_proof_verifier::types::Documents;
use js_sys::{Array, Map};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wasm_dpp2::data_contract::document::DocumentWasm;
use wasm_dpp2::identifier::{IdentifierLikeJs, IdentifierWasm};
use wasm_dpp2::identity::IdentityWasm;
use wasm_dpp2::utils::{try_from_options_optional_with, try_to_u32};
use wasm_dpp2::PrivateKeyWasm;

#[wasm_bindgen(typescript_custom_section)]
const MODERATION_CHARTERS_TS: &'static str = r#"
/** One page of the proposals for a contract (`getModerationSubmittedCharters`). */
export interface ModerationSubmittedChartersQuery {
  /** The contract the proposals are for. */
  targetContractId: IdentifierLike;
  /** Continue after this proposal; omit for the first page. */
  startAfter?: IdentifierLike;
  /**
   * Maximum number of proposals to return, 1 to 100.
   * @default 100
   */
  limit?: number;
}

/** One page of the join requests for a proposal (`getModerationJoinRequests`). */
export interface ModerationJoinRequestsQuery {
  /** The proposal, a `submittedCharter` document. */
  submittedCharterId: IdentifierLike;
  /** Continue after this join request; omit for the first page. */
  startAfter?: IdentifierLike;
  /**
   * Maximum number of join requests to return, 1 to 100.
   * @default 100
   */
  limit?: number;
}

/**
 * Options for `buildModerationJoinRequest`: an identity's offer to serve on the team of a
 * proposal, with a message only the proposal's leader can read.
 */
export interface ModerationJoinRequestOptions {
  /** The proposal, a `submittedCharter` document. */
  submittedCharterId: IdentifierLike;
  /** Why the writer wants to join, at most 1023 bytes. A string is encoded as UTF-8. */
  message: Uint8Array | string;
  /** The identity offering to serve, the request's owner: an `Identity`, or its id to fetch it. */
  writer: Identity | IdentifierLike;
  /** The private half of the writer's encryption key bound to `joinRequest`. */
  writerEncryptionKey: PrivateKey;
}

/**
 * Options for `buildModerationResignationRequest`: a member's request to leave a seated team,
 * with a message only the leader can read. The leader acts on it with a removal; deleting the
 * document withdraws it.
 */
export interface ModerationResignationRequestOptions {
  /** The seated charter, an `electedCharter` document. */
  electedCharterId: IdentifierLike;
  /** Why the member leaves, at most 1023 bytes. A string is encoded as UTF-8. */
  message: Uint8Array | string;
  /** The member, the request's owner: an `Identity`, or its id to fetch it. */
  writer: Identity | IdentifierLike;
  /** The private half of the writer's encryption key bound to `joinRequest`. */
  writerEncryptionKey: PrivateKey;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ModerationSubmittedChartersQuery")]
    pub type ModerationSubmittedChartersQueryJs;

    #[wasm_bindgen(typescript_type = "ModerationJoinRequestsQuery")]
    pub type ModerationJoinRequestsQueryJs;

    #[wasm_bindgen(typescript_type = "ModerationJoinRequestOptions")]
    pub type ModerationJoinRequestOptionsJs;

    #[wasm_bindgen(typescript_type = "ModerationResignationRequestOptions")]
    pub type ModerationResignationRequestOptionsJs;
}

/// The team that moderates a contract: the seated charter's leader and its active members.
#[wasm_bindgen(js_name = "ModerationTeam")]
#[derive(Clone)]
pub struct ModerationTeamWasm(ModerationTeam);

#[wasm_bindgen(js_class = ModerationTeam)]
impl ModerationTeamWasm {
    /// The seated `electedCharter` document.
    #[wasm_bindgen(getter = electedCharterId)]
    pub fn elected_charter_id(&self) -> IdentifierWasm {
        self.0.elected_charter_id.into()
    }

    /// The proposal the team runs on.
    #[wasm_bindgen(getter = submittedCharterId)]
    pub fn submitted_charter_id(&self) -> IdentifierWasm {
        self.0.submitted_charter_id.into()
    }

    /// The leader, the owner of the elected charter.
    #[wasm_bindgen(getter = leaderId)]
    pub fn leader_id(&self) -> IdentifierWasm {
        self.0.leader_id.into()
    }

    /// The members besides the leader, in id order: the elected members and those the leader
    /// added, less those the leader removed.
    #[wasm_bindgen(getter = members, unchecked_return_type = "Identifier[]")]
    pub fn members(&self) -> Array {
        self.0
            .members
            .iter()
            .map(|id| JsValue::from(IdentifierWasm::from(*id)))
            .collect()
    }

    /// Whether `identityId` is on the team: the leader or an active member.
    #[wasm_bindgen(js_name = "contains")]
    pub fn contains(
        &self,
        #[wasm_bindgen(js_name = "identityId")] identity_id: IdentifierLikeJs,
    ) -> Result<bool, WasmSdkError> {
        let identity_id: Identifier = identity_id.try_into()?;
        Ok(self.0.contains(&identity_id))
    }
}

/// The identifier `id_field` of a page query and the page it asks for. Each field is read as
/// an `IdentifierLike` on its own: an `Identifier` instance does not survive the serde
/// conversion a whole-object deserialization goes through.
fn page_query(
    query: &JsValue,
    id_field: &str,
) -> Result<(Identifier, CharterDocumentsPage), WasmSdkError> {
    if query.is_undefined() || query.is_null() {
        return Err(WasmSdkError::invalid_argument("Query object is required"));
    }
    let id = IdentifierWasm::try_from_options(query, id_field)?.into();
    let start_after =
        IdentifierWasm::try_from_optional_options(query, "startAfter")?.map(Identifier::from);
    let limit = try_from_options_optional_with(query, "limit", |value| try_to_u32(value, "limit"))?;
    Ok((id, CharterDocumentsPage { limit, start_after }))
}

fn document_wasm(document: Document, document_type_name: &str) -> DocumentWasm {
    DocumentWasm::new(
        document,
        MODERATION_CHARTERS_CONTRACT_ID,
        document_type_name.to_string(),
        None,
    )
}

fn documents_map(documents: Documents, document_type_name: &str) -> Map {
    let map = Map::new();
    for (id, document) in documents {
        let key: JsValue = IdentifierWasm::from(id).to_base58().into();
        match document {
            Some(document) => {
                map.set(
                    &key,
                    &JsValue::from(document_wasm(document, document_type_name)),
                );
            }
            None => {
                map.set(&key, &JsValue::NULL);
            }
        }
    }
    map
}

fn request_wasm(request: ModerationCharterRequest) -> DocumentWasm {
    DocumentWasm::new(
        request.document,
        MODERATION_CHARTERS_CONTRACT_ID,
        request.document_type_name,
        Some(request.entropy.0),
    )
}

impl WasmSdk {
    /// The writer of a request: the `Identity` given, or the identity fetched by the id given.
    async fn writer_from_options(&self, options: &JsValue) -> Result<Identity, WasmSdkError> {
        let value = js_sys::Reflect::get(options, &JsValue::from_str("writer"))
            .map_err(|_| WasmSdkError::invalid_argument("writer is required"))?;
        if value.is_undefined() || value.is_null() {
            return Err(WasmSdkError::invalid_argument("writer is required"));
        }
        if let Ok(identity) = IdentityWasm::try_from(&value) {
            return Ok(identity.into());
        }
        let id: Identifier = IdentifierWasm::try_from(&value)
            .map_err(|_| {
                WasmSdkError::invalid_argument("writer must be an Identity or an identity id")
            })?
            .into();
        Identity::fetch(self.as_ref(), id)
            .await?
            .ok_or_else(|| WasmSdkError::not_found(format!("writer identity {id} not found")))
    }
}

#[wasm_bindgen]
impl WasmSdk {
    /// The seated charter of a contract: the `electedCharter` whose `targetContractId` is it.
    /// Only a contest's winner is ever stored, so there is at most one.
    ///
    /// @param targetContractId - The moderated contract.
    /// @returns The `electedCharter` document, or undefined when the contract has none.
    #[wasm_bindgen(js_name = "getModerationSeatedCharter")]
    pub async fn get_moderation_seated_charter(
        &self,
        #[wasm_bindgen(js_name = "targetContractId")] target_contract_id: IdentifierLikeJs,
    ) -> Result<Option<DocumentWasm>, WasmSdkError> {
        let target_contract_id: Identifier = target_contract_id.try_into()?;
        Ok(self
            .as_ref()
            .fetch_seated_charter(target_contract_id)
            .await?
            .map(|seated| document_wasm(seated.document, ELECTED_CHARTER_DOCUMENT_TYPE_NAME)))
    }

    /// A proposal by id, such as a seated charter's `submittedCharterId`.
    ///
    /// @param submittedCharterId - The `submittedCharter` document.
    /// @returns The proposal, or undefined when there is none.
    #[wasm_bindgen(js_name = "getModerationSubmittedCharter")]
    pub async fn get_moderation_submitted_charter(
        &self,
        #[wasm_bindgen(js_name = "submittedCharterId")] submitted_charter_id: IdentifierLikeJs,
    ) -> Result<Option<DocumentWasm>, WasmSdkError> {
        let submitted_charter_id: Identifier = submitted_charter_id.try_into()?;
        Ok(self
            .as_ref()
            .fetch_submitted_charter(submitted_charter_id)
            .await?
            .map(|document| document_wasm(document, SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME)))
    }

    /// The team that moderates a contract: the seated charter's leader plus its elected
    /// members and the members the leader added, less those the leader removed.
    ///
    /// @param targetContractId - The moderated contract.
    /// @returns The team, or undefined when the contract has no seated charter.
    #[wasm_bindgen(js_name = "getModerationTeam")]
    pub async fn get_moderation_team(
        &self,
        #[wasm_bindgen(js_name = "targetContractId")] target_contract_id: IdentifierLikeJs,
    ) -> Result<Option<ModerationTeamWasm>, WasmSdkError> {
        let target_contract_id: Identifier = target_contract_id.try_into()?;
        Ok(self
            .as_ref()
            .fetch_moderation_team(target_contract_id)
            .await?
            .map(ModerationTeamWasm))
    }

    /// One page of the proposals for a contract, in filing order.
    ///
    /// @returns The `submittedCharter` documents by id; pass the last id as `startAfter` for
    /// the next page.
    #[wasm_bindgen(
        js_name = "getModerationSubmittedCharters",
        unchecked_return_type = "Map<string, Document | undefined>"
    )]
    pub async fn get_moderation_submitted_charters(
        &self,
        query: ModerationSubmittedChartersQueryJs,
    ) -> Result<Map, WasmSdkError> {
        let (target_contract_id, page) = page_query(&query.into(), "targetContractId")?;
        let documents = self
            .as_ref()
            .fetch_submitted_charters(target_contract_id, page)
            .await?;
        Ok(documents_map(
            documents,
            SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME,
        ))
    }

    /// One page of the join requests for a proposal, in the order of their owners' ids.
    ///
    /// @returns The `joinRequest` documents by id; pass the last id as `startAfter` for the
    /// next page.
    #[wasm_bindgen(
        js_name = "getModerationJoinRequests",
        unchecked_return_type = "Map<string, Document | undefined>"
    )]
    pub async fn get_moderation_join_requests(
        &self,
        query: ModerationJoinRequestsQueryJs,
    ) -> Result<Map, WasmSdkError> {
        let (submitted_charter_id, page) = page_query(&query.into(), "submittedCharterId")?;
        let documents = self
            .as_ref()
            .fetch_join_requests(submitted_charter_id, page)
            .await?;
        Ok(documents_map(documents, JOIN_REQUEST_DOCUMENT_TYPE_NAME))
    }

    /// The resignation requests for a seated charter the leader has not acted on: those whose
    /// writer is still on the team (an added member is taken off by deleting its addition, an
    /// elected one by a removal). A withdrawn request is deleted, so it is not among them
    /// either.
    ///
    /// @param electedCharterId - The seated charter, an `electedCharter` document.
    #[wasm_bindgen(
        js_name = "getModerationPendingResignationRequests",
        unchecked_return_type = "Document[]"
    )]
    pub async fn get_moderation_pending_resignation_requests(
        &self,
        #[wasm_bindgen(js_name = "electedCharterId")] elected_charter_id: IdentifierLikeJs,
    ) -> Result<Array, WasmSdkError> {
        let elected_charter_id: Identifier = elected_charter_id.try_into()?;
        Ok(self
            .as_ref()
            .fetch_pending_resignation_requests(elected_charter_id)
            .await?
            .into_iter()
            .map(|document| {
                JsValue::from(document_wasm(
                    document,
                    RESIGNATION_REQUEST_DOCUMENT_TYPE_NAME,
                ))
            })
            .collect())
    }

    /// Builds a join request: the message encrypted to the proposal leader's decryption key
    /// bound to `submittedCharter`, from the writer's encryption key bound to `joinRequest`,
    /// with `recipientId`, `recipientKeyId` and `senderKeyId` set to match. Fetches the
    /// proposal and the leader.
    ///
    /// @returns The document, with its entropy set, to pass to `documentCreate`.
    #[wasm_bindgen(js_name = "buildModerationJoinRequest")]
    pub async fn build_moderation_join_request(
        &self,
        options: ModerationJoinRequestOptionsJs,
    ) -> Result<DocumentWasm, WasmSdkError> {
        let options: JsValue = options.into();
        let submitted_charter_id: Identifier =
            IdentifierWasm::try_from_options(&options, "submittedCharterId")?.into();
        let message = message_from_options(&options, "message")?;
        let writer_encryption_key =
            PrivateKeyWasm::try_from_options(&options, "writerEncryptionKey")?
                .inner()
                .inner;
        let writer = self.writer_from_options(&options).await?;
        let request = self
            .as_ref()
            .build_join_request(JoinRequestInput {
                submitted_charter_id,
                message,
                writer,
                writer_encryption_key,
            })
            .await?;
        Ok(request_wasm(request))
    }

    /// Builds a resignation request from a seated charter, encrypted to its leader as a join
    /// request is. Fetches the charter and the leader.
    ///
    /// @returns The document, with its entropy set, to pass to `documentCreate`.
    #[wasm_bindgen(js_name = "buildModerationResignationRequest")]
    pub async fn build_moderation_resignation_request(
        &self,
        options: ModerationResignationRequestOptionsJs,
    ) -> Result<DocumentWasm, WasmSdkError> {
        let options: JsValue = options.into();
        let elected_charter_id: Identifier =
            IdentifierWasm::try_from_options(&options, "electedCharterId")?.into();
        let message = message_from_options(&options, "message")?;
        let writer_encryption_key =
            PrivateKeyWasm::try_from_options(&options, "writerEncryptionKey")?
                .inner()
                .inner;
        let writer = self.writer_from_options(&options).await?;
        let request = self
            .as_ref()
            .build_resignation_request(ResignationRequestInput {
                elected_charter_id,
                message,
                writer,
                writer_encryption_key,
            })
            .await?;
        Ok(request_wasm(request))
    }
}
