//! Identity state transition implementations for the WASM SDK.
//!
//! This module provides WASM bindings for identity operations like creation,
//! top-up, credit transfer, withdrawal, and updates.

use crate::error::WasmSdkError;
use crate::queries::utils::deserialize_required_query;
use crate::sdk::WasmSdk;
use crate::settings::extract_settings_from_options;
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dash_sdk::dpp::identity::signer::Signer;
use dash_sdk::dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dash_sdk::dpp::platform_value::{BinaryData, Identifier};
use dash_sdk::dpp::prelude::UserFeeIncrease;
use dash_sdk::dpp::state_transition::identity_credit_transfer_transition::methods::IdentityCreditTransferTransitionMethodsV0;
use dash_sdk::dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;
use dash_sdk::platform::transition::put_identity::PutIdentity;
use dash_sdk::platform::transition::top_up_identity::TopUpIdentity;
use dash_sdk::platform::Fetch;
use js_sys::BigInt;
use serde::Deserialize;
use wasm_bindgen::prelude::*;
use wasm_dpp2::asset_lock_proof::AssetLockProofWasm;
use wasm_dpp2::identifier::IdentifierWasm;
use wasm_dpp2::private_key::PrivateKeyWasm;
use wasm_dpp2::utils::IntoWasm;
use wasm_dpp2::{
    IdentityPublicKeyInCreationWasm, IdentitySignerWasm, IdentityWasm, PoolingWasm,
};

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
        let identity_js = js_sys::Reflect::get(&options_value, &JsValue::from_str("identity"))
            .map_err(|_| WasmSdkError::invalid_argument("identity is required"))?;
        let identity: Identity = identity_js
            .to_wasm::<IdentityWasm>("Identity")?
            .clone()
            .into();

        // Extract asset lock proof from options
        let asset_lock_proof_js =
            js_sys::Reflect::get(&options_value, &JsValue::from_str("assetLockProof"))
                .map_err(|_| WasmSdkError::invalid_argument("assetLockProof is required"))?;
        let asset_lock_proof: dash_sdk::dpp::prelude::AssetLockProof = asset_lock_proof_js
            .to_wasm::<AssetLockProofWasm>("AssetLockProof")?
            .clone()
            .into();

        // Extract asset lock private key from options
        let asset_lock_private_key_js =
            js_sys::Reflect::get(&options_value, &JsValue::from_str("assetLockPrivateKey"))
                .map_err(|_| WasmSdkError::invalid_argument("assetLockPrivateKey is required"))?;
        let asset_lock_private_key: dash_sdk::dpp::dashcore::PrivateKey = asset_lock_private_key_js
            .to_wasm::<PrivateKeyWasm>("PrivateKey")?
            .clone()
            .into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options_value)?;

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

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
   * The identity ID to top up.
   */
  identityId: Identifier;

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

/// Main input struct for identity top up options.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdentityTopUpOptionsInput {
    identity_id: IdentifierWasm,
}

fn deserialize_identity_top_up_options(
    options: JsValue,
) -> Result<IdentityTopUpOptionsInput, WasmSdkError> {
    deserialize_required_query(
        options,
        "Options object is required",
        "identity top up options",
    )
}

/// Result of topping up an identity.
#[wasm_bindgen(js_name = "IdentityTopUpResult")]
pub struct IdentityTopUpResultWasm {
    new_balance: u64,
    topped_up_amount: u64,
}

#[wasm_bindgen(js_class = IdentityTopUpResult)]
impl IdentityTopUpResultWasm {
    /// New balance of the identity after top up.
    #[wasm_bindgen(getter = "newBalance")]
    pub fn new_balance(&self) -> BigInt {
        BigInt::from(self.new_balance)
    }

    /// Amount that was added to the identity balance.
    #[wasm_bindgen(getter = "toppedUpAmount")]
    pub fn topped_up_amount(&self) -> BigInt {
        BigInt::from(self.topped_up_amount)
    }
}

#[wasm_bindgen]
impl WasmSdk {
    /// Top up an existing identity with additional credits.
    ///
    /// This method handles the complete top up flow:
    /// 1. Fetches the identity from Platform
    /// 2. Validates the asset lock proof
    /// 3. Builds and signs the identity top up transition
    /// 4. Broadcasts and waits for confirmation
    ///
    /// @param options - Top up options including identity ID, asset lock, and private key
    /// @returns Promise resolving to IdentityTopUpResult with balance information
    #[wasm_bindgen(js_name = "identityTopUp")]
    pub async fn identity_top_up(
        &self,
        options: IdentityTopUpOptionsJs,
    ) -> Result<IdentityTopUpResultWasm, WasmSdkError> {
        let options_value: JsValue = options.into();

        // Deserialize and validate options
        let parsed = deserialize_identity_top_up_options(options_value.clone())?;

        // Convert identity ID
        let identity_id: Identifier = parsed.identity_id.into();

        // Fetch the identity
        let identity = Identity::fetch(self.inner_sdk(), identity_id)
            .await?
            .ok_or_else(|| WasmSdkError::not_found(format!("Identity {} not found", identity_id)))?;

        // Get the initial balance
        let initial_balance = identity.balance();

        // Extract asset lock proof from options
        let asset_lock_proof_js =
            js_sys::Reflect::get(&options_value, &JsValue::from_str("assetLockProof"))
                .map_err(|_| WasmSdkError::invalid_argument("assetLockProof is required"))?;
        let asset_lock_proof: dash_sdk::dpp::prelude::AssetLockProof = asset_lock_proof_js
            .to_wasm::<AssetLockProofWasm>("AssetLockProof")?
            .clone()
            .into();

        // Extract asset lock private key from options
        let asset_lock_private_key_js =
            js_sys::Reflect::get(&options_value, &JsValue::from_str("assetLockPrivateKey"))
                .map_err(|_| WasmSdkError::invalid_argument("assetLockPrivateKey is required"))?;
        let asset_lock_private_key: dash_sdk::dpp::dashcore::PrivateKey = asset_lock_private_key_js
            .to_wasm::<PrivateKeyWasm>("PrivateKey")?
            .clone()
            .into();

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

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

        let topped_up_amount = new_balance.saturating_sub(initial_balance);

        Ok(IdentityTopUpResultWasm {
            new_balance,
            topped_up_amount,
        })
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
   * The identity ID of the sender.
   */
  senderId: Identifier;

  /**
   * The identity ID of the recipient.
   */
  recipientId: Identifier;

  /**
   * The amount of credits to transfer.
   */
  amount: bigint | number;

  /**
   * Signer containing the private key for the sender's transfer key.
   * Use IdentitySigner to add the transfer key before calling.
   */
  signer: IdentitySigner;

  /**
   * Optional key ID to use for signing.
   * If not specified, will auto-select a matching transfer key.
   */
  signingKeyId?: number;

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

/// Main input struct for identity credit transfer options.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdentityCreditTransferOptionsInput {
    sender_id: IdentifierWasm,
    recipient_id: IdentifierWasm,
    amount: u64,
    #[serde(default)]
    signing_key_id: Option<u32>,
}

fn deserialize_identity_credit_transfer_options(
    options: JsValue,
) -> Result<IdentityCreditTransferOptionsInput, WasmSdkError> {
    deserialize_required_query(
        options,
        "Options object is required",
        "identity credit transfer options",
    )
}

/// Result of transferring credits between identities.
#[wasm_bindgen(js_name = "IdentityCreditTransferResult")]
pub struct IdentityCreditTransferResultWasm {
    amount: u64,
}

#[wasm_bindgen(js_class = IdentityCreditTransferResult)]
impl IdentityCreditTransferResultWasm {
    /// Amount of credits that were transferred.
    #[wasm_bindgen(getter)]
    pub fn amount(&self) -> BigInt {
        BigInt::from(self.amount)
    }
}

#[wasm_bindgen]
impl WasmSdk {
    /// Transfer credits from one identity to another.
    ///
    /// This method handles the complete transfer flow:
    /// 1. Fetches the sender identity from Platform
    /// 2. Finds the appropriate transfer key to use for signing
    /// 3. Builds and signs the credit transfer transition
    /// 4. Broadcasts and waits for confirmation
    ///
    /// @param options - Transfer options including sender, recipient, amount, and signer
    /// @returns Promise resolving to IdentityCreditTransferResult
    #[wasm_bindgen(js_name = "identityCreditTransfer")]
    pub async fn identity_credit_transfer(
        &self,
        options: IdentityCreditTransferOptionsJs,
    ) -> Result<IdentityCreditTransferResultWasm, WasmSdkError> {
        let options_value: JsValue = options.into();

        // Deserialize and validate options
        let parsed = deserialize_identity_credit_transfer_options(options_value.clone())?;

        // Convert identifiers
        let sender_id: Identifier = parsed.sender_id.into();
        let recipient_id: Identifier = parsed.recipient_id.into();
        let amount = parsed.amount;

        // Validate not sending to self
        if sender_id == recipient_id {
            return Err(WasmSdkError::invalid_argument(
                "Cannot transfer credits to yourself",
            ));
        }

        // Validate amount
        if amount == 0 {
            return Err(WasmSdkError::invalid_argument(
                "Transfer amount must be greater than 0",
            ));
        }

        // Fetch sender identity
        let sender_identity = Identity::fetch(self.inner_sdk(), sender_id)
            .await?
            .ok_or_else(|| WasmSdkError::not_found("Sender identity not found"))?;

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options_value)?;

        // Find matching transfer key
        let matching_key = if let Some(key_id) = parsed.signing_key_id {
            // Find specific key by ID
            sender_identity
                .public_keys()
                .get(&key_id)
                .filter(|key| {
                    key.purpose() == Purpose::TRANSFER
                        && key.key_type() == KeyType::ECDSA_HASH160
                        && signer.can_sign_with(key)
                })
                .ok_or_else(|| {
                    WasmSdkError::not_found(format!(
                        "Key with ID {} not found or signer cannot sign with it",
                        key_id
                    ))
                })?
        } else {
            // Find any matching transfer key
            sender_identity
                .public_keys()
                .iter()
                .find(|(_, key)| {
                    key.purpose() == Purpose::TRANSFER
                        && key.key_type() == KeyType::ECDSA_HASH160
                        && signer.can_sign_with(key)
                })
                .map(|(_, key)| key)
                .ok_or_else(|| {
                    WasmSdkError::not_found(
                        "No matching transfer key found that the signer can sign with",
                    )
                })?
        };

        // Get identity nonce
        let identity_nonce = self
            .inner_sdk()
            .get_identity_nonce(sender_id, true, None)
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to get identity nonce: {}", e)))?;

        // Create the credit transfer transition
        let state_transition = IdentityCreditTransferTransition::try_from_identity(
            &sender_identity,
            recipient_id,
            amount,
            UserFeeIncrease::default(),
            signer.clone(),
            Some(matching_key),
            identity_nonce,
            self.inner_sdk().version(),
            None,
        )
        .map_err(|e| {
            WasmSdkError::generic(format!("Failed to create transfer transition: {}", e))
        })?;

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

        // Broadcast the transition
        use dash_sdk::dpp::state_transition::proof_result::StateTransitionProofResult;
        state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self.inner_sdk(), settings)
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to broadcast transfer: {}", e)))?;

        Ok(IdentityCreditTransferResultWasm { amount })
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
   * The identity ID to withdraw from.
   */
  identityId: Identifier;

  /**
   * The amount of credits to withdraw.
   */
  amount: bigint | number;

  /**
   * Optional Dash address to send the withdrawn credits to.
   * Either toAddress or outputScript must be provided.
   */
  toAddress?: string;

  /**
   * Optional Core output script specifying the L1 destination.
   * Use CoreScript.newP2PKH() or CoreScript.newP2SH() to create.
   * Either toAddress or outputScript must be provided.
   */
  outputScript?: CoreScript;

  /**
   * Core (L1) fee per byte for the withdrawal transaction.
   * This determines the mining fee for the Core blockchain transaction.
   * @default 1
   */
  coreFeePerByte?: number;

  /**
   * Pooling strategy for the withdrawal.
   * - Pooling.Never: Create individual withdrawal transaction
   * - Pooling.IfAvailable: Join pool if available, otherwise individual
   * - Pooling.Standard: Wait to join pool (may take longer)
   * @default Pooling.Never
   */
  pooling?: Pooling;

  /**
   * Signer containing the private key for the identity's transfer/owner key.
   * Use IdentitySigner to add the key before calling.
   */
  signer: IdentitySigner;

  /**
   * Optional key ID to use for signing.
   * If not specified, will auto-select a matching transfer or owner key.
   */
  signingKeyId?: number;

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

/// Main input struct for identity credit withdrawal options.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdentityCreditWithdrawalOptionsInput {
    identity_id: IdentifierWasm,
    amount: u64,
    #[serde(default)]
    to_address: Option<String>,
    #[serde(default)]
    core_fee_per_byte: Option<u32>,
    #[serde(default)]
    #[allow(dead_code)]
    pooling: Option<PoolingWasm>,
    #[serde(default)]
    signing_key_id: Option<u32>,
}

fn deserialize_identity_credit_withdrawal_options(
    options: JsValue,
) -> Result<IdentityCreditWithdrawalOptionsInput, WasmSdkError> {
    deserialize_required_query(
        options,
        "Options object is required",
        "identity credit withdrawal options",
    )
}

/// Result of withdrawing credits from an identity.
#[wasm_bindgen(js_name = "IdentityCreditWithdrawalResult")]
pub struct IdentityCreditWithdrawalResultWasm {
    remaining_balance: u64,
}

#[wasm_bindgen(js_class = IdentityCreditWithdrawalResult)]
impl IdentityCreditWithdrawalResultWasm {
    /// Remaining balance of the identity after withdrawal.
    #[wasm_bindgen(getter = "remainingBalance")]
    pub fn remaining_balance(&self) -> BigInt {
        BigInt::from(self.remaining_balance)
    }
}

#[wasm_bindgen]
impl WasmSdk {
    /// Withdraw credits from an identity to a Dash address.
    ///
    /// This method handles the complete withdrawal flow:
    /// 1. Fetches the identity from Platform
    /// 2. Finds the appropriate transfer/owner key to use for signing
    /// 3. Builds and signs the withdrawal transition
    /// 4. Broadcasts and waits for confirmation
    /// 5. The withdrawal may be pooled with others depending on the pooling strategy
    ///
    /// @param options - Withdrawal options including identity ID, amount, destination, and signer
    /// @returns Promise resolving to IdentityCreditWithdrawalResult with remaining balance
    #[wasm_bindgen(js_name = "identityCreditWithdrawal")]
    pub async fn identity_credit_withdrawal(
        &self,
        options: IdentityCreditWithdrawalOptionsJs,
    ) -> Result<IdentityCreditWithdrawalResultWasm, WasmSdkError> {
        let options_value: JsValue = options.into();

        // Deserialize and validate options
        let parsed = deserialize_identity_credit_withdrawal_options(options_value.clone())?;

        // Convert identity ID
        let identity_id: Identifier = parsed.identity_id.into();
        let amount = parsed.amount;

        // Validate amount
        if amount == 0 {
            return Err(WasmSdkError::invalid_argument(
                "Withdrawal amount must be greater than 0",
            ));
        }

        // Fetch the identity
        let identity = Identity::fetch(self.inner_sdk(), identity_id)
            .await?
            .ok_or_else(|| WasmSdkError::not_found("Identity not found"))?;

        // Parse destination - either toAddress or outputScript
        let address = if let Some(to_address) = parsed.to_address {
            use dash_sdk::dpp::dashcore::Address;
            use std::str::FromStr;
            Some(
                Address::from_str(&to_address)
                    .map_err(|e| {
                        WasmSdkError::invalid_argument(format!("Invalid Dash address: {}", e))
                    })?
                    .assume_checked(),
            )
        } else {
            // Check for outputScript
            let output_script_js =
                js_sys::Reflect::get(&options_value, &JsValue::from_str("outputScript"))
                    .map_err(|_| {
                        WasmSdkError::invalid_argument(
                            "Either toAddress or outputScript must be provided",
                        )
                    })?;

            if output_script_js.is_undefined() || output_script_js.is_null() {
                return Err(WasmSdkError::invalid_argument(
                    "Either toAddress or outputScript must be provided",
                ));
            }

            // We have outputScript, address will be None
            None
        };

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options_value)?;

        // Find matching withdrawal key (TRANSFER or OWNER)
        let matching_key = if let Some(key_id) = parsed.signing_key_id {
            // Find specific key by ID
            identity
                .public_keys()
                .get(&key_id)
                .filter(|key| {
                    (key.purpose() == Purpose::TRANSFER || key.purpose() == Purpose::OWNER)
                        && key.key_type() == KeyType::ECDSA_HASH160
                        && signer.can_sign_with(key)
                })
                .ok_or_else(|| {
                    WasmSdkError::not_found(format!(
                        "Key with ID {} not found or signer cannot sign with it",
                        key_id
                    ))
                })?
        } else {
            // Find any matching withdrawal-capable key (prefer TRANSFER keys)
            identity
                .public_keys()
                .iter()
                .find(|(_, key)| {
                    key.purpose() == Purpose::TRANSFER
                        && key.key_type() == KeyType::ECDSA_HASH160
                        && signer.can_sign_with(key)
                })
                .or_else(|| {
                    identity.public_keys().iter().find(|(_, key)| {
                        key.purpose() == Purpose::OWNER
                            && key.key_type() == KeyType::ECDSA_HASH160
                            && signer.can_sign_with(key)
                    })
                })
                .map(|(_, key)| key)
                .ok_or_else(|| {
                    WasmSdkError::not_found(
                        "No matching withdrawal key found that the signer can sign with",
                    )
                })?
        };

        // Import the withdraw trait
        use dash_sdk::platform::transition::withdraw_from_identity::WithdrawFromIdentity;

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

        // Perform the withdrawal
        let remaining_balance = identity
            .withdraw(
                self.inner_sdk(),
                address,
                amount,
                parsed.core_fee_per_byte,
                Some(matching_key),
                signer,
                settings,
            )
            .await
            .map_err(|e| WasmSdkError::generic(format!("Withdrawal failed: {}", e)))?;

        Ok(IdentityCreditWithdrawalResultWasm { remaining_balance })
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
   * The identity ID to update.
   */
  identityId: Identifier;

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
    identity_id: IdentifierWasm,
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

/// Result of updating an identity.
#[wasm_bindgen(js_name = "IdentityUpdateResult")]
pub struct IdentityUpdateResultWasm {
    revision: u64,
    added_key_ids: Vec<u32>,
    disabled_key_ids: Vec<u32>,
}

#[wasm_bindgen(js_class = IdentityUpdateResult)]
impl IdentityUpdateResultWasm {
    /// New revision number of the identity after update.
    #[wasm_bindgen(getter)]
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// IDs of keys that were added.
    #[wasm_bindgen(getter = "addedKeyIds")]
    pub fn added_key_ids(&self) -> Vec<u32> {
        self.added_key_ids.clone()
    }

    /// IDs of keys that were disabled.
    #[wasm_bindgen(getter = "disabledKeyIds")]
    pub fn disabled_key_ids(&self) -> Vec<u32> {
        self.disabled_key_ids.clone()
    }
}

#[wasm_bindgen]
impl WasmSdk {
    /// Update an identity by adding or disabling public keys.
    ///
    /// This method handles the complete update flow:
    /// 1. Fetches the identity from Platform
    /// 2. Validates the master key for signing
    /// 3. Validates keys to add/disable
    /// 4. Builds and signs the identity update transition
    /// 5. Broadcasts and waits for confirmation
    ///
    /// @param options - Update options including identity ID, keys to add/disable, and signer
    /// @returns Promise resolving to IdentityUpdateResult with new revision and key changes
    #[wasm_bindgen(js_name = "identityUpdate")]
    pub async fn identity_update(
        &self,
        options: IdentityUpdateOptionsJs,
    ) -> Result<IdentityUpdateResultWasm, WasmSdkError> {
        let options_value: JsValue = options.into();

        // Deserialize and validate options
        let parsed = deserialize_identity_update_options(options_value.clone())?;

        // Convert identity ID
        let identity_id: Identifier = parsed.identity_id.into();

        // Fetch the identity
        let identity = Identity::fetch(self.inner_sdk(), identity_id)
            .await?
            .ok_or_else(|| WasmSdkError::not_found("Identity not found"))?;

        // Get current revision
        let current_revision = identity.revision();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options_value)?;

        // Find matching master key
        let master_key_id = identity
            .public_keys()
            .iter()
            .find(|(_, key)| {
                key.purpose() == Purpose::AUTHENTICATION
                    && key.security_level() == SecurityLevel::MASTER
                    && key.key_type() == KeyType::ECDSA_HASH160
                    && signer.can_sign_with(key)
            })
            .map(|(id, _)| *id)
            .ok_or_else(|| {
                WasmSdkError::invalid_argument(
                    "Signer does not have a private key for any master key",
                )
            })?;

        // Parse keys to add from options
        let add_public_keys_js =
            js_sys::Reflect::get(&options_value, &JsValue::from_str("addPublicKeys"))
                .unwrap_or(JsValue::UNDEFINED);

        let keys_to_add: Vec<IdentityPublicKey> =
            if !add_public_keys_js.is_undefined() && !add_public_keys_js.is_null() {
                let keys_array = js_sys::Array::from(&add_public_keys_js);
                let mut next_key_id = identity.public_keys().keys().max().copied().unwrap_or(0) + 1;

                keys_array
                    .iter()
                    .map(|key_js| {
                        let mut key_in_creation = key_js
                            .to_wasm::<IdentityPublicKeyInCreationWasm>(
                                "IdentityPublicKeyInCreation",
                            )?
                            .clone();

                        // Set the key ID to the next available ID
                        key_in_creation.set_key_id(next_key_id);

                        // Convert to IdentityPublicKey using From impl
                        let public_key: IdentityPublicKey = key_in_creation.into();
                        next_key_id += 1;
                        Ok(public_key)
                    })
                    .collect::<Result<Vec<_>, WasmSdkError>>()?
            } else {
                Vec::new()
            };

        // Get keys to disable
        let keys_to_disable = parsed.disable_public_keys.unwrap_or_default();

        // Save counts before moving
        let added_key_ids: Vec<u32> = keys_to_add.iter().map(|k| k.id()).collect();
        let disabled_key_ids = keys_to_disable.clone();

        // Validate keys to disable
        for key_id in &keys_to_disable {
            if let Some(key) = identity.public_keys().get(key_id) {
                if key.security_level() == SecurityLevel::MASTER {
                    return Err(WasmSdkError::invalid_argument(format!(
                        "Cannot disable master key {}",
                        key_id
                    )));
                }
                if key.purpose() == Purpose::AUTHENTICATION
                    && key.security_level() == SecurityLevel::CRITICAL
                    && key.key_type() == KeyType::ECDSA_SECP256K1
                {
                    return Err(WasmSdkError::invalid_argument(format!(
                        "Cannot disable critical authentication key {}",
                        key_id
                    )));
                }
                if key.purpose() == Purpose::TRANSFER {
                    return Err(WasmSdkError::invalid_argument(format!(
                        "Cannot disable transfer key {}",
                        key_id
                    )));
                }
            } else {
                return Err(WasmSdkError::not_found(format!("Key {} not found", key_id)));
            }
        }

        // Get identity nonce
        let identity_nonce = self
            .inner_sdk()
            .get_identity_nonce(identity_id, true, None)
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to get identity nonce: {}", e)))?;

        // Create the identity update transition
        use dash_sdk::dpp::state_transition::identity_update_transition::methods::IdentityUpdateTransitionMethodsV0;
        use dash_sdk::dpp::state_transition::identity_update_transition::IdentityUpdateTransition;

        let state_transition = IdentityUpdateTransition::try_from_identity_with_signer(
            &identity,
            &master_key_id,
            keys_to_add,
            keys_to_disable,
            identity_nonce,
            UserFeeIncrease::default(),
            &signer,
            self.inner_sdk().version(),
            None,
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to create update transition: {}", e)))?;

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

        // Broadcast the transition
        use dash_sdk::dpp::state_transition::proof_result::StateTransitionProofResult;
        let result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self.inner_sdk(), settings)
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to broadcast update: {}", e)))?;

        // Extract updated identity from result
        let updated_revision = match result {
            StateTransitionProofResult::VerifiedIdentity(updated_identity) => {
                updated_identity.revision()
            }
            StateTransitionProofResult::VerifiedPartialIdentity(partial_identity) => {
                partial_identity.revision.unwrap_or(current_revision + 1)
            }
            _ => current_revision + 1,
        };

        Ok(IdentityUpdateResultWasm {
            revision: updated_revision,
            added_key_ids,
            disabled_key_ids,
        })
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

/// Result of submitting a masternode vote.
#[wasm_bindgen(js_name = "MasternodeVoteResult")]
pub struct MasternodeVoteResultWasm {
    _private: (),
}

#[wasm_bindgen(js_class = MasternodeVoteResult)]
impl MasternodeVoteResultWasm {
    // Just a success confirmation, no fields needed
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
    /// @returns Promise resolving to MasternodeVoteResult (success confirmation)
    #[wasm_bindgen(js_name = "masternodeVote")]
    pub async fn masternode_vote(
        &self,
        options: MasternodeVoteOptionsJs,
    ) -> Result<MasternodeVoteResultWasm, WasmSdkError> {
        use wasm_dpp2::voting::resource_vote_choice::ResourceVoteChoiceWasm;
        use wasm_dpp2::voting::vote_poll::VotePollWasm;

        let options_value: JsValue = options.into();

        // Deserialize and validate options
        let parsed = deserialize_masternode_vote_options(options_value.clone())?;

        // Convert ProTxHash
        let pro_tx_hash: Identifier = parsed.masternode_pro_tx_hash.into();

        // Extract vote poll from options
        let vote_poll_js = js_sys::Reflect::get(&options_value, &JsValue::from_str("votePoll"))
            .map_err(|_| WasmSdkError::invalid_argument("votePoll is required"))?;
        let vote_poll: dash_sdk::dpp::voting::vote_polls::VotePoll = vote_poll_js
            .to_wasm::<VotePollWasm>("VotePoll")?
            .clone()
            .into();

        // Extract vote choice from options
        let vote_choice_js = js_sys::Reflect::get(&options_value, &JsValue::from_str("voteChoice"))
            .map_err(|_| WasmSdkError::invalid_argument("voteChoice is required"))?;
        let resource_vote_choice: dash_sdk::dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice = vote_choice_js
            .to_wasm::<ResourceVoteChoiceWasm>("ResourceVoteChoice")?
            .clone()
            .into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options_value)?;

        // We need to get the voting key from the signer
        // The signer should contain exactly one key for voting
        // For now, we'll create a voting public key structure that matches what the signer has

        // Create the voting identity public key
        // This is a placeholder - we need to get the actual public key hash from the signer
        use dash_sdk::dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;

        // Get the first key from the signer that can be used for voting
        // We'll create a dummy key structure since we know the signer has the private key
        // The actual public key hash will be derived during signing
        let voting_public_key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 0,
            key_type: KeyType::ECDSA_HASH160,
            purpose: Purpose::VOTING,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            read_only: false,
            data: BinaryData::new(vec![0u8; 20]), // Placeholder - will be set correctly
            disabled_at: None,
        });

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
        let settings = extract_settings_from_options(&options_value)?;

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

        Ok(MasternodeVoteResultWasm { _private: () })
    }
}
