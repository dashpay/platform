import SwiftUI
import SwiftData

struct FriendsView: View {
    @EnvironmentObject var appState: UnifiedAppState
    @State private var selectedIdentityId: String = ""
    @State private var friends: [Friend] = []
    @State private var isLoading = false
    @State private var showAddFriend = false
    
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
                    
                    // Friends list
                    if friends.isEmpty && !isLoading {
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
                            ProgressView("Loading friends...")
                            Spacer()
                        }
                    } else {
                        List(friends) { friend in
                            FriendRowView(friend: friend)
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
        // TODO: Load friends for the selected identity
        // This would query the platform for contacts/friends associated with this identity
        isLoading = true
        
        // Simulate loading
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) {
            isLoading = false
            // friends = [] // Load actual friends here
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

// Friend model
struct Friend: Identifiable {
    let id = UUID()
    let identityId: String
    let displayName: String
    let dpnsName: String?
    let isOnline: Bool
    let lastSeen: Date?
}

struct FriendRowView: View {
    let friend: Friend
    
    var body: some View {
        HStack {
            // Avatar
            Circle()
                .fill(Color.blue.opacity(0.2))
                .frame(width: 40, height: 40)
                .overlay(
                    Text(friend.displayName.prefix(1).uppercased())
                        .font(.headline)
                        .foregroundColor(.blue)
                )
            
            VStack(alignment: .leading, spacing: 2) {
                HStack {
                    Text(friend.displayName)
                        .font(.headline)
                    
                    if friend.isOnline {
                        Circle()
                            .fill(Color.green)
                            .frame(width: 8, height: 8)
                    }
                }
                
                if let dpnsName = friend.dpnsName {
                    Text(dpnsName)
                        .font(.caption)
                        .foregroundColor(.secondary)
                } else {
                    Text(friend.identityId.prefix(12) + "...")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            
            Spacer()
            
            if let lastSeen = friend.lastSeen, !friend.isOnline {
                Text(lastSeen, style: .relative)
                    .font(.caption2)
                    .foregroundColor(.secondary)
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