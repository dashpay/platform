// WalletsContentView.swift
// SwiftExampleApp
//
// Combined wallets + identities screen (Tab 2).

import SwiftUI
import SwiftDashSDK
import SwiftData

struct WalletsContentView: View {
    @EnvironmentObject var walletService: WalletService
    @EnvironmentObject var unifiedAppState: UnifiedAppState
    @EnvironmentObject var platformBalanceSyncService: PlatformBalanceSyncService
    @Environment(\.modelContext) private var modelContext
    @Query private var wallets: [HDWallet]
    @State private var showingCreateWallet = false
    @State private var showingLoadIdentity = false

    var body: some View {
        List {
            // Section 1: Wallets
            Section("Wallets (\(unifiedAppState.platformState.currentNetwork.displayName))") {
                if wallets.isEmpty {
                    VStack(spacing: 12) {
                        Image(systemName: "wallet.pass")
                            .font(.system(size: 40))
                            .foregroundColor(.gray)

                        Text("No \(unifiedAppState.platformState.currentNetwork.displayName) Wallets")
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
                                .environmentObject(unifiedAppState)
                        } label: {
                            WalletRowView(wallet: wallet)
                        }
                    }
                }
            }

            // Section 2: Identities
            Section("Identities") {
                if unifiedAppState.platformState.identities.isEmpty {
                    VStack(spacing: 12) {
                        Image(systemName: "person.crop.circle.badge.plus")
                            .font(.system(size: 40))
                            .foregroundColor(.gray)

                        Text("No Identities")
                            .font(.headline)

                        Text("Load an identity to interact with Dash Platform")
                            .font(.caption)
                            .foregroundColor(.secondary)

                        Button {
                            showingLoadIdentity = true
                        } label: {
                            Label("Load Identity", systemImage: "square.and.arrow.down")
                                .padding(.horizontal, 16)
                                .padding(.vertical, 8)
                        }
                        .buttonStyle(.borderedProminent)
                    }
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 20)
                } else {
                    ForEach(unifiedAppState.platformState.identities) { identity in
                        IdentityRow(identity: identity)
                            .environmentObject(unifiedAppState.platformState)
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
                    .environmentObject(walletService)
                    .environmentObject(unifiedAppState)
                    .environment(\.modelContext, modelContext)
            }
        }
        .sheet(isPresented: $showingLoadIdentity) {
            LoadIdentityView()
                .environmentObject(unifiedAppState.platformState)
        }
        .refreshable {
            await unifiedAppState.performPlatformBalanceSync()
        }
    }
}
