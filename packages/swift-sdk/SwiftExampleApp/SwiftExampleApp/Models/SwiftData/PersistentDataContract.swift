import Foundation
import SwiftData
import SwiftDashSDK

// Re-export SDK type for backward compatibility
public typealias PersistentDataContract = SwiftDashSDK.PersistentDataContract

// App-specific extensions that depend on app types
extension SwiftDashSDK.PersistentDataContract {
    /// Convert to app's ContractModel
    func toContractModel() -> ContractModel {
        var tokenConfigs: [TokenConfiguration] = []
        if let tokensDict = tokenConfigurations {
            tokenConfigs = tokensDict.compactMap { (_, value) in
                guard value is [String: Any] else { return nil }
                return nil
            }
        }

        return ContractModel(
            id: idBase58,
            name: name,
            version: version ?? 1,
            ownerId: ownerId ?? Data(),
            documentTypes: documentTypesList,
            schema: schema,
            dppDataContract: nil,
            tokens: tokenConfigs,
            keywords: self.keywords,
            description: contractDescription
        )
    }

    /// Create from ContractModel
    static func from(_ model: ContractModel, network: String = "testnet") -> SwiftDashSDK.PersistentDataContract {
        let idData = Data.identifier(fromBase58: model.id) ?? Data()
        let persistent = SwiftDashSDK.PersistentDataContract(
            id: idData,
            name: model.name,
            serializedContract: Data(),
            version: model.version,
            ownerId: model.ownerId,
            schema: model.schema,
            documentTypesList: model.documentTypes,
            keywords: model.keywords,
            description: model.description,
            hasTokens: !model.tokens.isEmpty,
            network: network
        )

        if let serialized = try? JSONSerialization.data(withJSONObject: model.schema) {
            persistent.serializedContract = serialized
        }

        if !model.tokens.isEmpty {
            var tokensDict: [String: Any] = [:]
            for token in model.tokens {
                tokensDict[token.symbol] = tokenConfigurationToJSON(token)
            }
            persistent.tokenConfigurations = tokensDict
        }

        if let dppContract = model.dppDataContract {
            var schemaDict: [String: Any] = [:]
            for (docType, documentType) in dppContract.documentTypes {
                var docSchema: [String: Any] = [:]
                docSchema["type"] = "object"
                docSchema["indices"] = documentType.indices.map { index in
                    return [
                        "name": index.name,
                        "properties": index.properties.map { $0.name },
                        "unique": index.unique
                    ]
                }
                docSchema["properties"] = documentType.properties.mapValues { prop in
                    return ["type": prop.type.rawValue]
                }
                schemaDict[docType] = docSchema
            }
            persistent.schema = schemaDict

            if !dppContract.groups.isEmpty {
                var groupsDict: [String: Any] = [:]
                for (groupId, group) in dppContract.groups {
                    groupsDict[String(groupId)] = [
                        "members": group.members.map { member in
                            Data(member).base64EncodedString()
                        },
                        "requiredPower": group.requiredPower
                    ]
                }
                persistent.groups = groupsDict
            }
        }

        return persistent
    }

    private static func tokenConfigurationToJSON(_ token: TokenConfiguration) -> [String: Any] {
        return [
            "name": token.name,
            "symbol": token.symbol,
            "description": token.description as Any,
            "decimals": token.decimals,
            "totalSupplyInLowestDenomination": token.totalSupplyInLowestDenomination,
            "mintable": token.mintable,
            "burnable": token.burnable,
            "cappedSupply": token.cappedSupply,
            "transferable": token.transferable,
            "tradeable": token.tradeable,
            "sellable": token.sellable,
            "freezable": token.freezable,
            "pausable": token.pausable
        ]
    }
}
