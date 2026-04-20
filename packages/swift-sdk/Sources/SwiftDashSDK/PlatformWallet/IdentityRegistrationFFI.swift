// Identity registration FFI function declarations
// Mirrors packages/rs-platform-wallet-ffi/src/identity_registration.rs.
//
// The rs-platform-wallet-ffi crate does not emit a public C header (see
// its `lib.rs`: everything is `#[no_mangle] extern "C"`), so the Swift
// side declares the symbols with @_silgen_name the same way the rest
// of the platform-wallet FFI surface is wired up.

import Foundation

// MARK: - FFI Structs

/// Mirrors `IdentityInputAddressFFI` from Rust.
///
/// - `address_type`: 0 = P2PKH, 1 = P2SH.
/// - `hash`: 20-byte address hash (BE tuple of `UInt8` x 20).
/// - `nonce`: current anti-replay nonce for the address.
/// - `credits`: credits to spend from this address.
@frozen
public struct IdentityInputAddressFFI {
    public var address_type: UInt8
    public var hash: (
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
    )
    public var nonce: UInt32
    public var credits: UInt64
}

/// Mirrors `IdentityOutputAddressFFI` from Rust. When `has_output` is
/// false the remaining fields are ignored.
@frozen
public struct IdentityOutputAddressFFI {
    public var has_output: Bool
    public var address_type: UInt8
    public var hash: (
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
    )
    public var credits: UInt64
}

// MARK: - FFI Functions

@_silgen_name("platform_wallet_register_identity_from_addresses")
func platform_wallet_register_identity_from_addresses(
    _ wallet_handle: Handle,
    _ identity_index: UInt32,
    _ key_count: UInt32,
    _ inputs: UnsafePointer<IdentityInputAddressFFI>?,
    _ inputs_count: Int,
    // Passed by pointer, not value — C struct-by-value across
    // the Swift/Rust boundary is brittle when the struct size
    // straddles the 16-byte register-passing threshold.
    _ output: UnsafePointer<IdentityOutputAddressFFI>?,
    _ out_identity_id: UnsafeMutablePointer<(
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
    )>,
    _ out_identity_handle: UnsafeMutablePointer<Handle>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult
