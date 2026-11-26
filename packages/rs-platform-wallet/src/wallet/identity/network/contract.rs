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
//! Mirrors the identity-flow shape:
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

use super::signing_key::AvailableSigningKey;
use async_trait::async_trait;

use dpp::address_funds::AddressWitness;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::v2::DataContractConfigGettersV2;
use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
use dpp::data_contract::INITIAL_DATA_CONTRACT_VERSION;
use dpp::identity::signer::Signer;
use dpp::identity::{IdentityPublicKey, KeyType, PartialIdentity, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use dpp::prelude::{DataContract, Identifier};
use dpp::state_transition::data_contract_update_transition::methods::DataContractUpdateTransitionMethodsV0;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::version::{PlatformVersion, TryFromPlatformVersioned};
use dpp::ProtocolError;
use std::collections::BTreeSet;

use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;
use dash_sdk::platform::transition::put_contract::{
    ensure_private_document_entries_fit, PutContract,
};
use dash_sdk::platform::Fetch;

use crate::error::{preserve_signer_key_unavailable_or, PlatformWalletError};

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
    /// - `config_json`: `DataContractConfig` JSON, or `None`. The
    ///   function always tags the assembled config with the
    ///   protocol-required `$formatVersion` (the running
    ///   `PlatformVersion`'s `contract_versions.config
    ///   .default_current_version`) — since protocol v12 the network
    ///   rejects a V0 config, so a hardcoded "0" tag would fail. A
    ///   flags-only or absent config deserializes into a valid V1
    ///   because every `DataContractConfigV1` field has a serde
    ///   default.
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
            drop(wm);
            // Contract create requires CRITICAL + AUTHENTICATION +
            // ECDSA_SECP256K1 specifically — DPP rejects HIGH /
            // MEDIUM / non-ECDSA keys on this state-transition
            // shape.
            identity
                .available_signing_key(
                    signer,
                    Purpose::AUTHENTICATION,
                    &[SecurityLevel::CRITICAL],
                    &[KeyType::ECDSA_SECP256K1],
                    false,
                )?
                .ok_or_else(|| {
                    PlatformWalletError::InvalidIdentityData(
                        "No CRITICAL authentication key found on owner identity \
                         (required to sign a contract-create state transition)"
                            .to_string(),
                    )
                })?
                .clone()
        };

        // Protocol-required config version. Since protocol v12 the
        // network rejects a V0 `DataContractConfig` (it lacks
        // `sized_integer_types`): the data-contract-create basic-
        // structure validator enforces
        // `config().version() >= contract_versions.config.min_version`,
        // and that minimum is 1 for v12+. We therefore tag every
        // config we build with the protocol's
        // `default_current_version` (which equals `min_version` here)
        // rather than a hardcoded "0".
        let platform_version = self.sdk.version();
        let config_format_version = platform_version
            .dpp
            .contract_versions
            .config
            .default_current_version;

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
        // We always insert a `config` block (even when `config_json` is
        // None) so the version tag is explicit rather than relying on
        // the serialization format's `#[serde(default)]`; this keeps the
        // emitted config aligned with the running protocol version. See
        // `build_config_object` for the full assembly contract.
        let config_obj = build_config_object(config_json, config_format_version)?;
        format_value.insert("config".to_string(), serde_json::Value::Object(config_obj));

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
                preserve_signer_key_unavailable_or(e, |e| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "Failed to put data contract to platform: {e}"
                    ))
                })
            })?;

        Ok(confirmed)
    }

    /// Update an existing data contract owned by `owner_identity_id`
    /// and broadcast the change to Platform.
    ///
    /// Mirrors `create_data_contract_with_signer` but targets an
    /// already-registered contract. Unlike create, the caller-supplied
    /// JSON sections are *merged onto the fetched on-chain contract*
    /// rather than used to build the payload from scratch. A
    /// `DataContractUpdateTransition` broadcasts a complete contract
    /// definition (not a diff), so anything the caller omits would
    /// otherwise be reset to serde defaults — wiping live keywords /
    /// description / config and, worse, dropping document-type / token /
    /// group entries (which DPP rejects as illegal removals). Seeding
    /// from `existing` keeps every untouched section intact. The
    /// function:
    ///   1. Looks up `owner_identity_id` in the in-memory wallet
    ///      manager and picks the CRITICAL + AUTHENTICATION + ECDSA
    ///      key (same triple a contract-create signature requires).
    ///   2. Fetches the *current* contract from Platform and validates
    ///      that it is actually owned by `owner_identity_id` — this
    ///      runs *before* any nonce fetch/bump so a bad (owner,
    ///      contract) pair fails as a local validation error instead of
    ///      consuming nonce state. The bumped version is
    ///      `existing.version() + 1` (DPP rejects an update that doesn't
    ///      strictly increment the version), so the caller doesn't have
    ///      to track local version state.
    ///   3. Serializes `existing` into its canonical
    ///      `DataContractInSerializationFormat` JSON (already a valid V1
    ///      on-chain contract, so the config version is preserved with
    ///      no manual tagging), then overlays the caller's sections:
    ///      `documentSchemas` / `tokens` / `groups` are merged key-by-
    ///      key (add or replace an entry; never drop an existing one),
    ///      and `keywords` / `description` / `config` are overridden
    ///      only when the caller supplies them. The id + owner stay as
    ///      `existing`'s and the version is the bumped value. The merged
    ///      Value is deserialized via the public
    ///      `DataContractInSerializationFormat` enum entry point and
    ///      validated through `try_from_platform_versioned`.
    ///   4. Fetches + bumps the identity-contract nonce, builds a
    ///      `DataContractUpdateTransition`, signs it via the external
    ///      signer, broadcasts, and waits for the confirmed contract.
    ///
    /// `expected_current_version` binds the update to the contract a
    /// caller reviewed (see `preview_data_contract_update`): when it is
    /// `Some` and the contract Platform now holds is at another version,
    /// the update is refused before any nonce is read, and the caller
    /// should review again. `None` updates whatever version is live.
    ///
    /// A broadcast that may have reached Platform but whose result could
    /// not be confirmed comes back as the SDK's
    /// `StateTransitionResultUnconfirmed`: the update may have executed.
    ///
    /// # JSON shapes
    ///
    /// Identical to `create_data_contract_with_signer`, but every
    /// section is *additive / overlay*: an empty `documents_schema_json`
    /// (`"{}"`) or a `None` for an optional section preserves whatever
    /// is already on-chain rather than clearing it. Because the payload
    /// is seeded from the fetched V1 contract, the config keeps its
    /// on-chain `$formatVersion` automatically; a caller-supplied
    /// `config_json` is only used to override it.
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
        expected_current_version: Option<u32>,
        signer: &S,
    ) -> Result<DataContract, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        // 1. Owner identity + signing key from the wallet manager.
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
            drop(wm);
            // Contract update requires the same CRITICAL +
            // AUTHENTICATION + ECDSA_SECP256K1 key as create — DPP
            // rejects HIGH / MEDIUM / non-ECDSA keys on this
            // state-transition shape.
            identity
                .available_signing_key(
                    signer,
                    Purpose::AUTHENTICATION,
                    &[SecurityLevel::CRITICAL],
                    &[KeyType::ECDSA_SECP256K1],
                    false,
                )?
                .ok_or_else(|| {
                    PlatformWalletError::InvalidIdentityData(
                        "No CRITICAL authentication key found on owner identity \
                         (required to sign a contract-update state transition)"
                            .to_string(),
                    )
                })?
                .clone()
        };

        // 2 + 3. Fetch the live contract and build the contract the
        //    update puts in its place. `preview_data_contract_update`
        //    builds it the same way, so what a preview reports is what
        //    this broadcasts.
        //    A contract that changed since the caller's review is refused
        //    there, before the draft is built and before any nonce is read.
        let ContractUpdateDraft { updated, .. } = self
            .draft_data_contract_update(
                owner_identity_id,
                contract_id,
                documents_schema_json,
                tokens_schema_json,
                groups_schema_json,
                keywords_json,
                description,
                config_json,
                expected_current_version,
            )
            .await?;
        let platform_version = self.sdk.version();

        // 4. Build + broadcast the update transition. There's no
        //    `UpdateContract` SDK extension mirroring `PutContract`,
        //    so assemble the `DataContractUpdateTransition` directly:
        //    fetch+bump the identity-contract nonce, sign via the
        //    external signer, broadcast on the platform-wallet 8 MB
        //    worker stack, and wait for the contract. The SDK's combined
        //    broadcast and wait marks a wait that fails after the
        //    transition may have been accepted as unconfirmed, and waits
        //    for the affected state, as an update proof requires.
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
            updated,
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

        let confirmed = transition
            .broadcast_and_wait_for_affected_state::<DataContract>(&self.sdk, None)
            .await
            .map_err(PlatformWalletError::Sdk)?;

        // The proof-verified contract replaces the SDK's resolver entry, so
        // a document of a type this update added can be built and verified
        // on this SDK without another fetch.
        self.register_contract_for_proof_verification(&confirmed);

        Ok(confirmed)
    }

    /// Report what `update_data_contract_with_signer` would change, called
    /// with the same sections, without signing or broadcasting anything.
    ///
    /// Fetches the contract as Platform holds it, checks that
    /// `owner_identity_id` owns it, merges the supplied sections onto it and
    /// builds the updated contract exactly as the update does, then compares
    /// the two. A section the update would refuse to build (JSON that does
    /// not parse, a schema the contract parser rejects, a private document
    /// type whose entries do not fit) is the same error here.
    ///
    /// The preview says what the update changes, not whether Platform accepts
    /// it: the contract update rules are checked by the network when the
    /// update is broadcast. It reads the contract once, so a contract that
    /// changes between the preview and the update is updated as it is then.
    #[allow(clippy::too_many_arguments)]
    pub async fn preview_data_contract_update(
        &self,
        owner_identity_id: &Identifier,
        contract_id: &Identifier,
        documents_schema_json: &str,
        tokens_schema_json: Option<&str>,
        groups_schema_json: Option<&str>,
        keywords_json: Option<&str>,
        description: Option<&str>,
        config_json: Option<&str>,
    ) -> Result<DataContractUpdatePreview, PlatformWalletError> {
        let draft = self
            .draft_data_contract_update(
                owner_identity_id,
                contract_id,
                documents_schema_json,
                tokens_schema_json,
                groups_schema_json,
                keywords_json,
                description,
                config_json,
                None,
            )
            .await?;
        let platform_version = self.sdk.version();
        let existing_format = contract_format_json(&draft.existing, "existing", platform_version)?;
        let updated_format = contract_format_json(&draft.updated, "updated", platform_version)?;

        let mut preview = DataContractUpdatePreview::between(
            draft.existing.version(),
            draft.updated.version(),
            &existing_format,
            &updated_format,
        );
        let (added, removed) = moderator_changes(
            &named_moderators(&draft.existing),
            &named_moderators(&draft.updated),
        );
        preview.moderators_added = added;
        preview.moderators_removed = removed;
        Ok(preview)
    }

    /// Fetch the contract `contract_id` as Platform holds it and build the
    /// contract an update with these sections puts in its place: the shared
    /// first half of `update_data_contract_with_signer` and
    /// `preview_data_contract_update`. Reads Platform, writes nothing and
    /// consumes no nonce.
    ///
    /// With `expected_current_version`, a contract Platform now holds at
    /// another version is refused right after the ownership check, before
    /// the sections are merged, so a build error cannot hide the mismatch.
    #[allow(clippy::too_many_arguments)]
    async fn draft_data_contract_update(
        &self,
        owner_identity_id: &Identifier,
        contract_id: &Identifier,
        documents_schema_json: &str,
        tokens_schema_json: Option<&str>,
        groups_schema_json: Option<&str>,
        keywords_json: Option<&str>,
        description: Option<&str>,
        config_json: Option<&str>,
        expected_current_version: Option<u32>,
    ) -> Result<ContractUpdateDraft, PlatformWalletError> {
        // 2. Fetch the live contract. We seed the update payload from
        //    it (so omitted sections are preserved) and read its
        //    current version to bump.
        // A failed fetch (a timeout, a proof that does not verify) keeps the
        // SDK's typed error; only a contract that is not there is reported
        // as invalid input.
        let existing = DataContract::fetch(&self.sdk, *contract_id)
            .await
            .map_err(PlatformWalletError::Sdk)?
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Data contract {contract_id} not found on Platform; cannot update"
                ))
            })?;

        // Reject an owner/contract mismatch *before* touching nonce
        // state. If `contract_id` belongs to a different owner, the
        // network would reject the transition anyway — but only after
        // we'd fetched+bumped the identity-contract nonce for the
        // supplied pair, turning a bad request into stateful nonce
        // consumption. Fail it as a local validation error instead.
        if existing.owner_id() != *owner_identity_id {
            return Err(PlatformWalletError::InvalidIdentityData(format!(
                "Data contract {contract_id} is owned by {}, not {owner_identity_id}; \
                 cannot update with this identity",
                existing.owner_id()
            )));
        }

        if let Some(expected) = expected_current_version {
            if existing.version() != expected {
                return Err(PlatformWalletError::InvalidIdentityData(format!(
                    "Data contract {contract_id} is at version {}, not the reviewed version \
                     {expected}; review the update again",
                    existing.version()
                )));
            }
        }

        let new_version = existing.version().checked_add(1).ok_or_else(|| {
            PlatformWalletError::InvalidIdentityData(
                "Contract version overflow; cannot update".to_string(),
            )
        })?;

        // 3. Build the updated serialization-format payload by merging
        //    the caller's sections onto the fetched contract. `existing`
        //    is already a valid V1 on-chain contract, so serializing it
        //    to the canonical `DataContractInSerializationFormat` JSON
        //    gives us a base that carries every live field (config keeps
        //    its on-chain version, document/token/group sets are
        //    complete). We then overlay only what the caller supplied.
        let platform_version = self.sdk.version();
        let base_value = contract_format_json(&existing, "existing", platform_version)?;

        let merged_value = merge_contract_update_payload(
            base_value,
            new_version,
            parse_required_json("documents_schema_json", documents_schema_json)?,
            parse_optional_json("tokens_schema_json", tokens_schema_json)?,
            parse_optional_json("groups_schema_json", groups_schema_json)?,
            parse_optional_json("keywords_json", keywords_json)?,
            description.filter(|s| !s.is_empty()),
            parse_optional_json("config_json", config_json)?,
        )?;

        // Round-trip through a string instead of `from_value` — see
        // `create_data_contract_with_signer` for why the tagged-enum
        // dispatch needs `to_string` + `from_str`.
        let serialized = serde_json::to_string_pretty(&merged_value).map_err(|e| {
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
        let updated =
            DataContract::try_from_platform_versioned(format, true, &mut errors, platform_version)
                .map_err(|e| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "Failed to build updated contract: {e}\n\nAssembled JSON:\n{serialized}"
                    ))
                })?;

        // A private document type the update adds, or whose properties it changes, takes entries
        // that must hold its largest payload, as `PutContract` checks on create; `existing` is the
        // contract as Platform holds it, so an unchanged type is not checked again.
        ensure_private_document_entries_fit(&updated, Some(&existing))
            .map_err(PlatformWalletError::Sdk)?;

        Ok(ContractUpdateDraft { existing, updated })
    }
}

/// The contract Platform holds and the contract an update would put in its
/// place, as `IdentityWallet::draft_data_contract_update` builds them.
struct ContractUpdateDraft {
    existing: DataContract,
    updated: DataContract,
}

/// The entries of one keyed section of a contract (document types by name,
/// tokens or groups by position) that an update adds or changes. An update
/// never removes an entry: the sections it supplies are merged onto the
/// contract Platform holds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContractSectionChanges {
    /// Keys the contract does not have yet.
    pub added: Vec<String>,
    /// Keys the contract has, with a different definition.
    pub changed: Vec<String>,
}

impl ContractSectionChanges {
    /// Whether the update leaves the section as it is.
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.changed.is_empty()
    }
}

/// What an update would change in a data contract, read against the contract
/// as Platform holds it. Built by
/// `IdentityWallet::preview_data_contract_update`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataContractUpdatePreview {
    /// The version of the contract Platform holds.
    pub current_version: u32,
    /// The version the update would give it.
    pub new_version: u32,
    /// Document types, by name.
    pub document_types: ContractSectionChanges,
    /// Tokens, by contract position.
    pub tokens: ContractSectionChanges,
    /// Groups, by contract position.
    pub groups: ContractSectionChanges,
    /// Whether the keywords would differ.
    pub keywords_changed: bool,
    /// Whether the description would differ.
    pub description_changed: bool,
    /// Whether the contract configuration would differ.
    pub config_changed: bool,
    /// The members of the contract configuration that would differ (for
    /// example `readonly` or `moderation`), sorted.
    pub config_changed_fields: Vec<String>,
    /// Identities the configuration would newly name as moderators, base58,
    /// sorted: the appointed set, or the interim set of an elected contract.
    pub moderators_added: Vec<String>,
    /// Identities the configuration would no longer name as moderators.
    pub moderators_removed: Vec<String>,
}

impl DataContractUpdatePreview {
    /// Whether the update would change anything besides the version, which
    /// every update raises.
    pub fn has_changes(&self) -> bool {
        !self.document_types.is_empty()
            || !self.tokens.is_empty()
            || !self.groups.is_empty()
            || self.keywords_changed
            || self.description_changed
            || self.config_changed
    }

    /// Compare two contracts in their serialization-format JSON
    /// (`contract_format_json`). Both sides are read in that one form, so a
    /// section the update did not touch compares equal however the caller
    /// wrote its JSON.
    fn between(
        current_version: u32,
        new_version: u32,
        existing: &serde_json::Value,
        updated: &serde_json::Value,
    ) -> Self {
        let differs = |field: &str| existing.get(field) != updated.get(field);
        Self {
            current_version,
            new_version,
            document_types: section_changes(existing, updated, "documentSchemas"),
            tokens: section_changes(existing, updated, "tokens"),
            groups: section_changes(existing, updated, "groups"),
            keywords_changed: differs("keywords"),
            description_changed: differs("description"),
            config_changed: differs("config"),
            config_changed_fields: object_member_changes(
                existing.get("config"),
                updated.get("config"),
            ),
            moderators_added: Vec::new(),
            moderators_removed: Vec::new(),
        }
    }
}

/// The members of two JSON objects whose values differ, sorted. A side that
/// is absent or not an object counts as empty.
fn object_member_changes(
    existing: Option<&serde_json::Value>,
    updated: Option<&serde_json::Value>,
) -> Vec<String> {
    let empty = serde_json::Map::new();
    let existing = existing
        .and_then(serde_json::Value::as_object)
        .unwrap_or(&empty);
    let updated = updated
        .and_then(serde_json::Value::as_object)
        .unwrap_or(&empty);
    let names: BTreeSet<&String> = existing.keys().chain(updated.keys()).collect();
    names
        .into_iter()
        .filter(|name| existing.get(*name) != updated.get(*name))
        .cloned()
        .collect()
}

/// The identities `contract`'s configuration names as moderators: the
/// appointed set, or the interim set of an elected declaration. Empty when
/// the contract has no moderation, or only its owner moderates.
fn named_moderators(contract: &DataContract) -> BTreeSet<Identifier> {
    contract
        .config()
        .moderation()
        .and_then(|moderation| moderation.moderators.identity_ids())
        .cloned()
        .unwrap_or_default()
}

/// The identities `updated` names that `existing` does not, and the reverse,
/// each base58 and sorted.
fn moderator_changes(
    existing: &BTreeSet<Identifier>,
    updated: &BTreeSet<Identifier>,
) -> (Vec<String>, Vec<String>) {
    let base58 = |ids: std::collections::btree_set::Difference<'_, Identifier>| {
        let mut names: Vec<String> = ids
            .map(|id| bs58::encode(id.to_buffer()).into_string())
            .collect();
        names.sort();
        names
    };
    (
        base58(updated.difference(existing)),
        base58(existing.difference(updated)),
    )
}

/// A document type definition with its `indices` listed by name, so a
/// reordering of unchanged indexes does not count as a change. Platform
/// reads indexes by name; a changed index, or a changed property order
/// within one, still differs.
fn with_indices_by_name(definition: &serde_json::Value) -> serde_json::Value {
    let mut definition = definition.clone();
    if let Some(serde_json::Value::Array(indices)) = definition.get_mut("indices") {
        indices.sort_by(|a, b| {
            let name = |index: &serde_json::Value| {
                index
                    .get("name")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned)
            };
            name(a).cmp(&name(b))
        });
    }
    definition
}

/// The keys of the keyed section `section` that `updated` adds to, or
/// defines differently from, `existing`. A section that is absent or not an
/// object counts as empty. Keys are sorted, numeric positions by value.
fn section_changes(
    existing: &serde_json::Value,
    updated: &serde_json::Value,
    section: &str,
) -> ContractSectionChanges {
    let existing_entries = existing.get(section).and_then(serde_json::Value::as_object);
    let mut changes = ContractSectionChanges::default();
    let Some(updated_entries) = updated.get(section).and_then(serde_json::Value::as_object) else {
        return changes;
    };
    // A document type is compared as text, member order included:
    // `serde_json::Value` equality ignores the order of an object's members,
    // but Platform stores and checks it (protocol version 15 refuses a
    // reorder), so a reordered schema must show as a change.
    let differs = |existing: &serde_json::Value, updated: &serde_json::Value| {
        if section == "documentSchemas" {
            let existing_text = with_indices_by_name(existing).to_string();
            let updated_text = with_indices_by_name(updated).to_string();
            existing_text != updated_text
        } else {
            existing != updated
        }
    };
    for (key, definition) in updated_entries {
        match existing_entries.and_then(|entries| entries.get(key)) {
            None => changes.added.push(key.clone()),
            Some(existing_definition) if differs(existing_definition, definition) => {
                changes.changed.push(key.clone())
            }
            Some(_) => {}
        }
    }
    let positions_then_names = |key: &String| match key.parse::<u64>() {
        Ok(position) => (0, position, String::new()),
        Err(_) => (1, 0, key.clone()),
    };
    changes.added.sort_by_cached_key(positions_then_names);
    changes.changed.sort_by_cached_key(positions_then_names);
    changes
}

/// `contract` in its serialization-format JSON, the form the update merges
/// the caller's sections onto.
/// `which` names the contract in errors (`existing` or `updated`).
fn contract_format_json(
    contract: &DataContract,
    which: &str,
    platform_version: &PlatformVersion,
) -> Result<serde_json::Value, PlatformWalletError> {
    let format =
        DataContractInSerializationFormat::try_from_platform_versioned(contract, platform_version)
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to convert {which} contract to serialization format: {e}"
                ))
            })?;
    serde_json::to_value(&format).map_err(|e| {
        PlatformWalletError::InvalidIdentityData(format!(
            "Failed to serialize {which} contract format: {e}"
        ))
    })
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

/// Overlay the caller-supplied update sections onto `base` — the
/// serialization-format JSON of the *fetched* on-chain contract.
///
/// `base` must be a JSON object (the serialized
/// `DataContractInSerializationFormat`), already carrying the live
/// `documentSchemas` / `tokens` / `groups` / `keywords` / `description`
/// / `config`. The merge is deliberately *additive* so an update that
/// only touches one section never resets the others to serde defaults:
///
/// - `version` is set to `new_version` (the bumped value).
/// - `documents` / `tokens` / `groups`: each supplied map is merged
///   key-by-key into the existing map — an entry is added or replaced,
///   but existing keys the caller didn't mention are kept. (DPP rejects
///   document-type / token / group *removals* on update, so dropping
///   the untouched ones would fail the transition.)
/// - `keywords` / `description` / `config`: overridden wholesale, but
///   only when the caller actually supplied them. A supplied `config`
///   that lacks the `$formatVersion` tag inherits the base contract's
///   on-chain config version so the tagged-enum dispatch still resolves.
///
/// The id + owner stay as whatever `base` carries (the fetched
/// contract's), so the caller cannot retarget the update.
#[allow(clippy::too_many_arguments)]
fn merge_contract_update_payload(
    mut base: serde_json::Value,
    new_version: u32,
    documents: serde_json::Value,
    tokens: Option<serde_json::Value>,
    groups: Option<serde_json::Value>,
    keywords: Option<serde_json::Value>,
    description: Option<&str>,
    config: Option<serde_json::Value>,
) -> Result<serde_json::Value, PlatformWalletError> {
    let obj = base.as_object_mut().ok_or_else(|| {
        PlatformWalletError::InvalidIdentityData(
            "Existing contract did not serialize to a JSON object".to_string(),
        )
    })?;

    obj.insert(
        "version".to_string(),
        serde_json::Value::Number(serde_json::Number::from(new_version)),
    );

    merge_map_section(obj, "documentSchemas", documents)?;
    if let Some(v) = tokens {
        merge_map_section(obj, "tokens", canonical_position_keys("tokens", v)?)?;
    }
    if let Some(v) = groups {
        merge_map_section(obj, "groups", canonical_position_keys("groups", v)?)?;
    }

    if let Some(v) = keywords {
        obj.insert("keywords".to_string(), v);
    }
    if let Some(s) = description {
        obj.insert(
            "description".to_string(),
            serde_json::Value::String(s.to_string()),
        );
    }
    if let Some(mut v) = config {
        // `DataContractConfig` is a `#[serde(tag = "$formatVersion")]`
        // enum and the Swift form-builder sends a bare flags dict. When
        // the caller omits the tag, inherit the base contract's
        // on-chain config version so the tagged-enum dispatch resolves
        // to the variant the network already accepts.
        if let Some(supplied) = v.as_object_mut() {
            if !supplied.contains_key("$formatVersion") {
                if let Some(existing_tag) = obj
                    .get("config")
                    .and_then(|c| c.get("$formatVersion"))
                    .cloned()
                {
                    supplied.insert("$formatVersion".to_string(), existing_tag);
                }
            }
        }
        obj.insert("config".to_string(), v);
    }

    Ok(base)
}

/// Re-key a caller-supplied tokens or groups map by its contract
/// positions written in canonical decimal form, the form the fetched
/// contract's section carries.
///
/// The contract parser reads a key as a `u16` position, so `"0"` and
/// `"00"` would name the same token or group, and one of the two
/// definitions would be dropped without a word. A key that is not a
/// position, or two keys naming one position, is refused instead. A
/// value that is not an object is left for `merge_map_section` to
/// judge.
fn canonical_position_keys(
    section: &str,
    supplied: serde_json::Value,
) -> Result<serde_json::Value, PlatformWalletError> {
    let serde_json::Value::Object(entries) = supplied else {
        return Ok(supplied);
    };
    let mut canonical = serde_json::Map::new();
    for (key, definition) in entries {
        let position: u16 = key.trim().parse().map_err(|_| {
            PlatformWalletError::InvalidIdentityData(format!(
                "{section} update payload key {key:?} is not a contract position (0 to {})",
                u16::MAX
            ))
        })?;
        if canonical.insert(position.to_string(), definition).is_some() {
            return Err(PlatformWalletError::InvalidIdentityData(format!(
                "{section} update payload names position {position} more than once"
            )));
        }
    }
    Ok(serde_json::Value::Object(canonical))
}

/// Merge a caller-supplied keyed map (`addition`, e.g. document
/// schemas keyed by type name, or tokens keyed by stringified slot)
/// into the same section already present in `base`. Adds or replaces
/// each key; never drops an existing one. Inserts the section if the
/// base didn't carry it. `addition` must be a JSON object.
fn merge_map_section(
    base: &mut serde_json::Map<String, serde_json::Value>,
    section: &str,
    addition: serde_json::Value,
) -> Result<(), PlatformWalletError> {
    let addition = match addition {
        serde_json::Value::Object(map) => map,
        // An empty / absent section is a no-op overlay (preserve base).
        serde_json::Value::Null => return Ok(()),
        _ => {
            return Err(PlatformWalletError::InvalidIdentityData(format!(
                "{section} update payload must be a JSON object keyed by entry name"
            )))
        }
    };

    match base.get_mut(section) {
        Some(serde_json::Value::Object(existing)) => {
            for (k, v) in addition {
                existing.insert(k, v);
            }
        }
        _ => {
            base.insert(section.to_string(), serde_json::Value::Object(addition));
        }
    }
    Ok(())
}

/// Assemble the `config` block for a data-contract serialization-format
/// payload, tagged with the protocol-required `$formatVersion`.
///
/// `DataContractConfig` is itself a `#[serde(tag = "$formatVersion")]`
/// enum; the Swift form-builder shape sends a bare flags dict (e.g.
/// `{"canBeDeleted": true}`) or omits config entirely. We tag the dict
/// with `config_format_version` so the tagged-enum dispatch picks a
/// variant the network will accept — since protocol v12 the network
/// rejects a V0 config, so the caller passes the running
/// `PlatformVersion`'s `contract_versions.config.default_current_version`
/// (1 on v12) rather than a hardcoded "0". `DataContractConfigV1` is
/// `#[serde(rename_all = "camelCase", default)]`, so a flags-only (or
/// empty) dict deserializes into a valid V1 — every field, including
/// `sized_integer_types: true`, has a serde default.
///
/// The `$formatVersion` insert is an **unconditional overwrite**: any
/// caller-supplied wire-level `$formatVersion` inside `config_json` is
/// replaced. This is deliberate — the helper's contract is to always
/// emit a network-acceptable version tag, no caller in this codebase
/// sets the wire-level tag (the whole point of the helper is to abstract
/// it), and honoring a caller-supplied "0" on v12+ would only earn a
/// network rejection.
///
/// Inputs:
///   - `None` / empty `config_json` -> empty map, then the version tag.
///   - `Some(object)` -> that object's fields preserved, then the tag.
///   - `Some(non-object)` (array / string / number) ->
///     `PlatformWalletError::InvalidIdentityData`.
fn build_config_object(
    config_json: Option<&str>,
    config_format_version: u16,
) -> Result<serde_json::Map<String, serde_json::Value>, PlatformWalletError> {
    let mut config_obj = match parse_optional_json("config_json", config_json)? {
        Some(serde_json::Value::Object(map)) => map,
        Some(other) => {
            return Err(PlatformWalletError::InvalidIdentityData(format!(
                "config_json must be a JSON object, got: {other}"
            )));
        }
        None => serde_json::Map::new(),
    };
    config_obj.insert(
        "$formatVersion".to_string(),
        serde_json::Value::String(config_format_version.to_string()),
    );
    Ok(config_obj)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// A minimal serialized-V1-contract base value, shaped like the
    /// output of `serde_json::to_value(&DataContractInSerializationFormat)`.
    fn base_contract() -> serde_json::Value {
        json!({
            "$formatVersion": "1",
            "id": "11111111111111111111111111111111",
            "ownerId": "22222222222222222222222222222222",
            "version": 1,
            "documentSchemas": {
                "note": { "type": "object" }
            },
            "tokens": {
                "0": { "conventions": {} }
            },
            "groups": {},
            "keywords": ["alpha", "beta"],
            "description": "original description",
            "config": { "$formatVersion": "1", "canBeDeleted": false }
        })
    }

    #[test]
    fn merge_preserves_omitted_sections_and_adds_supplied_document() {
        // Caller adds one new document type and supplies nothing else.
        let merged = merge_contract_update_payload(
            base_contract(),
            2,
            json!({ "comment": { "type": "object" } }),
            None,
            None,
            None,
            None,
            None,
        )
        .expect("merge");

        // Version bumped.
        assert_eq!(merged["version"], json!(2));

        // New document type added; existing one preserved (not dropped).
        let docs = merged["documentSchemas"].as_object().unwrap();
        assert!(docs.contains_key("note"), "existing doc type preserved");
        assert!(docs.contains_key("comment"), "new doc type added");
        assert_eq!(docs.len(), 2);

        // Omitted sections kept verbatim from the fetched contract.
        assert_eq!(merged["keywords"], json!(["alpha", "beta"]));
        assert_eq!(merged["description"], json!("original description"));
        assert_eq!(merged["tokens"], json!({ "0": { "conventions": {} } }));
        assert_eq!(
            merged["config"],
            json!({ "$formatVersion": "1", "canBeDeleted": false })
        );

        // Id + owner unchanged.
        assert_eq!(merged["id"], json!("11111111111111111111111111111111"));
        assert_eq!(merged["ownerId"], json!("22222222222222222222222222222222"));
    }

    #[test]
    fn merge_replaces_existing_document_entry_by_key() {
        let merged = merge_contract_update_payload(
            base_contract(),
            2,
            json!({ "note": { "type": "object", "revised": true } }),
            None,
            None,
            None,
            None,
            None,
        )
        .expect("merge");

        let docs = merged["documentSchemas"].as_object().unwrap();
        assert_eq!(docs.len(), 1, "no extra key added");
        assert_eq!(docs["note"], json!({ "type": "object", "revised": true }));
    }

    #[test]
    fn merge_empty_documents_is_a_noop_overlay() {
        // Token-only update path passes "{}" for documents.
        let merged = merge_contract_update_payload(
            base_contract(),
            2,
            json!({}),
            Some(json!({ "1": { "conventions": {} } })),
            None,
            None,
            None,
            None,
        )
        .expect("merge");

        // Existing document type still present.
        assert!(merged["documentSchemas"]
            .as_object()
            .unwrap()
            .contains_key("note"));
        // New token slot merged alongside the existing one.
        let tokens = merged["tokens"].as_object().unwrap();
        assert!(tokens.contains_key("0"), "existing token slot preserved");
        assert!(tokens.contains_key("1"), "new token slot added");
    }

    #[test]
    fn merge_overrides_keywords_description_and_config_only_when_supplied() {
        let merged = merge_contract_update_payload(
            base_contract(),
            2,
            json!({}),
            None,
            None,
            Some(json!(["gamma"])),
            Some("new description"),
            // Caller supplies a config without the format tag — should
            // inherit the base contract's on-chain version.
            Some(json!({ "canBeDeleted": true })),
        )
        .expect("merge");

        assert_eq!(merged["keywords"], json!(["gamma"]));
        assert_eq!(merged["description"], json!("new description"));
        assert_eq!(merged["config"]["$formatVersion"], json!("1"));
        assert_eq!(merged["config"]["canBeDeleted"], json!(true));
    }

    #[test]
    fn should_report_added_and_changed_entries_of_each_section() {
        let existing = base_contract();
        let updated = merge_contract_update_payload(
            existing.clone(),
            2,
            json!({
                "note": { "type": "object", "revised": true },
                "comment": { "type": "object" }
            }),
            Some(json!({ "0": { "conventions": {} }, "10": {}, "2": {} })),
            Some(json!({ "0": { "requiredPower": 1 } })),
            None,
            None,
            None,
        )
        .expect("merge");

        let preview = DataContractUpdatePreview::between(1, 2, &existing, &updated);

        assert_eq!(preview.current_version, 1);
        assert_eq!(preview.new_version, 2);
        assert_eq!(preview.document_types.added, vec!["comment"]);
        assert_eq!(preview.document_types.changed, vec!["note"]);
        // Token 0 was sent again as it is, so it is not a change; positions
        // sort by value, not as text.
        assert_eq!(preview.tokens.added, vec!["2", "10"]);
        assert!(preview.tokens.changed.is_empty());
        assert_eq!(preview.groups.added, vec!["0"]);
        assert!(!preview.keywords_changed);
        assert!(!preview.description_changed);
        assert!(!preview.config_changed);
        assert!(preview.has_changes());
    }

    #[test]
    fn should_report_changed_keywords_description_and_config() {
        let existing = base_contract();
        let updated = merge_contract_update_payload(
            existing.clone(),
            2,
            json!({}),
            None,
            None,
            Some(json!(["gamma"])),
            Some("new description"),
            Some(json!({ "canBeDeleted": true })),
        )
        .expect("merge");

        let preview = DataContractUpdatePreview::between(1, 2, &existing, &updated);

        assert!(preview.document_types.is_empty());
        assert!(preview.tokens.is_empty());
        assert!(preview.groups.is_empty());
        assert!(preview.keywords_changed);
        assert!(preview.description_changed);
        assert!(preview.config_changed);
        assert!(preview.has_changes());
    }

    #[test]
    fn should_report_no_changes_when_every_section_is_sent_as_it_is() {
        let existing = base_contract();
        let updated = merge_contract_update_payload(
            existing.clone(),
            2,
            json!({ "note": { "type": "object" } }),
            Some(json!({ "0": { "conventions": {} } })),
            None,
            Some(json!(["alpha", "beta"])),
            Some("original description"),
            Some(json!({ "canBeDeleted": false })),
        )
        .expect("merge");

        let preview = DataContractUpdatePreview::between(1, 2, &existing, &updated);

        assert_eq!(preview.new_version, 2);
        assert!(!preview.has_changes(), "{preview:?}");
    }

    /// A contract with one document type, built as the update builds one,
    /// from serialization-format JSON.
    fn build_contract(format: serde_json::Value) -> DataContract {
        let format: DataContractInSerializationFormat =
            serde_json::from_str(&format.to_string()).expect("format deserializes");
        let mut errors = vec![];
        DataContract::try_from_platform_versioned(
            format,
            true,
            &mut errors,
            PlatformVersion::latest(),
        )
        .expect("contract builds")
    }

    /// The preview compares the built contracts, so it has to read a section
    /// the update did not touch as unchanged after the merged payload went
    /// through the contract parser and back.
    #[test]
    fn should_report_only_the_supplied_document_type_after_the_contract_is_rebuilt() {
        let platform_version = PlatformVersion::latest();
        let note = json!({
            "type": "object",
            "properties": {
                "message": { "type": "string", "position": 0, "maxLength": 63 }
            },
            "additionalProperties": false
        });
        let id = bs58::encode(Identifier::default().to_buffer()).into_string();
        let existing = build_contract(json!({
            "$formatVersion": "1",
            "id": id,
            "ownerId": id,
            "version": INITIAL_DATA_CONTRACT_VERSION,
            "documentSchemas": { "note": note },
            "keywords": ["alpha"],
            "description": "original description",
            "config": { "$formatVersion": latest_config_version().to_string() }
        }));
        let existing_format =
            contract_format_json(&existing, "existing", platform_version).expect("format");

        let rebuild = |documents: serde_json::Value| {
            let merged = merge_contract_update_payload(
                existing_format.clone(),
                existing.version() + 1,
                documents,
                None,
                None,
                None,
                None,
                None,
            )
            .expect("merge");
            let updated = build_contract(merged);
            let updated_format =
                contract_format_json(&updated, "updated", platform_version).expect("format");
            DataContractUpdatePreview::between(
                existing.version(),
                updated.version(),
                &existing_format,
                &updated_format,
            )
        };

        let untouched = rebuild(json!({}));
        assert!(!untouched.has_changes(), "{untouched:?}");
        assert_eq!(untouched.new_version, untouched.current_version + 1);

        let comment = json!({
            "type": "object",
            "properties": {
                "text": { "type": "string", "position": 0, "maxLength": 63 }
            },
            "additionalProperties": false
        });
        let added = rebuild(json!({ "comment": comment }));
        assert_eq!(added.document_types.added, vec!["comment"]);
        assert!(added.document_types.changed.is_empty(), "{added:?}");
        assert!(!added.keywords_changed && !added.description_changed && !added.config_changed);
    }

    #[test]
    fn should_merge_token_and_group_positions_in_canonical_form() {
        let merged = merge_contract_update_payload(
            base_contract(),
            2,
            json!({}),
            Some(json!({ "01": { "conventions": {} } })),
            Some(json!({ " 2 ": { "requiredPower": 1 } })),
            None,
            None,
            None,
        )
        .expect("merge");

        let tokens = merged["tokens"].as_object().unwrap();
        assert!(tokens.contains_key("0") && tokens.contains_key("1"));
        assert!(!tokens.contains_key("01"));
        assert!(merged["groups"].as_object().unwrap().contains_key("2"));
    }

    #[test]
    fn should_refuse_two_keys_naming_one_position() {
        for (tokens, groups) in [
            (Some(json!({ "0": {}, "00": {} })), None),
            (
                None,
                Some(json!({ "3": { "requiredPower": 1 }, "03": { "requiredPower": 2 } })),
            ),
        ] {
            let err = merge_contract_update_payload(
                base_contract(),
                2,
                json!({}),
                tokens,
                groups,
                None,
                None,
                None,
            )
            .expect_err("a repeated position must be refused");
            assert!(
                err.to_string().contains("more than once"),
                "unexpected error: {err}"
            );
        }
    }

    #[test]
    fn should_refuse_a_key_that_is_not_a_position() {
        for key in ["group", "-1", "65536"] {
            let err = merge_contract_update_payload(
                base_contract(),
                2,
                json!({}),
                None,
                Some(json!({ key: { "requiredPower": 1 } })),
                None,
                None,
                None,
            )
            .expect_err("a key that is not a position must be refused");
            assert!(
                err.to_string().contains("is not a contract position"),
                "unexpected error for {key}: {err}"
            );
        }
    }

    /// A registered contract owned by `[1; 32]` with one document type,
    /// `note`.
    fn registered_contract() -> DataContract {
        use dpp::data_contract::factory::DataContractFactory;
        use dpp::platform_value::platform_value;
        DataContractFactory::new(PlatformVersion::latest().protocol_version)
            .unwrap()
            .create_with_value_config(
                Identifier::from([1; 32]),
                1,
                platform_value!({
                    "note": { "type": "object",
                    "properties": {"message": {"type": "string", "maxLength": 63, "position": 0}},
                    "additionalProperties": false }
                }),
                None,
                None,
            )
            .unwrap()
            .data_contract_owned()
    }

    /// A wallet over a mock SDK that answers only the fetch of `contract`:
    /// any other request (a nonce read, a broadcast) fails the call.
    async fn wallet_answering_only_the_fetch_of(
        contract: &DataContract,
    ) -> std::sync::Arc<crate::PlatformWallet> {
        use super::super::signing_key::tests::wallet_with_signing_keys_on;
        use crate::wallet::persister::NoPlatformPersistence;
        let mut sdk = dash_sdk::SdkBuilder::new_mock()
            .with_version(PlatformVersion::latest())
            .build()
            .unwrap();
        sdk.mock()
            .expect_fetch(contract.id(), Some(contract.clone()))
            .await
            .unwrap();
        wallet_with_signing_keys_on(sdk, std::sync::Arc::new(NoPlatformPersistence)).await
    }

    #[tokio::test]
    async fn should_preview_the_update_of_the_fetched_contract() {
        let contract = registered_contract();
        let wallet = wallet_answering_only_the_fetch_of(&contract).await;

        let preview = wallet
            .identity()
            .preview_data_contract_update(
                &Identifier::from([1; 32]),
                &contract.id(),
                r#"{"comment": {"type": "object", "properties": {"text": {"type": "string", "maxLength": 63, "position": 0}}, "additionalProperties": false}}"#,
                None,
                None,
                Some(r#"["notes"]"#),
                None,
                None,
            )
            .await
            .expect("preview");

        assert_eq!(preview.current_version, contract.version());
        assert_eq!(preview.new_version, contract.version() + 1);
        assert_eq!(preview.document_types.added, vec!["comment"]);
        assert!(preview.document_types.changed.is_empty());
        assert!(preview.keywords_changed);
        assert!(!preview.description_changed);
        assert!(preview.has_changes());
    }

    #[tokio::test]
    async fn should_refuse_a_preview_for_an_identity_that_does_not_own_the_contract() {
        let contract = registered_contract();
        let wallet = wallet_answering_only_the_fetch_of(&contract).await;

        let err = wallet
            .identity()
            .preview_data_contract_update(
                &Identifier::from([9; 32]),
                &contract.id(),
                "{}",
                None,
                None,
                None,
                Some("new description"),
                None,
            )
            .await
            .expect_err("a preview by another identity must be refused");

        assert!(
            err.to_string().contains("cannot update with this identity"),
            "unexpected error: {err}"
        );
    }

    /// Through the public update: the stale review is refused before the
    /// draft is built and before any nonce is read. The mock answers only the
    /// contract fetch, so a nonce request or a broadcast would fail the call
    /// with another error.
    #[tokio::test]
    async fn should_refuse_a_stale_reviewed_version_through_the_public_update() {
        use super::super::signing_key::tests::available;
        // Owned by the identity the test wallet holds, whose keys 1 and 2
        // are CRITICAL ECDSA authentication keys.
        let contract = {
            use dpp::data_contract::factory::DataContractFactory;
            use dpp::platform_value::platform_value;
            DataContractFactory::new(PlatformVersion::latest().protocol_version)
                .unwrap()
                .create_with_value_config(
                    Identifier::default(),
                    1,
                    platform_value!({
                        "note": { "type": "object",
                        "properties": {"message": {"type": "string", "maxLength": 63, "position": 0}},
                        "additionalProperties": false }
                    }),
                    None,
                    None,
                )
                .unwrap()
                .data_contract_owned()
        };
        let wallet = wallet_answering_only_the_fetch_of(&contract).await;

        let err = wallet
            .identity()
            .update_data_contract_with_signer(
                &Identifier::default(),
                &contract.id(),
                r#"{"broken": {"type": "not-a-type"}}"#,
                None,
                None,
                None,
                None,
                None,
                Some(contract.version() + 7),
                &available(&[1, 2]),
            )
            .await
            .expect_err("a stale reviewed version must be refused");

        assert!(
            err.to_string().contains("not the reviewed version"),
            "unexpected error: {err}"
        );
    }

    #[tokio::test]
    async fn should_refuse_a_stale_reviewed_version_before_building_the_draft() {
        let contract = registered_contract();
        let wallet = wallet_answering_only_the_fetch_of(&contract).await;

        // The documents section would fail to build; the stale version must
        // be what the caller hears about.
        let err = wallet
            .identity()
            .draft_data_contract_update(
                &Identifier::from([1; 32]),
                &contract.id(),
                r#"{"broken": {"type": "not-a-type"}}"#,
                None,
                None,
                None,
                None,
                None,
                Some(contract.version() + 7),
            )
            .await
            .err()
            .expect("a stale reviewed version must be refused");

        assert!(
            err.to_string().contains("not the reviewed version"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn should_not_count_a_reordering_of_unchanged_indexes_as_a_change() {
        let note = |indices: serde_json::Value| json!({ "note": { "type": "object", "properties": {}, "indices": indices } });
        let by_a = json!({ "name": "byA", "properties": [{ "a": "asc" }] });
        let by_b = json!({ "name": "byB", "properties": [{ "b": "asc" }] });
        let existing = json!({ "documentSchemas": note(json!([by_a.clone(), by_b.clone()])) });

        let reordered = json!({ "documentSchemas": note(json!([by_b.clone(), by_a.clone()])) });
        assert!(section_changes(&existing, &reordered, "documentSchemas").is_empty());

        let changed_index = json!({ "documentSchemas": note(json!([
            by_a,
            { "name": "byB", "properties": [{ "b": "desc" }] }
        ])) });
        assert_eq!(
            section_changes(&existing, &changed_index, "documentSchemas").changed,
            vec!["note"]
        );
    }

    #[test]
    fn should_name_the_configuration_members_that_change() {
        let existing =
            json!({ "config": { "$formatVersion": "2", "readonly": false, "moderation": null } });
        let updated = json!({ "config": {
            "$formatVersion": "2", "readonly": false,
            "moderation": { "moderators": { "$type": "contractOwner" } }
        } });

        let preview = DataContractUpdatePreview::between(1, 2, &existing, &updated);

        assert!(preview.config_changed);
        assert_eq!(preview.config_changed_fields, vec!["moderation"]);
    }

    #[test]
    fn should_list_moderators_added_and_removed() {
        let kept = Identifier::from([1; 32]);
        let revoked = Identifier::from([2; 32]);
        let granted = Identifier::from([3; 32]);
        let existing = BTreeSet::from([kept, revoked]);
        let updated = BTreeSet::from([kept, granted]);

        let (added, removed) = moderator_changes(&existing, &updated);

        assert_eq!(added, vec![bs58::encode(granted.to_buffer()).into_string()]);
        assert_eq!(
            removed,
            vec![bs58::encode(revoked.to_buffer()).into_string()]
        );
        assert_eq!(moderator_changes(&existing, &existing), (vec![], vec![]));
    }

    #[test]
    fn should_count_a_reordering_of_an_objects_members_as_a_change() {
        let existing: serde_json::Value = serde_json::from_str(
            r#"{"documentSchemas":{"note":{"type":"object","properties":{"z":{"type":"string","position":0},"a":{"type":"string","position":1}}}}}"#,
        )
        .unwrap();
        let reordered: serde_json::Value = serde_json::from_str(
            r#"{"documentSchemas":{"note":{"type":"object","properties":{"a":{"type":"string","position":1},"z":{"type":"string","position":0}}}}}"#,
        )
        .unwrap();

        // `Value` equality ignores member order; the preview must not.
        assert_eq!(existing, reordered);
        assert_eq!(
            section_changes(&existing, &reordered, "documentSchemas").changed,
            vec!["note"]
        );
        assert!(section_changes(&existing, &existing.clone(), "documentSchemas").is_empty());
    }

    /// Through the public preview, against a contract whose configuration
    /// appoints two moderators: the update keeps one and appoints another.
    #[tokio::test]
    async fn should_report_moderator_appointments_through_the_public_preview() {
        let platform_version = PlatformVersion::latest();
        let base58 = |id: Identifier| bs58::encode(id.to_buffer()).into_string();
        let kept = Identifier::from([0x11; 32]);
        let revoked = Identifier::from([0x22; 32]);
        let granted = Identifier::from([0x33; 32]);
        let owner = Identifier::from([1; 32]);
        let config_version = latest_config_version().to_string();
        let contract = build_contract(json!({
            "$formatVersion": "1",
            "id": base58(Identifier::from([9; 32])),
            "ownerId": base58(owner),
            "version": INITIAL_DATA_CONTRACT_VERSION,
            "documentSchemas": { "note": {
                "type": "object",
                "properties": { "message": { "type": "string", "position": 0, "maxLength": 63 } },
                "additionalProperties": false
            } },
            "config": {
                "$formatVersion": config_version,
                "moderation": { "moderators": {
                    "$type": "appointedModerators",
                    "identities": [base58(kept), base58(revoked)]
                } }
            }
        }));
        assert_eq!(
            named_moderators(&contract),
            BTreeSet::from([kept, revoked]),
            "fixture must appoint two moderators at {platform_version:?}"
        );
        let wallet = wallet_answering_only_the_fetch_of(&contract).await;

        let config = json!({
            "$formatVersion": config_version,
            "moderation": { "moderators": {
                "$type": "appointedModerators",
                "identities": [base58(kept), base58(granted)]
            } }
        })
        .to_string();
        let preview = wallet
            .identity()
            .preview_data_contract_update(
                &owner,
                &contract.id(),
                "{}",
                None,
                None,
                None,
                None,
                Some(&config),
            )
            .await
            .expect("preview");

        assert_eq!(preview.moderators_added, vec![base58(granted)]);
        assert_eq!(preview.moderators_removed, vec![base58(revoked)]);
        assert!(preview.config_changed);
        assert_eq!(preview.config_changed_fields, vec!["moderation"]);
    }

    /// The current default config version for the latest protocol — 1 on
    /// v12. Centralized so every assertion below tracks the real protocol
    /// value rather than a hardcoded literal.
    fn latest_config_version() -> u16 {
        PlatformVersion::latest()
            .dpp
            .contract_versions
            .config
            .default_current_version
    }

    #[test]
    fn build_config_object_none_emits_protocol_version_tag() {
        let version = latest_config_version();
        let obj = build_config_object(None, version).expect("None config builds");
        assert_eq!(
            obj.get("$formatVersion"),
            Some(&serde_json::Value::String(version.to_string())),
            "absent config must still carry the protocol-required version tag"
        );
        // Nothing else should have been invented.
        assert_eq!(obj.len(), 1);
    }

    #[test]
    fn build_config_object_empty_string_behaves_like_none() {
        let version = latest_config_version();
        let obj = build_config_object(Some(""), version).expect("empty config builds");
        assert_eq!(
            obj.get("$formatVersion"),
            Some(&serde_json::Value::String(version.to_string()))
        );
        assert_eq!(obj.len(), 1);
    }

    #[test]
    fn build_config_object_flags_object_preserves_caller_fields() {
        let version = latest_config_version();
        let obj = build_config_object(Some(r#"{"canBeDeleted":true}"#), version)
            .expect("flags-only config builds");
        // Caller field preserved verbatim ...
        assert_eq!(
            obj.get("canBeDeleted"),
            Some(&serde_json::Value::Bool(true)),
            "caller-supplied config flags must be preserved"
        );
        // ... alongside the protocol-required version tag.
        assert_eq!(
            obj.get("$formatVersion"),
            Some(&serde_json::Value::String(version.to_string()))
        );
    }

    #[test]
    fn build_config_object_overwrites_caller_supplied_format_version() {
        // The unconditional overwrite is deliberate (see helper docs):
        // a caller-set wire-level tag is replaced with the protocol's
        // required version so the emitted config is always acceptable.
        let version = latest_config_version();
        let obj = build_config_object(Some(r#"{"$formatVersion":"0"}"#), version)
            .expect("config with caller tag builds");
        assert_eq!(
            obj.get("$formatVersion"),
            Some(&serde_json::Value::String(version.to_string())),
            "caller-supplied $formatVersion must be overwritten with the protocol version"
        );
    }

    #[test]
    fn build_config_object_rejects_non_object_json() {
        let version = latest_config_version();
        for bad in [r#"[]"#, r#""x""#, r#"5"#] {
            let err = build_config_object(Some(bad), version)
                .expect_err("non-object config_json must be rejected");
            assert!(
                matches!(err, PlatformWalletError::InvalidIdentityData(_)),
                "expected InvalidIdentityData for {bad:?}, got {err:?}"
            );
        }
    }

    /// Round-trip: the bytes the helper feeds into the contract format
    /// must actually validate as a `DataContract` at the latest protocol
    /// version — this is the assertion that would have caught the original
    /// hardcoded-"0" v12 rejection at unit-test time.
    ///
    /// A document type is included because `full_validation` rejects a
    /// contract carrying neither document schemas nor tokens
    /// (`DocumentTypesAreMissingError`); the config block is still the
    /// subject under test — its version tag is what makes (or breaks)
    /// validation at v12.
    #[test]
    fn build_config_object_output_validates_at_latest_version() {
        let platform_version = PlatformVersion::latest();
        let version = latest_config_version();

        let config_obj = build_config_object(Some(r#"{"canBeDeleted":true}"#), version)
            .expect("flags-only config builds");

        // Minimal serialization-format payload: one trivial document type
        // plus the assembled config block under test.
        let mut document_schemas = serde_json::Map::new();
        document_schemas.insert(
            "note".to_string(),
            serde_json::json!({
                "type": "object",
                "properties": {
                    "message": {"type": "string", "position": 0, "maxLength": 63}
                },
                "additionalProperties": false
            }),
        );

        let mut format_value = serde_json::Map::new();
        format_value.insert(
            "$formatVersion".to_string(),
            serde_json::Value::String("1".to_string()),
        );
        format_value.insert(
            "id".to_string(),
            serde_json::Value::String(
                bs58::encode(Identifier::default().to_buffer()).into_string(),
            ),
        );
        format_value.insert(
            "ownerId".to_string(),
            serde_json::Value::String(
                bs58::encode(Identifier::default().to_buffer()).into_string(),
            ),
        );
        format_value.insert(
            "version".to_string(),
            serde_json::Value::Number(serde_json::Number::from(INITIAL_DATA_CONTRACT_VERSION)),
        );
        format_value.insert(
            "documentSchemas".to_string(),
            serde_json::Value::Object(document_schemas),
        );
        format_value.insert("config".to_string(), serde_json::Value::Object(config_obj));

        let serialized = serde_json::to_string(&serde_json::Value::Object(format_value))
            .expect("format value serializes");
        let format: DataContractInSerializationFormat =
            serde_json::from_str(&serialized).expect("format deserializes");

        let mut errors = vec![];
        let contract =
            DataContract::try_from_platform_versioned(format, true, &mut errors, platform_version);
        assert!(
            contract.is_ok(),
            "assembled config must validate at the latest version: {contract:?} (errors: {errors:?})"
        );
    }
}
