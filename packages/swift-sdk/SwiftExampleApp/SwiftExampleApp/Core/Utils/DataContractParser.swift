import Foundation
import SwiftData

struct DataContractParser {
    
    // MARK: - Parse Data Contract
    static func parseDataContract(contractData: [String: Any], contractId: Data, modelContext: ModelContext) throws {
        print("🔵 Parsing data contract with ID: \(contractId.toBase58String())")
        
        // Parse tokens if present
        if let tokens = contractData["tokens"] as? [String: Any] {
            print("📦 Found \(tokens.count) tokens in contract")
            try parseTokens(tokens: tokens, contractId: contractId, modelContext: modelContext)
        }
        
        // Parse document types
        if let documents = contractData["documents"] as? [String: Any] {
            print("📄 Found \(documents.count) document types in contract")
            try parseDocumentTypes(documentTypes: documents, contractId: contractId, modelContext: modelContext)
        } else if let documentSchemas = contractData["documentSchemas"] as? [String: Any] {
            // Some contracts use "documentSchemas" instead
            print("📄 Found \(documentSchemas.count) document schemas in contract")
            try parseDocumentTypes(documentTypes: documentSchemas, contractId: contractId, modelContext: modelContext)
        }
        
        // Update contract metadata
        if let existingContract = try? modelContext.fetch(
            FetchDescriptor<PersistentDataContract>(
                predicate: #Predicate { $0.id == contractId }
            )
        ).first {
            if let version = contractData["version"] as? Int {
                existingContract.version = version
            }
            if let ownerIdString = contractData["ownerId"] as? String,
               let ownerIdData = Data.identifier(fromBase58: ownerIdString) {
                existingContract.ownerId = ownerIdData
            }
            
            // Contract configuration
            if let canBeDeleted = contractData["canBeDeleted"] as? Bool {
                existingContract.canBeDeleted = canBeDeleted
            }
            if let readonly = contractData["readonly"] as? Bool {
                existingContract.readonly = readonly
            }
            if let keepsHistory = contractData["keepsHistory"] as? Bool {
                existingContract.keepsHistory = keepsHistory
            }
            if let schemaDefs = contractData["schemaDefs"] as? Int {
                existingContract.schemaDefs = schemaDefs
            }
            
            // Document defaults
            if let documentsKeepHistoryContractDefault = contractData["documentsKeepHistoryContractDefault"] as? Bool {
                existingContract.documentsKeepHistoryContractDefault = documentsKeepHistoryContractDefault
            }
            if let documentsMutableContractDefault = contractData["documentsMutableContractDefault"] as? Bool {
                existingContract.documentsMutableContractDefault = documentsMutableContractDefault
            }
            if let documentsCanBeDeletedContractDefault = contractData["documentsCanBeDeletedContractDefault"] as? Bool {
                existingContract.documentsCanBeDeletedContractDefault = documentsCanBeDeletedContractDefault
            }
        }
    }
    
    // MARK: - Parse Tokens
    private static func parseTokens(tokens: [String: Any], contractId: Data, modelContext: ModelContext) throws {
        // First, get the contract
        let descriptor = FetchDescriptor<PersistentDataContract>(
            predicate: #Predicate { $0.id == contractId }
        )
        guard let contract = try modelContext.fetch(descriptor).first else {
            print("⚠️ Could not find contract to link tokens")
            return
        }
        
        for (positionKey, tokenData) in tokens {
            guard let position = Int(positionKey),
                  let tokenDict = tokenData as? [String: Any] else {
                print("⚠️ Skipping invalid token at position: \(positionKey)")
                continue
            }
            
            // Extract token name (might be in different places)
            let tokenName = extractTokenName(from: tokenDict, position: position)
            
            // Extract base supply
            let baseSupply = extractTokenSupply(from: tokenDict, key: "baseSupply")
            print("📊 Token \(position) - Base Supply: \(baseSupply), raw value: \(tokenDict["baseSupply"] ?? "nil")")
            
            // Create persistent token
            let token = PersistentToken(
                contractId: contractId,
                position: position,
                name: tokenName,
                baseSupply: baseSupply
            )
            
            // Parse and set all token properties
            parseTokenConfiguration(token: token, from: tokenDict)
            
            // Link to contract
            token.dataContract = contract
            
            modelContext.insert(token)
            print("✅ Created token: \(tokenName) at position \(position)")
        }
    }
    
    // MARK: - Parse Document Types
    private static func parseDocumentTypes(documentTypes: [String: Any], contractId: Data, modelContext: ModelContext) throws {
        // First, get the contract
        let descriptor = FetchDescriptor<PersistentDataContract>(
            predicate: #Predicate { $0.id == contractId }
        )
        guard let contract = try modelContext.fetch(descriptor).first else {
            print("⚠️ Could not find contract to link document types")
            return
        }
        
        for (typeName, typeData) in documentTypes {
            guard let typeDict = typeData as? [String: Any] else {
                print("⚠️ Skipping invalid document type: \(typeName)")
                continue
            }
            
            // Extract schema - make sure we store the whole typeDict as schema
            // and only properties as the properties field
            let schemaJSON = try JSONSerialization.data(withJSONObject: typeDict, options: [])
            
            // Extract actual properties for the form
            let properties = typeDict["properties"] as? [String: Any] ?? [:]
            let propertiesJSON = try JSONSerialization.data(withJSONObject: properties, options: [])
            
            // Create document type
            let docType = PersistentDocumentType(
                contractId: contractId,
                name: typeName,
                schemaJSON: schemaJSON,
                propertiesJSON: propertiesJSON
            )
            
            // Set document behavior
            if let keepsHistory = typeDict["documentsKeepHistory"] as? Bool {
                docType.documentsKeepHistory = keepsHistory
            }
            
            if let mutable = typeDict["documentsMutable"] as? Bool {
                docType.documentsMutable = mutable
            }
            
            // The actual field name is just "canBeDeleted" not "documentsCanBeDeleted"
            if let canDelete = typeDict["canBeDeleted"] as? Bool {
                docType.documentsCanBeDeleted = canDelete
            }
            
            // The actual field name is "transferable" and it can be an integer (0 = false, non-zero = true)
            if let transferable = typeDict["transferable"] {
                // Handle both boolean and integer values (0 = false, non-zero = true)
                if let boolValue = transferable as? Bool {
                    docType.documentsTransferable = boolValue
                } else if let intValue = transferable as? Int {
                    docType.documentsTransferable = intValue != 0
                }
            }
            
            // Trade mode - can be integer or boolean
            if let tradeMode = typeDict["tradeMode"] {
                if let intValue = tradeMode as? Int {
                    docType.tradeMode = intValue
                } else if let boolValue = tradeMode as? Bool {
                    docType.tradeMode = boolValue ? 1 : 0
                }
            }
            
            // Creation restriction mode
            if let creationRestrictionMode = typeDict["creationRestrictionMode"] as? Int {
                docType.creationRestrictionMode = creationRestrictionMode
            }
            
            // Identity encryption keys
            if let requiresEncryption = typeDict["requiresIdentityEncryptionBoundedKey"] as? Bool {
                docType.requiresIdentityEncryptionBoundedKey = requiresEncryption
            }
            
            if let requiresDecryption = typeDict["requiresIdentityDecryptionBoundedKey"] as? Bool {
                docType.requiresIdentityDecryptionBoundedKey = requiresDecryption
            }
            
            // Extract required fields
            if let required = typeDict["required"] as? [String] {
                docType.requiredFieldsJSON = try? JSONSerialization.data(withJSONObject: required, options: [])
            }
            
            // Security level - the field name in contracts is "signatureSecurityLevelRequirement"
            if let securityLevel = typeDict["signatureSecurityLevelRequirement"] as? Int {
                docType.securityLevel = securityLevel
            } else if let securityLevel = typeDict["securityLevelRequirement"] as? Int {
                // Fallback to old name for compatibility
                docType.securityLevel = securityLevel
            } else {
                // Default to HIGH (value 2) as per DPP specification
                docType.securityLevel = 2
            }
            
            // Link to contract
            docType.dataContract = contract
            
            modelContext.insert(docType)
            print("✅ Created document type: \(typeName)")
            
            // Parse indices
            if let indices = typeDict["indices"] as? [[String: Any]] {
                try parseIndices(indices: indices, contractId: contractId, documentTypeName: typeName, documentType: docType, modelContext: modelContext)
            }
            
            // Parse properties into separate entities
            if let properties = typeDict["properties"] as? [String: Any] {
                try parseProperties(properties: properties, contractId: contractId, documentTypeName: typeName, documentType: docType, requiredFields: typeDict["required"] as? [String] ?? [], modelContext: modelContext)
            }
        }
    }
    
    // MARK: - Parse Indices
    private static func parseIndices(indices: [[String: Any]], contractId: Data, documentTypeName: String, documentType: PersistentDocumentType, modelContext: ModelContext) throws {
        for indexData in indices {
            guard let name = indexData["name"] as? String else {
                print("⚠️ Skipping index without name")
                continue
            }
            
            // Extract properties array with sorting
            let properties = indexData["properties"] as? [[String: Any]] ?? []
            var propertyNames: [String] = []
            
            // Parse property names with their sort order
            for prop in properties {
                if let propName = prop.keys.first {
                    // Include sort order if not default "asc"
                    if let sortOrder = prop[propName] as? String, sortOrder != "asc" {
                        propertyNames.append("\(propName) (\(sortOrder))")
                    } else {
                        propertyNames.append(propName)
                    }
                }
            }
            
            // Create persistent index
            let index = PersistentIndex(
                contractId: contractId,
                documentTypeName: documentTypeName,
                name: name,
                properties: propertyNames
            )
            
            // Set index attributes
            if let unique = indexData["unique"] as? Bool {
                index.unique = unique
            }
            
            if let nullSearchable = indexData["nullSearchable"] as? Bool {
                index.nullSearchable = nullSearchable
            }
            
            // Handle contested - can be bool or object
            if let contestedBool = indexData["contested"] as? Bool {
                index.contested = contestedBool
            } else if let contestedDict = indexData["contested"] as? [String: Any] {
                index.contested = true
                // Store contested details as JSON
                if let contestedData = try? JSONSerialization.data(withJSONObject: contestedDict, options: []) {
                    index.contestedDetailsJSON = contestedData
                }
            }
            
            // Link to document type
            index.documentType = documentType
            
            modelContext.insert(index)
            print("✅ Created index: \(name) for document type: \(documentTypeName)")
        }
    }
    
    // MARK: - Parse Properties
    private static func parseProperties(properties: [String: Any], contractId: Data, documentTypeName: String, documentType: PersistentDocumentType, requiredFields: [String], modelContext: ModelContext) throws {
        for (propertyName, propertyData) in properties {
            guard let propertyDict = propertyData as? [String: Any] else {
                print("⚠️ Skipping invalid property: \(propertyName)")
                continue
            }
            
            // Extract type
            let type = propertyDict["type"] as? String ?? "unknown"
            
            // Create persistent property
            let property = PersistentProperty(
                contractId: contractId,
                documentTypeName: documentTypeName,
                name: propertyName,
                type: type
            )
            
            // Set property attributes
            if let format = propertyDict["format"] as? String {
                property.format = format
            }
            
            if let contentMediaType = propertyDict["contentMediaType"] as? String {
                property.contentMediaType = contentMediaType
            }
            
            if let byteArray = propertyDict["byteArray"] as? Bool {
                property.byteArray = byteArray
            }
            
            if let minItems = propertyDict["minItems"] as? Int {
                property.minItems = minItems
            }
            
            if let maxItems = propertyDict["maxItems"] as? Int {
                property.maxItems = maxItems
            }
            
            if let pattern = propertyDict["pattern"] as? String {
                property.pattern = pattern
            }
            
            if let minLength = propertyDict["minLength"] as? Int {
                property.minLength = minLength
            }
            
            if let maxLength = propertyDict["maxLength"] as? Int {
                property.maxLength = maxLength
            }
            
            if let minValue = propertyDict["minValue"] as? Int {
                property.minValue = minValue
            } else if let minimum = propertyDict["minimum"] as? Int {
                property.minValue = minimum
            }
            
            if let maxValue = propertyDict["maxValue"] as? Int {
                property.maxValue = maxValue
            } else if let maximum = propertyDict["maximum"] as? Int {
                property.maxValue = maximum
            }
            
            if let description = propertyDict["description"] as? String {
                property.fieldDescription = description
                print("  📝 Property \(propertyName) has description: \(description)")
            } else {
                print("  ⚠️ Property \(propertyName) has no description")
            }
            
            if let transient = propertyDict["transient"] as? Bool {
                property.transient = transient
            }
            
            // Check if required
            property.isRequired = requiredFields.contains(propertyName)
            
            // Link to document type
            property.documentType = documentType
            
            modelContext.insert(property)
            print("✅ Created property: \(propertyName) for document type: \(documentTypeName)")
        }
    }
    
    // MARK: - Helper Methods
    private static func extractTokenName(from tokenDict: [String: Any], position: Int) -> String {
        // Try different possible locations for the name
        if let name = tokenDict["name"] as? String { return name }
        if let conventions = tokenDict["conventions"] as? [String: Any],
           let name = conventions["name"] as? String { return name }
        if let description = tokenDict["description"] as? String { return description }
        return "Token \(position)"
    }
    
    private static func extractTokenSupply(from tokenDict: [String: Any], key: String) -> String {
        // Handle different number formats
        if let supplyInt = tokenDict[key] as? Int {
            return String(supplyInt)
        }
        if let supplyDouble = tokenDict[key] as? Double {
            return String(format: "%.0f", supplyDouble)
        }
        if let supplyString = tokenDict[key] as? String {
            return supplyString
        }
        return "0"
    }
    
    private static func parseTokenConfiguration(token: PersistentToken, from tokenDict: [String: Any]) {
        // Basic properties
        let maxSupplyStr = extractTokenSupply(from: tokenDict, key: "maxSupply")
        if maxSupplyStr != "0" {
            token.maxSupply = maxSupplyStr
        }
        
        if let decimals = tokenDict["decimals"] as? Int {
            token.decimals = decimals
        }
        
        if let description = tokenDict["description"] as? String {
            token.tokenDescription = description
        }
        
        // Status flags
        if let startAsPaused = tokenDict["startAsPaused"] as? Bool {
            token.isPaused = startAsPaused
        }
        
        if let allowTransfer = tokenDict["allowTransferToFrozenBalance"] as? Bool {
            token.allowTransferToFrozenBalance = allowTransfer
        }
        
        // Parse conventions/localizations
        if let conventions = tokenDict["conventions"] as? [String: Any] {
            if let decimals = conventions["decimals"] as? Int {
                token.decimals = decimals
            }
            if let localizations = conventions["localizations"] as? [String: Any] {
                var tokenLocalizations: [String: TokenLocalization] = [:]
                for (langCode, locData) in localizations {
                    if let locDict = locData as? [String: Any] {
                        // Skip format version keys
                        if langCode == "$format_version" { continue }
                        
                        tokenLocalizations[langCode] = TokenLocalization(
                            singularForm: locDict["singular"] as? String ?? locDict["singularForm"] as? String ?? "",
                            pluralForm: locDict["plural"] as? String ?? locDict["pluralForm"] as? String ?? "",
                            description: locDict["description"] as? String
                        )
                    }
                }
                token.localizations = tokenLocalizations
            }
        }
        
        // Parse history keeping rules
        if let keepsHistory = tokenDict["keepsHistory"] as? [String: Any] {
            token.keepsTransferHistory = keepsHistory["keepsTransferHistory"] as? Bool ?? true
            token.keepsFreezingHistory = keepsHistory["keepsFreezingHistory"] as? Bool ?? true
            token.keepsMintingHistory = keepsHistory["keepsMintingHistory"] as? Bool ?? true
            token.keepsBurningHistory = keepsHistory["keepsBurningHistory"] as? Bool ?? true
            token.keepsDirectPricingHistory = keepsHistory["keepsDirectPricingHistory"] as? Bool ?? true
            token.keepsDirectPurchaseHistory = keepsHistory["keepsDirectPurchaseHistory"] as? Bool ?? true
        } else if let keepsHistory = tokenDict["keepsHistory"] as? Bool {
            // Simple boolean for all history
            token.keepsTransferHistory = keepsHistory
            token.keepsFreezingHistory = keepsHistory
            token.keepsMintingHistory = keepsHistory
            token.keepsBurningHistory = keepsHistory
            token.keepsDirectPricingHistory = keepsHistory
            token.keepsDirectPurchaseHistory = keepsHistory
        }
        
        // Parse control rules
        token.conventionsChangeRules = parseChangeControlRule(tokenDict["conventionsChangeRules"])
        token.maxSupplyChangeRules = parseChangeControlRule(tokenDict["maxSupplyChangeRules"])
        token.manualMintingRules = parseChangeControlRule(tokenDict["manualMintingRules"])
        token.manualBurningRules = parseChangeControlRule(tokenDict["manualBurningRules"])
        token.freezeRules = parseChangeControlRule(tokenDict["freezeRules"])
        token.unfreezeRules = parseChangeControlRule(tokenDict["unfreezeRules"])
        token.destroyFrozenFundsRules = parseChangeControlRule(tokenDict["destroyFrozenFundsRules"])
        token.emergencyActionRules = parseChangeControlRule(tokenDict["emergencyActionRules"])
        
        // Parse distribution rules
        if let distributionRules = tokenDict["distributionRules"] as? [String: Any] {
            // Perpetual distribution
            if let perpetual = distributionRules["perpetualDistribution"] as? [String: Any] {
                var dist = TokenPerpetualDistribution()
                if let distType = perpetual["distributionType"] {
                    // Convert to JSON string for storage
                    if let jsonData = try? JSONSerialization.data(withJSONObject: distType, options: []),
                       let jsonString = String(data: jsonData, encoding: .utf8) {
                        dist.distributionType = jsonString
                    } else {
                        dist.distributionType = "{}"
                    }
                }
                if let recipient = perpetual["distributionRecipient"] as? String {
                    dist.distributionRecipient = recipient
                }
                // Set enabled flag if it exists (defaults to true in init)
                if let enabled = perpetual["enabled"] as? Bool {
                    dist.enabled = enabled
                } else {
                    dist.enabled = true // Default to enabled if not specified
                }
                token.perpetualDistribution = dist
            }
            
            // Pre-programmed distribution
            if let preProgrammed = distributionRules["preProgrammedDistribution"] as? [String: Any] {
                var dist = TokenPreProgrammedDistribution()
                if let schedule = preProgrammed["distributionSchedule"] as? [[String: Any]] {
                    dist.distributionSchedule = schedule.compactMap { eventDict in
                        guard let amount = eventDict["amount"] as? String else { return nil }
                        var event = DistributionEvent(
                            triggerTime: Date(),
                            amount: amount
                        )
                        if let triggerType = eventDict["triggerType"] as? String {
                            event.triggerType = triggerType
                        }
                        if let time = eventDict["triggerTime"] as? TimeInterval {
                            event.triggerTime = Date(timeIntervalSince1970: time)
                        }
                        if let block = eventDict["triggerBlock"] as? Int64 {
                            event.triggerBlock = block
                        }
                        if let condition = eventDict["triggerCondition"] as? String {
                            event.triggerCondition = condition
                        }
                        if let recipient = eventDict["recipient"] as? String {
                            event.recipient = recipient
                        }
                        if let desc = eventDict["description"] as? String {
                            event.description = desc
                        }
                        return event
                    }
                }
                token.preProgrammedDistribution = dist
            }
            
            // New tokens destination
            if let destinationId = distributionRules["newTokensDestinationIdentity"] as? String,
               let destinationData = Data.identifier(fromBase58: destinationId) {
                token.newTokensDestinationIdentity = destinationData
            }
            
            // Minting destination choice
            if let allowChoice = distributionRules["mintingAllowChoosingDestination"] as? Bool {
                token.mintingAllowChoosingDestination = allowChoice
            }
            
            // Store distribution change rules
            var changeRules = TokenDistributionChangeRules()
            changeRules.perpetualDistributionRules = parseChangeControlRule(distributionRules["perpetualDistributionRules"])
            changeRules.newTokensDestinationIdentityRules = parseChangeControlRule(distributionRules["newTokensDestinationIdentityRules"])
            changeRules.mintingAllowChoosingDestinationRules = parseChangeControlRule(distributionRules["mintingAllowChoosingDestinationRules"])
            changeRules.changeDirectPurchasePricingRules = parseChangeControlRule(distributionRules["changeDirectPurchasePricingRules"])
            token.distributionChangeRules = changeRules
        }
        
        // Parse marketplace rules
        if let marketplaceRules = tokenDict["marketplaceRules"] as? [String: Any] {
            if let tradeModeStr = marketplaceRules["tradeMode"] as? String,
               let tradeMode = TokenTradeMode(rawValue: tradeModeStr) {
                token.tradeMode = tradeMode
            }
            token.tradeModeChangeRules = parseChangeControlRule(marketplaceRules["tradeModeChangeRules"])
        }
        
        // Main control group
        if let mainControlGroup = tokenDict["mainControlGroup"] as? Int {
            token.mainControlGroupPosition = mainControlGroup
        }
        
        if let canModify = tokenDict["mainControlGroupCanBeModified"] as? String {
            token.mainControlGroupCanBeModified = canModify
        }
    }
    
    private static func parseChangeControlRule(_ ruleData: Any?) -> ChangeControlRules? {
        guard let ruleContainer = ruleData as? [String: Any] else { return nil }
        
        // Handle V0 format where the actual rules are nested under "V0" key
        let rule: [String: Any]
        if let v0Rules = ruleContainer["V0"] as? [String: Any] {
            rule = v0Rules
        } else {
            // Fall back to direct format if not wrapped in V0
            rule = ruleContainer
        }
        
        var controlRules = ChangeControlRules.mostRestrictive()
        
        // Handle both snake_case (from JSON) and camelCase
        if let authorized = rule["authorized_to_make_change"] as? String ?? rule["authorizedToMakeChange"] as? String {
            controlRules.authorizedToMakeChange = authorized
        }
        
        if let admin = rule["admin_action_takers"] as? String ?? rule["adminActionTakers"] as? String {
            controlRules.adminActionTakers = admin
        }
        
        if let flag = rule["changing_authorized_action_takers_to_no_one_allowed"] as? Bool ?? rule["changingAuthorizedActionTakersToNoOneAllowed"] as? Bool {
            controlRules.changingAuthorizedActionTakersToNoOneAllowed = flag
        }
        
        if let flag = rule["changing_admin_action_takers_to_no_one_allowed"] as? Bool ?? rule["changingAdminActionTakersToNoOneAllowed"] as? Bool {
            controlRules.changingAdminActionTakersToNoOneAllowed = flag
        }
        
        if let flag = rule["self_changing_admin_action_takers_allowed"] as? Bool ?? rule["selfChangingAdminActionTakersAllowed"] as? Bool {
            controlRules.selfChangingAdminActionTakersAllowed = flag
        }
        
        return controlRules
    }
}