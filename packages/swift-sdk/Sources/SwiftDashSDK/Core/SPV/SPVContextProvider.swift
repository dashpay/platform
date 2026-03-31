import DashSDKFFI
import Foundation

// MARK: - C Callback: Get quorum public key from SPV

/// C-compatible callback that bridges Platform SDK quorum key requests to the SPV client.
///
/// `handle` is the raw `FFIDashSpvClient*` pointer, passed as `core_handle` in
/// `ContextProviderCallbacks`. The SPV FFI function `ffi_dash_spv_get_quorum_public_key`
/// retrieves the BLS public key for the given quorum from the locally synced masternode list.
///
/// - Parameters:
///   - handle: Raw pointer to `FFIDashSpvClient` (cast from `void*`).
///   - quorumType: The quorum type identifier.
///   - quorumHash: Pointer to 32-byte quorum hash.
///   - coreChainLockedHeight: The core chain locked height for the request.
///   - outPubkey: Pointer to a 48-byte output buffer for the BLS public key.
/// - Returns: `CallbackResult` with `success: true` on success or error details on failure.
func spvGetQuorumPublicKey(
    handle: UnsafeMutableRawPointer?,
    quorumType: UInt32,
    quorumHash: UnsafePointer<UInt8>?,
    coreChainLockedHeight: UInt32,
    outPubkey: UnsafeMutablePointer<UInt8>?
) -> CallbackResult {
    guard let handle = handle else {
        return CallbackResult(success: false, error_code: -1, error_message: nil)
    }
    guard let quorumHash = quorumHash, let outPubkey = outPubkey else {
        return CallbackResult(success: false, error_code: -2, error_message: nil)
    }

    let client = handle.assumingMemoryBound(to: FFIDashSpvClient.self)
    let ffiResult = ffi_dash_spv_get_quorum_public_key(
        client, quorumType, quorumHash, coreChainLockedHeight, outPubkey, 48
    )

    if ffiResult.error_code == 0 {
        return CallbackResult(success: true, error_code: 0, error_message: nil)
    } else {
        return CallbackResult(
            success: false,
            error_code: ffiResult.error_code,
            error_message: ffiResult.error_message
        )
    }
}

// MARK: - C Callback: Get platform activation height from SPV

/// C-compatible callback that bridges Platform SDK activation height requests to the SPV client.
///
/// `handle` is the raw `FFIDashSpvClient*` pointer. The SPV FFI function
/// `ffi_dash_spv_get_platform_activation_height` returns the core block height at which
/// Platform was activated, as determined from the locally synced chain.
///
/// - Parameters:
///   - handle: Raw pointer to `FFIDashSpvClient` (cast from `void*`).
///   - outHeight: Pointer to a `UInt32` where the activation height will be written.
/// - Returns: `CallbackResult` with `success: true` on success or error details on failure.
func spvGetPlatformActivationHeight(
    handle: UnsafeMutableRawPointer?,
    outHeight: UnsafeMutablePointer<UInt32>?
) -> CallbackResult {
    guard let handle = handle else {
        return CallbackResult(success: false, error_code: -1, error_message: nil)
    }
    guard let outHeight = outHeight else {
        return CallbackResult(success: false, error_code: -2, error_message: nil)
    }

    let client = handle.assumingMemoryBound(to: FFIDashSpvClient.self)
    let ffiResult = ffi_dash_spv_get_platform_activation_height(client, outHeight)

    if ffiResult.error_code == 0 {
        return CallbackResult(success: true, error_code: 0, error_message: nil)
    } else {
        return CallbackResult(
            success: false,
            error_code: ffiResult.error_code,
            error_message: ffiResult.error_message
        )
    }
}

// MARK: - Helper to create ContextProviderCallbacks

/// Creates a `ContextProviderCallbacks` struct configured to use the SPV client
/// for quorum key lookups and platform activation height.
///
/// The returned struct is suitable for passing to `dash_sdk_create_with_callbacks`.
/// The SPV client must remain alive for the entire lifetime of the SDK instance
/// that uses these callbacks.
///
/// - Parameter spvClientHandle: Raw pointer to the `FFIDashSpvClient`, obtained
///   from `SPVClient.unsafeFFIClientPointer`.
/// - Returns: A `ContextProviderCallbacks` ready to pass to `dash_sdk_create_with_callbacks`.
func makeSPVContextProviderCallbacks(spvClientHandle: UnsafeMutableRawPointer) -> ContextProviderCallbacks {
    return ContextProviderCallbacks(
        core_handle: spvClientHandle,
        get_platform_activation_height: spvGetPlatformActivationHeight,
        get_quorum_public_key: spvGetQuorumPublicKey
    )
}
