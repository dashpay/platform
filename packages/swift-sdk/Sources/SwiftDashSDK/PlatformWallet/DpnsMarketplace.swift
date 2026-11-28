import Foundation
import DashSDKFFI

// MARK: - Value types

/// A DPNS name read off Platform with its marketplace state: the domain
/// document id every trade transition needs, and the listed `$price`.
///
/// Prices are **credits** (1 duff = 1000 credits). `priceCredits == nil`
/// means the name is NOT for sale — distinct from a 0-credit listing.
/// Timestamps are Unix milliseconds and `nil` when the document doesn't
/// carry them, so a UI shows "unknown" rather than the epoch.
public struct DpnsMarketplaceName: Sendable, Equatable {
    /// The DPNS `domain` document id — stable across transfers and
    /// purchases.
    public let documentId: Data
    /// The document's owner: the identity that owns (and may sell) the
    /// name.
    public let ownerId: Data
    /// `records.identity` — the identity the name resolves to. The
    /// protocol rewrites it to the new owner on purchase/transfer.
    public let recordsIdentityId: Data?
    /// Display label, e.g. "Alice".
    public let label: String
    /// Homograph-normalized label, e.g. "a11ce".
    public let normalizedLabel: String
    /// Listed sale price in credits. `nil` = not for sale.
    public let priceCredits: UInt64?
    /// Document `$createdAt` in Unix ms, when carried.
    public let createdAtMs: UInt64?
    /// Document `$updatedAt` in Unix ms — bumps on price changes.
    public let updatedAtMs: UInt64?
    /// Document `$transferredAt` in Unix ms — set on purchase/transfer.
    public let transferredAtMs: UInt64?

    public init(
        documentId: Data,
        ownerId: Data,
        recordsIdentityId: Data?,
        label: String,
        normalizedLabel: String,
        priceCredits: UInt64?,
        createdAtMs: UInt64?,
        updatedAtMs: UInt64?,
        transferredAtMs: UInt64?
    ) {
        self.documentId = documentId
        self.ownerId = ownerId
        self.recordsIdentityId = recordsIdentityId
        self.label = label
        self.normalizedLabel = normalizedLabel
        self.priceCredits = priceCredits
        self.createdAtMs = createdAtMs
        self.updatedAtMs = updatedAtMs
        self.transferredAtMs = transferredAtMs
    }
}

/// Where a tracked DPNS name stands relative to the wallet identity that
/// owned it. `Sold` / `Transferred` rows are retained (not deleted) so
/// the host can surface "your name was sold" affordances.
public enum DpnsNameSaleStatus: Sendable, Equatable {
    /// The wallet identity still owns the name.
    case owned
    /// The name left through a purchase; the associated value is the
    /// buyer.
    case sold(to: Data)
    /// The name left through a plain transfer (gift / off-market
    /// handover); the associated value is the recipient.
    case transferred(to: Data)
}

/// One locally persisted marketplace row: a name tracked for a wallet
/// identity, with its last-known sale state.
///
/// Unlike ``DpnsMarketplaceName`` this is the wallet's own bookkeeping
/// (no network read), so it names the wallet identity and — for names
/// that already left — the counterparty, rather than the live document's
/// owner.
public struct DpnsNameStateRow: Sendable, Equatable {
    /// The DPNS `domain` document id — this row's key.
    public let documentId: Data
    /// The wallet identity this row is tracked for. For `.owned` rows the
    /// current owner; otherwise the previous owner (ours).
    public let walletIdentityId: Data
    /// Display label, e.g. "Alice".
    public let label: String
    /// Homograph-normalized label, e.g. "a11ce".
    public let normalizedLabel: String
    /// Last-known listed price in credits. `nil` = not for sale.
    public let priceCredits: UInt64?
    /// Ownership status relative to `walletIdentityId`.
    public let status: DpnsNameSaleStatus
    /// Document `$createdAt` in Unix ms, when carried.
    public let createdAtMs: UInt64?
    /// Document `$updatedAt` in Unix ms, when carried.
    public let updatedAtMs: UInt64?
    /// Document `$transferredAt` in Unix ms, when carried.
    public let transferredAtMs: UInt64?
    /// Unix ms of the sync pass / confirmed transition that wrote this
    /// row.
    public let lastSyncedAtMs: UInt64

    public init(
        documentId: Data,
        walletIdentityId: Data,
        label: String,
        normalizedLabel: String,
        priceCredits: UInt64?,
        status: DpnsNameSaleStatus,
        createdAtMs: UInt64?,
        updatedAtMs: UInt64?,
        transferredAtMs: UInt64?,
        lastSyncedAtMs: UInt64
    ) {
        self.documentId = documentId
        self.walletIdentityId = walletIdentityId
        self.label = label
        self.normalizedLabel = normalizedLabel
        self.priceCredits = priceCredits
        self.status = status
        self.createdAtMs = createdAtMs
        self.updatedAtMs = updatedAtMs
        self.transferredAtMs = transferredAtMs
        self.lastSyncedAtMs = lastSyncedAtMs
    }
}

/// One event in a DPNS name's trade timeline, assembled from the
/// Document History system contract plus the domain document's own
/// creation time. Prices are credits; `atMs` is Unix milliseconds.
public enum DpnsNameHistoryEvent: Sendable, Equatable {
    /// The domain document was registered.
    case registered(atMs: UInt64)
    /// The owner listed or re-priced the name.
    case priceSet(price: UInt64, atMs: UInt64, blockHeight: UInt64?)
    /// The name was purchased: `seller` received `price` credits from
    /// `buyer`, who became the owner.
    case purchased(price: UInt64, seller: Data, buyer: Data, atMs: UInt64, blockHeight: UInt64?)
    /// The name was transferred without payment — a gift/handover, or a
    /// transfer-to-self delist when `from == to`.
    case transferred(from: Data, to: Data, atMs: UInt64, blockHeight: UInt64?)

    /// Block time of the event in Unix ms, whatever the case.
    public var atMs: UInt64 {
        switch self {
        case .registered(let atMs):
            return atMs
        case .priceSet(_, let atMs, _), .transferred(_, _, let atMs, _):
            return atMs
        case .purchased(_, _, _, let atMs, _):
            return atMs
        }
    }
}

/// Per-pass delta returned by
/// ``ManagedPlatformWallet/syncDpnsMarketplace()``.
public struct DpnsMarketplaceSyncSummary: Sendable, Equatable {
    /// Owned-name rows refreshed this pass.
    public let tracked: UInt32
    /// Labels newly observed on a wallet identity.
    public let added: UInt32
    /// Names that left a wallet identity (sold or transferred away).
    public let departed: UInt32
    /// Listed-price changes since the previous pass.
    public let pricesChanged: UInt32
    /// Labels newly observed, with the wallet identity that owns each one.
    public let addedNames: [DpnsNameAdded]
    /// Names that left a wallet identity, including the classified destination
    /// when the wallet could establish it.
    public let departedNames: [DpnsNameDeparture]
    /// Full before/after values for listings whose price changed.
    public let priceChanges: [DpnsPriceChange]
    /// Unix milliseconds when this pass completed.
    public let syncUnixMs: UInt64

    public init(
        tracked: UInt32,
        added: UInt32,
        departed: UInt32,
        pricesChanged: UInt32,
        addedNames: [DpnsNameAdded] = [],
        departedNames: [DpnsNameDeparture] = [],
        priceChanges: [DpnsPriceChange] = [],
        syncUnixMs: UInt64 = 0
    ) {
        self.tracked = tracked
        self.added = added
        self.departed = departed
        self.pricesChanged = pricesChanged
        self.addedNames = addedNames
        self.departedNames = departedNames
        self.priceChanges = priceChanges
        self.syncUnixMs = syncUnixMs
    }
}

/// One label newly observed by a marketplace sync pass.
public struct DpnsNameAdded: Sendable, Equatable {
    public let identityId: Data
    public let label: String

    public init(identityId: Data, label: String) {
        self.identityId = identityId
        self.label = label
    }
}

/// One name that left a wallet identity during marketplace sync.
public struct DpnsNameDeparture: Sendable, Equatable {
    public let identityId: Data
    public let label: String
    public let documentId: Data?
    /// `.sold` or `.transferred` when the destination was classifiable;
    /// nil when the sweep could only establish that the name departed.
    public let status: DpnsNameSaleStatus?

    public init(
        identityId: Data,
        label: String,
        documentId: Data?,
        status: DpnsNameSaleStatus?
    ) {
        self.identityId = identityId
        self.label = label
        self.documentId = documentId
        self.status = status
    }
}

/// One listing-price change observed during marketplace sync.
public struct DpnsPriceChange: Sendable, Equatable {
    public let documentId: Data
    public let label: String
    public let previousPriceCredits: UInt64?
    public let currentPriceCredits: UInt64?

    public init(
        documentId: Data,
        label: String,
        previousPriceCredits: UInt64?,
        currentPriceCredits: UInt64?
    ) {
        self.documentId = documentId
        self.label = label
        self.previousPriceCredits = previousPriceCredits
        self.currentPriceCredits = currentPriceCredits
    }
}

/// The way the identity holding a DPNS name gives it up, from Platform
/// protocol version 15.
public enum DpnsNameGiveUpMethod: Sendable, Equatable, CaseIterable {
    /// The name's `domain` document is deleted. The name is free again:
    /// anybody, someone else included, may register it.
    case delete
    /// A `tombstone` document takes the name's place. The name is gone and
    /// nobody can register it again. This can not be undone, by anybody,
    /// the identity that created it included.
    case tombstone

    /// The byte the FFI takes and returns for this method
    /// (`DPNS_NAME_GIVE_UP_METHOD_*`).
    var ffiValue: UInt8 {
        switch self {
        case .delete:
            return UInt8(DPNS_NAME_GIVE_UP_METHOD_DELETE)
        case .tombstone:
            return UInt8(DPNS_NAME_GIVE_UP_METHOD_TOMBSTONE)
        }
    }

    /// The method an FFI byte names, or nil for a byte this build does not
    /// know.
    init?(ffiValue: UInt8) {
        switch ffiValue {
        case UInt8(DPNS_NAME_GIVE_UP_METHOD_DELETE):
            self = .delete
        case UInt8(DPNS_NAME_GIVE_UP_METHOD_TOMBSTONE):
            self = .tombstone
        default:
            return nil
        }
    }
}

/// Which ways the network accepts for giving up one name a wallet identity
/// holds (``ManagedPlatformWallet/dpnsNameGiveUpChoices(ownerIdentityId:name:)``).
///
/// The rule behind the two flags is the protocol's, applied in Rust: a
/// contested name can only be tombstoned, any other name either way. A host
/// offers exactly the ways whose flag is `true` and decides nothing itself.
public struct DpnsNameGiveUpChoices: Sendable, Equatable {
    /// The name's `domain` document id.
    public let documentId: Data
    /// Whether the network accepts ``DpnsNameGiveUpMethod/delete`` for this
    /// name.
    public let canDelete: Bool
    /// Whether the network accepts ``DpnsNameGiveUpMethod/tombstone`` for
    /// this name.
    public let canTombstone: Bool

    public init(documentId: Data, canDelete: Bool, canTombstone: Bool) {
        self.documentId = documentId
        self.canDelete = canDelete
        self.canTombstone = canTombstone
    }
}

/// A DPNS name given up, as the network confirmed it
/// (``ManagedPlatformWallet/giveUpDpnsName(ownerIdentityId:name:expectedDocumentId:method:signer:)``).
public struct DpnsNameGivenUp: Sendable, Equatable {
    /// The `domain` document checked before signing. A delete removed exactly
    /// this one. A tombstone consumed the owner's domain of the name at
    /// execution, which is this one unless the name was deleted and
    /// registered again in the moments between.
    public let documentId: Data
    /// How the name was given up.
    public let method: DpnsNameGiveUpMethod
    /// The id of the tombstone the network stored; set exactly for
    /// ``DpnsNameGiveUpMethod/tombstone``.
    public let tombstoneId: Data?

    public init(documentId: Data, method: DpnsNameGiveUpMethod, tombstoneId: Data?) {
        self.documentId = documentId
        self.method = method
        self.tombstoneId = tombstoneId
    }
}

// MARK: - FFI decoding

extension DpnsNameGiveUpChoices {
    init(ffi: DpnsNameGiveUpChoicesFFI) {
        var documentTuple = ffi.document_id
        self.init(
            documentId: Swift.withUnsafeBytes(of: &documentTuple) { Data($0) },
            canDelete: ffi.can_delete,
            canTombstone: ffi.can_tombstone
        )
    }
}

extension DpnsNameGivenUp {
    /// Copy a confirmed give-up into an owned Swift value. Returns nil for
    /// a method byte this build does not know, or for a tombstone id that
    /// does not match the method (a tombstone without one, a delete with
    /// one): the result is not presented as something this build cannot
    /// establish.
    init?(ffi: DpnsNameGivenUpFFI) {
        guard let method = DpnsNameGiveUpMethod(ffiValue: ffi.method) else { return nil }
        var documentTuple = ffi.document_id
        var tombstoneTuple = ffi.tombstone_id
        let tombstoneId: Data? = ffi.has_tombstone
            ? Swift.withUnsafeBytes(of: &tombstoneTuple) { Data($0) }
            : nil
        switch (method, tombstoneId) {
        case (.delete, .none), (.tombstone, .some):
            break
        default:
            return nil
        }
        self.init(
            documentId: Swift.withUnsafeBytes(of: &documentTuple) { Data($0) },
            method: method,
            tombstoneId: tombstoneId
        )
    }
}

extension DpnsMarketplaceName {
    /// Copy a Rust-owned row into an owned Swift value. Every `has_*`
    /// flag gates its field: a `false` flag becomes `nil`, never the
    /// zero the FFI struct happens to hold. Zero timestamps mean
    /// "unknown" on this boundary and decode to `nil` for the same
    /// reason.
    ///
    /// Must be called while the Rust allocation is still alive — the
    /// label strings are copied here, not retained.
    init(ffi: DpnsMarketplaceNameFFI) {
        var documentTuple = ffi.document_id
        var ownerTuple = ffi.owner_id
        var recordsTuple = ffi.records_identity_id
        self.init(
            documentId: Swift.withUnsafeBytes(of: &documentTuple) { Data($0) },
            ownerId: Swift.withUnsafeBytes(of: &ownerTuple) { Data($0) },
            recordsIdentityId: ffi.has_records_identity
                ? Swift.withUnsafeBytes(of: &recordsTuple) { Data($0) }
                : nil,
            label: ffi.label.map { String(cString: $0) } ?? "",
            normalizedLabel: ffi.normalized_label.map { String(cString: $0) } ?? "",
            priceCredits: ffi.has_price ? ffi.price : nil,
            createdAtMs: ffi.created_at_ms == 0 ? nil : ffi.created_at_ms,
            updatedAtMs: ffi.updated_at_ms == 0 ? nil : ffi.updated_at_ms,
            transferredAtMs: ffi.transferred_at_ms == 0 ? nil : ffi.transferred_at_ms
        )
    }
}

extension DpnsNameStateRow {
    /// Copy a Rust-owned persisted row into an owned Swift value.
    ///
    /// The status/counterparty pair is one invariant at this boundary:
    /// owned rows must not carry a counterparty, while sold/transferred
    /// rows must carry one. Unknown or inconsistent rows fail closed and
    /// are omitted by the collection wrapper rather than being presented
    /// as an ownership state this Swift build cannot establish.
    init?(ffi: DpnsNameStateRowFFI) {
        var documentTuple = ffi.document_id
        var walletIdentityTuple = ffi.wallet_identity_id
        var counterpartyTuple = ffi.counterparty_id
        let counterparty: Data? = ffi.has_counterparty
            ? Swift.withUnsafeBytes(of: &counterpartyTuple) { Data($0) }
            : nil
        let status: DpnsNameSaleStatus
        switch (ffi.status, counterparty) {
        case (0, .none):
            status = .owned
        case (1, .some(let to)):
            status = .sold(to: to)
        case (2, .some(let to)):
            status = .transferred(to: to)
        default:
            return nil
        }
        self.init(
            documentId: Swift.withUnsafeBytes(of: &documentTuple) { Data($0) },
            walletIdentityId: Swift.withUnsafeBytes(of: &walletIdentityTuple) { Data($0) },
            label: ffi.label.map { String(cString: $0) } ?? "",
            normalizedLabel: ffi.normalized_label.map { String(cString: $0) } ?? "",
            priceCredits: ffi.has_price ? ffi.price : nil,
            status: status,
            createdAtMs: ffi.created_at_ms == 0 ? nil : ffi.created_at_ms,
            updatedAtMs: ffi.updated_at_ms == 0 ? nil : ffi.updated_at_ms,
            transferredAtMs: ffi.transferred_at_ms == 0 ? nil : ffi.transferred_at_ms,
            lastSyncedAtMs: ffi.last_synced_at_ms
        )
    }
}

extension DpnsNameAdded {
    init(ffi: DpnsNameAddedFFI) {
        var identityTuple = ffi.identity_id
        self.init(
            identityId: Swift.withUnsafeBytes(of: &identityTuple) { Data($0) },
            label: ffi.label.map { String(cString: $0) } ?? ""
        )
    }
}

extension DpnsNameDeparture {
    init(ffi: DpnsNameDepartedFFI) {
        var identityTuple = ffi.identity_id
        var documentTuple = ffi.document_id
        var counterpartyTuple = ffi.counterparty_id
        let status: DpnsNameSaleStatus?
        switch (ffi.has_status, ffi.status) {
        case (true, 1):
            status = .sold(to: Swift.withUnsafeBytes(of: &counterpartyTuple) { Data($0) })
        case (true, 2):
            status = .transferred(
                to: Swift.withUnsafeBytes(of: &counterpartyTuple) { Data($0) }
            )
        default:
            status = nil
        }
        self.init(
            identityId: Swift.withUnsafeBytes(of: &identityTuple) { Data($0) },
            label: ffi.label.map { String(cString: $0) } ?? "",
            documentId: ffi.has_document_id
                ? Swift.withUnsafeBytes(of: &documentTuple) { Data($0) }
                : nil,
            status: status
        )
    }
}

extension DpnsPriceChange {
    init(ffi: DpnsPriceChangeFFI) {
        var documentTuple = ffi.document_id
        self.init(
            documentId: Swift.withUnsafeBytes(of: &documentTuple) { Data($0) },
            label: ffi.label.map { String(cString: $0) } ?? "",
            previousPriceCredits: ffi.has_previous ? ffi.previous : nil,
            currentPriceCredits: ffi.has_current ? ffi.current : nil
        )
    }
}

extension DpnsNameHistoryEvent {
    /// Copy a Rust-owned timeline row into an owned Swift value.
    /// Returns `nil` for a `kind` byte this build doesn't know, or for a
    /// row missing a payload its kind requires — an unreadable event is
    /// dropped from the timeline rather than rendered with invented
    /// values.
    init?(ffi: DpnsNameHistoryEventFFI) {
        var fromTuple = ffi.from_id
        var toTuple = ffi.to_id
        let from: Data? = ffi.has_from
            ? Swift.withUnsafeBytes(of: &fromTuple) { Data($0) }
            : nil
        let to: Data? = ffi.has_to ? Swift.withUnsafeBytes(of: &toTuple) { Data($0) } : nil
        let blockHeight: UInt64? = ffi.has_block_height ? ffi.block_height : nil
        let price: UInt64? = ffi.has_price ? ffi.price : nil

        switch ffi.kind {
        case 0:
            self = .registered(atMs: ffi.at_ms)
        case 1:
            guard let price else { return nil }
            self = .priceSet(price: price, atMs: ffi.at_ms, blockHeight: blockHeight)
        case 2:
            guard let price, let from, let to else { return nil }
            self = .purchased(
                price: price,
                seller: from,
                buyer: to,
                atMs: ffi.at_ms,
                blockHeight: blockHeight
            )
        case 3:
            guard let from, let to else { return nil }
            self = .transferred(from: from, to: to, atMs: ffi.at_ms, blockHeight: blockHeight)
        default:
            return nil
        }
    }
}

// MARK: - ManagedPlatformWallet operations

extension ManagedPlatformWallet {
    /// Prefix-search DPNS names on Platform, with each hit's full
    /// marketplace state (document id, owner, `$price`, timestamps).
    ///
    /// An empty `prefix` is a valid alphabetical browse. `limit == 0`
    /// uses the wallet's default page size. `startAfter` is the cursor:
    /// pass the previous page's last `documentId` to fetch the next page.
    ///
    /// There is no server-side price filter or ordering — `$price` is not
    /// an indexable system property on Dash Platform, so a global
    /// "everything for sale, cheapest first" query is not buildable at
    /// any layer today. The marketplace is search-driven.
    public func searchDpnsMarketplace(
        prefix: String,
        limit: UInt32 = 0,
        startAfter: Data? = nil
    ) async throws -> [DpnsMarketplaceName] {
        let handle = self.handle
        let cursorBytes = try Self.validatedOptionalIdentifier(
            startAfter,
            parameter: "startAfter"
        )
        return try await Task.detached(priority: .userInitiated) { () -> [DpnsMarketplaceName] in
            var outPtr: UnsafeMutablePointer<DpnsMarketplaceNameFFI>? = nil
            var outCount: UInt = 0
            let result = prefix.withCString { prefixPtr -> PlatformWalletFFIResult in
                Self.withOptionalBytes(cursorBytes) { cursorPtr in
                    platform_wallet_dpns_marketplace_search(
                        handle,
                        prefixPtr,
                        limit,
                        cursorPtr,
                        &outPtr,
                        &outCount
                    )
                }
            }
            try result.check()
            guard let ptr = outPtr, outCount > 0 else { return [] }
            defer { dpns_marketplace_names_free(ptr, outCount) }
            return (0..<Int(outCount)).map { DpnsMarketplaceName(ffi: ptr[$0]) }
        }.value
    }

    /// Fetch the authoritative marketplace state of a single DPNS name
    /// (`"alice"` or `"alice.dash"`).
    ///
    /// Returns `nil` when the name is not registered — an expected
    /// outcome, not an error. A name hidden inside an active
    /// contested-name vote is also absent from the documents tree; the
    /// trade operations classify that case into
    /// `PlatformWalletError.contestedNameNotTradable`, this read does
    /// not.
    public func dpnsMarketplaceNameState(name: String) async throws -> DpnsMarketplaceName? {
        let handle = self.handle
        return try await Task.detached(priority: .userInitiated) { () -> DpnsMarketplaceName? in
            var outPtr: UnsafeMutablePointer<DpnsMarketplaceNameFFI>? = nil
            let result = name.withCString { namePtr in
                platform_wallet_dpns_marketplace_name_state(handle, namePtr, &outPtr)
            }
            try result.check()
            guard let ptr = outPtr else { return nil }
            defer { dpns_marketplace_name_free(ptr) }
            return DpnsMarketplaceName(ffi: ptr.pointee)
        }.value
    }

    /// This wallet's locally persisted marketplace rows — owned names
    /// with their sale state, plus retained sold/transferred rows.
    ///
    /// Pass `identityId` to filter to one wallet identity, or `nil` for
    /// every identity in the wallet. Reads the in-memory working set: no
    /// network round-trip, so this is the cheap read behind a "my names"
    /// screen. Refresh it with ``syncDpnsMarketplace()``.
    public func myDpnsMarketplaceNames(
        identityId: Data? = nil
    ) async throws -> [DpnsNameStateRow] {
        let handle = self.handle
        let idBytes = try Self.validatedOptionalIdentifier(
            identityId,
            parameter: "identityId"
        )
        return try await Task.detached(priority: .userInitiated) { () -> [DpnsNameStateRow] in
            var outPtr: UnsafeMutablePointer<DpnsNameStateRowFFI>? = nil
            var outCount: UInt = 0
            let result = Self.withOptionalBytes(idBytes) { idPtr in
                platform_wallet_dpns_marketplace_my_names(handle, idPtr, &outPtr, &outCount)
            }
            try result.check()
            guard let ptr = outPtr, outCount > 0 else { return [] }
            defer { dpns_name_state_rows_free(ptr, outCount) }
            return (0..<Int(outCount)).compactMap { DpnsNameStateRow(ffi: ptr[$0]) }
        }.value
    }

    /// The trade timeline of a DPNS name: registration, price changes,
    /// purchases (with price and counterparties), and transfers — ordered
    /// by block time ascending.
    ///
    /// Works for names that already left the wallet. Events this build
    /// cannot decode are dropped rather than rendered with invented
    /// values, so the returned array can be shorter than the FFI's.
    public func dpnsNameHistory(name: String) async throws -> [DpnsNameHistoryEvent] {
        let handle = self.handle
        return try await Task.detached(priority: .userInitiated) { () -> [DpnsNameHistoryEvent] in
            var outPtr: UnsafeMutablePointer<DpnsNameHistoryEventFFI>? = nil
            var outCount: UInt = 0
            let result = name.withCString { namePtr in
                platform_wallet_dpns_name_history(handle, namePtr, &outPtr, &outCount)
            }
            try result.check()
            guard let ptr = outPtr, outCount > 0 else { return [] }
            defer { dpns_name_history_events_free(ptr, outCount) }
            return (0..<Int(outCount)).compactMap { DpnsNameHistoryEvent(ffi: ptr[$0]) }
        }.value
    }

    /// List (or re-price) a DPNS name for sale at `priceCredits`.
    ///
    /// The Rust side resolves the name authoritatively, checks ownership,
    /// auto-selects the AUTHENTICATION + ECDSA signing key on the owner,
    /// broadcasts, and records the sale state from the CONFIRMED
    /// document — which is what this returns.
    ///
    /// Lifetime contract: the `signer` instance MUST stay alive for the
    /// duration of the synchronous FFI call inside this async wrapper
    /// (Rust holds a `passUnretained` ctx pointer to the underlying
    /// `KeychainSigner`). The wrapper pins it with
    /// `withExtendedLifetime(signer)` around the full marshalling chain —
    /// a bare `_ = signer` is unreliable (the optimizer may elide it).
    @discardableResult
    public func setDpnsNamePrice(
        ownerIdentityId: Identifier,
        name: String,
        priceCredits: UInt64,
        signer: KeychainSigner
    ) async throws -> DpnsMarketplaceName {
        let handle = self.handle
        let signerHandle = signer.handle
        let ownerBytes: [UInt8] = ownerIdentityId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        return try await Task.detached(priority: .userInitiated) { () -> DpnsMarketplaceName in
            var outPtr: UnsafeMutablePointer<DpnsMarketplaceNameFFI>? = nil
            let result = withExtendedLifetime(signer) {
                ownerBytes.withUnsafeBufferPointer { ownerBp -> PlatformWalletFFIResult in
                    name.withCString { namePtr in
                        platform_wallet_dpns_set_name_price(
                            handle,
                            ownerBp.baseAddress!,
                            namePtr,
                            priceCredits,
                            signerHandle,
                            &outPtr
                        )
                    }
                }
            }
            try result.check()
            return try Self.takeConfirmedState(outPtr, operation: "dpns_set_name_price")
        }.value
    }

    /// Delist a DPNS name — remove its price while keeping ownership.
    ///
    /// Broadcasts a transfer to the owner's OWN identity: consensus
    /// strips `$price` on transfer, and DPNS has no dedicated
    /// remove-price transition. The Rust side verifies the confirmed
    /// document actually carries no price before recording the delist, so
    /// a consensus change fails loudly instead of persisting a delist
    /// that didn't happen.
    ///
    /// Same signer lifetime contract as ``setDpnsNamePrice(ownerIdentityId:name:priceCredits:signer:)``.
    @discardableResult
    public func delistDpnsName(
        ownerIdentityId: Identifier,
        name: String,
        signer: KeychainSigner
    ) async throws -> DpnsMarketplaceName {
        let handle = self.handle
        let signerHandle = signer.handle
        let ownerBytes: [UInt8] = ownerIdentityId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        return try await Task.detached(priority: .userInitiated) { () -> DpnsMarketplaceName in
            var outPtr: UnsafeMutablePointer<DpnsMarketplaceNameFFI>? = nil
            let result = withExtendedLifetime(signer) {
                ownerBytes.withUnsafeBufferPointer { ownerBp -> PlatformWalletFFIResult in
                    name.withCString { namePtr in
                        platform_wallet_dpns_delist_name(
                            handle,
                            ownerBp.baseAddress!,
                            namePtr,
                            signerHandle,
                            &outPtr
                        )
                    }
                }
            }
            try result.check()
            return try Self.takeConfirmedState(outPtr, operation: "dpns_delist_name")
        }.value
    }

    /// Which ways the network accepts for `ownerIdentityId` to give up
    /// `name` (`"alice"` or `"alice.dash"`). Nothing is signed or
    /// broadcast.
    ///
    /// Offer exactly the ways whose flag is `true`: a contested name can
    /// only be tombstoned, which Rust decides and reports here as
    /// `canDelete == false`. A delete frees the name for anybody to register
    /// again; a tombstone can not be undone, by anybody.
    ///
    /// Needs Platform protocol version 15. On a network proven below it
    /// this throws `PlatformWalletError.protocolVersionNotActive` and no
    /// name can be given up there; an SDK pinned below 15 throws
    /// `.sdkPinnedBelowProtocolVersion`. Throws `.notFound` when the name is
    /// not registered and `.invalidParameter` when another identity holds it.
    public func dpnsNameGiveUpChoices(
        ownerIdentityId: Identifier,
        name: String
    ) async throws -> DpnsNameGiveUpChoices {
        try Self.requireGiveUpInputs(name: name, identifiers: [("ownerIdentityId", ownerIdentityId)])
        let handle = self.handle
        let ownerBytes: [UInt8] = ownerIdentityId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        return try await Task.detached(priority: .userInitiated) { [self] () -> DpnsNameGiveUpChoices in
            var out = DpnsNameGiveUpChoicesFFI()
            // Keep this wallet (and so its handle) alive until Rust has looked it up
            let result = withExtendedLifetime(self) {
                ownerBytes.withUnsafeBufferPointer { ownerBp -> PlatformWalletFFIResult in
                    name.withCString { namePtr in
                        platform_wallet_dpns_name_give_up_choices(
                            handle,
                            ownerBp.baseAddress!,
                            namePtr,
                            &out
                        )
                    }
                }
            }
            try result.check()
            return DpnsNameGiveUpChoices(ffi: out)
        }.value
    }

    /// Give up `name`, held by `ownerIdentityId`, by `method`.
    ///
    /// ``DpnsNameGiveUpMethod/delete`` frees the name: anybody may register
    /// it again. ``DpnsNameGiveUpMethod/tombstone`` removes it for good:
    /// nobody can register it again and nobody can undo the tombstone, its
    /// creator included. A contested name can only be tombstoned; ask
    /// ``dpnsNameGiveUpChoices(ownerIdentityId:name:)`` first and offer only
    /// what it allows. Rust reads the name again before signing and throws
    /// `.invalidParameter` for a method the network does not accept, and for
    /// a name that is no longer `expectedDocumentId` (the
    /// ``DpnsNameGiveUpChoices/documentId`` the user was shown): a replacement
    /// registration that read detects is refused before signing. A delete
    /// targets that document, so the network refuses it once the document is
    /// gone. A tombstone is bound to the owner and name, not the document: if
    /// the name is replaced between that read and execution, the tombstone
    /// consumes the replacement (see ``DpnsNameGivenUp/documentId``).
    ///
    /// `.transactionBroadcastUnconfirmed` means the transition may have been
    /// delivered: do not offer another give-up of the name until a read shows
    /// whether it was.
    ///
    /// Needs Platform protocol version 15. On a network proven below it
    /// this throws `PlatformWalletError.protocolVersionNotActive` and
    /// nothing is signed. Also throws `.notFound` (name not registered),
    /// `.invalidParameter` (held by another identity) and
    /// `.signingKeyUnavailable`.
    ///
    /// The wallet's stored state changes only after the network confirmed
    /// the name gone, and the row disappears from views observing
    /// `PersistentDPNSName` once that cleanup is persisted. If the backend
    /// refuses the cleanup, the give-up is still reported as confirmed and
    /// the name stays listed until the next Name Marketplace sync removes it.
    ///
    /// Same signer lifetime contract as ``setDpnsNamePrice(ownerIdentityId:name:priceCredits:signer:)``.
    @discardableResult
    public func giveUpDpnsName(
        ownerIdentityId: Identifier,
        name: String,
        expectedDocumentId: Data,
        method: DpnsNameGiveUpMethod,
        signer: KeychainSigner
    ) async throws -> DpnsNameGivenUp {
        try Self.requireGiveUpInputs(
            name: name,
            identifiers: [
                ("ownerIdentityId", ownerIdentityId),
                ("expectedDocumentId", expectedDocumentId),
            ]
        )
        let handle = self.handle
        let signerHandle = signer.handle
        let methodByte = method.ffiValue
        let ownerBytes: [UInt8] = ownerIdentityId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        let expectedBytes: [UInt8] = expectedDocumentId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        return try await Task.detached(priority: .userInitiated) { [self] () -> DpnsNameGivenUp in
            var out = DpnsNameGivenUpFFI()
            // Keep this wallet (and so its handle) and the signer alive through the call
            let result = withExtendedLifetime((self, signer)) {
                ownerBytes.withUnsafeBufferPointer { ownerBp -> PlatformWalletFFIResult in
                    expectedBytes.withUnsafeBufferPointer { expectedBp in
                        name.withCString { namePtr in
                            platform_wallet_dpns_give_up_name(
                                handle,
                                ownerBp.baseAddress!,
                                namePtr,
                                expectedBp.baseAddress!,
                                methodByte,
                                signerHandle,
                                &out
                            )
                        }
                    }
                }
            }
            try result.check()
            guard let givenUp = DpnsNameGivenUp(ffi: out) else {
                throw PlatformWalletError.walletOperation(
                    "dpns_give_up_name reported success but returned an unreadable result",
                    payment: .maybeSent
                )
            }
            return givenUp
        }.value
    }

    /// Refuse inputs that can not cross the give-up ABI unchanged: a name with an
    /// embedded NUL (the C string would end there, and Rust would act on the
    /// prefix) and an identifier that is not 32 bytes. These are checks of the
    /// representation, not of DPNS naming rules, which Rust applies.
    static func requireGiveUpInputs(
        name: String,
        identifiers: [(String, Data)]
    ) throws {
        try KeyDerivation.requireNoNUL(name, "name")
        for (label, identifier) in identifiers where identifier.count != 32 {
            throw PlatformWalletError.invalidParameter(
                "\(label) must be 32 bytes, got \(identifier.count)"
            )
        }
    }

    /// Transfer a DPNS name to `recipientId` without payment (a gift or
    /// off-market handover). Consensus strips any price on transfer, so
    /// this also delists.
    ///
    /// Use ``delistDpnsName(ownerIdentityId:name:signer:)`` for a
    /// transfer to self — this call rejects `recipientId ==
    /// ownerIdentityId` with `.invalidParameter`.
    ///
    /// Same signer lifetime contract as ``setDpnsNamePrice(ownerIdentityId:name:priceCredits:signer:)``.
    @discardableResult
    public func transferDpnsName(
        ownerIdentityId: Identifier,
        name: String,
        recipientId: Identifier,
        signer: KeychainSigner
    ) async throws -> DpnsMarketplaceName {
        let handle = self.handle
        let signerHandle = signer.handle
        let ownerBytes: [UInt8] = ownerIdentityId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        let recipientBytes: [UInt8] = recipientId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        return try await Task.detached(priority: .userInitiated) { () -> DpnsMarketplaceName in
            var outPtr: UnsafeMutablePointer<DpnsMarketplaceNameFFI>? = nil
            let result = withExtendedLifetime(signer) {
                ownerBytes.withUnsafeBufferPointer { ownerBp -> PlatformWalletFFIResult in
                    recipientBytes.withUnsafeBufferPointer { recipBp -> PlatformWalletFFIResult in
                        name.withCString { namePtr in
                            platform_wallet_dpns_transfer_name(
                                handle,
                                ownerBp.baseAddress!,
                                namePtr,
                                recipBp.baseAddress!,
                                signerHandle,
                                &outPtr
                            )
                        }
                    }
                }
            }
            try result.check()
            return try Self.takeConfirmedState(outPtr, operation: "dpns_transfer_name")
        }.value
    }

    /// Purchase a DPNS name at exactly `expectedPriceCredits` — the price
    /// the user confirmed.
    ///
    /// The pre-flight is fully typed: `.notForSale` when the name isn't
    /// listed, `.priceChanged` when the listing no longer matches (both
    /// prices attached), `.insufficientIdentityCredits` when the buyer's
    /// balance can't cover the price plus the fee reserve, and
    /// `.contestedNameNotTradable` for a name inside an active contest.
    ///
    /// The broadcast carries `expectedPriceCredits`, never a re-read
    /// price, so a listing change between pre-flight and broadcast is
    /// rejected by consensus and surfaces as the same `.priceChanged` —
    /// a purchase never executes at a price the user didn't confirm.
    ///
    /// Same signer lifetime contract as ``setDpnsNamePrice(ownerIdentityId:name:priceCredits:signer:)``.
    @discardableResult
    public func purchaseDpnsName(
        purchaserIdentityId: Identifier,
        name: String,
        expectedPriceCredits: UInt64,
        signer: KeychainSigner
    ) async throws -> DpnsMarketplaceName {
        let handle = self.handle
        let signerHandle = signer.handle
        let purchaserBytes: [UInt8] = purchaserIdentityId.withFFIBytes { ptr in
            Array(UnsafeBufferPointer(start: ptr, count: 32))
        }
        return try await Task.detached(priority: .userInitiated) { () -> DpnsMarketplaceName in
            var outPtr: UnsafeMutablePointer<DpnsMarketplaceNameFFI>? = nil
            let result = withExtendedLifetime(signer) {
                purchaserBytes.withUnsafeBufferPointer { buyerBp -> PlatformWalletFFIResult in
                    name.withCString { namePtr in
                        platform_wallet_dpns_purchase_name(
                            handle,
                            buyerBp.baseAddress!,
                            namePtr,
                            expectedPriceCredits,
                            signerHandle,
                            &outPtr
                        )
                    }
                }
            }
            try result.check()
            return try Self.takeConfirmedState(outPtr, operation: "dpns_purchase_name")
        }.value
    }

    /// Run one marketplace sync pass on THIS wallet and return its delta.
    ///
    /// Refreshes owned-name rows (price / sale state), adds newly
    /// observed names, detects names that left an identity (sold or
    /// transferred away), and refreshes the balances of identities that
    /// sold a name. This is the pull-to-refresh entry point; the
    /// recurring cross-wallet sweep is
    /// `PlatformWalletManager.startDpnsSync()`.
    @discardableResult
    public func syncDpnsMarketplace() async throws -> DpnsMarketplaceSyncSummary {
        let handle = self.handle
        return try await Task.detached(priority: .userInitiated) {
            () -> DpnsMarketplaceSyncSummary in
            var ffi = DpnsMarketplaceSyncSummaryFFI()
            try platform_wallet_dpns_marketplace_sync_detailed(handle, &ffi).check()
            defer { dpns_marketplace_sync_summary_free(&ffi) }

            let addedNames: [DpnsNameAdded]
            if let rows = ffi.names_added, ffi.names_added_count > 0 {
                addedNames = (0..<Int(ffi.names_added_count)).map {
                    DpnsNameAdded(ffi: rows[$0])
                }
            } else {
                addedNames = []
            }

            let departedNames: [DpnsNameDeparture]
            if let rows = ffi.names_departed, ffi.names_departed_count > 0 {
                departedNames = (0..<Int(ffi.names_departed_count)).map {
                    DpnsNameDeparture(ffi: rows[$0])
                }
            } else {
                departedNames = []
            }

            let priceChanges: [DpnsPriceChange]
            if let rows = ffi.prices_changed, ffi.prices_changed_count > 0 {
                priceChanges = (0..<Int(ffi.prices_changed_count)).map {
                    DpnsPriceChange(ffi: rows[$0])
                }
            } else {
                priceChanges = []
            }

            return DpnsMarketplaceSyncSummary(
                tracked: ffi.names_tracked,
                added: UInt32(clamping: ffi.names_added_count),
                departed: UInt32(clamping: ffi.names_departed_count),
                pricesChanged: UInt32(clamping: ffi.prices_changed_count),
                addedNames: addedNames,
                departedNames: departedNames,
                priceChanges: priceChanges,
                syncUnixMs: ffi.sync_unix_ms
            )
        }.value
    }

    // MARK: - Shared marshalling helpers

    /// Validate values passed to native APIs that read a fixed 32-byte
    /// identifier. Checking before taking the pointer prevents Rust from
    /// reading beyond a shorter `Data` allocation.
    static func validatedOptionalIdentifier(
        _ data: Data?,
        parameter: String
    ) throws -> [UInt8]? {
        guard let data else { return nil }
        guard data.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "\(parameter) must contain exactly 32 bytes (got \(data.count))"
            )
        }
        return Array(data)
    }

    /// Run `body` with a pointer to `bytes`, or a NULL pointer when it is
    /// nil — the "optional 32-byte argument" shape the marketplace search
    /// cursor and the my-names identity filter both use.
    fileprivate static func withOptionalBytes<R>(
        _ bytes: [UInt8]?,
        _ body: (UnsafePointer<UInt8>?) -> R
    ) -> R {
        guard let bytes else { return body(nil) }
        return bytes.withUnsafeBufferPointer { body($0.baseAddress) }
    }

    /// Take ownership of a confirmed-state out-pointer, copy it into a
    /// Swift value, and release the Rust allocation.
    ///
    /// A successful trade always writes the confirmed state; a null
    /// pointer here is an FFI contract violation, so this throws rather
    /// than returning a fabricated "empty" name for the UI to display as
    /// if the trade had produced it.
    fileprivate static func takeConfirmedState(
        _ ptr: UnsafeMutablePointer<DpnsMarketplaceNameFFI>?,
        operation: String
    ) throws -> DpnsMarketplaceName {
        guard let ptr else {
            throw PlatformWalletError.walletOperation(
                "\(operation) reported success but returned no confirmed name state",
                payment: .maybeSent
            )
        }
        defer { dpns_marketplace_name_free(ptr) }
        return DpnsMarketplaceName(ffi: ptr.pointee)
    }
}
