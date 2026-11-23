import Foundation
import DashSDKFFI

// MARK: - Private document entries (protocol version 15)
//
// A private document type's store holds fixed-size encrypted entries that
// name nobody in the clear, each sealed to one key of one identity and
// appended by an anonymous transition paid from the shielded pool. A wallet
// finds its own by trying every entry with its identities' keys, and
// recovers the ones its identities sealed with their outgoing viewing keys.
// `ManagedPlatformWallet.readPrivateDocumentEntries` reads a store that way
// in Rust and hands back what opened. Which keys, which identities and how
// a payload reads are Rust's; this file only marshals.

/// Which way an entry went.
public enum PrivateDocumentDirection: UInt8, Sendable, Equatable {
    /// Sealed to a key of one of the wallet's identities, which opened it.
    case received = 0
    /// Sealed by one of the wallet's identities, whose outgoing viewing key
    /// recovered it.
    case sent = 1
}

/// How an entry tells its recipient who wrote it.
public enum PrivateDocumentAuthMode: UInt8, Sendable, Equatable {
    /// The entry names no sender.
    case anonymous = 0
    /// The entry names a sender and carries a MAC only the sender and the
    /// recipient can make, so it proves nothing to anyone else.
    case deniableMac = 1
    /// The entry names a sender and carries a signature.
    case signature = 2
}

/// What an entry's payload held, read against the type's schema.
public enum PrivateDocumentPayload: Sendable, Equatable {
    /// The properties, decoded and checked against the type's schema, as a
    /// JSON object in the encoding a document query returns: binary values
    /// as base64 strings, identifiers as base58 strings.
    case properties(json: String)
    /// Why the payload's properties were refused.
    case refused(reason: String)
}

/// An entry of a private document type's store one of the wallet's
/// identities opened or recovered. An entry from one of the wallet's
/// identities to another is read twice, once each way.
public struct PrivateDocumentEntry: Sendable, Equatable {
    /// The entry's position in the store.
    public let position: UInt64
    /// The height of the block that appended it.
    public let blockHeight: UInt64
    /// The time of the block that appended it, in milliseconds since 1970.
    public let blockTimeMs: UInt64
    /// Which way it went.
    public let direction: PrivateDocumentDirection
    /// The wallet's identity that opened it, when received, or whose
    /// outgoing viewing key recovered it, when sent.
    public let walletIdentityId: Identifier
    /// That identity's key: the one the entry is sealed to, when received,
    /// or the one the outgoing viewing key is derived from, when sent.
    public let walletKeyId: UInt32
    /// The identity the entry is sealed to.
    public let recipientIdentityId: Identifier
    /// How the entry tells its recipient who wrote it.
    public let authMode: PrivateDocumentAuthMode
    /// The identity the entry names as its sender, `nil` when it is
    /// anonymous. A claim: this read does not check it against the sender
    /// identity's keys, which is the type's rule to set.
    public let claimedSenderIdentityId: Identifier?
    /// The key the entry names as its sender's, `nil` when anonymous. A
    /// claim, as `claimedSenderIdentityId`.
    public let claimedSenderKeyId: UInt32?
    /// The entry's commitment: two entries with the same one are copies.
    public let commitment: Data
    /// The properties its payload holds, or why they are refused.
    public let payload: PrivateDocumentPayload

    /// The block time as a date.
    public var blockTime: Date {
        Date(timeIntervalSince1970: TimeInterval(blockTimeMs) / 1000)
    }

    init(ffi: PrivateDocumentEntryFFI) throws {
        guard let direction = PrivateDocumentDirection(rawValue: ffi.direction) else {
            throw PlatformWalletError.unknown(
                "private document entry with an unknown direction \(ffi.direction)")
        }
        guard let authMode = PrivateDocumentAuthMode(rawValue: ffi.auth_mode) else {
            throw PlatformWalletError.unknown(
                "private document entry with an unknown auth mode \(ffi.auth_mode)")
        }
        let payload: PrivateDocumentPayload
        if let json = ffi.properties_json {
            payload = .properties(json: String(cString: json))
        } else if let refusal = ffi.properties_refusal {
            payload = .refused(reason: String(cString: refusal))
        } else {
            throw PlatformWalletError.unknown(
                "private document entry with neither properties nor a refusal")
        }
        self.position = ffi.position
        self.blockHeight = ffi.block_height
        self.blockTimeMs = ffi.block_time_ms
        self.direction = direction
        self.walletIdentityId = withUnsafeBytes(of: ffi.wallet_identity_id) { Data($0) }
        self.walletKeyId = ffi.wallet_key_id
        self.recipientIdentityId = withUnsafeBytes(of: ffi.recipient_identity_id) { Data($0) }
        self.authMode = authMode
        self.claimedSenderIdentityId = ffi.has_claimed_sender
            ? withUnsafeBytes(of: ffi.claimed_sender_identity_id) { Data($0) }
            : nil
        self.claimedSenderKeyId = ffi.has_claimed_sender ? ffi.claimed_sender_key_id : nil
        self.commitment = withUnsafeBytes(of: ffi.commitment) { Data($0) }
        self.payload = payload
    }
}

/// Where reading a private document type's store goes on from. Keep the
/// one a read returned and hand it back, as it came, as the next read's
/// `from`: Rust decides from it where that read starts.
///
/// It carries the largest store size a page proved so far, so a page from a
/// node behind does not end the reading early, the fingerprint of the keys
/// the reads used, so a read after the wallet's keys changed starts over
/// (`PrivateDocumentEntriesRead.restarted`), and the fingerprint of the
/// store read, so a read of another contract or type refuses it.
public struct PrivateDocumentReadCursor: Sendable, Equatable {
    /// The position the next read starts at.
    public let position: UInt64
    /// The largest store size a page proved, in the reads this cursor comes
    /// from; `nil` while no page was ever proved: the size is unknown, not
    /// 0.
    public let provedTotalCount: UInt64?
    /// The fingerprint of the keys the reads scanned with, 32 bytes. Opaque
    /// to Swift: it only crosses back.
    let keySet: Data
    /// The fingerprint of the store (network, contract id and type name) the
    /// reads read, 32 bytes. Opaque to Swift: it only crosses back.
    let store: Data

    init(ffi: PrivateDocumentCursorFFI) {
        self.position = ffi.position
        self.provedTotalCount = ffi.has_proved_total_count ? ffi.proved_total_count : nil
        self.keySet = withUnsafeBytes(of: ffi.key_set) { Data($0) }
        self.store = withUnsafeBytes(of: ffi.store) { Data($0) }
    }

    /// The cursor as the FFI takes it back.
    var ffi: PrivateDocumentCursorFFI {
        var cursor = PrivateDocumentCursorFFI()
        cursor.position = position
        cursor.proved_total_count = provedTotalCount ?? 0
        cursor.has_proved_total_count = provedTotalCount != nil
        withUnsafeMutableBytes(of: &cursor.key_set) { $0.copyBytes(from: keySet.prefix($0.count)) }
        withUnsafeMutableBytes(of: &cursor.store) { $0.copyBytes(from: store.prefix($0.count)) }
        return cursor
    }
}

/// What one read of a private document type's store found for the wallet.
/// Nothing of it is kept anywhere: hold it in view state.
public struct PrivateDocumentEntriesRead: Sendable, Equatable {
    /// The entries opened or recovered, in store order; at one position,
    /// the received one first.
    public let entries: [PrivateDocumentEntry]
    /// Where to read on from: pass it back as `from`. A store only grows, so
    /// a later read from there holds what was appended since.
    /// `cursor.provedTotalCount` is the largest store size proved so far,
    /// `nil` while unknown.
    public let cursor: PrivateDocumentReadCursor
    /// The wallet's keys changed since the `from` cursor, so the read
    /// started over at the store's start: its entries replace those of the
    /// earlier reads rather than follow them.
    public let restarted: Bool
    /// The store pages read.
    public let pagesRead: UInt32
    /// Entries that decrypted under one of the wallet's keys but are
    /// refused.
    public let rejectedCount: UInt32
    /// The wallet's identities the store was read for: those with an HD
    /// index and a key an entry can be sealed to. With none, nothing was
    /// read.
    public let identityCount: UInt32
    /// Keys of the wallet's identities an entry could be sealed to that the
    /// wallet does not derive at their identity key slot, such as a DashPay
    /// Connect application key: the read did not scan with them, so no entry
    /// sealed to them is in it, whether or not the read is complete.
    public let unscannedKeyCount: UInt32
    /// Whether the read reached the store's last entry, as large a store as
    /// any page proved, with every key but the unscanned ones
    /// (`unscannedKeyCount`). A later read from `cursor` reads the entries
    /// appended since. `false` with `pagesRead` 0 means nothing was read (a
    /// page budget of 0, or no identity with a key the wallet can scan with),
    /// and reading again changes nothing until that does.
    public let complete: Bool
}

extension ManagedPlatformWallet {
    /// Read the store of the private document type `documentTypeName` of
    /// `contractId` for every identity of this wallet, at most `maxPages`
    /// proved pages from the cursor `from`, or from the store's start when
    /// `from` is nil: the entries sealed to them, opened, and the entries
    /// they sealed, recovered from the store alone, each with its payload
    /// read against the type's schema.
    ///
    /// The keys are derived inside the Keychain mnemonic resolver Rust-side
    /// and stay there. The read keeps nothing: the caller keeps the
    /// returned `cursor` and passes it back, unchanged, to read on. When
    /// the wallet's keys changed since that cursor the read starts over
    /// and sets `restarted`. A failed page fetch fails the read; read again
    /// from the cursor you had. An entry's sender is what the entry claims
    /// (`PrivateDocumentEntry.claimedSenderIdentityId`): whether that
    /// identity wrote it is not checked here.
    ///
    /// Two reads read no page. With `maxPages` 0, or no identity holding a
    /// key with a path, it succeeds before the mnemonic resolver, the seed
    /// check, the protocol version check and any request: its success shows
    /// none of them. When keys have paths but none derives its registered
    /// key, the resolver is asked first, so the read throws as that fails (a
    /// locked Keychain, another wallet's mnemonic), then succeeds with no page.
    ///
    /// Throws `PlatformWalletError.invalidParameter` when `contractId` is
    /// not 32 bytes, when `documentTypeName` holds a NUL, when `from` was
    /// read from another store (another contract or type), or when the
    /// resolver's mnemonic is not this wallet's (before any page is read),
    /// `.protocolVersionNotActive` below protocol version 15, and the error
    /// the wallet reports when the contract declares no such private type.
    public func readPrivateDocumentEntries(
        contractId: Identifier,
        documentTypeName: String,
        from: PrivateDocumentReadCursor? = nil,
        maxPages: UInt32
    ) async throws -> PrivateDocumentEntriesRead {
        guard contractId.count == 32 else {
            throw PlatformWalletError.invalidParameter("contractId must be exactly 32 bytes")
        }
        // `withCString` would hand Rust only the part before a NUL.
        guard !documentTypeName.utf8.contains(0) else {
            throw PlatformWalletError.invalidParameter(
                "documentTypeName contains an embedded NUL byte")
        }
        let coreSigner = MnemonicResolver()
        return try await Task.detached(priority: .userInitiated) {
            [self] () -> PrivateDocumentEntriesRead in
            var read = PrivateDocumentReadFFI()
            let fromFFI = from?.ffi
            // Keep the wallet and the resolver alive across the synchronous
            // FFI call: the wallet's `deinit` destroys the handle the call
            // uses, and the resolver's vtable callback fires during it.
            let result = withExtendedLifetime((self, coreSigner)) {
                contractId.withFFIBytes { contractPtr in
                    documentTypeName.withCString { namePtr in
                        withOptionalPointer(to: fromFFI) { fromPtr in
                            platform_wallet_read_private_document_entries(
                                handle,
                                coreSigner.handle,
                                contractPtr,
                                namePtr,
                                fromPtr,
                                maxPages,
                                &read
                            )
                        }
                    }
                }
            }
            defer { platform_wallet_private_document_read_free(&read) }
            try result.check()
            var entries: [PrivateDocumentEntry] = []
            if let pointer = read.entries, read.entry_count > 0 {
                let buffer = UnsafeBufferPointer(start: pointer, count: Int(read.entry_count))
                entries.reserveCapacity(buffer.count)
                for entry in buffer {
                    entries.append(try PrivateDocumentEntry(ffi: entry))
                }
            }
            return PrivateDocumentEntriesRead(
                entries: entries,
                cursor: PrivateDocumentReadCursor(ffi: read.cursor),
                restarted: read.restarted,
                pagesRead: read.pages_read,
                rejectedCount: read.rejected_count,
                identityCount: read.identity_count,
                unscannedKeyCount: read.unscanned_key_count,
                complete: read.complete
            )
        }.value
    }
}

// MARK: - Creating an entry (protocol version 15)
//
// `PlatformWalletManager.createPrivateDocumentEntry` seals one entry of any
// private type from one of a wallet's identities to a key of any identity,
// in Rust and inside the Keychain mnemonic resolver, and appends it paying
// from a shielded account. Which sender key, which recipient keys qualify,
// the schema check, the fee and the outcome are Rust's; this file only
// marshals.

/// How a created entry tells its recipient who wrote it. A signed entry is
/// not offered: which keys may sign for a private type is not decided.
public enum PrivateDocumentSenderAuth: UInt8, Sendable, Equatable, CaseIterable {
    /// The entry names no sender.
    case anonymous = 0
    /// The entry names the sender's identity and encryption key, with a MAC
    /// only the sender and the recipient can make.
    case deniableMac = 1
}

/// A key of an identity an entry of a private document type may be sealed
/// to, as Rust decides it (an enabled ECDSA_SECP256K1 DECRYPTION or
/// ENCRYPTION key).
public struct PrivateDocumentRecipientKey: Sendable, Equatable, Hashable {
    /// The key's id.
    public let keyId: UInt32
    /// Its purpose as dpp numbers it: 1 ENCRYPTION, 2 DECRYPTION.
    public let purpose: UInt8

    /// The purpose's name, for display.
    public var purposeName: String {
        switch purpose {
        case 1: return "Encryption"
        case 2: return "Decryption"
        default: return "Purpose \(purpose)"
        }
    }
}

/// What a created entry is known by.
public struct PrivateDocumentEntryCreated: Sendable, Equatable {
    /// The entry's commitment, 32 bytes: a read of the store reports the
    /// entry with it (`PrivateDocumentEntry.commitment`).
    public let commitment: Data
    /// The sender's key the entry was sealed with: its outgoing viewing key
    /// recovers the entry, and a deniable MAC names it.
    public let senderKeyId: UInt32
}

extension PlatformWalletManager {
    /// The keys of `identityId`, as the network proves the identity now,
    /// that an entry of any private document type may be sealed to, by key
    /// id. An identity with no such key gives none.
    ///
    /// Throws `PlatformWalletError.identityNotFound` when the network holds
    /// no such identity.
    public func privateDocumentRecipientKeys(
        identityId: Identifier
    ) async throws -> [PrivateDocumentRecipientKey] {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle("PlatformWalletManager not configured")
        }
        guard identityId.count == 32 else {
            throw PlatformWalletError.invalidParameter("identityId must be exactly 32 bytes")
        }
        try admitNativeOp("privateDocumentRecipientKeys")
        defer { finishNativeOp() }
        let handle = self.handle
        return try await Task.detached(priority: .userInitiated) {
            () -> [PrivateDocumentRecipientKey] in
            var keys = PrivateDocumentRecipientKeysFFI()
            let result = identityId.withFFIBytes { idPtr in
                platform_wallet_manager_private_document_recipient_keys(handle, idPtr, &keys)
            }
            defer { platform_wallet_private_document_recipient_keys_free(&keys) }
            try result.check()
            guard let pointer = keys.keys, keys.count > 0 else { return [] }
            return UnsafeBufferPointer(start: pointer, count: Int(keys.count)).map {
                PrivateDocumentRecipientKey(keyId: $0.key_id, purpose: $0.purpose)
            }
        }.value
    }

    /// The fee, in credits, a create of `entryCount` entries of the private
    /// type `documentTypeName` of `contractId` reserves from the notes when
    /// it spends one or two of them, computed in Rust at the network's
    /// protocol version with the type's entry size: the least the spend
    /// pays.
    ///
    /// Throws `PlatformWalletError.invalidParameter` for a type that is not
    /// private or an entry count one create cannot append, and
    /// `.protocolVersionNotActive` below protocol version 15.
    public func privateDocumentCreateFee(
        contractId: Identifier,
        documentTypeName: String,
        entryCount: UInt32 = 1
    ) async throws -> UInt64 {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle("PlatformWalletManager not configured")
        }
        guard contractId.count == 32 else {
            throw PlatformWalletError.invalidParameter("contractId must be exactly 32 bytes")
        }
        guard !documentTypeName.utf8.contains(0) else {
            throw PlatformWalletError.invalidParameter(
                "documentTypeName contains an embedded NUL byte")
        }
        try admitNativeOp("privateDocumentCreateFee")
        defer { finishNativeOp() }
        let handle = self.handle
        return try await Task.detached(priority: .userInitiated) { () -> UInt64 in
            var fee: UInt64 = 0
            try contractId.withFFIBytes { contractPtr in
                try documentTypeName.withCString { namePtr in
                    try platform_wallet_manager_private_document_create_fee(
                        handle,
                        contractPtr,
                        namePtr,
                        entryCount,
                        &fee
                    ).check()
                }
            }
            return fee
        }.value
    }

    /// Create one entry of the private type `documentTypeName` of
    /// `contractId` from the wallet's identity `senderIdentityId` to the key
    /// `recipientKeyId` of `recipientIdentityId` (one of
    /// `privateDocumentRecipientKeys`), paying for it from `account`'s
    /// shielded notes: nothing on chain names either identity.
    ///
    /// `propertiesJSON` is the entry's properties as a document create takes
    /// them (byte arrays as hex, identifiers as base58); Rust checks them
    /// against the type's schema. The entry is sealed with the sender's
    /// encryption key inside `resolver`, which also supplies the Orchard
    /// spend authority, so `readPrivateDocumentEntries` on the sender's
    /// wallet recovers it as sent.
    ///
    /// Throws `PlatformWalletError.invalidParameter` or `.identityNotFound`
    /// with Rust's reason for what is refused before anything is paid (a
    /// sender without an encryption key, a recipient key an entry cannot be
    /// sealed to, properties the schema refuses), `.protocolVersionNotActive`
    /// below protocol version 15, and `.shieldedSpendUnconfirmed` once the
    /// entry may already be in the store: do not create it again.
    public func createPrivateDocumentEntry(
        walletId: Data,
        resolver: MnemonicResolver,
        account: UInt32 = 0,
        contractId: Identifier,
        documentTypeName: String,
        senderIdentityId: Identifier,
        recipientIdentityId: Identifier,
        recipientKeyId: UInt32,
        auth: PrivateDocumentSenderAuth,
        propertiesJSON: String
    ) async throws -> PrivateDocumentEntryCreated {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle("PlatformWalletManager not configured")
        }
        for (name, id) in [
            ("walletId", walletId),
            ("contractId", contractId),
            ("senderIdentityId", senderIdentityId),
            ("recipientIdentityId", recipientIdentityId),
        ] where id.count != 32 {
            throw PlatformWalletError.invalidParameter("\(name) must be exactly 32 bytes")
        }
        // `withCString` would hand Rust only the part before a NUL.
        guard !documentTypeName.utf8.contains(0), !propertiesJSON.utf8.contains(0) else {
            throw PlatformWalletError.invalidParameter(
                "documentTypeName or propertiesJSON contains an embedded NUL byte")
        }
        guard let resolverHandle = resolver.handle else {
            throw PlatformWalletError.invalidParameter("MnemonicResolver has no handle")
        }
        // Counted as the other native manager operations are, so a teardown
        // waits for this call to return before it frees the manager.
        try admitNativeOp("createPrivateDocumentEntry")
        defer { finishNativeOp() }
        let handle = self.handle
        return try await Task.detached(priority: .userInitiated) {
            () -> PrivateDocumentEntryCreated in
            var created = PrivateDocumentEntryCreatedFFI()
            // Guaranteed resolver keepalive across the FFI call, whose
            // vtable callback fires during it.
            try withExtendedLifetime(resolver) {
                try walletId.withFFIBytes { walletPtr in
                    try contractId.withFFIBytes { contractPtr in
                        try senderIdentityId.withFFIBytes { senderPtr in
                            try recipientIdentityId.withFFIBytes { recipientPtr in
                                try documentTypeName.withCString { namePtr in
                                    try propertiesJSON.withCString { propertiesPtr in
                                        try platform_wallet_manager_create_private_document_entry(
                                            handle,
                                            walletPtr,
                                            resolverHandle,
                                            account,
                                            contractPtr,
                                            namePtr,
                                            senderPtr,
                                            recipientPtr,
                                            recipientKeyId,
                                            auth.rawValue,
                                            propertiesPtr,
                                            &created
                                        ).check()
                                    }
                                }
                            }
                        }
                    }
                }
            }
            return PrivateDocumentEntryCreated(
                commitment: withUnsafeBytes(of: created.commitment) { Data($0) },
                senderKeyId: created.sender_key_id
            )
        }.value
    }
}

/// `body` with a pointer to `value`, or with nil when it is nil.
private func withOptionalPointer<T, R>(
    to value: T?,
    _ body: (UnsafePointer<T>?) throws -> R
) rethrows -> R {
    guard var value else { return try body(nil) }
    return try withUnsafePointer(to: &value) { try body($0) }
}
