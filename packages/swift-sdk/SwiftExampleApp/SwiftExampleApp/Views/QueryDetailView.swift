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
                Button(action: {
                    print("🔵 QueryDetailView: Execute Query button tapped")
                    executeQuery()
                }) {
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
                .onAppear {
                    print("🔵 QueryDetailView: Button appeared, disabled: \(isLoading || !hasRequiredInputs()), hasRequiredInputs: \(hasRequiredInputs())")
                }
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
        .onAppear {
            print("🔵 QueryDetailView: View appeared for query: \(query.name)")
            print("🔵 QueryDetailView: appState.platformState.sdk is \(appState.platformState.sdk != nil ? "initialized" : "nil")")
        }
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
        print("🔵 QueryDetailView: executeQuery() called for query: \(query.name)")
        
        guard let sdk = appState.platformState.sdk else {
            print("❌ QueryDetailView: SDK not initialized")
            error = "SDK not initialized"
            return
        }
        
        print("🔵 QueryDetailView: SDK is initialized, preparing to execute query")
        print("🔵 QueryDetailView: Query inputs: \(queryInputs)")
        
        isLoading = true
        error = ""
        result = ""
        showResult = false
        
        Task {
            do {
                print("🔵 QueryDetailView: Calling performQuery...")
                let queryResult = try await performQuery(sdk: sdk)
                print("✅ QueryDetailView: performQuery returned successfully")
                print("🔵 QueryDetailView: Query result type: \(type(of: queryResult))")
                
                await MainActor.run {
                    result = formatResult(queryResult)
                    showResult = true
                    isLoading = false
                    print("✅ QueryDetailView: Result displayed, showResult: \(showResult)")
                }
            } catch let sdkError as SDKError {
                print("❌ QueryDetailView: SDK error occurred: \(sdkError)")
                await MainActor.run {
                    // Handle SDK errors with more detail
                    switch sdkError {
                    case .invalidParameter(let message):
                        self.error = "Invalid Parameter: \(message)"
                    case .invalidState(let message):
                        self.error = "Invalid State: \(message)"
                    case .networkError(let message):
                        self.error = "Network Error: \(message)"
                    case .serializationError(let message):
                        self.error = "Serialization Error: \(message)"
                    case .protocolError(let message):
                        self.error = "Protocol Error: \(message)"
                    case .cryptoError(let message):
                        self.error = "Crypto Error: \(message)"
                    case .notFound(let message):
                        self.error = "Not Found: \(message)"
                    case .timeout(let message):
                        self.error = "Timeout: \(message)"
                    case .notImplemented(let message):
                        self.error = "Not Implemented: \(message)"
                    case .internalError(let message):
                        self.error = "Internal Error: \(message)"
                    case .unknown(let message):
                        self.error = "Unknown Error: \(message)"
                    }
                    isLoading = false
                    print("❌ QueryDetailView: Error set to: \(self.error)")
                }
            } catch {
                print("❌ QueryDetailView: General error occurred: \(error)")
                await MainActor.run {
                    // For non-SDK errors, try to get more information
                    let nsError = error as NSError
                    var errorMessage = ""
                    
                    print("❌ QueryDetailView: NSError domain: \(nsError.domain), code: \(nsError.code)")
                    
                    // Try to get the most descriptive error message
                    if let failureReason = nsError.localizedFailureReason {
                        errorMessage = failureReason
                    } else if !nsError.localizedDescription.isEmpty && nsError.localizedDescription != "The operation couldn't be completed. (\(nsError.domain) error \(nsError.code).)" {
                        errorMessage = nsError.localizedDescription
                    } else {
                        errorMessage = "Error Domain: \(nsError.domain)\nError Code: \(nsError.code)"
                    }
                    
                    // Add user info if available
                    if !nsError.userInfo.isEmpty {
                        errorMessage += "\n\nDetails:"
                        for (key, value) in nsError.userInfo {
                            if let stringValue = value as? String {
                                errorMessage += "\n\(key): \(stringValue)"
                            } else if let debugDescription = (value as? CustomDebugStringConvertible)?.debugDescription {
                                errorMessage += "\n\(key): \(debugDescription)"
                            }
                        }
                    }
                    
                    self.error = errorMessage
                    isLoading = false
                    print("❌ QueryDetailView: Final error message: \(errorMessage)")
                }
            }
        }
    }
    
    private func performQuery(sdk: SDK) async throws -> Any {
        print("🔵 QueryDetailView: performQuery called with query name: \(query.name)")
        
        switch query.name {
        // Identity Queries
        case "getIdentity":
            let id = queryInputs["id"] ?? ""
            print("🔵 QueryDetailView: Executing getIdentity with ID: \(id)")
            return try await sdk.identityGet(identityId: id)
            
        case "getIdentityKeys":
            let identityId = queryInputs["identityId"] ?? ""
            let keyRequestType = queryInputs["keyRequestType"] ?? "all"
            let specificKeyIds = queryInputs["specificKeyIds"]?.split(separator: ",").map { String($0.trimmingCharacters(in: .whitespaces)) }
            let searchPurposeMap = queryInputs["searchPurposeMap"]
            let limitStr = queryInputs["limit"] ?? ""
            let offsetStr = queryInputs["offset"] ?? ""
            let limit = limitStr.isEmpty ? nil : UInt32(limitStr)
            let offset = offsetStr.isEmpty ? nil : UInt32(offsetStr)
            return try await sdk.identityGetKeys(
                identityId: identityId,
                keyRequestType: keyRequestType,
                specificKeyIds: specificKeyIds,
                searchPurposeMap: searchPurposeMap,
                limit: limit,
                offset: offset
            )
            
        case "getIdentitiesContractKeys":
            let identityIds = (queryInputs["identitiesIds"] ?? "").split(separator: ",").map { String($0.trimmingCharacters(in: .whitespaces)) }
            let contractId = queryInputs["contractId"] ?? ""
            let documentType = queryInputs["documentTypeName"]
            let purposes = queryInputs["purposes"]?.split(separator: ",").map { String($0.trimmingCharacters(in: .whitespaces)) }
            return try await sdk.identityGetContractKeys(
                identityIds: identityIds,
                contractId: contractId,
                documentType: documentType,
                purposes: purposes
            )
            
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
            let startAfter = queryInputs["startAfter"]
            return try await sdk.identityGetByNonUniquePublicKeyHash(publicKeyHash: publicKeyHash, startAfter: startAfter)
            
        // Data Contract Queries
        case "getDataContract":
            let id = queryInputs["id"] ?? ""
            return try await sdk.dataContractGet(id: id)
            
        case "getDataContractHistory":
            let id = queryInputs["id"] ?? ""
            let limitStr = queryInputs["limit"] ?? ""
            let offsetStr = queryInputs["offset"] ?? ""
            let startAtMsStr = queryInputs["startAtMs"] ?? ""
            let limit = limitStr.isEmpty ? nil : UInt32(limitStr)
            let offset = offsetStr.isEmpty ? nil : UInt32(offsetStr)
            let startAtMs = startAtMsStr.isEmpty ? nil : UInt64(startAtMsStr)
            return try await sdk.dataContractGetHistory(id: id, limit: limit, offset: offset, startAtMs: startAtMs)
            
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
            let startAfter = queryInputs["startAfter"]
            let startAt = queryInputs["startAt"]
            
            return try await sdk.documentList(
                dataContractId: contractId,
                documentType: documentType,
                whereClause: whereClause,
                orderByClause: orderBy,
                limit: limit,
                startAfter: startAfter,
                startAt: startAt
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
            
        case "dpnsSearch":
            let prefix = queryInputs["prefix"] ?? ""
            let limitStr = queryInputs["limit"] ?? ""
            let limit = limitStr.isEmpty ? nil : UInt32(limitStr)
            return try await sdk.dpnsSearch(prefix: prefix, limit: limit)
            
        // Voting & Contested Resources Queries
        case "getContestedResources":
            let documentTypeName = queryInputs["documentTypeName"] ?? ""
            let dataContractId = queryInputs["dataContractId"] ?? ""
            let indexName = queryInputs["indexName"] ?? ""
            let resultType = queryInputs["resultType"] ?? "documents"
            let allowIncludeLockedAndAbstainingVoteTally = queryInputs["allowIncludeLockedAndAbstainingVoteTally"] == "true"
            let startAtValue = queryInputs["startAtValue"]
            let limitStr = queryInputs["limit"] ?? ""
            let offsetStr = queryInputs["offset"] ?? ""
            let limit = limitStr.isEmpty ? nil : UInt32(limitStr)
            let offset = offsetStr.isEmpty ? nil : UInt32(offsetStr)
            let orderAscending = queryInputs["orderAscending"] == "true"
            return try await sdk.getContestedResources(
                documentTypeName: documentTypeName,
                dataContractId: dataContractId,
                indexName: indexName,
                resultType: resultType,
                allowIncludeLockedAndAbstainingVoteTally: allowIncludeLockedAndAbstainingVoteTally,
                startAtValue: startAtValue,
                limit: limit,
                offset: offset,
                orderAscending: orderAscending
            )
            
        case "getContestedResourceVoteState":
            let dataContractId = queryInputs["dataContractId"] ?? ""
            let documentTypeName = queryInputs["documentTypeName"] ?? ""
            let indexName = queryInputs["indexName"] ?? ""
            let resultType = queryInputs["resultType"] ?? "contenders"
            let allowIncludeLockedAndAbstainingVoteTally = queryInputs["allowIncludeLockedAndAbstainingVoteTally"] == "true"
            let startAtIdentifierInfo = queryInputs["startAtIdentifierInfo"]
            let countStr = queryInputs["count"] ?? ""
            let count = countStr.isEmpty ? nil : UInt32(countStr)
            let orderAscending = queryInputs["orderAscending"] == "true"
            return try await sdk.getContestedResourceVoteState(
                dataContractId: dataContractId,
                documentTypeName: documentTypeName,
                indexName: indexName,
                resultType: resultType,
                allowIncludeLockedAndAbstainingVoteTally: allowIncludeLockedAndAbstainingVoteTally,
                startAtIdentifierInfo: startAtIdentifierInfo,
                count: count,
                orderAscending: orderAscending
            )
            
        case "getContestedResourceVotersForIdentity":
            let dataContractId = queryInputs["dataContractId"] ?? ""
            let documentTypeName = queryInputs["documentTypeName"] ?? ""
            let indexName = queryInputs["indexName"] ?? ""
            let contestantId = queryInputs["contestantId"] ?? ""
            let startAtIdentifierInfo = queryInputs["startAtIdentifierInfo"]
            let countStr = queryInputs["count"] ?? ""
            let count = countStr.isEmpty ? nil : UInt32(countStr)
            let orderAscending = queryInputs["orderAscending"] == "true"
            return try await sdk.getContestedResourceVotersForIdentity(
                dataContractId: dataContractId,
                documentTypeName: documentTypeName,
                indexName: indexName,
                contestantId: contestantId,
                startAtIdentifierInfo: startAtIdentifierInfo,
                count: count,
                orderAscending: orderAscending
            )
            
        case "getContestedResourceIdentityVotes":
            let identityId = queryInputs["identityId"] ?? ""
            let limitStr = queryInputs["limit"] ?? ""
            let offsetStr = queryInputs["offset"] ?? ""
            let limit = limitStr.isEmpty ? nil : UInt32(limitStr)
            let offset = offsetStr.isEmpty ? nil : UInt32(offsetStr)
            let orderAscending = queryInputs["orderAscending"] == "true"
            return try await sdk.getContestedResourceIdentityVotes(
                identityId: identityId,
                limit: limit,
                offset: offset,
                orderAscending: orderAscending
            )
            
        case "getVotePollsByEndDate":
            let startTimeMsStr = queryInputs["startTimeMs"] ?? ""
            let endTimeMsStr = queryInputs["endTimeMs"] ?? ""
            let startTimeMs = startTimeMsStr.isEmpty ? nil : UInt64(startTimeMsStr)
            let endTimeMs = endTimeMsStr.isEmpty ? nil : UInt64(endTimeMsStr)
            let limitStr = queryInputs["limit"] ?? ""
            let offsetStr = queryInputs["offset"] ?? ""
            let limit = limitStr.isEmpty ? nil : UInt32(limitStr)
            let offset = offsetStr.isEmpty ? nil : UInt32(offsetStr)
            let orderAscending = queryInputs["orderAscending"] == "true"
            return try await sdk.getVotePollsByEndDate(
                startTimeMs: startTimeMs,
                endTimeMs: endTimeMs,
                limit: limit,
                offset: offset,
                orderAscending: orderAscending
            )
            
        // Protocol & Version Queries
        case "getProtocolVersionUpgradeState":
            return try await sdk.getProtocolVersionUpgradeState()
            
        case "getProtocolVersionUpgradeVoteStatus":
            let startProTxHash = queryInputs["startProTxHash"]
            let countStr = queryInputs["count"] ?? ""
            let count = countStr.isEmpty ? nil : UInt32(countStr)
            return try await sdk.getProtocolVersionUpgradeVoteStatus(startProTxHash: startProTxHash, count: count)
            
        // Epoch & Block Queries
        case "getEpochsInfo":
            let startEpochStr = queryInputs["startEpoch"] ?? ""
            let startEpoch = startEpochStr.isEmpty ? nil : UInt32(startEpochStr)
            let countStr = queryInputs["count"] ?? ""
            let count = countStr.isEmpty ? nil : UInt32(countStr)
            let ascending = queryInputs["ascending"] == "true"
            return try await sdk.getEpochsInfo(startEpoch: startEpoch, count: count, ascending: ascending)
            
        case "getCurrentEpoch":
            return try await sdk.getCurrentEpoch()
            
        case "getFinalizedEpochInfos":
            let startEpochStr = queryInputs["startEpoch"] ?? ""
            let startEpoch = startEpochStr.isEmpty ? nil : UInt32(startEpochStr)
            let countStr = queryInputs["count"] ?? ""
            let count = countStr.isEmpty ? nil : UInt32(countStr)
            let ascending = queryInputs["ascending"] == "true"
            return try await sdk.getFinalizedEpochInfos(startEpoch: startEpoch, count: count, ascending: ascending)
            
        case "getEvonodesProposedEpochBlocksByIds":
            let epochStr = queryInputs["epoch"] ?? ""
            let epoch = UInt32(epochStr) ?? 0
            let ids = (queryInputs["ids"] ?? "").split(separator: ",").map { String($0.trimmingCharacters(in: .whitespaces)) }
            return try await sdk.getEvonodesProposedEpochBlocksByIds(epoch: epoch, ids: ids)
            
        case "getEvonodesProposedEpochBlocksByRange":
            let epochStr = queryInputs["epoch"] ?? ""
            let epoch = UInt32(epochStr) ?? 0
            let limitStr = queryInputs["limit"] ?? ""
            let limit = limitStr.isEmpty ? nil : UInt32(limitStr)
            let startAfter = queryInputs["startAfter"]
            let orderAscending = queryInputs["orderAscending"] == "true"
            return try await sdk.getEvonodesProposedEpochBlocksByRange(
                epoch: epoch,
                limit: limit,
                startAfter: startAfter,
                orderAscending: orderAscending
            )
            
        // Token Queries
        case "getIdentitiesTokenBalances":
            let identityIds = (queryInputs["identityIds"] ?? "").split(separator: ",").map { String($0.trimmingCharacters(in: .whitespaces)) }
            let tokenId = queryInputs["tokenId"] ?? ""
            return try await sdk.getIdentitiesTokenBalances(identityIds: identityIds, tokenId: tokenId)
            
        case "getIdentityTokenInfos":
            let identityId = queryInputs["identityId"] ?? ""
            let tokenIds = queryInputs["tokenIds"]?.split(separator: ",").map { String($0.trimmingCharacters(in: .whitespaces)) }
            let limitStr = queryInputs["limit"] ?? ""
            let offsetStr = queryInputs["offset"] ?? ""
            let limit = limitStr.isEmpty ? nil : UInt32(limitStr)
            let offset = offsetStr.isEmpty ? nil : UInt32(offsetStr)
            return try await sdk.getIdentityTokenInfos(
                identityId: identityId,
                tokenIds: tokenIds,
                limit: limit,
                offset: offset
            )
            
        case "getIdentitiesTokenInfos":
            let identityIds = (queryInputs["identityIds"] ?? "").split(separator: ",").map { String($0.trimmingCharacters(in: .whitespaces)) }
            let tokenId = queryInputs["tokenId"] ?? ""
            return try await sdk.getIdentitiesTokenInfos(identityIds: identityIds, tokenId: tokenId)
            
        case "getTokenStatuses":
            let tokenIds = (queryInputs["tokenIds"] ?? "").split(separator: ",").map { String($0.trimmingCharacters(in: .whitespaces)) }
            return try await sdk.getTokenStatuses(tokenIds: tokenIds)
            
        case "getTokenDirectPurchasePrices":
            let tokenIds = (queryInputs["tokenIds"] ?? "").split(separator: ",").map { String($0.trimmingCharacters(in: .whitespaces)) }
            return try await sdk.getTokenDirectPurchasePrices(tokenIds: tokenIds)
            
        case "getTokenContractInfo":
            let tokenId = queryInputs["tokenId"] ?? ""
            return try await sdk.getTokenContractInfo(tokenId: tokenId)
            
        case "getTokenPerpetualDistributionLastClaim":
            let identityId = queryInputs["identityId"] ?? ""
            let tokenId = queryInputs["tokenId"] ?? ""
            return try await sdk.getTokenPerpetualDistributionLastClaim(identityId: identityId, tokenId: tokenId)
            
        case "getTokenTotalSupply":
            let tokenId = queryInputs["tokenId"] ?? ""
            return try await sdk.getTokenTotalSupply(tokenId: tokenId)
            
        // Group Queries
        case "getGroupInfo":
            let contractId = queryInputs["contractId"] ?? ""
            let groupContractPositionStr = queryInputs["groupContractPosition"] ?? ""
            let groupContractPosition = UInt32(groupContractPositionStr) ?? 0
            return try await sdk.getGroupInfo(contractId: contractId, groupContractPosition: groupContractPosition)
            
        case "getGroupInfos":
            let contractId = queryInputs["contractId"] ?? ""
            let startAtGroupContractPositionStr = queryInputs["startAtGroupContractPosition"] ?? ""
            let startAtGroupContractPosition = startAtGroupContractPositionStr.isEmpty ? nil : UInt32(startAtGroupContractPositionStr)
            let startGroupContractPositionIncluded = queryInputs["startGroupContractPositionIncluded"] == "true"
            let countStr = queryInputs["count"] ?? ""
            let count = countStr.isEmpty ? nil : UInt32(countStr)
            return try await sdk.getGroupInfos(
                contractId: contractId,
                startAtGroupContractPosition: startAtGroupContractPosition,
                startGroupContractPositionIncluded: startGroupContractPositionIncluded,
                count: count
            )
            
        case "getGroupActions":
            let contractId = queryInputs["contractId"] ?? ""
            let groupContractPositionStr = queryInputs["groupContractPosition"] ?? ""
            let groupContractPosition = UInt32(groupContractPositionStr) ?? 0
            let status = queryInputs["status"] ?? "ACTIVE"
            let startActionId = queryInputs["startActionId"]
            let startActionIdIncluded = queryInputs["startActionIdIncluded"] == "true"
            let countStr = queryInputs["count"] ?? ""
            let count = countStr.isEmpty ? nil : UInt32(countStr)
            return try await sdk.getGroupActions(
                contractId: contractId,
                groupContractPosition: groupContractPosition,
                status: status,
                startActionId: startActionId,
                startActionIdIncluded: startActionIdIncluded,
                count: count
            )
            
        case "getGroupActionSigners":
            let contractId = queryInputs["contractId"] ?? ""
            let groupContractPositionStr = queryInputs["groupContractPosition"] ?? ""
            let groupContractPosition = UInt32(groupContractPositionStr) ?? 0
            let status = queryInputs["status"] ?? "ACTIVE"
            let actionId = queryInputs["actionId"] ?? ""
            return try await sdk.getGroupActionSigners(
                contractId: contractId,
                groupContractPosition: groupContractPosition,
                status: status,
                actionId: actionId
            )
            
        // System Queries
        case "getStatus":
            return try await sdk.getStatus()
            
        case "getTotalCreditsInPlatform":
            return try await sdk.getTotalCreditsInPlatform()
            
        case "getCurrentQuorumsInfo":
            return try await sdk.getCurrentQuorumsInfo()
            
        case "getPrefundedSpecializedBalance":
            let id = queryInputs["id"] ?? ""
            return try await sdk.getPrefundedSpecializedBalance(id: id)
            
        case "runAllQueries":
            // This is handled by DiagnosticsView - should not reach here
            throw SDKError.notImplemented("Use DiagnosticsView for running all queries")
            
        default:
            throw SDKError.notImplemented("Query \(query.name) not implemented yet")
        }
    }
    
    private func formatResult(_ result: Any) -> String {
        // Handle primitive types that can't be directly serialized as JSON
        if result is String || result is NSNumber || result is Bool || 
           result is Int || result is Int32 || result is Int64 || 
           result is UInt || result is UInt32 || result is UInt64 ||
           result is Float || result is Double {
            // For primitive types, wrap in an object for display
            let wrappedResult = ["value": result]
            if let data = try? JSONSerialization.data(withJSONObject: wrappedResult, options: .prettyPrinted),
               let string = String(data: data, encoding: .utf8) {
                return string
            }
        }
        
        // Try to serialize as JSON for objects and arrays
        if let data = try? JSONSerialization.data(withJSONObject: result, options: .prettyPrinted),
           let string = String(data: data, encoding: .utf8) {
            return string
        }
        
        // Fallback to string description
        return String(describing: result)
    }
    
    private func inputFields(for queryName: String) -> [QueryInput] {
        switch queryName {
        // Identity Queries
        case "getIdentity":
            return [QueryInput(name: "id", label: "Identity ID", required: true)]
            
        case "getIdentityKeys":
            return [
                QueryInput(name: "identityId", label: "Identity ID", required: true),
                QueryInput(name: "keyRequestType", label: "Key Request Type", required: true, placeholder: "all, specific, or search"),
                QueryInput(name: "specificKeyIds", label: "Key IDs (comma-separated)", required: false, placeholder: "Required if type is 'specific'"),
                QueryInput(name: "searchPurposeMap", label: "Search Purpose Map (JSON)", required: false, placeholder: "{\"0\": {\"0\": \"current\"}, \"1\": {\"0\": \"all\"}}"),
                QueryInput(name: "limit", label: "Limit", required: false),
                QueryInput(name: "offset", label: "Offset", required: false)
            ]
            
        case "getIdentitiesContractKeys":
            return [
                QueryInput(name: "identitiesIds", label: "Identity IDs (comma-separated)", required: true),
                QueryInput(name: "contractId", label: "Contract ID", required: true),
                QueryInput(name: "documentTypeName", label: "Document Type Name", required: false),
                QueryInput(name: "purposes", label: "Key Purposes (comma-separated)", required: false, placeholder: "0=Auth, 1=Encryption, 2=Decryption, 3=Transfer, 5=Voting")
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
            return [
                QueryInput(name: "publicKeyHash", label: "Public Key Hash", required: true, placeholder: "e.g., 518038dc858461bcee90478fd994bba8057b7531"),
                QueryInput(name: "startAfter", label: "Start After (Identity ID)", required: false, placeholder: "For pagination")
            ]
            
        // Data Contract Queries
        case "getDataContract":
            return [QueryInput(name: "id", label: "Data Contract ID", required: true, placeholder: "e.g., GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec")]
            
        case "getDataContractHistory":
            return [
                QueryInput(name: "id", label: "Data Contract ID", required: true),
                QueryInput(name: "limit", label: "Limit", required: false),
                QueryInput(name: "offset", label: "Offset", required: false),
                QueryInput(name: "startAtMs", label: "Start At (milliseconds)", required: false, placeholder: "Start from specific timestamp")
            ]
            
        case "getDataContracts":
            return [QueryInput(name: "ids", label: "Data Contract IDs (comma-separated)", required: true)]
            
        // Document Queries
        case "getDocuments":
            return [
                QueryInput(name: "dataContractId", label: "Data Contract ID", required: true),
                QueryInput(name: "documentType", label: "Document Type", required: true, placeholder: "e.g., domain"),
                QueryInput(name: "whereClause", label: "Where Clause (JSON)", required: false, placeholder: "[{\"field\": \"field\", \"operator\": \"=\", \"value\": \"value\"}]"),
                QueryInput(name: "orderBy", label: "Order By (JSON)", required: false, placeholder: "[{\"field\": \"$createdAt\", \"ascending\": false}]"),
                QueryInput(name: "limit", label: "Limit", required: false),
                QueryInput(name: "startAfter", label: "Start After (Document ID)", required: false, placeholder: "For pagination"),
                QueryInput(name: "startAt", label: "Start At (Document ID)", required: false, placeholder: "For pagination (inclusive)")
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
            
        case "dpnsSearch":
            return [
                QueryInput(name: "prefix", label: "Name Prefix", required: true, placeholder: "e.g., ali"),
                QueryInput(name: "limit", label: "Limit", required: false, placeholder: "Default: 10")
            ]
            
        // Voting & Contested Resources Queries
        case "getContestedResources":
            return [
                QueryInput(name: "documentTypeName", label: "Document Type Name", required: true),
                QueryInput(name: "dataContractId", label: "Data Contract ID", required: true),
                QueryInput(name: "indexName", label: "Index Name", required: true),
                QueryInput(name: "resultType", label: "Result Type", required: true, placeholder: "documents, vote_tally, or document_with_vote_tally"),
                QueryInput(name: "allowIncludeLockedAndAbstainingVoteTally", label: "Include Locked and Abstaining", required: false, placeholder: "true/false"),
                QueryInput(name: "startAtValue", label: "Start At Value (hex bytes)", required: false),
                QueryInput(name: "limit", label: "Limit", required: false),
                QueryInput(name: "offset", label: "Offset", required: false),
                QueryInput(name: "orderAscending", label: "Order Ascending", required: false, placeholder: "true/false")
            ]
            
        case "getContestedResourceVoteState":
            return [
                QueryInput(name: "dataContractId", label: "Data Contract ID", required: true),
                QueryInput(name: "documentTypeName", label: "Document Type Name", required: true),
                QueryInput(name: "indexName", label: "Index Name", required: true),
                QueryInput(name: "resultType", label: "Result Type", required: true, placeholder: "contenders, abstainers, or locked"),
                QueryInput(name: "allowIncludeLockedAndAbstainingVoteTally", label: "Include Locked and Abstaining", required: false, placeholder: "true/false"),
                QueryInput(name: "startAtIdentifierInfo", label: "Start At Identifier Info (JSON)", required: false),
                QueryInput(name: "count", label: "Count", required: false),
                QueryInput(name: "orderAscending", label: "Order Ascending", required: false, placeholder: "true/false")
            ]
            
        case "getContestedResourceVotersForIdentity":
            return [
                QueryInput(name: "dataContractId", label: "Data Contract ID", required: true),
                QueryInput(name: "documentTypeName", label: "Document Type Name", required: true),
                QueryInput(name: "indexName", label: "Index Name", required: true),
                QueryInput(name: "contestantId", label: "Contestant ID", required: true),
                QueryInput(name: "startAtIdentifierInfo", label: "Start At Identifier Info (JSON)", required: false),
                QueryInput(name: "count", label: "Count", required: false),
                QueryInput(name: "orderAscending", label: "Order Ascending", required: false, placeholder: "true/false")
            ]
            
        case "getContestedResourceIdentityVotes":
            return [
                QueryInput(name: "identityId", label: "Identity ID", required: true),
                QueryInput(name: "limit", label: "Limit", required: false),
                QueryInput(name: "offset", label: "Offset", required: false),
                QueryInput(name: "orderAscending", label: "Order Ascending", required: false, placeholder: "true/false")
            ]
            
        case "getVotePollsByEndDate":
            return [
                QueryInput(name: "startTimeMs", label: "Start Time (ms)", required: false),
                QueryInput(name: "endTimeMs", label: "End Time (ms)", required: false),
                QueryInput(name: "limit", label: "Limit", required: false),
                QueryInput(name: "offset", label: "Offset", required: false),
                QueryInput(name: "orderAscending", label: "Ascending Order", required: false, placeholder: "true/false")
            ]
            
        // Protocol & Version Queries
        case "getProtocolVersionUpgradeState":
            return []
            
        case "getProtocolVersionUpgradeVoteStatus":
            return [
                QueryInput(name: "startProTxHash", label: "Start ProTx Hash", required: false, placeholder: "Leave empty to start from beginning"),
                QueryInput(name: "count", label: "Count", required: false, placeholder: "Default: 100")
            ]
            
        // Epoch & Block Queries
        case "getCurrentEpoch":
            return []
            
        case "getEpochsInfo":
            return [
                QueryInput(name: "startEpoch", label: "Start Epoch", required: false),
                QueryInput(name: "count", label: "Count", required: false),
                QueryInput(name: "ascending", label: "Ascending Order", required: false, placeholder: "true/false")
            ]
            
        case "getFinalizedEpochInfos":
            return [
                QueryInput(name: "startEpoch", label: "Start Epoch", required: false),
                QueryInput(name: "count", label: "Count", required: false),
                QueryInput(name: "ascending", label: "Ascending Order", required: false, placeholder: "true/false")
            ]
            
        case "getEvonodesProposedEpochBlocksByIds":
            return [
                QueryInput(name: "epoch", label: "Epoch", required: true),
                QueryInput(name: "ids", label: "Evonode IDs (comma-separated)", required: true)
            ]
            
        case "getEvonodesProposedEpochBlocksByRange":
            return [
                QueryInput(name: "epoch", label: "Epoch", required: true),
                QueryInput(name: "limit", label: "Limit", required: false),
                QueryInput(name: "startAfter", label: "Start After (Evonode ID)", required: false),
                QueryInput(name: "orderAscending", label: "Order Ascending", required: false, placeholder: "true/false")
            ]
            
        // Token Queries
        case "getIdentitiesTokenBalances":
            return [
                QueryInput(name: "identityIds", label: "Identity IDs (comma-separated)", required: true),
                QueryInput(name: "tokenId", label: "Token ID", required: true)
            ]
            
        case "getIdentityTokenInfos":
            return [
                QueryInput(name: "identityId", label: "Identity ID", required: true),
                QueryInput(name: "tokenIds", label: "Token IDs (comma-separated)", required: false),
                QueryInput(name: "limit", label: "Limit", required: false),
                QueryInput(name: "offset", label: "Offset", required: false)
            ]
            
        case "getIdentitiesTokenInfos":
            return [
                QueryInput(name: "identityIds", label: "Identity IDs (comma-separated)", required: true),
                QueryInput(name: "tokenId", label: "Token ID", required: true)
            ]
            
        case "getTokenStatuses":
            return [
                QueryInput(name: "tokenIds", label: "Token IDs (comma-separated)", required: true)
            ]
            
        case "getTokenDirectPurchasePrices":
            return [
                QueryInput(name: "tokenIds", label: "Token IDs (comma-separated)", required: true)
            ]
            
        case "getTokenContractInfo":
            return [
                QueryInput(name: "dataContractId", label: "Token ID", required: true)
            ]
            
        case "getTokenPerpetualDistributionLastClaim":
            return [
                QueryInput(name: "identityId", label: "Identity ID", required: true),
                QueryInput(name: "tokenId", label: "Token ID", required: true)
            ]
            
        case "getTokenTotalSupply":
            return [
                QueryInput(name: "tokenId", label: "Token ID", required: true)
            ]
            
        // Group Queries
        case "getGroupInfo":
            return [
                QueryInput(name: "contractId", label: "Contract ID", required: true),
                QueryInput(name: "groupContractPosition", label: "Group Contract Position", required: true)
            ]
            
        case "getGroupInfos":
            return [
                QueryInput(name: "contractId", label: "Contract ID", required: true),
                QueryInput(name: "startAtGroupContractPosition", label: "Start at Position", required: false),
                QueryInput(name: "startGroupContractPositionIncluded", label: "Include Start Position", required: false, placeholder: "true/false"),
                QueryInput(name: "count", label: "Count", required: false)
            ]
            
        case "getGroupActions":
            return [
                QueryInput(name: "contractId", label: "Contract ID", required: true),
                QueryInput(name: "groupContractPosition", label: "Group Contract Position", required: true),
                QueryInput(name: "status", label: "Status", required: true, placeholder: "ACTIVE or CLOSED"),
                QueryInput(name: "startActionId", label: "Start Action ID", required: false),
                QueryInput(name: "startActionIdIncluded", label: "Include Start Action", required: false, placeholder: "true/false"),
                QueryInput(name: "count", label: "Count", required: false)
            ]
            
        case "getGroupActionSigners":
            return [
                QueryInput(name: "contractId", label: "Contract ID", required: true),
                QueryInput(name: "groupContractPosition", label: "Group Contract Position", required: true),
                QueryInput(name: "status", label: "Status", required: true, placeholder: "ACTIVE or CLOSED"),
                QueryInput(name: "actionId", label: "Action ID", required: true)
            ]
            
        // System Queries
        case "getStatus":
            return []
            
        case "getTotalCreditsInPlatform":
            return []
            
        case "getCurrentQuorumsInfo":
            return []
            
        case "getPrefundedSpecializedBalance":
            return [
                QueryInput(name: "id", label: "ID", required: true, placeholder: "Base58 encoded ID")
            ]
            
        case "runAllQueries":
            // No inputs needed - it uses predefined test data
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

