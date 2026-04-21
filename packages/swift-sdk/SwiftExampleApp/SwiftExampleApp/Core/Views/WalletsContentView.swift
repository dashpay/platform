// WalletsContentView.swift
// SwiftExampleApp
//
// Wallets-only tab content. Identities live in the sibling
// `IdentitiesContentView` tab so users navigate to the right place
// instead of scrolling through a combined list.

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

    var body: some View {
        List {
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
        }
        .navigationTitle("Wallets")
        .toolbar {
            ToolbarItem(placement: .navigationBarTrailing) {
                Button {
                    showingCreateWallet = true
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
        .refreshable {
            await platformBalanceSyncService.performSync()
        }
    }
}
