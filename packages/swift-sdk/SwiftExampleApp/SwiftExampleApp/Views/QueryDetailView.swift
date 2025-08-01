import SwiftUI
import SwiftDashSDK

struct QueryDetailView: View {
    let query: QueryDefinition
    @EnvironmentObject var appState: UnifiedAppState
    @State private var queryInputs: [String: String] = [:]
    @State private var isLoading = false
    @State private var result: String = ""
    @State private var error: String = ""
    @State private var showResult = false
    
    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 20) {
                // Description
                VStack(alignment: .leading, spacing: 8) {
                    Text("Description")
                        .font(.headline)
                    Text(query.description)
                        .font(.body)
                        .foregroundColor(.secondary)
                }
                .padding()
                .background(Color.gray.opacity(0.1))
                .cornerRadius(10)
                
                // Input Fields
                VStack(alignment: .leading, spacing: 16) {
                    Text("Parameters")
                        .font(.headline)
                    
                    ForEach(inputFields(for: query.name), id: \.name) { input in
                        VStack(alignment: .leading, spacing: 4) {
                            HStack {
                                Text(input.label)
                                    .font(.subheadline)
                                    .fontWeight(.medium)
                                if input.required {
                                    Text("*")
                                        .foregroundColor(.red)
                                }
                            }
                            
                            if let placeholder = input.placeholder {
                                Text(placeholder)
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                            }
                            
                            TextField(input.label, text: binding(for: input.name))
                                .textFieldStyle(RoundedBorderTextFieldStyle())
                                .autocapitalization(.none)
                                .disableAutocorrection(true)
                        }
                    }
                }
                .padding()
                
                // Execute Button
                Button(action: executeQuery) {
                    HStack {
                        if isLoading {
                            ProgressView()
                                .progressViewStyle(CircularProgressViewStyle(tint: .white))
                                .scaleEffect(0.8)
                        } else {
                            Image(systemName: "play.fill")
                        }
                        Text("Execute Query")
                            .fontWeight(.semibold)
                    }
                    .frame(maxWidth: .infinity)
                    .padding()
                    .background(isLoading ? Color.gray : Color.blue)
                    .foregroundColor(.white)
                    .cornerRadius(10)
                }
                .disabled(isLoading || !hasRequiredInputs())
                .padding(.horizontal)
                
                // Result Section
                if showResult {
                    VStack(alignment: .leading, spacing: 8) {
                        Text("Result")
                            .font(.headline)
                        
                        ScrollView(.horizontal) {
                            Text(result.isEmpty ? "No result" : result)
                                .font(.system(.body, design: .monospaced))
                                .padding()
                                .background(Color.gray.opacity(0.1))
                                .cornerRadius(8)
                                .textSelection(.enabled)
                        }
                    }
                    .padding()
                }
                
                // Error Section
                if !error.isEmpty {
                    VStack(alignment: .leading, spacing: 8) {
                        Text("Error")
                            .font(.headline)
                            .foregroundColor(.red)
                        
                        Text(error)
                            .font(.body)
                            .foregroundColor(.red)
                            .padding()
                            .background(Color.red.opacity(0.1))
                            .cornerRadius(8)
                    }
                    .padding()
                }
            }
        }
        .navigationTitle(query.label)
        .navigationBarTitleDisplayMode(.inline)
    }
    
    private func binding(for key: String) -> Binding<String> {
        Binding(
            get: { queryInputs[key] ?? "" },
            set: { queryInputs[key] = $0 }
        )
    }
    
    private func hasRequiredInputs() -> Bool {
        let fields = inputFields(for: query.name)
        for field in fields where field.required {
            if (queryInputs[field.name] ?? "").isEmpty {
                return false
            }
        }
        return true
    }
    
    private func executeQuery() {
        guard let sdk = appState.platformState.sdk else {
            error = "SDK not initialized"
            return
        }
        
        isLoading = true
        error = ""
        result = ""
        showResult = false
        
        Task {
            do {
                let queryResult = try await performQuery(sdk: sdk)
                await MainActor.run {
                    result = formatResult(queryResult)
                    showResult = true
                    isLoading = false
                }
            } catch {
                await MainActor.run {
                    self.error = error.localizedDescription
                    isLoading = false
                }
            }
        }
    }
    
    private func performQuery(sdk: SDK) async throws -> Any {
        switch query.name {
        // Identity Queries
        case "getIdentity":
            let id = queryInputs["id"] ?? ""
            return try await sdk.identityGet(identityId: id)
            
        case "getIdentityKeys":
            let identityId = queryInputs["identityId"] ?? ""
            return try await sdk.identityGetKeys(identityId: identityId)
            
        case "getIdentitiesContractKeys":
            let identityIds = (queryInputs["identitiesIds"] ?? "").split(separator: ",").map { String($0.trimmingCharacters(in: .whitespaces)) }
            let contractId = queryInputs["contractId"] ?? ""
            let documentType = queryInputs["documentTypeName"]
            return try await sdk.identityGetContractKeys(identityIds: identityIds, contractId: contractId, documentType: documentType)
            
        case "getIdentityNonce":
            let identityId = queryInputs["identityId"] ?? ""
            return try await sdk.identityGetNonce(identityId: identityId)
            
        case "getIdentityContractNonce":
            let identityId = queryInputs["identityId"] ?? ""
            let contractId = queryInputs["contractId"] ?? ""
            return try await sdk.identityGetContractNonce(identityId: identityId, contractId: contractId)
            
        case "getIdentityBalance":
            let id = queryInputs["id"] ?? ""
            return try await sdk.identityGetBalance(identityId: id)
            
        case "getIdentitiesBalances":
            let identityIds = (queryInputs["identityIds"] ?? "").split(separator: ",").map { String($0.trimmingCharacters(in: .whitespaces)) }
            return try await sdk.identityGetBalances(identityIds: identityIds)
            
        case "getIdentityBalanceAndRevision":
            let id = queryInputs["id"] ?? ""
            return try await sdk.identityGetBalanceAndRevision(identityId: id)
            
        case "getIdentityByPublicKeyHash":
            let publicKeyHash = queryInputs["publicKeyHash"] ?? ""
            return try await sdk.identityGetByPublicKeyHash(publicKeyHash: publicKeyHash)
            
        case "getIdentityByNonUniquePublicKeyHash":
            let publicKeyHash = queryInputs["publicKeyHash"] ?? ""
            return try await sdk.identityGetByNonUniquePublicKeyHash(publicKeyHash: publicKeyHash)
            
        // Data Contract Queries
        case "getDataContract":
            let id = queryInputs["id"] ?? ""
            return try await sdk.dataContractGet(id: id)
            
        case "getDataContractHistory":
            let id = queryInputs["id"] ?? ""
            let limitStr = queryInputs["limit"] ?? ""
            let offsetStr = queryInputs["offset"] ?? ""
            let limit = limitStr.isEmpty ? nil : UInt32(limitStr)
            let offset = offsetStr.isEmpty ? nil : UInt32(offsetStr)
            return try await sdk.dataContractGetHistory(id: id, limit: limit, offset: offset)
            
        case "getDataContracts":
            let ids = (queryInputs["ids"] ?? "").split(separator: ",").map { String($0.trimmingCharacters(in: .whitespaces)) }
            return try await sdk.dataContractGetMultiple(ids: ids)
            
        // Document Queries
        case "getDocuments":
            let contractId = queryInputs["dataContractId"] ?? ""
            let documentType = queryInputs["documentType"] ?? ""
            let whereClause = queryInputs["whereClause"]
            let orderBy = queryInputs["orderBy"]
            let limitStr = queryInputs["limit"] ?? ""
            let limit = limitStr.isEmpty ? nil : UInt32(limitStr)
            
            return try await sdk.documentList(
                dataContractId: contractId,
                documentType: documentType,
                whereClause: whereClause,
                orderByClause: orderBy,
                limit: limit
            )
            
        case "getDocument":
            let contractId = queryInputs["dataContractId"] ?? ""
            let documentType = queryInputs["documentType"] ?? ""
            let documentId = queryInputs["documentId"] ?? ""
            return try await sdk.documentGet(dataContractId: contractId, documentType: documentType, documentId: documentId)
            
        // DPNS Queries
        case "getDpnsUsername":
            let identityId = queryInputs["identityId"] ?? ""
            let limitStr = queryInputs["limit"] ?? ""
            let limit = limitStr.isEmpty ? nil : UInt32(limitStr)
            return try await sdk.dpnsGetUsername(identityId: identityId, limit: limit)
            
        case "dpnsCheckAvailability":
            let label = queryInputs["label"] ?? ""
            return try await sdk.dpnsCheckAvailability(name: label)
            
        case "dpnsResolve":
            let name = queryInputs["name"] ?? ""
            return try await sdk.dpnsResolve(name: name)
            
        // Voting & Contested Resources Queries
        case "getContestedResources":
            let resourceType = queryInputs["resourceType"] ?? "dpns"
            let limitStr = queryInputs["limit"] ?? ""
            let offsetStr = queryInputs["offset"] ?? ""
            let limit = limitStr.isEmpty ? nil : UInt32(limitStr)
            let offset = offsetStr.isEmpty ? nil : UInt32(offsetStr)
            return try await sdk.getContestedResources(resourceType: resourceType, limit: limit, offset: offset)
            
        case "getContestedResourceVotes":
            let resourceId = queryInputs["resourceId"] ?? ""
            return try await sdk.getContestedResourceVotes(resourceId: resourceId)
            
        case "getMasternodeVotes":
            let masternodeId = queryInputs["masternodeId"] ?? ""
            return try await sdk.getMasternodeVotes(masternodeId: masternodeId)
            
        case "getActiveProposals":
            return try await sdk.getActiveProposals()
            
        case "getProposal":
            let proposalId = queryInputs["proposalId"] ?? ""
            return try await sdk.getProposal(proposalId: proposalId)
            
        // Protocol & Version Queries
        case "getProtocolVersion":
            return try await sdk.getProtocolVersion()
            
        case "getVersionUpgradeState":
            return try await sdk.getVersionUpgradeState()
            
        // Epoch & Block Queries
        case "getCurrentEpoch":
            return try await sdk.getCurrentEpoch()
            
        case "getEpoch":
            let epochIndexStr = queryInputs["epochIndex"] ?? ""
            let epochIndex = UInt32(epochIndexStr) ?? 0
            return try await sdk.getEpoch(epochIndex: epochIndex)
            
        case "getBestBlockHeight":
            return try await sdk.getBestBlockHeight()
            
        case "getBlock":
            let heightStr = queryInputs["height"] ?? ""
            let height = UInt64(heightStr) ?? 0
            return try await sdk.getBlock(height: height)
            
        case "getBlockByHash":
            let hash = queryInputs["hash"] ?? ""
            return try await sdk.getBlockByHash(hash: hash)
            
        // Token Queries
        case "getIdentityTokenBalance":
            let identityId = queryInputs["identityId"] ?? ""
            let tokenId = queryInputs["tokenId"] ?? ""
            return try await sdk.getIdentityTokenBalance(identityId: identityId, tokenId: tokenId)
            
        case "getIdentityTokenBalances":
            let identityId = queryInputs["identityId"] ?? ""
            return try await sdk.getIdentityTokenBalances(identityId: identityId)
            
        case "getTokenInfo":
            let tokenId = queryInputs["tokenId"] ?? ""
            return try await sdk.getTokenInfo(tokenId: tokenId)
            
        case "getTokenHolders":
            let tokenId = queryInputs["tokenId"] ?? ""
            let limitStr = queryInputs["limit"] ?? ""
            let offsetStr = queryInputs["offset"] ?? ""
            let limit = limitStr.isEmpty ? nil : UInt32(limitStr)
            let offset = offsetStr.isEmpty ? nil : UInt32(offsetStr)
            return try await sdk.getTokenHolders(tokenId: tokenId, limit: limit, offset: offset)
            
        case "getTotalTokenSupply":
            let tokenId = queryInputs["tokenId"] ?? ""
            return try await sdk.getTotalTokenSupply(tokenId: tokenId)
            
        // Group Queries
        case "getGroupMembers":
            let groupId = queryInputs["groupId"] ?? ""
            return try await sdk.getGroupMembers(groupId: groupId)
            
        case "getIdentityGroups":
            let identityId = queryInputs["identityId"] ?? ""
            return try await sdk.getIdentityGroups(identityId: identityId)
            
        case "getGroupInfo":
            let groupId = queryInputs["groupId"] ?? ""
            return try await sdk.getGroupInfo(groupId: groupId)
            
        case "checkGroupMembership":
            let groupId = queryInputs["groupId"] ?? ""
            let identityId = queryInputs["identityId"] ?? ""
            return try await sdk.checkGroupMembership(groupId: groupId, identityId: identityId)
            
        // System Queries
        case "getStatus":
            return try await sdk.getStatus()
            
        case "getTotalCreditsInPlatform":
            return try await sdk.getTotalCreditsInPlatform()
            
        default:
            throw SDKError.notImplemented("Query \(query.name) not implemented yet")
        }
    }
    
    private func formatResult(_ result: Any) -> String {
        if let data = try? JSONSerialization.data(withJSONObject: result, options: .prettyPrinted),
           let string = String(data: data, encoding: .utf8) {
            return string
        }
        return String(describing: result)
    }
    
    private func inputFields(for queryName: String) -> [QueryInput] {
        switch queryName {
        // Identity Queries
        case "getIdentity":
            return [QueryInput(name: "id", label: "Identity ID", required: true)]
            
        case "getIdentityKeys":
            return [QueryInput(name: "identityId", label: "Identity ID", required: true)]
            
        case "getIdentitiesContractKeys":
            return [
                QueryInput(name: "identitiesIds", label: "Identity IDs (comma-separated)", required: true),
                QueryInput(name: "contractId", label: "Contract ID", required: true),
                QueryInput(name: "documentTypeName", label: "Document Type", required: false)
            ]
            
        case "getIdentityNonce":
            return [QueryInput(name: "identityId", label: "Identity ID", required: true)]
            
        case "getIdentityContractNonce":
            return [
                QueryInput(name: "identityId", label: "Identity ID", required: true),
                QueryInput(name: "contractId", label: "Contract ID", required: true)
            ]
            
        case "getIdentityBalance":
            return [QueryInput(name: "id", label: "Identity ID", required: true)]
            
        case "getIdentitiesBalances":
            return [QueryInput(name: "identityIds", label: "Identity IDs (comma-separated)", required: true)]
            
        case "getIdentityBalanceAndRevision":
            return [QueryInput(name: "id", label: "Identity ID", required: true)]
            
        case "getIdentityByPublicKeyHash":
            return [QueryInput(name: "publicKeyHash", label: "Public Key Hash", required: true, placeholder: "e.g., b7e904ce25ed97594e72f7af0e66f298031c1754")]
            
        case "getIdentityByNonUniquePublicKeyHash":
            return [QueryInput(name: "publicKeyHash", label: "Public Key Hash", required: true, placeholder: "e.g., 518038dc858461bcee90478fd994bba8057b7531")]
            
        // Data Contract Queries
        case "getDataContract":
            return [QueryInput(name: "id", label: "Data Contract ID", required: true, placeholder: "e.g., GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec")]
            
        case "getDataContractHistory":
            return [
                QueryInput(name: "id", label: "Data Contract ID", required: true),
                QueryInput(name: "limit", label: "Limit", required: false),
                QueryInput(name: "offset", label: "Offset", required: false)
            ]
            
        case "getDataContracts":
            return [QueryInput(name: "ids", label: "Data Contract IDs (comma-separated)", required: true)]
            
        // Document Queries
        case "getDocuments":
            return [
                QueryInput(name: "dataContractId", label: "Data Contract ID", required: true),
                QueryInput(name: "documentType", label: "Document Type", required: true, placeholder: "e.g., domain"),
                QueryInput(name: "whereClause", label: "Where Clause (JSON)", required: false, placeholder: "[[\"field\", \"==\", \"value\"]]"),
                QueryInput(name: "orderBy", label: "Order By (JSON)", required: false, placeholder: "[[\"$createdAt\", \"desc\"]]"),
                QueryInput(name: "limit", label: "Limit", required: false)
            ]
            
        case "getDocument":
            return [
                QueryInput(name: "dataContractId", label: "Data Contract ID", required: true),
                QueryInput(name: "documentType", label: "Document Type", required: true),
                QueryInput(name: "documentId", label: "Document ID", required: true)
            ]
            
        // DPNS Queries
        case "getDpnsUsername":
            return [
                QueryInput(name: "identityId", label: "Identity ID", required: true),
                QueryInput(name: "limit", label: "Limit", required: false, placeholder: "Default: 10")
            ]
            
        case "dpnsCheckAvailability":
            return [QueryInput(name: "label", label: "Label (Username)", required: true)]
            
        case "dpnsResolve":
            return [QueryInput(name: "name", label: "Name", required: true)]
            
        // Voting & Contested Resources Queries
        case "getContestedResources":
            return [
                QueryInput(name: "resourceType", label: "Resource Type", required: false, placeholder: "e.g., dpns"),
                QueryInput(name: "limit", label: "Limit", required: false),
                QueryInput(name: "offset", label: "Offset", required: false)
            ]
            
        case "getContestedResourceVotes":
            return [QueryInput(name: "resourceId", label: "Resource ID", required: true)]
            
        case "getMasternodeVotes":
            return [QueryInput(name: "masternodeId", label: "Masternode ID", required: true)]
            
        case "getActiveProposals":
            return []
            
        case "getProposal":
            return [QueryInput(name: "proposalId", label: "Proposal ID", required: true)]
            
        // Protocol & Version Queries
        case "getProtocolVersion":
            return []
            
        case "getVersionUpgradeState":
            return []
            
        // Epoch & Block Queries
        case "getCurrentEpoch":
            return []
            
        case "getEpoch":
            return [QueryInput(name: "epochIndex", label: "Epoch Index", required: true)]
            
        case "getBestBlockHeight":
            return []
            
        case "getBlock":
            return [QueryInput(name: "height", label: "Block Height", required: true)]
            
        case "getBlockByHash":
            return [QueryInput(name: "hash", label: "Block Hash", required: true)]
            
        // Token Queries
        case "getIdentityTokenBalance":
            return [
                QueryInput(name: "identityId", label: "Identity ID", required: true),
                QueryInput(name: "tokenId", label: "Token ID", required: true)
            ]
            
        case "getIdentityTokenBalances":
            return [QueryInput(name: "identityId", label: "Identity ID", required: true)]
            
        case "getTokenInfo":
            return [QueryInput(name: "tokenId", label: "Token ID", required: true)]
            
        case "getTokenHolders":
            return [
                QueryInput(name: "tokenId", label: "Token ID", required: true),
                QueryInput(name: "limit", label: "Limit", required: false),
                QueryInput(name: "offset", label: "Offset", required: false)
            ]
            
        case "getTotalTokenSupply":
            return [QueryInput(name: "tokenId", label: "Token ID", required: true)]
            
        // Group Queries
        case "getGroupMembers":
            return [QueryInput(name: "groupId", label: "Group ID", required: true)]
            
        case "getIdentityGroups":
            return [QueryInput(name: "identityId", label: "Identity ID", required: true)]
            
        case "getGroupInfo":
            return [QueryInput(name: "groupId", label: "Group ID", required: true)]
            
        case "checkGroupMembership":
            return [
                QueryInput(name: "groupId", label: "Group ID", required: true),
                QueryInput(name: "identityId", label: "Identity ID", required: true)
            ]
            
        // System Queries
        case "getStatus":
            return []
            
        case "getTotalCreditsInPlatform":
            return []
            
        default:
            return []
        }
    }
}

struct QueryInput {
    let name: String
    let label: String
    let required: Bool
    let placeholder: String?
    
    init(name: String, label: String, required: Bool, placeholder: String? = nil) {
        self.name = name
        self.label = label
        self.required = required
        self.placeholder = placeholder
    }
}

