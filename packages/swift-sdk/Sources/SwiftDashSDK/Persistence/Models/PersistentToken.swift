import Foundation
import SwiftData

/// SwiftData model for persisting token configuration
@Model
public final class PersistentToken {
    @Attribute(.unique) public var id: Data
    public var contractId: Data
    public var position: Int
    public var name: String

    // Basic token supply info
    public var baseSupply: String
    public var maxSupply: String?
    public var decimals: Int

    // Token conventions
    public var localizations: [String: TokenLocalization]?

    // Status flags
    public var isPaused: Bool
    public var allowTransferToFrozenBalance: Bool

    // History keeping rules
    public var keepsTransferHistory: Bool
    public var keepsFreezingHistory: Bool
    public var keepsMintingHistory: Bool
    public var keepsBurningHistory: Bool
    public var keepsDirectPricingHistory: Bool
    public var keepsDirectPurchaseHistory: Bool

    // Control rules
    public var conventionsChangeRules: ChangeControlRules?
    public var maxSupplyChangeRules: ChangeControlRules?
    public var manualMintingRules: ChangeControlRules?
    public var manualBurningRules: ChangeControlRules?
    public var freezeRules: ChangeControlRules?
    public var unfreezeRules: ChangeControlRules?
    public var destroyFrozenFundsRules: ChangeControlRules?
    public var emergencyActionRules: ChangeControlRules?

    // Distribution rules
    public var perpetualDistribution: TokenPerpetualDistribution?
    public var preProgrammedDistribution: TokenPreProgrammedDistribution?
    public var newTokensDestinationIdentity: Data?
    public var mintingAllowChoosingDestination: Bool
    public var distributionChangeRules: TokenDistributionChangeRules?

    // Marketplace rules
    public var tradeMode: TokenTradeMode
    public var tradeModeChangeRules: ChangeControlRules?

    // Main control group
    public var mainControlGroupPosition: Int?
    public var mainControlGroupCanBeModified: String?

    // Description
    public var tokenDescription: String?

    // Timestamps
    public var createdAt: Date
    public var lastUpdatedAt: Date

    // Relationships
    public var dataContract: PersistentDataContract?

    @Relationship(deleteRule: .cascade)
    public var balances: [PersistentTokenBalance]?

    @Relationship(deleteRule: .cascade)
    public var historyEvents: [PersistentTokenHistoryEvent]?

    public init(contractId: Data, position: Int, name: String, baseSupply: String, decimals: Int = 8) {
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
    public var displayName: String {
        if let desc = tokenDescription, !desc.isEmpty {
            return desc
        }
        return getSingularForm() ?? name
    }

    public var formattedBaseSupply: String {
        Self.formatSupply(baseSupply, decimals: decimals)
    }

    /// Exact formatter for protocol integer strings. It never passes through
    /// `Double`/`Int`, so `UInt64.max` and decimals-zero supplies are safe.
    public static func formatSupply(_ raw: String, decimals: Int) -> String {
        guard !raw.isEmpty, raw.allSatisfy({ $0.isASCII && $0.isNumber }) else {
            return raw
        }
        let normalized = String(raw.drop(while: { $0 == "0" }))
        let digits = normalized.isEmpty ? "0" : normalized
        let scale = max(0, decimals)
        let integer: String
        var fraction = ""
        if scale == 0 {
            integer = digits
        } else if digits.count <= scale {
            integer = "0"
            fraction = String(repeating: "0", count: scale - digits.count) + digits
        } else {
            let split = digits.index(digits.endIndex, offsetBy: -scale)
            integer = String(digits[..<split])
            fraction = String(digits[split...])
        }
        while fraction.last == "0" { fraction.removeLast() }

        var grouped = ""
        for (offset, character) in integer.reversed().enumerated() {
            if offset > 0 && offset.isMultiple(of: 3) { grouped.append(",") }
            grouped.append(character)
        }
        grouped = String(grouped.reversed())
        return fraction.isEmpty ? grouped : "\(grouped).\(fraction)"
    }

    public var contractIdBase58: String {
        contractId.toBase58String()
    }

    // MARK: - Indexed Properties for Querying

    public var canManuallyMint: Bool {
        manualMintingRules != nil
    }

    public var canManuallyBurn: Bool {
        manualBurningRules != nil
    }

    public var canFreeze: Bool {
        freezeRules != nil
    }

    public var canUnfreeze: Bool {
        unfreezeRules != nil
    }

    public var canDestroyFrozenFunds: Bool {
        destroyFrozenFundsRules != nil
    }

    public var hasEmergencyActions: Bool {
        emergencyActionRules != nil
    }

    public var canChangeMaxSupply: Bool {
        maxSupplyChangeRules != nil
    }

    public var canChangeConventions: Bool {
        conventionsChangeRules != nil
    }

    public var hasDistribution: Bool {
        perpetualDistribution != nil || preProgrammedDistribution != nil
    }

    public var canChangeTradeMode: Bool {
        tradeModeChangeRules != nil
    }

    public var keepsAnyHistory: Bool {
        keepsTransferHistory ||
        keepsFreezingHistory ||
        keepsMintingHistory ||
        keepsBurningHistory ||
        keepsDirectPricingHistory ||
        keepsDirectPurchaseHistory
    }

    public var totalSupply: String {
        guard let balances = balances, !balances.isEmpty else { return baseSupply }
        return Self.sumUnsignedBalances(balances.map(\.unsignedBalance))
    }

    public var totalFrozenBalance: String {
        guard let balances = balances else { return "0" }
        return Self.sumUnsignedBalances(
            balances.lazy.filter(\.frozen).map(\.unsignedBalance)
        )
    }

    public var activeHolders: Int {
        balances?.filter { $0.unsignedBalance > 0 }.count ?? 0
    }

    /// Sums protocol amounts without narrowing either individual balances or
    /// the aggregate to a signed/fixed-width integer. Several valid UInt64
    /// balances can exceed UInt64.max when combined.
    private static func sumUnsignedBalances<S: Sequence>(_ values: S) -> String
    where S.Element == UInt64 {
        var digits: [UInt8] = [0] // little-endian decimal digits

        for value in values {
            var carry = 0
            let addend = String(value).utf8.reversed().map { Int($0 - 48) }
            let width = max(digits.count, addend.count)
            if digits.count < width {
                digits.append(contentsOf: repeatElement(0, count: width - digits.count))
            }

            for index in 0..<width {
                let sum = Int(digits[index]) + (index < addend.count ? addend[index] : 0) + carry
                digits[index] = UInt8(sum % 10)
                carry = sum / 10
            }
            while carry > 0 {
                digits.append(UInt8(carry % 10))
                carry /= 10
            }
        }

        return String(digits.reversed().map { Character(String($0)) })
    }

    public var hasMaxSupply: Bool {
        maxSupply != nil
    }

    public var isTradeable: Bool {
        tradeMode != .notTradeable
    }

    public var newTokensDestinationIdentityBase58: String? {
        newTokensDestinationIdentity?.toBase58String()
    }
}

// MARK: - Localization Methods
extension PersistentToken {
    public func setLocalization(languageCode: String, singularForm: String, pluralForm: String, description: String? = nil) {
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

    public func getSingularForm(languageCode: String = "en") -> String? {
        return localizations?[languageCode]?.singularForm ?? localizations?["en"]?.singularForm
    }

    public func getPluralForm(languageCode: String = "en") -> String? {
        return localizations?[languageCode]?.pluralForm ?? localizations?["en"]?.pluralForm
    }
}

// MARK: - Control Rules Methods
extension PersistentToken {
    public func getChangeControlRules(for type: ChangeControlRuleType) -> ChangeControlRules? {
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

    public func setChangeControlRules(_ rules: ChangeControlRules, for type: ChangeControlRuleType) {
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

// MARK: - Query Helpers
extension PersistentToken {
    public static func mintableTokensPredicate() -> Predicate<PersistentToken> {
        #Predicate<PersistentToken> { token in
            token.manualMintingRules != nil
        }
    }

    public static func burnableTokensPredicate() -> Predicate<PersistentToken> {
        #Predicate<PersistentToken> { token in
            token.manualBurningRules != nil
        }
    }

    public static func freezableTokensPredicate() -> Predicate<PersistentToken> {
        #Predicate<PersistentToken> { token in
            token.freezeRules != nil
        }
    }

    public static func distributionTokensPredicate() -> Predicate<PersistentToken> {
        #Predicate<PersistentToken> { token in
            token.perpetualDistribution != nil || token.preProgrammedDistribution != nil
        }
    }

    public static func pausedTokensPredicate() -> Predicate<PersistentToken> {
        #Predicate<PersistentToken> { token in
            token.isPaused == true
        }
    }

    public static func tokensByContractPredicate(contractId: Data) -> Predicate<PersistentToken> {
        #Predicate<PersistentToken> { token in
            token.contractId == contractId
        }
    }

    public static func tokensWithControlRulePredicate(rule: ControlRuleType) -> Predicate<PersistentToken> {
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
