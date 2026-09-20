import Foundation

/// DashPay profile published via the DashPay data contract.
///
/// Carries the user-visible profile fields (display name, public
/// message / bio, avatar URL) plus the DIP-15 integrity hashes for
/// the avatar (`avatarHash` + `avatarFingerprint`). Every field is
/// optional — an identity without a profile is represented by a
/// `nil` value rather than an all-empty `DashPayProfile`.
///
/// Built from the Rust-side `DashPayProfile` cached on
/// `ManagedIdentity.dashpay_profile`. Use
/// `ManagedIdentity.getDashPayProfile()` to read the cache and
/// `ManagedPlatformWallet.syncDashPayProfiles()` to refresh it from
/// Platform.
public struct DashPayProfile: Sendable, Equatable {
    /// Display name (publicly visible, max 25 chars per DIP-15).
    public let displayName: String?
    /// Biography / public message (max 140 chars per DIP-15).
    public let publicMessage: String?
    /// HTTPS URL of the avatar image (max 2048 chars per DIP-15).
    public let avatarUrl: String?
    /// SHA-256 hash of the avatar image bytes, as 32 bytes. Present
    /// whenever the on-chain document carried an `avatarHash`.
    public let avatarHash: Data?
    /// Perceptual dHash (8 bytes / 64 bits) of the avatar. Present
    /// whenever the on-chain document carried an `avatarFingerprint`.
    public let avatarFingerprint: Data?
    /// Core address storage encoding: type byte followed by HASH160 (21 bytes).
    public let corePaymentAddress: Data?
    /// Platform address storage encoding: type byte followed by HASH160 (21 bytes).
    public let platformPaymentAddress: Data?
    /// Complete raw Orchard address: diversifier (11 bytes) and public key (32 bytes).
    public let shieldedAddress: Data?


    public init(
        displayName: String? = nil,
        publicMessage: String? = nil,
        avatarUrl: String? = nil,
        avatarHash: Data? = nil,
        avatarFingerprint: Data? = nil,
        corePaymentAddress: Data? = nil,
        platformPaymentAddress: Data? = nil,
        shieldedAddress: Data? = nil
    ) {
        self.displayName = displayName
        self.publicMessage = publicMessage
        self.avatarUrl = avatarUrl
        self.avatarHash = avatarHash
        self.avatarFingerprint = avatarFingerprint
        self.corePaymentAddress = corePaymentAddress
        self.platformPaymentAddress = platformPaymentAddress
        self.shieldedAddress = shieldedAddress

    }

    /// Copy a `DashPayProfileFFI` into a Swift-owned value. The
    /// caller retains ownership of the FFI struct and is responsible
    /// for freeing it afterward with `dashpay_profile_ffi_free` — this
    /// initializer only *reads* the pointers.
    init(ffi: DashPayProfileFFI) {
        self.displayName = cStringToOptional(ffi.display_name)
        self.publicMessage = cStringToOptional(ffi.public_message)
        self.avatarUrl = cStringToOptional(ffi.avatar_url)
        self.avatarHash = ffi.avatar_hash_is_some
            ? Data(fromTuple32: ffi.avatar_hash)
            : nil
        self.avatarFingerprint = ffi.avatar_fingerprint_is_some
            ? Data(fromTuple8: ffi.avatar_fingerprint)
            : nil
        self.corePaymentAddress = ffi.core_payment_address_is_some ? Swift.withUnsafeBytes(of: ffi.core_payment_address) { Data($0) } : nil
        self.platformPaymentAddress = ffi.platform_payment_address_is_some ? Swift.withUnsafeBytes(of: ffi.platform_payment_address) { Data($0) } : nil
        self.shieldedAddress = ffi.shielded_address_is_some ? Swift.withUnsafeBytes(of: ffi.shielded_address) { Data($0) } : nil

    }
}

/// Input for `ManagedPlatformWallet.createDashPayProfile` /
/// `updateDashPayProfile`. Optional text/avatar fields left as `nil`
/// are omitted. Payment addresses explicitly distinguish keep, set, and remove.
///
/// `avatarBytes` is the raw image payload pre-downloaded by the app
/// layer. When provided, platform-wallet computes the SHA-256 hash
/// + dHash fingerprint internally and then discards the bytes; only
/// the hashes live on-chain alongside `avatarUrl`.
public struct DashPayProfileUpdate: Sendable {
    public var displayName: String?
    public var publicMessage: String?
    public var avatarUrl: String?
    public var avatarBytes: Data?
    public var corePaymentAddress: DashPayPaymentAddressUpdate
    public var platformPaymentAddress: DashPayPaymentAddressUpdate
    public var shieldedAddress: DashPayPaymentAddressUpdate


    public init(
        displayName: String? = nil,
        publicMessage: String? = nil,
        avatarUrl: String? = nil,
        avatarBytes: Data? = nil,
        corePaymentAddress: DashPayPaymentAddressUpdate = .keep,
        platformPaymentAddress: DashPayPaymentAddressUpdate = .keep,
        shieldedAddress: DashPayPaymentAddressUpdate = .keep
    ) {
        self.displayName = displayName
        self.publicMessage = publicMessage
        self.avatarUrl = avatarUrl
        self.avatarBytes = avatarBytes
        self.corePaymentAddress = corePaymentAddress
        self.platformPaymentAddress = platformPaymentAddress
        self.shieldedAddress = shieldedAddress

    }
}

// MARK: - Private helpers

/// Turn an optional C-string pointer into an optional `String`.
/// Returns `nil` for a null pointer, and a decoded UTF-8 string
/// otherwise. The pointer is not consumed — the caller still owns
/// the allocation.
private func cStringToOptional(_ ptr: UnsafeMutablePointer<CChar>?) -> String? {
    guard let ptr else { return nil }
    return String(cString: ptr)
}

private extension Data {
    /// Convert a 32-byte C tuple into owned `Data` bytes. Swift
    /// insists on writing these out explicitly because C tuples don't
    /// expose array-like iteration.
    ///
    /// The `Swift.withUnsafeBytes(of:)` qualifier is needed because
    /// `Data` itself exposes an instance method of the same name,
    /// which shadows the global one inside a `Data` extension body.
    init(
        fromTuple32 tuple: (
            UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
            UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
            UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
            UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
        )
    ) {
        var value = tuple
        self = Swift.withUnsafeBytes(of: &value) { Data($0) }
    }

    /// Convert an 8-byte C tuple into owned `Data` bytes.
    init(fromTuple8 tuple: (UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8)) {
        var value = tuple
        self = Swift.withUnsafeBytes(of: &value) { Data($0) }
    }
}

/// Explicit operation on a payment address property.
public enum DashPayPaymentAddressUpdate: Sendable, Equatable {
    case keep
    case set(Data)
    case remove

    func withFFI<T>(_ body: (UnsafePointer<PaymentAddressUpdateFFI>) throws -> T) rethrows -> T {
        let action: UInt32
        let data: Data
        switch self {
        case .keep: action = 0; data = Data()
        case .set(let bytes): action = 1; data = bytes
        case .remove: action = 2; data = Data()
        }
        return try data.withUnsafeBytes { bytes in
            var value = PaymentAddressUpdateFFI(action: action, bytes: bytes.baseAddress?.assumingMemoryBound(to: UInt8.self), len: UInt(data.count))
            return try withUnsafePointer(to: &value, body)
        }
    }
}
