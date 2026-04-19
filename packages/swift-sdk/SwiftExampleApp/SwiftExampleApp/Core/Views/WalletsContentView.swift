// WalletsContentView.swift
// SwiftExampleApp
//
// Combined wallets + identities screen (Tab 2).

import SwiftUI
import SwiftDashSDK
import SwiftData

struct WalletsContentView: View {
    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var platformState: AppState
    @EnvironmentObject var platformBalanceSyncService: PlatformBalanceSyncService
    @Environment(\.modelContext) private var modelContext
    @Query private var wallets: [HDWallet]
    @State private var showingCreateWallet = false
    @State private var showingLoadIdentity = false
    @State private var showingCreateIdentity = false

    var body: some View {
        List {
            // Section 1: Wallets
            Section("Wallets (\(platformState.currentNetwork.displayName))") {
                if wallets.isEmpty {
                    VStack(spacing: 12) {
                        Image(systemName: "wallet.pass")
                            .font(.system(size: 40))
                            .foregroundColor(.gray)

                        Text("No \(platformState.currentNetwork.displayName) Wallets")
                            .font(.headline)

                        Text("Create a wallet to get started")
                            .font(.caption)
                            .foregroundColor(.secondary)

                        Button {
                            showingCreateWallet = true
                        } label: {
                            Text("Create Wallet")
                                .foregroundColor(.white)
                                .padding(.horizontal, 16)
                                .padding(.vertical, 8)
                                .background(Color.blue)
                                .cornerRadius(8)
                        }
                    }
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 20)
                } else {
                    ForEach(wallets) { wallet in
                        NavigationLink {
                            WalletDetailView(wallet: wallet)
                        } label: {
                            WalletRowView(wallet: wallet)
                        }
                    }
                }
            }

            // Section 2: Identities
            Section("Identities") {
                if platformState.identities.isEmpty {
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
                } else {
                    ForEach(platformState.identities) { identity in
                        IdentityRow(identity: identity)
                            .environmentObject(platformState)
                    }
                }
            }
        }
        .navigationTitle("Wallets")
        .toolbar {
            ToolbarItem(placement: .navigationBarTrailing) {
                Menu {
                    Button {
                        showingCreateWallet = true
                    } label: {
                        Label("Create Wallet", systemImage: "plus")
                    }

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
                } label: {
                    Image(systemName: "plus")
                }
            }
        }
        .sheet(isPresented: $showingCreateWallet) {
            NavigationStack {
                CreateWalletView()
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
        .refreshable {
            await platformBalanceSyncService.performSync()
        }
    }
}
