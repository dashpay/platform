//! Token state transition implementations for the WASM SDK.
//!
//! This module provides WASM bindings for token operations like mint, burn, transfer, etc.

use crate::error::WasmSdkError;
use crate::queries::utils::deserialize_required_query;
use crate::sdk::WasmSdk;
use crate::settings::extract_settings_from_options;
use dash_sdk::dpp::balances::credits::TokenAmount;
use dash_sdk::dpp::identity::IdentityPublicKey;
use dash_sdk::dpp::platform_value::Identifier;
use crate::settings::get_user_fee_increase;
use dash_sdk::dpp::state_transition::batch_transition::methods::v1::DocumentsBatchTransitionMethodsV1;
use dash_sdk::dpp::state_transition::batch_transition::BatchTransition;
use dash_sdk::dpp::state_transition::proof_result::StateTransitionProofResult;
use dash_sdk::dpp::tokens::calculate_token_id;
use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;
use dash_sdk::platform::Fetch;
use js_sys::BigInt;
use serde::Deserialize;
use wasm_bindgen::prelude::*;
use wasm_dpp2::identifier::IdentifierWasm;
use wasm_dpp2::identity::IdentityPublicKeyWasm;
use wasm_dpp2::state_transitions::base::GroupStateTransitionInfoStatusWasm;
use wasm_dpp2::IdentitySignerWasm;

// Helper methods for token operations
impl WasmSdk {
    /// Fetch and cache data contract in trusted context
    async fn fetch_and_cache_token_contract(
        &self,
        contract_id: Identifier,
    ) -> Result<dash_sdk::platform::DataContract, WasmSdkError> {
        let sdk = self.inner_sdk();

        // Fetch the data contract
        let data_contract = dash_sdk::platform::DataContract::fetch(sdk, contract_id)
            .await?
            .ok_or_else(|| WasmSdkError::not_found("Data contract not found"))?;

        // Add the contract to the context provider's cache if using trusted mode
        match sdk.network {
            dash_sdk::dpp::dashcore::Network::Testnet => {
                let guard = crate::sdk::TESTNET_TRUSTED_CONTEXT.lock().unwrap();
                let context = guard.as_ref().ok_or_else(|| {
                    WasmSdkError::generic("Testnet trusted context not initialized")
                })?;
                context.add_known_contract(data_contract.clone());
            }
            dash_sdk::dpp::dashcore::Network::Dash => {
                let guard = crate::sdk::MAINNET_TRUSTED_CONTEXT.lock().unwrap();
                let context = guard.as_ref().ok_or_else(|| {
                    WasmSdkError::generic("Mainnet trusted context not initialized")
                })?;
                context.add_known_contract(data_contract.clone());
            }
            dash_sdk::dpp::dashcore::Network::Regtest => {
                let guard = crate::sdk::LOCAL_TRUSTED_CONTEXT.lock().unwrap();
                let context = guard.as_ref().ok_or_else(|| {
                    WasmSdkError::generic("Local trusted context not initialized")
                })?;
                context.add_known_contract(data_contract.clone());
            }
            network => {
                return Err(WasmSdkError::generic(format!(
                    "Unsupported network for trusted context: {:?}",
                    network
                )));
            }
        }

        Ok(data_contract)
    }
}

// ============================================================================
// Token Mint
// ============================================================================

/// TypeScript interface for token mint options
#[wasm_bindgen(typescript_custom_section)]
const TOKEN_MINT_OPTIONS_TS: &'static str = r#"
/**
 * Options for minting new tokens.
 */
export interface TokenMintOptions {
  /**
   * The ID of the data contract containing the token.
   */
  dataContractId: Identifier;

  /**
   * The position of the token in the contract (0-indexed).
   */
  tokenPosition: number;

  /**
   * The amount of tokens to mint.
   */
  amount: bigint | number;

  /**
   * The identity ID of the minter.
   */
  identityId: Identifier;

  /**
   * Optional recipient identity ID.
   * If not provided, mints to the minter's identity.
   */
  recipientId?: Identifier;

  /**
   * Optional public note for the mint operation.
   */
  publicNote?: string;

  /**
   * The identity public key to use for signing the transition.
   * Get this from the minter identity's public keys.
   */
  identityKey: IdentityPublicKey;

  /**
   * Signer containing the private key that corresponds to the identity key.
   * Use IdentitySigner to add the private key before calling.
   */
  signer: IdentitySigner;

  /**
   * Optional group action info for group-managed token minting.
   * Use GroupStateTransitionInfoStatus.proposer() to propose a new group action,
   * or GroupStateTransitionInfoStatus.otherSigner() to vote on an existing action.
   */
  groupInfo?: GroupStateTransitionInfoStatus;

  /**
   * Optional settings for the broadcast operation.
   * Includes retries, timeouts, userFeeIncrease, etc.
   */
  settings?: PutSettings;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "TokenMintOptions")]
    pub type TokenMintOptionsJs;
}

/// Main input struct for token mint options.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenMintOptionsInput {
    data_contract_id: IdentifierWasm,
    token_position: u16,
    amount: u64,
    identity_id: IdentifierWasm,
    #[serde(default)]
    recipient_id: Option<IdentifierWasm>,
    #[serde(default)]
    public_note: Option<String>,
}

fn deserialize_token_mint_options(options: JsValue) -> Result<TokenMintOptionsInput, WasmSdkError> {
    deserialize_required_query(options, "Options object is required", "token mint options")
}

/// Result of minting tokens.
#[wasm_bindgen(js_name = "TokenMintResult")]
pub struct TokenMintResultWasm {
    recipient_id: IdentifierWasm,
    new_balance: u64,
}

#[wasm_bindgen(js_class = TokenMintResult)]
impl TokenMintResultWasm {
    /// The recipient's identity ID.
    #[wasm_bindgen(getter = "recipientId")]
    pub fn recipient_id(&self) -> IdentifierWasm {
        self.recipient_id.clone()
    }

    /// The new token balance after minting.
    #[wasm_bindgen(getter = "newBalance")]
    pub fn new_balance(&self) -> BigInt {
        BigInt::from(self.new_balance)
    }
}

#[wasm_bindgen]
impl WasmSdk {
    /// Mint new tokens according to the token's configuration.
    ///
    /// @param options - Mint options including contract ID, token position, amount, and signer
    /// @returns Promise resolving to TokenMintResult with the new balance
    #[wasm_bindgen(js_name = "tokenMint")]
    pub async fn token_mint(
        &self,
        options: TokenMintOptionsJs,
    ) -> Result<TokenMintResultWasm, WasmSdkError> {
        let options_value: JsValue = options.into();

        // Deserialize and validate options
        let parsed = deserialize_token_mint_options(options_value.clone())?;

        // Convert identifiers
        let contract_id: Identifier = parsed.data_contract_id.into();
        let identity_id: Identifier = parsed.identity_id.into();
        let recipient_id: Option<Identifier> = parsed.recipient_id.map(Into::into);
        let amount = parsed.amount as TokenAmount;

        // Extract identity key from options
        let identity_key_wasm =
            IdentityPublicKeyWasm::try_from_options(&options_value, "identityKey")?;
        let identity_key: IdentityPublicKey = identity_key_wasm.into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options_value)?;

        // Extract optional group info from options
        let group_info = GroupStateTransitionInfoStatusWasm::try_from_optional_options(
            &options_value,
            "groupInfo",
        )?
        .map(Into::into);

        // Fetch and cache the data contract
        let _data_contract = self.fetch_and_cache_token_contract(contract_id).await?;

        // Calculate token ID
        let token_id = Identifier::new(calculate_token_id(contract_id.as_bytes(), parsed.token_position));

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

        // Get identity nonce for the token contract
        let identity_nonce = self
            .inner_sdk()
            .get_identity_contract_nonce(identity_id, contract_id, true, settings)
            .await?;

        // Create the mint transition
        let state_transition = BatchTransition::new_token_mint_transition(
            token_id,
            identity_id,
            contract_id,
            parsed.token_position,
            amount,
            recipient_id,
            parsed.public_note,
            group_info,
            &identity_key,
            identity_nonce,
            get_user_fee_increase(settings.as_ref()),
            &signer,
            self.inner_sdk().version(),
            None, // options
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to create mint transition: {}", e)))?;

        // Broadcast the transition
        let result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self.inner_sdk(), settings)
            .await?;

        // Extract balance from result
        let new_balance = match result {
            StateTransitionProofResult::VerifiedTokenBalance(_, balance) => balance,
            _ => 0,
        };

        // If no recipient was specified, tokens go to the issuer
        let actual_recipient_id = recipient_id.unwrap_or(identity_id);

        Ok(TokenMintResultWasm {
            recipient_id: actual_recipient_id.into(),
            new_balance,
        })
    }
}

// ============================================================================
// Token Burn
// ============================================================================

/// TypeScript interface for token burn options
#[wasm_bindgen(typescript_custom_section)]
const TOKEN_BURN_OPTIONS_TS: &'static str = r#"
/**
 * Options for burning tokens.
 */
export interface TokenBurnOptions {
  /**
   * The ID of the data contract containing the token.
   */
  dataContractId: Identifier;

  /**
   * The position of the token in the contract (0-indexed).
   */
  tokenPosition: number;

  /**
   * The amount of tokens to burn.
   */
  amount: bigint | number;

  /**
   * The identity ID of the token holder burning tokens.
   */
  identityId: Identifier;

  /**
   * Optional public note for the burn operation.
   */
  publicNote?: string;

  /**
   * The identity public key to use for signing the transition.
   */
  identityKey: IdentityPublicKey;

  /**
   * Signer containing the private key that corresponds to the identity key.
   * Use IdentitySigner to add the private key before calling.
   */
  signer: IdentitySigner;

  /**
   * Optional group action info for group-managed token burning.
   * Use GroupStateTransitionInfoStatus.proposer() to propose a new group action,
   * or GroupStateTransitionInfoStatus.otherSigner() to vote on an existing action.
   */
  groupInfo?: GroupStateTransitionInfoStatus;

  /**
   * Optional settings for the broadcast operation.
   * Includes retries, timeouts, userFeeIncrease, etc.
   */
  settings?: PutSettings;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "TokenBurnOptions")]
    pub type TokenBurnOptionsJs;
}

/// Main input struct for token burn options.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenBurnOptionsInput {
    data_contract_id: IdentifierWasm,
    token_position: u16,
    amount: u64,
    identity_id: IdentifierWasm,
    #[serde(default)]
    public_note: Option<String>,
}

fn deserialize_token_burn_options(options: JsValue) -> Result<TokenBurnOptionsInput, WasmSdkError> {
    deserialize_required_query(options, "Options object is required", "token burn options")
}

/// Result of burning tokens.
#[wasm_bindgen(js_name = "TokenBurnResult")]
pub struct TokenBurnResultWasm {
    identity_id: IdentifierWasm,
    new_balance: u64,
}

#[wasm_bindgen(js_class = TokenBurnResult)]
impl TokenBurnResultWasm {
    /// The identity ID that burned tokens.
    #[wasm_bindgen(getter = "identityId")]
    pub fn identity_id(&self) -> IdentifierWasm {
        self.identity_id.clone()
    }

    /// The new token balance after burning.
    #[wasm_bindgen(getter = "newBalance")]
    pub fn new_balance(&self) -> BigInt {
        BigInt::from(self.new_balance)
    }
}

#[wasm_bindgen]
impl WasmSdk {
    /// Burn tokens from an identity's balance.
    ///
    /// @param options - Burn options including contract ID, token position, amount, and signer
    /// @returns Promise resolving to TokenBurnResult with the new balance
    #[wasm_bindgen(js_name = "tokenBurn")]
    pub async fn token_burn(
        &self,
        options: TokenBurnOptionsJs,
    ) -> Result<TokenBurnResultWasm, WasmSdkError> {
        let options_value: JsValue = options.into();

        // Deserialize and validate options
        let parsed = deserialize_token_burn_options(options_value.clone())?;

        // Convert identifiers
        let contract_id: Identifier = parsed.data_contract_id.into();
        let identity_id: Identifier = parsed.identity_id.into();
        let amount = parsed.amount as TokenAmount;

        // Extract identity key from options
        let identity_key_wasm =
            IdentityPublicKeyWasm::try_from_options(&options_value, "identityKey")?;
        let identity_key: IdentityPublicKey = identity_key_wasm.into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options_value)?;

        // Extract optional group info from options
        let group_info = GroupStateTransitionInfoStatusWasm::try_from_optional_options(
            &options_value,
            "groupInfo",
        )?
        .map(Into::into);

        // Fetch and cache the data contract
        let _data_contract = self.fetch_and_cache_token_contract(contract_id).await?;

        // Calculate token ID
        let token_id = Identifier::new(calculate_token_id(contract_id.as_bytes(), parsed.token_position));

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

        // Get identity nonce for the token contract
        let identity_nonce = self
            .inner_sdk()
            .get_identity_contract_nonce(identity_id, contract_id, true, settings)
            .await?;

        // Create the burn transition
        let state_transition = BatchTransition::new_token_burn_transition(
            token_id,
            identity_id,
            contract_id,
            parsed.token_position,
            amount,
            parsed.public_note,
            group_info,
            &identity_key,
            identity_nonce,
            get_user_fee_increase(settings.as_ref()),
            &signer,
            self.inner_sdk().version(),
            None, // options
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to create burn transition: {}", e)))?;

        // Broadcast the transition
        let result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self.inner_sdk(), settings)
            .await?;

        // Extract balance from result
        let new_balance = match result {
            StateTransitionProofResult::VerifiedTokenBalance(_, balance) => balance,
            _ => 0,
        };

        Ok(TokenBurnResultWasm {
            identity_id: identity_id.into(),
            new_balance,
        })
    }
}

// ============================================================================
// Token Transfer
// ============================================================================

/// TypeScript interface for token transfer options
#[wasm_bindgen(typescript_custom_section)]
const TOKEN_TRANSFER_OPTIONS_TS: &'static str = r#"
/**
 * Options for transferring tokens between identities.
 */
export interface TokenTransferOptions {
  /**
   * The ID of the data contract containing the token.
   */
  dataContractId: Identifier;

  /**
   * The position of the token in the contract (0-indexed).
   */
  tokenPosition: number;

  /**
   * The amount of tokens to transfer.
   */
  amount: bigint | number;

  /**
   * The sender's identity ID.
   */
  senderId: Identifier;

  /**
   * The recipient's identity ID.
   */
  recipientId: Identifier;

  /**
   * Optional public note for the transfer.
   */
  publicNote?: string;

  /**
   * Signer containing the private key for the sender's authentication key.
   * Use IdentitySigner to add the authentication key before calling.
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
    #[wasm_bindgen(typescript_type = "TokenTransferOptions")]
    pub type TokenTransferOptionsJs;
}

/// Main input struct for token transfer options.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenTransferOptionsInput {
    data_contract_id: IdentifierWasm,
    token_position: u16,
    amount: u64,
    sender_id: IdentifierWasm,
    recipient_id: IdentifierWasm,
    #[serde(default)]
    public_note: Option<String>,
}

fn deserialize_token_transfer_options(
    options: JsValue,
) -> Result<TokenTransferOptionsInput, WasmSdkError> {
    deserialize_required_query(
        options,
        "Options object is required",
        "token transfer options",
    )
}

/// Result of transferring tokens.
#[wasm_bindgen(js_name = "TokenTransferResult")]
pub struct TokenTransferResultWasm {
    sender_id: IdentifierWasm,
    recipient_id: IdentifierWasm,
    amount: u64,
}

#[wasm_bindgen(js_class = TokenTransferResult)]
impl TokenTransferResultWasm {
    /// The sender's identity ID.
    #[wasm_bindgen(getter = "senderId")]
    pub fn sender_id(&self) -> IdentifierWasm {
        self.sender_id.clone()
    }

    /// The recipient's identity ID.
    #[wasm_bindgen(getter = "recipientId")]
    pub fn recipient_id(&self) -> IdentifierWasm {
        self.recipient_id.clone()
    }

    /// The amount of tokens transferred.
    #[wasm_bindgen(getter)]
    pub fn amount(&self) -> BigInt {
        BigInt::from(self.amount)
    }
}

#[wasm_bindgen]
impl WasmSdk {
    /// Transfer tokens from one identity to another.
    ///
    /// @param options - Transfer options including contract ID, token position, amount, sender, recipient, and signer
    /// @returns Promise resolving to TokenTransferResult with transfer info
    #[wasm_bindgen(js_name = "tokenTransfer")]
    pub async fn token_transfer(
        &self,
        options: TokenTransferOptionsJs,
    ) -> Result<TokenTransferResultWasm, WasmSdkError> {
        let options_value: JsValue = options.into();

        // Deserialize and validate options
        let parsed = deserialize_token_transfer_options(options_value.clone())?;

        // Convert identifiers
        let contract_id: Identifier = parsed.data_contract_id.into();
        let sender_id: Identifier = parsed.sender_id.into();
        let recipient_id: Identifier = parsed.recipient_id.into();
        let amount = parsed.amount as TokenAmount;

        // Validate not transferring to self
        if sender_id == recipient_id {
            return Err(WasmSdkError::invalid_argument(
                "Cannot transfer tokens to yourself",
            ));
        }

        // Extract identity key from options
        let identity_key_wasm =
            IdentityPublicKeyWasm::try_from_options(&options_value, "identityKey")?;
        let identity_key: IdentityPublicKey = identity_key_wasm.into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options_value)?;

        // Fetch and cache the data contract
        let _data_contract = self.fetch_and_cache_token_contract(contract_id).await?;

        // Calculate token ID
        let token_id = Identifier::new(calculate_token_id(contract_id.as_bytes(), parsed.token_position));

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

        // Get identity nonce for the token contract
        let identity_nonce = self
            .inner_sdk()
            .get_identity_contract_nonce(sender_id, contract_id, true, settings)
            .await?;

        // Create the transfer transition
        let state_transition = BatchTransition::new_token_transfer_transition(
            token_id,
            sender_id,
            contract_id,
            parsed.token_position,
            amount,
            recipient_id,
            parsed.public_note,
            None, // shared_encrypted_note
            None, // private_encrypted_note
            &identity_key,
            identity_nonce,
            get_user_fee_increase(settings.as_ref()),
            &signer,
            self.inner_sdk().version(),
            None, // options
        )
        .map_err(|e| {
            WasmSdkError::generic(format!("Failed to create transfer transition: {}", e))
        })?;

        // Broadcast the transition
        state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self.inner_sdk(), settings)
            .await?;

        Ok(TokenTransferResultWasm {
            sender_id: sender_id.into(),
            recipient_id: recipient_id.into(),
            amount: parsed.amount,
        })
    }
}

// ============================================================================
// Token Freeze
// ============================================================================

/// TypeScript interface for token freeze options
#[wasm_bindgen(typescript_custom_section)]
const TOKEN_FREEZE_OPTIONS_TS: &'static str = r#"
/**
 * Options for freezing an identity's token balance.
 */
export interface TokenFreezeOptions {
  /**
   * The ID of the data contract containing the token.
   */
  dataContractId: Identifier;

  /**
   * The position of the token in the contract (0-indexed).
   */
  tokenPosition: number;

  /**
   * The identity ID of the token authority performing the freeze.
   */
  authorityId: Identifier;

  /**
   * The identity ID to freeze.
   */
  frozenIdentityId: Identifier;

  /**
   * Optional public note for the freeze operation.
   */
  publicNote?: string;

  /**
   * Signer containing the private key for the authority's authentication key.
   * Use IdentitySigner to add the authentication key before calling.
   */
  signer: IdentitySigner;

  /**
   * Optional group action info for group-managed token freezing.
   * Use GroupStateTransitionInfoStatus.proposer() to propose a new group action,
   * or GroupStateTransitionInfoStatus.otherSigner() to vote on an existing action.
   */
  groupInfo?: GroupStateTransitionInfoStatus;

  /**
   * Optional settings for the broadcast operation.
   * Includes retries, timeouts, userFeeIncrease, etc.
   */
  settings?: PutSettings;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "TokenFreezeOptions")]
    pub type TokenFreezeOptionsJs;
}

/// Main input struct for token freeze options.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenFreezeOptionsInput {
    data_contract_id: IdentifierWasm,
    token_position: u16,
    authority_id: IdentifierWasm,
    frozen_identity_id: IdentifierWasm,
    #[serde(default)]
    public_note: Option<String>,
}

fn deserialize_token_freeze_options(
    options: JsValue,
) -> Result<TokenFreezeOptionsInput, WasmSdkError> {
    deserialize_required_query(options, "Options object is required", "token freeze options")
}

/// Result of freezing tokens.
#[wasm_bindgen(js_name = "TokenFreezeResult")]
pub struct TokenFreezeResultWasm {
    frozen_identity_id: IdentifierWasm,
}

#[wasm_bindgen(js_class = TokenFreezeResult)]
impl TokenFreezeResultWasm {
    /// The identity ID that was frozen.
    #[wasm_bindgen(getter = "frozenIdentityId")]
    pub fn frozen_identity_id(&self) -> IdentifierWasm {
        self.frozen_identity_id.clone()
    }
}

#[wasm_bindgen]
impl WasmSdk {
    /// Freeze an identity's token balance.
    ///
    /// @param options - Freeze options including contract ID, token position, authority, frozen identity, and signer
    /// @returns Promise resolving to TokenFreezeResult
    #[wasm_bindgen(js_name = "tokenFreeze")]
    pub async fn token_freeze(
        &self,
        options: TokenFreezeOptionsJs,
    ) -> Result<TokenFreezeResultWasm, WasmSdkError> {
        let options_value: JsValue = options.into();

        // Deserialize and validate options
        let parsed = deserialize_token_freeze_options(options_value.clone())?;

        // Convert identifiers
        let contract_id: Identifier = parsed.data_contract_id.into();
        let authority_id: Identifier = parsed.authority_id.into();
        let frozen_identity_id: Identifier = parsed.frozen_identity_id.into();

        // Extract identity key from options
        let identity_key_wasm =
            IdentityPublicKeyWasm::try_from_options(&options_value, "identityKey")?;
        let identity_key: IdentityPublicKey = identity_key_wasm.into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options_value)?;

        // Extract optional group info from options
        let group_info = GroupStateTransitionInfoStatusWasm::try_from_optional_options(
            &options_value,
            "groupInfo",
        )?
        .map(Into::into);

        // Fetch and cache the data contract
        let _data_contract = self.fetch_and_cache_token_contract(contract_id).await?;

        // Calculate token ID
        let token_id = Identifier::new(calculate_token_id(contract_id.as_bytes(), parsed.token_position));

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

        // Get identity nonce for the token contract
        let identity_nonce = self
            .inner_sdk()
            .get_identity_contract_nonce(authority_id, contract_id, true, settings)
            .await?;

        // Create the freeze transition
        let state_transition = BatchTransition::new_token_freeze_transition(
            token_id,
            authority_id,
            contract_id,
            parsed.token_position,
            frozen_identity_id,
            parsed.public_note,
            group_info,
            &identity_key,
            identity_nonce,
            get_user_fee_increase(settings.as_ref()),
            &signer,
            self.inner_sdk().version(),
            None, // options
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to create freeze transition: {}", e)))?;

        // Broadcast the transition
        state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self.inner_sdk(), settings)
            .await?;

        Ok(TokenFreezeResultWasm {
            frozen_identity_id: frozen_identity_id.into(),
        })
    }
}

// ============================================================================
// Token Unfreeze
// ============================================================================

/// TypeScript interface for token unfreeze options
#[wasm_bindgen(typescript_custom_section)]
const TOKEN_UNFREEZE_OPTIONS_TS: &'static str = r#"
/**
 * Options for unfreezing an identity's token balance.
 */
export interface TokenUnfreezeOptions {
  /**
   * The ID of the data contract containing the token.
   */
  dataContractId: Identifier;

  /**
   * The position of the token in the contract (0-indexed).
   */
  tokenPosition: number;

  /**
   * The identity ID of the token authority performing the unfreeze.
   */
  authorityId: Identifier;

  /**
   * The identity ID to unfreeze.
   */
  frozenIdentityId: Identifier;

  /**
   * Optional public note for the unfreeze operation.
   */
  publicNote?: string;

  /**
   * Signer containing the private key for the authority's authentication key.
   * Use IdentitySigner to add the authentication key before calling.
   */
  signer: IdentitySigner;

  /**
   * Optional group action info for group-managed token unfreezing.
   * Use GroupStateTransitionInfoStatus.proposer() to propose a new group action,
   * or GroupStateTransitionInfoStatus.otherSigner() to vote on an existing action.
   */
  groupInfo?: GroupStateTransitionInfoStatus;

  /**
   * Optional settings for the broadcast operation.
   * Includes retries, timeouts, userFeeIncrease, etc.
   */
  settings?: PutSettings;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "TokenUnfreezeOptions")]
    pub type TokenUnfreezeOptionsJs;
}

/// Main input struct for token unfreeze options.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenUnfreezeOptionsInput {
    data_contract_id: IdentifierWasm,
    token_position: u16,
    authority_id: IdentifierWasm,
    frozen_identity_id: IdentifierWasm,
    #[serde(default)]
    public_note: Option<String>,
}

fn deserialize_token_unfreeze_options(
    options: JsValue,
) -> Result<TokenUnfreezeOptionsInput, WasmSdkError> {
    deserialize_required_query(
        options,
        "Options object is required",
        "token unfreeze options",
    )
}

/// Result of unfreezing tokens.
#[wasm_bindgen(js_name = "TokenUnfreezeResult")]
pub struct TokenUnfreezeResultWasm {
    unfrozen_identity_id: IdentifierWasm,
}

#[wasm_bindgen(js_class = TokenUnfreezeResult)]
impl TokenUnfreezeResultWasm {
    /// The identity ID that was unfrozen.
    #[wasm_bindgen(getter = "unfrozenIdentityId")]
    pub fn unfrozen_identity_id(&self) -> IdentifierWasm {
        self.unfrozen_identity_id.clone()
    }
}

#[wasm_bindgen]
impl WasmSdk {
    /// Unfreeze an identity's token balance.
    ///
    /// @param options - Unfreeze options including contract ID, token position, authority, frozen identity, and signer
    /// @returns Promise resolving to TokenUnfreezeResult
    #[wasm_bindgen(js_name = "tokenUnfreeze")]
    pub async fn token_unfreeze(
        &self,
        options: TokenUnfreezeOptionsJs,
    ) -> Result<TokenUnfreezeResultWasm, WasmSdkError> {
        let options_value: JsValue = options.into();

        // Deserialize and validate options
        let parsed = deserialize_token_unfreeze_options(options_value.clone())?;

        // Convert identifiers
        let contract_id: Identifier = parsed.data_contract_id.into();
        let authority_id: Identifier = parsed.authority_id.into();
        let frozen_identity_id: Identifier = parsed.frozen_identity_id.into();

        // Extract identity key from options
        let identity_key_wasm =
            IdentityPublicKeyWasm::try_from_options(&options_value, "identityKey")?;
        let identity_key: IdentityPublicKey = identity_key_wasm.into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options_value)?;

        // Extract optional group info from options
        let group_info = GroupStateTransitionInfoStatusWasm::try_from_optional_options(
            &options_value,
            "groupInfo",
        )?
        .map(Into::into);

        // Fetch and cache the data contract
        let _data_contract = self.fetch_and_cache_token_contract(contract_id).await?;

        // Calculate token ID
        let token_id = Identifier::new(calculate_token_id(contract_id.as_bytes(), parsed.token_position));

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

        // Get identity nonce for the token contract
        let identity_nonce = self
            .inner_sdk()
            .get_identity_contract_nonce(authority_id, contract_id, true, settings)
            .await?;

        // Create the unfreeze transition
        let state_transition = BatchTransition::new_token_unfreeze_transition(
            token_id,
            authority_id,
            contract_id,
            parsed.token_position,
            frozen_identity_id,
            parsed.public_note,
            group_info,
            &identity_key,
            identity_nonce,
            get_user_fee_increase(settings.as_ref()),
            &signer,
            self.inner_sdk().version(),
            None, // options
        )
        .map_err(|e| {
            WasmSdkError::generic(format!("Failed to create unfreeze transition: {}", e))
        })?;

        // Broadcast the transition
        state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self.inner_sdk(), settings)
            .await?;

        Ok(TokenUnfreezeResultWasm {
            unfrozen_identity_id: frozen_identity_id.into(),
        })
    }
}

// ============================================================================
// Token Destroy Frozen
// ============================================================================

/// TypeScript interface for token destroy frozen options
#[wasm_bindgen(typescript_custom_section)]
const TOKEN_DESTROY_FROZEN_OPTIONS_TS: &'static str = r#"
/**
 * Options for destroying a frozen identity's token balance.
 */
export interface TokenDestroyFrozenOptions {
  /**
   * The ID of the data contract containing the token.
   */
  dataContractId: Identifier;

  /**
   * The position of the token in the contract (0-indexed).
   */
  tokenPosition: number;

  /**
   * The identity ID of the token authority performing the destruction.
   */
  authorityId: Identifier;

  /**
   * The frozen identity ID whose tokens will be destroyed.
   */
  frozenIdentityId: Identifier;

  /**
   * Optional public note for the destruction operation.
   */
  publicNote?: string;

  /**
   * Signer containing the private key for the authority's authentication key.
   * Use IdentitySigner to add the authentication key before calling.
   */
  signer: IdentitySigner;

  /**
   * Optional group action info for group-managed token destruction.
   * Use GroupStateTransitionInfoStatus.proposer() to propose a new group action,
   * or GroupStateTransitionInfoStatus.otherSigner() to vote on an existing action.
   */
  groupInfo?: GroupStateTransitionInfoStatus;

  /**
   * Optional settings for the broadcast operation.
   * Includes retries, timeouts, userFeeIncrease, etc.
   */
  settings?: PutSettings;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "TokenDestroyFrozenOptions")]
    pub type TokenDestroyFrozenOptionsJs;
}

/// Main input struct for token destroy frozen options.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenDestroyFrozenOptionsInput {
    data_contract_id: IdentifierWasm,
    token_position: u16,
    authority_id: IdentifierWasm,
    frozen_identity_id: IdentifierWasm,
    #[serde(default)]
    public_note: Option<String>,
}

fn deserialize_token_destroy_frozen_options(
    options: JsValue,
) -> Result<TokenDestroyFrozenOptionsInput, WasmSdkError> {
    deserialize_required_query(
        options,
        "Options object is required",
        "token destroy frozen options",
    )
}

/// Result of destroying frozen tokens.
#[wasm_bindgen(js_name = "TokenDestroyFrozenResult")]
pub struct TokenDestroyFrozenResultWasm {
    destroyed_identity_id: IdentifierWasm,
}

#[wasm_bindgen(js_class = TokenDestroyFrozenResult)]
impl TokenDestroyFrozenResultWasm {
    /// The identity ID whose tokens were destroyed.
    #[wasm_bindgen(getter = "destroyedIdentityId")]
    pub fn destroyed_identity_id(&self) -> IdentifierWasm {
        self.destroyed_identity_id.clone()
    }
}

#[wasm_bindgen]
impl WasmSdk {
    /// Destroy a frozen identity's token balance.
    ///
    /// @param options - Destroy frozen options including contract ID, token position, authority, frozen identity, and signer
    /// @returns Promise resolving to TokenDestroyFrozenResult
    #[wasm_bindgen(js_name = "tokenDestroyFrozen")]
    pub async fn token_destroy_frozen(
        &self,
        options: TokenDestroyFrozenOptionsJs,
    ) -> Result<TokenDestroyFrozenResultWasm, WasmSdkError> {
        let options_value: JsValue = options.into();

        // Deserialize and validate options
        let parsed = deserialize_token_destroy_frozen_options(options_value.clone())?;

        // Convert identifiers
        let contract_id: Identifier = parsed.data_contract_id.into();
        let authority_id: Identifier = parsed.authority_id.into();
        let frozen_identity_id: Identifier = parsed.frozen_identity_id.into();

        // Extract identity key from options
        let identity_key_wasm =
            IdentityPublicKeyWasm::try_from_options(&options_value, "identityKey")?;
        let identity_key: IdentityPublicKey = identity_key_wasm.into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options_value)?;

        // Extract optional group info from options
        let group_info = GroupStateTransitionInfoStatusWasm::try_from_optional_options(
            &options_value,
            "groupInfo",
        )?
        .map(Into::into);

        // Fetch and cache the data contract
        let _data_contract = self.fetch_and_cache_token_contract(contract_id).await?;

        // Calculate token ID
        let token_id = Identifier::new(calculate_token_id(contract_id.as_bytes(), parsed.token_position));

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

        // Get identity nonce for the token contract
        let identity_nonce = self
            .inner_sdk()
            .get_identity_contract_nonce(authority_id, contract_id, true, settings)
            .await?;

        // Create the destroy frozen transition
        let state_transition = BatchTransition::new_token_destroy_frozen_funds_transition(
            token_id,
            authority_id,
            contract_id,
            parsed.token_position,
            frozen_identity_id,
            parsed.public_note,
            group_info,
            &identity_key,
            identity_nonce,
            get_user_fee_increase(settings.as_ref()),
            &signer,
            self.inner_sdk().version(),
            None, // options
        )
        .map_err(|e| {
            WasmSdkError::generic(format!(
                "Failed to create destroy frozen transition: {}",
                e
            ))
        })?;

        // Broadcast the transition
        state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self.inner_sdk(), settings)
            .await?;

        Ok(TokenDestroyFrozenResultWasm {
            destroyed_identity_id: frozen_identity_id.into(),
        })
    }
}

// ============================================================================
// Token Emergency Action (Pause/Resume)
// ============================================================================

/// TypeScript interface for token emergency action options
#[wasm_bindgen(typescript_custom_section)]
const TOKEN_EMERGENCY_ACTION_OPTIONS_TS: &'static str = r#"
/**
 * Options for performing an emergency action (pause/resume) on a token.
 */
export interface TokenEmergencyActionOptions {
  /**
   * The ID of the data contract containing the token.
   */
  dataContractId: Identifier;

  /**
   * The position of the token in the contract (0-indexed).
   */
  tokenPosition: number;

  /**
   * The identity ID of the token authority performing the action.
   */
  authorityId: Identifier;

  /**
   * The emergency action to perform: "pause" or "resume".
   */
  action: "pause" | "resume";

  /**
   * Optional public note for the emergency action.
   */
  publicNote?: string;

  /**
   * Signer containing the private key for the authority's authentication key.
   * Use IdentitySigner to add the authentication key before calling.
   */
  signer: IdentitySigner;

  /**
   * Optional group action info for group-managed emergency actions.
   * Use GroupStateTransitionInfoStatus.proposer() to propose a new group action,
   * or GroupStateTransitionInfoStatus.otherSigner() to vote on an existing action.
   */
  groupInfo?: GroupStateTransitionInfoStatus;

  /**
   * Optional settings for the broadcast operation.
   * Includes retries, timeouts, userFeeIncrease, etc.
   */
  settings?: PutSettings;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "TokenEmergencyActionOptions")]
    pub type TokenEmergencyActionOptionsJs;
}

/// Main input struct for token emergency action options.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenEmergencyActionOptionsInput {
    data_contract_id: IdentifierWasm,
    token_position: u16,
    authority_id: IdentifierWasm,
    action: String,
    #[serde(default)]
    public_note: Option<String>,
}

fn deserialize_token_emergency_action_options(
    options: JsValue,
) -> Result<TokenEmergencyActionOptionsInput, WasmSdkError> {
    deserialize_required_query(
        options,
        "Options object is required",
        "token emergency action options",
    )
}

/// Result of an emergency action.
#[wasm_bindgen(js_name = "TokenEmergencyActionResult")]
pub struct TokenEmergencyActionResultWasm {
    action: String,
}

#[wasm_bindgen(js_class = TokenEmergencyActionResult)]
impl TokenEmergencyActionResultWasm {
    /// The action that was performed ("pause" or "resume").
    #[wasm_bindgen(getter)]
    pub fn action(&self) -> String {
        self.action.clone()
    }
}

#[wasm_bindgen]
impl WasmSdk {
    /// Perform an emergency action (pause or resume) on a token.
    ///
    /// @param options - Emergency action options including contract ID, token position, action type, and signer
    /// @returns Promise resolving to TokenEmergencyActionResult
    #[wasm_bindgen(js_name = "tokenEmergencyAction")]
    pub async fn token_emergency_action(
        &self,
        options: TokenEmergencyActionOptionsJs,
    ) -> Result<TokenEmergencyActionResultWasm, WasmSdkError> {
        use dash_sdk::dpp::tokens::emergency_action::TokenEmergencyAction;

        let options_value: JsValue = options.into();

        // Deserialize and validate options
        let parsed = deserialize_token_emergency_action_options(options_value.clone())?;

        // Convert identifiers
        let contract_id: Identifier = parsed.data_contract_id.into();
        let authority_id: Identifier = parsed.authority_id.into();

        // Parse the action
        let emergency_action = match parsed.action.to_lowercase().as_str() {
            "pause" => TokenEmergencyAction::Pause,
            "resume" => TokenEmergencyAction::Resume,
            _ => {
                return Err(WasmSdkError::invalid_argument(
                    "action must be 'pause' or 'resume'",
                ))
            }
        };

        // Extract identity key from options
        let identity_key_wasm =
            IdentityPublicKeyWasm::try_from_options(&options_value, "identityKey")?;
        let identity_key: IdentityPublicKey = identity_key_wasm.into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options_value)?;

        // Extract optional group info from options
        let group_info = GroupStateTransitionInfoStatusWasm::try_from_optional_options(
            &options_value,
            "groupInfo",
        )?
        .map(Into::into);

        // Fetch and cache the data contract
        let _data_contract = self.fetch_and_cache_token_contract(contract_id).await?;

        // Calculate token ID
        let token_id =
            Identifier::new(calculate_token_id(contract_id.as_bytes(), parsed.token_position));

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

        // Get identity nonce for the token contract
        let identity_nonce = self
            .inner_sdk()
            .get_identity_contract_nonce(authority_id, contract_id, true, settings)
            .await?;

        // Create the emergency action transition
        let state_transition = BatchTransition::new_token_emergency_action_transition(
            token_id,
            authority_id,
            contract_id,
            parsed.token_position,
            emergency_action,
            parsed.public_note,
            group_info,
            &identity_key,
            identity_nonce,
            get_user_fee_increase(settings.as_ref()),
            &signer,
            self.inner_sdk().version(),
            None, // options
        )
        .map_err(|e| {
            WasmSdkError::generic(format!("Failed to create emergency action transition: {}", e))
        })?;

        // Broadcast the transition
        state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self.inner_sdk(), settings)
            .await?;

        Ok(TokenEmergencyActionResultWasm {
            action: parsed.action,
        })
    }
}

// ============================================================================
// Token Claim
// ============================================================================

/// TypeScript interface for token claim options
#[wasm_bindgen(typescript_custom_section)]
const TOKEN_CLAIM_OPTIONS_TS: &'static str = r#"
/**
 * Options for claiming tokens from a distribution.
 */
export interface TokenClaimOptions {
  /**
   * The ID of the data contract containing the token.
   */
  dataContractId: Identifier;

  /**
   * The position of the token in the contract (0-indexed).
   */
  tokenPosition: number;

  /**
   * The identity ID claiming the tokens.
   */
  identityId: Identifier;

  /**
   * The type of distribution to claim from: "preProgrammed" or "perpetual".
   */
  distributionType: "preProgrammed" | "perpetual";

  /**
   * Optional public note for the claim operation.
   */
  publicNote?: string;

  /**
   * The identity public key to use for signing the transition.
   */
  identityKey: IdentityPublicKey;

  /**
   * Signer containing the private key that corresponds to the identity key.
   * Use IdentitySigner to add the private key before calling.
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
    #[wasm_bindgen(typescript_type = "TokenClaimOptions")]
    pub type TokenClaimOptionsJs;
}

/// Main input struct for token claim options.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenClaimOptionsInput {
    data_contract_id: IdentifierWasm,
    token_position: u16,
    identity_id: IdentifierWasm,
    distribution_type: String,
    #[serde(default)]
    public_note: Option<String>,
}

fn deserialize_token_claim_options(options: JsValue) -> Result<TokenClaimOptionsInput, WasmSdkError> {
    deserialize_required_query(options, "Options object is required", "token claim options")
}

/// Result of claiming tokens.
#[wasm_bindgen(js_name = "TokenClaimResult")]
pub struct TokenClaimResultWasm {
    identity_id: IdentifierWasm,
    distribution_type: String,
}

#[wasm_bindgen(js_class = TokenClaimResult)]
impl TokenClaimResultWasm {
    /// The identity ID that claimed tokens.
    #[wasm_bindgen(getter = "identityId")]
    pub fn identity_id(&self) -> IdentifierWasm {
        self.identity_id.clone()
    }

    /// The distribution type that was claimed from.
    #[wasm_bindgen(getter = "distributionType")]
    pub fn distribution_type(&self) -> String {
        self.distribution_type.clone()
    }
}

#[wasm_bindgen]
impl WasmSdk {
    /// Claim tokens from a distribution.
    ///
    /// @param options - Claim options including contract ID, token position, distribution type, and signer
    /// @returns Promise resolving to TokenClaimResult
    #[wasm_bindgen(js_name = "tokenClaim")]
    pub async fn token_claim(
        &self,
        options: TokenClaimOptionsJs,
    ) -> Result<TokenClaimResultWasm, WasmSdkError> {
        use dash_sdk::dpp::data_contract::associated_token::token_distribution_key::TokenDistributionType;

        let options_value: JsValue = options.into();

        // Deserialize and validate options
        let parsed = deserialize_token_claim_options(options_value.clone())?;

        // Convert identifiers
        let contract_id: Identifier = parsed.data_contract_id.into();
        let identity_id: Identifier = parsed.identity_id.into();

        // Parse the distribution type
        let distribution_type = match parsed.distribution_type.to_lowercase().as_str() {
            "preprogrammed" => TokenDistributionType::PreProgrammed,
            "perpetual" => TokenDistributionType::Perpetual,
            _ => {
                return Err(WasmSdkError::invalid_argument(
                    "distributionType must be 'preProgrammed' or 'perpetual'",
                ))
            }
        };

        // Extract identity key from options
        let identity_key_wasm =
            IdentityPublicKeyWasm::try_from_options(&options_value, "identityKey")?;
        let identity_key: IdentityPublicKey = identity_key_wasm.into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options_value)?;

        // Fetch and cache the data contract
        let _data_contract = self.fetch_and_cache_token_contract(contract_id).await?;

        // Calculate token ID
        let token_id =
            Identifier::new(calculate_token_id(contract_id.as_bytes(), parsed.token_position));

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

        // Get identity nonce for the token contract
        let identity_nonce = self
            .inner_sdk()
            .get_identity_contract_nonce(identity_id, contract_id, true, settings)
            .await?;

        // Create the claim transition
        let state_transition = BatchTransition::new_token_claim_transition(
            token_id,
            identity_id,
            contract_id,
            parsed.token_position,
            distribution_type,
            parsed.public_note,
            &identity_key,
            identity_nonce,
            get_user_fee_increase(settings.as_ref()),
            &signer,
            self.inner_sdk().version(),
            None, // options
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to create claim transition: {}", e)))?;

        // Broadcast the transition
        state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self.inner_sdk(), settings)
            .await?;

        Ok(TokenClaimResultWasm {
            identity_id: identity_id.into(),
            distribution_type: parsed.distribution_type,
        })
    }
}

// ============================================================================
// Token Set Price for Direct Purchase
// ============================================================================

/// TypeScript interface for token set price options
#[wasm_bindgen(typescript_custom_section)]
const TOKEN_SET_PRICE_OPTIONS_TS: &'static str = r#"
/**
 * Options for setting the price of a token for direct purchase.
 */
export interface TokenSetPriceOptions {
  /**
   * The ID of the data contract containing the token.
   */
  dataContractId: Identifier;

  /**
   * The position of the token in the contract (0-indexed).
   */
  tokenPosition: number;

  /**
   * The identity ID of the token authority setting the price.
   */
  authorityId: Identifier;

  /**
   * The price in credits for one token.
   * Set to null to disable direct purchases.
   */
  price: bigint | number | null;

  /**
   * Optional public note for the price change.
   */
  publicNote?: string;

  /**
   * Signer containing the private key for the authority's authentication key.
   * Use IdentitySigner to add the authentication key before calling.
   */
  signer: IdentitySigner;

  /**
   * Optional group action info for group-managed price changes.
   * Use GroupStateTransitionInfoStatus.proposer() to propose a new group action,
   * or GroupStateTransitionInfoStatus.otherSigner() to vote on an existing action.
   */
  groupInfo?: GroupStateTransitionInfoStatus;

  /**
   * Optional settings for the broadcast operation.
   * Includes retries, timeouts, userFeeIncrease, etc.
   */
  settings?: PutSettings;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "TokenSetPriceOptions")]
    pub type TokenSetPriceOptionsJs;
}

/// Main input struct for token set price options.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenSetPriceOptionsInput {
    data_contract_id: IdentifierWasm,
    token_position: u16,
    authority_id: IdentifierWasm,
    #[serde(default)]
    price: Option<u64>,
    #[serde(default)]
    public_note: Option<String>,
}

fn deserialize_token_set_price_options(
    options: JsValue,
) -> Result<TokenSetPriceOptionsInput, WasmSdkError> {
    deserialize_required_query(
        options,
        "Options object is required",
        "token set price options",
    )
}

/// Result of setting the token price.
#[wasm_bindgen(js_name = "TokenSetPriceResult")]
pub struct TokenSetPriceResultWasm {
    price: Option<u64>,
}

#[wasm_bindgen(js_class = TokenSetPriceResult)]
impl TokenSetPriceResultWasm {
    /// The new price that was set, or null if purchases are disabled.
    #[wasm_bindgen(getter)]
    pub fn price(&self) -> Option<BigInt> {
        self.price.map(BigInt::from)
    }
}

#[wasm_bindgen]
impl WasmSdk {
    /// Set the price of a token for direct purchase.
    ///
    /// @param options - Price options including contract ID, token position, price, and signer
    /// @returns Promise resolving to TokenSetPriceResult
    #[wasm_bindgen(js_name = "tokenSetPrice")]
    pub async fn token_set_price(
        &self,
        options: TokenSetPriceOptionsJs,
    ) -> Result<TokenSetPriceResultWasm, WasmSdkError> {
        use dash_sdk::dpp::tokens::token_pricing_schedule::TokenPricingSchedule;

        let options_value: JsValue = options.into();

        // Deserialize and validate options
        let parsed = deserialize_token_set_price_options(options_value.clone())?;

        // Convert identifiers
        let contract_id: Identifier = parsed.data_contract_id.into();
        let authority_id: Identifier = parsed.authority_id.into();

        // Convert price to pricing schedule
        let pricing_schedule = parsed.price.map(TokenPricingSchedule::SinglePrice);

        // Extract identity key from options
        let identity_key_wasm =
            IdentityPublicKeyWasm::try_from_options(&options_value, "identityKey")?;
        let identity_key: IdentityPublicKey = identity_key_wasm.into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options_value)?;

        // Extract optional group info from options
        let group_info = GroupStateTransitionInfoStatusWasm::try_from_optional_options(
            &options_value,
            "groupInfo",
        )?
        .map(Into::into);

        // Fetch and cache the data contract
        let _data_contract = self.fetch_and_cache_token_contract(contract_id).await?;

        // Calculate token ID
        let token_id =
            Identifier::new(calculate_token_id(contract_id.as_bytes(), parsed.token_position));

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

        // Get identity nonce for the token contract
        let identity_nonce = self
            .inner_sdk()
            .get_identity_contract_nonce(authority_id, contract_id, true, settings)
            .await?;

        // Create the set price transition
        let state_transition = BatchTransition::new_token_change_direct_purchase_price_transition(
            token_id,
            authority_id,
            contract_id,
            parsed.token_position,
            pricing_schedule,
            parsed.public_note,
            group_info,
            &identity_key,
            identity_nonce,
            get_user_fee_increase(settings.as_ref()),
            &signer,
            self.inner_sdk().version(),
            None, // options
        )
        .map_err(|e| {
            WasmSdkError::generic(format!("Failed to create set price transition: {}", e))
        })?;

        // Broadcast the transition
        state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self.inner_sdk(), settings)
            .await?;

        Ok(TokenSetPriceResultWasm {
            price: parsed.price,
        })
    }
}

// ============================================================================
// Token Direct Purchase
// ============================================================================

/// TypeScript interface for token direct purchase options
#[wasm_bindgen(typescript_custom_section)]
const TOKEN_DIRECT_PURCHASE_OPTIONS_TS: &'static str = r#"
/**
 * Options for directly purchasing tokens.
 */
export interface TokenDirectPurchaseOptions {
  /**
   * The ID of the data contract containing the token.
   */
  dataContractId: Identifier;

  /**
   * The position of the token in the contract (0-indexed).
   */
  tokenPosition: number;

  /**
   * The identity ID purchasing the tokens.
   */
  buyerId: Identifier;

  /**
   * The amount of tokens to purchase.
   */
  amount: bigint | number;

  /**
   * The maximum total credits the buyer is willing to pay.
   * The actual cost may be less if the token price is lower.
   */
  maxTotalCost: bigint | number;

  /**
   * Signer containing the private key for the buyer's authentication key.
   * Use IdentitySigner to add the authentication key before calling.
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
    #[wasm_bindgen(typescript_type = "TokenDirectPurchaseOptions")]
    pub type TokenDirectPurchaseOptionsJs;
}

/// Main input struct for token direct purchase options.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenDirectPurchaseOptionsInput {
    data_contract_id: IdentifierWasm,
    token_position: u16,
    buyer_id: IdentifierWasm,
    amount: u64,
    max_total_cost: u64,
}

fn deserialize_token_direct_purchase_options(
    options: JsValue,
) -> Result<TokenDirectPurchaseOptionsInput, WasmSdkError> {
    deserialize_required_query(
        options,
        "Options object is required",
        "token direct purchase options",
    )
}

/// Result of a direct token purchase.
#[wasm_bindgen(js_name = "TokenDirectPurchaseResult")]
pub struct TokenDirectPurchaseResultWasm {
    buyer_id: IdentifierWasm,
    amount: u64,
}

#[wasm_bindgen(js_class = TokenDirectPurchaseResult)]
impl TokenDirectPurchaseResultWasm {
    /// The buyer's identity ID.
    #[wasm_bindgen(getter = "buyerId")]
    pub fn buyer_id(&self) -> IdentifierWasm {
        self.buyer_id.clone()
    }

    /// The amount of tokens purchased.
    #[wasm_bindgen(getter)]
    pub fn amount(&self) -> BigInt {
        BigInt::from(self.amount)
    }
}

#[wasm_bindgen]
impl WasmSdk {
    /// Directly purchase tokens using credits.
    ///
    /// @param options - Purchase options including contract ID, token position, amount, max cost, and signer
    /// @returns Promise resolving to TokenDirectPurchaseResult
    #[wasm_bindgen(js_name = "tokenDirectPurchase")]
    pub async fn token_direct_purchase(
        &self,
        options: TokenDirectPurchaseOptionsJs,
    ) -> Result<TokenDirectPurchaseResultWasm, WasmSdkError> {
        let options_value: JsValue = options.into();

        // Deserialize and validate options
        let parsed = deserialize_token_direct_purchase_options(options_value.clone())?;

        // Convert identifiers
        let contract_id: Identifier = parsed.data_contract_id.into();
        let buyer_id: Identifier = parsed.buyer_id.into();
        let amount = parsed.amount as TokenAmount;
        let max_total_cost = parsed.max_total_cost;

        // Extract identity key from options
        let identity_key_wasm =
            IdentityPublicKeyWasm::try_from_options(&options_value, "identityKey")?;
        let identity_key: IdentityPublicKey = identity_key_wasm.into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options_value)?;

        // Fetch and cache the data contract
        let _data_contract = self.fetch_and_cache_token_contract(contract_id).await?;

        // Calculate token ID
        let token_id =
            Identifier::new(calculate_token_id(contract_id.as_bytes(), parsed.token_position));

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

        // Get identity nonce for the token contract
        let identity_nonce = self
            .inner_sdk()
            .get_identity_contract_nonce(buyer_id, contract_id, true, settings)
            .await?;

        // Create the direct purchase transition
        let state_transition = BatchTransition::new_token_direct_purchase_transition(
            token_id,
            buyer_id,
            contract_id,
            parsed.token_position,
            amount,
            max_total_cost,
            &identity_key,
            identity_nonce,
            get_user_fee_increase(settings.as_ref()),
            &signer,
            self.inner_sdk().version(),
            None, // options
        )
        .map_err(|e| {
            WasmSdkError::generic(format!("Failed to create direct purchase transition: {}", e))
        })?;

        // Broadcast the transition
        state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self.inner_sdk(), settings)
            .await?;

        Ok(TokenDirectPurchaseResultWasm {
            buyer_id: buyer_id.into(),
            amount: parsed.amount,
        })
    }
}
