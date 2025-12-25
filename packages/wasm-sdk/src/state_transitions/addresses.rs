//! Address funds state transition implementations for the WASM SDK.
//!
//! This module provides WASM bindings for address fund operations like transfers and withdrawals.

use crate::error::WasmSdkError;
use crate::queries::address::PlatformAddressInfoWasm;
use crate::queries::utils::deserialize_required_query;
use crate::sdk::WasmSdk;
use crate::settings::{parse_put_settings, PutSettingsJs};
use dash_sdk::dpp::identity::core_script::CoreScript;
use dash_sdk::platform::transition::address_credit_withdrawal::WithdrawAddressFunds;
use dash_sdk::platform::transition::top_up_identity_from_addresses::TopUpIdentityFromAddresses;
use dash_sdk::platform::transition::transfer_address_funds::TransferAddressFunds;
use dash_sdk::platform::transition::transfer_to_addresses::TransferToAddresses;
use dash_sdk::platform::{Fetch, Identifier, Identity};
use js_sys::{BigInt, Map};
use serde::Deserialize;
use wasm_bindgen::prelude::*;
use wasm_dpp2::identifier::IdentifierWasm;
use wasm_dpp2::utils::IntoWasm;
use wasm_dpp2::{
    fee_strategy_from_steps_or_default, outputs_to_btree_map, outputs_to_optional_btree_map,
    CoreScriptWasm, FeeStrategyStepWasm, IdentitySignerWasm, PlatformAddressOutputWasm,
    PlatformAddressSignerWasm, PlatformAddressWasm, PoolingWasm,
};

/// Main input struct for address funds transfer options.
/// Uses types from wasm-dpp2 for inputs, outputs, and fee strategy.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddressFundsTransferOptionsInput {
    inputs: Vec<PlatformAddressOutputWasm>,
    outputs: Vec<PlatformAddressOutputWasm>,
    #[serde(default)]
    fee_strategy: Option<Vec<FeeStrategyStepWasm>>,
}

fn deserialize_transfer_options(
    options: JsValue,
) -> Result<AddressFundsTransferOptionsInput, WasmSdkError> {
    let parsed: AddressFundsTransferOptionsInput = deserialize_required_query(
        options,
        "Options object is required",
        "address funds transfer options",
    )?;

    if parsed.inputs.is_empty() {
        return Err(WasmSdkError::invalid_argument(
            "At least one input is required",
        ));
    }

    if parsed.outputs.is_empty() {
        return Err(WasmSdkError::invalid_argument(
            "At least one output is required",
        ));
    }

    Ok(parsed)
}

/// Extracts PutSettings from the 'settings' field of an options object.
fn extract_settings_from_options(
    options: &JsValue,
) -> Result<Option<dash_sdk::platform::transition::put_settings::PutSettings>, WasmSdkError> {
    let settings_js = js_sys::Reflect::get(options, &JsValue::from_str("settings"))
        .map_err(|e| WasmSdkError::generic(format!("Failed to extract settings: {:?}", e)))?;

    if settings_js.is_undefined() || settings_js.is_null() {
        return Ok(None);
    }

    // Convert JsValue to PutSettingsJs and parse
    let settings_typed: PutSettingsJs = settings_js.into();
    Ok(parse_put_settings(Some(settings_typed)))
}

/// TypeScript interface for address transfer options
#[wasm_bindgen(typescript_custom_section)]
const ADDRESS_TRANSFER_OPTIONS_TS: &'static str = r#"
/**
 * Options for transferring funds between Platform addresses.
 */
export interface AddressFundsTransferOptions {
  /**
   * Array of input addresses with amounts to spend.
   * Use PlatformAddressInput for typed inputs (nonces fetched automatically).
   */
  inputs: PlatformAddressInput[];

  /**
   * Array of output addresses with amounts to receive.
   * Use PlatformAddressOutput for typed outputs.
   */
  outputs: PlatformAddressOutput[];

  /**
   * Signer containing private keys for all input addresses.
   * Use PlatformAddressSigner to add keys before calling transfer.
   */
  signer: PlatformAddressSigner;

  /**
   * Fee strategy defining how transaction fees are paid.
   * Array of FeeStrategyStep, each specifying to deduct from an input or reduce an output.
   * @default [FeeStrategyStep.deductFromInput(0)]
   */
  feeStrategy?: FeeStrategyStep[];

  /**
   * Optional settings for the broadcast operation.
   * Includes retries, timeouts, userFeeIncrease, etc.
   */
  settings?: PutSettings;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "AddressFundsTransferOptions")]
    pub type AddressFundsTransferOptionsJs;
}

#[wasm_bindgen]
impl WasmSdk {
    /// Transfers credits between Platform addresses.
    ///
    /// This method handles the complete transfer flow:
    /// 1. Fetches current nonces for all input addresses
    /// 2. Builds and signs the transfer transition
    /// 3. Broadcasts and waits for confirmation
    ///
    /// @param options - Transfer options including inputs, outputs, and private keys
    /// @returns Promise resolving to Map of PlatformAddress to PlatformAddressInfo
    #[wasm_bindgen(
        js_name = "addressFundsTransfer",
        unchecked_return_type = "Map<PlatformAddress, PlatformAddressInfo>"
    )]
    pub async fn address_funds_transfer(
        &self,
        options: AddressFundsTransferOptionsJs,
    ) -> Result<Map, WasmSdkError> {
        let options_value: JsValue = options.into();

        // Deserialize and validate options
        let parsed = deserialize_transfer_options(options_value.clone())?;

        // Convert inputs and outputs to maps
        let inputs_map = outputs_to_btree_map(parsed.inputs);
        let outputs_map = outputs_to_btree_map(parsed.outputs);

        // Extract signer from options
        let signer = PlatformAddressSignerWasm::try_from_options(&options_value)?;

        // Convert fee strategy from input using wasm-dpp2 helper
        let fee_strategy = fee_strategy_from_steps_or_default(parsed.fee_strategy);

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

        // Use the SDK's transfer_address_funds method which handles nonces, building, and broadcasting
        let address_infos = self
            .inner_sdk()
            .transfer_address_funds(inputs_map, outputs_map, fee_strategy, &signer, settings)
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to transfer funds: {}", e)))?;

        // Convert to Map<PlatformAddress, PlatformAddressInfo>
        let results_map = Map::new();

        for (address, info_opt) in address_infos {
            let info = info_opt.ok_or_else(|| {
                WasmSdkError::generic(format!("Address {} has no info after transfer", address))
            })?;
            let key = JsValue::from(PlatformAddressWasm::from(address));
            let value = JsValue::from(PlatformAddressInfoWasm::from(info));
            results_map.set(&key, &value);
        }

        Ok(results_map)
    }
}

/// Main input struct for identity top up from addresses options.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdentityTopUpFromAddressesOptionsInput {
    identity_id: IdentifierWasm,
    inputs: Vec<PlatformAddressOutputWasm>,
}

fn deserialize_identity_top_up_options(
    options: JsValue,
) -> Result<IdentityTopUpFromAddressesOptionsInput, WasmSdkError> {
    let parsed: IdentityTopUpFromAddressesOptionsInput = deserialize_required_query(
        options,
        "Options object is required",
        "identity top up from addresses options",
    )?;

    if parsed.inputs.is_empty() {
        return Err(WasmSdkError::invalid_argument(
            "At least one input is required",
        ));
    }

    Ok(parsed)
}

/// Result of topping up an identity from Platform addresses.
#[wasm_bindgen(js_name = "IdentityTopUpFromAddressesResult")]
pub struct IdentityTopUpFromAddressesResultWasm {
    address_infos: Map,
    new_balance: u64,
}

#[wasm_bindgen(js_class = IdentityTopUpFromAddressesResult)]
impl IdentityTopUpFromAddressesResultWasm {
    /// Map of addresses to their updated info after the top up.
    #[wasm_bindgen(getter = "addressInfos")]
    pub fn address_infos(&self) -> Map {
        self.address_infos.clone()
    }

    /// New balance of the identity after top up.
    #[wasm_bindgen(getter = "newBalance")]
    pub fn new_balance(&self) -> BigInt {
        BigInt::from(self.new_balance)
    }
}

/// TypeScript interface for identity top up from addresses options
#[wasm_bindgen(typescript_custom_section)]
const IDENTITY_TOP_UP_OPTIONS_TS: &'static str = r#"
/**
 * Options for topping up an identity from Platform addresses.
 */
export interface IdentityTopUpFromAddressesOptions {
  /**
   * The identity ID to top up.
   */
  identityId: Identifier;

  /**
   * Array of input addresses with amounts to use for top up.
   * Use PlatformAddressInput for typed inputs (nonces fetched automatically).
   */
  inputs: PlatformAddressInput[];

  /**
   * Signer containing private keys for all input addresses.
   * Use PlatformAddressSigner to add keys before calling top up.
   */
  signer: PlatformAddressSigner;

  /**
   * Optional settings for the broadcast operation.
   * Includes retries, timeouts, userFeeIncrease, etc.
   */
  settings?: PutSettings;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IdentityTopUpFromAddressesOptions")]
    pub type IdentityTopUpFromAddressesOptionsJs;
}

#[wasm_bindgen]
impl WasmSdk {
    /// Top up an identity from Platform addresses.
    ///
    /// This method handles the complete top up flow:
    /// 1. Fetches the identity from Platform
    /// 2. Fetches current nonces for all input addresses
    /// 3. Builds and signs the identity top up transition
    /// 4. Broadcasts and waits for confirmation
    ///
    /// @param options - Top up options including identity ID, inputs, and signer
    /// @returns Promise resolving to result with updated address infos and new identity balance
    #[wasm_bindgen(js_name = "identityTopUpFromAddresses")]
    pub async fn identity_top_up_from_addresses(
        &self,
        options: IdentityTopUpFromAddressesOptionsJs,
    ) -> Result<IdentityTopUpFromAddressesResultWasm, WasmSdkError> {
        let options_value: JsValue = options.into();

        // Deserialize and validate options
        let parsed = deserialize_identity_top_up_options(options_value.clone())?;

        // Convert identity ID
        let identity_id: Identifier = parsed.identity_id.into();

        // Fetch the identity
        let identity = Identity::fetch_by_identifier(self.as_ref(), identity_id)
            .await?
            .ok_or_else(|| {
                WasmSdkError::not_found(format!("Identity {} not found", identity_id))
            })?;

        // Convert inputs to map
        let inputs_map = outputs_to_btree_map(parsed.inputs);

        // Extract signer from options
        let signer = PlatformAddressSignerWasm::try_from_options(&options_value)?;

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

        // Use the SDK's top_up_from_addresses method
        let (address_infos, new_balance) = identity
            .top_up_from_addresses(self.inner_sdk(), inputs_map, &signer, settings)
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to top up identity: {}", e)))?;

        // Convert to Map<PlatformAddress, PlatformAddressInfo>
        let address_infos_map = Map::new();

        for (address, info_opt) in address_infos {
            let info = info_opt.ok_or_else(|| {
                WasmSdkError::generic(format!("Address {} has no info after top up", address))
            })?;
            let key = JsValue::from(PlatformAddressWasm::from(address));
            let value = JsValue::from(PlatformAddressInfoWasm::from(info));
            address_infos_map.set(&key, &value);
        }

        Ok(IdentityTopUpFromAddressesResultWasm {
            address_infos: address_infos_map,
            new_balance,
        })
    }
}

/// Main input struct for address funds withdraw options.
/// Uses types from wasm-dpp2 for inputs, change output, and fee strategy.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddressFundsWithdrawOptionsInput {
    inputs: Vec<PlatformAddressOutputWasm>,
    #[serde(default)]
    change_output: Option<PlatformAddressOutputWasm>,
    #[serde(default)]
    fee_strategy: Option<Vec<FeeStrategyStepWasm>>,
    core_fee_per_byte: u32,
    pooling: PoolingWasm,
}

fn deserialize_withdraw_options(
    options: JsValue,
) -> Result<AddressFundsWithdrawOptionsInput, WasmSdkError> {
    let parsed: AddressFundsWithdrawOptionsInput = deserialize_required_query(
        options,
        "Options object is required",
        "address funds withdraw options",
    )?;

    if parsed.inputs.is_empty() {
        return Err(WasmSdkError::invalid_argument(
            "At least one input is required",
        ));
    }

    Ok(parsed)
}

/// TypeScript interface for address withdraw options
#[wasm_bindgen(typescript_custom_section)]
const ADDRESS_WITHDRAW_OPTIONS_TS: &'static str = r#"
/**
 * Options for withdrawing Platform address credits to Core (L1).
 */
export interface AddressFundsWithdrawOptions {
  /**
   * Array of input addresses with amounts to withdraw.
   * Use PlatformAddressInput for typed inputs (nonces fetched automatically).
   */
  inputs: PlatformAddressInput[];

  /**
   * Optional change output address and amount.
   * If provided, specifies where to send any change from the withdrawal.
   */
  changeOutput?: PlatformAddressOutput;

  /**
   * Fee strategy defining how transaction fees are paid.
   * Array of FeeStrategyStep, each specifying to deduct from an input or reduce an output.
   * @default [FeeStrategyStep.deductFromInput(0)]
   */
  feeStrategy?: FeeStrategyStep[];

  /**
   * Core (L1) fee per byte for the withdrawal transaction.
   * This determines the mining fee for the Core blockchain transaction.
   */
  coreFeePerByte: number;

  /**
   * Pooling strategy for the withdrawal.
   * - Pooling.Never: Create individual withdrawal transaction
   * - Pooling.IfAvailable: Join pool if available, otherwise individual
   * - Pooling.Standard: Wait to join pool (may take longer)
   */
  pooling: Pooling;

  /**
   * Core output script specifying the L1 destination address.
   * Use CoreScript.newP2PKH() or CoreScript.newP2SH() to create.
   */
  outputScript: CoreScript;

  /**
   * Signer containing private keys for all input addresses.
   * Use PlatformAddressSigner to add keys before calling withdraw.
   */
  signer: PlatformAddressSigner;

  /**
   * Optional settings for the broadcast operation.
   * Includes retries, timeouts, userFeeIncrease, etc.
   */
  settings?: PutSettings;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "AddressFundsWithdrawOptions")]
    pub type AddressFundsWithdrawOptionsJs;
}

#[wasm_bindgen]
impl WasmSdk {
    /// Withdraws Platform address credits to Core (L1).
    ///
    /// This method handles the complete withdrawal flow:
    /// 1. Fetches current nonces for all input addresses
    /// 2. Builds and signs the withdrawal transition
    /// 3. Broadcasts and waits for confirmation
    /// 4. The withdrawal may be pooled with others depending on the pooling strategy
    ///
    /// @param options - Withdrawal options including inputs, output script, and private keys
    /// @returns Promise resolving to Map of PlatformAddress to PlatformAddressInfo
    #[wasm_bindgen(
        js_name = "addressFundsWithdraw",
        unchecked_return_type = "Map<PlatformAddress, PlatformAddressInfo>"
    )]
    pub async fn address_funds_withdraw(
        &self,
        options: AddressFundsWithdrawOptionsJs,
    ) -> Result<Map, WasmSdkError> {
        let options_value: JsValue = options.into();

        // Deserialize and validate options
        let parsed = deserialize_withdraw_options(options_value.clone())?;

        // Convert inputs to map
        let inputs_map = outputs_to_btree_map(parsed.inputs);

        // Convert change output if provided
        let change_output = parsed.change_output.map(|output| output.into_inner());

        // Extract output script from options
        let output_script_js =
            js_sys::Reflect::get(&options_value, &JsValue::from_str("outputScript"))
                .map_err(|_| WasmSdkError::invalid_argument("outputScript is required"))?;

        let output_script: CoreScript = output_script_js
            .to_wasm::<CoreScriptWasm>("CoreScript")?
            .clone()
            .into();

        // Extract signer from options
        let signer = PlatformAddressSignerWasm::try_from_options(&options_value)?;

        // Convert fee strategy from input using wasm-dpp2 helper
        let fee_strategy = fee_strategy_from_steps_or_default(parsed.fee_strategy);

        // Convert pooling
        let pooling = parsed.pooling.into();

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

        // Use the SDK's withdraw_address_funds method which handles nonces, building, and broadcasting
        let address_infos = self
            .inner_sdk()
            .withdraw_address_funds(
                inputs_map,
                change_output,
                fee_strategy,
                parsed.core_fee_per_byte,
                pooling,
                output_script,
                &signer,
                settings,
            )
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to withdraw funds: {}", e)))?;

        // Convert to Map<PlatformAddress, PlatformAddressInfo>
        let results_map = Map::new();

        for (address, info_opt) in address_infos {
            let info = info_opt.ok_or_else(|| {
                WasmSdkError::generic(format!("Address {} has no info after withdrawal", address))
            })?;
            let key = JsValue::from(PlatformAddressWasm::from(address));
            let value = JsValue::from(PlatformAddressInfoWasm::from(info));
            results_map.set(&key, &value);
        }

        Ok(results_map)
    }

    /// Transfer credits from an identity to Platform addresses.
    ///
    /// This method handles the complete transfer flow:
    /// 1. Fetches the identity from Platform
    /// 2. Finds the appropriate transfer key to use for signing (if signingTransferKeyId specified)
    /// 3. Builds and signs the identity credit transfer to addresses transition
    /// 4. Broadcasts and waits for confirmation
    ///
    /// @param options - Transfer options including identity ID, outputs, and signer
    /// @returns Promise resolving to result with updated address infos and new identity balance
    #[wasm_bindgen(js_name = "identityTransferToAddresses")]
    pub async fn identity_transfer_to_addresses(
        &self,
        options: IdentityTransferToAddressesOptionsJs,
    ) -> Result<IdentityTransferToAddressesResultWasm, WasmSdkError> {
        let options_value: JsValue = options.into();

        // Deserialize and validate options
        let parsed = deserialize_identity_transfer_options(options_value.clone())?;

        // Convert identity ID
        let identity_id: Identifier = parsed.identity_id.into();

        // Fetch the identity
        let identity = Identity::fetch_by_identifier(self.as_ref(), identity_id)
            .await?
            .ok_or_else(|| {
                WasmSdkError::not_found(format!("Identity {} not found", identity_id))
            })?;

        // Convert outputs to map (recipient addresses with amounts)
        let outputs_map = outputs_to_btree_map(parsed.outputs);

        // Get the signing key if a specific key ID was requested
        use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
        let signing_key = parsed
            .signing_transfer_key_id
            .map(|key_id| {
                identity.get_public_key_by_id(key_id).ok_or_else(|| {
                    WasmSdkError::not_found(format!("Key with ID {} not found", key_id))
                })
            })
            .transpose()?;

        // Extract signer from options
        let signer = IdentitySignerWasm::try_from_options(&options_value)?;

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

        // Use the SDK's transfer_credits_to_addresses method
        let (address_infos, new_balance) = identity
            .transfer_credits_to_addresses(
                self.inner_sdk(),
                outputs_map,
                signing_key,
                &signer,
                settings,
            )
            .await
            .map_err(|e| {
                WasmSdkError::generic(format!("Failed to transfer credits to addresses: {}", e))
            })?;

        // Convert to Map<PlatformAddress, PlatformAddressInfo>
        let address_infos_map = Map::new();

        for (address, info_opt) in address_infos {
            let info = info_opt.ok_or_else(|| {
                WasmSdkError::generic(format!("Address {} has no info after transfer", address))
            })?;
            let key = JsValue::from(PlatformAddressWasm::from(address));
            let value = JsValue::from(PlatformAddressInfoWasm::from(info));
            address_infos_map.set(&key, &value);
        }

        Ok(IdentityTransferToAddressesResultWasm {
            address_infos: address_infos_map,
            new_balance,
        })
    }
}

/// Main input struct for identity transfer to addresses options.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdentityTransferToAddressesOptionsInput {
    identity_id: IdentifierWasm,
    outputs: Vec<PlatformAddressOutputWasm>,
    #[serde(default)]
    signing_transfer_key_id: Option<u32>,
}

fn deserialize_identity_transfer_options(
    options: JsValue,
) -> Result<IdentityTransferToAddressesOptionsInput, WasmSdkError> {
    let parsed: IdentityTransferToAddressesOptionsInput = deserialize_required_query(
        options,
        "Options object is required",
        "identity transfer to addresses options",
    )?;

    if parsed.outputs.is_empty() {
        return Err(WasmSdkError::invalid_argument(
            "At least one output is required",
        ));
    }

    Ok(parsed)
}

/// Result of transferring credits from an identity to Platform addresses.
#[wasm_bindgen(js_name = "IdentityTransferToAddressesResult")]
pub struct IdentityTransferToAddressesResultWasm {
    address_infos: Map,
    new_balance: u64,
}

#[wasm_bindgen(js_class = IdentityTransferToAddressesResult)]
impl IdentityTransferToAddressesResultWasm {
    /// Map of addresses to their updated info after the transfer.
    #[wasm_bindgen(getter = "addressInfos")]
    pub fn address_infos(&self) -> Map {
        self.address_infos.clone()
    }

    /// New balance of the identity after transfer.
    #[wasm_bindgen(getter = "newBalance")]
    pub fn new_balance(&self) -> BigInt {
        BigInt::from(self.new_balance)
    }
}

/// TypeScript interface for identity transfer to addresses options
#[wasm_bindgen(typescript_custom_section)]
const IDENTITY_TRANSFER_OPTIONS_TS: &'static str = r#"
/**
 * Options for transferring credits from an identity to Platform addresses.
 */
export interface IdentityTransferToAddressesOptions {
  /**
   * The identity ID to transfer credits from.
   */
  identityId: Identifier;

  /**
   * Array of output addresses with amounts to receive.
   * Use PlatformAddressOutput for typed outputs.
   */
  outputs: PlatformAddressOutput[];

  /**
   * Signer containing the private key(s) for signing with identity transfer key(s).
   * Use IdentitySigner to add keys before calling transfer.
   */
  signer: IdentitySigner;

  /**
   * Optional key ID to use for signing.
   * If not specified, will auto-select a matching transfer key.
   */
  signingTransferKeyId?: number;

  /**
   * Optional settings for the broadcast operation.
   * Includes retries, timeouts, userFeeIncrease, etc.
   */
  settings?: PutSettings;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IdentityTransferToAddressesOptions")]
    pub type IdentityTransferToAddressesOptionsJs;
}

// ============================================================================
// Address Funding from Asset Lock
// ============================================================================

/// Main input struct for address funding from asset lock options.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddressFundingFromAssetLockOptionsInput {
    outputs: Vec<PlatformAddressOutputWasm>,
    #[serde(default)]
    fee_strategy: Option<Vec<FeeStrategyStepWasm>>,
}

fn deserialize_address_funding_options(
    options: JsValue,
) -> Result<AddressFundingFromAssetLockOptionsInput, WasmSdkError> {
    let parsed: AddressFundingFromAssetLockOptionsInput = deserialize_required_query(
        options,
        "Options object is required",
        "address funding from asset lock options",
    )?;

    if parsed.outputs.is_empty() {
        return Err(WasmSdkError::invalid_argument(
            "At least one output is required",
        ));
    }

    Ok(parsed)
}

/// TypeScript interface for address funding from asset lock options
#[wasm_bindgen(typescript_custom_section)]
const ADDRESS_FUNDING_OPTIONS_TS: &'static str = r#"
/**
 * Options for funding Platform addresses from an asset lock.
 */
export interface AddressFundingFromAssetLockOptions {
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
   * Array of output addresses with amounts to fund.
   * Use PlatformAddressOutput for typed outputs.
   */
  outputs: PlatformAddressOutput[];

  /**
   * Signer containing private keys for all output addresses.
   * Use PlatformAddressSigner to add keys before calling fund.
   */
  signer: PlatformAddressSigner;

  /**
   * Fee strategy defining how transaction fees are paid.
   * Array of FeeStrategyStep, each specifying to deduct from an input or reduce an output.
   * @default [FeeStrategyStep.deductFromInput(0)]
   */
  feeStrategy?: FeeStrategyStep[];

  /**
   * Optional settings for the broadcast operation.
   * Includes retries, timeouts, userFeeIncrease, etc.
   */
  settings?: PutSettings;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "AddressFundingFromAssetLockOptions")]
    pub type AddressFundingFromAssetLockOptionsJs;
}

#[wasm_bindgen]
impl WasmSdk {
    /// Fund Platform addresses from an asset lock.
    ///
    /// This method handles the complete funding flow:
    /// 1. Validates the asset lock proof
    /// 2. Builds and signs the address funding transition
    /// 3. Broadcasts and waits for confirmation
    ///
    /// @param options - Funding options including asset lock, outputs, and signer
    /// @returns Promise resolving to Map of PlatformAddress to PlatformAddressInfo
    #[wasm_bindgen(
        js_name = "addressFundingFromAssetLock",
        unchecked_return_type = "Map<PlatformAddress, PlatformAddressInfo>"
    )]
    pub async fn address_funding_from_asset_lock(
        &self,
        options: AddressFundingFromAssetLockOptionsJs,
    ) -> Result<Map, WasmSdkError> {
        use dash_sdk::platform::transition::top_up_address::TopUpAddress;
        use wasm_dpp2::asset_lock_proof::AssetLockProofWasm;
        use wasm_dpp2::private_key::PrivateKeyWasm;
        use wasm_dpp2::utils::IntoWasm;

        let options_value: JsValue = options.into();

        // Deserialize and validate options
        let parsed = deserialize_address_funding_options(options_value.clone())?;

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

        // Convert outputs to map (address -> optional amount)
        let outputs_map = outputs_to_optional_btree_map(parsed.outputs);

        // Extract signer from options
        let signer = PlatformAddressSignerWasm::try_from_options(&options_value)?;

        // Convert fee strategy from input using wasm-dpp2 helper
        let fee_strategy = fee_strategy_from_steps_or_default(parsed.fee_strategy);

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

        // Use the SDK's top_up method for addresses
        let address_infos = outputs_map
            .top_up(
                self.inner_sdk(),
                asset_lock_proof,
                asset_lock_private_key,
                fee_strategy,
                &signer,
                settings,
            )
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to fund addresses: {}", e)))?;

        // Convert to Map<PlatformAddress, PlatformAddressInfo>
        let results_map = Map::new();

        for (address, info_opt) in address_infos {
            let info = info_opt.ok_or_else(|| {
                WasmSdkError::generic(format!("Address {} has no info after funding", address))
            })?;
            let key = JsValue::from(PlatformAddressWasm::from(address));
            let value = JsValue::from(PlatformAddressInfoWasm::from(info));
            results_map.set(&key, &value);
        }

        Ok(results_map)
    }
}

// ============================================================================
// Identity Create from Platform Addresses
// ============================================================================

/// Main input struct for identity create from addresses options.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdentityCreateFromAddressesOptionsInput {
    inputs: Vec<PlatformAddressOutputWasm>,
    #[serde(default)]
    change_output: Option<PlatformAddressOutputWasm>,
}

fn deserialize_identity_create_options(
    options: JsValue,
) -> Result<IdentityCreateFromAddressesOptionsInput, WasmSdkError> {
    let parsed: IdentityCreateFromAddressesOptionsInput = deserialize_required_query(
        options,
        "Options object is required",
        "identity create from addresses options",
    )?;

    if parsed.inputs.is_empty() {
        return Err(WasmSdkError::invalid_argument(
            "At least one input is required",
        ));
    }

    Ok(parsed)
}

/// Result of creating an identity from Platform addresses.
#[wasm_bindgen(js_name = "IdentityCreateFromAddressesResult")]
pub struct IdentityCreateFromAddressesResultWasm {
    identity: wasm_dpp2::IdentityWasm,
    address_infos: Map,
}

#[wasm_bindgen(js_class = IdentityCreateFromAddressesResult)]
impl IdentityCreateFromAddressesResultWasm {
    /// The newly created identity.
    #[wasm_bindgen(getter)]
    pub fn identity(&self) -> wasm_dpp2::IdentityWasm {
        self.identity.clone()
    }

    /// Map of addresses to their updated info after the identity creation.
    #[wasm_bindgen(getter = "addressInfos")]
    pub fn address_infos(&self) -> Map {
        self.address_infos.clone()
    }
}

/// TypeScript interface for identity create from addresses options
#[wasm_bindgen(typescript_custom_section)]
const IDENTITY_CREATE_FROM_ADDRESSES_OPTIONS_TS: &'static str = r#"
/**
 * Options for creating an identity funded from Platform addresses.
 */
export interface IdentityCreateFromAddressesOptions {
  /**
   * The identity to create (with public keys set up).
   * Use Identity.create() to build the identity structure first.
   */
  identity: Identity;

  /**
   * Array of input addresses with amounts to use for funding.
   * Use PlatformAddressInput for typed inputs (nonces fetched automatically).
   */
  inputs: PlatformAddressInput[];

  /**
   * Optional change output address and amount.
   * If provided, remaining credits will be sent to this address.
   */
  changeOutput?: PlatformAddressOutput;

  /**
   * Signer containing private keys for the identity's public keys.
   * Use IdentitySigner to add keys for signing identity key proofs.
   */
  identitySigner: IdentitySigner;

  /**
   * Signer containing private keys for all input addresses.
   * Use PlatformAddressSigner to add keys for signing address inputs.
   */
  addressSigner: PlatformAddressSigner;

  /**
   * Optional settings for the broadcast operation.
   * Includes retries, timeouts, userFeeIncrease, etc.
   */
  settings?: PutSettings;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IdentityCreateFromAddressesOptions")]
    pub type IdentityCreateFromAddressesOptionsJs;
}

#[wasm_bindgen]
impl WasmSdk {
    /// Create an identity funded from Platform addresses.
    ///
    /// This method handles the complete identity creation flow:
    /// 1. Fetches current nonces for all input addresses
    /// 2. Builds and signs the identity create from addresses transition
    /// 3. Broadcasts and waits for confirmation
    ///
    /// @param options - Creation options including identity, inputs, and signers
    /// @returns Promise resolving to result with created identity and updated address infos
    #[wasm_bindgen(js_name = "identityCreateFromAddresses")]
    pub async fn identity_create_from_addresses(
        &self,
        options: IdentityCreateFromAddressesOptionsJs,
    ) -> Result<IdentityCreateFromAddressesResultWasm, WasmSdkError> {
        use dash_sdk::platform::transition::put_identity::PutIdentity;
        use wasm_dpp2::utils::IntoWasm;

        let options_value: JsValue = options.into();

        // Deserialize and validate options
        let parsed = deserialize_identity_create_options(options_value.clone())?;

        // Extract identity from options
        let identity_js = js_sys::Reflect::get(&options_value, &JsValue::from_str("identity"))
            .map_err(|_| WasmSdkError::invalid_argument("identity is required"))?;
        let identity: Identity = identity_js
            .to_wasm::<wasm_dpp2::IdentityWasm>("Identity")?
            .clone()
            .into();

        // Convert inputs to map (address -> amount)
        let inputs_map = outputs_to_btree_map(parsed.inputs);

        // Convert change output if provided
        let change_output = parsed.change_output.map(|output| output.into_inner());

        // Extract signers from options using helper methods
        let identity_signer =
            IdentitySignerWasm::try_from_options_with_field(&options_value, "identitySigner")?;
        let address_signer = PlatformAddressSignerWasm::try_from_options_with_field(
            &options_value,
            "addressSigner",
        )?;

        // Extract settings from options
        let settings = extract_settings_from_options(&options_value)?;

        // Use the SDK's put_with_address_funding method
        let (created_identity, address_infos) = identity
            .put_with_address_funding(
                self.inner_sdk(),
                inputs_map,
                change_output,
                &identity_signer,
                &address_signer,
                settings,
            )
            .await
            .map_err(|e| {
                WasmSdkError::generic(format!("Failed to create identity from addresses: {}", e))
            })?;

        // Convert to Map<PlatformAddress, PlatformAddressInfo>
        let address_infos_map = Map::new();

        for (address, info_opt) in address_infos {
            let info = info_opt.ok_or_else(|| {
                WasmSdkError::generic(format!(
                    "Address {} has no info after identity creation",
                    address
                ))
            })?;
            let key = JsValue::from(PlatformAddressWasm::from(address));
            let value = JsValue::from(PlatformAddressInfoWasm::from(info));
            address_infos_map.set(&key, &value);
        }

        Ok(IdentityCreateFromAddressesResultWasm {
            identity: created_identity.into(),
            address_infos: address_infos_map,
        })
    }
}
