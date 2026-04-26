import Foundation
import DashSDKFFI

/// Managed Identity with DashPay metadata.
///
/// `@unchecked Sendable`: immutable `Handle` (UInt64); all mutable
/// state lives in the Rust-side `MANAGED_IDENTITY_STORAGE` under
/// parking_lot RwLock. Same pattern as `ContactRequest`,
/// `EstablishedContact`, and `ManagedPlatformWallet`.
public final class ManagedIdentity: @unchecked Sendable {
    internal let handle: Handle

    internal init(handle: Handle) {
        self.handle = handle
    }

    deinit {
        _ = managed_identity_destroy(handle)
    }

    /// Create a ManagedIdentity from identity bytes
    public static func fromIdentityBytes(_ bytes: Data) throws -> ManagedIdentity {
        var handle: Handle = NULL_HANDLE
        var error = PlatformWalletFFIError()

        let result = bytes.withUnsafeBytes { bytesPtr in
            managed_identity_create_from_identity_bytes(
                bytesPtr.baseAddress?.assumingMemoryBound(to: UInt8.self),
                bytes.count,
                &handle,
                &error
            )
        }

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return ManagedIdentity(handle: handle)
    }

    /// Get the identity ID
    public func getId() throws -> Identifier {
        var buf = [UInt8](repeating: 0, count: 32)
        var error = PlatformWalletFFIError()

        let result = buf.withUnsafeMutableBufferPointer { bp -> PlatformWalletFFIResult in
            managed_identity_get_id(handle, bp.baseAddress!, &error)
        }
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return Data(buf)
    }

    /// Get the identity balance
    public func getBalance() throws -> UInt64 {
        var balance: UInt64 = 0
        var error = PlatformWalletFFIError()

        let result = managed_identity_get_balance(handle, &balance, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return balance
    }

    /// Get the identity revision.
    ///
    /// Revisions advance on every Platform-side state transition
    /// (key add/remove, balance change, etc.). Value 0 means the
    /// identity was just created and has not been mutated.
    public func getRevision() throws -> UInt64 {
        var revision: UInt64 = 0
        var error = PlatformWalletFFIError()

        let result = managed_identity_get_revision(handle, &revision, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
        return revision
    }

    /// Get the BIP-9 identity index recorded on this managed
    /// identity — the position in
    /// `m/9'/coin'/5'/0'/key_type'/identity_index'/key_id'` used to
    /// derive its keys.
    ///
    /// Returns `nil` for out-of-wallet (observed) identities, which
    /// have no derivation context. Both `Some` and `None` are valid
    /// states; the FFI surfaces them through a paired `out_has_index`
    /// flag and only signals an error for invalid handles / null
    /// pointers.
    public func getIdentityIndex() throws -> UInt32? {
        var hasIndex: Bool = false
        var index: UInt32 = 0
        var error = PlatformWalletFFIError()

        let result = managed_identity_get_identity_index(handle, &hasIndex, &index, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
        return hasIndex ? index : nil
    }

    /// Get the lifecycle status this identity carries on the wallet
    /// side. Maps to `IdentityStatusFFI` on the Rust side.
    public func getStatus() throws -> IdentityStatus {
        var raw: UInt8 = 0
        var error = PlatformWalletFFIError()

        let result = managed_identity_get_status(handle, &raw, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
        return IdentityStatus(rawValue: raw) ?? .unknown
    }

    /// Snapshot of an identity's registered public keys. Mirrors the
    /// DPP `IdentityPublicKeyV0` shape. Contract bounds aren't
    /// included yet — see the FFI docstring.
    public struct IdentityPublicKeyInfo: Sendable {
        public let keyId: Int32
        public let purpose: KeyPurpose
        public let securityLevel: SecurityLevel
        public let keyType: KeyType
        public let readOnly: Bool
        /// Milliseconds-since-epoch when this key was disabled, or
        /// `nil` if it's still active.
        public let disabledAt: Int64?
        /// Public-key payload. Format depends on `keyType`
        /// (compressed secp256k1 pubkey for ECDSA, hash160 for
        /// HASH160 variants, etc.).
        public let data: Data
    }

    /// Return every `IdentityPublicKey` registered on this identity.
    /// Calls into the Rust side, copies the flat C buffer into
    /// Swift-owned values, and releases the FFI buffer before
    /// returning.
    public func getPublicKeys() throws -> [IdentityPublicKeyInfo] {
        var outPtr: UnsafeMutablePointer<IdentityPublicKeyFFI>? = nil
        var outCount: Int = 0
        var error = PlatformWalletFFIError()

        let result = managed_identity_get_public_keys(
            handle,
            &outPtr,
            &outCount,
            &error
        )
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        // Empty-identity branch. Rust sets ptr=null / count=0.
        guard let ptr = outPtr, outCount > 0 else {
            return []
        }

        defer {
            managed_identity_free_public_keys(ptr, outCount)
        }

        var keys: [IdentityPublicKeyInfo] = []
        keys.reserveCapacity(outCount)
        for i in 0..<outCount {
            let ffi = ptr[i]

            guard let purpose = KeyPurpose(rawValue: ffi.purpose),
                  let securityLevel = SecurityLevel(rawValue: ffi.security_level),
                  let keyType = KeyType(rawValue: ffi.key_type)
            else {
                // Rust emitted a discriminant Swift doesn't know
                // about. Keep the deallocation (defer) and surface
                // a typed error so callers can recover.
                throw PlatformWalletError.deserialization(
                    "Unknown public-key enum discriminant at index \(i)"
                )
            }

            let data: Data
            if let blob = ffi.data_ptr, ffi.data_len > 0 {
                data = Data(bytes: blob, count: ffi.data_len)
            } else {
                data = Data()
            }

            keys.append(
                IdentityPublicKeyInfo(
                    keyId: Int32(bitPattern: ffi.key_id),
                    purpose: purpose,
                    securityLevel: securityLevel,
                    keyType: keyType,
                    readOnly: ffi.read_only,
                    disabledAt: ffi.disabled_at_is_some
                        ? Int64(bitPattern: ffi.disabled_at)
                        : nil,
                    data: data
                )
            )
        }

        return keys
    }

    /// Get the identity label
    public func getLabel() throws -> String? {
        var labelPtr: UnsafeMutablePointer<CChar>? = nil
        var error = PlatformWalletFFIError()

        let result = managed_identity_get_label(handle, &labelPtr, &error)

        if result == ErrorIdentityNotFound {
            return nil
        }

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        defer {
            if let ptr = labelPtr {
                platform_wallet_string_free(ptr)
            }
        }

        guard let ptr = labelPtr else {
            return nil
        }

        return String(cString: ptr)
    }

    /// Set the identity label
    public func setLabel(_ label: String) throws {
        var error = PlatformWalletFFIError()
        let labelCStr = (label as NSString).utf8String

        let result = managed_identity_set_label(handle, labelCStr, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
    }

    /// Get the last updated balance block time
    public func getLastUpdatedBalanceBlockTime() throws -> BlockTime? {
        var ffiBlockTime = FFIBlockTime(height: 0, core_height: 0, timestamp: 0)
        var error = PlatformWalletFFIError()

        let result = managed_identity_get_last_updated_balance_block_time(handle, &ffiBlockTime, &error)

        if result == ErrorIdentityNotFound {
            return nil
        }

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return BlockTime(ffiBlockTime: ffiBlockTime)
    }

    /// Set the last updated balance block time
    public func setLastUpdatedBalanceBlockTime(_ blockTime: BlockTime) throws {
        var error = PlatformWalletFFIError()
        var ffiBlockTime = blockTime.ffiValue

        let result = managed_identity_set_last_updated_balance_block_time(
            handle, &ffiBlockTime, &error
        )
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
    }

    /// Get the last synced keys block time
    public func getLastSyncedKeysBlockTime() throws -> BlockTime? {
        var ffiBlockTime = FFIBlockTime(height: 0, core_height: 0, timestamp: 0)
        var error = PlatformWalletFFIError()

        let result = managed_identity_get_last_synced_keys_block_time(handle, &ffiBlockTime, &error)

        if result == ErrorIdentityNotFound {
            return nil
        }

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return BlockTime(ffiBlockTime: ffiBlockTime)
    }

    // MARK: - Contact Request Management

    /// Get all sent contact request IDs
    public func getSentContactRequestIds() throws -> [Identifier] {
        var array = IdentifierArray(items: nil, count: 0)
        var error = PlatformWalletFFIError()

        let result = managed_identity_get_sent_contact_request_ids(handle, &array, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        defer {
            platform_wallet_identifier_array_free(&array)
        }

        guard array.items != nil, array.count > 0 else {
            return []
        }

        var identifiers: [Identifier] = []
        identifiers.reserveCapacity(array.count)
        for i in 0..<array.count {
            identifiers.append(identifierFromFFIArray(array, at: i))
        }

        return identifiers
    }

    /// Get all incoming contact request IDs
    public func getIncomingContactRequestIds() throws -> [Identifier] {
        var array = IdentifierArray(items: nil, count: 0)
        var error = PlatformWalletFFIError()

        let result = managed_identity_get_incoming_contact_request_ids(handle, &array, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        defer {
            platform_wallet_identifier_array_free(&array)
        }

        guard array.items != nil, array.count > 0 else {
            return []
        }

        var identifiers: [Identifier] = []
        identifiers.reserveCapacity(array.count)
        for i in 0..<array.count {
            identifiers.append(identifierFromFFIArray(array, at: i))
        }

        return identifiers
    }

    /// Get all established contact IDs
    public func getEstablishedContactIds() throws -> [Identifier] {
        var array = IdentifierArray(items: nil, count: 0)
        var error = PlatformWalletFFIError()

        let result = managed_identity_get_established_contact_ids(handle, &array, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        defer {
            platform_wallet_identifier_array_free(&array)
        }

        guard array.items != nil, array.count > 0 else {
            return []
        }

        var identifiers: [Identifier] = []
        identifiers.reserveCapacity(array.count)
        for i in 0..<array.count {
            identifiers.append(identifierFromFFIArray(array, at: i))
        }

        return identifiers
    }

    /// Get a sent contact request by recipient ID
    public func getSentContactRequest(recipientId: Identifier) throws -> ContactRequest? {
        var requestHandle: Handle = NULL_HANDLE
        var error = PlatformWalletFFIError()

        let result = recipientId.withFFIBytes { idPtr in
            managed_identity_get_sent_contact_request(handle, idPtr, &requestHandle, &error)
        }

        if result == ErrorContactNotFound {
            return nil
        }

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return ContactRequest(handle: requestHandle)
    }

    /// Get an incoming contact request by sender ID
    public func getIncomingContactRequest(senderId: Identifier) throws -> ContactRequest? {
        var requestHandle: Handle = NULL_HANDLE
        var error = PlatformWalletFFIError()

        let result = senderId.withFFIBytes { idPtr in
            managed_identity_get_incoming_contact_request(handle, idPtr, &requestHandle, &error)
        }

        if result == ErrorContactNotFound {
            return nil
        }

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return ContactRequest(handle: requestHandle)
    }

    /// Get an established contact by contact ID
    public func getEstablishedContact(contactId: Identifier) throws -> EstablishedContact? {
        var contactHandle: Handle = NULL_HANDLE
        var error = PlatformWalletFFIError()

        let result = contactId.withFFIBytes { idPtr in
            managed_identity_get_established_contact(handle, idPtr, &contactHandle, &error)
        }

        if result == ErrorContactNotFound {
            return nil
        }

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return EstablishedContact(handle: contactHandle)
    }

    /// Check if a contact is established
    public func isContactEstablished(contactId: Identifier) throws -> Bool {
        var isEstablished: Bool = false
        var error = PlatformWalletFFIError()

        let result = contactId.withFFIBytes { idPtr in
            managed_identity_is_contact_established(handle, idPtr, &isEstablished, &error)
        }
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return isEstablished
    }

    /// Send a contact request by attaching a pre-built request handle.
    /// Build the request via `ContactRequest.create(...)` first.
    public func sendContactRequest(_ request: ContactRequest) throws {
        var error = PlatformWalletFFIError()

        let result = managed_identity_send_contact_request(handle, request.handle, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
    }

    /// Accept an incoming contact request.
    /// The request handle typically comes from `getIncomingContactRequest(senderId:)`.
    public func acceptContactRequest(_ request: ContactRequest) throws {
        var error = PlatformWalletFFIError()

        let result = managed_identity_accept_contact_request(handle, request.handle, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
    }

    /// Reject a contact request from another identity
    public func rejectContactRequest(senderId: Identifier) throws {
        var error = PlatformWalletFFIError()
        let result = senderId.withFFIBytes { idPtr in
            managed_identity_reject_contact_request(handle, idPtr, &error)
        }
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
    }

    // MARK: - DPNS names

    /// Read the cached DPNS labels for this identity. Empty when
    /// the cache hasn't been populated yet (new identity, or an
    /// existing identity that hasn't been synced). Call
    /// `ManagedPlatformWallet.syncDpnsNames(identityId:)` to
    /// refresh from Platform before reading.
    ///
    /// Returns the label strings only (no acquired-at metadata) —
    /// they're what the iOS UI needs today; exposing the full
    /// `DpnsNameInfo` is a future wrapper if the UI grows an
    /// acquisition-date column.
    public func getDpnsNames() throws -> [String] {
        try readDpnsNameArray { managed_identity_get_dpns_names(handle, &$0, &$1) }
    }

    /// Read the cached contested DPNS labels for this identity —
    /// names where this identity is a contender and the contest
    /// hasn't resolved yet. Empty until the wallet has run
    /// `syncContestedDpnsNames(identityId:)` at least once.
    public func getContestedDpnsNames() throws -> [String] {
        try readDpnsNameArray { managed_identity_get_contested_dpns_names(handle, &$0, &$1) }
    }

    /// Shared body for the two DPNS-name readers. Both share the
    /// same `DpnsNameArray` FFI shape + free helper, so the only
    /// per-method variance is which C entry point fills the array.
    /// Keeping the plumbing in one place avoids duplicating the
    /// error handling + defer + pointer iteration block twice.
    private func readDpnsNameArray(
        _ fetch: (inout DpnsNameArray, inout PlatformWalletFFIError) -> PlatformWalletFFIResult
    ) throws -> [String] {
        var array = DpnsNameArray(labels: nil, count: 0)
        var error = PlatformWalletFFIError()
        let result = fetch(&array, &error)
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
        defer { dpns_name_array_free(&array) }
        guard let labels = array.labels, array.count > 0 else {
            return []
        }
        var names: [String] = []
        names.reserveCapacity(array.count)
        for i in 0..<array.count {
            if let labelPtr = labels[i] {
                names.append(String(cString: labelPtr))
            }
        }
        return names
    }

    // MARK: - DashPay Profile

    /// Read the cached DashPay profile for this identity.
    ///
    /// Returns `nil` when no profile has been fetched from Platform
    /// (or when the on-chain profile was explicitly cleared). This is
    /// a sync, lock-free read of the in-memory cache — call
    /// `ManagedPlatformWallet.syncDashPayProfiles()` first when you
    /// want the freshest on-chain data.
    public func getDashPayProfile() throws -> DashPayProfile? {
        var ffiProfile = dashPayProfileFFIEmpty()
        var hasProfile: Bool = false
        var error = PlatformWalletFFIError()

        let result = managed_identity_get_dashpay_profile(
            handle,
            &ffiProfile,
            &hasProfile,
            &error
        )
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        // `dashpay_profile_ffi_free` walks the three nullable string
        // pointers — calling it on an empty / no-profile struct is
        // safe because each pointer is independently null-checked on
        // the Rust side.
        defer { dashpay_profile_ffi_free(&ffiProfile) }

        guard hasProfile else { return nil }
        return DashPayProfile(ffi: ffiProfile)
    }
}
