import Foundation
import SwiftData

@Model
final class PersistentTokenHistoryEvent {
    @Attribute(.unique) var id: UUID
    
    // Event details
    var eventType: String // TokenEventType enum as string
    var transactionId: Data?
    var blockHeight: Int64?
    var coreBlockHeight: Int64?
    
    // Participants
    var fromIdentity: Data?
    var toIdentity: Data?
    var performedByIdentity: Data
    
    // Amounts
    var amount: String?
    var balanceBefore: String?
    var balanceAfter: String?
    
    // Additional data stored as JSON
    var additionalDataJSON: Data?
    
    // Description
    var eventDescription: String?
    
    // Timestamps
    var createdAt: Date
    var eventTimestamp: Date
    
    // Relationship to token
    @Relationship(inverse: \PersistentToken.historyEvents)
    var token: PersistentToken?
    
    init(
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
    var eventTypeEnum: TokenEventType {
        TokenEventType(rawValue: eventType) ?? .unknown
    }
    
    var fromIdentityBase58: String? {
        fromIdentity?.toBase58String()
    }
    
    var toIdentityBase58: String? {
        toIdentity?.toBase58String()
    }
    
    var performedByIdentityBase58: String {
        performedByIdentity.toBase58String()
    }
    
    var displayTitle: String {
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
    func setAdditionalData(_ data: [String: Any]) {
        additionalDataJSON = try? JSONSerialization.data(withJSONObject: data)
    }
    
    func getAdditionalData() -> [String: Any]? {
        guard let data = additionalDataJSON else { return nil }
        return try? JSONSerialization.jsonObject(with: data) as? [String: Any]
    }
}

// MARK: - TokenEventType enum
enum TokenEventType: String, CaseIterable {
    case mint = "Mint"
    case burn = "Burn"
    case transfer = "Transfer"
    case freeze = "Freeze"
    case unfreeze = "Unfreeze"
    case destroyFrozenFunds = "DestroyFrozenFunds"
    case configUpdate = "ConfigUpdate"
    case emergencyAction = "EmergencyAction"
    case perpetualDistribution = "PerpetualDistribution"
    case preProgrammedRelease = "PreProgrammedRelease"
    case directPricing = "DirectPricing"
    case directPurchase = "DirectPurchase"
    case unknown = "Unknown"
    
    var requiresHistory: Bool {
        // These events ALWAYS require history entries
        switch self {
        case .configUpdate, .destroyFrozenFunds, .emergencyAction, .preProgrammedRelease:
            return true
        default:
            return false
        }
    }
    
    var icon: String {
        switch self {
        case .mint: return "plus.circle.fill"
        case .burn: return "flame.fill"
        case .transfer: return "arrow.right.circle.fill"
        case .freeze: return "snowflake"
        case .unfreeze: return "sun.max.fill"
        case .destroyFrozenFunds: return "trash.fill"
        case .configUpdate: return "gearshape.fill"
        case .emergencyAction: return "exclamationmark.triangle.fill"
        case .perpetualDistribution: return "clock.arrow.circlepath"
        case .preProgrammedRelease: return "calendar.badge.clock"
        case .directPricing: return "tag.fill"
        case .directPurchase: return "cart.fill"
        case .unknown: return "questionmark.circle.fill"
        }
    }
}