package com.dash.sdk.ffi

import com.sun.jna.*
import com.sun.jna.ptr.IntByReference
import com.sun.jna.ptr.LongByReference
import com.sun.jna.ptr.PointerByReference

/**
 * JNA interface for the Dash SDK FFI library
 */
interface DashSDKFFI : Library {
    companion object {
        val INSTANCE: DashSDKFFI = Native.load("dash_sdk_ffi", DashSDKFFI::class.java)
    }

    // Enums
    object DashSDKNetwork {
        const val Mainnet = 0
        const val Testnet = 1
        const val Devnet = 2
        const val Local = 3
    }

    object DashSDKErrorCode {
        const val Success = 0
        const val InvalidParameter = 1
        const val InvalidState = 2
        const val NetworkError = 3
        const val SerializationError = 4
        const val ProtocolError = 5
        const val CryptoError = 6
        const val NotFound = 7
        const val Timeout = 8
        const val NotImplemented = 9
        const val InternalError = 99
    }

    object DashSDKResultDataType {
        const val None = 0
        const val String = 1
        const val BinaryData = 2
        const val IdentityHandle = 3
        const val DocumentHandle = 4
        const val DataContractHandle = 5
        const val IdentityBalanceMap = 6
    }

    object DashSDKGasFeesPaidBy {
        const val DocumentOwner = 0
        const val ContractOwner = 1
        const val PreferContractOwner = 2
    }

    object DashSDKAuthorizedActionTakers {
        const val NoOne = 0
        const val ContractOwner = 1
        const val MainGroup = 2
        const val Identity = 3
        const val Group = 4
    }

    object DashSDKTokenConfigUpdateType {
        const val NoChange = 0
        const val MaxSupply = 1
        const val MintingAllowChoosingDestination = 2
        const val NewTokensDestinationIdentity = 3
        const val ManualMinting = 4
        const val ManualBurning = 5
        const val Freeze = 6
        const val Unfreeze = 7
        const val MainControlGroup = 8
    }

    object DashSDKTokenDistributionType {
        const val PreProgrammed = 0
        const val Perpetual = 1
    }

    object DashSDKTokenEmergencyAction {
        const val Pause = 0
        const val Resume = 1
    }

    object DashSDKTokenPricingType {
        const val SinglePrice = 0
        const val SetPrices = 1
    }

    // Structures
    @Structure.FieldOrder("code", "message")
    class DashSDKError : Structure() {
        @JvmField var code: Int = 0
        @JvmField var message: String? = null
    }

    @Structure.FieldOrder("data_type", "data", "error")
    class DashSDKResult : Structure() {
        @JvmField var data_type: Int = 0
        @JvmField var data: Pointer? = null
        @JvmField var error: DashSDKError? = null
    }

    @Structure.FieldOrder("network", "skip_asset_lock_proof_verification", "request_retry_count", "request_timeout_ms", "core_ip_address", "platform_port", "dump_lookup_sessions_on_drop")
    class DashSDKConfig : Structure() {
        @JvmField var network: Int = 0
        @JvmField var skip_asset_lock_proof_verification: Boolean = false
        @JvmField var request_retry_count: Int = 3
        @JvmField var request_timeout_ms: Long = 30000
        @JvmField var core_ip_address: String? = null
        @JvmField var platform_port: Int = 0
        @JvmField var dump_lookup_sessions_on_drop: Boolean = false
    }

    @Structure.FieldOrder("data_contract_handle", "document_type", "owner_identity_handle", "properties_json")
    class DashSDKDocumentCreateParams : Structure() {
        @JvmField var data_contract_handle: Pointer? = null
        @JvmField var document_type: String? = null
        @JvmField var owner_identity_handle: Pointer? = null
        @JvmField var properties_json: String? = null
    }

    @Structure.FieldOrder("payment_token_contract_id", "token_contract_position", "minimum_token_cost", "maximum_token_cost", "gas_fees_paid_by")
    class DashSDKTokenPaymentInfo : Structure() {
        @JvmField var payment_token_contract_id: ByteArray? = null
        @JvmField var token_contract_position: Short = 0
        @JvmField var minimum_token_cost: Long = 0
        @JvmField var maximum_token_cost: Long = 0
        @JvmField var gas_fees_paid_by: Int = 0
    }

    // Core SDK functions
    fun dash_sdk_init()
    fun dash_sdk_create(config: DashSDKConfig): DashSDKResult
    fun dash_sdk_destroy(handle: Pointer)

    // Identity functions
    fun dash_sdk_identity_create(
        sdk_handle: Pointer,
        asset_lock_proof_base64: String?
    ): DashSDKResult

    fun dash_sdk_identity_fetch(
        sdk_handle: Pointer,
        identity_id_bytes: ByteArray,
        identity_id_len: Int
    ): DashSDKResult

    fun dash_sdk_identity_fetch_balance(
        sdk_handle: Pointer,
        identity_id_bytes: ByteArray,
        identity_id_len: Int,
        balance_out: LongByReference
    ): DashSDKResult

    fun dash_sdk_identities_fetch_balances(
        sdk_handle: Pointer,
        identity_ids_bytes: ByteArray,
        identity_ids_len: Int,
        identity_count: Int
    ): DashSDKResult

    fun dash_sdk_identity_topup(
        sdk_handle: Pointer,
        identity_handle: Pointer,
        asset_lock_proof_base64: String?
    ): DashSDKResult

    fun dash_sdk_identity_withdraw(
        sdk_handle: Pointer,
        identity_handle: Pointer,
        amount: Long,
        to_script_bytes: ByteArray,
        to_script_len: Int,
        core_fee_per_byte: Int
    ): DashSDKResult

    // Data Contract functions
    fun dash_sdk_data_contract_fetch(
        sdk_handle: Pointer,
        contract_id_bytes: ByteArray,
        contract_id_len: Int
    ): DashSDKResult

    fun dash_sdk_data_contract_put(
        sdk_handle: Pointer,
        contract_json: String,
        identity_handle: Pointer
    ): DashSDKResult

    // Document functions
    fun dash_sdk_document_create(
        sdk_handle: Pointer,
        params: DashSDKDocumentCreateParams
    ): DashSDKResult

    fun dash_sdk_document_search(
        sdk_handle: Pointer,
        data_contract_handle: Pointer,
        document_type: String,
        query_json: String,
        limit: Int
    ): DashSDKResult

    fun dash_sdk_document_transfer(
        sdk_handle: Pointer,
        document_handle: Pointer,
        recipient_id_bytes: ByteArray,
        recipient_id_len: Int
    ): DashSDKResult

    fun dash_sdk_document_delete(
        sdk_handle: Pointer,
        document_handle: Pointer
    ): DashSDKResult

    fun dash_sdk_document_update(
        sdk_handle: Pointer,
        document_handle: Pointer,
        updates_json: String
    ): DashSDKResult

    // Token functions
    fun dash_sdk_token_mint(
        sdk_handle: Pointer,
        contract_handle: Pointer,
        token_position: Short,
        amount: Long,
        recipient_id_bytes: ByteArray?,
        recipient_id_len: Int,
        issuer_identity_handle: Pointer
    ): DashSDKResult

    fun dash_sdk_token_transfer(
        sdk_handle: Pointer,
        contract_handle: Pointer,
        token_position: Short,
        amount: Long,
        sender_identity_handle: Pointer,
        recipient_id_bytes: ByteArray,
        recipient_id_len: Int
    ): DashSDKResult

    fun dash_sdk_token_burn(
        sdk_handle: Pointer,
        contract_handle: Pointer,
        token_position: Short,
        amount: Long,
        owner_identity_handle: Pointer
    ): DashSDKResult

    fun dash_sdk_token_balance(
        sdk_handle: Pointer,
        contract_id_bytes: ByteArray,
        contract_id_len: Int,
        token_position: Short,
        identity_id_bytes: ByteArray,
        identity_id_len: Int,
        balance_out: LongByReference
    ): DashSDKResult

    fun dash_sdk_token_freeze(
        sdk_handle: Pointer,
        contract_handle: Pointer,
        token_position: Short,
        identity_to_freeze_bytes: ByteArray,
        identity_to_freeze_len: Int,
        action_taker_identity_handle: Pointer
    ): DashSDKResult

    fun dash_sdk_token_unfreeze(
        sdk_handle: Pointer,
        contract_handle: Pointer,
        token_position: Short,
        identity_to_unfreeze_bytes: ByteArray,
        identity_to_unfreeze_len: Int,
        action_taker_identity_handle: Pointer
    ): DashSDKResult

    // Handle functions
    fun dash_sdk_identity_handle_destroy(handle: Pointer)
    fun dash_sdk_document_handle_destroy(handle: Pointer)
    fun dash_sdk_data_contract_handle_destroy(handle: Pointer)

    // Utility functions
    fun dash_sdk_error_free(error: DashSDKError)
    fun dash_sdk_string_free(str: String)
    fun dash_sdk_binary_data_free(data: Pointer)

    // Result accessors
    fun dash_sdk_result_get_string(result: DashSDKResult): String?
    fun dash_sdk_result_get_binary_data(result: DashSDKResult, length_out: IntByReference): Pointer?
    fun dash_sdk_result_get_identity_handle(result: DashSDKResult): Pointer?
    fun dash_sdk_result_get_document_handle(result: DashSDKResult): Pointer?
    fun dash_sdk_result_get_data_contract_handle(result: DashSDKResult): Pointer?

    // System information
    fun dash_sdk_mainnet_core_chains_json(): String
    fun dash_sdk_testnet_core_chains_json(): String
    fun dash_sdk_devnet_core_chains_json(): String
    fun dash_sdk_current_time_ms(): Long
    fun dash_sdk_version(): String
}