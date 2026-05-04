import Foundation
import DashSDKFFI

// MARK: - Public types

/// Status of a group-action proposal on Platform. Mirrors
/// `dpp::group::group_action_status::GroupActionStatus`. Platform
/// indexes proposals under one status at a time — `.pending` lists
/// proposals still collecting signatures; `.closed` lists proposals
/// that have already executed (or been finalized otherwise).
public enum TokenGroupActionStatus: UInt8, CaseIterable, Equatable {
    case pending = 0
    case closed = 1

    public var displayName: String {
        switch self {
        case .pending: return "Pending"
        case .closed: return "Closed"
        }
    }
}

/// One pending or closed group-action proposal, decoded from the
/// JSON document emitted by
/// `platform_wallet_token_pending_group_actions`.
public struct TokenGroupAction: Decodable, Equatable, Sendable {
    public let actionId: Data
    public let type: TokenGroupActionType
    public let proposer: Data
    public let tokenContractPosition: UInt16
    public let closed: Bool
    public let params: TokenGroupActionParams

    enum CodingKeys: String, CodingKey {
        case actionId
        case type
        case proposer
        case tokenContractPosition
        case closed
        case params
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let actionIdString = try container.decode(String.self, forKey: .actionId)
        guard let action = Data.identifier(fromBase58: actionIdString) else {
            throw DecodingError.dataCorruptedError(
                forKey: .actionId,
                in: container,
                debugDescription: "actionId is not a valid 32-byte base58 identifier"
            )
        }
        self.actionId = action

        let typeRaw = try container.decode(String.self, forKey: .type)
        self.type = TokenGroupActionType(rawValue: typeRaw) ?? .other(name: typeRaw)

        let proposerString = try container.decode(String.self, forKey: .proposer)
        guard let proposer = Data.identifier(fromBase58: proposerString) else {
            throw DecodingError.dataCorruptedError(
                forKey: .proposer,
                in: container,
                debugDescription: "proposer is not a valid 32-byte base58 identifier"
            )
        }
        self.proposer = proposer

        self.tokenContractPosition = try container.decode(UInt16.self, forKey: .tokenContractPosition)
        self.closed = try container.decode(Bool.self, forKey: .closed)
        self.params = try TokenGroupActionParams(
            from: container.superDecoder(forKey: .params),
            type: self.type
        )
    }
}

/// Discriminator on a `TokenGroupAction`. The raw string matches the
/// `"type"` field emitted by Rust — keep these in sync with
/// `render_params` in
/// `rs-platform-wallet-ffi/src/tokens/group_queries.rs`.
public enum TokenGroupActionType: Equatable, Sendable {
    case mint
    case burn
    case freeze
    case unfreeze
    case destroyFrozenFunds
    case pause
    case resume
    case setPrice
    case directPurchase
    /// Variant the co-sign UI doesn't replay yet; `name` is the
    /// `TokenEvent::associated_document_type_name()` from Rust so the
    /// list can still render a readable label.
    case other(name: String)

    public init?(rawValue: String) {
        switch rawValue {
        case "mint": self = .mint
        case "burn": self = .burn
        case "freeze": self = .freeze
        case "unfreeze": self = .unfreeze
        case "destroyFrozenFunds": self = .destroyFrozenFunds
        case "pause": self = .pause
        case "resume": self = .resume
        case "setPrice": self = .setPrice
        case "directPurchase": self = .directPurchase
        default:
            return nil
        }
    }

    public var displayLabel: String {
        switch self {
        case .mint: return "Mint"
        case .burn: return "Burn"
        case .freeze: return "Freeze"
        case .unfreeze: return "Unfreeze"
        case .destroyFrozenFunds: return "Destroy Frozen Funds"
        case .pause: return "Pause"
        case .resume: return "Resume"
        case .setPrice: return "Set Price"
        case .directPurchase: return "Direct Purchase"
        case .other(let name): return name
        }
    }
}

/// Per-variant payload on a `TokenGroupAction`. Numeric token amounts
/// and credits arrive as JSON strings to dodge JS's 53-bit integer
/// precision cliff; we re-parse them here into `UInt64`.
public enum TokenGroupActionParams: Equatable, Sendable {
    case mint(amount: UInt64, recipient: Data, publicNote: String?)
    case burn(amount: UInt64, burnFrom: Data, publicNote: String?)
    case freeze(target: Data, publicNote: String?)
    case unfreeze(target: Data, publicNote: String?)
    case destroyFrozenFunds(target: Data, amount: UInt64, publicNote: String?)
    case pause(publicNote: String?)
    case resume(publicNote: String?)
    case setPrice(pricePerToken: UInt64?, publicNote: String?)
    case directPurchase(amount: UInt64, totalCost: UInt64)
    case other

    init(from decoder: Decoder, type: TokenGroupActionType) throws {
        let c = try decoder.container(keyedBy: ParamsKeys.self)
        switch type {
        case .mint:
            self = .mint(
                amount: try TokenGroupActionParams.decodeAmount(c, key: .amount),
                recipient: try TokenGroupActionParams.decodeIdentifier(c, key: .recipient),
                publicNote: try c.decodeIfPresent(String.self, forKey: .publicNote)
            )
        case .burn:
            self = .burn(
                amount: try TokenGroupActionParams.decodeAmount(c, key: .amount),
                burnFrom: try TokenGroupActionParams.decodeIdentifier(c, key: .burnFrom),
                publicNote: try c.decodeIfPresent(String.self, forKey: .publicNote)
            )
        case .freeze:
            self = .freeze(
                target: try TokenGroupActionParams.decodeIdentifier(c, key: .target),
                publicNote: try c.decodeIfPresent(String.self, forKey: .publicNote)
            )
        case .unfreeze:
            self = .unfreeze(
                target: try TokenGroupActionParams.decodeIdentifier(c, key: .target),
                publicNote: try c.decodeIfPresent(String.self, forKey: .publicNote)
            )
        case .destroyFrozenFunds:
            self = .destroyFrozenFunds(
                target: try TokenGroupActionParams.decodeIdentifier(c, key: .target),
                amount: try TokenGroupActionParams.decodeAmount(c, key: .amount),
                publicNote: try c.decodeIfPresent(String.self, forKey: .publicNote)
            )
        case .pause:
            self = .pause(publicNote: try c.decodeIfPresent(String.self, forKey: .publicNote))
        case .resume:
            self = .resume(publicNote: try c.decodeIfPresent(String.self, forKey: .publicNote))
        case .setPrice:
            // Treat a present-but-non-numeric `pricePerToken` as a
            // hard decode error rather than silently mapping it to
            // `nil` (which would render as "disable direct purchase").
            let priceString = try c.decodeIfPresent(String.self, forKey: .pricePerToken)
            let parsed: UInt64?
            if let priceString {
                guard let value = UInt64(priceString) else {
                    throw DecodingError.dataCorruptedError(
                        forKey: .pricePerToken,
                        in: c,
                        debugDescription: "expected stringly-typed UInt64 price, got \(priceString)"
                    )
                }
                parsed = value
            } else {
                parsed = nil
            }
            self = .setPrice(
                pricePerToken: parsed,
                publicNote: try c.decodeIfPresent(String.self, forKey: .publicNote)
            )
        case .directPurchase:
            self = .directPurchase(
                amount: try TokenGroupActionParams.decodeAmount(c, key: .amount),
                totalCost: try TokenGroupActionParams.decodeAmount(c, key: .totalCost)
            )
        case .other:
            self = .other
        }
    }

    private static func decodeAmount(
        _ c: KeyedDecodingContainer<ParamsKeys>,
        key: ParamsKeys
    ) throws -> UInt64 {
        let raw = try c.decode(String.self, forKey: key)
        guard let v = UInt64(raw) else {
            throw DecodingError.dataCorruptedError(
                forKey: key,
                in: c,
                debugDescription: "expected stringly-typed UInt64 amount, got \(raw)"
            )
        }
        return v
    }

    private static func decodeIdentifier(
        _ c: KeyedDecodingContainer<ParamsKeys>,
        key: ParamsKeys
    ) throws -> Data {
        let raw = try c.decode(String.self, forKey: key)
        guard let id = Data.identifier(fromBase58: raw) else {
            throw DecodingError.dataCorruptedError(
                forKey: key,
                in: c,
                debugDescription: "expected base58 identifier, got \(raw)"
            )
        }
        return id
    }

    private enum ParamsKeys: String, CodingKey {
        case amount
        case recipient
        case burnFrom
        case target
        case publicNote
        case pricePerToken
        case totalCost
    }
}

/// One signer's entry on a group-action proposal.
public struct TokenGroupActionSigner: Decodable, Equatable, Sendable {
    public let identityId: Data
    public let power: UInt32

    enum CodingKeys: String, CodingKey {
        case identityId
        case power
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        let raw = try c.decode(String.self, forKey: .identityId)
        guard let id = Data.identifier(fromBase58: raw) else {
            throw DecodingError.dataCorruptedError(
                forKey: .identityId,
                in: c,
                debugDescription: "expected base58 identifier, got \(raw)"
            )
        }
        self.identityId = id
        self.power = try c.decode(UInt32.self, forKey: .power)
    }
}

// MARK: - ManagedPlatformWallet group-action queries

extension ManagedPlatformWallet {
    /// List group-action proposals on `(contractId, groupContractPosition)`
    /// filtered by `status`. Returns an empty array when no proposals
    /// exist (Platform returns an empty result rather than an error).
    ///
    /// `startAtActionId` lets the caller paginate through long lists;
    /// `limit == 0` defers to the rs-sdk default limit.
    public func tokenPendingGroupActions(
        contractId: Identifier,
        groupContractPosition: UInt16,
        status: TokenGroupActionStatus = .pending,
        startAtActionId: Data? = nil,
        limit: UInt16 = 0
    ) async throws -> [TokenGroupAction] {
        let handle = self.handle
        let contractBytes = contractId.toFFIByteArray()
        // FFI expects a 32-byte identifier when present; surface
        // length drift loudly instead of forwarding garbage.
        if let startAtActionId, startAtActionId.count != 32 {
            throw PlatformWalletError.serialization(
                "startAtActionId must be 32 bytes, got \(startAtActionId.count)"
            )
        }
        let startBytes: [UInt8]? = startAtActionId.map { Array($0) }
        let statusRaw = status.rawValue

        return try await Task.detached(priority: .userInitiated) {
            var jsonOut: UnsafeMutablePointer<CChar>? = nil
            try contractBytes.withUnsafeBufferPointer { contractBp in
                try ManagedPlatformWallet.tokenWithOptionalActionIdBytes(startBytes) { startPtr in
                    try platform_wallet_token_pending_group_actions(
                        handle,
                        contractBp.baseAddress!,
                        groupContractPosition,
                        statusRaw,
                        startPtr,
                        limit,
                        &jsonOut
                    ).check()
                }
            }
            return try ManagedPlatformWallet.consumeJSONArray(jsonOut)
        }.value
    }

    /// List signers currently signed onto `actionId` on
    /// `(contractId, groupContractPosition)` for the given `status`.
    public func tokenGroupActionSigners(
        contractId: Identifier,
        groupContractPosition: UInt16,
        status: TokenGroupActionStatus = .pending,
        actionId: Data
    ) async throws -> [TokenGroupActionSigner] {
        let handle = self.handle
        let contractBytes = contractId.toFFIByteArray()
        let actionBytes = actionId.toFFIByteArray()
        let statusRaw = status.rawValue

        return try await Task.detached(priority: .userInitiated) {
            var jsonOut: UnsafeMutablePointer<CChar>? = nil
            try contractBytes.withUnsafeBufferPointer { contractBp in
                try actionBytes.withUnsafeBufferPointer { actionBp in
                    try platform_wallet_token_group_action_signers(
                        handle,
                        contractBp.baseAddress!,
                        groupContractPosition,
                        statusRaw,
                        actionBp.baseAddress!,
                        &jsonOut
                    ).check()
                }
            }
            return try ManagedPlatformWallet.consumeJSONArray(jsonOut)
        }.value
    }

    // MARK: Helpers

    /// Run `body` with a `*const u8` to the 32-byte action id, or NULL
    /// when no action id is present (initial page of pending actions).
    fileprivate static func tokenWithOptionalActionIdBytes<R>(
        _ value: [UInt8]?,
        _ body: (UnsafePointer<UInt8>?) throws -> R
    ) rethrows -> R {
        guard let value = value else {
            return try body(nil)
        }
        return try value.withUnsafeBufferPointer { bp in
            try body(bp.baseAddress)
        }
    }

    /// Consume a `*mut c_char` JSON document allocated by the FFI,
    /// decode it into `[T]`, and free the C string. Errors out if the
    /// FFI returned NULL or the JSON didn't decode cleanly.
    fileprivate static func consumeJSONArray<T: Decodable>(
        _ ptr: UnsafeMutablePointer<CChar>?
    ) throws -> [T] {
        guard let ptr else {
            throw PlatformWalletError.serialization(
                "platform-wallet returned a NULL JSON pointer"
            )
        }
        defer { platform_wallet_free_string(ptr) }
        let str = String(cString: ptr)
        guard let data = str.data(using: .utf8) else {
            throw PlatformWalletError.serialization(
                "JSON response is not valid UTF-8"
            )
        }
        do {
            return try JSONDecoder().decode([T].self, from: data)
        } catch {
            throw PlatformWalletError.deserialization(
                "Failed to decode group-action JSON: \(error.localizedDescription)"
            )
        }
    }
}

// MARK: - Identifier helper

private extension Data {
    /// Materialise the 32-byte payload as a `[UInt8]` for use with
    /// `withUnsafeBufferPointer`. Identifiers on Platform are always
    /// exactly 32 bytes; the precondition surfaces the drift loudly
    /// rather than letting Rust read garbage on dereference.
    func toFFIByteArray() -> [UInt8] {
        precondition(count == 32, "identifier must be 32 bytes, got \(count)")
        return Array(self)
    }
}
