import SwiftUI

struct DPNSTestView: View {
    @EnvironmentObject var appState: UnifiedAppState
    @State private var testResults: String = ""
    @State private var isLoading = false
    @State private var searchPrefix = "test"
    @State private var checkName = "testname"
    @State private var identityId = ""
    
    var body: some View {
        VStack(spacing: 20) {
            Text("DPNS Query Tests")
                .font(.title)
                .padding()
            
            ScrollView {
                VStack(alignment: .leading, spacing: 15) {
                    // Test 1: Search DPNS names
                    VStack(alignment: .leading) {
                        Text("Search DPNS Names")
                            .font(.headline)
                        
                        HStack {
                            TextField("Search prefix", text: $searchPrefix)
                                .textFieldStyle(RoundedBorderTextFieldStyle())
                            
                            Button("Search") {
                                Task {
                                    await testDPNSSearch()
                                }
                            }
                            .disabled(isLoading)
                        }
                    }
                    
                    Divider()
                    
                    // Test 2: Check availability
                    VStack(alignment: .leading) {
                        Text("Check Name Availability")
                            .font(.headline)
                        
                        HStack {
                            TextField("Name to check", text: $checkName)
                                .textFieldStyle(RoundedBorderTextFieldStyle())
                            
                            Button("Check") {
                                Task {
                                    await testDPNSAvailability()
                                }
                            }
                            .disabled(isLoading)
                        }
                    }
                    
                    Divider()
                    
                    // Test 3: Get usernames for identity
                    VStack(alignment: .leading) {
                        Text("Get Usernames for Identity")
                            .font(.headline)
                        
                        HStack {
                            TextField("Identity ID (hex)", text: $identityId)
                                .textFieldStyle(RoundedBorderTextFieldStyle())
                            
                            Button("Get") {
                                Task {
                                    await testGetUsernames()
                                }
                            }
                            .disabled(isLoading || identityId.isEmpty)
                        }
                    }
                    
                    Divider()
                    
                    // Results
                    VStack(alignment: .leading) {
                        Text("Results:")
                            .font(.headline)
                        
                        ScrollView {
                            Text(testResults)
                                .font(.system(.body, design: .monospaced))
                                .padding()
                                .background(Color.gray.opacity(0.1))
                                .cornerRadius(8)
                        }
                        .frame(maxHeight: 300)
                    }
                }
                .padding()
            }
            
            if isLoading {
                ProgressView()
                    .progressViewStyle(CircularProgressViewStyle())
            }
        }
        .navigationTitle("DPNS Tests")
        .navigationBarTitleDisplayMode(.inline)
    }
    
    private func testDPNSSearch() async {
        isLoading = true
        testResults = "Searching for names starting with '\(searchPrefix)'...\n"
        
        do {
            let results = try await appState.sdk?.dpnsSearch(prefix: searchPrefix, limit: 10)
            
            if let results = results {
                testResults += "Found \(results.count) names:\n"
                
                for (index, username) in results.enumerated() {
                    testResults += "\n[\(index + 1)]\n"
                    if let label = username["label"] as? String {
                        testResults += "  Label: \(label)\n"
                    }
                    if let normalizedLabel = username["normalizedLabel"] as? String {
                        testResults += "  Normalized: \(normalizedLabel)\n"
                    }
                    if let fullName = username["fullName"] as? String {
                        testResults += "  Full Name: \(fullName)\n"
                    }
                    if let ownerId = username["ownerId"] as? String {
                        testResults += "  Owner ID: \(ownerId)\n"
                    }
                }
            } else {
                testResults += "No results found.\n"
            }
        } catch {
            testResults += "Error: \(error)\n"
        }
        
        isLoading = false
    }
    
    private func testDPNSAvailability() async {
        isLoading = true
        testResults = "Checking availability of '\(checkName)'...\n"
        
        do {
            let isAvailable = try await appState.sdk?.dpnsCheckAvailability(name: checkName)
            
            if let isAvailable = isAvailable {
                testResults += "Name '\(checkName)' is \(isAvailable ? "AVAILABLE ✅" : "NOT AVAILABLE ❌")\n"
            } else {
                testResults += "Could not check availability.\n"
            }
        } catch {
            testResults += "Error: \(error)\n"
        }
        
        isLoading = false
    }
    
    private func testGetUsernames() async {
        isLoading = true
        testResults = "Getting usernames for identity '\(identityId)'...\n"
        
        do {
            let usernames = try await appState.sdk?.dpnsGetUsername(identityId: identityId, limit: 10)
            
            if let usernames = usernames {
                testResults += "Found \(usernames.count) usernames:\n"
                
                for (index, username) in usernames.enumerated() {
                    testResults += "\n[\(index + 1)]\n"
                    if let label = username["label"] as? String {
                        testResults += "  Label: \(label)\n"
                    }
                    if let normalizedLabel = username["normalizedLabel"] as? String {
                        testResults += "  Normalized: \(normalizedLabel)\n"
                    }
                    if let fullName = username["fullName"] as? String {
                        testResults += "  Full Name: \(fullName)\n"
                    }
                    if let recordsIdentityId = username["recordsIdentityId"] as? String {
                        testResults += "  Records Identity: \(recordsIdentityId)\n"
                    }
                    if let recordsAliasId = username["recordsAliasIdentityId"] as? String {
                        testResults += "  Alias Identity: \(recordsAliasId)\n"
                    }
                }
            } else {
                testResults += "No usernames found.\n"
            }
        } catch {
            testResults += "Error: \(error)\n"
        }
        
        isLoading = false
    }
}

struct DPNSTestView_Previews: PreviewProvider {
    static var previews: some View {
        DPNSTestView()
            .environmentObject(UnifiedAppState())
    }
}