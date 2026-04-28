// IdentitiesContentView.swift
// SwiftExampleApp
//
// Identities-only tab content. Split off from the old combined
// Wallets+Identities screen so each concern gets its own root tab.

import SwiftUI
import SwiftDashSDK
import SwiftData

struct IdentitiesContentView: View {
    @EnvironmentObject var platformState: AppState
    @EnvironmentObject var platformBalanceSyncService: PlatformBalanceSyncService
    @Environment(\.modelContext) private var modelContext
    @Query(sort: \PersistentIdentity.identityIndex)
    private var identities: [PersistentIdentity]
    @State private var showingLoadIdentity = false
    @State private var showingCreateIdentity = false
    @State private var showingSearchWallets = false
    /// Identity targeted by a pending Remove swipe. Non-nil presents
    /// the confirmation dialog below. Stored as a reference rather than
    /// an id so the dialog can show the display name / truncated id
    /// without a second fetch.
    @State private var identityPendingRemoval: PersistentIdentity?

    var body: some View {
        List {
            if identities.isEmpty {
                Section {
                    VStack(spacing: 12) {
                        Image(systemName: "person.crop.circle.badge.plus")
                            .font(.system(size: 40))
                            .foregroundColor(.gray)

                        Text("No Identities")
                            .font(.headline)

                        Text("Load an identity to interact with Dash Platform")
                            .font(.caption)
                            .foregroundColor(.secondary)

                        HStack(spacing: 12) {
                            Button {
                                showingCreateIdentity = true
                            } label: {
                                Label("Create Identity", systemImage: "person.crop.circle.badge.plus")
                                    .padding(.horizontal, 16)
                                    .padding(.vertical, 8)
                            }
                            .buttonStyle(.borderedProminent)

                            Button {
                                showingLoadIdentity = true
                            } label: {
                                Label("Load Identity", systemImage: "square.and.arrow.down")
                                    .padding(.horizontal, 16)
                                    .padding(.vertical, 8)
                            }
                            .buttonStyle(.bordered)
                        }
                    }
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 20)
                }
            } else {
                Section("Identities") {
                    ForEach(identities) { identity in
                        IdentityRow(identity: identity)
                            .environmentObject(platformState)
                            // `allowsFullSwipe: false` so the user
                            // can't accidentally bypass the
                            // confirmation with a long swipe. We
                            // deliberately avoid `role: .destructive`
                            // on the swipe button: SwiftUI animates
                            // destructive swipe buttons as if the row
                            // is already gone the moment they're
                            // tapped, and when the confirmation
                            // dialog is dismissed the row "pops
                            // back" because the underlying @Query
                            // still yields it. Painting red via
                            // `.tint` keeps the look without that
                            // pre-commit animation.
                            .swipeActions(edge: .trailing, allowsFullSwipe: false) {
                                Button {
                                    identityPendingRemoval = identity
                                } label: {
                                    Label("Remove", systemImage: "trash")
                                }
                                .tint(.red)
                            }
                    }
                }
            }
        }
        .confirmationDialog(
            removalDialogTitle,
            isPresented: Binding(
                get: { identityPendingRemoval != nil },
                set: { newValue in
                    if !newValue { identityPendingRemoval = nil }
                }
            ),
            titleVisibility: .visible,
            presenting: identityPendingRemoval
        ) { identity in
            Button("Remove from Device", role: .destructive) {
                removeIdentityLocally(identity)
                identityPendingRemoval = nil
            }
            Button("Keep on Device", role: .cancel) {
                identityPendingRemoval = nil
            }
        } message: { _ in
            Text(
                "This only deletes the local copy. The identity remains on the Dash Platform network. You can reload it later if you still have the keys."
            )
        }
        .navigationTitle("Identities")
        .toolbar {
            ToolbarItem(placement: .navigationBarTrailing) {
                Menu {
                    Button {
                        showingCreateIdentity = true
                    } label: {
                        Label("Create Identity", systemImage: "person.crop.circle.badge.plus")
                    }

                    Button {
                        showingLoadIdentity = true
                    } label: {
                        Label("Load Identity", systemImage: "square.and.arrow.down")
                    }

                    Button {
                        showingSearchWallets = true
                    } label: {
                        Label("Search Wallets for Identities", systemImage: "magnifyingglass")
                    }
                    .accessibilityIdentifier("identities.searchWalletsMenuItem")
                } label: {
                    Image(systemName: "plus")
                }
                .accessibilityIdentifier("identities.addMenu")
            }
        }
        .sheet(isPresented: $showingLoadIdentity) {
            LoadIdentityView()
                .environmentObject(platformState)
        }
        .sheet(isPresented: $showingCreateIdentity) {
            CreateIdentityView()
                .environmentObject(platformState)
        }
        .sheet(isPresented: $showingSearchWallets) {
            SearchWalletsForIdentitiesView()
        }
        .refreshable {
            await platformBalanceSyncService.performSync()
        }
    }

    /// Short title for the removal confirmation dialog. Uses
    /// `displayName` so the user sees the DPNS name / alias when
    /// available, and a truncated-hex id otherwise — matches the
    /// label shown in the row itself.
    private var removalDialogTitle: String {
        guard let identity = identityPendingRemoval else {
            return "Remove identity from this device?"
        }
        return "Remove \(identity.displayName) from this device?"
    }

    /// Delete a single identity locally. This does **not** touch the
    /// Dash Platform network — the identity on-chain is untouched and
    /// can be reloaded later if the keys survive (either in the
    /// originating wallet mnemonic or via an import flow).
    ///
    /// Keychain cleanup policy — we walk the identity's public keys
    /// and delete each entry by the stored
    /// `privateKeyKeychainIdentifier` account string. That covers
    /// both write formats the app uses (see `KeychainManager`):
    ///   * `privkey_<identityHex>_<keyIndex>` — legacy direct storage
    ///     via `storePrivateKey`.
    ///   * `identity_privkey.<derivationPath>` — wallet-derived
    ///     entries written by the Rust persister callback.
    /// Because the cascade delete on `PersistentPublicKey` rows runs
    /// *after* we've harvested these strings, the keychain entries
    /// would otherwise orphan (the legacy `deleteAllPrivateKeys(for:)`
    /// filter only matches the `privkey_…_` prefix and would miss
    /// derivation-path entries). We also clear the three special-key
    /// identifiers stored directly on the identity
    /// (`votingPrivateKeyIdentifier`, `ownerPrivateKeyIdentifier`,
    /// `payoutPrivateKeyIdentifier`) since those are likewise keyed
    /// by string identifier.
    ///
    /// After keychain cleanup the SwiftData cascade does the rest:
    /// `@Relationship(.cascade)` on `publicKeys` / `documents` drops
    /// those child rows, `@Relationship(.nullify)` on `tokenBalances`
    /// severs the link, and the `PersistentWallet.identities` inverse
    /// (declared on the wallet side, also `.nullify`) naturally drops
    /// this identity from its owning wallet — no manual detachment
    /// required.
    private func removeIdentityLocally(_ identity: PersistentIdentity) {
        let keychain = KeychainManager.shared

        for publicKey in identity.publicKeys {
            if let identifier = publicKey.privateKeyKeychainIdentifier {
                _ = keychain.deleteKeyData(identifier: identifier)
            }
        }

        if let votingIdentifier = identity.votingPrivateKeyIdentifier {
            _ = keychain.deleteKeyData(identifier: votingIdentifier)
        }
        if let ownerIdentifier = identity.ownerPrivateKeyIdentifier {
            _ = keychain.deleteKeyData(identifier: ownerIdentifier)
        }
        if let payoutIdentifier = identity.payoutPrivateKeyIdentifier {
            _ = keychain.deleteKeyData(identifier: payoutIdentifier)
        }

        modelContext.delete(identity)
        do {
            try modelContext.save()
        } catch {
            platformState.showError(
                message: "Failed to remove identity from device: \(error.localizedDescription)"
            )
        }
    }
}
