import Foundation
import DashSDKFFI

// MARK: - DashPay private contact requests (protocol version 15)
//
// A private contact request is sealed to one of the recipient's DashPay keys
// and paid for from the shielded pool, so nothing on chain names either side.
// It is signed by the same identity signer as a public request. The wallet
// records it as an ordinary `ContactRequest`; whether it came or went
// privately, and the contact's shielded address, are read with
// `ManagedIdentity.privateContact(contactId:)`.
//
// Accepting: `PlatformWalletManager.acceptContactRequestReplyingInKind` is
// the one accept a host calls; the wallet replies privately to a request that
// came privately, and with a document otherwise. The explicitly public
// `acceptContactRequest` and `sendContactRequest` throw
// `PlatformWalletError.privateContactRequestNeedsPrivateReply` for a contact
// whose requests came or went privately; send to it with
// `PlatformWalletManager.sendPrivateContactRequest` instead.

/// How an identity and one contact exchanged private contact requests. Rust
/// reads it from the stored requests, so it holds across launches. The
/// contact's shielded address holds across launches too where the store
/// attests `PlatformWalletPersistenceCapabilities.privateContactRequestScanState`
/// (the SwiftData handler does); otherwise it comes back with the next
/// `ManagedPlatformWallet.syncPrivateContactRequests`.
public struct PrivateContactInfo: Sendable, Equatable {
    /// The contact's request came privately, so accepting it replies
    /// privately.
    public let receivedPrivately: Bool
    /// This identity's request to the contact went privately.
    public let sentPrivately: Bool
    /// The 43-byte Orchard address the contact asked to be paid at
    /// privately; only when `receivedPrivately`, and only once a sync this
    /// launch has read it.
    public let shieldedAddress: Data?

    init(ffi: PrivateContactFFI) {
        receivedPrivately = ffi.received_privately
        sentPrivately = ffi.sent_privately
        shieldedAddress = ffi.has_shielded_address
            ? withUnsafeBytes(of: ffi.shielded_address) { Data($0) }
            : nil
    }
}

/// What a contact request from one of the wallet's identities waits for, as
/// the wallet decides it (`platform_wallet_get_private_contact_readiness`),
/// so a host shows "restoring private payments" apart from "reading the
/// private store" and from a contact's pending request rather than inferring
/// them from the refusals it got.
public struct PrivateContactReadiness: Sendable, Equatable {
    /// Whether the wallet's private payments are restored for the identity.
    /// While false, its contact requests and acceptances, public or private,
    /// are refused with `PlatformWalletError.privateContactPaymentsNotRestored`
    /// (72) until the shielded store is configured or the wallet's shielded
    /// account bound, or a contact request's own retry of a failed
    /// restoration succeeds; no private sync lifts it.
    public let privatePaymentsRestored: Bool
    /// Whether a private sync read DashPay's private contact request store to
    /// its end for the identity with its current keys. Every contact request
    /// and acceptance reads it again from the cursor before it is decided, and
    /// is refused with `PlatformWalletError.privateContactStoreNotRead` (74)
    /// while that read cannot finish.
    public let privateStoreRead: Bool
    /// Whether a private contact request the identity paid for may still be
    /// unresolved: the marker its private scan state stores, or a payment the
    /// shielded store's journal keeps unresolved. One pair's hold is
    /// `ManagedPlatformWallet.privatePairHeld(senderId:recipientId:)`.
    public let unresolvedPrivatePayment: Bool
    /// Whether the last restoration of the wallet's private payments failed
    /// while `privatePaymentsRestored` is false: 72 then holds until one
    /// succeeds, not only until one runs.
    /// `ManagedPlatformWallet.privatePaymentsRestoreFailure(identityId:)` reads
    /// why.
    public let privatePaymentsRestoreFailed: Bool

    public init(
        privatePaymentsRestored: Bool,
        privateStoreRead: Bool,
        unresolvedPrivatePayment: Bool,
        privatePaymentsRestoreFailed: Bool = false
    ) {
        self.privatePaymentsRestored = privatePaymentsRestored
        self.privateStoreRead = privateStoreRead
        self.unresolvedPrivatePayment = unresolvedPrivatePayment
        self.privatePaymentsRestoreFailed = privatePaymentsRestoreFailed
    }

    init(ffi: PrivateContactReadinessFFI) {
        self.init(
            privatePaymentsRestored: ffi.private_payments_restored,
            privateStoreRead: ffi.private_store_read,
            unresolvedPrivatePayment: ffi.unresolved_private_payment,
            privatePaymentsRestoreFailed: ffi.private_payments_restore_failed)
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
    /// How this identity and `contactId` exchanged private contact requests,
    /// or `nil` when neither tracked request between them came or went
    /// privately.
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
    /// What a contact request from `identityId`, one of this wallet's
    /// identities, waits for, as the wallet decides it. Not a pure read: a
    /// journal record discharged since (a payment proven never to execute) is
    /// applied first, briefly taking the wallet's write lock, as a send would
    /// see it; then it reads live wallet state. Read it again after the
    /// shielded store is configured or the wallet binds its shielded account
    /// to see the restore. Throws `PlatformWalletError.identityNotFound`
    /// (code 7) for an identity the wallet does not hold.
    public func privateContactReadiness(identityId: Identifier) throws -> PrivateContactReadiness {
        var ffi = PrivateContactReadinessFFI()
        let handle = self.handle
        let result = identityId.withFFIBytes { idPtr in
            platform_wallet_get_private_contact_readiness(handle, idPtr, &ffi)
        }
        try result.check()
        return PrivateContactReadiness(ffi: ffi)
    }

    /// Whether a public contact request from `senderId`, one of this
    /// wallet's identities, to `recipientId` was broadcast and may have been
    /// delivered, with no proof settling its outcome yet. The send or accept
    /// that broadcast it threw `PlatformWalletError.transactionBroadcastUnconfirmed`
    /// (20); while it stands, Rust refuses every contact request or
    /// acceptance to the recipient, public or private
    /// (`PlatformWalletError.publicContactRequestUnconfirmed`, 76), so show the
    /// pair as unconfirmed and offer neither. A public contact request sync
    /// (the DashPay sync) settles it. Read it beside
    /// `privatePairHeld(senderId:recipientId:)`, which reads a private request
    /// pending. Throws `PlatformWalletError.identityNotFound` (code 7) for a
    /// sender the wallet does not hold.
    public func publicContactRequestUnconfirmed(senderId: Identifier, recipientId: Identifier) throws -> Bool {
        var unconfirmed = false
        let handle = self.handle
        let result = senderId.withFFIBytes { senderPtr in
            recipientId.withFFIBytes { recipientPtr in
                platform_wallet_get_public_contact_request_unconfirmed(
                    handle, senderPtr, recipientPtr, &unconfirmed)
            }
        }
        try result.check()
        return unconfirmed
    }

    /// Why the last restoration of this wallet's private payments failed,
    /// while they are not restored for `identityId` and none succeeded since,
    /// or `nil` when none failed (restored, or only not restored yet). What
    /// `PrivateContactReadiness.privatePaymentsRestoreFailed` flags. Throws
    /// `PlatformWalletError.identityNotFound` (code 7) for an identity the
    /// wallet does not hold.
    public func privatePaymentsRestoreFailure(identityId: Identifier) throws -> String? {
        var reason: UnsafeMutablePointer<CChar>?
        let handle = self.handle
        let result = identityId.withFFIBytes { idPtr in
            platform_wallet_get_private_payments_restore_failure(handle, idPtr, &reason)
        }
        defer { platform_wallet_string_free(reason) }
        try result.check()
        return reason.map { String(cString: $0) }
    }

    /// Whether a contact request from `senderId`, one of this wallet's
    /// identities, to `recipientId`, public or private, is held for a
    /// private one, as a send decides it: a private request to the recipient
    /// is being paid for, or was paid for and is not yet read back from the
    /// store, or a private payment the shielded store's journal keeps
    /// unresolved holds the pair, also after the sender identity was removed
    /// and added again. While it is, Rust refuses another private send or
    /// reply to the recipient (`PlatformWalletError.privateContactRequestPending`)
    /// and holds a public one, so show the pair as pending and offer
    /// neither. A `syncPrivateContactRequests` that reads the entry back
    /// frees it, as does the payment proven never to execute (its anchor
    /// pruned, which the shielded sync judges); an endpoint's refusal of the
    /// payment does not. Not a pure read: a journal record discharged since
    /// is applied first, so a discharge reads as freed with no send or sync
    /// in between. Throws `PlatformWalletError.identityNotFound` (code 7)
    /// for a sender the wallet does not hold.
    public func privatePairHeld(senderId: Identifier, recipientId: Identifier) throws -> Bool {
        var held = false
        let handle = self.handle
        let result = senderId.withFFIBytes { senderPtr in
            recipientId.withFFIBytes { recipientPtr in
                platform_wallet_get_private_pair_held(handle, senderPtr, recipientPtr, &held)
            }
        }
        try result.check()
        return held
    }

    /// Read DashPay's private contact request store for every identity of
    /// this wallet, at most `maxPages` pages (the default reads to the end),
    /// and ingest what it holds for them: requests to them, each sender
    /// checked, and requests they sent, recovered from the store alone.
    ///
    /// The store is opened with keys that stay in the Keychain mnemonic
    /// resolver Rust-side; each request's signature is checked against its
    /// sender's keys. Where the store attests
    /// `PlatformWalletPersistenceCapabilities.privateContactRequestScanState`
    /// (the SwiftData handler does), each identity's read position is stored
    /// with the requests read before it and restored at load, so a pass
    /// after launch reads on from there; otherwise the first pass after
    /// launch reads the store from its start. A request it ingests is stored
    /// as having come privately, which holds across launches.
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
    /// keys. `signer` is the identity signer `sendContactRequest` takes: it
    /// signs the request with one of the sender's authentication keys, as it
    /// signs a public request's transition. Before anything is paid the
    /// wallet reads DashPay's private store for the sender to its end.
    /// Throws `PlatformWalletError.shieldedSpendUnconfirmed` when the payment
    /// may already be on chain, `.transactionBroadcastUnconfirmed` when the
    /// SDK could not confirm a transition it may have accepted, and
    /// `.privateContactRequestPending` when the
    /// pair is held for a private request (in flight, paid and not yet read
    /// back, or kept by the journal), or `.publicContactRequestUnconfirmed`
    /// when a public request to the pair awaits its outcome: in each case do
    /// not send again.
    /// `.privateContactStoreNotRead` sent nothing; calling again continues
    /// the read. `.privateContactPaymentsNotRestored` and
    /// `.privateContactRecoveryUnsupported` refuse before the read.
    public func sendPrivateContactRequest(
        walletId: Data,
        resolver: MnemonicResolver,
        account: UInt32 = 0,
        senderIdentityId: Identifier,
        recipientIdentityId: Identifier,
        accountLabel: String? = nil,
        autoAcceptProof: Data? = nil,
        shieldedAddress: Data? = nil,
        signer: KeychainSigner
    ) async throws -> ContactRequest {
        let handle = try privateContactRequestManagerHandle(
            walletId: walletId,
            resolver: resolver,
            shieldedAddress: shieldedAddress
        )
        // Counted as the other native manager operations are, so a teardown
        // waits for this call to return before it frees the manager.
        try admitNativeOp("sendPrivateContactRequest")
        defer { finishNativeOp() }
        let signerHandle = signer.handle
        let requestHandle: Handle = try await Task.detached(priority: .userInitiated) {
            () -> Handle in
            var outHandle: Handle = NULL_HANDLE
            let resolverHandle = resolver.handle
            // Guaranteed keepalive of the resolver and the signer across the
            // FFI call, whose vtable callbacks fire during it: same rationale
            // as `shieldedTransfer`.
            try withExtendedLifetime((resolver, signer)) {
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
                                            signerHandle,
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
    /// us at privately. `signer` is the identity signer
    /// `acceptContactRequest` takes; it signs the reply. It throws as
    /// `sendPrivateContactRequest` does.
    public func acceptPrivateContactRequest(
        walletId: Data,
        resolver: MnemonicResolver,
        account: UInt32 = 0,
        request: ContactRequest,
        shieldedAddress: Data? = nil,
        signer: KeychainSigner
    ) async throws -> EstablishedContact {
        let handle = try privateContactRequestManagerHandle(
            walletId: walletId,
            resolver: resolver,
            shieldedAddress: shieldedAddress
        )
        // Counted as the other native manager operations are, so a teardown
        // waits for this call to return before it frees the manager.
        try admitNativeOp("acceptPrivateContactRequest")
        defer { finishNativeOp() }
        let requestHandle = request.handle
        let signerHandle = signer.handle
        let establishedHandle: Handle = try await Task.detached(priority: .userInitiated) {
            () -> Handle in
            var outHandle: Handle = NULL_HANDLE
            let resolverHandle = resolver.handle
            try withExtendedLifetime((resolver, signer, request)) {
                try walletId.withFFIBytes { widPtr in
                    try withOptionalBytes(shieldedAddress) { addressPtr, _ in
                        try platform_wallet_manager_accept_private_contact_request(
                            handle,
                            widPtr,
                            resolverHandle,
                            account,
                            requestHandle,
                            addressPtr,
                            signerHandle,
                            &outHandle
                        ).check()
                    }
                }
            }
            return outHandle
        }.value
        return EstablishedContact(handle: establishedHandle)
    }

    /// Accept a DashPay contact request replying the way it came: privately,
    /// paid from `account`'s shielded notes, when it came privately and no
    /// reply exists yet; otherwise with a `contactRequest` document signed by
    /// `signer`, or by adopting the reply already sent. Which reply a request
    /// takes is the wallet's decision, so this is the accept a host calls.
    /// `shieldedAddress` is the 43-byte Orchard address the contact may pay
    /// us at privately; a public reply ignores it. A public or adopted reply
    /// needs neither shielded support nor a mnemonic read for the payment.
    /// Throws `PlatformWalletError.shieldedSpendUnconfirmed` (or
    /// `.transactionBroadcastUnconfirmed`) when a private reply may already be
    /// on chain, or `.privateContactRequestPending` when
    /// the pair is held for one: do not accept again; `.privateContactStoreNotRead`
    /// when the private store read every accept runs first did not finish
    /// (nothing sent; calling again continues it); and
    /// `PlatformWalletError.shieldedBuildRequired` when the request needs a
    /// private reply and the native library has no shielded support.
    public func acceptContactRequestReplyingInKind(
        walletId: Data,
        resolver: MnemonicResolver,
        account: UInt32 = 0,
        request: ContactRequest,
        shieldedAddress: Data? = nil,
        signer: KeychainSigner
    ) async throws -> EstablishedContact {
        let handle = try privateContactRequestManagerHandle(
            walletId: walletId,
            resolver: resolver,
            shieldedAddress: shieldedAddress
        )
        // Counted as the other native manager operations are, so a teardown
        // waits for this call to return before it frees the manager.
        try admitNativeOp("acceptContactRequestReplyingInKind")
        defer { finishNativeOp() }
        let requestHandle = request.handle
        let signerHandle = signer.handle
        let establishedHandle: Handle = try await Task.detached(priority: .userInitiated) {
            () -> Handle in
            var outHandle: Handle = NULL_HANDLE
            let resolverHandle = resolver.handle
            // Keep the resolver, the signer and the request alive across the
            // FFI call, whose vtable callbacks fire during it.
            try withExtendedLifetime((resolver, signer, request)) {
                try walletId.withFFIBytes { widPtr in
                    try withOptionalBytes(shieldedAddress) { addressPtr, _ in
                        try platform_wallet_manager_accept_contact_request_replying_in_kind(
                            handle,
                            widPtr,
                            resolverHandle,
                            account,
                            requestHandle,
                            addressPtr,
                            signerHandle,
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
