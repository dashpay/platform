import Foundation

// MARK: - Token Localization

/// Localized token display information
public struct TokenLocalization: Codable, Equatable, Sendable {
    public let singularForm: String
    public let pluralForm: String
    public let description: String?

    public init(singularForm: String, pluralForm: String, description: String? = nil) {
        self.singularForm = singularForm
        self.pluralForm = pluralForm
        self.description = description
    }
}

// MARK: - Change Control Rules

/// Rules governing who can make changes to token configuration
public struct ChangeControlRules: Codable, Equatable, Sendable {
    public var authorizedToMakeChange: String
    public var adminActionTakers: String
    public var changingAuthorizedActionTakersToNoOneAllowed: Bool
    public var changingAdminActionTakersToNoOneAllowed: Bool
    public var selfChangingAdminActionTakersAllowed: Bool

    public init(
        authorizedToMakeChange: String = AuthorizedActionTakers.noOne.rawValue,
        adminActionTakers: String = AuthorizedActionTakers.noOne.rawValue,
        changingAuthorizedActionTakersToNoOneAllowed: Bool = false,
        changingAdminActionTakersToNoOneAllowed: Bool = false,
        selfChangingAdminActionTakersAllowed: Bool = false
    ) {
        self.authorizedToMakeChange = authorizedToMakeChange
        self.adminActionTakers = adminActionTakers
        self.changingAuthorizedActionTakersToNoOneAllowed = changingAuthorizedActionTakersToNoOneAllowed
        self.changingAdminActionTakersToNoOneAllowed = changingAdminActionTakersToNoOneAllowed
        self.selfChangingAdminActionTakersAllowed = selfChangingAdminActionTakersAllowed
    }

    /// Most restrictive configuration - no one can make changes
    public static func mostRestrictive() -> ChangeControlRules {
        return ChangeControlRules()
    }

    /// Contract owner has full control
    public static func contractOwnerControlled() -> ChangeControlRules {
        return ChangeControlRules(
            authorizedToMakeChange: AuthorizedActionTakers.contractOwner.rawValue,
            adminActionTakers: AuthorizedActionTakers.noOne.rawValue,
            selfChangingAdminActionTakersAllowed: true
        )
    }
}

// MARK: - Perpetual Distribution

/// Configuration for perpetual token distribution
public struct TokenPerpetualDistribution: Codable, Equatable, Sendable {
    public var distributionType: String
    public var distributionRecipient: String
    public var enabled: Bool
    public var lastDistributionTime: Date?
    public var nextDistributionTime: Date?

    public init(distributionRecipient: String = "AllEqualShare", enabled: Bool = true) {
        self.distributionType = "{}"
        self.distributionRecipient = distributionRecipient
        self.enabled = enabled
    }
}

// MARK: - Pre-Programmed Distribution

/// Configuration for pre-programmed token distribution schedule
public struct TokenPreProgrammedDistribution: Codable, Equatable, Sendable {
    public var distributionSchedule: [DistributionEvent]
    public var currentEventIndex: Int
    public var totalDistributed: String
    public var remainingToDistribute: String
    public var isActive: Bool
    public var isPaused: Bool
    public var isCompleted: Bool

    public init() {
        self.distributionSchedule = []
        self.currentEventIndex = 0
        self.totalDistributed = "0"
        self.remainingToDistribute = "0"
        self.isActive = true
        self.isPaused = false
        self.isCompleted = false
    }
}

// MARK: - Distribution Event

/// A single distribution event in a pre-programmed schedule
public struct DistributionEvent: Codable, Equatable, Sendable {
    public var id: UUID
    public var triggerType: String
    public var triggerTime: Date?
    public var triggerBlock: Int64?
    public var triggerCondition: String?
    public var amount: String
    public var recipient: String
    public var description: String?

    public init(triggerTime: Date, amount: String, recipient: String = "AllHolders", description: String? = nil) {
        self.id = UUID()
        self.triggerType = "Time"
        self.triggerTime = triggerTime
        self.amount = amount
        self.recipient = recipient
        self.description = description
    }
}

// MARK: - Once-Per-Identity Distribution

/// A fixed amount every identity may claim exactly once (protocol
/// version 14).
///
/// rs-dpp serialises it inside a token's `distributionRules` as
/// `"oncePerIdentityDistribution": {"$formatVersion": "0", "amount": 5000}`.
///
/// `amount` is a protocol `u64`, so it is carried as an exact decimal string
/// here, the same convention the other token amounts use. Values above
/// `Int64.max` arrive as JSON strings and must survive verbatim, which a
/// fixed-width or floating-point carrier could not guarantee.
///
/// Unlike `TokenPerpetualDistribution` and `TokenPreProgrammedDistribution`
/// this value has no column on `PersistentToken`: it is derived from the
/// owning contract's stored JSON through
/// `PersistentToken.oncePerIdentityDistribution`.
public struct TokenOncePerIdentityDistribution: Codable, Equatable, Sendable {
    /// The amount minted to an identity on its single claim, as an exact
    /// decimal string.
    public var amount: String

    public init(amount: String) {
        self.amount = amount
    }
}

// MARK: - Distribution Change Rules

/// Rules governing changes to distribution configuration
public struct TokenDistributionChangeRules: Codable, Equatable, Sendable {
    public var perpetualDistributionRules: ChangeControlRules?
    public var newTokensDestinationIdentityRules: ChangeControlRules?
    public var mintingAllowChoosingDestinationRules: ChangeControlRules?
    public var changeDirectPurchasePricingRules: ChangeControlRules?

    public init(
        perpetualDistributionRules: ChangeControlRules? = nil,
        newTokensDestinationIdentityRules: ChangeControlRules? = nil,
        mintingAllowChoosingDestinationRules: ChangeControlRules? = nil,
        changeDirectPurchasePricingRules: ChangeControlRules? = nil
    ) {
        self.perpetualDistributionRules = perpetualDistributionRules
        self.newTokensDestinationIdentityRules = newTokensDestinationIdentityRules
        self.mintingAllowChoosingDestinationRules = mintingAllowChoosingDestinationRules
        self.changeDirectPurchasePricingRules = changeDirectPurchasePricingRules
    }
}

// MARK: - Authorized Action Takers

/// Enum defining who can take actions on a token
public enum AuthorizedActionTakers: String, CaseIterable, Codable, Sendable {
    case noOne = "NoOne"
    case contractOwner = "ContractOwner"
    case mainGroup = "MainGroup"

    /// The `$type` discriminators rs-dpp emits for an
    /// `AuthorizedActionTakers` value on the wire. Since 4.0.0-beta.4
    /// the value arrives as a flat tagged map, for example
    /// `{"$type": "contractOwner"}`,
    /// `{"$type": "identity", "identity": "<base58>"}` or
    /// `{"$type": "group", "position": 3}`. These constants are the
    /// values of that tag, which are distinct from the canonical
    /// persisted strings above.
    public enum WireType {
        public static let noOne = "noOne"
        public static let contractOwner = "contractOwner"
        public static let identity = "identity"
        public static let mainGroup = "mainGroup"
        public static let group = "group"
    }

    public static func identity(_ id: Data) -> String {
        return "Identity:\(id.toBase58String())"
    }

    /// Overload for an identity id that is already base58-encoded, the
    /// form it takes in contract JSON. The string is used verbatim.
    public static func identity(_ base58Id: String) -> String {
        return "Identity:\(base58Id)"
    }

    public static func group(_ position: Int) -> String {
        return "Group:\(position)"
    }
}

// MARK: - Token Distribution Recipient

/// Canonical strings stored in `TokenPerpetualDistribution.distributionRecipient`.
///
/// rs-dpp serialises its `TokenDistributionRecipient` as a flat map tagged by
/// `$type` (since 4.0.0-beta.4), for example `{"$type": "contractOwner"}` or
/// `{"$type": "identity", "identity": "<base58>"}`. `DataContractParser` folds
/// that map into one of the strings below. This type is a plain helper, it is
/// not part of the SwiftData model graph and carries no stored properties.
///
/// The string shapes mirror `AuthorizedActionTakers`, so both columns read the
/// same way in the UI: `"ContractOwner"`, `"Identity:<base58>"`.
public enum TokenDistributionRecipient: String, CaseIterable, Codable, Sendable {
    case contractOwner = "ContractOwner"
    case evonodesByParticipation = "EvonodesByParticipation"

    /// The `$type` discriminators emitted by rs-dpp on the wire. Kept here so
    /// the parser does not scatter string literals.
    public enum WireType {
        public static let contractOwner = "contractOwner"
        public static let identity = "identity"
        public static let evonodesByParticipation = "evonodesByParticipation"
    }

    /// The `identity` variant. The base58 identifier arrives already encoded in
    /// the contract JSON, so it is used verbatim rather than re-encoded.
    public static func identity(_ base58Id: String) -> String {
        return "Identity:\(base58Id)"
    }
}

// MARK: - Token Trade Mode

/// Trading modes for tokens
public enum TokenTradeMode: String, CaseIterable, Codable, Sendable {
    case notTradeable = "NotTradeable"

    public var displayName: String {
        switch self {
        case .notTradeable:
            return "Not Tradeable"
        }
    }
}

// MARK: - Control Rule Types

/// Types of control rules that can be configured on tokens
public enum ControlRuleType: Sendable {
    case conventions
    case maxSupply
    case manualMinting
    case manualBurning
    case freeze
    case unfreeze
    case destroyFrozenFunds
    case emergencyAction
}

/// Types of change control rules for token configuration
public enum ChangeControlRuleType: Sendable {
    case conventions
    case maxSupply
    case manualMinting
    case manualBurning
    case freeze
    case unfreeze
    case destroyFrozenFunds
    case emergencyAction
    case tradeMode
}

// MARK: - Token Event Types

/// Types of token history events
public enum TokenEventType: String, CaseIterable, Sendable {
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

    /// Whether this event type always requires a history entry
    public var requiresHistory: Bool {
        switch self {
        case .configUpdate, .destroyFrozenFunds, .emergencyAction, .preProgrammedRelease:
            return true
        default:
            return false
        }
    }

    /// SF Symbol icon for this event type
    public var icon: String {
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

// MARK: - Identity Type

/// Types of identities on the Dash Platform
public enum IdentityType: String, CaseIterable, Sendable {
    case user = "User"
    case masternode = "Masternode"
    case evonode = "Evonode"
}
