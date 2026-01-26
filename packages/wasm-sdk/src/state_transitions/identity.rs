//! Identity state transition implementations for the WASM SDK.
//!
//! This module provides WASM bindings for identity operations like creation,
//! top-up, credit transfer, withdrawal, and updates.

use crate::error::WasmSdkError;
use crate::queries::utils::deserialize_required_query;
use crate::sdk::WasmSdk;
use crate::settings::PutSettingsInput;
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dash_sdk::dpp::identity::signer::Signer;
use dash_sdk::dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dash_sdk::dpp::platform_value::Identifier;
use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;
use dash_sdk::platform::transition::put_identity::PutIdentity;
use dash_sdk::platform::transition::top_up_identity::TopUpIdentity;
use js_sys::BigInt;
use serde::Deserialize;
use wasm_bindgen::prelude::*;
use wasm_dpp2::asset_lock_proof::AssetLockProofWasm;
use wasm_dpp2::identifier::IdentifierWasm;
use wasm_dpp2::identity::IdentityPublicKeyWasm;
use wasm_dpp2::utils::{
    try_from_options_optional, try_from_options_optional_with, try_from_options_with, try_to_array,
    try_to_u64, IntoWasm,
};
use wasm_dpp2::PrivateKeyWasm;
use wasm_dpp2::{IdentityPublicKeyInCreationWasm, IdentitySignerWasm, IdentityWasm};

// ============================================================================
// Identity Create
// ============================================================================

/// TypeScript interface for identity create options
#[wasm_bindgen(typescript_custom_section)]
const IDENTITY_CREATE_OPTIONS_TS: &'static str = r#"
/**
 * Options for creating a new identity on Dash Platform.
 */
export interface IdentityCreateOptions {
  /**
   * The identity to create (with public keys set up).
   * Use Identity.create() to build the identity structure first.
   */
  identity: Identity;

  /**
   * Asset lock proof from the Core chain.
   * Use AssetLockProof.createInstantAssetLockProof() or AssetLockProof.createChainAssetLockProof().
   */
  assetLockProof: AssetLockProof;

  /**
   * Private key for signing the asset lock proof.
   * This is the private key that controls the asset lock output.
   */
  assetLockPrivateKey: PrivateKey;

  /**
   * Signer containing private keys for the identity's public keys.
   * Use IdentitySigner to add keys for signing identity key proofs.
   */
  signer: IdentitySigner;

  /**
   * Optional settings for the broadcast operation.
   * Includes retries, timeouts, userFeeIncrease, etc.
   */
  settings?: PutSettings;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IdentityCreateOptions")]
    pub type IdentityCreateOptionsJs;
}

#[wasm_bindgen]
impl WasmSdk {
    /// Create a new identity on Dash Platform.
    ///
    /// This method handles the complete identity creation flow:
    /// 1. Validates the asset lock proof
    /// 2. Signs each public key with the corresponding private key
    /// 3. Builds and signs the identity create transition
    /// 4. Broadcasts and waits for confirmation
    ///
    /// @param options - Creation options including identity, asset lock, and signer
    /// @returns Promise that resolves when the identity is created
    #[wasm_bindgen(js_name = "identityCreate")]
    pub async fn identity_create(
        &self,
        options: IdentityCreateOptionsJs,
    ) -> Result<(), WasmSdkError> {
        let options_value: JsValue = options.into();

        // Extract identity from options
        let identity: Identity = IdentityWasm::try_from_options(&options_value, "identity")?.into();

        // Extract asset lock proof from options
        let asset_lock_proof: dash_sdk::dpp::prelude::AssetLockProof =
            AssetLockProofWasm::try_from_options(&options_value, "assetLockProof")?.into();

        // Extract asset lock private key from options
        let asset_lock_private_key: dash_sdk::dpp::dashcore::PrivateKey =
            PrivateKeyWasm::try_from_options(&options_value, "assetLockPrivateKey")?.into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options_value)?;

        // Extract settings from options
        let settings = try_from_options_optional::<PutSettingsInput>(&options_value, "settings")?
            .map(Into::into);

        // Put identity to platform and wait
        identity
            .put_to_platform_and_wait_for_response(
                self.inner_sdk(),
                asset_lock_proof,
                &asset_lock_private_key,
                &signer,
                settings,
            )
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to create identity: {}", e)))?;

        Ok(())
    }
}

// ============================================================================
// Identity TopUp
// ============================================================================

/// TypeScript interface for identity top up options
#[wasm_bindgen(typescript_custom_section)]
const IDENTITY_TOP_UP_OPTIONS_TS: &'static str = r#"
/**
 * Options for topping up an identity with additional credits.
 */
export interface IdentityTopUpOptions {
  /**
   * The identity to top up.
   */
  identity: Identity;

  /**
   * Asset lock proof from the Core chain.
   * Use AssetLockProof.createInstantAssetLockProof() or AssetLockProof.createChainAssetLockProof().
   */
  assetLockProof: AssetLockProof;

  /**
   * Private key for signing the asset lock proof.
   * This is the private key that controls the asset lock output.
   */
  assetLockPrivateKey: PrivateKey;

  /**
   * Optional settings for the broadcast operation.
   * Includes retries, timeouts, userFeeIncrease, etc.
   */
  settings?: PutSettings;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IdentityTopUpOptions")]
    pub type IdentityTopUpOptionsJs;
}

#[wasm_bindgen]
impl WasmSdk {
    /// Top up an existing identity with additional credits.
    ///
    /// This method handles the complete top up flow:
    /// 1. Validates the asset lock proof
    /// 2. Builds and signs the identity top up transition
    /// 3. Broadcasts and waits for confirmation
    ///
    /// @param options - Top up options including identity, asset lock, and private key
    /// @returns Promise resolving to the new balance after top up
    #[wasm_bindgen(js_name = "identityTopUp")]
    pub async fn identity_top_up(
        &self,
        options: IdentityTopUpOptionsJs,
    ) -> Result<BigInt, WasmSdkError> {
        let options_value: JsValue = options.into();

        // Extract identity from options
        let identity: Identity = IdentityWasm::try_from_options(&options_value, "identity")?.into();

        // Extract asset lock proof from options
        let asset_lock_proof: dash_sdk::dpp::prelude::AssetLockProof =
            AssetLockProofWasm::try_from_options(&options_value, "assetLockProof")?.into();

        // Extract asset lock private key from options
        let asset_lock_private_key: dash_sdk::dpp::dashcore::PrivateKey =
            PrivateKeyWasm::try_from_options(&options_value, "assetLockPrivateKey")?.into();

        // Extract settings from options
        let settings = try_from_options_optional::<PutSettingsInput>(&options_value, "settings")?
            .map(Into::into);

        // Top up the identity
        let new_balance = identity
            .top_up_identity(
                self.inner_sdk(),
                asset_lock_proof,
                &asset_lock_private_key,
                None,
                settings,
            )
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to top up identity: {}", e)))?;

        Ok(BigInt::from(new_balance))
    }
}

// ============================================================================
// Identity Credit Transfer
// ============================================================================

/// TypeScript interface for identity credit transfer options
#[wasm_bindgen(typescript_custom_section)]
const IDENTITY_CREDIT_TRANSFER_OPTIONS_TS: &'static str = r#"
/**
 * Options for transferring credits from one identity to another.
 */
export interface IdentityCreditTransferOptions {
  /**
   * The sender identity.
   */
  identity: Identity;

  /**
   * The identity ID of the recipient.
   */
  recipientId: IdentifierLike;

  /**
   * The amount of credits to transfer.
   */
  amount: bigint;

  /**
   * Signer containing the private key for the sender's transfer key.
   * Use IdentitySigner to add the transfer key before calling.
   */
  signer: IdentitySigner;

  /**
   * Optional identity public key to use for signing.
   * If not provided, auto-selects an available transfer key.
   */
  signingKey?: IdentityPublicKey;

  /**
   * Optional settings for the broadcast operation.
   * Includes retries, timeouts, userFeeIncrease, etc.
   */
  settings?: PutSettings;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IdentityCreditTransferOptions")]
    pub type IdentityCreditTransferOptionsJs;
}

/// Result of transferring credits between identities.
#[wasm_bindgen(js_name = "IdentityCreditTransferResult")]
pub struct IdentityCreditTransferResultWasm {
    sender_balance: u64,
    recipient_balance: u64,
}

#[wasm_bindgen(js_class = IdentityCreditTransferResult)]
impl IdentityCreditTransferResultWasm {
    /// Balance of the sender identity after the transfer.
    #[wasm_bindgen(getter = "senderBalance")]
    pub fn sender_balance(&self) -> BigInt {
        BigInt::from(self.sender_balance)
    }

    /// Balance of the recipient identity after the transfer.
    #[wasm_bindgen(getter = "recipientBalance")]
    pub fn recipient_balance(&self) -> BigInt {
        BigInt::from(self.recipient_balance)
    }
}

#[wasm_bindgen]
impl WasmSdk {
    /// Transfer credits from one identity to another.
    ///
    /// This method handles the complete transfer flow:
    /// 1. Finds the appropriate transfer key to use for signing (or uses the provided one)
    /// 2. Builds and signs the credit transfer transition
    /// 3. Broadcasts and waits for confirmation
    ///
    /// @param options - Transfer options including identity, recipient, amount, and signer
    /// @returns Promise resolving to IdentityCreditTransferResult with both balances
    #[wasm_bindgen(js_name = "identityCreditTransfer")]
    pub async fn identity_credit_transfer(
        &self,
        options: IdentityCreditTransferOptionsJs,
    ) -> Result<IdentityCreditTransferResultWasm, WasmSdkError> {
        use dash_sdk::platform::transition::transfer::TransferToIdentity;

        let options_value: JsValue = options.into();

        // Extract identity from options
        let identity: Identity = IdentityWasm::try_from_options(&options_value, "identity")?.into();

        // Extract recipient ID from options
        let recipient_id: Identifier =
            IdentifierWasm::try_from_options(&options_value, "recipientId")?.into();

        // Extract amount from options
        let amount: u64 =
            try_from_options_with(&options_value, "amount", |v| try_to_u64(v, "amount"))?;

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options_value)?;

        // Extract optional signing key from options
        let signing_key: Option<IdentityPublicKey> =
            IdentityPublicKeyWasm::try_from_optional_options(&options_value, "signingKey")?
                .map(|k| k.into());

        // Extract settings from options
        let settings = try_from_options_optional::<PutSettingsInput>(&options_value, "settings")?
            .map(Into::into);

        // Transfer credits using rs-sdk method
        let (sender_balance, recipient_balance) = identity
            .transfer_credits(
                self.inner_sdk(),
                recipient_id,
                amount,
                signing_key.as_ref(),
                signer,
                settings,
            )
            .await?;

        Ok(IdentityCreditTransferResultWasm {
            sender_balance,
            recipient_balance,
        })
    }
}

// ============================================================================
// Identity Credit Withdrawal
// ============================================================================

/// TypeScript interface for identity credit withdrawal options
#[wasm_bindgen(typescript_custom_section)]
const IDENTITY_CREDIT_WITHDRAWAL_OPTIONS_TS: &'static str = r#"
/**
 * Options for withdrawing credits from an identity to a Dash address.
 */
export interface IdentityCreditWithdrawalOptions {
  /**
   * The identity to withdraw from.
   */
  identity: Identity;

  /**
   * The amount of credits to withdraw.
   */
  amount: bigint;

  /**
   * Optional Dash address to send the withdrawn credits to.
   */
  toAddress?: string;

  /**
   * Core (L1) fee per byte for the withdrawal transaction.
   * This determines the mining fee for the Core blockchain transaction.
   * @default 1
   */
  coreFeePerByte?: number;

  /**
   * Signer containing the private key for the identity's transfer/owner key.
   * Use IdentitySigner to add the key before calling.
   */
  signer: IdentitySigner;

  /**
   * Optional identity public key to use for signing.
   * If not provided, auto-selects a matching transfer or owner key.
   */
  signingKey?: IdentityPublicKey;

  /**
   * Optional settings for the broadcast operation.
   * Includes retries, timeouts, userFeeIncrease, etc.
   */
  settings?: PutSettings;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IdentityCreditWithdrawalOptions")]
    pub type IdentityCreditWithdrawalOptionsJs;
}

/// Input struct for identity credit withdrawal options (serde-deserializable fields).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdentityCreditWithdrawalOptionsInput {
    amount: u64,
    #[serde(default)]
    to_address: Option<String>,
    #[serde(default)]
    core_fee_per_byte: Option<u32>,
}

fn deserialize_withdrawal_options(
    options: JsValue,
) -> Result<IdentityCreditWithdrawalOptionsInput, WasmSdkError> {
    deserialize_required_query(
        options,
        "Options object is required",
        "identity credit withdrawal options",
    )
}

#[wasm_bindgen]
impl WasmSdk {
    /// Withdraw credits from an identity to a Dash address.
    ///
    /// This method handles the complete withdrawal flow:
    /// 1. Finds the appropriate transfer/owner key to use for signing (or uses the provided one)
    /// 2. Builds and signs the withdrawal transition
    /// 3. Broadcasts and waits for confirmation
    /// 4. The withdrawal may be pooled with others depending on the pooling strategy
    ///
    /// @param options - Withdrawal options including identity, amount, destination, and signer
    /// @returns Promise resolving to the remaining balance after withdrawal
    #[wasm_bindgen(js_name = "identityCreditWithdrawal")]
    pub async fn identity_credit_withdrawal(
        &self,
        options: IdentityCreditWithdrawalOptionsJs,
    ) -> Result<BigInt, WasmSdkError> {
        use dash_sdk::dpp::dashcore::Address;
        use dash_sdk::platform::transition::withdraw_from_identity::WithdrawFromIdentity;
        use std::str::FromStr;

        let options_value: JsValue = options.into();

        // Deserialize simple fields using serde
        let parsed = deserialize_withdrawal_options(options_value.clone())?;

        // Validate amount
        if parsed.amount == 0 {
            return Err(WasmSdkError::invalid_argument(
                "Withdrawal amount must be greater than 0",
            ));
        }

        // Extract identity from options (WASM type)
        let identity: Identity = IdentityWasm::try_from_options(&options_value, "identity")?.into();

        // Parse address if provided
        let address = parsed
            .to_address
            .map(|addr| {
                Address::from_str(&addr)
                    .map(|a| a.assume_checked())
                    .map_err(|e| {
                        WasmSdkError::invalid_argument(format!("Invalid Dash address: {}", e))
                    })
            })
            .transpose()?;

        // Extract signer from options (WASM type)
        let signer = IdentitySignerWasm::try_from_options(&options_value)?;

        // Extract optional signing key from options (WASM type)
        let signing_key: Option<IdentityPublicKey> =
            IdentityPublicKeyWasm::try_from_optional_options(&options_value, "signingKey")?
                .map(|k| k.into());

        // Extract settings from options
        let settings = try_from_options_optional::<PutSettingsInput>(&options_value, "settings")?
            .map(Into::into);

        // Perform the withdrawal
        let remaining_balance = identity
            .withdraw(
                self.inner_sdk(),
                address,
                parsed.amount,
                parsed.core_fee_per_byte,
                signing_key.as_ref(),
                signer,
                settings,
            )
            .await
            .map_err(|e| WasmSdkError::generic(format!("Withdrawal failed: {}", e)))?;

        Ok(BigInt::from(remaining_balance))
    }
}

// ============================================================================
// Identity Update
// ============================================================================

/// TypeScript interface for identity update options
#[wasm_bindgen(typescript_custom_section)]
const IDENTITY_UPDATE_OPTIONS_TS: &'static str = r#"
/**
 * Options for updating an identity (adding or disabling public keys).
 */
export interface IdentityUpdateOptions {
  /**
   * The identity to update.
   */
  identity: Identity;

  /**
   * Array of public keys to add to the identity.
   * Use IdentityPublicKeyInCreation to create new keys.
   */
  addPublicKeys?: IdentityPublicKeyInCreation[];

  /**
   * Array of key IDs to disable.
   * Cannot disable master, critical auth, or transfer keys.
   */
  disablePublicKeys?: number[];

  /**
   * Signer containing the private key for the identity's master key.
   * Use IdentitySigner to add the master key before calling.
   */
  signer: IdentitySigner;

  /**
   * Optional settings for the broadcast operation.
   * Includes retries, timeouts, userFeeIncrease, etc.
   */
  settings?: PutSettings;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IdentityUpdateOptions")]
    pub type IdentityUpdateOptionsJs;
}

/// Main input struct for identity update options.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdentityUpdateOptionsInput {
    #[serde(default)]
    disable_public_keys: Option<Vec<u32>>,
}

fn deserialize_identity_update_options(
    options: JsValue,
) -> Result<IdentityUpdateOptionsInput, WasmSdkError> {
    deserialize_required_query(
        options,
        "Options object is required",
        "identity update options",
    )
}

#[wasm_bindgen]
impl WasmSdk {
    /// Update an identity by adding or disabling public keys.
    ///
    /// This method handles the complete update flow:
    /// 1. Validates the master key for signing
    /// 2. Validates keys to add/disable
    /// 3. Builds and signs the identity update transition
    /// 4. Broadcasts and waits for confirmation
    ///
    /// @param options - Update options including identity, keys to add/disable, and signer
    /// @returns Promise that resolves when the update is complete
    #[wasm_bindgen(js_name = "identityUpdate")]
    pub async fn identity_update(
        &self,
        options: IdentityUpdateOptionsJs,
    ) -> Result<(), WasmSdkError> {
        let options_value: JsValue = options.into();

        // Deserialize and validate options
        let parsed = deserialize_identity_update_options(options_value.clone())?;

        // Extract identity from options (WASM type)
        let mut identity: Identity =
            IdentityWasm::try_from_options(&options_value, "identity")?.into();

        // Increment the identity revision for the update transition
        // The platform expects the new revision (current + 1) in the state transition
        use dash_sdk::dpp::identity::accessors::IdentitySettersV0;
        let original_revision = identity.revision();
        identity.set_revision(original_revision + 1);

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options_value)?;

        // Find all valid master keys (AUTHENTICATION + MASTER + supported key type)
        let master_keys: Vec<_> = identity
            .public_keys()
            .iter()
            .filter(|(_, key)| {
                key.purpose() == Purpose::AUTHENTICATION
                    && key.security_level() == SecurityLevel::MASTER
                    && (key.key_type() == KeyType::ECDSA_HASH160
                        || key.key_type() == KeyType::ECDSA_SECP256K1)
            })
            .collect();

        // Check if identity has any master keys
        if master_keys.is_empty() {
            return Err(WasmSdkError::invalid_argument(
                "Identity does not have any master key with supported key type (ECDSA_HASH160 or ECDSA_SECP256K1)",
            ));
        }

        // Find a master key that the signer can sign with
        let master_key_id = master_keys
            .iter()
            .find(|(_, key)| signer.can_sign_with(key))
            .map(|(id, _)| **id)
            .ok_or_else(|| {
                WasmSdkError::invalid_argument(
                    "Signer does not have a private key for any of the identity's master keys",
                )
            })?;

        // Parse keys to add from options
        let keys_to_add: Vec<IdentityPublicKey> = if let Some(keys_array) =
            try_from_options_optional_with(&options_value, "addPublicKeys", |v| {
                try_to_array(v, "addPublicKeys")
            })? {
            let max_existing_key_id = identity.public_keys().keys().max().copied().unwrap_or(0);
            let mut next_key_id = max_existing_key_id.checked_add(1).ok_or_else(|| {
                WasmSdkError::invalid_argument("Key ID overflow: identity has too many keys")
            })?;

            keys_array
                .iter()
                .map(|key_js| {
                    let mut key_in_creation = key_js
                        .to_wasm::<IdentityPublicKeyInCreationWasm>("IdentityPublicKeyInCreation")?
                        .clone();

                    // Set the key ID to the next available ID
                    key_in_creation.set_key_id(next_key_id.into())?;

                    // Convert to IdentityPublicKey using From impl
                    let public_key: IdentityPublicKey = key_in_creation.into();
                    next_key_id = next_key_id.checked_add(1).ok_or_else(|| {
                        WasmSdkError::invalid_argument("Key ID overflow: too many keys to add")
                    })?;
                    Ok(public_key)
                })
                .collect::<Result<Vec<_>, WasmSdkError>>()?
        } else {
            Vec::new()
        };

        // Get keys to disable
        let keys_to_disable = parsed.disable_public_keys.unwrap_or_default();

        // Extract settings from options
        let settings = try_from_options_optional::<PutSettingsInput>(&options_value, "settings")?
            .map(Into::into);

        // Get identity nonce
        let identity_nonce = self
            .inner_sdk()
            .get_identity_nonce(identity.id(), true, settings)
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to get identity nonce: {}", e)))?;

        // Create the identity update transition
        use crate::settings::get_user_fee_increase;
        use dash_sdk::dpp::state_transition::identity_update_transition::methods::IdentityUpdateTransitionMethodsV0;
        use dash_sdk::dpp::state_transition::identity_update_transition::IdentityUpdateTransition;

        let state_transition = IdentityUpdateTransition::try_from_identity_with_signer(
            &identity,
            &master_key_id,
            keys_to_add,
            keys_to_disable,
            identity_nonce,
            get_user_fee_increase(settings.as_ref()),
            &signer,
            self.inner_sdk().version(),
            None,
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to create update transition: {}", e)))?;

        // Broadcast the transition
        use dash_sdk::dpp::state_transition::proof_result::StateTransitionProofResult;
        state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self.inner_sdk(), settings)
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to broadcast update: {}", e)))?;

        Ok(())
    }
}

// ============================================================================
// Masternode Vote
// ============================================================================

/// TypeScript interface for masternode vote options
#[wasm_bindgen(typescript_custom_section)]
const MASTERNODE_VOTE_OPTIONS_TS: &'static str = r#"
/**
 * Options for submitting a masternode vote for a contested resource.
 */
export interface MasternodeVoteOptions {
  /**
   * The ProTxHash of the masternode.
   */
  masternodeProTxHash: Identifier;

  /**
   * The vote poll to vote on.
   * Use VotePoll.createContestedDocumentResourceVotePoll() to create.
   */
  votePoll: VotePoll;

  /**
   * The vote choice.
   * Use ResourceVoteChoice.towardsIdentity(), ResourceVoteChoice.abstain(), or ResourceVoteChoice.lock().
   */
  voteChoice: ResourceVoteChoice;

  /**
   * The masternode's voting public key.
   * This should be the voting key associated with the masternode.
   */
  votingKey: IdentityPublicKey;

  /**
   * Signer containing the private key for the masternode's voting key.
   * Use IdentitySigner to add the voting key before calling.
   */
  signer: IdentitySigner;

  /**
   * Optional settings for the broadcast operation.
   * Includes retries, timeouts, userFeeIncrease, etc.
   */
  settings?: PutSettings;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "MasternodeVoteOptions")]
    pub type MasternodeVoteOptionsJs;
}

/// Main input struct for masternode vote options.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MasternodeVoteOptionsInput {
    masternode_pro_tx_hash: IdentifierWasm,
}

fn deserialize_masternode_vote_options(
    options: JsValue,
) -> Result<MasternodeVoteOptionsInput, WasmSdkError> {
    deserialize_required_query(
        options,
        "Options object is required",
        "masternode vote options",
    )
}

#[wasm_bindgen]
impl WasmSdk {
    /// Submit a masternode vote for a contested resource.
    ///
    /// This method handles the complete voting flow:
    /// 1. Creates the voting public key from the signer
    /// 2. Builds and signs the vote transition
    /// 3. Broadcasts and waits for confirmation
    ///
    /// @param options - Vote options including masternode ID, vote poll, choice, and signer
    /// @returns Promise that resolves when the vote is submitted
    #[wasm_bindgen(js_name = "masternodeVote")]
    pub async fn masternode_vote(
        &self,
        options: MasternodeVoteOptionsJs,
    ) -> Result<(), WasmSdkError> {
        use wasm_dpp2::voting::resource_vote_choice::ResourceVoteChoiceWasm;
        use wasm_dpp2::voting::vote_poll::VotePollWasm;

        let options_value: JsValue = options.into();

        // Deserialize and validate options
        let parsed = deserialize_masternode_vote_options(options_value.clone())?;

        // Convert ProTxHash
        let pro_tx_hash: Identifier = parsed.masternode_pro_tx_hash.into();

        // Extract vote poll from options
        let vote_poll: dash_sdk::dpp::voting::vote_polls::VotePoll =
            VotePollWasm::try_from_options(&options_value, "votePoll")?.into();

        // Extract vote choice from options
        let resource_vote_choice: dash_sdk::dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice =
            ResourceVoteChoiceWasm::try_from_options(&options_value, "voteChoice")?.into();

        // Extract voting key from options
        let voting_public_key: IdentityPublicKey =
            IdentityPublicKeyWasm::try_from_options(&options_value, "votingKey")?.into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options_value)?;

        // Create the resource vote
        use dash_sdk::dpp::voting::votes::resource_vote::v0::ResourceVoteV0;
        use dash_sdk::dpp::voting::votes::resource_vote::ResourceVote;
        let resource_vote = ResourceVote::V0(ResourceVoteV0 {
            vote_poll,
            resource_vote_choice,
        });

        // Create the vote
        use dash_sdk::dpp::voting::votes::Vote;
        let vote = Vote::ResourceVote(resource_vote);

        // Extract settings from options
        let settings = try_from_options_optional::<PutSettingsInput>(&options_value, "settings")?
            .map(Into::into);

        // Submit the vote using PutVote trait
        use dash_sdk::platform::transition::vote::PutVote;

        vote.put_to_platform(
            pro_tx_hash,
            &voting_public_key,
            self.inner_sdk(),
            &signer,
            settings,
        )
        .await?;

        Ok(())
    }
}
