import SwiftUI
import SwiftDashSDK

struct QueryDetailView: View {
    let query: QueryDefinition
    @EnvironmentObject var appState: AppState
    @State private var queryInputs: [String: String] = [:]
    @State private var isLoading = false
    @State private var result: String = ""
    @State private var error: String = ""
    @State private var showResult = false
    /// The token of each Execute; editing an input of a route that ties its
    /// result to its inputs (``resultFollowsInputs``) invalidates it, so a
    /// running read's completion, which answers the inputs it was submitted
    /// with, is dropped.
    @State private var submissions = InputBoundSubmissions()

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
            print("🔵 QueryDetailView: appState.sdk is \(appState.sdk != nil ? "initialized" : "nil")")
        }
    }

    private func binding(for key: String) -> Binding<String> {
        Binding(
            get: { queryInputs[key] ?? "" },
            set: { newValue in
                guard queryInputs[key] ?? "" != newValue else { return }
                queryInputs[key] = newValue
                if resultFollowsInputs {
                    // The result shown, and the read running, answer the old inputs
                    submissions.invalidate()
                    result = ""
                    error = ""
                    showResult = false
                    isLoading = false
                }
            }
        )
    }

    /// Whether the result shown names the inputs it answers and is cleared
    /// when they change: the contract moderation and fee pot reads, whose
    /// answers name neither the contract nor most of what was asked.
    private var resultFollowsInputs: Bool {
        Self.inputBoundQueries.contains(query.name)
    }

    private static let inputBoundQueries: Set<String> = [
        "getContractModerationStatus",
        "getContractModerationEntries",
        "getContractDocumentRemovals",
        "getContractTeamActions",
        "getContractTeamActionSigners",
        "getContractModerationActionCounts",
        "getContractFeePots",
    ]

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

        guard let sdk = appState.sdk else {
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
        // A moderation read is built from its inputs as they are now, so an
        // edit made before the task runs cannot change what it sends; its
        // result echoes the same snapshot
        let submission = submissions.submit(resultFollowsInputs ? submittedInputs() : [:])

        Task {
            do {
                print("🔵 QueryDetailView: Calling performQuery...")
                let queryResult: Any
                if resultFollowsInputs {
                    queryResult = try await performModerationQuery(sdk: sdk, inputs: submission.inputs)
                } else {
                    queryResult = try await performQuery(sdk: sdk)
                }
                print("✅ QueryDetailView: performQuery returned successfully")
                print("🔵 QueryDetailView: Query result type: \(type(of: queryResult))")

                await MainActor.run {
                    guard submissions.isCurrent(submission) else { return }
                    result = formatResult(queryResult)
                    showResult = true
                    isLoading = false
                    print("✅ QueryDetailView: Result displayed, showResult: \(showResult)")
                }
            } catch let sdkError as SDKError {
                print("❌ QueryDetailView: SDK error occurred: \(sdkError)")
                await MainActor.run {
                    guard submissions.isCurrent(submission) else { return }
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
                    case .stateTransitionResultUnconfirmed:
                        self.error = sdkError.localizedDescription
                    case .consensusRejection(let consensus, let message):
                        self.error = "Rejected by Platform (code \(consensus.code)): \(message)"
                    case .unknown(let message):
                        self.error = "Unknown Error: \(message)"
                    }
                    isLoading = false
                    print("❌ QueryDetailView: Error set to: \(self.error)")
                }
            } catch {
                print("❌ QueryDetailView: General error occurred: \(error)")
                await MainActor.run {
                    guard submissions.isCurrent(submission) else { return }
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

        case "getDataContractsByRange":
            let limitStr = queryInputs["limit"] ?? ""
            let limit = limitStr.isEmpty ? nil : UInt32(limitStr)
            let startAfter = (queryInputs["startAfter"] ?? "").isEmpty ? nil : queryInputs["startAfter"]
            let startAt = (queryInputs["startAt"] ?? "").isEmpty ? nil : queryInputs["startAt"]
            let idsOnly = Bool(queryInputs["idsOnly"] ?? "") ?? false
            return try await sdk.getDataContractsByRange(limit: limit, startAfter: startAfter, startAt: startAt, idsOnly: idsOnly)

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

        case "getDocumentHistory":
            let contractId = queryInputs["dataContractId"] ?? ""
            let documentType = queryInputs["documentType"] ?? ""
            let documentId = queryInputs["documentId"] ?? ""
            // Every non-empty number must parse, and exactly one filter
            // group may be given, so a typo or a half cursor is refused
            // rather than silently read as the first page.
            func number(_ name: String) throws -> UInt64? {
                let raw = (queryInputs[name] ?? "").trimmingCharacters(in: .whitespaces)
                if raw.isEmpty { return nil }
                guard let value = UInt64(raw) else {
                    throw SDKError.invalidParameter("\(name) must be a non-negative integer")
                }
                return value
            }
            let limit = try number("limit").map { UInt32(clamping: $0) }
            let startAtMs = try number("startAtMs")
            let startAfterTimeMs = try number("startAfterTimeMs")
            let startAfterRevision = try number("startAfterRevision")
            let startAtRevision = try number("startAtRevision")
            let revision = try number("revision")
            if (startAfterTimeMs == nil) != (startAfterRevision == nil) {
                throw SDKError.invalidParameter("Start After needs both its time and its revision")
            }
            let groups = [startAtMs != nil, startAfterTimeMs != nil, startAtRevision != nil, revision != nil]
            if groups.filter({ $0 }).count > 1 {
                throw SDKError.invalidParameter("Give only one of Start At, Start After, Start At Revision, or Revision")
            }
            let filter: DocumentHistoryFilter
            if let revision {
                filter = .revision(revision)
            } else if let startAtRevision {
                filter = .startAtRevision(startAtRevision)
            } else if let timeMs = startAfterTimeMs, let rev = startAfterRevision {
                filter = .startAfter(timeMs: timeMs, revision: rev)
            } else {
                filter = .startAtTime(ms: startAtMs ?? 0)
            }
            return try await sdk.documentGetHistory(
                dataContractId: contractId,
                documentType: documentType,
                documentId: documentId,
                filter: filter,
                limit: limit
            )

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

        // Contested DPNS Queries
        case "getContestedDpnsNames":
            let limitStr = queryInputs["limit"] ?? ""
            let limit = limitStr.isEmpty ? 100 : (UInt32(limitStr) ?? 100)
            let startAfterInput = (queryInputs["startAfter"] ?? "").trimmingCharacters(in: .whitespaces)
            let startAfter = startAfterInput.isEmpty ? nil : startAfterInput

            return try await sdk.dpnsGetAllContestedUsernames(limit: limit, startAfter: startAfter)

        case "getContestedDpnsNameVoteState":
            let name = queryInputs["name"] ?? ""
            guard !name.isEmpty else {
                throw SDKError.internalError("DPNS name is required")
            }
            // 0 reads every contender (the SDK pages them)
            return try await sdk.dpnsGetContestedVoteState(
                name: name.trimmingCharacters(in: .whitespaces), limit: 0)

        case "getContestedDpnsNameVotersForIdentity":
            let name = queryInputs["name"] ?? ""
            let identityId = queryInputs["identityId"] ?? ""
            guard !name.isEmpty else {
                throw SDKError.internalError("DPNS name is required")
            }
            guard !identityId.isEmpty else {
                throw SDKError.internalError("Identity ID is required")
            }

            // 0 reads every voter (the SDK pages them)
            return try await sdk.dpnsGetContestedVotersForIdentity(
                name: name.trimmingCharacters(in: .whitespaces),
                contestantId: identityId.trimmingCharacters(in: .whitespaces),
                limit: 0
            )

        case "getContestedDpnsNameIdentityVotes":
            let identityId = queryInputs["identityId"] ?? ""
            let limitStr = queryInputs["limit"] ?? ""
            let limit = limitStr.isEmpty ? 100 : (UInt32(limitStr) ?? 100)
            let orderAscending = queryInputs["orderAscending"]?.lowercased() == "true"

            guard !identityId.isEmpty else {
                throw SDKError.internalError("Identity ID is required")
            }

            // Query all contested resource votes by this identity, filtered for DPNS
            let dpnsContractId = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"
            let votes = try await sdk.getContestedResourceIdentityVotes(
                identityId: identityId,
                limit: limit,
                offset: 0,
                orderAscending: orderAscending
            )
            // Filter results to only show DPNS-related votes
            let dpnsVotes = votes.filter { vote in
                if let contractId = vote["contractId"] as? String {
                    return contractId == dpnsContractId
                }
                return false
            }
            return dpnsVotes

        case "getDpnsVotePollsByEndDate":
            let startDateStr = queryInputs["startDate"] ?? ""
            let endDateStr = queryInputs["endDate"] ?? ""
            let limitStr = queryInputs["limit"] ?? ""
            let limit = limitStr.isEmpty ? 100 : (UInt32(limitStr) ?? 100)

            // Parse dates if provided
            let dateFormatter = ISO8601DateFormatter()
            let startTimestamp: UInt64? = startDateStr.isEmpty ? nil :
                (dateFormatter.date(from: startDateStr)?.timeIntervalSince1970).map { UInt64($0 * 1000) }
            let endTimestamp: UInt64? = endDateStr.isEmpty ? nil :
                (dateFormatter.date(from: endDateStr)?.timeIntervalSince1970).map { UInt64($0 * 1000) }

            let polls = try await sdk.getVotePollsByEndDate(
                startTimeMs: startTimestamp,
                endTimeMs: endTimestamp,
                limit: limit,
                offset: 0,
                orderAscending: true
            )

            // The listing is grouped by end date:
            // `[{"timestamp": <end date ms>, "vote_polls": [<vote poll>, ...]}]`,
            // each poll its canonical JSON tagged with `$type`. Flatten the
            // groups, keep the contested polls of the DPNS contract (yes/no
            // polls carry no `contractId`), and carry each group's end date
            // onto its polls so it is not lost with the grouping.
            let dpnsPolls = polls.flatMap { group -> [[String: Any]] in
                let votePolls = group["vote_polls"] as? [[String: Any]] ?? []
                return votePolls.compactMap { poll in
                    guard poll["$type"] as? String == "contestedDocumentResourceVotePoll",
                          poll["contractId"] as? String == DPNSVotePoll.contractId else {
                        return nil
                    }
                    var dpnsPoll = poll
                    dpnsPoll["timestamp"] = group["timestamp"]
                    return dpnsPoll
                }
            }
            return dpnsPolls

        // Voting & Contested Resources Queries
        case "getContestedResources":
            let documentTypeName = queryInputs["documentTypeName"] ?? ""
            let dataContractId = queryInputs["dataContractId"] ?? ""
            let indexName = queryInputs["indexName"] ?? ""
            let startIndexValues = try parseIndexValues(queryInputs["startIndexValues"])
            let endIndexValues = try parseIndexValues(queryInputs["endIndexValues"])
            let limitStr = queryInputs["limit"] ?? ""
            let limit = limitStr.isEmpty ? nil : UInt32(limitStr)
            let orderAscending = queryInputs["orderAscending"] == "true"
            return try await sdk.getContestedResources(
                documentTypeName: documentTypeName,
                dataContractId: dataContractId,
                indexName: indexName,
                startIndexValues: startIndexValues,
                endIndexValues: endIndexValues,
                limit: limit,
                orderAscending: orderAscending
            )

        case "getContestedResourceVoteState":
            let dataContractId = queryInputs["dataContractId"] ?? ""
            let documentTypeName = queryInputs["documentTypeName"] ?? ""
            let indexName = queryInputs["indexName"] ?? ""
            let indexValues = try parseIndexValues(queryInputs["indexValues"]) ?? []
            let resultTypeInput = (queryInputs["resultType"] ?? "").trimmingCharacters(in: .whitespaces)
            let resultType = resultTypeInput.isEmpty ? "documents_and_vote_tally" : resultTypeInput
            let allowIncludeLockedAndAbstainingVoteTally = queryInputs["allowIncludeLockedAndAbstainingVoteTally"] == "true"
            let countStr = queryInputs["count"] ?? ""
            let count = countStr.isEmpty ? nil : UInt32(countStr)
            return try await sdk.getContestedResourceVoteState(
                dataContractId: dataContractId,
                documentTypeName: documentTypeName,
                indexName: indexName,
                indexValues: indexValues,
                resultType: resultType,
                allowIncludeLockedAndAbstainingVoteTally: allowIncludeLockedAndAbstainingVoteTally,
                count: count
            )

        case "getContestedResourceVotersForIdentity":
            let dataContractId = queryInputs["dataContractId"] ?? ""
            let documentTypeName = queryInputs["documentTypeName"] ?? ""
            let indexName = queryInputs["indexName"] ?? ""
            let indexValues = try parseIndexValues(queryInputs["indexValues"]) ?? []
            let contestantId = queryInputs["contestantId"] ?? ""
            let countStr = queryInputs["count"] ?? ""
            let count = countStr.isEmpty ? nil : UInt32(countStr)
            let orderAscending = queryInputs["orderAscending"] == "true"
            return try await sdk.getContestedResourceVotersForIdentity(
                dataContractId: dataContractId,
                documentTypeName: documentTypeName,
                indexName: indexName,
                indexValues: indexValues,
                contestantId: contestantId,
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
        case "getIdentityTokenBalances":
            let identityId = queryInputs["identityId"] ?? ""
            let tokenIds = (queryInputs["tokenIds"] ?? "").split(separator: ",").map { String($0.trimmingCharacters(in: .whitespaces)) }
            return try await sdk.getIdentityTokenBalances(identityId: identityId, tokenIds: tokenIds)

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

        case "getContractGroupInfo":
            let contractGroupId = queryInputs["contractGroupId"] ?? ""
            return try await sdk.getContractGroupInfo(contractGroupId: contractGroupId)

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
        // JSON already written for display
        if let json = result as? PreformattedJSON {
            return json.text
        }
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

    /// Parse an index-values input: a JSON string array when it starts with
    /// "[", otherwise comma-separated values, trimmed. Values pass through
    /// unchanged: the FFI reads a leading backslash as text with that
    /// backslash removed, else a "0x" prefix as hex bytes, else text. So a
    /// text value starting with "0x" or a backslash is typed with one more
    /// backslash in front (the DPNS label oxford is `\0xf0rd`). Empty input
    /// is nil.
    /// A comma-separated list refuses an empty value (a stray or trailing
    /// comma); an empty string can only be given in a JSON array.
    private func parseIndexValues(_ input: String?) throws -> [String]? {
        let trimmed = (input ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty { return nil }
        if trimmed.hasPrefix("[") {
            guard let data = trimmed.data(using: .utf8),
                  let values = try? JSONSerialization.jsonObject(with: data) as? [String] else {
                throw SDKError.invalidParameter("Index values must be a JSON array of strings")
            }
            return values
        }
        let values = trimmed
            .split(separator: ",", omittingEmptySubsequences: false)
            .map { $0.trimmingCharacters(in: .whitespaces) }
        if let emptyIndex = values.firstIndex(where: { $0.isEmpty }) {
            throw SDKError.invalidParameter(
                "Index values: empty value at position \(emptyIndex + 1); use a JSON array for an empty string"
            )
        }
        return values
    }

    // MARK: - Contract moderation reads

    /// Runs one of the contract moderation and fee pot reads (protocol
    /// version 14) from `inputs`, the snapshot ``submittedInputs()`` took at
    /// Execute, and nothing else: the form may have changed since. The result
    /// names the same inputs.
    private func performModerationQuery(sdk: SDK, inputs: [String: String]) async throws -> Any {
        let contractId = inputs["contractId"] ?? ""
        switch query.name {
        case "getContractModerationStatus":
            let lists = try inputs["lists"].map { try parseModerationLists($0) }
            return try moderationResult(inputs, await sdk.contractModerationStatus(
                contractId: contractId,
                identityId: inputs["identityId"] ?? "",
                lists: lists
            ))

        case "getContractModerationEntries":
            return try moderationResult(inputs, await sdk.contractModerationEntries(
                contractId: contractId,
                list: try parseModerationList(inputs["list"] ?? ""),
                startAfter: inputs["startAfter"],
                limit: try moderationLimit(inputs)
            ))

        case "getContractDocumentRemovals":
            let documentTypeName = inputs["documentTypeName"] ?? ""
            // Two reads: the records of the ids named, or one page of them all.
            if let documentIds = inputs["documentIds"] {
                if inputs["startAfter"] != nil || inputs["limit"] != nil {
                    throw SDKError.invalidParameter("Give either Document IDs or Start After and Limit")
                }
                return try moderationResult(inputs, await sdk.contractDocumentRemovals(
                    contractId: contractId,
                    documentTypeName: documentTypeName,
                    documentIds: documentIds.split(separator: ",").map { $0.trimmingCharacters(in: .whitespaces) }
                ))
            }
            return try moderationResult(inputs, await sdk.contractDocumentRemovals(
                contractId: contractId,
                documentTypeName: documentTypeName,
                startAfter: inputs["startAfter"],
                limit: try moderationLimit(inputs)
            ))

        case "getContractTeamActions":
            return try moderationResult(inputs, await sdk.contractTeamActions(
                contractId: contractId,
                status: try parseTeamActionStatus(inputs["status"] ?? ""),
                startAtActionId: inputs["startAtActionId"],
                startAtActionIdIncluded: try parseStrictBool(
                    inputs["startAtActionIdIncluded"], field: "Include Start Action"),
                limit: try moderationLimit(inputs)
            ))

        case "getContractTeamActionSigners":
            return try moderationResult(inputs, await sdk.contractTeamActionSigners(
                contractId: contractId,
                status: try parseTeamActionStatus(inputs["status"] ?? ""),
                actionId: inputs["actionId"] ?? ""
            ))

        case "getContractModerationActionCounts":
            return try moderationResult(inputs, await sdk.contractModerationActionCounts(
                contractId: contractId
            ))

        case "getContractFeePots":
            return try moderationResult(inputs, await sdk.contractFeePots(contractId: contractId))

        default:
            throw SDKError.notImplemented("Query \(query.name) is not a moderation read")
        }
    }

    /// The trimmed form input `name`, `nil` when empty.
    private func moderationInput(_ name: String) -> String? {
        let value = (queryInputs[name] ?? "").trimmingCharacters(in: .whitespaces)
        return value.isEmpty ? nil : value
    }

    /// The `limit` input of `inputs`, `nil` when absent (the SDK's default
    /// page).
    private func moderationLimit(_ inputs: [String: String]) throws -> UInt32? {
        guard let raw = inputs["limit"] else { return nil }
        guard let limit = UInt32(raw) else {
            throw SDKError.invalidParameter("Limit must be a non-negative integer")
        }
        return limit
    }

    /// A yes/no input read strictly: absent is `false`, and only `true` or
    /// `false` (any case) is accepted, so a typo is shown as an input error
    /// before anything is sent rather than read as `false`.
    private func parseStrictBool(_ raw: String?, field: String) throws -> Bool {
        guard let raw else { return false }
        switch raw.lowercased() {
        case "true": return true
        case "false": return false
        default:
            throw SDKError.invalidParameter("\(field) must be true or false, got '\(raw)'")
        }
    }

    private func parseModerationList(_ raw: String) throws -> ContractModerationList {
        let name = raw.trimmingCharacters(in: .whitespaces).lowercased()
        guard let list = ContractModerationList(rawValue: name) else {
            throw SDKError.invalidParameter("List must be banlist, suspensions or warnings, got '\(raw)'")
        }
        return list
    }

    private func parseModerationLists(_ raw: String) throws -> [ContractModerationList] {
        try raw.split(separator: ",").map { try parseModerationList(String($0)) }
    }

    private func parseTeamActionStatus(_ raw: String) throws -> ContractTeamActionStatus {
        let name = raw.trimmingCharacters(in: .whitespaces).lowercased()
        guard let status = ContractTeamActionStatus(rawValue: name) else {
            throw SDKError.invalidParameter("Status must be active or closed, got '\(raw)'")
        }
        return status
    }

    /// The inputs of this query as submitted, by name, each trimmed; an
    /// empty input is left out.
    private func submittedInputs() -> [String: String] {
        var inputs: [String: String] = [:]
        for field in inputFields(for: query.name) {
            if let value = moderationInput(field.name) {
                inputs[field.name] = value
            }
        }
        return inputs
    }

    /// A moderation result as JSON: `{"inputs": ..., "result": ...}`, the
    /// inputs it was read for beside the answer, `null` when the FFI returned
    /// none. The answer is re-encoded from the decoded value as `JSONEncoder`
    /// writes it, the models' `Codable` form, so a removal record's kept
    /// values appear tagged (`{"type": "double", "value": 1}`).
    private func moderationResult<T: Encodable>(_ inputs: [String: String], _ answer: T?) throws -> Any {
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
        let data = try encoder.encode(ModerationCatalogResult(inputs: inputs, result: answer))
        return PreformattedJSON(text: String(decoding: data, as: UTF8.self))
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

        case "getDataContractsByRange":
            return [
                QueryInput(name: "limit", label: "Limit", required: false, placeholder: "100"),
                QueryInput(name: "startAfter", label: "Start After (Contract ID)", required: false, placeholder: "Last id of the previous page"),
                QueryInput(name: "startAt", label: "Start At (Contract ID)", required: false, placeholder: "Inclusive start"),
                QueryInput(name: "idsOnly", label: "IDs Only", required: false, placeholder: "true/false")
            ]

        // Contract Moderation Queries
        case "getContractModerationStatus":
            return [
                QueryInput(name: "contractId", label: "Contract ID", required: true),
                QueryInput(name: "identityId", label: "Identity ID", required: true),
                QueryInput(name: "lists", label: "Lists (comma-separated)", required: false, placeholder: "banlist, suspensions, warnings; empty reads every list the contract keeps")
            ]

        case "getContractModerationEntries":
            return [
                QueryInput(name: "contractId", label: "Contract ID", required: true),
                QueryInput(name: "list", label: "List", required: true, placeholder: "banlist, suspensions or warnings"),
                QueryInput(name: "startAfter", label: "Start After (Identity ID)", required: false, placeholder: "The previous page's nextStartAfter"),
                QueryInput(name: "limit", label: "Limit", required: false, placeholder: "Empty for the network's page cap")
            ]

        case "getContractDocumentRemovals":
            return [
                QueryInput(name: "contractId", label: "Contract ID", required: true),
                QueryInput(name: "documentTypeName", label: "Document Type", required: true),
                QueryInput(name: "documentIds", label: "Document IDs (comma-separated)", required: false, placeholder: "Read these records instead of a page"),
                QueryInput(name: "startAfter", label: "Start After (Document ID)", required: false, placeholder: "The previous page's nextStartAfter"),
                QueryInput(name: "limit", label: "Limit", required: false, placeholder: "Empty for the network's page cap")
            ]

        case "getContractTeamActions":
            return [
                QueryInput(name: "contractId", label: "Contract ID", required: true),
                QueryInput(name: "status", label: "Status", required: true, placeholder: "active or closed"),
                QueryInput(name: "startAtActionId", label: "Start At Action ID", required: false, placeholder: "The previous page's nextStartAtActionId"),
                QueryInput(name: "startAtActionIdIncluded", label: "Include Start Action", required: false, placeholder: "true or false; empty is false"),
                QueryInput(name: "limit", label: "Limit", required: false, placeholder: "Empty for the network's page cap")
            ]

        case "getContractTeamActionSigners":
            return [
                QueryInput(name: "contractId", label: "Contract ID", required: true),
                QueryInput(name: "status", label: "Status", required: true, placeholder: "active or closed"),
                QueryInput(name: "actionId", label: "Action ID", required: true)
            ]

        case "getContractModerationActionCounts", "getContractFeePots":
            return [
                QueryInput(name: "contractId", label: "Contract ID", required: true)
            ]

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

        case "getDocumentHistory":
            return [
                QueryInput(name: "dataContractId", label: "Data Contract ID", required: true),
                QueryInput(name: "documentType", label: "Document Type", required: true, placeholder: "A documentsKeepHistory type"),
                QueryInput(name: "documentId", label: "Document ID", required: true),
                QueryInput(name: "limit", label: "Limit (max 10)", required: false),
                QueryInput(name: "startAtMs", label: "Start At (milliseconds)", required: false, placeholder: "First page: inclusive lower time bound"),
                QueryInput(name: "startAfterTimeMs", label: "Start After Time (milliseconds)", required: false, placeholder: "Next page: last entry's time_ms"),
                QueryInput(name: "startAfterRevision", label: "Start After Revision", required: false, placeholder: "Next page: last entry's revision"),
                QueryInput(name: "startAtRevision", label: "Start At Revision", required: false, placeholder: "Revision range instead of time"),
                QueryInput(name: "revision", label: "Revision", required: false, placeholder: "One revision (limit 1)")
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

        // Contested DPNS Queries
        case "getContestedDpnsNames":
            return [
                QueryInput(name: "limit", label: "Limit", required: false, placeholder: "Default: 100"),
                QueryInput(name: "startAfter", label: "Start After", required: false, placeholder: "Normalized label, exclusive (e.g., a11ce)")
            ]

        case "getContestedDpnsNameVoteState":
            return [
                QueryInput(name: "name", label: "DPNS Name", required: true, placeholder: "e.g., alice or alice.dash")
            ]

        case "getContestedDpnsNameVotersForIdentity":
            return [
                QueryInput(name: "name", label: "DPNS Name", required: true, placeholder: "e.g., alice or alice.dash"),
                QueryInput(name: "identityId", label: "Identity ID", required: true, placeholder: "Base58 identity ID")
            ]

        case "getContestedDpnsNameIdentityVotes":
            return [
                QueryInput(name: "identityId", label: "Identity ID", required: true, placeholder: "Base58 identity ID"),
                QueryInput(name: "limit", label: "Limit", required: false, placeholder: "Default: 100"),
                QueryInput(name: "orderAscending", label: "Order Ascending", required: false, placeholder: "true/false")
            ]

        case "getDpnsVotePollsByEndDate":
            return [
                QueryInput(name: "startDate", label: "Start Date", required: false, placeholder: "ISO date"),
                QueryInput(name: "endDate", label: "End Date", required: false, placeholder: "ISO date"),
                QueryInput(name: "limit", label: "Limit", required: false, placeholder: "Default: 100")
            ]

        // Voting & Contested Resources Queries
        case "getContestedResources":
            return [
                QueryInput(name: "documentTypeName", label: "Document Type Name", required: true),
                QueryInput(name: "dataContractId", label: "Data Contract ID", required: true),
                QueryInput(name: "indexName", label: "Index Name", required: true),
                QueryInput(name: "startIndexValues", label: "Start Index Values", required: false, placeholder: "e.g., dash or [\"dash\"]"),
                QueryInput(name: "endIndexValues", label: "End Index Values", required: false, placeholder: "Values after the listed property"),
                QueryInput(name: "limit", label: "Limit", required: false),
                QueryInput(name: "orderAscending", label: "Order Ascending", required: false, placeholder: "true/false")
            ]

        case "getContestedResourceVoteState":
            return [
                QueryInput(name: "dataContractId", label: "Data Contract ID", required: true),
                QueryInput(name: "documentTypeName", label: "Document Type Name", required: true),
                QueryInput(name: "indexName", label: "Index Name", required: true),
                QueryInput(name: "indexValues", label: "Index Values", required: true, placeholder: "e.g., dash, a11ce; 0x… is bytes, \\0x… is text"),
                QueryInput(name: "resultType", label: "Result Type", required: false, placeholder: "Default: documents_and_vote_tally (or documents, vote_tally)"),
                QueryInput(name: "allowIncludeLockedAndAbstainingVoteTally", label: "Include Locked and Abstaining", required: false, placeholder: "true/false"),
                QueryInput(name: "count", label: "Request Limit", required: false, placeholder: "Single request; finished polls return all contenders")
            ]

        case "getContestedResourceVotersForIdentity":
            return [
                QueryInput(name: "dataContractId", label: "Data Contract ID", required: true),
                QueryInput(name: "documentTypeName", label: "Document Type Name", required: true),
                QueryInput(name: "indexName", label: "Index Name", required: true),
                QueryInput(name: "indexValues", label: "Index Values", required: true, placeholder: "e.g., dash, a11ce; 0x… is bytes, \\0x… is text"),
                QueryInput(name: "contestantId", label: "Contestant ID", required: true),
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
        case "getIdentityTokenBalances":
            return [
                QueryInput(name: "identityId", label: "Identity ID", required: true),
                QueryInput(name: "tokenIds", label: "Token IDs (comma-separated)", required: true)
            ]

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

        case "getContractGroupInfo":
            return [
                QueryInput(name: "contractGroupId", label: "Contract Group ID", required: true, placeholder: "Base58 encoded ID")
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

/// A moderation result of the catalog: the inputs the read answered and its
/// answer, `null` when the FFI returned none.
private struct ModerationCatalogResult<Answer: Encodable>: Encodable {
    let inputs: [String: String]
    let result: Answer?

    private enum CodingKeys: String, CodingKey {
        case inputs, result
    }

    func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encode(inputs, forKey: .inputs)
        try container.encode(result, forKey: .result)
    }
}

/// A query result already written as JSON text, shown as it is.
struct PreformattedJSON {
    let text: String
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
