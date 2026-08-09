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

    public init(tracked: UInt32, added: UInt32, departed: UInt32, pricesChanged: UInt32) {
        self.tracked = tracked
        self.added = added
        self.departed = departed
        self.pricesChanged = pricesChanged
    }
}

// MARK: - FFI decoding

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
    /// An unrecognised `status` byte decodes to `.owned` with a
    /// counterparty of `nil` only when `has_counterparty` is false;
    /// otherwise it is treated as `.transferred`, the wallet layer's own
    /// documented fallback for an unattributable departure — the row is
    /// never reported as a sale the wallet cannot evidence.
    init(ffi: DpnsNameStateRowFFI) {
        var documentTuple = ffi.document_id
        var walletIdentityTuple = ffi.wallet_identity_id
        var counterpartyTuple = ffi.counterparty_id
        let counterparty: Data? = ffi.has_counterparty
            ? Swift.withUnsafeBytes(of: &counterpartyTuple) { Data($0) }
            : nil
        let status: DpnsNameSaleStatus
        switch (ffi.status, counterparty) {
        case (1, .some(let to)):
            status = .sold(to: to)
        case (_, .some(let to)):
            status = .transferred(to: to)
        default:
            status = .owned
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
        let cursorBytes: [UInt8]? = startAfter.map { Array($0) }
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
        let idBytes: [UInt8]? = identityId.map { Array($0) }
        return try await Task.detached(priority: .userInitiated) { () -> [DpnsNameStateRow] in
            var outPtr: UnsafeMutablePointer<DpnsNameStateRowFFI>? = nil
            var outCount: UInt = 0
            let result = Self.withOptionalBytes(idBytes) { idPtr in
                platform_wallet_dpns_marketplace_my_names(handle, idPtr, &outPtr, &outCount)
            }
            try result.check()
            guard let ptr = outPtr, outCount > 0 else { return [] }
            defer { dpns_name_state_rows_free(ptr, outCount) }
            return (0..<Int(outCount)).map { DpnsNameStateRow(ffi: ptr[$0]) }
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
            var tracked: UInt32 = 0
            var added: UInt32 = 0
            var departed: UInt32 = 0
            var pricesChanged: UInt32 = 0
            try platform_wallet_dpns_marketplace_sync(
                handle,
                &tracked,
                &added,
                &departed,
                &pricesChanged
            ).check()
            return DpnsMarketplaceSyncSummary(
                tracked: tracked,
                added: added,
                departed: departed,
                pricesChanged: pricesChanged
            )
        }.value
    }

    // MARK: - Shared marshalling helpers

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
                "\(operation) reported success but returned no confirmed name state"
            )
        }
        defer { dpns_marketplace_name_free(ptr) }
        return DpnsMarketplaceName(ffi: ptr.pointee)
    }
}
