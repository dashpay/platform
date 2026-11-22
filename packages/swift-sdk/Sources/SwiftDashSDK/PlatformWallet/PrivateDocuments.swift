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
/// node behind does not end the reading early, and the fingerprint of the
/// keys the reads used, so a read after the wallet's keys changed starts
/// over (`PrivateDocumentEntriesRead.restarted`).
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

    init(ffi: PrivateDocumentCursorFFI) {
        self.position = ffi.position
        self.provedTotalCount = ffi.has_proved_total_count ? ffi.proved_total_count : nil
        self.keySet = withUnsafeBytes(of: ffi.key_set) { Data($0) }
    }

    /// The cursor as the FFI takes it back.
    var ffi: PrivateDocumentCursorFFI {
        var cursor = PrivateDocumentCursorFFI()
        cursor.position = position
        cursor.proved_total_count = provedTotalCount ?? 0
        cursor.has_proved_total_count = provedTotalCount != nil
        withUnsafeMutableBytes(of: &cursor.key_set) { $0.copyBytes(from: keySet.prefix($0.count)) }
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
    /// Whether the read reached the store's last entry, as large a store as
    /// any page proved. A later read from `cursor` reads the entries
    /// appended since. `false` with `pagesRead` 0 means nothing was read (no
    /// identity can open entries, or a page budget of 0), and reading again
    /// changes nothing until that does.
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
    /// Throws `PlatformWalletError.invalidParameter` when `contractId` is
    /// not 32 bytes, when `documentTypeName` holds a NUL, or when the
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
        let handle = self.handle
        let coreSigner = MnemonicResolver()
        return try await Task.detached(priority: .userInitiated) {
            () -> PrivateDocumentEntriesRead in
            var read = PrivateDocumentReadFFI()
            let fromFFI = from?.ffi
            // Keep the resolver alive across the synchronous FFI call: its
            // vtable callback fires during it.
            let result = withExtendedLifetime(coreSigner) {
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
                complete: read.complete
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
