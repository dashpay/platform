import SwiftUI
import SwiftDashSDK

struct AddressQueriesView: View {
    @EnvironmentObject var appState: UnifiedAppState
    
    var body: some View {
        List {
            NavigationLink(destination: GetAddressInfoView()) {
                VStack(alignment: .leading, spacing: 4) {
                    Text("Get Address Info")
                        .font(.headline)
                    Text("Fetch balance and nonce for a single Platform address")
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .lineLimit(2)
                }
                .padding(.vertical, 4)
            }
            
            NavigationLink(destination: GetAddressesInfosView()) {
                VStack(alignment: .leading, spacing: 4) {
                    Text("Get Addresses Infos")
                        .font(.headline)
                    Text("Fetch balance and nonce for multiple Platform addresses")
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .lineLimit(2)
                }
                .padding(.vertical, 4)
            }
            
            NavigationLink(destination: GetTrunkStateView()) {
                VStack(alignment: .leading, spacing: 4) {
                    Text("Get Trunk State")
                        .font(.headline)
                    Text("Fetch address tree trunk state for privacy-preserving sync")
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .lineLimit(2)
                }
                .padding(.vertical, 4)
            }
            
            NavigationLink(destination: GetBranchStateView()) {
                VStack(alignment: .leading, spacing: 4) {
                    Text("Get Branch State")
                        .font(.headline)
                    Text("Query a specific branch of the address tree")
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .lineLimit(2)
                }
                .padding(.vertical, 4)
            }
            
            NavigationLink(destination: GetRecentBalanceChangesView()) {
                VStack(alignment: .leading, spacing: 4) {
                    Text("Get Recent Balance Changes")
                        .font(.headline)
                    Text("Fetch address balance changes since a block height")
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .lineLimit(2)
                }
                .padding(.vertical, 4)
            }
        }
        .navigationTitle("Address Queries")
        .navigationBarTitleDisplayMode(.inline)
    }
}

// MARK: - Get Address Info View

struct GetAddressInfoView: View {
    @EnvironmentObject var appState: UnifiedAppState
    @State private var addressInput: String = ""
    @State private var isLoading = false
    @State private var result: PlatformAddressInfo?
    @State private var errorMessage: String?
    @State private var showResult = false
    
    // Test addresses
    private let testBech32m = "tdashevo1qqyfsqyzcn5hzu7echru54njypdq0v4d7gv8pkdf"
    private let testAddressHex = "00" + "1234567890abcdef1234567890abcdef12345678"
    
    var body: some View {
        ScrollView {
            VStack(spacing: 20) {
                // Header
                VStack(alignment: .leading, spacing: 8) {
                    Text("Get Address Info")
                        .font(.title2)
                        .fontWeight(.bold)
                    
                    Text("Query the balance and nonce for a Platform address. Supports both bech32m (tdashevo1.../dashevo1...) and hex formats.")
                        .font(.body)
                        .foregroundColor(.secondary)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding()
                
                // Input
                VStack(alignment: .leading, spacing: 8) {
                    Text("Address")
                        .font(.subheadline)
                        .fontWeight(.medium)
                    
                    TextField("Enter bech32m or hex address", text: $addressInput)
                        .textFieldStyle(RoundedBorderTextFieldStyle())
                        .autocapitalization(.none)
                        .disableAutocorrection(true)
                    
                    HStack {
                        Button("Use Bech32m Test") {
                            addressInput = testBech32m
                        }
                        .font(.caption)
                        .foregroundColor(.blue)
                        
                        Text("•")
                            .foregroundColor(.secondary)
                        
                        Button("Use Hex Test") {
                            addressInput = testAddressHex
                        }
                        .font(.caption)
                        .foregroundColor(.blue)
                    }
                    
                    // Format indicator
                    if !addressInput.isEmpty {
                        VStack(alignment: .leading, spacing: 4) {
                            HStack {
                                Image(systemName: detectedFormat.icon)
                                    .foregroundColor(detectedFormat.color)
                                Text(detectedFormat.description)
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                            }
                            
                            // Debug info for bech32m
                            if addressInput.lowercased().hasPrefix("dashevo1") || addressInput.lowercased().hasPrefix("tdashevo1") {
                                let debug = Bech32m.debugDecode(addressInput)
                                if let hex = debug.hex, let count = debug.byteCount {
                                    Text("Decoded: \(count) bytes")
                                        .font(.caption2)
                                        .foregroundColor(.secondary)
                                    Text("Hex: \(hex)")
                                        .font(.caption2)
                                        .foregroundColor(.secondary)
                                        .lineLimit(1)
                                } else if let error = debug.error {
                                    Text("Error: \(error)")
                                        .font(.caption2)
                                        .foregroundColor(.red)
                                }
                            }
                        }
                    }
                }
                .padding(.horizontal)
                
                // Query Button
                Button(action: fetchAddressInfo) {
                    HStack {
                        if isLoading {
                            ProgressView()
                                .progressViewStyle(CircularProgressViewStyle(tint: .white))
                                .scaleEffect(0.8)
                        } else {
                            Image(systemName: "magnifyingglass")
                        }
                        Text(isLoading ? "Fetching..." : "Fetch Address Info")
                            .fontWeight(.semibold)
                    }
                    .frame(maxWidth: .infinity)
                    .padding()
                    .background(isLoading || addressInput.isEmpty ? Color.gray : Color.blue)
                    .foregroundColor(.white)
                    .cornerRadius(10)
                }
                .disabled(isLoading || addressInput.isEmpty || appState.platformState.sdk == nil)
                .padding(.horizontal)
                
                // Result
                if showResult {
                    VStack(alignment: .leading, spacing: 12) {
                        Text("Result")
                            .font(.headline)
                        
                        if let error = errorMessage {
                            HStack {
                                Image(systemName: "xmark.circle.fill")
                                    .foregroundColor(.red)
                                Text(error)
                                    .foregroundColor(.red)
                            }
                            .padding()
                            .background(Color.red.opacity(0.1))
                            .cornerRadius(8)
                        } else if let info = result {
                            VStack(alignment: .leading, spacing: 8) {
                                // Show bech32m address (use testnet prefix based on input)
                                // DashSDKNetwork: 0 = mainnet, 1 = testnet
                                let network = addressInput.lowercased().hasPrefix("tdashevo") 
                                    ? DashSDKNetwork(rawValue: 1)  // testnet
                                    : DashSDKNetwork(rawValue: 0)  // mainnet
                                let bech32mAddress = info.toBech32m(network: network) ?? info.addressHex
                                
                                ResultRow(label: "Address", value: bech32mAddress)
                                BalanceRow(label: "Balance", credits: info.balance)
                                ResultRow(label: "Nonce", value: "\(info.nonce)")
                                ResultRow(label: "Found", value: info.isFound ? "Yes" : "No")
                            }
                            .padding()
                            .background(Color.green.opacity(0.1))
                            .cornerRadius(8)
                        } else {
                            HStack {
                                Image(systemName: "questionmark.circle")
                                    .foregroundColor(.orange)
                                Text("Address not found on Platform")
                                    .foregroundColor(.orange)
                            }
                            .padding()
                            .background(Color.orange.opacity(0.1))
                            .cornerRadius(8)
                        }
                    }
                    .padding()
                }
                
                Spacer()
            }
        }
        .navigationTitle("Get Address Info")
        .navigationBarTitleDisplayMode(.inline)
    }
    
    // Format detection
    private var detectedFormat: (icon: String, color: Color, description: String) {
        let trimmed = addressInput.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        
        if trimmed.hasPrefix("dashevo1") || trimmed.hasPrefix("tdashevo1") {
            if Bech32m.isValidPlatformAddress(trimmed) {
                return ("checkmark.circle.fill", .green, "Valid bech32m address")
            } else {
                return ("xmark.circle.fill", .red, "Invalid bech32m address")
            }
        } else if trimmed.count == 42 && trimmed.allSatisfy({ $0.isHexDigit }) {
            return ("checkmark.circle.fill", .green, "Hex format (42 characters)")
        } else if !trimmed.isEmpty {
            return ("questionmark.circle", .orange, "Unknown format")
        }
        return ("circle", .gray, "")
    }
    
    private func fetchAddressInfo() {
        guard let sdk = appState.platformState.sdk else { return }
        
        isLoading = true
        errorMessage = nil
        result = nil
        showResult = false
        
        Task {
            do {
                // Use auto-detecting method
                let info = try sdk.addresses.getInfo(address: addressInput)
                
                await MainActor.run {
                    result = info
                    showResult = true
                    isLoading = false
                }
            } catch {
                await MainActor.run {
                    errorMessage = error.localizedDescription
                    showResult = true
                    isLoading = false
                }
            }
        }
    }
}

// MARK: - Get Addresses Infos View

struct GetAddressesInfosView: View {
    @EnvironmentObject var appState: UnifiedAppState
    @State private var addressesText: String = ""
    @State private var isLoading = false
    @State private var result: PlatformAddressInfosResult?
    @State private var errorMessage: String?
    @State private var showResult = false
    
    // Test addresses (mixed formats)
    private let testBech32mAddresses = """
    tdashevo1qqyfsqyzcn5hzu7echru54njypdq0v4d7gv8pkdf
    tdashevo1qq0rs5w7e3xv6ls3f7s4hz82e44p29e38fqlmhs
    """
    
    private let testHexAddresses = """
    001234567890abcdef1234567890abcdef12345678
    00abcdef1234567890abcdef1234567890abcdef12
    """
    
    var body: some View {
        ScrollView {
            VStack(spacing: 20) {
                // Header
                VStack(alignment: .leading, spacing: 8) {
                    Text("Get Addresses Infos")
                        .font(.title2)
                        .fontWeight(.bold)
                    
                    Text("Query balance and nonce for multiple Platform addresses. Enter one address per line. Supports bech32m and hex formats (can be mixed).")
                        .font(.body)
                        .foregroundColor(.secondary)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding()
                
                // Input
                VStack(alignment: .leading, spacing: 8) {
                    Text("Addresses (one per line)")
                        .font(.subheadline)
                        .fontWeight(.medium)
                    
                    TextEditor(text: $addressesText)
                        .frame(height: 120)
                        .overlay(
                            RoundedRectangle(cornerRadius: 8)
                                .stroke(Color.gray.opacity(0.3), lineWidth: 1)
                        )
                        .autocapitalization(.none)
                        .disableAutocorrection(true)
                    
                    HStack {
                        Button("Use Bech32m Test") {
                            addressesText = testBech32mAddresses
                        }
                        .font(.caption)
                        .foregroundColor(.blue)
                        
                        Text("•")
                            .foregroundColor(.secondary)
                        
                        Button("Use Hex Test") {
                            addressesText = testHexAddresses
                        }
                        .font(.caption)
                        .foregroundColor(.blue)
                    }
                }
                .padding(.horizontal)
                
                // Query Button
                Button(action: fetchAddressesInfos) {
                    HStack {
                        if isLoading {
                            ProgressView()
                                .progressViewStyle(CircularProgressViewStyle(tint: .white))
                                .scaleEffect(0.8)
                        } else {
                            Image(systemName: "magnifyingglass")
                        }
                        Text(isLoading ? "Fetching..." : "Fetch Addresses Infos")
                            .fontWeight(.semibold)
                    }
                    .frame(maxWidth: .infinity)
                    .padding()
                    .background(isLoading || addressesText.isEmpty ? Color.gray : Color.blue)
                    .foregroundColor(.white)
                    .cornerRadius(10)
                }
                .disabled(isLoading || addressesText.isEmpty || appState.platformState.sdk == nil)
                .padding(.horizontal)
                
                // Result
                if showResult {
                    VStack(alignment: .leading, spacing: 12) {
                        Text("Result")
                            .font(.headline)
                        
                        if let error = errorMessage {
                            HStack {
                                Image(systemName: "xmark.circle.fill")
                                    .foregroundColor(.red)
                                Text(error)
                                    .foregroundColor(.red)
                            }
                            .padding()
                            .background(Color.red.opacity(0.1))
                            .cornerRadius(8)
                        } else if let result = result {
                            // Determine network from input
                            // DashSDKNetwork: 0 = mainnet, 1 = testnet
                            let isTestnet = addressesText.lowercased().contains("tdashevo")
                            let network = isTestnet 
                                ? DashSDKNetwork(rawValue: 1)  // testnet
                                : DashSDKNetwork(rawValue: 0)  // mainnet
                            
                            VStack(alignment: .leading, spacing: 12) {
                                // Summary
                                HStack {
                                    Text("Total: \(result.infos.count)")
                                    Spacer()
                                    Text("Found: \(result.foundAddresses.count)")
                                        .foregroundColor(.green)
                                    Text("Not Found: \(result.notFoundAddresses.count)")
                                        .foregroundColor(.orange)
                                }
                                .font(.subheadline)
                                .padding()
                                .background(Color.gray.opacity(0.1))
                                .cornerRadius(8)
                                
                                // Total balance in both formats
                                VStack(alignment: .leading, spacing: 4) {
                                    Text("Total Balance")
                                        .font(.subheadline)
                                        .fontWeight(.medium)
                                    Text("\(formatCredits(result.totalBalance)) credits")
                                        .font(.caption)
                                        .foregroundColor(.secondary)
                                    Text("\(formatDash(result.totalBalance)) DASH")
                                        .font(.caption)
                                        .foregroundColor(.blue)
                                }
                                .padding(8)
                                .background(Color.blue.opacity(0.05))
                                .cornerRadius(6)
                                
                                // Individual results
                                ForEach(Array(result.infos.values), id: \.addressHex) { info in
                                    let bech32mAddress = info.toBech32m(network: network) ?? info.addressHex
                                    
                                    VStack(alignment: .leading, spacing: 4) {
                                        HStack {
                                            Image(systemName: info.isFound ? "checkmark.circle.fill" : "questionmark.circle")
                                                .foregroundColor(info.isFound ? .green : .orange)
                                            Text(bech32mAddress)
                                                .font(.caption)
                                                .monospaced()
                                                .lineLimit(1)
                                        }
                                        if info.isFound {
                                            HStack {
                                                VStack(alignment: .leading) {
                                                    Text("\(formatCredits(info.balance)) credits")
                                                    Text("\(formatDash(info.balance)) DASH")
                                                        .foregroundColor(.blue)
                                                }
                                                Spacer()
                                                Text("Nonce: \(info.nonce)")
                                            }
                                            .font(.caption)
                                            .foregroundColor(.secondary)
                                        }
                                    }
                                    .padding(8)
                                    .background(info.isFound ? Color.green.opacity(0.05) : Color.orange.opacity(0.05))
                                    .cornerRadius(6)
                                }
                            }
                        }
                    }
                    .padding()
                }
                
                Spacer()
            }
        }
        .navigationTitle("Get Addresses Infos")
        .navigationBarTitleDisplayMode(.inline)
    }
    
    private func fetchAddressesInfos() {
        guard let sdk = appState.platformState.sdk else { return }
        
        isLoading = true
        errorMessage = nil
        result = nil
        showResult = false
        
        // Parse addresses from text
        let addresses = addressesText
            .components(separatedBy: .newlines)
            .map { $0.trimmingCharacters(in: .whitespaces) }
            .filter { !$0.isEmpty }
        
        guard !addresses.isEmpty else {
            errorMessage = "No valid addresses entered"
            showResult = true
            isLoading = false
            return
        }
        
        Task {
            do {
                // Use auto-detecting method that handles both bech32m and hex
                let infosResult = try sdk.addresses.getInfos(addresses: addresses)
                
                await MainActor.run {
                    result = infosResult
                    showResult = true
                    isLoading = false
                }
            } catch {
                await MainActor.run {
                    errorMessage = error.localizedDescription
                    showResult = true
                    isLoading = false
                }
            }
        }
    }
}

// MARK: - Get Trunk State View

struct GetTrunkStateView: View {
    @EnvironmentObject var appState: UnifiedAppState
    @State private var isLoading = false
    @State private var result: PlatformTrunkState?
    @State private var errorMessage: String?
    @State private var showResult = false
    
    var body: some View {
        ScrollView {
            VStack(spacing: 20) {
                // Header
                VStack(alignment: .leading, spacing: 8) {
                    Text("Get Trunk State")
                        .font(.title2)
                        .fontWeight(.bold)
                    
                    Text("Fetch the trunk state of the address tree. This returns addresses at the top levels of the tree and leaf boundaries for subtrees that need further queries.")
                        .font(.body)
                        .foregroundColor(.secondary)
                    
                    Text("This is a low-level API used for privacy-preserving address synchronization.")
                        .font(.caption)
                        .foregroundColor(.orange)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding()
                
                // Query Button
                Button(action: fetchTrunkState) {
                    HStack {
                        if isLoading {
                            ProgressView()
                                .progressViewStyle(CircularProgressViewStyle(tint: .white))
                                .scaleEffect(0.8)
                        } else {
                            Image(systemName: "square.stack.3d.up")
                        }
                        Text(isLoading ? "Fetching..." : "Fetch Trunk State")
                            .fontWeight(.semibold)
                    }
                    .frame(maxWidth: .infinity)
                    .padding()
                    .background(isLoading ? Color.gray : Color.blue)
                    .foregroundColor(.white)
                    .cornerRadius(10)
                }
                .disabled(isLoading || appState.platformState.sdk == nil)
                .padding(.horizontal)
                
                // Result
                if showResult {
                    VStack(alignment: .leading, spacing: 12) {
                        Text("Result")
                            .font(.headline)
                        
                        if let error = errorMessage {
                            HStack {
                                Image(systemName: "xmark.circle.fill")
                                    .foregroundColor(.red)
                                Text(error)
                                    .foregroundColor(.red)
                            }
                            .padding()
                            .background(Color.red.opacity(0.1))
                            .cornerRadius(8)
                        } else if let state = result {
                            VStack(alignment: .leading, spacing: 16) {
                                // Summary
                                VStack(alignment: .leading, spacing: 8) {
                                    HStack {
                                        Label("Checkpoint Height", systemImage: "number.square")
                                        Spacer()
                                        Text("\(state.checkpointHeight)")
                                            .fontWeight(.medium)
                                    }
                                    
                                    HStack {
                                        Label("Elements", systemImage: "person.2")
                                        Spacer()
                                        Text("\(state.elements.count)")
                                            .fontWeight(.medium)
                                    }
                                    
                                    HStack {
                                        Label("Leaf Boundaries", systemImage: "leaf")
                                        Spacer()
                                        Text("\(state.leafBoundaries.count)")
                                            .fontWeight(.medium)
                                    }
                                    
                                    if !state.elements.isEmpty {
                                        VStack(alignment: .leading, spacing: 4) {
                                            Text("Total Balance")
                                                .font(.subheadline)
                                            Text("\(formatCredits(state.totalBalance)) credits")
                                                .fontWeight(.medium)
                                            Text("\(formatDash(state.totalBalance)) DASH")
                                                .foregroundColor(.blue)
                                        }
                                    }
                                }
                                .font(.subheadline)
                                .padding()
                                .background(Color.gray.opacity(0.1))
                                .cornerRadius(8)
                                
                                // Elements section
                                if !state.elements.isEmpty {
                                    VStack(alignment: .leading, spacing: 8) {
                                        Text("Elements (\(state.elements.count))")
                                            .font(.subheadline)
                                            .fontWeight(.semibold)
                                        
                                        ForEach(Array(state.elements.enumerated()), id: \.offset) { index, element in
                                            VStack(alignment: .leading, spacing: 4) {
                                                Text("Key: \(element.keyHex.prefix(16))...")
                                                    .font(.caption)
                                                    .monospaced()
                                                HStack {
                                                    Text("\(formatCredits(element.balance)) credits")
                                                    Spacer()
                                                    Text("Nonce: \(element.nonce)")
                                                }
                                                .font(.caption)
                                                .foregroundColor(.secondary)
                                            }
                                            .padding(8)
                                            .background(Color.green.opacity(0.05))
                                            .cornerRadius(6)
                                        }
                                    }
                                }
                                
                                // Leaf boundaries section
                                if !state.leafBoundaries.isEmpty {
                                    VStack(alignment: .leading, spacing: 8) {
                                        Text("Leaf Boundaries (\(state.leafBoundaries.count)) - Tap to copy")
                                            .font(.subheadline)
                                            .fontWeight(.semibold)
                                        
                                        Button(action: {
                                            UIPasteboard.general.string = "\(state.checkpointHeight)"
                                        }) {
                                            HStack {
                                                Text("Checkpoint Height: \(state.checkpointHeight)")
                                                    .font(.caption)
                                                Image(systemName: "doc.on.doc")
                                                    .font(.caption2)
                                            }
                                        }
                                        .buttonStyle(.plain)
                                        .foregroundColor(.blue)
                                        
                                        ForEach(Array(state.leafBoundaries.enumerated()), id: \.offset) { index, boundary in
                                            VStack(alignment: .leading, spacing: 6) {
                                                HStack {
                                                    Text("Key:")
                                                        .font(.caption)
                                                        .frame(width: 40, alignment: .leading)
                                                    Button(action: {
                                                        UIPasteboard.general.string = boundary.keyHex
                                                    }) {
                                                        HStack(spacing: 4) {
                                                            Text(boundary.keyHex)
                                                                .font(.caption2)
                                                                .monospaced()
                                                                .lineLimit(1)
                                                                .truncationMode(.middle)
                                                            Image(systemName: "doc.on.doc")
                                                                .font(.caption2)
                                                        }
                                                    }
                                                    .buttonStyle(.plain)
                                                    .foregroundColor(.blue)
                                                }
                                                
                                                HStack {
                                                    Text("Hash:")
                                                        .font(.caption)
                                                        .frame(width: 40, alignment: .leading)
                                                    Button(action: {
                                                        UIPasteboard.general.string = boundary.hashHex
                                                    }) {
                                                        HStack(spacing: 4) {
                                                            Text(boundary.hashHex)
                                                                .font(.caption2)
                                                                .monospaced()
                                                                .lineLimit(1)
                                                                .truncationMode(.middle)
                                                            Image(systemName: "doc.on.doc")
                                                                .font(.caption2)
                                                        }
                                                    }
                                                    .buttonStyle(.plain)
                                                    .foregroundColor(.blue)
                                                }
                                                
                                                if boundary.estimatedCount > 0 {
                                                    Text("Est. count: \(boundary.estimatedCount)")
                                                        .font(.caption)
                                                        .foregroundColor(.secondary)
                                                }
                                            }
                                            .padding(8)
                                            .frame(maxWidth: .infinity, alignment: .leading)
                                            .background(Color.orange.opacity(0.05))
                                            .cornerRadius(6)
                                        }
                                    }
                                }
                            }
                        }
                    }
                    .padding()
                }
                
                Spacer()
            }
        }
        .navigationTitle("Get Trunk State")
        .navigationBarTitleDisplayMode(.inline)
    }
    
    private func fetchTrunkState() {
        guard let sdk = appState.platformState.sdk else { return }
        
        isLoading = true
        errorMessage = nil
        result = nil
        showResult = false
        
        Task {
            do {
                let trunkState = try sdk.addresses.getTrunkState()
                
                await MainActor.run {
                    result = trunkState
                    showResult = true
                    isLoading = false
                }
            } catch {
                await MainActor.run {
                    errorMessage = error.localizedDescription
                    showResult = true
                    isLoading = false
                }
            }
        }
    }
}

// MARK: - Get Branch State View

struct GetBranchStateView: View {
    @EnvironmentObject var appState: UnifiedAppState
    @State private var keyHex: String = ""
    @State private var depth: String = "6"  // Valid range: 6-9
    @State private var expectedHashHex: String = ""
    @State private var checkpointHeight: String = ""
    @State private var isLoading = false
    @State private var result: PlatformBranchState?
    @State private var errorMessage: String?
    @State private var showResult = false
    
    var body: some View {
        ScrollView {
            VStack(spacing: 20) {
                // Header
                VStack(alignment: .leading, spacing: 8) {
                    Text("Get Branch State")
                        .font(.title2)
                        .fontWeight(.bold)
                    
                    Text("Query a specific branch of the address tree. Use leaf boundary info from a trunk state query.")
                        .font(.body)
                        .foregroundColor(.secondary)
                    
                    Text("This is a low-level API. Parameters come from trunk state leaf boundaries.")
                        .font(.caption)
                        .foregroundColor(.orange)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding()
                
                // Inputs
                VStack(alignment: .leading, spacing: 16) {
                    VStack(alignment: .leading, spacing: 4) {
                        Text("Key (hex)")
                            .font(.subheadline)
                            .fontWeight(.medium)
                        TextField("Leaf boundary key from trunk state", text: $keyHex)
                            .textFieldStyle(RoundedBorderTextFieldStyle())
                            .autocapitalization(.none)
                            .disableAutocorrection(true)
                    }
                    
                    VStack(alignment: .leading, spacing: 4) {
                        Text("Depth (6-9)")
                            .font(.subheadline)
                            .fontWeight(.medium)
                        TextField("Query depth (6-9)", text: $depth)
                            .textFieldStyle(RoundedBorderTextFieldStyle())
                            .keyboardType(.numberPad)
                    }
                    
                    VStack(alignment: .leading, spacing: 4) {
                        Text("Expected Hash (hex, 64 chars)")
                            .font(.subheadline)
                            .fontWeight(.medium)
                        TextField("Hash from leaf boundary info", text: $expectedHashHex)
                            .textFieldStyle(RoundedBorderTextFieldStyle())
                            .autocapitalization(.none)
                            .disableAutocorrection(true)
                    }
                    
                    VStack(alignment: .leading, spacing: 4) {
                        Text("Checkpoint Height")
                            .font(.subheadline)
                            .fontWeight(.medium)
                        TextField("Height from trunk state", text: $checkpointHeight)
                            .textFieldStyle(RoundedBorderTextFieldStyle())
                            .keyboardType(.numberPad)
                    }
                }
                .padding(.horizontal)
                
                // Query Button
                Button(action: fetchBranchState) {
                    HStack {
                        if isLoading {
                            ProgressView()
                                .progressViewStyle(CircularProgressViewStyle(tint: .white))
                                .scaleEffect(0.8)
                        } else {
                            Image(systemName: "arrow.branch")
                        }
                        Text(isLoading ? "Fetching..." : "Fetch Branch State")
                            .fontWeight(.semibold)
                    }
                    .frame(maxWidth: .infinity)
                    .padding()
                    .background(isLoading || !isFormValid ? Color.gray : Color.blue)
                    .foregroundColor(.white)
                    .cornerRadius(10)
                }
                .disabled(isLoading || !isFormValid || appState.platformState.sdk == nil)
                .padding(.horizontal)
                
                // Result
                if showResult {
                    VStack(alignment: .leading, spacing: 12) {
                        Text("Result")
                            .font(.headline)
                        
                        if let error = errorMessage {
                            HStack {
                                Image(systemName: "xmark.circle.fill")
                                    .foregroundColor(.red)
                                Text(error)
                                    .foregroundColor(.red)
                            }
                            .padding()
                            .background(Color.red.opacity(0.1))
                            .cornerRadius(8)
                        } else if let state = result {
                            VStack(alignment: .leading, spacing: 16) {
                                // Summary
                                VStack(alignment: .leading, spacing: 8) {
                                    HStack {
                                        Label("Elements", systemImage: "person.2")
                                        Spacer()
                                        Text("\(state.elements.count)")
                                            .fontWeight(.medium)
                                    }
                                    
                                    HStack {
                                        Label("Leaf Boundaries", systemImage: "leaf")
                                        Spacer()
                                        Text("\(state.leafBoundaries.count)")
                                            .fontWeight(.medium)
                                    }
                                    
                                    if !state.elements.isEmpty {
                                        VStack(alignment: .leading, spacing: 4) {
                                            Text("Total Balance")
                                                .font(.subheadline)
                                            Text("\(formatCredits(state.totalBalance)) credits")
                                                .fontWeight(.medium)
                                            Text("\(formatDash(state.totalBalance)) DASH")
                                                .foregroundColor(.blue)
                                        }
                                    }
                                }
                                .font(.subheadline)
                                .padding()
                                .background(Color.gray.opacity(0.1))
                                .cornerRadius(8)
                                
                                // Elements section
                                if !state.elements.isEmpty {
                                    VStack(alignment: .leading, spacing: 8) {
                                        Text("Elements (\(state.elements.count))")
                                            .font(.subheadline)
                                            .fontWeight(.semibold)
                                        
                                        ForEach(Array(state.elements.enumerated()), id: \.offset) { _, element in
                                            VStack(alignment: .leading, spacing: 4) {
                                                Text("Key: \(element.keyHex.prefix(16))...")
                                                    .font(.caption)
                                                    .monospaced()
                                                HStack {
                                                    Text("\(formatCredits(element.balance)) credits")
                                                    Spacer()
                                                    Text("Nonce: \(element.nonce)")
                                                }
                                                .font(.caption)
                                                .foregroundColor(.secondary)
                                            }
                                            .padding(8)
                                            .background(Color.green.opacity(0.05))
                                            .cornerRadius(6)
                                        }
                                    }
                                }
                                
                                // Leaf boundaries section
                                if !state.leafBoundaries.isEmpty {
                                    VStack(alignment: .leading, spacing: 8) {
                                        Text("Leaf Boundaries (\(state.leafBoundaries.count)) - Tap to copy")
                                            .font(.subheadline)
                                            .fontWeight(.semibold)
                                        
                                        ForEach(Array(state.leafBoundaries.enumerated()), id: \.offset) { _, boundary in
                                            VStack(alignment: .leading, spacing: 6) {
                                                HStack {
                                                    Text("Key:")
                                                        .font(.caption)
                                                        .frame(width: 40, alignment: .leading)
                                                    Button(action: {
                                                        UIPasteboard.general.string = boundary.keyHex
                                                    }) {
                                                        HStack(spacing: 4) {
                                                            Text(boundary.keyHex)
                                                                .font(.caption2)
                                                                .monospaced()
                                                                .lineLimit(1)
                                                                .truncationMode(.middle)
                                                            Image(systemName: "doc.on.doc")
                                                                .font(.caption2)
                                                        }
                                                    }
                                                    .buttonStyle(.plain)
                                                    .foregroundColor(.blue)
                                                }
                                                
                                                HStack {
                                                    Text("Hash:")
                                                        .font(.caption)
                                                        .frame(width: 40, alignment: .leading)
                                                    Button(action: {
                                                        UIPasteboard.general.string = boundary.hashHex
                                                    }) {
                                                        HStack(spacing: 4) {
                                                            Text(boundary.hashHex)
                                                                .font(.caption2)
                                                                .monospaced()
                                                                .lineLimit(1)
                                                                .truncationMode(.middle)
                                                            Image(systemName: "doc.on.doc")
                                                                .font(.caption2)
                                                        }
                                                    }
                                                    .buttonStyle(.plain)
                                                    .foregroundColor(.blue)
                                                }
                                                
                                                if boundary.estimatedCount > 0 {
                                                    Text("Est. count: \(boundary.estimatedCount)")
                                                        .font(.caption)
                                                        .foregroundColor(.secondary)
                                                }
                                            }
                                            .padding(8)
                                            .frame(maxWidth: .infinity, alignment: .leading)
                                            .background(Color.orange.opacity(0.05))
                                            .cornerRadius(6)
                                        }
                                    }
                                }
                            }
                        }
                    }
                    .padding()
                }
                
                Spacer()
            }
        }
        .navigationTitle("Get Branch State")
        .navigationBarTitleDisplayMode(.inline)
    }
    
    private var isFormValid: Bool {
        guard let depthValue = UInt32(depth) else { return false }
        return !keyHex.isEmpty && 
            !expectedHashHex.isEmpty && 
            expectedHashHex.count == 64 &&
            !checkpointHeight.isEmpty &&
            depthValue >= 6 && depthValue <= 9 &&
            UInt64(checkpointHeight) != nil
    }
    
    private func fetchBranchState() {
        guard let sdk = appState.platformState.sdk else { return }
        
        guard let keyData = Data(hexString: keyHex) else {
            errorMessage = "Invalid key hex"
            showResult = true
            return
        }
        
        guard let hashData = Data(hexString: expectedHashHex), hashData.count == 32 else {
            errorMessage = "Invalid expected hash hex (must be 64 hex characters = 32 bytes)"
            showResult = true
            return
        }
        
        guard let depthValue = UInt32(depth), depthValue >= 6 && depthValue <= 9 else {
            errorMessage = "Depth must be between 6 and 9"
            showResult = true
            return
        }
        
        guard let heightValue = UInt64(checkpointHeight) else {
            errorMessage = "Invalid checkpoint height"
            showResult = true
            return
        }
        
        isLoading = true
        errorMessage = nil
        result = nil
        showResult = false
        
        Task {
            do {
                let branchState = try sdk.addresses.getBranchState(
                    key: keyData,
                    depth: depthValue,
                    expectedHash: hashData,
                    checkpointHeight: heightValue
                )
                
                await MainActor.run {
                    result = branchState
                    showResult = true
                    isLoading = false
                }
            } catch {
                await MainActor.run {
                    errorMessage = error.localizedDescription
                    showResult = true
                    isLoading = false
                }
            }
        }
    }
}

// MARK: - Get Recent Balance Changes View

struct GetRecentBalanceChangesView: View {
    @EnvironmentObject var appState: UnifiedAppState
    @State private var startHeightInput: String = "0"
    @State private var isLoading = false
    @State private var result: RecentBalanceChanges?
    @State private var errorMessage: String?
    @State private var showResult = false
    
    var body: some View {
        ScrollView {
            VStack(spacing: 20) {
                // Header
                VStack(alignment: .leading, spacing: 8) {
                    Text("Get Recent Balance Changes")
                        .font(.title2)
                        .fontWeight(.bold)
                    
                    Text("Fetch all address balance changes that occurred since the specified block height. Useful for syncing wallet balances after initial sync.")
                        .font(.body)
                        .foregroundColor(.secondary)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding()
                
                // Input
                VStack(alignment: .leading, spacing: 8) {
                    Text("Start Block Height")
                        .font(.subheadline)
                        .fontWeight(.medium)
                    
                    TextField("Enter start block height", text: $startHeightInput)
                        .textFieldStyle(RoundedBorderTextFieldStyle())
                        .keyboardType(.numberPad)
                    
                    Text("Enter 0 to get all recent balance changes")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                .padding(.horizontal)
                
                // Fetch Button
                Button(action: fetchRecentChanges) {
                    HStack {
                        if isLoading {
                            ProgressView()
                                .progressViewStyle(CircularProgressViewStyle(tint: .white))
                        } else {
                            Image(systemName: "arrow.triangle.2.circlepath")
                        }
                        Text(isLoading ? "Fetching..." : "Fetch Recent Changes")
                    }
                    .frame(maxWidth: .infinity)
                    .padding()
                    .background(Color.blue)
                    .foregroundColor(.white)
                    .cornerRadius(10)
                }
                .disabled(isLoading || !isFormValid)
                .opacity((isLoading || !isFormValid) ? 0.6 : 1.0)
                .padding(.horizontal)
                
                // Result
                if showResult {
                    if let error = errorMessage {
                        VStack(alignment: .leading, spacing: 8) {
                            Label("Error", systemImage: "exclamationmark.triangle.fill")
                                .foregroundColor(.red)
                                .font(.headline)
                            Text(error)
                                .font(.body)
                                .foregroundColor(.red)
                        }
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding()
                        .background(Color.red.opacity(0.1))
                        .cornerRadius(10)
                        .padding(.horizontal)
                    } else if let changes = result {
                        // Get network from SDK (0 = mainnet, 1 = testnet)
                        let network = appState.platformState.sdk?.network ?? DashSDKNetwork(rawValue: 1)
                        RecentBalanceChangesResultView(changes: changes, network: network)
                            .padding(.horizontal)
                    }
                }
                
                Spacer()
            }
        }
        .navigationTitle("Recent Balance Changes")
        .navigationBarTitleDisplayMode(.inline)
    }
    
    private var isFormValid: Bool {
        return UInt64(startHeightInput) != nil
    }
    
    private func fetchRecentChanges() {
        guard let sdk = appState.platformState.sdk else { return }
        guard let startHeight = UInt64(startHeightInput) else {
            errorMessage = "Invalid start height"
            showResult = true
            return
        }
        
        isLoading = true
        errorMessage = nil
        result = nil
        showResult = false
        
        Task {
            do {
                let changes = try sdk.addresses.getRecentBalanceChanges(startHeight: startHeight)
                
                await MainActor.run {
                    result = changes
                    showResult = true
                    isLoading = false
                }
            } catch {
                await MainActor.run {
                    errorMessage = error.localizedDescription
                    showResult = true
                    isLoading = false
                }
            }
        }
    }
}

// MARK: - Recent Balance Changes Result View

struct RecentBalanceChangesResultView: View {
    let changes: RecentBalanceChanges
    let network: DashSDKNetwork
    
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            // Summary
            VStack(alignment: .leading, spacing: 12) {
                Label("Summary", systemImage: "list.bullet.rectangle")
                    .font(.headline)
                
                ResultRow(label: "Blocks", value: "\(changes.blocks.count)")
                ResultRow(label: "Total Changes", value: "\(changes.totalChangesCount)")
                
                if let range = changes.heightRange {
                    ResultRow(label: "Height Range", value: "\(range.lowerBound) - \(range.upperBound)")
                }
            }
            .padding()
            .background(Color.gray.opacity(0.1))
            .cornerRadius(10)
            
            // Block-by-block details
            if !changes.blocks.isEmpty {
                VStack(alignment: .leading, spacing: 12) {
                    Label("Balance Changes by Block", systemImage: "cube.fill")
                        .font(.headline)
                    
                    ForEach(Array(changes.blocks.enumerated()), id: \.offset) { _, block in
                        BlockBalanceChangesView(block: block, network: network)
                    }
                }
            } else {
                Text("No balance changes found from the specified block height.")
                    .font(.body)
                    .foregroundColor(.secondary)
                    .padding()
            }
        }
    }
}

struct BlockBalanceChangesView: View {
    let block: BlockBalanceChanges
    let network: DashSDKNetwork
    
    @State private var isExpanded = false
    
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Button(action: { isExpanded.toggle() }) {
                HStack {
                    VStack(alignment: .leading, spacing: 4) {
                        Text("Block \(block.blockHeight)")
                            .font(.subheadline)
                            .fontWeight(.semibold)
                        Text("\(block.changes.count) change(s)")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                    Spacer()
                    Image(systemName: isExpanded ? "chevron.up" : "chevron.down")
                        .foregroundColor(.secondary)
                }
                .padding()
                .background(Color.blue.opacity(0.1))
                .cornerRadius(8)
            }
            .buttonStyle(PlainButtonStyle())
            
            if isExpanded {
                ForEach(Array(block.changes.enumerated()), id: \.offset) { _, change in
                    AddressBalanceChangeView(change: change, network: network)
                }
            }
        }
    }
}

struct AddressBalanceChangeView: View {
    let change: AddressBalanceChange
    let network: DashSDKNetwork
    
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            // Address
            if let bech32m = change.toBech32m(network: network) {
                VStack(alignment: .leading, spacing: 2) {
                    Text("Address")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    Text(bech32m)
                        .font(.system(.caption, design: .monospaced))
                        .lineLimit(1)
                        .truncationMode(.middle)
                }
            } else {
                VStack(alignment: .leading, spacing: 2) {
                    Text("Address (hex)")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    Text(change.addressHex)
                        .font(.system(.caption, design: .monospaced))
                        .lineLimit(1)
                        .truncationMode(.middle)
                }
            }
            
            // Operation
            HStack {
                Text("Operation")
                    .font(.caption)
                    .foregroundColor(.secondary)
                Spacer()
                switch change.operation {
                case .setCredits(let credits):
                    HStack(spacing: 4) {
                        Image(systemName: "equal")
                            .foregroundColor(.orange)
                        Text("Set to \(formatCredits(credits))")
                            .font(.caption)
                            .fontWeight(.medium)
                    }
                case .addToCredits(let credits):
                    HStack(spacing: 4) {
                        Image(systemName: "plus")
                            .foregroundColor(.green)
                        Text("Add \(formatCredits(credits))")
                            .font(.caption)
                            .fontWeight(.medium)
                    }
                }
            }
            
            // Dash equivalent
            HStack {
                Spacer()
                Text("(\(formatDash(change.operation.credits)) DASH)")
                    .font(.caption2)
                    .foregroundColor(.blue)
            }
        }
        .padding()
        .background(Color.gray.opacity(0.05))
        .cornerRadius(8)
        .padding(.leading, 16)
    }
}

// MARK: - Helper Functions

/// Credits per Dash: 1 DASH = 10^11 credits = 100,000,000,000 credits
private let creditsPerDash: UInt64 = 100_000_000_000

/// Format credits with thousands separator
private func formatCredits(_ credits: UInt64) -> String {
    let formatter = NumberFormatter()
    formatter.numberStyle = .decimal
    formatter.groupingSeparator = ","
    return formatter.string(from: NSNumber(value: credits)) ?? "\(credits)"
}

/// Convert credits to Dash with proper decimal formatting
private func formatDash(_ credits: UInt64) -> String {
    let dash = Double(credits) / Double(creditsPerDash)
    let formatter = NumberFormatter()
    formatter.numberStyle = .decimal
    formatter.minimumFractionDigits = 0
    formatter.maximumFractionDigits = 8
    formatter.groupingSeparator = ","
    return formatter.string(from: NSNumber(value: dash)) ?? String(format: "%.8f", dash)
}

// MARK: - Helper Views

struct ResultRow: View {
    let label: String
    let value: String
    
    var body: some View {
        HStack {
            Text(label)
                .font(.subheadline)
                .foregroundColor(.secondary)
            Spacer()
            Text(value)
                .font(.subheadline)
                .fontWeight(.medium)
                .lineLimit(1)
        }
    }
}

/// Balance row showing both credits and Dash
struct BalanceRow: View {
    let label: String
    let credits: UInt64
    
    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            HStack {
                Text(label)
                    .font(.subheadline)
                    .foregroundColor(.secondary)
                Spacer()
            }
            HStack {
                Spacer()
                VStack(alignment: .trailing, spacing: 2) {
                    Text("\(formatCredits(credits)) credits")
                        .font(.subheadline)
                        .fontWeight(.medium)
                    Text("\(formatDash(credits)) DASH")
                        .font(.caption)
                        .foregroundColor(.blue)
                        .fontWeight(.semibold)
                }
            }
        }
    }
}

// MARK: - Previews

struct AddressQueriesView_Previews: PreviewProvider {
    static var previews: some View {
        NavigationView {
            AddressQueriesView()
                .environmentObject(UnifiedAppState())
        }
    }
}
