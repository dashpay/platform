import SwiftUI
import SwiftData
import SwiftDashSDK

struct FriendsView: View {
    @EnvironmentObject var appState: UnifiedAppState
    @StateObject private var dashPayService = DashPayService()
    @State private var selectedIdentityId: String = ""
    @State private var contacts: [DashPayContact] = []
    @State private var incomingRequests: [DashPayContactRequest] = []
    @State private var sentRequests: [DashPayContactRequest] = []
    @State private var isLoading = false
    @State private var showAddFriend = false
    @State private var showIncomingRequests = false
    @State private var errorMessage: String?
    
    var availableIdentities: [IdentityModel] {
        appState.platformState.identities
    }
    
    var selectedIdentity: IdentityModel? {
        availableIdentities.first { $0.idString == selectedIdentityId }
    }
    
    var body: some View {
        NavigationStack {
            if availableIdentities.isEmpty {
                // No identities view
                VStack(spacing: 20) {
                    Spacer()
                    
                    Image(systemName: "person.crop.circle.badge.exclamationmark")
                        .font(.system(size: 60))
                        .foregroundColor(.gray)
                    
                    Text("No Identity Found")
                        .font(.title2)
                        .fontWeight(.semibold)
                    
                    Text("Please create or load an identity first\nto manage your friends")
                        .multilineTextAlignment(.center)
                        .foregroundColor(.secondary)
                    
                    HStack(spacing: 20) {
                        NavigationLink(destination: LoadIdentityView()) {
                            Label("Load Identity", systemImage: "square.and.arrow.down")
                                .frame(maxWidth: .infinity)
                        }
                        .buttonStyle(.bordered)
                        
                        NavigationLink(destination: TransitionDetailView(transitionKey: "identityCreate", transitionLabel: "Create Identity")) {
                            Label("Create Identity", systemImage: "plus.circle")
                                .frame(maxWidth: .infinity)
                        }
                        .buttonStyle(.borderedProminent)
                    }
                    .padding(.horizontal)
                    
                    Spacer()
                }
                .navigationTitle("Friends")
                .navigationBarTitleDisplayMode(.large)
            } else {
                VStack(spacing: 0) {
                    // Identity selector
                    VStack(spacing: 0) {
                        HStack {
                            Text("Selected Identity")
                                .font(.caption)
                                .foregroundColor(.secondary)
                            Spacer()
                        }
                        .padding(.horizontal)
                        .padding(.top, 8)
                        
                        Picker("Identity", selection: $selectedIdentityId) {
                            ForEach(availableIdentities) { identity in
                                HStack {
                                    VStack(alignment: .leading) {
                                        Text(identity.alias ?? "Identity")
                                            .font(.headline)
                                        Text(identity.idString.prefix(12) + "...")
                                            .font(.caption)
                                            .foregroundColor(.secondary)
                                    }
                                    Spacer()
                                    if identity.balance > 0 {
                                        Text(formatBalance(identity.balance))
                                            .font(.caption)
                                            .foregroundColor(.blue)
                                    }
                                }
                                .tag(identity.idString)
                            }
                        }
                        .pickerStyle(.menu)
                        .padding(.horizontal)
                        .padding(.bottom, 8)
                        .background(Color(UIColor.secondarySystemBackground))
                    }
                    
                    // Incoming requests section
                    if !incomingRequests.isEmpty {
                        Section {
                            ForEach(incomingRequests) { request in
                                ContactRequestRow(request: request, isIncoming: true) {
                                    acceptRequest(request)
                                } onReject: {
                                    rejectRequest(request)
                                }
                            }
                        } header: {
                            Text("Incoming Requests (\(incomingRequests.count))")
                        }
                    }

                    // Friends list
                    if contacts.isEmpty && !isLoading && incomingRequests.isEmpty {
                        VStack(spacing: 20) {
                            Spacer()

                            Image(systemName: "person.2.slash")
                                .font(.system(size: 50))
                                .foregroundColor(.gray)

                            Text("No Friends Yet")
                                .font(.title3)
                                .fontWeight(.medium)

                            Text("Add friends to send messages\nand share documents")
                                .multilineTextAlignment(.center)
                                .font(.caption)
                                .foregroundColor(.secondary)

                            Button {
                                showAddFriend = true
                            } label: {
                                Label("Add Friend", systemImage: "person.badge.plus")
                            }
                            .buttonStyle(.borderedProminent)

                            Spacer()
                        }
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                    } else if isLoading {
                        VStack {
                            Spacer()
                            ProgressView("Loading contacts...")
                            Spacer()
                        }
                    } else {
                        List {
                            ForEach(contacts.filter { !$0.isHidden }) { contact in
                                ContactRowView(contact: contact)
                            }
                        }
                    }
                }
                .navigationTitle("Friends")
                .navigationBarTitleDisplayMode(.large)
                .toolbar {
                    ToolbarItem(placement: .navigationBarTrailing) {
                        Button {
                            showAddFriend = true
                        } label: {
                            Image(systemName: "person.badge.plus")
                        }
                    }
                }
                .sheet(isPresented: $showAddFriend) {
                    AddFriendView(selectedIdentity: selectedIdentity)
                }
                .onAppear {
                    // Set initial selected identity if not set
                    if selectedIdentityId.isEmpty && !availableIdentities.isEmpty {
                        selectedIdentityId = availableIdentities[0].idString
                    }
                }
                .onChange(of: selectedIdentityId) { _, newValue in
                    loadFriends()
                }
            }
        }
    }
    
    private func loadFriends() {
        guard selectedIdentity != nil else { return }

        isLoading = true

        Task {
            // Load the managed identity for this identity
            // In a real implementation, you would serialize the identity to bytes
            // For now, we'll skip this and show the pattern

            // If we had a ManagedIdentity:
            // let establishedContacts = try dashPayService.getEstablishedContacts(identity: managedIdentity)
            // let incoming = try dashPayService.getIncomingContactRequests(identity: managedIdentity)
            // let sent = try dashPayService.getSentContactRequests(identity: managedIdentity)

            // For now, show empty state
            await MainActor.run {
                contacts = []
                incomingRequests = []
                sentRequests = []
                isLoading = false
            }
        }
    }

    private func acceptRequest(_ request: DashPayContactRequest) {
        guard selectedIdentity != nil else { return }

        Task {
            // In real implementation:
            // try await dashPayService.acceptContactRequest(identity: managedIdentity, from: request.senderId)
            loadFriends()
        }
    }

    private func rejectRequest(_ request: DashPayContactRequest) {
        guard selectedIdentity != nil else { return }

        Task {
            // In real implementation:
            // try await dashPayService.rejectContactRequest(identity: managedIdentity, from: request.senderId)
            loadFriends()
        }
    }
    
    private func formatBalance(_ amount: UInt64) -> String {
        let dash = Double(amount) / 100_000_000.0
        
        if dash == 0 {
            return "0 DASH"
        }
        
        let formatter = NumberFormatter()
        formatter.minimumFractionDigits = 0
        formatter.maximumFractionDigits = 8
        formatter.numberStyle = .decimal
        formatter.groupingSeparator = ","
        formatter.decimalSeparator = "."
        
        if let formatted = formatter.string(from: NSNumber(value: dash)) {
            return formatted
        }
        
        return String(format: "%.8f", dash)
    }
}

// MARK: - Contact Row View

struct ContactRowView: View {
    let contact: DashPayContact

    var body: some View {
        HStack {
            // Avatar
            Circle()
                .fill(Color.blue.opacity(0.2))
                .frame(width: 40, height: 40)
                .overlay(
                    Text(contact.displayName.prefix(1).uppercased())
                        .font(.headline)
                        .foregroundColor(.blue)
                )

            VStack(alignment: .leading, spacing: 2) {
                Text(contact.displayName)
                    .font(.headline)

                if let dpnsName = contact.dpnsName {
                    Text(dpnsName)
                        .font(.caption)
                        .foregroundColor(.secondary)
                } else {
                    Text(contact.id.hexString.prefix(12) + "...")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }

                if let note = contact.note {
                    Text(note)
                        .font(.caption2)
                        .foregroundColor(.secondary)
                        .lineLimit(1)
                }
            }

            Spacer()
        }
        .padding(.vertical, 4)
    }
}

// MARK: - Contact Request Row View

struct ContactRequestRow: View {
    let request: DashPayContactRequest
    let isIncoming: Bool
    let onAccept: () -> Void
    let onReject: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                VStack(alignment: .leading) {
                    Text(isIncoming ? "From" : "To")
                        .font(.caption)
                        .foregroundColor(.secondary)

                    Text((isIncoming ? request.senderId : request.recipientId).hexString.prefix(12) + "...")
                        .font(.subheadline)
                        .fontWeight(.medium)
                }

                Spacer()

                Text(request.createdAt, style: .relative)
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }

            if isIncoming {
                HStack(spacing: 12) {
                    Button("Accept") {
                        onAccept()
                    }
                    .buttonStyle(.borderedProminent)
                    .controlSize(.small)

                    Button("Reject") {
                        onReject()
                    }
                    .buttonStyle(.bordered)
                    .controlSize(.small)
                    .tint(.red)
                }
            }
        }
        .padding(.vertical, 4)
    }
}

struct AddFriendView: View {
    let selectedIdentity: IdentityModel?
    @Environment(\.dismiss) private var dismiss
    @State private var searchText = ""
    @State private var searchMethod = 0 // 0: DPNS, 1: Identity ID
    
    var body: some View {
        NavigationStack {
            VStack {
                Picker("Search by", selection: $searchMethod) {
                    Text("DPNS Name").tag(0)
                    Text("Identity ID").tag(1)
                }
                .pickerStyle(.segmented)
                .padding()
                
                Form {
                    Section {
                        TextField(
                            searchMethod == 0 ? "Enter DPNS name" : "Enter Identity ID",
                            text: $searchText
                        )
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                    } header: {
                        Text(searchMethod == 0 ? "DPNS Name" : "Identity ID")
                    } footer: {
                        Text(searchMethod == 0 ? 
                            "Search for friends by their Dash Platform Name Service (DPNS) username" :
                            "Search for friends by their unique identity identifier")
                    }
                    
                    Section {
                        Button {
                            // TODO: Implement friend search and add
                            dismiss()
                        } label: {
                            HStack {
                                Spacer()
                                Label("Search & Add", systemImage: "magnifyingglass")
                                Spacer()
                            }
                        }
                        .disabled(searchText.isEmpty)
                    }
                }
            }
            .navigationTitle("Add Friend")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") {
                        dismiss()
                    }
                }
            }
        }
    }
}

#Preview {
    FriendsView()
        .environmentObject(UnifiedAppState())
}