import Foundation
import DashSDKFFI

// MARK: - DashPay private contact requests (protocol version 15)
//
// A private contact request is sealed to one of the recipient's DashPay keys
// and paid for from the shielded pool, so nothing on chain names either side.
// The wallet records it as an ordinary `ContactRequest`; what that type has no
// field for (that it came privately, and the contact's shielded address) is
// read with `ManagedIdentity.privateContact(contactId:)`.
//
// Routing an accept: the public `acceptContactRequest` throws
// `PlatformWalletError.privateContactRequestNeedsPrivateReply` for a request
// that came privately; accept it with
// `PlatformWalletManager.acceptPrivateContactRequest` instead.

/// What an identity's private contact requests with one contact carry beyond
/// the `ContactRequest`. Rust keeps it in memory only and rebuilds it with
/// `ManagedPlatformWallet.syncPrivateContactRequests`.
public struct PrivateContactInfo: Sendable, Equatable {
    /// The contact's request came privately, so accepting it replies
    /// privately.
    public let receivedPrivately: Bool
    /// This identity's request to the contact went privately.
    public let sentPrivately: Bool
    /// The 43-byte Orchard address the contact asked to be paid at privately.
    public let shieldedAddress: Data?

    init(ffi: PrivateContactFFI) {
        receivedPrivately = ffi.received_privately
        sentPrivately = ffi.sent_privately
        shieldedAddress = ffi.has_shielded_address
            ? withUnsafeBytes(of: ffi.shielded_address) { Data($0) }
            : nil
    }
}

/// What one pass over DashPay's private contact request store did.
public struct PrivateContactSyncResult: Sendable {
    /// The requests the pass ingested, received and sent, including ones the
    /// wallet already held.
    public let requests: [ContactRequest]
    /// The store pages read.
    public let pagesRead: UInt32
    /// Entries that opened for one of the wallet's identities but were
    /// refused.
    public let rejectedCount: UInt32
    /// The store position the pass stopped at.
    public let nextPosition: UInt64
    /// Whether the pass read to the store's last entry. When false, sync
    /// again to read on.
    public let complete: Bool
    /// Identities whose ingest did not reach disk; the next pass reads their
    /// entries again.
    public let unpersistedIdentityCount: UInt32
}

extension ManagedIdentity {
    /// What this identity's private contact requests with `contactId` carry,
    /// or `nil` when none between them went privately as far as this process
    /// has seen.
    public func privateContact(contactId: Identifier) throws -> PrivateContactInfo? {
        var ffi = PrivateContactFFI()
        let result = contactId.withFFIBytes { idPtr in
            PlatformWalletResult(managed_identity_get_private_contact(handle, idPtr, &ffi))
        }
        if result.code == .notFound {
            return nil
        }
        try result.throwIfError()
        return PrivateContactInfo(ffi: ffi)
    }
}

extension ManagedPlatformWallet {
    /// Read DashPay's private contact request store for every identity of
    /// this wallet, at most `maxPages` pages (the default reads to the end),
    /// and ingest what it holds for them: requests to them, each sender
    /// checked, and requests they sent, recovered from the store alone.
    ///
    /// The keys stay in the Keychain mnemonic resolver Rust-side. The store
    /// cursors live in memory only, so the first pass after launch reads the
    /// store from its start, and until it has run the public accept cannot
    /// tell that a request came privately.
    public func syncPrivateContactRequests(
        maxPages: UInt32 = .max
    ) async throws -> PrivateContactSyncResult {
        let handle = self.handle
        let coreSigner = MnemonicResolver()
        return try await Task.detached(priority: .userInitiated) {
            () -> PrivateContactSyncResult in
            var array = ContactRequestHandleArray()
            var report = PrivateContactSyncReportFFI()
            // Keep the resolver alive across the synchronous FFI call: its
            // vtable callback fires during it.
            let result = withExtendedLifetime(coreSigner) {
                platform_wallet_sync_private_contact_requests(
                    handle,
                    coreSigner.handle,
                    maxPages,
                    &array,
                    &report
                )
            }
            defer { platform_wallet_contact_request_handle_array_free(&array) }
            try result.check()
            var requests: [ContactRequest] = []
            if let handles = array.handles, array.count > 0 {
                requests.reserveCapacity(Int(array.count))
                for i in 0..<Int(array.count) {
                    requests.append(ContactRequest(handle: handles[i]))
                }
            }
            return PrivateContactSyncResult(
                requests: requests,
                pagesRead: report.pages_read,
                rejectedCount: report.rejected_count,
                nextPosition: report.next_position,
                complete: report.complete,
                unpersistedIdentityCount: report.unpersisted_identity_count
            )
        }.value
    }
}

extension PlatformWalletManager {
    /// Send a DashPay private contact request from `senderIdentityId` to
    /// `recipientIdentityId`, paying for it from `account`'s shielded notes,
    /// so nothing on chain names either identity. `shieldedAddress` is the
    /// 43-byte Orchard address the contact may pay us at privately.
    ///
    /// `resolver` supplies both the Orchard spend authority and the DashPay
    /// keys. Throws `PlatformWalletError.shieldedSpendUnconfirmed` when the
    /// payment may already be on chain: do not send again.
    public func sendPrivateContactRequest(
        walletId: Data,
        resolver: MnemonicResolver,
        account: UInt32 = 0,
        senderIdentityId: Identifier,
        recipientIdentityId: Identifier,
        accountLabel: String? = nil,
        autoAcceptProof: Data? = nil,
        shieldedAddress: Data? = nil
    ) async throws -> ContactRequest {
        let handle = try privateContactRequestManagerHandle(
            walletId: walletId,
            resolver: resolver,
            shieldedAddress: shieldedAddress
        )
        let requestHandle: Handle = try await Task.detached(priority: .userInitiated) {
            () -> Handle in
            var outHandle: Handle = NULL_HANDLE
            let resolverHandle = resolver.handle
            // Guaranteed resolver keepalive across the FFI call: same
            // rationale as `shieldedTransfer`.
            try withExtendedLifetime(resolver) {
                try walletId.withFFIBytes { widPtr in
                    try senderIdentityId.withFFIBytes { senderPtr in
                        try recipientIdentityId.withFFIBytes { recipientPtr in
                            try withOptionalCString(accountLabel) { labelPtr in
                                try withOptionalBytes(autoAcceptProof) { proofPtr, proofLen in
                                    try withOptionalBytes(shieldedAddress) { addressPtr, _ in
                                        try platform_wallet_manager_send_private_contact_request(
                                            handle,
                                            widPtr,
                                            resolverHandle,
                                            account,
                                            senderPtr,
                                            recipientPtr,
                                            labelPtr,
                                            proofPtr,
                                            proofLen,
                                            addressPtr,
                                            &outHandle
                                        ).check()
                                    }
                                }
                            }
                        }
                    }
                }
            }
            return outHandle
        }.value
        return ContactRequest(handle: requestHandle)
    }

    /// Accept a DashPay contact request that came privately, replying
    /// privately and paying for the reply from `account`'s shielded notes.
    /// `shieldedAddress` is the 43-byte Orchard address the contact may pay
    /// us at privately.
    public func acceptPrivateContactRequest(
        walletId: Data,
        resolver: MnemonicResolver,
        account: UInt32 = 0,
        request: ContactRequest,
        shieldedAddress: Data? = nil
    ) async throws -> EstablishedContact {
        let handle = try privateContactRequestManagerHandle(
            walletId: walletId,
            resolver: resolver,
            shieldedAddress: shieldedAddress
        )
        let requestHandle = request.handle
        let establishedHandle: Handle = try await Task.detached(priority: .userInitiated) {
            () -> Handle in
            var outHandle: Handle = NULL_HANDLE
            let resolverHandle = resolver.handle
            try withExtendedLifetime((resolver, request)) {
                try walletId.withFFIBytes { widPtr in
                    try withOptionalBytes(shieldedAddress) { addressPtr, _ in
                        try platform_wallet_manager_accept_private_contact_request(
                            handle,
                            widPtr,
                            resolverHandle,
                            account,
                            requestHandle,
                            addressPtr,
                            &outHandle
                        ).check()
                    }
                }
            }
            return outHandle
        }.value
        return EstablishedContact(handle: establishedHandle)
    }

    /// The manager handle of a private contact request call, refused before
    /// any FFI call when the arguments cannot be passed: an unconfigured
    /// manager, a wallet id that is not 32 bytes, a shielded address that is
    /// not the 43 bytes Rust reads, or a resolver without a handle.
    private func privateContactRequestManagerHandle(
        walletId: Data,
        resolver: MnemonicResolver,
        shieldedAddress: Data?
    ) throws -> Handle {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle("PlatformWalletManager not configured")
        }
        guard walletId.count == 32 else {
            throw PlatformWalletError.invalidParameter("walletId must be exactly 32 bytes")
        }
        if let shieldedAddress, shieldedAddress.count != 43 {
            throw PlatformWalletError.invalidParameter(
                "shieldedAddress must be exactly 43 raw Orchard bytes"
            )
        }
        guard resolver.handle != nil else {
            throw PlatformWalletError.invalidParameter("MnemonicResolver has no handle")
        }
        return handle
    }
}

/// Run `body` with `text` as a C string, or with null when `text` is nil.
private func withOptionalCString<R>(
    _ text: String?,
    _ body: (UnsafePointer<CChar>?) throws -> R
) rethrows -> R {
    guard let text else { return try body(nil) }
    return try text.withCString { try body($0) }
}

/// Run `body` with a pointer to `data` and its length, or with `(nil, 0)`
/// when `data` is nil or empty.
private func withOptionalBytes<R>(
    _ data: Data?,
    _ body: (UnsafePointer<UInt8>?, UInt) throws -> R
) rethrows -> R {
    guard let data, !data.isEmpty else { return try body(nil, 0) }
    return try data.withUnsafeBytes { raw in
        try body(raw.bindMemory(to: UInt8.self).baseAddress, UInt(data.count))
    }
}
