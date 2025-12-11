import Foundation
import SwiftData

/// SwiftData model for persisting token history events
@Model
public final class PersistentTokenHistoryEvent {
    @Attribute(.unique) public var id: UUID

    // Event details
    public var eventType: String
    public var transactionId: Data?
    public var blockHeight: Int64?
    public var coreBlockHeight: Int64?

    // Participants
    public var fromIdentity: Data?
    public var toIdentity: Data?
    public var performedByIdentity: Data

    // Amounts
    public var amount: String?
    public var balanceBefore: String?
    public var balanceAfter: String?

    // Additional data stored as JSON
    public var additionalDataJSON: Data?

    // Description
    public var eventDescription: String?

    // Timestamps
    public var createdAt: Date
    public var eventTimestamp: Date

    // Relationship to token
    @Relationship(inverse: \PersistentToken.historyEvents)
    public var token: PersistentToken?

    public init(
        eventType: TokenEventType,
        performedByIdentity: Data,
        eventTimestamp: Date = Date()
    ) {
        self.id = UUID()
        self.eventType = eventType.rawValue
        self.performedByIdentity = performedByIdentity
        self.eventTimestamp = eventTimestamp
        self.createdAt = Date()
    }

    // MARK: - Computed Properties
    public var eventTypeEnum: TokenEventType {
        TokenEventType(rawValue: eventType) ?? .unknown
    }

    public var fromIdentityBase58: String? {
        fromIdentity?.toBase58String()
    }

    public var toIdentityBase58: String? {
        toIdentity?.toBase58String()
    }

    public var performedByIdentityBase58: String {
        performedByIdentity.toBase58String()
    }

    public var displayTitle: String {
        switch eventTypeEnum {
        case .mint:
            return "Minted \(formattedAmount)"
        case .burn:
            return "Burned \(formattedAmount)"
        case .transfer:
            return "Transfer \(formattedAmount)"
        case .freeze:
            return "Frozen \(formattedAmount)"
        case .unfreeze:
            return "Unfrozen \(formattedAmount)"
        case .destroyFrozenFunds:
            return "Destroyed Frozen Funds \(formattedAmount)"
        case .configUpdate:
            return "Configuration Updated"
        case .emergencyAction:
            return "Emergency Action"
        case .perpetualDistribution:
            return "Perpetual Distribution \(formattedAmount)"
        case .preProgrammedRelease:
            return "Pre-programmed Release \(formattedAmount)"
        case .directPricing:
            return "Direct Pricing Updated"
        case .directPurchase:
            return "Direct Purchase \(formattedAmount)"
        case .unknown:
            return "Unknown Event"
        }
    }

    private var formattedAmount: String {
        guard let amount = amount else { return "" }
        return amount
    }

    // MARK: - Additional Data Methods
    public func setAdditionalData(_ data: [String: Any]) {
        additionalDataJSON = try? JSONSerialization.data(withJSONObject: data)
    }

    public func getAdditionalData() -> [String: Any]? {
        guard let data = additionalDataJSON else { return nil }
        return try? JSONSerialization.jsonObject(with: data) as? [String: Any]
    }
}
