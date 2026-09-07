import Foundation
import SwiftData

// Inline value types exactly as schema DashSchemaV1 stored them, generated
// by scripts/freeze_schema_models.py from TokenTypes.swift
// at commit 96a103375e. SwiftData expands a stored Codable struct into composite
// attributes of the owning entity, so these shapes are inputs to that
// version's checksum just like the model's own properties. Do not edit.
extension DashSchemaV1 {
    struct ChangeControlRules: Codable, Equatable, Sendable {
        var authorizedToMakeChange: String
        var adminActionTakers: String
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

    enum AuthorizedActionTakers: String, CaseIterable, Codable, Sendable {
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

    struct TokenPerpetualDistribution: Codable, Equatable, Sendable {
        var distributionType: String
        var distributionRecipient: String
        var enabled: Bool
        var lastDistributionTime: Date?
        var nextDistributionTime: Date?

        init(distributionRecipient: String = "AllEqualShare", enabled: Bool = true) {
            self.distributionType = "{}"
            self.distributionRecipient = distributionRecipient
            self.enabled = enabled
        }
    }

    struct TokenPreProgrammedDistribution: Codable, Equatable, Sendable {
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

    struct DistributionEvent: Codable, Equatable, Sendable {
        var id: UUID
        var triggerType: String
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

    struct TokenDistributionChangeRules: Codable, Equatable, Sendable {
        var perpetualDistributionRules: ChangeControlRules?
        var newTokensDestinationIdentityRules: ChangeControlRules?
        var mintingAllowChoosingDestinationRules: ChangeControlRules?
        var changeDirectPurchasePricingRules: ChangeControlRules?

        init(
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

    enum TokenTradeMode: String, CaseIterable, Codable, Sendable {
        case notTradeable = "NotTradeable"

        var displayName: String {
            switch self {
            case .notTradeable:
                return "Not Tradeable"
            }
        }
    }

    struct TokenLocalization: Codable, Equatable, Sendable {
        let singularForm: String
        let pluralForm: String
        let description: String?

        init(singularForm: String, pluralForm: String, description: String? = nil) {
            self.singularForm = singularForm
            self.pluralForm = pluralForm
            self.description = description
        }
    }
}
