use crate::error::WasmSdkError;
use crate::impl_wasm_serde_conversions;
use crate::queries::utils::deserialize_required_query;
use crate::queries::ProofMetadataResponseWasm;
use crate::sdk::WasmSdk;
use dash_sdk::dpp::data_contract::DataContract;
use dash_sdk::dpp::document::{Document, DocumentV0Getters};
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::identity::IdentityPublicKey;
use dash_sdk::dpp::platform_value::{string_encoding::Encoding, Value};
use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::platform::documents::transitions::DocumentDeleteResult;
use dash_sdk::platform::dpns_usernames::{
    convert_to_homograph_safe_chars, is_contested_username, is_valid_username,
    RegisterDpnsNameInput,
};
use dash_sdk::platform::{documents::document_query::DocumentQuery, Fetch, FetchMany, Identity};
use drive::query::{WhereClause, WhereOperator};
use drive_proof_verifier::types::Documents;
use drive_proof_verifier::ContextProvider;
use js_sys::Array;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_dpp2::data_contract::document::DocumentWasm;
use wasm_dpp2::identifier::{IdentifierLikeJs, IdentifierWasm};
use wasm_dpp2::identity::IdentityPublicKeyWasm;
use wasm_dpp2::identity::IdentityWasm;
use wasm_dpp2::utils::{
    try_from_options_optional_with, try_from_options_with, try_to_string, try_to_u64,
};
use wasm_dpp2::IdentitySignerWasm;

#[wasm_bindgen(js_name = "RegisterDpnsNameResult")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterDpnsNameResultWasm {
    #[wasm_bindgen(getter_with_clone, js_name = "preorderDocumentId")]
    pub preorder_document_id: IdentifierWasm,
    #[wasm_bindgen(getter_with_clone, js_name = "domainDocumentId")]
    pub domain_document_id: IdentifierWasm,
    #[wasm_bindgen(getter_with_clone, js_name = "fullDomainName")]
    pub full_domain_name: String,
}

#[wasm_bindgen(js_name = "DpnsUsernameInfo")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DpnsUsernameInfoWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub username: String,
    #[wasm_bindgen(getter_with_clone, js_name = "identityId")]
    pub identity_id: IdentifierWasm,
    #[wasm_bindgen(getter_with_clone, js_name = "documentId")]
    pub document_id: IdentifierWasm,
}

#[wasm_bindgen(js_class = DpnsUsernameInfo)]
impl DpnsUsernameInfoWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        username: String,
        identity_id: IdentifierWasm,
        document_id: IdentifierWasm,
    ) -> Self {
        Self {
            username,
            identity_id,
            document_id,
        }
    }
}

impl_wasm_serde_conversions!(RegisterDpnsNameResultWasm, RegisterDpnsNameResult);
impl_wasm_serde_conversions!(DpnsUsernameInfoWasm, DpnsUsernameInfo);

const DEFAULT_DPNS_USERNAMES_LIMIT: u32 = 10;

fn resolve_dpns_usernames_limit(limit: Option<u32>) -> u32 {
    match limit {
        Some(0) | None => DEFAULT_DPNS_USERNAMES_LIMIT,
        Some(value) => value,
    }
}

fn usernames_from_documents(documents_result: Documents) -> Array {
    let usernames_array = Array::new();
    for (_, doc_opt) in documents_result {
        if let Some(doc) = doc_opt {
            let properties = doc.properties();
            if let (Some(Value::Text(label)), Some(Value::Text(parent_domain))) = (
                properties.get("label"),
                properties.get("normalizedParentDomainName"),
            ) {
                let username = format!("{}.{}", label, parent_domain);
                usernames_array.push(&JsValue::from(username));
            }
        }
    }
    usernames_array
}

// ============================================================================
// DPNS Register Name
// ============================================================================

/// TypeScript interface for DPNS name registration options
#[wasm_bindgen(typescript_custom_section)]
const DPNS_REGISTER_NAME_OPTIONS_TS: &'static str = r#"
/**
 * Options for registering a DPNS username on Dash Platform.
 */
export interface DpnsRegisterNameOptions {
  /**
   * The username label to register (without the .dash suffix).
   * Must be a valid DPNS username (3-63 characters, alphanumeric and hyphens).
   */
  label: string;

  /**
   * The identity that will own the username.
   * Fetch the identity first using `getIdentity()`.
   */
  identity: Identity;

  /**
   * The identity public key to use for signing the transition.
   * Get this from the identity's public keys.
   */
  identityKey: IdentityPublicKey;

  /**
   * Signer containing the private key that corresponds to the identity key.
   * Use IdentitySigner to add the private key before calling.
   */
  signer: IdentitySigner;

  /**
   * Optional callback called after the preorder document is submitted.
   * Receives the preorder Document object.
   */
  preorderCallback?: (preorderDocument: Document) => void;

  /**
   * The most, in credits, the registration pays into the contest a contested
   * name joins. From protocol version 14 it is charged the fund to join the
   * contest, which doubles once the contest holds 250 contenders and again
   * for every 50 more, and is refused, paid, when that is more than this.
   * The identity must hold what it states. Leave it out to state the fund to
   * join read just before the domain is submitted.
   */
  contestFund?: bigint;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "DpnsRegisterNameOptions")]
    pub type DpnsRegisterNameOptionsJs;
}

// ============================================================================
// DPNS Delete Name / Tombstone Name
// ============================================================================

/// TypeScript interface for DPNS name deletion options
#[wasm_bindgen(typescript_custom_section)]
const DPNS_DELETE_NAME_OPTIONS_TS: &'static str = r#"
/**
 * Options for deleting a DPNS name its owner no longer wants (protocol
 * version 15). The name is free again: anybody may register it.
 *
 * A contested name (3 to 19 characters of a-z, 0, 1 and -, once normalized)
 * can not be deleted, only tombstoned: see `DpnsTombstoneNameOptions`.
 */
export interface DpnsDeleteNameOptions {
  /**
   * The name to delete: its label ("alice") or the full name ("alice.dash").
   */
  name: string;

  /**
   * The key of the identity holding the name that signs the delete.
   * Get this from the identity's public keys.
   */
  identityKey: IdentityPublicKey;

  /**
   * Signer containing the private key that corresponds to the identity key.
   * Use IdentitySigner to add the private key before calling.
   */
  signer: IdentitySigner;
}

/**
 * Options for tombstoning a DPNS name its owner no longer wants (protocol
 * version 15). The name is deleted and nobody can register it again while
 * the tombstone exists.
 *
 * A tombstone can not be undone by anyone: neither its owner nor anybody
 * else can delete it. The DPNS contract says only that a tombstoned name may
 * later be brought back and redistributed through a direct or indirect will
 * of the DAO.
 *
 * Any name can be tombstoned. It is the only way to give up a contested name.
 */
export interface DpnsTombstoneNameOptions {
  /**
   * The name to tombstone: its label ("alice") or the full name ("alice.dash").
   */
  name: string;

  /**
   * The identity holding the name.
   * Fetch the identity first using `getIdentity()`.
   */
  identity: Identity;

  /**
   * The key of that identity that signs the tombstone.
   * Get this from the identity's public keys.
   */
  identityKey: IdentityPublicKey;

  /**
   * Signer containing the private key that corresponds to the identity key.
   * Use IdentitySigner to add the private key before calling.
   */
  signer: IdentitySigner;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "DpnsDeleteNameOptions")]
    pub type DpnsDeleteNameOptionsJs;

    #[wasm_bindgen(typescript_type = "DpnsTombstoneNameOptions")]
    pub type DpnsTombstoneNameOptionsJs;
}

// TS definition for DpnsUsernamesQuery used by getDpnsUsernames*
#[wasm_bindgen(typescript_custom_section)]
const DPNS_USERNAMES_QUERY_TS: &'static str = r#"
/**
 * Query parameters for retrieving DPNS usernames.
 */
export interface DpnsUsernamesQuery {
  /**
   * Identity to fetch usernames for.
   */
  identityId: IdentifierLike;

  /**
   * Maximum number of usernames to return. Use 0 for default.
   * @default 10
   */
  limit?: number;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "DpnsUsernamesQuery")]
    pub type DpnsUsernamesQueryJs;
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DpnsUsernamesQueryInput {
    identity_id: IdentifierWasm,
    #[serde(default)]
    limit: Option<u32>,
}

struct DpnsUsernamesQueryParsed {
    identity_id: Identifier,
    limit: Option<u32>,
}

fn parse_dpns_usernames_query(
    query: DpnsUsernamesQueryJs,
) -> Result<DpnsUsernamesQueryParsed, WasmSdkError> {
    let input: DpnsUsernamesQueryInput =
        deserialize_required_query(query, "Query object is required", "DPNS usernames query")?;

    Ok(DpnsUsernamesQueryParsed {
        identity_id: input.identity_id.into(),
        limit: input.limit,
    })
}

/// Extracts an optional JS function from options.
fn extract_callback_from_options(
    options: &JsValue,
    field_name: &str,
) -> Result<Option<js_sys::Function>, WasmSdkError> {
    Ok(try_from_options_optional_with(options, field_name, |v| {
        v.clone().dyn_into::<js_sys::Function>().map_err(|_| {
            wasm_dpp2::error::WasmDppError::invalid_argument(format!(
                "{} must be a function",
                field_name
            ))
        })
    })?)
}

/// DPNS contract ID constant
const DPNS_CONTRACT_ID: &str = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec";
/// DPNS domain document type
const DPNS_DOCUMENT_TYPE: &str = "domain";
/// DPNS document type of a name given up for good, from DPNS v3
const DPNS_TOMBSTONE_DOCUMENT_TYPE: &str = "tombstone";

/// What a lookup by name reports of `domain`, the `domain` document of `username`: the name
/// as it was asked for, the document, and its owner.
fn dpns_username_info(username: &str, domain: &Document) -> DpnsUsernameInfoWasm {
    DpnsUsernameInfoWasm {
        username: username.to_string(),
        identity_id: IdentifierWasm::from(domain.owner_id()),
        document_id: IdentifierWasm::from(domain.id()),
    }
}

impl WasmSdk {
    /// Get DPNS contract, checking context provider cache first, then fetching if needed.
    async fn get_dpns_contract(&self) -> Result<Arc<DataContract>, WasmSdkError> {
        let contract_id =
            Identifier::from_string(DPNS_CONTRACT_ID, Encoding::Base58).map_err(|e| {
                WasmSdkError::invalid_argument(format!("Invalid DPNS contract ID: {}", e))
            })?;

        // First check if the contract is available in the context provider
        if let Some(context_provider) = self.as_ref().context_provider() {
            if let Ok(Some(contract)) =
                context_provider.get_data_contract(&contract_id, self.as_ref().version())
            {
                return Ok(contract);
            }
        }

        // If not in context, fetch from platform
        let contract = DataContract::fetch(self.as_ref(), contract_id)
            .await?
            .ok_or_else(|| WasmSdkError::generic("DPNS contract not found"))?;
        Ok(Arc::new(contract))
    }

    async fn fetch_dpns_usernames(
        &self,
        identity_id: Identifier,
        limit: Option<u32>,
    ) -> Result<Array, WasmSdkError> {
        // Use rs-sdk method with resolved limit for consistency with proof path
        let resolved_limit = resolve_dpns_usernames_limit(limit);
        let usernames = self
            .as_ref()
            .get_dpns_usernames_by_identity(identity_id, Some(resolved_limit))
            .await?;

        let array = Array::new();
        for username in usernames {
            array.push(&JsValue::from_str(&username.full_name));
        }
        Ok(array)
    }

    async fn fetch_dpns_usernames_with_proof(
        &self,
        identity_id: Identifier,
        limit: Option<u32>,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        // For proof queries, we still need to use document query directly
        // as rs-sdk doesn't have a with_proof variant
        let dpns_contract = self.get_dpns_contract().await?;

        let query = DocumentQuery {
            select: dash_sdk::drive::query::SelectProjection::documents(),
            data_contract: dpns_contract,
            document_type_name: DPNS_DOCUMENT_TYPE.to_string(),
            where_clauses: vec![WhereClause {
                field: "records.identity".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Identifier(identity_id.to_buffer()),
            }],
            time_range_clauses: vec![],
            integer_range_clauses: vec![],
            sub_queries: vec![],
            group_by: vec![],
            having: vec![],
            order_by_clauses: vec![],
            limit: resolve_dpns_usernames_limit(limit),
            offset: None,
            start: None,
        };

        let (documents_result, metadata, proof) =
            Document::fetch_many_with_metadata_and_proof(self.as_ref(), query, None).await?;
        let usernames_array = usernames_from_documents(documents_result);

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            usernames_array,
            metadata,
            proof,
        ))
    }
}

#[wasm_bindgen]
impl WasmSdk {
    #[wasm_bindgen(js_name = "dpnsConvertToHomographSafe")]
    pub fn dpns_convert_to_homograph_safe(input: &str) -> String {
        convert_to_homograph_safe_chars(input)
    }

    #[wasm_bindgen(js_name = "dpnsIsValidUsername")]
    pub fn dpns_is_valid_username(label: &str) -> bool {
        is_valid_username(label)
    }

    #[wasm_bindgen(js_name = "dpnsIsContestedUsername")]
    pub fn dpns_is_contested_username(label: &str) -> bool {
        is_contested_username(label)
    }

    /// Register a DPNS username on Dash Platform.
    ///
    /// This method handles the complete DPNS registration flow:
    /// 1. Creates and submits a preorder document
    /// 2. Waits for preorder confirmation
    /// 3. Creates and submits the domain document
    /// 4. Returns the result with both document IDs
    ///
    /// @param options - Registration options including label, identity, key, and signer
    /// @returns Promise that resolves to the registration result
    #[wasm_bindgen(js_name = "dpnsRegisterName")]
    pub async fn dpns_register_name(
        &self,
        options: DpnsRegisterNameOptionsJs,
    ) -> Result<RegisterDpnsNameResultWasm, WasmSdkError> {
        // Extract label from options
        let label: String =
            try_from_options_with(&options, "label", |v| try_to_string(v, "label"))?;

        // Extract identity from options
        let identity: Identity = IdentityWasm::try_from_options(&options, "identity")?.into();

        // Extract identity key from options
        let identity_key_wasm = IdentityPublicKeyWasm::try_from_options(&options, "identityKey")?;
        let identity_public_key: IdentityPublicKey = identity_key_wasm.into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options, "signer")?;

        // Extract optional preorder callback
        let preorder_callback = extract_callback_from_options(&options, "preorderCallback")?;

        // The most the registration pays into the contest a contested name joins
        let contest_fund = try_from_options_optional_with(&options, "contestFund", |v| {
            try_to_u64(v, "contestFund")
        })?;

        // Set up the callback if provided
        thread_local! {
            static PREORDER_CALLBACK: std::cell::RefCell<Option<js_sys::Function>>
                = const { std::cell::RefCell::new(None) };
        }

        if let Some(ref js_callback) = preorder_callback {
            PREORDER_CALLBACK.with(|cb| {
                *cb.borrow_mut() = Some(js_callback.clone());
            });
        }

        // Get DPNS contract ID for the callback
        let dpns_contract_id = Identifier::from_string(DPNS_CONTRACT_ID, Encoding::Base58).unwrap();

        let callback_box = if preorder_callback.is_some() {
            Some(Box::new(move |doc: &Document| {
                PREORDER_CALLBACK.with(|cb| {
                    if let Some(js_callback) = cb.borrow().as_ref() {
                        // Convert to DocumentWasm with DPNS metadata
                        let doc_wasm = DocumentWasm::new(
                            doc.clone(),
                            dpns_contract_id,
                            "preorder".to_string(),
                            None,
                        );
                        let _ = js_callback.call1(&JsValue::NULL, &JsValue::from(doc_wasm));
                    }
                });
            }) as Box<dyn FnOnce(&Document) + Send>)
        } else {
            None
        };

        let input = RegisterDpnsNameInput {
            label,
            identity,
            identity_public_key,
            signer,
            preorder_callback: callback_box,
            contest_fund,
        };

        let result = self.as_ref().register_dpns_name(input).await?;

        // Clean up callback
        PREORDER_CALLBACK.with(|cb| {
            *cb.borrow_mut() = None;
        });

        Ok(RegisterDpnsNameResultWasm {
            preorder_document_id: IdentifierWasm::from(result.preorder_document.id()),
            domain_document_id: IdentifierWasm::from(result.domain_document.id()),
            full_domain_name: result.full_domain_name,
        })
    }

    #[wasm_bindgen(js_name = "dpnsIsNameAvailable")]
    pub async fn dpns_is_name_available(&self, label: &str) -> Result<bool, WasmSdkError> {
        self.as_ref()
            .is_dpns_name_available(label)
            .await
            .map_err(WasmSdkError::from)
    }

    /// Whether a DPNS name is tombstoned: its owner gave it up for good, and nobody
    /// can register it while the tombstone exists. `dpnsIsNameAvailable` is false
    /// for such a name. False for a registered name and for a free one. Before
    /// protocol version 15 no name is tombstoned, so a read that succeeds answers
    /// false.
    ///
    /// A read that fails rejects: it is never answered as false.
    ///
    /// @param name - The label ("alice") or the full name ("alice.dash")
    /// @returns Promise that resolves to whether the name is tombstoned, and
    ///          rejects when the network can not be read
    #[wasm_bindgen(js_name = "dpnsIsNameTombstoned")]
    pub async fn dpns_is_name_tombstoned(&self, name: &str) -> Result<bool, WasmSdkError> {
        self.as_ref()
            .is_dpns_name_tombstoned(name)
            .await
            .map_err(WasmSdkError::from)
    }

    /// Delete a DPNS name its owner no longer wants (protocol version 15). The name
    /// is free again: anybody may register it. The owner is refunded what is left of
    /// the name's storage fee.
    ///
    /// A contested name (3 to 19 characters of a-z, 0, 1 and -, once normalized) can
    /// not be deleted, only tombstoned with `dpnsTombstoneName`: the call rejects
    /// before anything is broadcast.
    ///
    /// @param options - Delete options including name, identity key, and signer
    /// @returns Promise that resolves to the id of the deleted domain document
    #[wasm_bindgen(js_name = "dpnsDeleteName")]
    pub async fn dpns_delete_name(
        &self,
        options: DpnsDeleteNameOptionsJs,
    ) -> Result<IdentifierWasm, WasmSdkError> {
        let name: String = try_from_options_with(&options, "name", |v| try_to_string(v, "name"))?;
        let identity_public_key: IdentityPublicKey =
            IdentityPublicKeyWasm::try_from_options(&options, "identityKey")?.into();
        let signer = IdentitySignerWasm::try_from_options(&options, "signer")?;

        let DocumentDeleteResult::Deleted(document_id) = self
            .as_ref()
            .delete_dpns_name(&name, &identity_public_key, &signer)
            .await?;

        Ok(IdentifierWasm::from(document_id))
    }

    /// Tombstone a DPNS name its owner no longer wants (protocol version 15): the
    /// name is deleted and nobody can register it again while the tombstone exists.
    /// The owner is refunded what is left of the name's storage fee and pays for the
    /// tombstone's.
    ///
    /// A tombstone can not be undone by anyone: neither its owner nor anybody else
    /// can delete it. The DPNS contract says only that a tombstoned name may later
    /// be brought back and redistributed through a direct or indirect will of the
    /// DAO.
    ///
    /// Any name can be tombstoned. It is the only way to give up a contested name.
    ///
    /// @param options - Tombstone options including name, identity, identity key, and signer
    /// @returns Promise that resolves to the tombstone document the network stored
    #[wasm_bindgen(js_name = "dpnsTombstoneName")]
    pub async fn dpns_tombstone_name(
        &self,
        options: DpnsTombstoneNameOptionsJs,
    ) -> Result<DocumentWasm, WasmSdkError> {
        let name: String = try_from_options_with(&options, "name", |v| try_to_string(v, "name"))?;
        let identity: Identity = IdentityWasm::try_from_options(&options, "identity")?.into();
        let identity_public_key: IdentityPublicKey =
            IdentityPublicKeyWasm::try_from_options(&options, "identityKey")?.into();
        let signer = IdentitySignerWasm::try_from_options(&options, "signer")?;

        let dpns_contract_id = Identifier::from_string(DPNS_CONTRACT_ID, Encoding::Base58)
            .map_err(|e| {
                WasmSdkError::invalid_argument(format!("Invalid DPNS contract ID: {}", e))
            })?;

        let tombstone = self
            .as_ref()
            .tombstone_dpns_name(&name, identity.id(), identity_public_key, &signer)
            .await?;

        Ok(DocumentWasm::new(
            tombstone,
            dpns_contract_id,
            DPNS_TOMBSTONE_DOCUMENT_TYPE.to_string(),
            None,
        ))
    }

    #[wasm_bindgen(js_name = "dpnsResolveName")]
    pub async fn dpns_resolve_name(&self, name: &str) -> Result<Option<String>, WasmSdkError> {
        let result = self.as_ref().resolve_dpns_name(name).await?;

        Ok(result.map(|identity_id| identity_id.to_string(Encoding::Base58)))
    }

    #[wasm_bindgen(js_name = "getDpnsUsernameByName")]
    pub async fn get_dpns_username_by_name(
        &self,
        username: &str,
    ) -> Result<Option<DpnsUsernameInfoWasm>, WasmSdkError> {
        let domain = self
            .as_ref()
            .get_dpns_domain_document_by_name(username)
            .await?;

        Ok(domain.map(|domain| dpns_username_info(username, &domain)))
    }

    #[wasm_bindgen(
        js_name = "getDpnsUsernameByNameWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<DpnsUsernameInfo | undefined>"
    )]
    pub async fn get_dpns_username_by_name_with_proof_info(
        &self,
        username: &str,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let (domain, metadata, proof) = self
            .as_ref()
            .get_dpns_domain_document_by_name_with_metadata_and_proof(username)
            .await?;

        let data = if let Some(domain) = domain {
            // Use json_compatible() to ensure objects become plain JS objects (not Maps)
            let serializer = serde_wasm_bindgen::Serializer::json_compatible();
            dpns_username_info(username, &domain)
                .serialize(&serializer)
                .map_err(|e| {
                    WasmSdkError::serialization(format!("Failed to serialize username info: {}", e))
                })?
        } else {
            JsValue::NULL
        };

        let response = ProofMetadataResponseWasm::from_sdk_parts(data, metadata, proof);

        Ok(response)
    }

    #[wasm_bindgen(js_name = "getDpnsUsernames", unchecked_return_type = "Array<string>")]
    pub async fn get_dpns_usernames(
        &self,
        query: DpnsUsernamesQueryJs,
    ) -> Result<Array, WasmSdkError> {
        let params = parse_dpns_usernames_query(query)?;
        self.fetch_dpns_usernames(params.identity_id, params.limit)
            .await
    }

    #[wasm_bindgen(js_name = "getDpnsUsername")]
    pub async fn get_dpns_username(
        &self,
        #[wasm_bindgen(js_name = "identityId")] identity_id: IdentifierLikeJs,
    ) -> Result<Option<String>, WasmSdkError> {
        let identity_id_parsed = IdentifierWasm::try_from(identity_id)?.into();

        let array = self
            .fetch_dpns_usernames(identity_id_parsed, Some(1))
            .await?;

        if array.length() == 0 {
            return Ok(None);
        }

        array
            .get(0)
            .as_string()
            .map(Some)
            .ok_or_else(|| WasmSdkError::generic("DPNS username is not a string"))
    }

    #[wasm_bindgen(
        js_name = "getDpnsUsernamesWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<Array<string>>"
    )]
    pub async fn get_dpns_usernames_with_proof_info(
        &self,
        query: DpnsUsernamesQueryJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let params = parse_dpns_usernames_query(query)?;
        self.fetch_dpns_usernames_with_proof(params.identity_id, params.limit)
            .await
    }

    #[wasm_bindgen(
        js_name = "getDpnsUsernameWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<string | undefined>"
    )]
    pub async fn get_dpns_username_with_proof_info(
        &self,
        #[wasm_bindgen(js_name = "identityId")] identity_id: IdentifierLikeJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let identity_id_parsed = IdentifierWasm::try_from(identity_id)?.into();

        let mut response = self
            .fetch_dpns_usernames_with_proof(identity_id_parsed, Some(1))
            .await?;

        let usernames = js_sys::Array::from(&response.data());
        let username = if usernames.length() > 0 {
            usernames.get(0)
        } else {
            JsValue::NULL
        };

        response.set_data(username);

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context_provider::WasmTrustedContext;
    #[cfg(not(target_arch = "wasm32"))]
    use crate::error::WasmSdkErrorKind;
    use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dash_sdk::dpp::document::DocumentV0;
    use dash_sdk::dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dash_sdk::drive::query::SelectProjection;
    use dash_sdk::Sdk;

    const DOMAIN_OWNER: [u8; 32] = [7; 32];
    const DOMAIN_DOCUMENT_ID: [u8; 32] = [9; 32];

    /// An SDK whose network holds one name, stored as DPNS stores `alice.dash`: under the
    /// homograph-safe label `a11ce` and the parent domain `dash`. It answers the query for
    /// that stored form and no other.
    async fn sdk_holding_alice_dot_dash() -> WasmSdk {
        let mut inner_sdk = Sdk::new_mock();
        let dpns = load_system_data_contract(SystemDataContract::DPNS, inner_sdk.version())
            .expect("the DPNS contract loads");
        let domain = Document::V0(DocumentV0 {
            id: Identifier::new(DOMAIN_DOCUMENT_ID),
            owner_id: Identifier::new(DOMAIN_OWNER),
            ..Default::default()
        });
        // The Rust SDK asks the context provider for the DPNS contract first, and fetches it
        // when the provider does not hold it.
        let context = WasmTrustedContext::for_testing(vec![]);
        inner_sdk
            .set_context_provider(context.clone())
            .expect("the context provider is set");
        inner_sdk
            .mock()
            .expect_fetch(dpns.id(), Some(dpns.clone()))
            .await
            .expect("the DPNS contract fetch is expected");
        let stored_name_query = DocumentQuery {
            select: SelectProjection::documents(),
            data_contract: Arc::new(dpns),
            document_type_name: DPNS_DOCUMENT_TYPE.to_string(),
            where_clauses: vec![
                WhereClause {
                    field: "normalizedParentDomainName".to_string(),
                    operator: WhereOperator::Equal,
                    value: Value::Text("dash".to_string()),
                },
                WhereClause {
                    field: "normalizedLabel".to_string(),
                    operator: WhereOperator::Equal,
                    value: Value::Text("a11ce".to_string()),
                },
            ],
            time_range_clauses: vec![],
            integer_range_clauses: vec![],
            sub_queries: vec![],
            group_by: vec![],
            having: vec![],
            order_by_clauses: vec![],
            limit: 1,
            offset: None,
            start: None,
        };
        inner_sdk
            .mock()
            .expect_fetch_many::<Identifier, Document, DocumentQuery, Documents>(
                stored_name_query,
                Some([(domain.id(), Some(domain))].into_iter().collect()),
            )
            .await
            .expect("the domain query is expected");
        WasmSdk::new_for_testing(inner_sdk, Some(context))
    }

    /// DPNS keeps a name under its homograph-safe label: `alice.dash` is stored as `a11ce`
    /// under `dash`. A lookup of the name as a user types it has to search for that form, or
    /// it finds nothing for any label with an `i`, `l` or `o`.
    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn should_find_a_username_whose_label_dpns_stores_in_homograph_safe_form() {
        let sdk = sdk_holding_alice_dot_dash().await;

        let info = sdk
            .get_dpns_username_by_name("Alice.DASH")
            .await
            .expect("the lookup succeeds")
            .expect("the name is found");

        assert_eq!(info.username, "Alice.DASH");
        assert_eq!(
            Identifier::from(info.identity_id),
            Identifier::new(DOMAIN_OWNER)
        );
        assert_eq!(
            Identifier::from(info.document_id),
            Identifier::new(DOMAIN_DOCUMENT_ID)
        );
    }

    /// The lookup that also returns the proof searches for the same stored form, and
    /// reports the name as it was typed, the document's owner and the document. This
    /// response carries the two identifiers as their bytes.
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn should_find_a_username_in_homograph_safe_form_with_proof_info() {
        let sdk = sdk_holding_alice_dot_dash().await;

        let response = sdk
            .get_dpns_username_by_name_with_proof_info("Alice.DASH")
            .await
            .expect("the lookup succeeds");

        assert_eq!(
            wasm_dpp2::serialization::js_value_to_json(&response.data()).expect("the info as JSON"),
            serde_json::json!({
                "username": "Alice.DASH",
                "identityId": DOMAIN_OWNER,
                "documentId": DOMAIN_DOCUMENT_ID,
            })
        );
    }

    /// A name that is not a label and a parent domain is refused as an invalid argument,
    /// before anything is read: the mock SDK holds no expectation.
    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn should_refuse_a_username_that_is_not_a_label_and_a_domain() {
        let sdk = WasmSdk::new_for_testing(Sdk::new_mock(), None);

        let error = sdk
            .get_dpns_username_by_name("alice")
            .await
            .err()
            .expect("a bare label is refused");

        assert_eq!(error.kind(), WasmSdkErrorKind::InvalidArgument);
        assert_eq!(
            error.message(),
            "Invalid username format. Expected format: label.domain"
        );
    }
}
