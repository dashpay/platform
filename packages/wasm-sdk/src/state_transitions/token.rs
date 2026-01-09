//! Token state transition implementations for the WASM SDK.
//!
//! This module provides WASM bindings for token operations like mint, burn, transfer, etc.

use crate::error::WasmSdkError;
use crate::queries::utils::deserialize_required_query;
use crate::sdk::WasmSdk;
use crate::settings::{extract_settings_from_options, get_user_fee_increase};
use dash_sdk::dpp::balances::credits::TokenAmount;
use dash_sdk::dpp::document::Document;
use dash_sdk::dpp::identity::IdentityPublicKey;
use dash_sdk::dpp::platform_value::Identifier;
use dash_sdk::platform::tokens::builders::{
    burn::TokenBurnTransitionBuilder, claim::TokenClaimTransitionBuilder,
    destroy::TokenDestroyFrozenFundsTransitionBuilder,
    emergency_action::TokenEmergencyActionTransitionBuilder, freeze::TokenFreezeTransitionBuilder,
    mint::TokenMintTransitionBuilder, purchase::TokenDirectPurchaseTransitionBuilder,
    set_price::TokenChangeDirectPurchasePriceTransitionBuilder,
    transfer::TokenTransferTransitionBuilder, unfreeze::TokenUnfreezeTransitionBuilder,
};
use dash_sdk::platform::tokens::transitions::{
    BurnResult, ClaimResult, DestroyFrozenFundsResult, DirectPurchaseResult, EmergencyActionResult,
    FreezeResult, MintResult, SetPriceResult, TransferResult, UnfreezeResult,
};
use dash_sdk::platform::Fetch;
use js_sys::BigInt;
use serde::Deserialize;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_dpp2::data_contract::document::DocumentWasm;
use wasm_dpp2::identifier::IdentifierWasm;
use wasm_dpp2::identity::IdentityPublicKeyWasm;
use wasm_dpp2::state_transitions::base::GroupStateTransitionInfoStatusWasm;
use wasm_dpp2::IdentitySignerWasm;

/// Helper function to convert a Document to DocumentWasm with the required metadata.
/// Token historical documents use the contract_id and a document type name based on the operation.
fn document_to_wasm(
    doc: Document,
    contract_id: Identifier,
    document_type_name: &str,
) -> DocumentWasm {
    DocumentWasm::new(doc, contract_id, document_type_name.to_string(), None)
}

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
///
/// The result type depends on token configuration:
/// - Standard tokens: returns recipient ID and new balance
/// - Tokens with history: returns a document
/// - Group-managed tokens: returns group power and action status
///
/// Check which optional fields are present to determine the result type.
#[wasm_bindgen(js_name = "TokenMintResult")]
pub struct TokenMintResultWasm {
    /// For TokenBalance result - recipient identity ID
    recipient_id: Option<IdentifierWasm>,
    /// For TokenBalance or GroupActionWithBalance - the new balance
    new_balance: Option<u64>,
    /// For group actions - accumulated group power
    group_power: Option<u32>,
    /// For GroupActionWithBalance - action status
    group_action_status: Option<String>,
    /// For HistoricalDocument or GroupActionWithDocument - the document
    document: Option<DocumentWasm>,
}

#[wasm_bindgen(js_class = TokenMintResult)]
impl TokenMintResultWasm {
    /// The recipient's identity ID (for balance results).
    #[wasm_bindgen(getter = "recipientId")]
    pub fn recipient_id(&self) -> Option<IdentifierWasm> {
        self.recipient_id.clone()
    }

    /// The new token balance after minting.
    #[wasm_bindgen(getter = "newBalance")]
    pub fn new_balance(&self) -> Option<BigInt> {
        self.new_balance.map(BigInt::from)
    }

    /// The accumulated group power (for group actions).
    #[wasm_bindgen(getter = "groupPower")]
    pub fn group_power(&self) -> Option<u32> {
        self.group_power
    }

    /// The group action status (for group actions).
    #[wasm_bindgen(getter = "groupActionStatus")]
    pub fn group_action_status(&self) -> Option<String> {
        self.group_action_status.clone()
    }

    /// The historical document (for tokens with history tracking).
    #[wasm_bindgen(getter)]
    pub fn document(&self) -> Option<DocumentWasm> {
        self.document.clone()
    }
}

impl TokenMintResultWasm {
    /// Convert from SDK MintResult with the required contract context
    fn from_result(result: MintResult, contract_id: Identifier) -> Self {
        match result {
            MintResult::TokenBalance(recipient_id, balance) => TokenMintResultWasm {
                recipient_id: Some(recipient_id.into()),
                new_balance: Some(balance),
                group_power: None,
                group_action_status: None,
                document: None,
            },
            MintResult::HistoricalDocument(doc) => TokenMintResultWasm {
                recipient_id: None,
                new_balance: None,
                group_power: None,
                group_action_status: None,
                document: Some(document_to_wasm(doc, contract_id, "mint")),
            },
            MintResult::GroupActionWithDocument(power, doc) => TokenMintResultWasm {
                recipient_id: None,
                new_balance: None,
                group_power: Some(power as u32),
                group_action_status: None,
                document: doc.map(|d| document_to_wasm(d, contract_id, "mint")),
            },
            MintResult::GroupActionWithBalance(power, status, balance) => TokenMintResultWasm {
                recipient_id: None,
                new_balance: balance,
                group_power: Some(power as u32),
                group_action_status: Some(format!("{:?}", status)),
                document: None,
            },
        }
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
        let amount = parsed.amount as TokenAmount;

        // Extract identity key from options
        let identity_key_wasm =
            IdentityPublicKeyWasm::try_from_options(&options_value, "identityKey")?;
        let identity_key: IdentityPublicKey = identity_key_wasm.into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options_value)?;

        // Fetch and cache the data contract
        let data_contract = self.fetch_and_cache_token_contract(contract_id).await?;

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

        // Build the mint transition using rs-sdk builder
        let mut builder = TokenMintTransitionBuilder::new(
            Arc::new(data_contract),
            parsed.token_position,
            identity_id,
            amount,
        );

        // Add optional recipient
        if let Some(recipient_id) = parsed.recipient_id {
            builder = builder.issued_to_identity_id(recipient_id.into());
        }

        // Add optional public note
        if let Some(note) = parsed.public_note {
            builder = builder.with_public_note(note);
        }

        // Add optional group info
        if let Some(group_info) = GroupStateTransitionInfoStatusWasm::try_from_optional_options(
            &options_value,
            "groupInfo",
        )? {
            builder = builder.with_using_group_info(group_info.into());
        }

        // Add settings
        if let Some(settings) = settings {
            builder = builder.with_settings(settings);
        }

        // Add user fee increase from settings
        let user_fee_increase =
            get_user_fee_increase(extract_settings_from_options(&options_value)?.as_ref());
        if user_fee_increase > 0 {
            builder = builder.with_user_fee_increase(user_fee_increase);
        }

        // Use the SDK's token_mint method which handles signing, broadcasting, and result parsing
        let result = self
            .inner_sdk()
            .token_mint(builder, &identity_key, &signer)
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to mint tokens: {}", e)))?;

        Ok(TokenMintResultWasm::from_result(result, contract_id))
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
///
/// The result type depends on token configuration:
/// - Standard tokens: returns owner ID and remaining balance
/// - Tokens with history: returns a document
/// - Group-managed tokens: returns group power and action status
///
/// Check which optional fields are present to determine the result type.
#[wasm_bindgen(js_name = "TokenBurnResult")]
pub struct TokenBurnResultWasm {
    /// For TokenBalance result
    owner_id: Option<IdentifierWasm>,
    remaining_balance: Option<u64>,
    /// For group actions
    group_power: Option<u32>,
    group_action_status: Option<String>,
    /// For HistoricalDocument or GroupActionWithDocument - the document
    document: Option<DocumentWasm>,
}

#[wasm_bindgen(js_class = TokenBurnResult)]
impl TokenBurnResultWasm {
    /// The owner's identity ID (for balance results).
    #[wasm_bindgen(getter = "ownerId")]
    pub fn owner_id(&self) -> Option<IdentifierWasm> {
        self.owner_id.clone()
    }

    /// The remaining token balance after burning.
    #[wasm_bindgen(getter = "remainingBalance")]
    pub fn remaining_balance(&self) -> Option<BigInt> {
        self.remaining_balance.map(BigInt::from)
    }

    /// The accumulated group power (for group actions).
    #[wasm_bindgen(getter = "groupPower")]
    pub fn group_power(&self) -> Option<u32> {
        self.group_power
    }

    /// The group action status (for group actions).
    #[wasm_bindgen(getter = "groupActionStatus")]
    pub fn group_action_status(&self) -> Option<String> {
        self.group_action_status.clone()
    }

    /// The historical document (for tokens with history tracking).
    #[wasm_bindgen(getter)]
    pub fn document(&self) -> Option<DocumentWasm> {
        self.document.clone()
    }
}

impl TokenBurnResultWasm {
    /// Convert from SDK BurnResult with the required contract context
    fn from_result(result: BurnResult, contract_id: Identifier) -> Self {
        match result {
            BurnResult::TokenBalance(owner_id, balance) => TokenBurnResultWasm {
                owner_id: Some(owner_id.into()),
                remaining_balance: Some(balance),
                group_power: None,
                group_action_status: None,
                document: None,
            },
            BurnResult::HistoricalDocument(doc) => TokenBurnResultWasm {
                owner_id: None,
                remaining_balance: None,
                group_power: None,
                group_action_status: None,
                document: Some(document_to_wasm(doc, contract_id, "burn")),
            },
            BurnResult::GroupActionWithDocument(power, doc) => TokenBurnResultWasm {
                owner_id: None,
                remaining_balance: None,
                group_power: Some(power as u32),
                group_action_status: None,
                document: doc.map(|d| document_to_wasm(d, contract_id, "burn")),
            },
            BurnResult::GroupActionWithBalance(power, status, balance) => TokenBurnResultWasm {
                owner_id: None,
                remaining_balance: balance,
                group_power: Some(power as u32),
                group_action_status: Some(format!("{:?}", status)),
                document: None,
            },
        }
    }
}

#[wasm_bindgen]
impl WasmSdk {
    /// Burn tokens from an identity's balance.
    ///
    /// @param options - Burn options including contract ID, token position, amount, and signer
    /// @returns Promise resolving to TokenBurnResult with the remaining balance
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

        // Fetch and cache the data contract
        let data_contract = self.fetch_and_cache_token_contract(contract_id).await?;

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

        // Build the burn transition using rs-sdk builder
        let mut builder = TokenBurnTransitionBuilder::new(
            Arc::new(data_contract),
            parsed.token_position,
            identity_id,
            amount,
        );

        // Add optional public note
        if let Some(note) = parsed.public_note {
            builder = builder.with_public_note(note);
        }

        // Add optional group info
        if let Some(group_info) = GroupStateTransitionInfoStatusWasm::try_from_optional_options(
            &options_value,
            "groupInfo",
        )? {
            builder = builder.with_using_group_info(group_info.into());
        }

        // Add settings
        if let Some(settings) = settings {
            builder = builder.with_settings(settings);
        }

        // Add user fee increase from settings
        let user_fee_increase =
            get_user_fee_increase(extract_settings_from_options(&options_value)?.as_ref());
        if user_fee_increase > 0 {
            builder = builder.with_user_fee_increase(user_fee_increase);
        }

        // Use the SDK's token_burn method
        let result = self
            .inner_sdk()
            .token_burn(builder, &identity_key, &signer)
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to burn tokens: {}", e)))?;

        Ok(TokenBurnResultWasm::from_result(result, contract_id))
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
   * The identity public key to use for signing the transition.
   */
  identityKey: IdentityPublicKey;

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
///
/// The result type depends on token configuration:
/// - Standard tokens: returns identities balances
/// - Tokens with history: returns a document
/// - Group-managed tokens: returns group power and document
///
/// Check which optional fields are present to determine the result type.
#[wasm_bindgen(js_name = "TokenTransferResult")]
pub struct TokenTransferResultWasm {
    /// For IdentitiesBalances result - sender's new balance
    sender_balance: Option<u64>,
    /// For IdentitiesBalances result - recipient's new balance
    recipient_balance: Option<u64>,
    /// For group actions
    group_power: Option<u32>,
    /// For HistoricalDocument or GroupActionWithDocument - the document
    document: Option<DocumentWasm>,
}

#[wasm_bindgen(js_class = TokenTransferResult)]
impl TokenTransferResultWasm {
    /// The sender's new balance after transfer.
    #[wasm_bindgen(getter = "senderBalance")]
    pub fn sender_balance(&self) -> Option<BigInt> {
        self.sender_balance.map(BigInt::from)
    }

    /// The recipient's new balance after transfer.
    #[wasm_bindgen(getter = "recipientBalance")]
    pub fn recipient_balance(&self) -> Option<BigInt> {
        self.recipient_balance.map(BigInt::from)
    }

    /// The accumulated group power (for group actions).
    #[wasm_bindgen(getter = "groupPower")]
    pub fn group_power(&self) -> Option<u32> {
        self.group_power
    }

    /// The historical document (for tokens with history tracking).
    #[wasm_bindgen(getter)]
    pub fn document(&self) -> Option<DocumentWasm> {
        self.document.clone()
    }
}

impl TokenTransferResultWasm {
    /// Convert from SDK TransferResult with the required contract context
    fn from_result(result: TransferResult, contract_id: Identifier) -> Self {
        match result {
            TransferResult::IdentitiesBalances(balances) => {
                // Get the first two balances (sender and recipient)
                let mut iter = balances.into_values();
                let first_balance = iter.next();
                let second_balance = iter.next();
                TokenTransferResultWasm {
                    sender_balance: first_balance,
                    recipient_balance: second_balance,
                    group_power: None,
                    document: None,
                }
            }
            TransferResult::HistoricalDocument(doc) => TokenTransferResultWasm {
                sender_balance: None,
                recipient_balance: None,
                group_power: None,
                document: Some(document_to_wasm(doc, contract_id, "transfer")),
            },
            TransferResult::GroupActionWithDocument(power, doc) => TokenTransferResultWasm {
                sender_balance: None,
                recipient_balance: None,
                group_power: Some(power as u32),
                document: doc.map(|d| document_to_wasm(d, contract_id, "transfer")),
            },
        }
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
        let data_contract = self.fetch_and_cache_token_contract(contract_id).await?;

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

        // Build the transfer transition using rs-sdk builder
        let mut builder = TokenTransferTransitionBuilder::new(
            Arc::new(data_contract),
            parsed.token_position,
            sender_id,
            recipient_id,
            amount,
        );

        // Add optional public note
        if let Some(note) = parsed.public_note {
            builder = builder.with_public_note(note);
        }

        // Add settings
        if let Some(settings) = settings {
            builder = builder.with_settings(settings);
        }

        // Add user fee increase from settings
        let user_fee_increase =
            get_user_fee_increase(extract_settings_from_options(&options_value)?.as_ref());
        if user_fee_increase > 0 {
            builder = builder.with_user_fee_increase(user_fee_increase);
        }

        // Use the SDK's token_transfer method
        let result = self
            .inner_sdk()
            .token_transfer(builder, &identity_key, &signer)
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to transfer tokens: {}", e)))?;

        Ok(TokenTransferResultWasm::from_result(result, contract_id))
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
    deserialize_required_query(
        options,
        "Options object is required",
        "token freeze options",
    )
}

/// Result of freezing tokens.
///
/// The result type depends on token configuration:
/// - Standard tokens: returns frozen identity ID
/// - Tokens with history: returns a document
/// - Group-managed tokens: returns group power and status
///
/// Check which optional fields are present to determine the result type.
#[wasm_bindgen(js_name = "TokenFreezeResult")]
pub struct TokenFreezeResultWasm {
    /// For IdentityInfo result
    frozen_identity_id: Option<IdentifierWasm>,
    /// For group actions
    group_power: Option<u32>,
    /// For HistoricalDocument or GroupActionWithDocument - the document
    document: Option<DocumentWasm>,
}

#[wasm_bindgen(js_class = TokenFreezeResult)]
impl TokenFreezeResultWasm {
    /// The identity ID that was frozen.
    #[wasm_bindgen(getter = "frozenIdentityId")]
    pub fn frozen_identity_id(&self) -> Option<IdentifierWasm> {
        self.frozen_identity_id.clone()
    }

    /// The accumulated group power (for group actions).
    #[wasm_bindgen(getter = "groupPower")]
    pub fn group_power(&self) -> Option<u32> {
        self.group_power
    }

    /// The historical document (for tokens with history tracking).
    #[wasm_bindgen(getter)]
    pub fn document(&self) -> Option<DocumentWasm> {
        self.document.clone()
    }
}

impl TokenFreezeResultWasm {
    /// Convert from SDK FreezeResult with the required contract context
    fn from_result(result: FreezeResult, contract_id: Identifier) -> Self {
        match result {
            FreezeResult::IdentityInfo(frozen_id, _info) => TokenFreezeResultWasm {
                frozen_identity_id: Some(frozen_id.into()),
                group_power: None,
                document: None,
            },
            FreezeResult::HistoricalDocument(doc) => TokenFreezeResultWasm {
                frozen_identity_id: None,
                group_power: None,
                document: Some(document_to_wasm(doc, contract_id, "freeze")),
            },
            FreezeResult::GroupActionWithDocument(power, doc) => TokenFreezeResultWasm {
                frozen_identity_id: None,
                group_power: Some(power as u32),
                document: doc.map(|d| document_to_wasm(d, contract_id, "freeze")),
            },
            FreezeResult::GroupActionWithIdentityInfo(power, _info) => TokenFreezeResultWasm {
                frozen_identity_id: None,
                group_power: Some(power as u32),
                document: None,
            },
        }
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

        // Fetch and cache the data contract
        let data_contract = self.fetch_and_cache_token_contract(contract_id).await?;

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

        // Build the freeze transition using rs-sdk builder
        let mut builder = TokenFreezeTransitionBuilder::new(
            Arc::new(data_contract),
            parsed.token_position,
            authority_id,
            frozen_identity_id,
        );

        // Add optional public note
        if let Some(note) = parsed.public_note {
            builder = builder.with_public_note(note);
        }

        // Add optional group info
        if let Some(group_info) = GroupStateTransitionInfoStatusWasm::try_from_optional_options(
            &options_value,
            "groupInfo",
        )? {
            builder = builder.with_using_group_info(group_info.into());
        }

        // Add settings
        if let Some(settings) = settings {
            builder = builder.with_settings(settings);
        }

        // Add user fee increase from settings
        let user_fee_increase =
            get_user_fee_increase(extract_settings_from_options(&options_value)?.as_ref());
        if user_fee_increase > 0 {
            builder = builder.with_user_fee_increase(user_fee_increase);
        }

        // Use the SDK's token_freeze method
        let result = self
            .inner_sdk()
            .token_freeze(builder, &identity_key, &signer)
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to freeze tokens: {}", e)))?;

        Ok(TokenFreezeResultWasm::from_result(result, contract_id))
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
///
/// The result type depends on token configuration:
/// - Standard tokens: returns unfrozen identity ID
/// - Tokens with history: returns a document
/// - Group-managed tokens: returns group power and status
///
/// Check which optional fields are present to determine the result type.
#[wasm_bindgen(js_name = "TokenUnfreezeResult")]
pub struct TokenUnfreezeResultWasm {
    /// For IdentityInfo result
    unfrozen_identity_id: Option<IdentifierWasm>,
    /// For group actions
    group_power: Option<u32>,
    /// For HistoricalDocument or GroupActionWithDocument - the document
    document: Option<DocumentWasm>,
}

#[wasm_bindgen(js_class = TokenUnfreezeResult)]
impl TokenUnfreezeResultWasm {
    /// The identity ID that was unfrozen.
    #[wasm_bindgen(getter = "unfrozenIdentityId")]
    pub fn unfrozen_identity_id(&self) -> Option<IdentifierWasm> {
        self.unfrozen_identity_id.clone()
    }

    /// The accumulated group power (for group actions).
    #[wasm_bindgen(getter = "groupPower")]
    pub fn group_power(&self) -> Option<u32> {
        self.group_power
    }

    /// The historical document (for tokens with history tracking).
    #[wasm_bindgen(getter)]
    pub fn document(&self) -> Option<DocumentWasm> {
        self.document.clone()
    }
}

impl TokenUnfreezeResultWasm {
    /// Convert from SDK UnfreezeResult with the required contract context
    fn from_result(result: UnfreezeResult, contract_id: Identifier) -> Self {
        match result {
            UnfreezeResult::IdentityInfo(unfrozen_id, _info) => TokenUnfreezeResultWasm {
                unfrozen_identity_id: Some(unfrozen_id.into()),
                group_power: None,
                document: None,
            },
            UnfreezeResult::HistoricalDocument(doc) => TokenUnfreezeResultWasm {
                unfrozen_identity_id: None,
                group_power: None,
                document: Some(document_to_wasm(doc, contract_id, "unfreeze")),
            },
            UnfreezeResult::GroupActionWithDocument(power, doc) => TokenUnfreezeResultWasm {
                unfrozen_identity_id: None,
                group_power: Some(power as u32),
                document: doc.map(|d| document_to_wasm(d, contract_id, "unfreeze")),
            },
            UnfreezeResult::GroupActionWithIdentityInfo(power, _info) => TokenUnfreezeResultWasm {
                unfrozen_identity_id: None,
                group_power: Some(power as u32),
                document: None,
            },
        }
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

        // Fetch and cache the data contract
        let data_contract = self.fetch_and_cache_token_contract(contract_id).await?;

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

        // Build the unfreeze transition using rs-sdk builder
        let mut builder = TokenUnfreezeTransitionBuilder::new(
            Arc::new(data_contract),
            parsed.token_position,
            authority_id,
            frozen_identity_id,
        );

        // Add optional public note
        if let Some(note) = parsed.public_note {
            builder = builder.with_public_note(note);
        }

        // Add optional group info
        if let Some(group_info) = GroupStateTransitionInfoStatusWasm::try_from_optional_options(
            &options_value,
            "groupInfo",
        )? {
            builder = builder.with_using_group_info(group_info.into());
        }

        // Add settings
        if let Some(settings) = settings {
            builder = builder.with_settings(settings);
        }

        // Add user fee increase from settings
        let user_fee_increase =
            get_user_fee_increase(extract_settings_from_options(&options_value)?.as_ref());
        if user_fee_increase > 0 {
            builder = builder.with_user_fee_increase(user_fee_increase);
        }

        // Use the SDK's token_unfreeze_identity method
        let result = self
            .inner_sdk()
            .token_unfreeze_identity(builder, &identity_key, &signer)
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to unfreeze tokens: {}", e)))?;

        Ok(TokenUnfreezeResultWasm::from_result(result, contract_id))
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
///
/// The result type depends on token configuration:
/// - Standard tokens: returns historical document
/// - Group-managed tokens: returns group power and document
///
/// Check which optional fields are present to determine the result type.
#[wasm_bindgen(js_name = "TokenDestroyFrozenResult")]
pub struct TokenDestroyFrozenResultWasm {
    /// For group actions
    group_power: Option<u32>,
    /// For HistoricalDocument or GroupActionWithDocument - the document
    document: Option<DocumentWasm>,
}

#[wasm_bindgen(js_class = TokenDestroyFrozenResult)]
impl TokenDestroyFrozenResultWasm {
    /// The accumulated group power (for group actions).
    #[wasm_bindgen(getter = "groupPower")]
    pub fn group_power(&self) -> Option<u32> {
        self.group_power
    }

    /// The historical document (for tokens with history tracking).
    #[wasm_bindgen(getter)]
    pub fn document(&self) -> Option<DocumentWasm> {
        self.document.clone()
    }
}

impl TokenDestroyFrozenResultWasm {
    /// Convert from SDK DestroyFrozenFundsResult with the required contract context
    fn from_result(result: DestroyFrozenFundsResult, contract_id: Identifier) -> Self {
        match result {
            DestroyFrozenFundsResult::HistoricalDocument(doc) => TokenDestroyFrozenResultWasm {
                group_power: None,
                document: Some(document_to_wasm(doc, contract_id, "destroyFrozenFunds")),
            },
            DestroyFrozenFundsResult::GroupActionWithDocument(power, doc) => {
                TokenDestroyFrozenResultWasm {
                    group_power: Some(power as u32),
                    document: doc.map(|d| document_to_wasm(d, contract_id, "destroyFrozenFunds")),
                }
            }
        }
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

        // Fetch and cache the data contract
        let data_contract = self.fetch_and_cache_token_contract(contract_id).await?;

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

        // Build the destroy frozen transition using rs-sdk builder
        let mut builder = TokenDestroyFrozenFundsTransitionBuilder::new(
            Arc::new(data_contract),
            parsed.token_position,
            authority_id,
            frozen_identity_id,
        );

        // Add optional public note
        if let Some(note) = parsed.public_note {
            builder = builder.with_public_note(note);
        }

        // Add optional group info
        if let Some(group_info) = GroupStateTransitionInfoStatusWasm::try_from_optional_options(
            &options_value,
            "groupInfo",
        )? {
            builder = builder.with_using_group_info(group_info.into());
        }

        // Add settings
        if let Some(settings) = settings {
            builder = builder.with_settings(settings);
        }

        // Add user fee increase from settings
        let user_fee_increase =
            get_user_fee_increase(extract_settings_from_options(&options_value)?.as_ref());
        if user_fee_increase > 0 {
            builder = builder.with_user_fee_increase(user_fee_increase);
        }

        // Use the SDK's token_destroy_frozen_funds method
        let result = self
            .inner_sdk()
            .token_destroy_frozen_funds(builder, &identity_key, &signer)
            .await
            .map_err(|e| {
                WasmSdkError::generic(format!("Failed to destroy frozen tokens: {}", e))
            })?;

        Ok(TokenDestroyFrozenResultWasm::from_result(
            result,
            contract_id,
        ))
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
///
/// The result type depends on token configuration:
/// - Standard tokens: returns document
/// - Group-managed tokens: returns group power and document
///
/// Check which optional fields are present to determine the result type.
#[wasm_bindgen(js_name = "TokenEmergencyActionResult")]
pub struct TokenEmergencyActionResultWasm {
    /// For group actions
    group_power: Option<u32>,
    /// The document
    document: Option<DocumentWasm>,
}

#[wasm_bindgen(js_class = TokenEmergencyActionResult)]
impl TokenEmergencyActionResultWasm {
    /// The accumulated group power (for group actions).
    #[wasm_bindgen(getter = "groupPower")]
    pub fn group_power(&self) -> Option<u32> {
        self.group_power
    }

    /// The document.
    #[wasm_bindgen(getter)]
    pub fn document(&self) -> Option<DocumentWasm> {
        self.document.clone()
    }
}

impl TokenEmergencyActionResultWasm {
    /// Convert from SDK EmergencyActionResult with the required contract context
    fn from_result(result: EmergencyActionResult, contract_id: Identifier) -> Self {
        match result {
            EmergencyActionResult::Document(doc) => TokenEmergencyActionResultWasm {
                group_power: None,
                document: Some(document_to_wasm(doc, contract_id, "emergencyAction")),
            },
            EmergencyActionResult::GroupActionWithDocument(power, doc) => {
                TokenEmergencyActionResultWasm {
                    group_power: Some(power as u32),
                    document: doc.map(|d| document_to_wasm(d, contract_id, "emergencyAction")),
                }
            }
        }
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
        let options_value: JsValue = options.into();

        // Deserialize and validate options
        let parsed = deserialize_token_emergency_action_options(options_value.clone())?;

        // Convert identifiers
        let contract_id: Identifier = parsed.data_contract_id.into();
        let authority_id: Identifier = parsed.authority_id.into();

        // Extract identity key from options
        let identity_key_wasm =
            IdentityPublicKeyWasm::try_from_options(&options_value, "identityKey")?;
        let identity_key: IdentityPublicKey = identity_key_wasm.into();

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options_value)?;

        // Fetch and cache the data contract
        let data_contract = self.fetch_and_cache_token_contract(contract_id).await?;

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

        // Build the emergency action transition using rs-sdk builder
        // Use the appropriate constructor based on the action
        let mut builder = match parsed.action.to_lowercase().as_str() {
            "pause" => TokenEmergencyActionTransitionBuilder::pause(
                Arc::new(data_contract),
                parsed.token_position,
                authority_id,
            ),
            "resume" => TokenEmergencyActionTransitionBuilder::resume(
                Arc::new(data_contract),
                parsed.token_position,
                authority_id,
            ),
            _ => {
                return Err(WasmSdkError::invalid_argument(
                    "action must be 'pause' or 'resume'",
                ))
            }
        };

        // Add optional public note
        if let Some(note) = parsed.public_note {
            builder = builder.with_public_note(note);
        }

        // Add optional group info
        if let Some(group_info) = GroupStateTransitionInfoStatusWasm::try_from_optional_options(
            &options_value,
            "groupInfo",
        )? {
            builder = builder.with_using_group_info(group_info.into());
        }

        // Add settings
        if let Some(settings) = settings {
            builder = builder.with_settings(settings);
        }

        // Add user fee increase from settings
        let user_fee_increase =
            get_user_fee_increase(extract_settings_from_options(&options_value)?.as_ref());
        if user_fee_increase > 0 {
            builder = builder.with_user_fee_increase(user_fee_increase);
        }

        // Use the SDK's token_emergency_action method
        let result = self
            .inner_sdk()
            .token_emergency_action(builder, &identity_key, &signer)
            .await
            .map_err(|e| {
                WasmSdkError::generic(format!("Failed to perform emergency action: {}", e))
            })?;

        Ok(TokenEmergencyActionResultWasm::from_result(
            result,
            contract_id,
        ))
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

fn deserialize_token_claim_options(
    options: JsValue,
) -> Result<TokenClaimOptionsInput, WasmSdkError> {
    deserialize_required_query(options, "Options object is required", "token claim options")
}

/// Result of claiming tokens.
///
/// The result type depends on token configuration:
/// - Standard tokens: returns document
/// - Group-managed tokens: returns group power and document
///
/// Check which optional fields are present to determine the result type.
#[wasm_bindgen(js_name = "TokenClaimResult")]
pub struct TokenClaimResultWasm {
    /// For group actions
    group_power: Option<u32>,
    /// The document
    document: Option<DocumentWasm>,
}

#[wasm_bindgen(js_class = TokenClaimResult)]
impl TokenClaimResultWasm {
    /// The accumulated group power (for group actions).
    #[wasm_bindgen(getter = "groupPower")]
    pub fn group_power(&self) -> Option<u32> {
        self.group_power
    }

    /// The document.
    #[wasm_bindgen(getter)]
    pub fn document(&self) -> Option<DocumentWasm> {
        self.document.clone()
    }
}

impl TokenClaimResultWasm {
    /// Convert from SDK ClaimResult with the required contract context
    fn from_result(result: ClaimResult, contract_id: Identifier) -> Self {
        match result {
            ClaimResult::Document(doc) => TokenClaimResultWasm {
                group_power: None,
                document: Some(document_to_wasm(doc, contract_id, "claim")),
            },
            ClaimResult::GroupActionWithDocument(power, doc) => TokenClaimResultWasm {
                group_power: Some(power as u32),
                document: Some(document_to_wasm(doc, contract_id, "claim")),
            },
        }
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
        let data_contract = self.fetch_and_cache_token_contract(contract_id).await?;

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

        // Build the claim transition using rs-sdk builder
        let mut builder = TokenClaimTransitionBuilder::new(
            Arc::new(data_contract),
            parsed.token_position,
            identity_id,
            distribution_type,
        );

        // Add optional public note
        if let Some(note) = parsed.public_note {
            builder = builder.with_public_note(note);
        }

        // Add settings
        if let Some(settings) = settings {
            builder = builder.with_settings(settings);
        }

        // Add user fee increase from settings
        let user_fee_increase =
            get_user_fee_increase(extract_settings_from_options(&options_value)?.as_ref());
        if user_fee_increase > 0 {
            builder = builder.with_user_fee_increase(user_fee_increase);
        }

        // Use the SDK's token_claim method
        let result = self
            .inner_sdk()
            .token_claim(builder, &identity_key, &signer)
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to claim tokens: {}", e)))?;

        Ok(TokenClaimResultWasm::from_result(result, contract_id))
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
///
/// The result type depends on token configuration:
/// - Standard tokens: returns pricing schedule
/// - Tokens with history: returns document
/// - Group-managed tokens: returns group power and status
///
/// Check which optional fields are present to determine the result type.
#[wasm_bindgen(js_name = "TokenSetPriceResult")]
pub struct TokenSetPriceResultWasm {
    /// For group actions
    group_power: Option<u32>,
    /// Group action status
    group_action_status: Option<String>,
    /// For HistoricalDocument or GroupActionWithDocument - the document
    document: Option<DocumentWasm>,
}

#[wasm_bindgen(js_class = TokenSetPriceResult)]
impl TokenSetPriceResultWasm {
    /// The accumulated group power (for group actions).
    #[wasm_bindgen(getter = "groupPower")]
    pub fn group_power(&self) -> Option<u32> {
        self.group_power
    }

    /// The group action status (for group actions).
    #[wasm_bindgen(getter = "groupActionStatus")]
    pub fn group_action_status(&self) -> Option<String> {
        self.group_action_status.clone()
    }

    /// The historical document (for tokens with history tracking).
    #[wasm_bindgen(getter)]
    pub fn document(&self) -> Option<DocumentWasm> {
        self.document.clone()
    }
}

impl TokenSetPriceResultWasm {
    /// Convert from SDK SetPriceResult with the required contract context
    fn from_result(result: SetPriceResult, contract_id: Identifier) -> Self {
        match result {
            SetPriceResult::PricingSchedule(_owner_id, _schedule) => TokenSetPriceResultWasm {
                group_power: None,
                group_action_status: None,
                document: None,
            },
            SetPriceResult::HistoricalDocument(doc) => TokenSetPriceResultWasm {
                group_power: None,
                group_action_status: None,
                document: Some(document_to_wasm(doc, contract_id, "directPricing")),
            },
            SetPriceResult::GroupActionWithDocument(power, doc) => TokenSetPriceResultWasm {
                group_power: Some(power as u32),
                group_action_status: None,
                document: doc.map(|d| document_to_wasm(d, contract_id, "directPricing")),
            },
            SetPriceResult::GroupActionWithPricingSchedule(power, status, _schedule) => {
                TokenSetPriceResultWasm {
                    group_power: Some(power as u32),
                    group_action_status: Some(format!("{:?}", status)),
                    document: None,
                }
            }
        }
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

        // Fetch and cache the data contract
        let data_contract = self.fetch_and_cache_token_contract(contract_id).await?;

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

        // Build the set price transition using rs-sdk builder
        let mut builder = TokenChangeDirectPurchasePriceTransitionBuilder::new(
            Arc::new(data_contract),
            parsed.token_position,
            authority_id,
        );

        // Set pricing schedule
        if let Some(schedule) = pricing_schedule {
            builder = builder.with_token_pricing_schedule(schedule);
        }

        // Add optional public note
        if let Some(note) = parsed.public_note {
            builder = builder.with_public_note(note);
        }

        // Add optional group info
        if let Some(group_info) = GroupStateTransitionInfoStatusWasm::try_from_optional_options(
            &options_value,
            "groupInfo",
        )? {
            builder = builder.with_using_group_info(group_info.into());
        }

        // Add settings
        if let Some(settings) = settings {
            builder = builder.with_settings(settings);
        }

        // Add user fee increase from settings
        let user_fee_increase =
            get_user_fee_increase(extract_settings_from_options(&options_value)?.as_ref());
        if user_fee_increase > 0 {
            builder = builder.with_user_fee_increase(user_fee_increase);
        }

        // Use the SDK's token_set_price_for_direct_purchase method
        let result = self
            .inner_sdk()
            .token_set_price_for_direct_purchase(builder, &identity_key, &signer)
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to set token price: {}", e)))?;

        Ok(TokenSetPriceResultWasm::from_result(result, contract_id))
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
///
/// The result type depends on token configuration:
/// - Standard tokens: returns balance
/// - Tokens with history: returns document
/// - Group-managed tokens: returns group power and document
///
/// Check which optional fields are present to determine the result type.
#[wasm_bindgen(js_name = "TokenDirectPurchaseResult")]
pub struct TokenDirectPurchaseResultWasm {
    /// For TokenBalance result
    buyer_id: Option<IdentifierWasm>,
    /// New balance after purchase
    new_balance: Option<u64>,
    /// For group actions
    group_power: Option<u32>,
    /// For HistoricalDocument or GroupActionWithDocument - the document
    document: Option<DocumentWasm>,
}

#[wasm_bindgen(js_class = TokenDirectPurchaseResult)]
impl TokenDirectPurchaseResultWasm {
    /// The buyer's identity ID.
    #[wasm_bindgen(getter = "buyerId")]
    pub fn buyer_id(&self) -> Option<IdentifierWasm> {
        self.buyer_id.clone()
    }

    /// The buyer's new balance after purchase.
    #[wasm_bindgen(getter = "newBalance")]
    pub fn new_balance(&self) -> Option<BigInt> {
        self.new_balance.map(BigInt::from)
    }

    /// The accumulated group power (for group actions).
    #[wasm_bindgen(getter = "groupPower")]
    pub fn group_power(&self) -> Option<u32> {
        self.group_power
    }

    /// The historical document (for tokens with history tracking).
    #[wasm_bindgen(getter)]
    pub fn document(&self) -> Option<DocumentWasm> {
        self.document.clone()
    }
}

impl TokenDirectPurchaseResultWasm {
    /// Convert from SDK DirectPurchaseResult with the required contract context
    fn from_result(result: DirectPurchaseResult, contract_id: Identifier) -> Self {
        match result {
            DirectPurchaseResult::TokenBalance(buyer_id, balance) => {
                TokenDirectPurchaseResultWasm {
                    buyer_id: Some(buyer_id.into()),
                    new_balance: Some(balance),
                    group_power: None,
                    document: None,
                }
            }
            DirectPurchaseResult::HistoricalDocument(doc) => TokenDirectPurchaseResultWasm {
                buyer_id: None,
                new_balance: None,
                group_power: None,
                document: Some(document_to_wasm(doc, contract_id, "directPurchase")),
            },
            DirectPurchaseResult::GroupActionWithDocument(power, doc) => {
                TokenDirectPurchaseResultWasm {
                    buyer_id: None,
                    new_balance: None,
                    group_power: Some(power as u32),
                    document: doc.map(|d| document_to_wasm(d, contract_id, "directPurchase")),
                }
            }
        }
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
        let data_contract = self.fetch_and_cache_token_contract(contract_id).await?;

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

        // Build the direct purchase transition using rs-sdk builder
        let mut builder = TokenDirectPurchaseTransitionBuilder::new(
            Arc::new(data_contract),
            parsed.token_position,
            buyer_id,
            amount,
            max_total_cost,
        );

        // Add settings
        if let Some(settings) = settings {
            builder = builder.with_settings(settings);
        }

        // Add user fee increase from settings
        let user_fee_increase =
            get_user_fee_increase(extract_settings_from_options(&options_value)?.as_ref());
        if user_fee_increase > 0 {
            builder = builder.with_user_fee_increase(user_fee_increase);
        }

        // Use the SDK's token_purchase method
        let result = self
            .inner_sdk()
            .token_purchase(builder, &identity_key, &signer)
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to purchase tokens: {}", e)))?;

        Ok(TokenDirectPurchaseResultWasm::from_result(
            result,
            contract_id,
        ))
    }
}
