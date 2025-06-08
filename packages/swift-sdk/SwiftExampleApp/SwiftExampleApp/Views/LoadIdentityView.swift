import SwiftUI

struct LoadIdentityView: View {
    @EnvironmentObject var appState: AppState
    @Environment(\.dismiss) var dismiss
    
    // Form inputs
    @State private var identityIdInput = ""
    @State private var selectedIdentityType: IdentityType = .user
    @State private var aliasInput = ""
    
    // Masternode/Evonode specific keys
    @State private var votingPrivateKeyInput = ""
    @State private var ownerPrivateKeyInput = ""
    @State private var payoutPrivateKeyInput = ""
    
    // User identity keys
    @State private var privateKeys: [String] = ["", "", ""]
    
    // Loading state
    @State private var isLoading = false
    @State private var errorMessage: String?
    @State private var showSuccess = false
    @State private var loadStartTime: Date?
    
    // Testnet nodes
    private let testnetNodes = TestnetNodesLoader.loadFromYAML()
    
    // Info popups
    @State private var showInfoPopup = false
    @State private var infoPopupMessage = ""
    
    var body: some View {
        NavigationView {
            if showSuccess {
                successView
            } else {
                formView
            }
        }
    }
    
    private var formView: some View {
        Form {
            if appState.sdk?.network?.rawValue == 1 && testnetNodes != nil { // testnet
                Section {
                    HStack {
                        Button("Fill Random HPMN") {
                            fillRandomHPMN()
                        }
                        .buttonStyle(.bordered)
                        
                        Button("Fill Random Masternode") {
                            fillRandomMasternode()
                        }
                        .buttonStyle(.bordered)
                    }
                }
            }
            
            Section("Identity Information") {
                VStack(alignment: .leading) {
                    Text("Identity ID / ProTxHash")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    TextField("Hex or Base58", text: $identityIdInput)
                        .textFieldStyle(RoundedBorderTextFieldStyle())
                }
                
                Picker("Identity Type", selection: $selectedIdentityType) {
                    ForEach(IdentityType.allCases, id: \.self) { type in
                        Text(type.rawValue).tag(type)
                    }
                }
                .pickerStyle(SegmentedPickerStyle())
                
                HStack {
                    VStack(alignment: .leading) {
                        Text("Alias (optional)")
                            .font(.caption)
                            .foregroundColor(.secondary)
                        TextField("Display name", text: $aliasInput)
                            .textFieldStyle(RoundedBorderTextFieldStyle())
                    }
                    
                    Button(action: {
                        infoPopupMessage = "Alias is optional. It is only used to help identify the identity in the app. It isn't saved to Dash Platform."
                        showInfoPopup = true
                    }) {
                        Image(systemName: "info.circle")
                            .foregroundColor(.blue)
                    }
                }
            }
            
            // Show appropriate key inputs based on identity type
            if selectedIdentityType == .masternode || selectedIdentityType == .evonode {
                masternodeKeyInputs
            } else {
                userKeyInputs
            }
            
            if let errorMessage = errorMessage {
                Section {
                    Text(errorMessage)
                        .foregroundColor(.red)
                }
            }
            
            Section {
                loadIdentityButton
            }
        }
        .navigationTitle("Load Identity")
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .navigationBarLeading) {
                Button("Cancel") {
                    dismiss()
                }
            }
        }
        .disabled(isLoading)
        .sheet(isPresented: $showInfoPopup) {
            InfoPopupView(message: infoPopupMessage)
        }
    }
    
    private var masternodeKeyInputs: some View {
        Section("Masternode Keys") {
            VStack(alignment: .leading) {
                Text("Voting Private Key")
                    .font(.caption)
                    .foregroundColor(.secondary)
                TextField("Hex or WIF", text: $votingPrivateKeyInput)
                    .textFieldStyle(RoundedBorderTextFieldStyle())
            }
            
            VStack(alignment: .leading) {
                Text("Owner Private Key")
                    .font(.caption)
                    .foregroundColor(.secondary)
                TextField("Hex or WIF", text: $ownerPrivateKeyInput)
                    .textFieldStyle(RoundedBorderTextFieldStyle())
            }
            
            if selectedIdentityType == .evonode {
                VStack(alignment: .leading) {
                    Text("Payout Address Private Key")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    TextField("Hex or WIF", text: $payoutPrivateKeyInput)
                        .textFieldStyle(RoundedBorderTextFieldStyle())
                }
            }
        }
    }
    
    private var userKeyInputs: some View {
        Section("Private Keys") {
            ForEach(privateKeys.indices, id: \.self) { index in
                HStack {
                    VStack(alignment: .leading) {
                        HStack {
                            Text("Private Key \(index + 1)")
                                .font(.caption)
                                .foregroundColor(.secondary)
                            
                            Button(action: {
                                infoPopupMessage = "You don't need to add all or even any private keys here. Private keys can be added later. However, without private keys, you won't be able to sign any transactions."
                                showInfoPopup = true
                            }) {
                                Image(systemName: "info.circle")
                                    .font(.caption)
                                    .foregroundColor(.blue)
                            }
                        }
                        
                        TextField("Hex or WIF", text: $privateKeys[index])
                            .textFieldStyle(RoundedBorderTextFieldStyle())
                    }
                    
                    if privateKeys.count > 1 {
                        Button(action: {
                            privateKeys.remove(at: index)
                        }) {
                            Image(systemName: "minus.circle.fill")
                                .foregroundColor(.red)
                        }
                    }
                }
            }
            
            Button(action: {
                privateKeys.append("")
            }) {
                Label("Add Key", systemImage: "plus.circle.fill")
            }
        }
    }
    
    private var loadIdentityButton: some View {
        Button(action: loadIdentity) {
            HStack {
                if isLoading {
                    ProgressView()
                        .progressViewStyle(CircularProgressViewStyle())
                        .scaleEffect(0.8)
                } else {
                    Text("Load Identity")
                }
                
                if let startTime = loadStartTime {
                    let elapsed = Date().timeIntervalSince(startTime)
                    Text(formatElapsedTime(elapsed))
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            .frame(maxWidth: .infinity)
        }
        .buttonStyle(.borderedProminent)
        .disabled(identityIdInput.isEmpty || isLoading)
    }
    
    private var successView: some View {
        VStack(spacing: 20) {
            Spacer()
            
            Image(systemName: "checkmark.circle.fill")
                .font(.system(size: 80))
                .foregroundColor(.green)
            
            Text("Successfully loaded identity!")
                .font(.title2)
                .fontWeight(.semibold)
            
            VStack(spacing: 10) {
                Button("Load Another") {
                    resetForm()
                    showSuccess = false
                }
                .buttonStyle(.borderedProminent)
                
                Button("Back to Identities") {
                    dismiss()
                }
                .buttonStyle(.bordered)
            }
            
            Spacer()
        }
        .padding()
        .navigationTitle("Success")
        .navigationBarTitleDisplayMode(.inline)
    }
    
    // MARK: - Actions
    
    private func loadIdentity() {
        errorMessage = nil
        isLoading = true
        loadStartTime = Date()
        
        Task {
            do {
                // Create the identity model
                let identity = IdentityModel(
                    id: identityIdInput.trimmingCharacters(in: .whitespacesAndNewlines),
                    balance: 0,
                    isLocal: true,
                    alias: aliasInput.isEmpty ? nil : aliasInput,
                    type: selectedIdentityType,
                    privateKeys: privateKeys.filter { !$0.isEmpty },
                    votingPrivateKey: votingPrivateKeyInput.isEmpty ? nil : votingPrivateKeyInput,
                    ownerPrivateKey: ownerPrivateKeyInput.isEmpty ? nil : ownerPrivateKeyInput,
                    payoutPrivateKey: payoutPrivateKeyInput.isEmpty ? nil : payoutPrivateKeyInput
                )
                
                // In a real app, we would verify the identity exists on the network
                // For now, we'll simulate a network call
                try await Task.sleep(nanoseconds: 2_000_000_000) // 2 second delay
                
                // Add to app state
                await MainActor.run {
                    appState.addIdentity(identity)
                    showSuccess = true
                }
            } catch {
                await MainActor.run {
                    errorMessage = error.localizedDescription
                }
            }
            
            await MainActor.run {
                isLoading = false
                loadStartTime = nil
            }
        }
    }
    
    private func fillRandomHPMN() {
        guard let nodes = testnetNodes?.hpMasternodes.randomElement() else { return }
        
        let (name, hpmn) = nodes
        identityIdInput = hpmn.protxTxHash
        selectedIdentityType = .evonode
        aliasInput = name
        votingPrivateKeyInput = hpmn.voter.privateKey
        ownerPrivateKeyInput = hpmn.owner.privateKey
        payoutPrivateKeyInput = hpmn.payout.privateKey
    }
    
    private func fillRandomMasternode() {
        guard let nodes = testnetNodes?.masternodes.randomElement() else { return }
        
        let (name, masternode) = nodes
        identityIdInput = masternode.proTxHash
        selectedIdentityType = .masternode
        aliasInput = name
        votingPrivateKeyInput = masternode.voter.privateKey
        ownerPrivateKeyInput = masternode.owner.privateKey
        payoutPrivateKeyInput = ""
    }
    
    private func resetForm() {
        identityIdInput = ""
        selectedIdentityType = .user
        aliasInput = ""
        votingPrivateKeyInput = ""
        ownerPrivateKeyInput = ""
        payoutPrivateKeyInput = ""
        privateKeys = ["", "", ""]
        errorMessage = nil
    }
    
    private func formatElapsedTime(_ seconds: TimeInterval) -> String {
        let intSeconds = Int(seconds)
        if intSeconds < 60 {
            return "\(intSeconds)s"
        } else {
            let minutes = intSeconds / 60
            let remainingSeconds = intSeconds % 60
            return "\(minutes)m \(remainingSeconds)s"
        }
    }
}

struct InfoPopupView: View {
    let message: String
    @Environment(\.dismiss) var dismiss
    
    var body: some View {
        NavigationView {
            VStack(spacing: 20) {
                Text(message)
                    .padding()
                
                Button("Close") {
                    dismiss()
                }
                .buttonStyle(.borderedProminent)
            }
            .padding()
            .navigationTitle("Information")
            .navigationBarTitleDisplayMode(.inline)
        }
    }
}