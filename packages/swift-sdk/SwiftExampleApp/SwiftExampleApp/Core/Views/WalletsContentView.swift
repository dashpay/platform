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
    @Query private var wallets: [PersistentWallet]
    @State private var showingCreateWallet = false

    /// Wallets on the currently-selected network. The persistent
    /// store keeps every network's rows so flipping back to testnet
    /// re-surfaces what was synced there, but the tab only renders
    /// the wallets tied to `platformState.currentNetwork`. Rows with
    /// `networkRaw == nil` (legacy / not yet hydrated by the
    /// persister) drop out — once the persister fills the column in,
    /// they reappear under the matching network.
    private var visibleWallets: [PersistentWallet] {
        let raw = platformState.currentNetwork.rawValue
        return wallets.filter { $0.networkRaw == raw }
    }

    var body: some View {
        List {
            Section("Wallets (\(platformState.currentNetwork.displayName))") {
                if visibleWallets.isEmpty {
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
                        .accessibilityIdentifier("wallets.empty.createWalletButton")
                    }
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 20)
                } else {
                    ForEach(visibleWallets) { wallet in
                        NavigationLink(value: wallet) {
                            WalletRowView(wallet: wallet)
                        }
                        .accessibilityIdentifier("wallets.walletRow.\(wallet.walletId.toHexString())")
                        .accessibilityLabel(wallet.label)
                    }
                }
            }
        }
        .accessibilityIdentifier("wallets.screen")
        // All navigation pushes from this stack are value-based —
        // closure-based pushes eagerly construct their destination on
        // every parent body invocation (which fires constantly during
        // sync) and stall the click on iOS 26. Value-based push only
        // builds the destination on actual navigate. Both routes have
        // to be declared on the stack root (here) — declaring on a
        // pushed view is unreliable and mixing paradigms (closure
        // outer + value inner) makes the inner destination
        // animate-then-pop.
        .navigationDestination(for: PersistentWallet.self) { wallet in
            WalletDetailView(wallet: wallet)
        }
        .navigationDestination(for: TransactionsRoute.self) { route in
            TransactionListView(walletId: route.walletId)
        }
        .navigationDestination(for: ShieldedActivityRoute.self) { route in
            ShieldedActivityListView(walletId: route.walletId)
        }
        .navigationTitle("Wallets")
        .toolbar {
            ToolbarItem(placement: .navigationBarTrailing) {
                Button {
                    showingCreateWallet = true
                } label: {
                    Image(systemName: "plus")
                }
                .accessibilityIdentifier("wallets.addWalletButton")
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
