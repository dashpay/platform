import SwiftUI
import SwiftDashSDK
import UIKit

struct DiagnosticsView: View {
    @EnvironmentObject var appState: UnifiedAppState
    @State private var isRunning = false
    @State private var results: [QueryTestResult] = []
    @State private var currentQuery = ""
    @State private var progress: Double = 0
    @State private var showResults = false
    @State private var showCopiedAlert = false
    
    struct QueryTestResult: Identifiable {
        let id = UUID()
        let queryName: String
        let queryLabel: String
        let category: String
        let success: Bool
        let result: String?
        let error: String?
        let duration: TimeInterval
    }
    
    // Test data from WASM SDK docs.html - exact same values for consistency
    struct TestData {
        // Identity IDs from WASM SDK examples
        static let testIdentityId = "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk"
        static let testIdentityId2 = "4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF"
        
        // Contract IDs
        static let dpnsContractId = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"
        static let testContractId = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"
        static let contractWithHistory = "HLY575cNazmc5824FxqaEMEBuzFeE4a98GDRNKbyJqCM"
        
        // Public key hashes from WASM SDK
        static let testPublicKeyHash = "b7e904ce25ed97594e72f7af0e66f298031c1754"
        static let testNonUniquePublicKeyHash = "518038dc858461bcee90478fd994bba8057b7531"
        
        // Document data
        static let testDocumentType = "domain"
        static let testDocumentId = "7NYmEKQsYtniQRUmxwdPGeVcirMoPh5ZPyAKz8BWFy3r"
        
        // DPNS
        static let testUsername = "dash"
        
        // Token
        static let testTokenId = "Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv"
        
        // Group
        static let testGroupContractId = "49PJEnNx7ReCitzkLdkDNr4s6RScGsnNexcdSZJ1ph5N"
        static let testActionId = "6XJzL6Qb8Zhwxt4HFwh8NAn7q1u4dwdoUf8EmgzDudFZ"
        
        // System
        static let testPrefundedSpecializedBalanceId = "AzaU7zqCT7X1kxh8yWxkT9PxAgNqWDu4Gz13emwcRyAT"
        
        // Contested resources test data
        static let testContestedIndexValues = ["dash", "alice"]
    }
    
    var body: some View {
        ScrollView {
            VStack(spacing: 20) {
                // Header
                VStack(alignment: .leading, spacing: 8) {
                    Text("Platform Query Diagnostics")
                        .font(.title2)
                        .fontWeight(.bold)
                    
                    Text("This tool runs all platform queries with test data to verify connectivity and functionality.")
                        .font(.body)
                        .foregroundColor(.secondary)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding()
                
                // Run Button
                Button(action: runAllQueries) {
                    HStack {
                        if isRunning {
                            ProgressView()
                                .progressViewStyle(CircularProgressViewStyle(tint: .white))
                                .scaleEffect(0.8)
                        } else {
                            Image(systemName: "play.fill")
                        }
                        Text(isRunning ? "Running..." : "Run All Queries")
                            .fontWeight(.semibold)
                    }
                    .frame(maxWidth: .infinity)
                    .padding()
                    .background(isRunning ? Color.gray : Color.blue)
                    .foregroundColor(.white)
                    .cornerRadius(10)
                }
                .disabled(isRunning || appState.platformState.sdk == nil)
                .padding(.horizontal)
                
                // Progress
                if isRunning {
                    VStack(spacing: 8) {
                        ProgressView(value: progress, total: 1.0)
                            .progressViewStyle(LinearProgressViewStyle())
                        
                        Text(currentQuery)
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                    .padding(.horizontal)
                }
                
                // Results
                if showResults && !results.isEmpty {
                    VStack(alignment: .leading, spacing: 16) {
                        HStack {
                            Text("Results")
                                .font(.headline)
                            
                            Spacer()
                            
                            let successCount = results.filter { $0.success }.count
                            let totalCount = results.count
                            
                            Text("\(successCount)/\(totalCount) passed")
                                .font(.caption)
                                .foregroundColor(successCount == totalCount ? .green : .orange)
                        }
                        
                        // Copy Report Button
                        Button(action: copyReport) {
                            HStack {
                                Image(systemName: "doc.on.doc")
                                Text("Copy Report")
                            }
                            .frame(maxWidth: .infinity)
                            .padding()
                            .background(Color.blue)
                            .foregroundColor(.white)
                            .cornerRadius(10)
                        }
                        .padding(.bottom, 8)
                        
                        ForEach(results) { result in
                            QueryResultRow(result: result)
                        }
                    }
                    .padding()
                }
            }
        }
        .navigationTitle("Run All Queries")
        .navigationBarTitleDisplayMode(.inline)
        .alert("Report Copied", isPresented: $showCopiedAlert) {
            Button("OK", role: .cancel) { }
        } message: {
            Text("The diagnostic report has been copied to your clipboard.")
        }
    }
    
    private func runAllQueries() {
        guard let sdk = appState.platformState.sdk else { return }
        
        isRunning = true
        results = []
        showResults = false
        progress = 0
        
        Task {
            var testResults: [QueryTestResult] = []
            
            // Define all queries to test with categories
            let queriesToTest: [(name: String, label: String, category: String, test: () async throws -> Any)] = [
                // Identity Queries (10 queries)
                ("getIdentity", "Get Identity", "Identity", {
                    try await sdk.identityGet(identityId: TestData.testIdentityId)
                }),
                
                ("getIdentityKeys", "Get Identity Keys", "Identity", {
                    try await sdk.identityGetKeys(identityId: TestData.testIdentityId)
                }),
                
                ("getIdentitiesContractKeys", "Get Identities Contract Keys", "Identity", {
                    try await sdk.identityGetContractKeys(
                        identityIds: [TestData.testIdentityId, TestData.testIdentityId2],
                        contractId: TestData.dpnsContractId,
                        documentType: "domain",
                        purposes: ["0", "1", "2", "3"]
                    )
                }),
                
                ("getIdentityNonce", "Get Identity Nonce", "Identity", {
                    try await sdk.identityGetNonce(identityId: TestData.testIdentityId)
                }),
                
                ("getIdentityContractNonce", "Get Identity Contract Nonce", "Identity", {
                    try await sdk.identityGetContractNonce(
                        identityId: TestData.testIdentityId,
                        contractId: TestData.dpnsContractId
                    )
                }),
                
                ("getIdentityBalance", "Get Identity Balance", "Identity", {
                    try await sdk.identityGetBalance(identityId: TestData.testIdentityId)
                }),
                
                ("getIdentitiesBalances", "Get Identities Balances", "Identity", {
                    try await sdk.identityGetBalances(identityIds: [TestData.testIdentityId, TestData.testIdentityId2])
                }),
                
                ("getIdentityBalanceAndRevision", "Get Identity Balance and Revision", "Identity", {
                    try await sdk.identityGetBalanceAndRevision(identityId: TestData.testIdentityId)
                }),
                
                ("getIdentityByPublicKeyHash", "Get Identity by Public Key Hash", "Identity", {
                    try await sdk.identityGetByPublicKeyHash(publicKeyHash: TestData.testPublicKeyHash)
                }),
                
                ("getIdentityByNonUniquePublicKeyHash", "Get Identity by Non-Unique Public Key Hash", "Identity", {
                    try await sdk.identityGetByNonUniquePublicKeyHash(
                        publicKeyHash: TestData.testNonUniquePublicKeyHash,
                        startAfter: nil
                    )
                }),
                
                // Data Contract Queries (3 queries)
                ("getDataContract", "Get Data Contract", "Data Contract", {
                    try await sdk.dataContractGet(id: TestData.dpnsContractId)
                }),
                
                ("getDataContractHistory", "Get Data Contract History", "Data Contract", {
                    try await sdk.dataContractGetHistory(id: TestData.contractWithHistory, limit: 10, offset: 0)
                }),
                
                ("getDataContracts", "Get Data Contracts", "Data Contract", {
                    try await sdk.dataContractGetMultiple(ids: [TestData.dpnsContractId])
                }),
                
                // Document Queries (2 queries)
                ("getDocuments", "Get Documents", "Documents", {
                    try await sdk.documentList(
                        dataContractId: TestData.dpnsContractId,
                        documentType: TestData.testDocumentType,
                        limit: 5
                    )
                }),
                
                ("getDocument", "Get Document", "Documents", {
                    try await sdk.documentGet(
                        dataContractId: TestData.dpnsContractId,
                        documentType: TestData.testDocumentType,
                        documentId: TestData.testDocumentId
                    )
                }),
                
                // DPNS Queries (4 queries)
                ("getDpnsUsername", "Get DPNS Usernames", "DPNS", {
                    try await sdk.dpnsGetUsername(identityId: TestData.testIdentityId, limit: 5)
                }),
                
                ("dpnsCheckAvailability", "DPNS Check Availability", "DPNS", {
                    try await sdk.dpnsCheckAvailability(name: "test-name-\(Int.random(in: 1000...9999))")
                }),
                
                ("dpnsResolve", "DPNS Resolve", "DPNS", {
                    try await sdk.dpnsResolve(name: TestData.testUsername)
                }),
                
                ("dpnsSearch", "DPNS Search", "DPNS", {
                    try await sdk.dpnsSearch(prefix: "dash", limit: 5)
                }),
                
                // Voting & Contested Resources Queries (5 queries)
                ("getContestedResources", "Get Contested Resources", "Voting", {
                    try await sdk.getContestedResources(
                        documentTypeName: "domain",
                        dataContractId: TestData.dpnsContractId,
                        indexName: "parentNameAndLabel",
                        resultType: "documents",
                        allowIncludeLockedAndAbstainingVoteTally: false,
                        startAtValue: nil,
                        limit: 5,
                        offset: 0,
                        orderAscending: true
                    )
                }),
                
                ("getContestedResourceVoteState", "Get Contested Resource Vote State", "Voting", {
                    try await sdk.getContestedResourceVoteState(
                        dataContractId: TestData.dpnsContractId,
                        documentTypeName: "domain",
                        indexName: "parentNameAndLabel",
                        indexValues: TestData.testContestedIndexValues,
                        resultType: "contenders",
                        allowIncludeLockedAndAbstainingVoteTally: false,
                        startAtIdentifierInfo: nil,
                        count: 5,
                        orderAscending: true
                    )
                }),
                
                ("getContestedResourceVotersForIdentity", "Get Contested Resource Voters for Identity", "Voting", {
                    try await sdk.getContestedResourceVotersForIdentity(
                        dataContractId: TestData.dpnsContractId,
                        documentTypeName: "domain",
                        indexName: "parentNameAndLabel",
                        indexValues: TestData.testContestedIndexValues,
                        contestantId: TestData.testIdentityId,
                        startAtIdentifierInfo: nil,
                        count: 5,
                        orderAscending: true
                    )
                }),
                
                ("getContestedResourceIdentityVotes", "Get Contested Resource Identity Votes", "Voting", {
                    try await sdk.getContestedResourceIdentityVotes(
                        identityId: TestData.testIdentityId,
                        limit: 5,
                        offset: 0,
                        orderAscending: true
                    )
                }),
                
                ("getVotePollsByEndDate", "Get Vote Polls by End Date", "Voting", {
                    try await sdk.getVotePollsByEndDate(
                        startTimeMs: nil,
                        endTimeMs: nil,
                        limit: 5,
                        offset: 0,
                        orderAscending: true
                    )
                }),
                
                // Protocol & Version Queries (2 queries)
                ("getProtocolVersionUpgradeState", "Get Protocol Version Upgrade State", "Protocol", {
                    try await sdk.getProtocolVersionUpgradeState()
                }),
                
                ("getProtocolVersionUpgradeVoteStatus", "Get Protocol Version Upgrade Vote Status", "Protocol", {
                    try await sdk.getProtocolVersionUpgradeVoteStatus(startProTxHash: nil, count: 5)
                }),
                
                // Epoch & Block Queries (5 queries)
                ("getEpochsInfo", "Get Epochs Info", "Epoch", {
                    try await sdk.getEpochsInfo(startEpoch: nil, count: 1, ascending: true)
                }),
                
                ("getCurrentEpoch", "Get Current Epoch", "Epoch", {
                    try await sdk.getCurrentEpoch()
                }),
                
                ("getFinalizedEpochInfos", "Get Finalized Epoch Infos", "Epoch", {
                    try await sdk.getFinalizedEpochInfos(startEpoch: nil, count: 1, ascending: true)
                }),
                
                ("getEvonodesProposedEpochBlocksByIds", "Get Evonodes Proposed Epoch Blocks by IDs", "Epoch", {
                    try await sdk.getEvonodesProposedEpochBlocksByIds(
                        epoch: 5,
                        ids: ["78adfbe419a528bb0f17e9a31b4ecc4f6b73ad1c97cdcef90f96bb6f0c432c87"]
                    )
                }),
                
                ("getEvonodesProposedEpochBlocksByRange", "Get Evonodes Proposed Epoch Blocks by Range", "Epoch", {
                    try await sdk.getEvonodesProposedEpochBlocksByRange(
                        epoch: 100,
                        limit: 5,
                        startAfter: "85F15A31D3838293A9C1D72A1A0FA21E66110CE20878BD4C1024C4AE1D5BE824",
                        orderAscending: true
                    )
                }),
                
                // Token Queries (8 queries)
                ("getIdentitiesTokenBalances", "Get Identities Token Balances", "Token", {
                    try await sdk.getIdentitiesTokenBalances(
                        identityIds: [TestData.testIdentityId],
                        tokenId: TestData.testTokenId
                    )
                }),
                
                ("getIdentityTokenInfos", "Get Identity Token Infos", "Token", {
                    try await sdk.getIdentityTokenInfos(
                        identityId: TestData.testIdentityId,
                        tokenIds: [TestData.testTokenId],
                        limit: nil,
                        offset: nil
                    )
                }),
                
                ("getIdentitiesTokenInfos", "Get Identities Token Infos", "Token", {
                    try await sdk.getIdentitiesTokenInfos(
                        identityIds: [TestData.testIdentityId],
                        tokenId: TestData.testTokenId
                    )
                }),
                
                ("getTokenStatuses", "Get Token Statuses", "Token", {
                    try await sdk.getTokenStatuses(tokenIds: [TestData.testTokenId])
                }),
                
                ("getTokenDirectPurchasePrices", "Get Token Direct Purchase Prices", "Token", {
                    try await sdk.getTokenDirectPurchasePrices(tokenIds: [TestData.testTokenId])
                }),
                
                ("getTokenContractInfo", "Get Token Contract Info", "Token", {
                    try await sdk.getTokenContractInfo(tokenId: TestData.testTokenId)
                }),
                
                ("getTokenPerpetualDistributionLastClaim", "Get Token Perpetual Distribution Last Claim", "Token", {
                    try await sdk.getTokenPerpetualDistributionLastClaim(
                        identityId: TestData.testIdentityId,
                        tokenId: TestData.testTokenId
                    )
                }),
                
                ("getTokenTotalSupply", "Get Token Total Supply", "Token", {
                    try await sdk.getTokenTotalSupply(tokenId: TestData.testTokenId)
                }),
                
                // Group Queries (4 queries)
                ("getGroupInfo", "Get Group Info", "Group", {
                    try await sdk.getGroupInfo(
                        contractId: TestData.testGroupContractId,
                        groupContractPosition: 0
                    )
                }),
                
                ("getGroupInfos", "Get Group Infos", "Group", {
                    try await sdk.getGroupInfos(
                        contractId: TestData.testGroupContractId,
                        startAtGroupContractPosition: nil,
                        startGroupContractPositionIncluded: true,
                        count: 5
                    )
                }),
                
                ("getGroupActions", "Get Group Actions", "Group", {
                    try await sdk.getGroupActions(
                        contractId: TestData.testGroupContractId,
                        groupContractPosition: 0,
                        status: "ACTIVE",
                        startActionId: nil,
                        startActionIdIncluded: true,
                        count: 5
                    )
                }),
                
                ("getGroupActionSigners", "Get Group Action Signers", "Group", {
                    try await sdk.getGroupActionSigners(
                        contractId: TestData.testGroupContractId,
                        groupContractPosition: 0,
                        status: "ACTIVE",
                        actionId: TestData.testActionId
                    )
                }),
                
                // System & Utility Queries (4 queries)
                ("getStatus", "Get Platform Status", "System", {
                    try await sdk.getStatus()
                }),
                
                ("getTotalCreditsInPlatform", "Get Total Credits in Platform", "System", {
                    try await sdk.getTotalCreditsInPlatform()
                }),
                
                ("getCurrentQuorumsInfo", "Get Current Quorums Info", "System", {
                    try await sdk.getCurrentQuorumsInfo()
                }),
                
                ("getPrefundedSpecializedBalance", "Get Prefunded Specialized Balance", "System", {
                    try await sdk.getPrefundedSpecializedBalance(id: TestData.testPrefundedSpecializedBalanceId)
                })
            ]
            
            let totalQueries = Double(queriesToTest.count)
            
            for (index, query) in queriesToTest.enumerated() {
                await MainActor.run {
                    currentQuery = "Testing: \(query.label)"
                    progress = Double(index) / totalQueries
                }
                
                let startTime = Date()
                var testResult: QueryTestResult
                
                do {
                    let result = try await query.test()
                    let duration = Date().timeIntervalSince(startTime)
                    
                    // Format result for display
                    let resultString = formatTestResult(result)
                    
                    testResult = QueryTestResult(
                        queryName: query.name,
                        queryLabel: query.label,
                        category: query.category,
                        success: true,
                        result: resultString,
                        error: nil,
                        duration: duration
                    )
                } catch {
                    let duration = Date().timeIntervalSince(startTime)
                    
                    testResult = QueryTestResult(
                        queryName: query.name,
                        queryLabel: query.label,
                        category: query.category,
                        success: false,
                        result: nil,
                        error: formatError(error),
                        duration: duration
                    )
                }
                
                testResults.append(testResult)
            }
            
            await MainActor.run {
                results = testResults
                showResults = true
                isRunning = false
                progress = 1.0
                currentQuery = "Complete"
            }
        }
    }
    
    private func copyReport() {
        var report = "Dash Platform iOS SDK - Query Diagnostics Report\n"
        report += "================================================\n\n"
        report += "Date: \(Date().formatted())\n"
        report += "SDK Network: Testnet\n\n"
        
        let successCount = results.filter { $0.success }.count
        let failedCount = results.filter { !$0.success }.count
        let totalCount = results.count
        
        report += "Summary:\n"
        report += "--------\n"
        report += "Total Queries: \(totalCount)\n"
        report += "Successful: \(successCount)\n"
        report += "Failed: \(failedCount)\n"
        report += "Success Rate: \(String(format: "%.1f%%", Double(successCount) / Double(totalCount) * 100))\n\n"
        
        // Group results by category
        let groupedResults = Dictionary(grouping: results, by: { $0.category })
        let sortedCategories = groupedResults.keys.sorted()
        
        // Successful Queries
        report += "SUCCESSFUL QUERIES:\n"
        report += "==================\n"
        for category in sortedCategories {
            let categoryResults = groupedResults[category] ?? []
            let successfulResults = categoryResults.filter { $0.success }
            if !successfulResults.isEmpty {
                report += "\n\(category):\n"
                for result in successfulResults {
                    report += "  ✓ \(result.queryLabel) (\(String(format: "%.3fs", result.duration)))\n"
                }
            }
        }
        
        // Failed Queries
        report += "\n\nFAILED QUERIES:\n"
        report += "===============\n"
        for category in sortedCategories {
            let categoryResults = groupedResults[category] ?? []
            let failedResults = categoryResults.filter { !$0.success }
            if !failedResults.isEmpty {
                report += "\n\(category):\n"
                for result in failedResults {
                    report += "  ✗ \(result.queryLabel)\n"
                    report += "    Error: \(result.error ?? "Unknown error")\n"
                    report += "    Duration: \(String(format: "%.3fs", result.duration))\n\n"
                }
            }
        }
        
        // Copy to pasteboard
        #if os(iOS)
        UIPasteboard.general.string = report
        #else
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(report, forType: .string)
        #endif
        
        showCopiedAlert = true
    }
    
    private func formatError(_ error: Error) -> String {
        if let sdkError = error as? SDKError {
            switch sdkError {
            case .invalidParameter(let msg):
                return "Invalid Parameter: \(msg)"
            case .invalidState(let msg):
                return "Invalid State: \(msg)"
            case .networkError(let msg):
                return "Network Error: \(msg)"
            case .serializationError(let msg):
                return "Serialization Error: \(msg)"
            case .protocolError(let msg):
                return "Protocol Error: \(msg)"
            case .cryptoError(let msg):
                return "Crypto Error: \(msg)"
            case .notFound(let msg):
                return "Not Found: \(msg)"
            case .timeout(let msg):
                return "Timeout: \(msg)"
            case .notImplemented(let msg):
                return "Not Implemented: \(msg)"
            case .internalError(let msg):
                return "Internal Error: \(msg)"
            case .unknown(let msg):
                return "Unknown Error: \(msg)"
            }
        }
        return error.localizedDescription
    }
    
    private func formatTestResult(_ result: Any) -> String {
        if let dict = result as? [String: Any] {
            return formatDictionary(dict)
        } else if let array = result as? [[String: Any]] {
            return "[\(array.count) items]"
        } else if let uint = result as? UInt64 {
            return String(uint)
        } else if let bool = result as? Bool {
            return bool ? "true" : "false"
        } else if let string = result as? String {
            return string
        } else {
            return String(describing: result)
        }
    }
    
    private func formatDictionary(_ dict: [String: Any]) -> String {
        if dict.isEmpty {
            return "{}"
        }
        
        // Show a few key fields for preview
        var preview = "{"
        let keys = Array(dict.keys.sorted().prefix(3))
        for (index, key) in keys.enumerated() {
            if index > 0 { preview += ", " }
            preview += "\(key): ..."
        }
        if dict.count > 3 {
            preview += ", ..."
        }
        preview += "}"
        return preview
    }
}

struct QueryResultRow: View {
    let result: DiagnosticsView.QueryTestResult
    @State private var isExpanded = false
    
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Button(action: { isExpanded.toggle() }) {
                HStack {
                    Image(systemName: result.success ? "checkmark.circle.fill" : "xmark.circle.fill")
                        .foregroundColor(result.success ? .green : .red)
                    
                    VStack(alignment: .leading, spacing: 2) {
                        Text(result.queryLabel)
                            .font(.subheadline)
                            .fontWeight(.medium)
                        
                        HStack {
                            Text(result.category)
                                .font(.caption2)
                                .foregroundColor(.blue)
                            
                            Text("•")
                                .font(.caption2)
                                .foregroundColor(.secondary)
                            
                            Text("\(String(format: "%.3f", result.duration))s")
                                .font(.caption2)
                                .foregroundColor(.secondary)
                        }
                    }
                    
                    Spacer()
                    
                    Image(systemName: isExpanded ? "chevron.up" : "chevron.down")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            .buttonStyle(PlainButtonStyle())
            
            if isExpanded {
                if let error = result.error {
                    Text("Error: \(error)")
                        .font(.caption)
                        .foregroundColor(.red)
                        .padding(.leading, 28)
                } else if let resultString = result.result {
                    Text("Result: \(resultString)")
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .padding(.leading, 28)
                        .lineLimit(5)
                }
            }
        }
        .padding(.vertical, 4)
        .padding(.horizontal, 12)
        .background(Color.gray.opacity(0.05))
        .cornerRadius(8)
    }
}

struct DiagnosticsView_Previews: PreviewProvider {
    static var previews: some View {
        NavigationView {
            DiagnosticsView()
                .environmentObject(UnifiedAppState())
        }
    }
}