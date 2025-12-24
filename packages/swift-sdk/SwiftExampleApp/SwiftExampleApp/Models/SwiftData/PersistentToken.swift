import Foundation
import SwiftData

@Model
final class PersistentToken {
    @Attribute(.unique) var id: Data // Combines contractId + position
    var contractId: Data
    var position: Int
    var name: String
    
    // Basic token supply info
    var baseSupply: String // Store as string to handle large numbers
    var maxSupply: String? // Optional max supply
    var decimals: Int
    
    // Token conventions
    var localizations: [String: TokenLocalization]?
    
    // Status flags
    var isPaused: Bool
    var allowTransferToFrozenBalance: Bool
    
    // History keeping rules
    var keepsTransferHistory: Bool
    var keepsFreezingHistory: Bool
    var keepsMintingHistory: Bool
    var keepsBurningHistory: Bool
    var keepsDirectPricingHistory: Bool
    var keepsDirectPurchaseHistory: Bool
    
    // Control rules
    var conventionsChangeRules: ChangeControlRules?
    var maxSupplyChangeRules: ChangeControlRules?
    var manualMintingRules: ChangeControlRules?
    var manualBurningRules: ChangeControlRules?
    var freezeRules: ChangeControlRules?
    var unfreezeRules: ChangeControlRules?
    var destroyFrozenFundsRules: ChangeControlRules?
    var emergencyActionRules: ChangeControlRules?
    
    // Distribution rules
    var perpetualDistribution: TokenPerpetualDistribution?
    var preProgrammedDistribution: TokenPreProgrammedDistribution?
    var newTokensDestinationIdentity: Data?
    var mintingAllowChoosingDestination: Bool
    var distributionChangeRules: TokenDistributionChangeRules?
    
    // Marketplace rules
    var tradeMode: TokenTradeMode
    var tradeModeChangeRules: ChangeControlRules?
    
    // Main control group
    var mainControlGroupPosition: Int?
    var mainControlGroupCanBeModified: String? // AuthorizedActionTakers enum as string
    
    // Description
    var tokenDescription: String?
    
    // Timestamps
    var createdAt: Date
    var lastUpdatedAt: Date
    
    // Relationships
    var dataContract: PersistentDataContract?
    
    @Relationship(deleteRule: .cascade)
    var balances: [PersistentTokenBalance]?
    
    @Relationship(deleteRule: .cascade)
    var historyEvents: [PersistentTokenHistoryEvent]?
    
    init(contractId: Data, position: Int, name: String, baseSupply: String, decimals: Int = 8) {
        // Create unique ID by combining contract ID and position
        var idData = contractId
        withUnsafeBytes(of: position.bigEndian) { bytes in
            idData.append(contentsOf: bytes)
        }
        self.id = idData
        
        self.contractId = contractId
        self.position = position
        self.name = name
        self.baseSupply = baseSupply
        self.decimals = decimals
        
        // Default values
        self.isPaused = false
        self.allowTransferToFrozenBalance = true
        self.keepsTransferHistory = true
        self.keepsFreezingHistory = true
        self.keepsMintingHistory = true
        self.keepsBurningHistory = true
        self.keepsDirectPricingHistory = true
        self.keepsDirectPurchaseHistory = true
        self.mintingAllowChoosingDestination = true
        self.tradeMode = TokenTradeMode.notTradeable
        
        self.createdAt = Date()
        self.lastUpdatedAt = Date()
    }
}

// MARK: - Computed Properties
extension PersistentToken {
    var displayName: String {
        if let desc = tokenDescription, !desc.isEmpty {
            return desc
        }
        return getSingularForm() ?? name
    }
    
    var formattedBaseSupply: String {
        // Format with decimals
        guard let supplyValue = Double(baseSupply) else { return baseSupply }
        
        // If decimals is 0, just return the raw value
        if decimals == 0 {
            return String(Int(supplyValue))
        }
        
        let divisor = pow(10.0, Double(decimals))
        let actualSupply = supplyValue / divisor
        
        let formatter = NumberFormatter()
        formatter.numberStyle = .decimal
        formatter.maximumFractionDigits = decimals
        formatter.minimumFractionDigits = 0
        formatter.groupingSeparator = ","
        
        return formatter.string(from: NSNumber(value: actualSupply)) ?? baseSupply
    }
    
    var contractIdBase58: String {
        contractId.toBase58String()
    }
    
    // MARK: - Indexed Properties for Querying
    
    /// Returns true if manual minting is allowed (has minting rules)
    var canManuallyMint: Bool {
        manualMintingRules != nil
    }
    
    /// Returns true if manual burning is allowed (has burning rules)
    var canManuallyBurn: Bool {
        manualBurningRules != nil
    }
    
    /// Returns true if tokens can be frozen (has freeze rules)
    var canFreeze: Bool {
        freezeRules != nil
    }
    
    /// Returns true if tokens can be unfrozen (has unfreeze rules)
    var canUnfreeze: Bool {
        unfreezeRules != nil
    }
    
    /// Returns true if frozen funds can be destroyed (has destroy rules)
    var canDestroyFrozenFunds: Bool {
        destroyFrozenFundsRules != nil
    }
    
    /// Returns true if emergency actions are available
    var hasEmergencyActions: Bool {
        emergencyActionRules != nil
    }
    
    /// Returns true if max supply can be changed
    var canChangeMaxSupply: Bool {
        maxSupplyChangeRules != nil
    }
    
    /// Returns true if conventions can be changed
    var canChangeConventions: Bool {
        conventionsChangeRules != nil
    }
    
    /// Returns true if has any distribution mechanism
    var hasDistribution: Bool {
        perpetualDistribution != nil || preProgrammedDistribution != nil
    }
    
    /// Returns true if trade mode can be changed
    var canChangeTradeMode: Bool {
        tradeModeChangeRules != nil
    }
    
    var keepsAnyHistory: Bool {
        keepsTransferHistory ||
        keepsFreezingHistory ||
        keepsMintingHistory ||
        keepsBurningHistory ||
        keepsDirectPricingHistory ||
        keepsDirectPurchaseHistory
    }
    
    var totalSupply: String {
        // Calculate from balances if available
        guard let balances = balances, !balances.isEmpty else { return baseSupply }
        let total = balances.reduce(0) { $0 + $1.balance }
        return String(total)
    }
    
    var totalFrozenBalance: String {
        guard let balances = balances else { return "0" }
        let frozen = balances.filter { $0.frozen }.reduce(0) { $0 + $1.balance }
        return String(frozen)
    }
    
    var activeHolders: Int {
        balances?.filter { $0.balance > 0 }.count ?? 0
    }
    
    var hasMaxSupply: Bool {
        maxSupply != nil
    }
    
    var isTradeable: Bool {
        tradeMode != .notTradeable
    }
    
    var newTokensDestinationIdentityBase58: String? {
        newTokensDestinationIdentity?.toBase58String()
    }
}

// MARK: - Localization Methods
extension PersistentToken {
    func setLocalization(languageCode: String, singularForm: String, pluralForm: String, description: String? = nil) {
        if localizations == nil {
            localizations = [:]
        }
        localizations?[languageCode] = TokenLocalization(
            singularForm: singularForm,
            pluralForm: pluralForm,
            description: description
        )
        lastUpdatedAt = Date()
    }
    
    func getSingularForm(languageCode: String = "en") -> String? {
        return localizations?[languageCode]?.singularForm ?? localizations?["en"]?.singularForm
    }
    
    func getPluralForm(languageCode: String = "en") -> String? {
        return localizations?[languageCode]?.pluralForm ?? localizations?["en"]?.pluralForm
    }
}

// MARK: - Control Rules Methods
extension PersistentToken {
    func getChangeControlRules(for type: ChangeControlRuleType) -> ChangeControlRules? {
        switch type {
        case .conventions: return conventionsChangeRules
        case .maxSupply: return maxSupplyChangeRules
        case .manualMinting: return manualMintingRules
        case .manualBurning: return manualBurningRules
        case .freeze: return freezeRules
        case .unfreeze: return unfreezeRules
        case .destroyFrozenFunds: return destroyFrozenFundsRules
        case .emergencyAction: return emergencyActionRules
        case .tradeMode: return tradeModeChangeRules
        }
    }
    
    func setChangeControlRules(_ rules: ChangeControlRules, for type: ChangeControlRuleType) {
        switch type {
        case .conventions: conventionsChangeRules = rules
        case .maxSupply: maxSupplyChangeRules = rules
        case .manualMinting: manualMintingRules = rules
        case .manualBurning: manualBurningRules = rules
        case .freeze: freezeRules = rules
        case .unfreeze: unfreezeRules = rules
        case .destroyFrozenFunds: destroyFrozenFundsRules = rules
        case .emergencyAction: emergencyActionRules = rules
        case .tradeMode: tradeModeChangeRules = rules
        }
        
        lastUpdatedAt = Date()
    }
}

// MARK: - Supporting Types
struct TokenLocalization: Codable, Equatable {
    let singularForm: String
    let pluralForm: String
    let description: String?
}

struct ChangeControlRules: Codable, Equatable {
    var authorizedToMakeChange: String // AuthorizedActionTakers enum as string
    var adminActionTakers: String // AuthorizedActionTakers enum as string
    var changingAuthorizedActionTakersToNoOneAllowed: Bool
    var changingAdminActionTakersToNoOneAllowed: Bool
    var selfChangingAdminActionTakersAllowed: Bool
    
    init(
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
    
    static func mostRestrictive() -> ChangeControlRules {
        return ChangeControlRules()
    }
    
    static func contractOwnerControlled() -> ChangeControlRules {
        return ChangeControlRules(
            authorizedToMakeChange: AuthorizedActionTakers.contractOwner.rawValue,
            adminActionTakers: AuthorizedActionTakers.noOne.rawValue,
            selfChangingAdminActionTakersAllowed: true
        )
    }
}

struct TokenPerpetualDistribution: Codable, Equatable {
    var distributionType: String // JSON representation of distribution type
    var distributionRecipient: String // TokenDistributionRecipient enum
    var enabled: Bool
    var lastDistributionTime: Date?
    var nextDistributionTime: Date?
    
    init(distributionRecipient: String = "AllEqualShare", enabled: Bool = true) {
        self.distributionType = "{}"
        self.distributionRecipient = distributionRecipient
        self.enabled = enabled
    }
}

struct TokenPreProgrammedDistribution: Codable, Equatable {
    var distributionSchedule: [DistributionEvent]
    var currentEventIndex: Int
    var totalDistributed: String
    var remainingToDistribute: String
    var isActive: Bool
    var isPaused: Bool
    var isCompleted: Bool
    
    init() {
        self.distributionSchedule = []
        self.currentEventIndex = 0
        self.totalDistributed = "0"
        self.remainingToDistribute = "0"
        self.isActive = true
        self.isPaused = false
        self.isCompleted = false
    }
}

struct DistributionEvent: Codable, Equatable {
    var id: UUID
    var triggerType: String // "Time", "Block", "Condition"
    var triggerTime: Date?
    var triggerBlock: Int64?
    var triggerCondition: String?
    var amount: String
    var recipient: String
    var description: String?
    
    init(triggerTime: Date, amount: String, recipient: String = "AllHolders", description: String? = nil) {
        self.id = UUID()
        self.triggerType = "Time"
        self.triggerTime = triggerTime
        self.amount = amount
        self.recipient = recipient
        self.description = description
    }
}

struct TokenDistributionChangeRules: Codable, Equatable {
    var perpetualDistributionRules: ChangeControlRules?
    var newTokensDestinationIdentityRules: ChangeControlRules?
    var mintingAllowChoosingDestinationRules: ChangeControlRules?
    var changeDirectPurchasePricingRules: ChangeControlRules?
}

enum ChangeControlRuleType {
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

enum AuthorizedActionTakers: String, CaseIterable, Codable {
    case noOne = "NoOne"
    case contractOwner = "ContractOwner"
    case mainGroup = "MainGroup"
    
    static func identity(_ id: Data) -> String {
        return "Identity:\(id.toBase58String())"
    }
    
    static func group(_ position: Int) -> String {
        return "Group:\(position)"
    }
}

enum TokenTradeMode: String, CaseIterable, Codable {
    case notTradeable = "NotTradeable"
    // Future trade modes can be added here
    
    var displayName: String {
        switch self {
        case .notTradeable:
            return "Not Tradeable"
        }
    }
}

// MARK: - Query Helpers
extension PersistentToken {
    /// Find all tokens that allow manual minting
    static func mintableTokensPredicate() -> Predicate<PersistentToken> {
        #Predicate<PersistentToken> { token in
            token.manualMintingRules != nil
        }
    }
    
    /// Find all tokens that allow manual burning
    static func burnableTokensPredicate() -> Predicate<PersistentToken> {
        #Predicate<PersistentToken> { token in
            token.manualBurningRules != nil
        }
    }
    
    /// Find all tokens that can be frozen
    static func freezableTokensPredicate() -> Predicate<PersistentToken> {
        #Predicate<PersistentToken> { token in
            token.freezeRules != nil
        }
    }
    
    /// Find all tokens with distribution mechanisms
    static func distributionTokensPredicate() -> Predicate<PersistentToken> {
        #Predicate<PersistentToken> { token in
            token.perpetualDistribution != nil || token.preProgrammedDistribution != nil
        }
    }
    
    /// Find all paused tokens
    static func pausedTokensPredicate() -> Predicate<PersistentToken> {
        #Predicate<PersistentToken> { token in
            token.isPaused == true
        }
    }
    
    /// Find tokens by contract ID
    static func tokensByContractPredicate(contractId: Data) -> Predicate<PersistentToken> {
        #Predicate<PersistentToken> { token in
            token.contractId == contractId
        }
    }
    
    /// Find tokens with specific control rules
    static func tokensWithControlRulePredicate(rule: ControlRuleType) -> Predicate<PersistentToken> {
        switch rule {
        case .manualMinting:
            return #Predicate<PersistentToken> { token in
                token.manualMintingRules != nil
            }
        case .manualBurning:
            return #Predicate<PersistentToken> { token in
                token.manualBurningRules != nil
            }
        case .freeze:
            return #Predicate<PersistentToken> { token in
                token.freezeRules != nil
            }
        case .unfreeze:
            return #Predicate<PersistentToken> { token in
                token.unfreezeRules != nil
            }
        case .destroyFrozenFunds:
            return #Predicate<PersistentToken> { token in
                token.destroyFrozenFundsRules != nil
            }
        case .emergencyAction:
            return #Predicate<PersistentToken> { token in
                token.emergencyActionRules != nil
            }
        case .conventions:
            return #Predicate<PersistentToken> { token in
                token.conventionsChangeRules != nil
            }
        case .maxSupply:
            return #Predicate<PersistentToken> { token in
                token.maxSupplyChangeRules != nil
            }
        }
    }
}

enum ControlRuleType {
    case conventions
    case maxSupply
    case manualMinting
    case manualBurning
    case freeze
    case unfreeze
    case destroyFrozenFunds
    case emergencyAction
}

// Note: PersistentTokenHistoryEvent remains as a separate model