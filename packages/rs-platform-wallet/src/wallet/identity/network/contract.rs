//! Data contract create / update operations on `IdentityWallet`.
//!
//! These live on `IdentityWallet` (rather than in `rs-sdk-ffi`)
//! because contract creation is a wallet-level operation: it spans
//! an identity (the owner), needs the wallet's signer, and changes
//! local state the persister tracks. Per `swift-sdk/CLAUDE.md`,
//! "anything that spans identities / platform balances / core sync
//! / tokens / DashPay / identity key derivation / identity
//! registration belongs in the `platform-wallet` crate."
//!
//! Mirrors the post-#3541 identity-flow shape:
//!   - Library function takes a `Signer<IdentityPublicKey>` reference
//!     so the FFI's external `KeychainSigner` trampoline can route
//!     signing back to Swift / Keychain without crossing seed bytes.
//!   - Caller passes the contract content as JSON strings (the V1
//!     serialization format struct is `pub(in crate::data_contract)`
//!     in `rs-dpp`, so we round-trip through serde_json to construct
//!     it via the public `DataContractInSerializationFormat::Deserialize`
//!     entry point).
//!   - Broadcast goes through
//!     `dash_sdk::platform::transition::put_contract::PutContract::put_to_platform_and_wait_for_response`
//!     on the platform-wallet runtime (8 MB worker stack) instead of
//!     the rs-sdk-ffi runtime (mobile-tuned default stack). That's
//!     what fixes the `EXC_BAD_ACCESS` in
//!     `grovedb_query::proofs::encoding::Op::decode` we saw under
//!     the rs-sdk-ffi path — classic stack-guard fingerprint, not
//!     memory unsafety.

use async_trait::async_trait;

use dpp::address_funds::AddressWitness;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
use dpp::data_contract::INITIAL_DATA_CONTRACT_VERSION;
use dpp::identity::signer::Signer;
use dpp::identity::{IdentityPublicKey, KeyType, PartialIdentity, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use dpp::prelude::{DataContract, Identifier};
use dpp::state_transition::data_contract_update_transition::methods::DataContractUpdateTransitionMethodsV0;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::ProtocolError;

use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;
use dash_sdk::platform::transition::put_contract::PutContract;
use dash_sdk::platform::transition::waitable::Waitable;
use dash_sdk::platform::Fetch;

use crate::error::PlatformWalletError;

use super::*;

/// Borrowed-signer adapter — same shape as the local `SignerRef`
/// in `update.rs` / `dpns.rs` / `transfer.rs`. Lets the
/// `Signer<IdentityPublicKey>` trait bound on the SDK's
/// `PutContract` extension be satisfied with a `&S` instead of a
/// `Box<S>` / `Arc<S>` per call.
struct SignerRef<'a, S: ?Sized>(&'a S);

impl<'a, S: ?Sized> std::fmt::Debug for SignerRef<'a, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SignerRef")
    }
}

#[async_trait]
impl<'a, K, S> Signer<K> for SignerRef<'a, S>
where
    K: Send + Sync,
    S: Signer<K> + ?Sized + Send + Sync,
{
    async fn sign(&self, key: &K, data: &[u8]) -> Result<BinaryData, ProtocolError> {
        self.0.sign(key, data).await
    }

    async fn sign_create_witness(
        &self,
        key: &K,
        data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        self.0.sign_create_witness(key, data).await
    }

    fn can_sign_with(&self, key: &K) -> bool {
        self.0.can_sign_with(key)
    }
}

impl IdentityWallet {
    /// Create a new data contract owned by `owner_identity_id` and
    /// broadcast it to Platform.
    ///
    /// The contract content arrives as JSON strings — every field
    /// beyond `documents_schema_json` is optional and absent when
    /// `None`. The function:
    ///   1. Looks up `owner_identity_id` in the in-memory wallet
    ///      manager, picks a CRITICAL + AUTHENTICATION + ECDSA
    ///      key (DPP requires that exact triple for a contract-
    ///      create signature).
    ///   2. Generates the deterministic contract id from
    ///      `(owner_id, identity_nonce)`.
    ///   3. Assembles a V1 serialization-format payload carrying
    ///      every supplied field, deserializes it via the public
    ///      `DataContractInSerializationFormat` enum entry point,
    ///      and validates through `try_from_platform_versioned`.
    ///   4. Broadcasts via
    ///      `DataContract::put_to_platform_and_wait_for_response`
    ///      on the platform-wallet runtime.
    ///   5. Returns the confirmed `DataContract` from Platform.
    ///
    /// # JSON shapes
    ///
    /// - `documents_schema_json`: object keyed by document type
    ///   name, each value a JSON Schema. Pass `"{}"` for token-
    ///   only contracts.
    /// - `tokens_schema_json`: object keyed by stringified slot
    ///   index (`"0"`, `"1"`, …), each value a `TokenConfiguration`
    ///   JSON. The Swift form's three-level `$formatVersion: "0"`
    ///   tags (token / convention / localization) are required —
    ///   the V1 struct's tagged enums fail to deserialize without
    ///   them.
    /// - `groups_schema_json`: object keyed by stringified group
    ///   position, each value a `Group` JSON.
    /// - `keywords_json`: JSON array of strings.
    /// - `description`: plain (non-JSON) string.
    /// - `config_json`: `DataContractConfig` JSON. The function
    ///   injects `$formatVersion: "0"` if the caller-supplied
    ///   config dict doesn't carry one (Swift form-builder shape
    ///   doesn't know the tag).
    #[allow(clippy::too_many_arguments)]
    pub async fn create_data_contract_with_signer<S>(
        &self,
        owner_identity_id: &Identifier,
        documents_schema_json: &str,
        tokens_schema_json: Option<&str>,
        groups_schema_json: Option<&str>,
        keywords_json: Option<&str>,
        description: Option<&str>,
        config_json: Option<&str>,
        signer: &S,
    ) -> Result<DataContract, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        // 1. Owner identity + signing key from the wallet manager.
        use dpp::identity::accessors::IdentityGettersV0;
        let signing_key = {
            let wm = self.wallet_manager.read().await;
            let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
                PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            let manager = &info.identity_manager;
            let identity = manager
                .identity(owner_identity_id)
                .map(|m| m.identity.clone())
                .ok_or(PlatformWalletError::IdentityNotFound(*owner_identity_id))?;
            // Contract create requires CRITICAL + AUTHENTICATION +
            // ECDSA_SECP256K1 specifically — DPP rejects HIGH /
            // MEDIUM / non-ECDSA keys on this state-transition
            // shape.
            identity
                .get_first_public_key_matching(
                    Purpose::AUTHENTICATION,
                    [SecurityLevel::CRITICAL].into(),
                    [KeyType::ECDSA_SECP256K1].into(),
                    false,
                )
                .ok_or_else(|| {
                    PlatformWalletError::InvalidIdentityData(
                        "No CRITICAL authentication key found on owner identity \
                         (required to sign a contract-create state transition)"
                            .to_string(),
                    )
                })?
                .clone()
        };

        // 2. Build the V1 serialization format with a placeholder id.
        //    The SDK's `DataContractCreateTransition::new_from_data_contract`
        //    fetches the identity nonce itself and overwrites the
        //    contract id with the canonical
        //    `generate_data_contract_id_v0(owner, fetched_nonce)` —
        //    so any id we set here is dropped on the floor. Earlier
        //    revisions also pre-fetched the nonce via
        //    `sdk.get_identity_nonce(.., bump = true, ..)` for an
        //    in-place id computation, which double-bumped the network
        //    nonce (the SDK's own put-path bumps it a second time)
        //    and wasted one slot per call. Dropping the local fetch
        //    keeps the broadcast on a single nonce increment.
        let placeholder_id = Identifier::default();

        let mut format_value = serde_json::Map::new();
        format_value.insert(
            "$formatVersion".to_string(),
            serde_json::Value::String("1".to_string()),
        );
        format_value.insert(
            "id".to_string(),
            serde_json::Value::String(bs58::encode(placeholder_id.to_buffer()).into_string()),
        );
        format_value.insert(
            "ownerId".to_string(),
            serde_json::Value::String(bs58::encode(owner_identity_id.to_buffer()).into_string()),
        );
        format_value.insert(
            "version".to_string(),
            serde_json::Value::Number(serde_json::Number::from(INITIAL_DATA_CONTRACT_VERSION)),
        );

        format_value.insert(
            "documentSchemas".to_string(),
            parse_required_json("documents_schema_json", documents_schema_json)?,
        );
        if let Some(v) = parse_optional_json("tokens_schema_json", tokens_schema_json)? {
            format_value.insert("tokens".to_string(), v);
        }
        if let Some(v) = parse_optional_json("groups_schema_json", groups_schema_json)? {
            format_value.insert("groups".to_string(), v);
        }
        if let Some(v) = parse_optional_json("keywords_json", keywords_json)? {
            format_value.insert("keywords".to_string(), v);
        }
        if let Some(s) = description.filter(|s| !s.is_empty()) {
            format_value.insert(
                "description".to_string(),
                serde_json::Value::String(s.to_string()),
            );
        }
        if let Some(mut v) = parse_optional_json("config_json", config_json)? {
            // `DataContractConfig` is itself a `#[serde(tag =
            // "$formatVersion")]` enum; Swift form-builder shape
            // sends a bare flags dict. Inject the V0 tag when
            // missing so the tagged-enum dispatch picks a variant.
            if let Some(obj) = v.as_object_mut() {
                obj.entry("$formatVersion")
                    .or_insert_with(|| serde_json::Value::String("0".to_string()));
            }
            format_value.insert("config".to_string(), v);
        }

        // Round-trip through a string instead of `from_value` —
        // serde's `#[serde(tag = "...")]` enum dispatch can drop /
        // reorder fields through the `from_value` path and produce
        // a misleading "missing field `$formatVersion`" error even
        // when the field is present in the assembled `Map`. Going
        // via `to_string` + `from_str` avoids that.
        let serialized = serde_json::to_string_pretty(&serde_json::Value::Object(format_value))
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to serialize contract format: {e}"
                ))
            })?;
        let format: DataContractInSerializationFormat =
            serde_json::from_str(&serialized).map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to assemble contract format: {e}\n\nAssembled JSON:\n{serialized}"
                ))
            })?;

        let platform_version = self.sdk.version();
        let mut errors = vec![];
        let data_contract =
            DataContract::try_from_platform_versioned(format, true, &mut errors, platform_version)
                .map_err(|e| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "Failed to build contract: {e}\n\nAssembled JSON:\n{serialized}"
                    ))
                })?;

        // 3. Broadcast via `PutContract`. This runs on the
        //    platform-wallet 8 MB-stack worker, so the proof-
        //    verification recursion in GroveDB doesn't blow the
        //    stack like it does on the rs-sdk-ffi runtime. The
        //    SDK fetches+bumps the identity nonce internally and
        //    overwrites the contract id with the canonical
        //    `(owner, nonce)` derivation; the placeholder id we
        //    set above is intentionally discarded.
        let confirmed = data_contract
            .put_to_platform_and_wait_for_response(&self.sdk, signing_key, &SignerRef(signer), None)
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to put data contract to platform: {e}"
                ))
            })?;

        Ok(confirmed)
    }

    /// Update an existing data contract owned by `owner_identity_id`
    /// and broadcast the change to Platform.
    ///
    /// Mirrors `create_data_contract_with_signer` but targets an
    /// already-registered contract. The new schemas / config arrive
    /// as JSON strings; the function:
    ///   1. Looks up `owner_identity_id` in the in-memory wallet
    ///      manager and picks the CRITICAL + AUTHENTICATION + ECDSA
    ///      key (same triple a contract-create signature requires).
    ///   2. Fetches the *current* contract from Platform to read its
    ///      live version — the new contract definition must carry
    ///      `current_version + 1` (DPP rejects an update that doesn't
    ///      strictly increment the version), and the caller doesn't
    ///      have to track local version state.
    ///   3. Assembles a serialization-format payload reusing the
    ///      existing contract id + owner, the bumped version, and the
    ///      supplied schemas / config, deserializes it via the public
    ///      `DataContractInSerializationFormat` enum entry point, and
    ///      validates through `try_from_platform_versioned`.
    ///   4. Fetches + bumps the identity-contract nonce, builds a
    ///      `DataContractUpdateTransition`, signs it via the external
    ///      signer, broadcasts, and waits for the confirmed contract.
    ///
    /// # JSON shapes
    ///
    /// Identical to `create_data_contract_with_signer`. The config
    /// `$formatVersion` is selected from the platform version's
    /// `contract_versions.config.default_current_version` (V1 under
    /// v12) rather than hardcoded to "0" — v12 rejects a V0 config
    /// on both registration and update.
    #[allow(clippy::too_many_arguments)]
    pub async fn update_data_contract_with_signer<S>(
        &self,
        owner_identity_id: &Identifier,
        contract_id: &Identifier,
        documents_schema_json: &str,
        tokens_schema_json: Option<&str>,
        groups_schema_json: Option<&str>,
        keywords_json: Option<&str>,
        description: Option<&str>,
        config_json: Option<&str>,
        signer: &S,
    ) -> Result<DataContract, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        // 1. Owner identity + signing key from the wallet manager.
        use dpp::identity::accessors::IdentityGettersV0;
        let signing_key = {
            let wm = self.wallet_manager.read().await;
            let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
                PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            let manager = &info.identity_manager;
            let identity = manager
                .identity(owner_identity_id)
                .map(|m| m.identity.clone())
                .ok_or(PlatformWalletError::IdentityNotFound(*owner_identity_id))?;
            // Contract update requires the same CRITICAL +
            // AUTHENTICATION + ECDSA_SECP256K1 key as create — DPP
            // rejects HIGH / MEDIUM / non-ECDSA keys on this
            // state-transition shape.
            identity
                .get_first_public_key_matching(
                    Purpose::AUTHENTICATION,
                    [SecurityLevel::CRITICAL].into(),
                    [KeyType::ECDSA_SECP256K1].into(),
                    false,
                )
                .ok_or_else(|| {
                    PlatformWalletError::InvalidIdentityData(
                        "No CRITICAL authentication key found on owner identity \
                         (required to sign a contract-update state transition)"
                            .to_string(),
                    )
                })?
                .clone()
        };

        // 2. Fetch the live contract to read its current version. The
        //    update must strictly increment the version, so we bump
        //    the on-chain value by one rather than trusting a
        //    caller-supplied number.
        let existing = DataContract::fetch(&self.sdk, *contract_id)
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to fetch contract {contract_id} for update: {e}"
                ))
            })?
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Data contract {contract_id} not found on Platform; cannot update"
                ))
            })?;
        let new_version = existing.version().checked_add(1).ok_or_else(|| {
            PlatformWalletError::InvalidIdentityData(
                "Contract version overflow; cannot update".to_string(),
            )
        })?;

        // 3. Assemble the updated serialization-format payload. Unlike
        //    create, the id is the *existing* contract id (the update
        //    transition keys off it) and the version is the bumped
        //    value above.
        let platform_version = self.sdk.version();
        // Config `$formatVersion` follows the platform's current
        // default (V1 under v12). Hardcoding "0" would build a V0
        // config that v12 rejects on update.
        let config_format_version = platform_version
            .dpp
            .contract_versions
            .config
            .default_current_version;

        let mut format_value = serde_json::Map::new();
        format_value.insert(
            "$formatVersion".to_string(),
            serde_json::Value::String("1".to_string()),
        );
        format_value.insert(
            "id".to_string(),
            serde_json::Value::String(bs58::encode(contract_id.to_buffer()).into_string()),
        );
        format_value.insert(
            "ownerId".to_string(),
            serde_json::Value::String(bs58::encode(owner_identity_id.to_buffer()).into_string()),
        );
        format_value.insert(
            "version".to_string(),
            serde_json::Value::Number(serde_json::Number::from(new_version)),
        );

        format_value.insert(
            "documentSchemas".to_string(),
            parse_required_json("documents_schema_json", documents_schema_json)?,
        );
        if let Some(v) = parse_optional_json("tokens_schema_json", tokens_schema_json)? {
            format_value.insert("tokens".to_string(), v);
        }
        if let Some(v) = parse_optional_json("groups_schema_json", groups_schema_json)? {
            format_value.insert("groups".to_string(), v);
        }
        if let Some(v) = parse_optional_json("keywords_json", keywords_json)? {
            format_value.insert("keywords".to_string(), v);
        }
        if let Some(s) = description.filter(|s| !s.is_empty()) {
            format_value.insert(
                "description".to_string(),
                serde_json::Value::String(s.to_string()),
            );
        }
        if let Some(mut v) = parse_optional_json("config_json", config_json)? {
            // `DataContractConfig` is a `#[serde(tag =
            // "$formatVersion")]` enum; the Swift form-builder sends a
            // bare flags dict. Inject the platform-selected config
            // version (V1 under v12) when missing so the tagged-enum
            // dispatch picks the variant the network accepts.
            if let Some(obj) = v.as_object_mut() {
                obj.entry("$formatVersion").or_insert_with(|| {
                    serde_json::Value::String(config_format_version.to_string())
                });
            }
            format_value.insert("config".to_string(), v);
        }

        // Round-trip through a string instead of `from_value` — see
        // `create_data_contract_with_signer` for why the tagged-enum
        // dispatch needs `to_string` + `from_str`.
        let serialized = serde_json::to_string_pretty(&serde_json::Value::Object(format_value))
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to serialize updated contract format: {e}"
                ))
            })?;
        let format: DataContractInSerializationFormat =
            serde_json::from_str(&serialized).map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to assemble updated contract format: {e}\n\nAssembled JSON:\n{serialized}"
                ))
            })?;

        let mut errors = vec![];
        let updated_contract =
            DataContract::try_from_platform_versioned(format, true, &mut errors, platform_version)
                .map_err(|e| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "Failed to build updated contract: {e}\n\nAssembled JSON:\n{serialized}"
                    ))
                })?;

        // 4. Build + broadcast the update transition. There's no
        //    `UpdateContract` SDK extension mirroring `PutContract`,
        //    so assemble the `DataContractUpdateTransition` directly:
        //    fetch+bump the identity-contract nonce, sign via the
        //    external signer, broadcast on the platform-wallet 8 MB
        //    worker stack, and wait for the confirmed contract.
        let new_identity_contract_nonce = self
            .sdk
            .get_identity_contract_nonce(*owner_identity_id, *contract_id, true, None)
            .await
            .map_err(PlatformWalletError::Sdk)?;

        let key_id = {
            use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
            signing_key.id()
        };
        let partial_identity = PartialIdentity {
            id: *owner_identity_id,
            loaded_public_keys: [(key_id, signing_key.clone())].into_iter().collect(),
            balance: None,
            revision: None,
            not_found_public_keys: Default::default(),
        };

        let transition = DataContractUpdateTransition::new_from_data_contract(
            updated_contract,
            &partial_identity,
            key_id,
            new_identity_contract_nonce,
            0,
            &SignerRef(signer),
            platform_version,
            None,
        )
        .await
        .map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to build contract-update transition: {e}"
            ))
        })?;

        transition
            .broadcast(&self.sdk, None)
            .await
            .map_err(PlatformWalletError::Sdk)?;

        let confirmed = DataContract::wait_for_response(&self.sdk, transition, None)
            .await
            .map_err(PlatformWalletError::Sdk)?;

        Ok(confirmed)
    }
}

/// Parse a required JSON `&str`. `field_name` only feeds the error
/// message — it has no semantic effect.
fn parse_required_json(
    field_name: &str,
    s: &str,
) -> Result<serde_json::Value, PlatformWalletError> {
    serde_json::from_str(s).map_err(|e| {
        PlatformWalletError::InvalidIdentityData(format!("Invalid {field_name} JSON: {e}"))
    })
}

/// Parse an optional JSON `&str`. `None` / empty input yields
/// `Ok(None)` so the caller can omit the field entirely from the
/// assembled format Value (and let the V1 struct's serde defaults
/// take over).
fn parse_optional_json(
    field_name: &str,
    s: Option<&str>,
) -> Result<Option<serde_json::Value>, PlatformWalletError> {
    let Some(s) = s.filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    serde_json::from_str(s).map(Some).map_err(|e| {
        PlatformWalletError::InvalidIdentityData(format!("Invalid {field_name} JSON: {e}"))
    })
}
