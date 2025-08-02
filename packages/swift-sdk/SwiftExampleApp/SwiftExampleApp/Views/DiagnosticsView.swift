import SwiftUI
import SwiftDashSDK

struct DiagnosticsView: View {
    @EnvironmentObject var appState: UnifiedAppState
    @State private var isRunning = false
    @State private var results: [QueryTestResult] = []
    @State private var currentQuery = ""
    @State private var progress: Double = 0
    @State private var showResults = false
    
    struct QueryTestResult: Identifiable {
        let id = UUID()
        let queryName: String
        let queryLabel: String
        let success: Bool
        let result: String?
        let error: String?
        let duration: TimeInterval
    }
    
    // Test data based on WASM SDK examples
    struct TestData {
        // Common test values from testnet
        static let testIdentityId = "6ZhrNvhzD7Qm1nJhWzvipH9cPRLqBamdnXnKjnrrKA2c"
        static let testIdentityId2 = "HqyuZoKnHRdKP88Tz5L37whXHa27RuLRoQHzGgJGvCdU"
        static let dpnsContractId = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"
        static let testPublicKeyHash = "b7e904ce25ed97594e72f7af0e66f298031c1754"
        static let testDocumentType = "domain"
        static let testUsername = "dash"
        static let testTokenId = "Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv"
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
    }
    
    private func runAllQueries() {
        guard let sdk = appState.platformState.sdk else { return }
        
        isRunning = true
        results = []
        showResults = false
        progress = 0
        
        Task {
            var testResults: [QueryTestResult] = []
            
            // Define all queries to test
            let queriesToTest: [(name: String, label: String, test: () async throws -> Any)] = [
                // Identity Queries
                ("getIdentity", "Get Identity", {
                    try await sdk.identityGet(identityId: TestData.testIdentityId)
                }),
                
                ("getIdentityBalance", "Get Identity Balance", {
                    try await sdk.identityGetBalance(identityId: TestData.testIdentityId)
                }),
                
                ("getIdentityBalanceAndRevision", "Get Identity Balance and Revision", {
                    try await sdk.identityGetBalanceAndRevision(identityId: TestData.testIdentityId)
                }),
                
                ("getIdentityNonce", "Get Identity Nonce", {
                    try await sdk.identityGetNonce(identityId: TestData.testIdentityId)
                }),
                
                ("getIdentityContractNonce", "Get Identity Contract Nonce", {
                    try await sdk.identityGetContractNonce(
                        identityId: TestData.testIdentityId,
                        contractId: TestData.dpnsContractId
                    )
                }),
                
                ("getIdentityKeys", "Get Identity Keys", {
                    try await sdk.identityGetKeys(identityId: TestData.testIdentityId)
                }),
                
                ("getIdentitiesBalances", "Get Identities Balances", {
                    try await sdk.identityGetBalances(identityIds: [TestData.testIdentityId, TestData.testIdentityId2])
                }),
                
                ("getIdentityByPublicKeyHash", "Get Identity by Public Key Hash", {
                    try await sdk.identityGetByPublicKeyHash(publicKeyHash: TestData.testPublicKeyHash)
                }),
                
                // Data Contract Queries
                ("getDataContract", "Get Data Contract", {
                    try await sdk.dataContractGet(id: TestData.dpnsContractId)
                }),
                
                ("getDataContractHistory", "Get Data Contract History", {
                    try await sdk.dataContractGetHistory(id: TestData.dpnsContractId, limit: 5, offset: 0)
                }),
                
                // Document Queries
                ("getDocuments", "Get Documents", {
                    try await sdk.documentList(
                        dataContractId: TestData.dpnsContractId,
                        documentType: TestData.testDocumentType,
                        limit: 5
                    )
                }),
                
                // DPNS Queries
                ("dpnsResolve", "DPNS Resolve", {
                    try await sdk.dpnsResolve(name: TestData.testUsername)
                }),
                
                ("dpnsCheckAvailability", "DPNS Check Availability", {
                    try await sdk.dpnsCheckAvailability(name: "test-name-\(Int.random(in: 1000...9999))")
                }),
                
                ("dpnsSearch", "DPNS Search", {
                    try await sdk.dpnsSearch(prefix: "dash", limit: 5)
                }),
                
                // System Queries
                ("getStatus", "Get Platform Status", {
                    try await sdk.getStatus()
                }),
                
                ("getTotalCreditsInPlatform", "Get Total Credits", {
                    try await sdk.getTotalCreditsInPlatform()
                }),
                
                // Protocol Version Queries
                ("getProtocolVersionUpgradeState", "Get Protocol Version Upgrade State", {
                    try await sdk.getProtocolVersionUpgradeState()
                }),
                
                // Epoch Queries
                ("getCurrentEpoch", "Get Current Epoch", {
                    try await sdk.getCurrentEpoch()
                }),
                
                ("getEpochsInfo", "Get Epochs Info", {
                    try await sdk.getEpochsInfo(startEpoch: nil, count: 1, ascending: true)
                }),
                
                // Token Queries (might fail if tokens don't exist)
                ("getTokenStatuses", "Get Token Statuses", {
                    try await sdk.getTokenStatuses(tokenIds: [TestData.testTokenId])
                }),
                
                // Voting Queries
                ("getVotePollsByEndDate", "Get Vote Polls by End Date", {
                    try await sdk.getVotePollsByEndDate(
                        startTimeMs: nil,
                        endTimeMs: nil,
                        limit: 5,
                        offset: 0,
                        orderAscending: true
                    )
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
                        success: false,
                        result: nil,
                        error: error.localizedDescription,
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
                        
                        Text("\(String(format: "%.3f", result.duration))s")
                            .font(.caption2)
                            .foregroundColor(.secondary)
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