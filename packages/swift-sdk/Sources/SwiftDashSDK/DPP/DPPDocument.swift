import Foundation

// MARK: - Document Models based on DPP

/// Main Document structure representing a Dash Platform document
public struct DPPDocument: Identifiable, Codable, Equatable, Sendable {
    public let id: Identifier
    public let ownerId: Identifier
    public let properties: [String: PlatformValue]
    public let revision: Revision?
    public let createdAt: TimestampMillis?
    public let updatedAt: TimestampMillis?
    public let transferredAt: TimestampMillis?
    public let createdAtBlockHeight: BlockHeight?
    public let updatedAtBlockHeight: BlockHeight?
    public let transferredAtBlockHeight: BlockHeight?
    public let createdAtCoreBlockHeight: CoreBlockHeight?
    public let updatedAtCoreBlockHeight: CoreBlockHeight?
    public let transferredAtCoreBlockHeight: CoreBlockHeight?

    /// Get the document ID as a base58 string
    public var idString: String {
        id.toBase58String()
    }

    /// Get the owner ID as a base58 string
    public var ownerIdString: String {
        ownerId.toBase58String()
    }

    public init(
        id: Identifier,
        ownerId: Identifier,
        properties: [String: PlatformValue],
        revision: Revision? = nil,
        createdAt: TimestampMillis? = nil,
        updatedAt: TimestampMillis? = nil,
        transferredAt: TimestampMillis? = nil,
        createdAtBlockHeight: BlockHeight? = nil,
        updatedAtBlockHeight: BlockHeight? = nil,
        transferredAtBlockHeight: BlockHeight? = nil,
        createdAtCoreBlockHeight: CoreBlockHeight? = nil,
        updatedAtCoreBlockHeight: CoreBlockHeight? = nil,
        transferredAtCoreBlockHeight: CoreBlockHeight? = nil
    ) {
        self.id = id
        self.ownerId = ownerId
        self.properties = properties
        self.revision = revision
        self.createdAt = createdAt
        self.updatedAt = updatedAt
        self.transferredAt = transferredAt
        self.createdAtBlockHeight = createdAtBlockHeight
        self.updatedAtBlockHeight = updatedAtBlockHeight
        self.transferredAtBlockHeight = transferredAtBlockHeight
        self.createdAtCoreBlockHeight = createdAtCoreBlockHeight
        self.updatedAtCoreBlockHeight = updatedAtCoreBlockHeight
        self.transferredAtCoreBlockHeight = transferredAtCoreBlockHeight
    }

    /// Get created date
    public var createdDate: Date? {
        guard let createdAt = createdAt else { return nil }
        return Date(timeIntervalSince1970: Double(createdAt) / 1000)
    }

    /// Get updated date
    public var updatedDate: Date? {
        guard let updatedAt = updatedAt else { return nil }
        return Date(timeIntervalSince1970: Double(updatedAt) / 1000)
    }

    /// Get transferred date
    public var transferredDate: Date? {
        guard let transferredAt = transferredAt else { return nil }
        return Date(timeIntervalSince1970: Double(transferredAt) / 1000)
    }
}

// MARK: - Token Payment Info

public struct TokenPaymentInfo: Codable, Equatable, Sendable {
    public let tokenId: Identifier
    public let amount: UInt64

    public var tokenIdString: String {
        tokenId.toBase58String()
    }

    public init(tokenId: Identifier, amount: UInt64) {
        self.tokenId = tokenId
        self.amount = amount
    }
}
