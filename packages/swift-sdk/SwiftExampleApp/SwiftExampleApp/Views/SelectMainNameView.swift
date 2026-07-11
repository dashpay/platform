import SwiftUI
import SwiftDashSDK

struct SelectMainNameView: View {
    let identity: PersistentIdentity
    @EnvironmentObject var appState: AppState
    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) var dismiss

    @State private var selectedName: String?
    /// DPNS names owned by this identity — loaded on appear from
    /// the owning wallet's `ManagedIdentity`. Empty when the
    /// identity isn't wallet-associated or the wallet hasn't
    /// synced DPNS names yet.
    @State private var dpnsNames: [String] = []
    /// Labels this identity is currently contending for — shown
    /// as read-only information ("contested, cannot be main").
    @State private var contestedDpnsNames: [String] = []

    var availableNames: [String] {
        // Only show non-contested names that the user actually owns.
        dpnsNames
    }

    var body: some View {
        NavigationView {
            Form {
                Section {
                    Text("Select which name to display as your main identity name throughout the app.")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }

                if availableNames.isEmpty {
                    Section {
                        VStack(spacing: 12) {
                            Image(systemName: "exclamationmark.triangle")
                                .font(.largeTitle)
                                .foregroundColor(.orange)
                            Text("No Names Available")
                                .font(.headline)
                            Text("You don't have any registered DPNS names yet. Contested names cannot be selected as main names.")
                                .font(.caption)
                                .foregroundColor(.secondary)
                                .multilineTextAlignment(.center)
                        }
                        .frame(maxWidth: .infinity)
                        .padding(.vertical)
                    }
                } else {
                    Section("Available Names") {
                        // Option to have no main name
                        HStack {
                            Text("None")
                                .foregroundColor(.secondary)
                            Spacer()
                            if selectedName == nil {
                                Image(systemName: "checkmark.circle.fill")
                                    .foregroundColor(.blue)
                            }
                        }
                        .contentShape(Rectangle())
                        .onTapGesture {
                            selectedName = nil
                        }

                        // List all available names
                        ForEach(availableNames, id: \.self) { name in
                            HStack {
                                VStack(alignment: .leading, spacing: 4) {
                                    Text(name)
                                        .font(.headline)
                                    if name == identity.dpnsName {
                                        Text("First registered name")
                                            .font(.caption)
                                            .foregroundColor(.secondary)
                                    }
                                }

                                Spacer()

                                if selectedName == name {
                                    Image(systemName: "checkmark.circle.fill")
                                        .foregroundColor(.blue)
                                }
                            }
                            .contentShape(Rectangle())
                            .onTapGesture {
                                selectedName = name
                            }
                        }
                    }

                    // Show current selection
                    if let currentMain = identity.mainDpnsName {
                        Section("Current Main Name") {
                            HStack {
                                Text(currentMain)
                                    .font(.headline)
                                Spacer()
                                Image(systemName: "star.fill")
                                    .foregroundColor(.yellow)
                            }
                        }
                    }
                }

                // Show contested names as information only
                if !contestedDpnsNames.isEmpty {
                    Section("Contested Names") {
                        ForEach(contestedDpnsNames, id: \.self) { name in
                            HStack {
                                Text(name)
                                    .foregroundColor(.secondary)
                                Spacer()
                                Label("Contested", systemImage: "flag.fill")
                                    .font(.caption)
                                    .foregroundColor(.orange)
                            }
                        }

                        Text("Contested names cannot be selected as main names until they are won.")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                }
            }
            .navigationTitle("Select Main Name")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") {
                        dismiss()
                    }
                }

                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Save") {
                        saveSelection()
                    }
                    .disabled(selectedName == identity.mainDpnsName)
                }
            }
            .onAppear {
                // Initialize with current main name
                selectedName = identity.mainDpnsName
                loadNamesFromWallet()
            }
        }
    }

    /// Populate `dpnsNames` / `contestedDpnsNames` by asking the
    /// owning wallet's `ManagedIdentity`. Skipped silently when
    /// the identity has no wallet association — the view then
    /// shows the "No Names Available" empty state.
    private func loadNamesFromWallet() {
        guard let walletId = identity.wallet?.walletId,
              let wallet = walletManager.wallet(for: walletId)
        else {
            return
        }
        guard let managed = try? wallet.managedIdentity(identityId: identity.identityId) else {
            return
        }
        dpnsNames = (try? managed.getDpnsNames()) ?? []
        contestedDpnsNames = (try? managed.getContestedDpnsNames()) ?? []
    }

    private func saveSelection() {
        // One direct SwiftData write — PersistentIdentity's unique
        // `identityId` lets `@Query` views upstream pick up the
        // change reactively on `save()`.
        PersistentIdentity.updateMainDpnsName(
            in: modelContext,
            identityId: identity.identityId,
            mainDpnsName: selectedName
        )
        try? modelContext.save()
        dismiss()
    }
}
