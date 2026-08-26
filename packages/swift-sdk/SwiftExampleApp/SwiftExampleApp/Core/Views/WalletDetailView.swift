import SwiftUI
import SwiftDashSDK
import SwiftData
import DashSDKFFI
import LocalAuthentication

/// Routes for value-based navigation from the wallets tab. All
/// pushes go through `.navigationDestination(for:)` modifiers
/// on the stack root (see `WalletsContentView`) — this avoids
/// closure-based `NavigationLink { Destination }` which on iOS 26
/// (a) eagerly constructs the destination on every parent body
/// invocation, stalling the click when the destination has any
/// meaningful `init`, and (b) when mixed with value-based pushes
/// further down the stack, makes SwiftUI animate-then-pop the
/// inner destination because the stack identity is split across
/// paradigms. Going value-based all the way fixes both.
struct TransactionsRoute: Hashable {
    let walletId: Data
}

struct WalletDetailView: View {
    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var platformState: AppState
    @EnvironmentObject var appUIState: AppUIState
    @EnvironmentObject var shieldedService: ShieldedService
    @Environment(\.dismiss) private var dismiss
    let wallet: PersistentWallet
    @State private var showReceiveAddress = false
    @State private var showSendTransaction = false
    @State private var showWalletInfo = false
    @State private var showFundPlatformAddress = false
    @State private var showTransferPlatformAddress = false
    @State private var showWithdrawPlatformAddress = false
    @State private var showShieldFromAssetLock = false
    /// Devnet/testnet-only shielded pool seeding sheet (Seed Pool Notes).
    @State private var showSeedShieldedPool = false
    /// Set by `PendingPlatformFundFromAssetLocksList`'s Resume tap.
    @State private var resumingAssetLock: PersistentAssetLock?

    @Query private var walletAssetLocks: [PersistentAssetLock]

    // Badge count for "View All Transactions". Transactions are no
    // longer wallet-scoped (the same on-chain tx can land in
    // multiple accounts / wallets), so we can't filter
    // `PersistentTransaction` by walletId directly. We query the
    // wallet's TXOs instead and count the distinct creating-or-
    // spending transactions in the body, then union in each account's
    // payload-only `involvedTransactions` — same union the list view
    // uses.
    @Query private var walletTxos: [PersistentTxo]
    /// This wallet's accounts, for the payload-only
    /// `involvedTransactions` contribution to `transactionCount` —
    /// special txs that matched an account by payload with no TXO,
    /// which the `walletTxos` join can't see. Scoped to this wallet
    /// row's network as well as its walletId: the same 32-byte
    /// walletId legitimately exists once per network (same mnemonic
    /// imported on mainnet and testnet), and matching on walletId
    /// alone would fold the sibling network's payload-only txs into
    /// this wallet's badge.
    @Query private var walletAccounts: [PersistentAccount]

    init(wallet: PersistentWallet) {
        self.wallet = wallet
        let walletId = wallet.walletId
        let networkRaw = wallet.networkRaw
        var descriptor = FetchDescriptor<PersistentTxo>(
            predicate: #Predicate { $0.walletId == walletId }
        )
        descriptor.propertiesToFetch = [\.walletId]
        _walletTxos = Query(descriptor)
        _walletAccounts = Query(
            filter: #Predicate<PersistentAccount> {
                $0.wallet.walletId == walletId
                    && $0.wallet.networkRaw == networkRaw
            }
        )
        _walletAssetLocks = Query(
            filter: PersistentAssetLock.predicate(walletId: walletId),
            sort: [SortDescriptor(\PersistentAssetLock.updatedAt, order: .reverse)]
        )
    }

    private var transactionCount: Int {
        var seen: Set<Data> = []
        for txo in walletTxos {
            if let tx = txo.transaction { seen.insert(tx.txid) }
            if let spending = txo.spendingTransaction { seen.insert(spending.txid) }
        }
        // Payload-only involvement: special txs matched by payload with
        // no TXO in the wallet, invisible to the `walletTxos` join.
        for account in walletAccounts {
            for tx in account.involvedTransactions { seen.insert(tx.txid) }
        }
        return seen.count
    }

    var body: some View {
        VStack(spacing: 0) {
            // Network indicator
            HStack {
                Label(platformState.currentNetwork.displayName, systemImage: "network")
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .padding(.horizontal, 12)
                    .padding(.vertical, 6)
                    .background(Color(UIColor.tertiarySystemBackground))
                    .cornerRadius(8)
                Spacer()
            }
            .padding(.horizontal)
            .padding(.top, 8)

            // Balance Card
            BalanceCardView(
                wallet: wallet,
                onFundPlatform: { showFundPlatformAddress = true },
                onTransferPlatform: { showTransferPlatformAddress = true },
                onWithdrawPlatform: { showWithdrawPlatformAddress = true },
                onFundShielded: { showShieldFromAssetLock = true }
            )
                .padding()

            // Action Buttons
            HStack(spacing: 16) {
                Button {
                    showSendTransaction = true
                } label: {
                    Label("Send", systemImage: "arrow.up.circle.fill")
                        .frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)

                Button {
                    showReceiveAddress = true
                } label: {
                    Label("Receive", systemImage: "arrow.down.circle.fill")
                        .frame(maxWidth: .infinity)
                }
                .buttonStyle(.bordered)
            }
            .padding(.horizontal)

            // Devnet/testnet-only: seed the shielded pool's anonymity set
            // so outgoing shielded transitions clear the 250-note minimum.
            // Hidden on mainnet (the pool is seeded at genesis there, and
            // the Rust side hard-errors on mainnet anyway).
            if platformState.currentNetwork != .mainnet {
                Button {
                    showSeedShieldedPool = true
                } label: {
                    Label("Seed Pool Notes", systemImage: "square.stack.3d.up.fill")
                        .frame(maxWidth: .infinity)
                }
                .buttonStyle(.bordered)
                .padding(.horizontal)
                .padding(.top, 8)
                .accessibilityIdentifier("walletDetail.seedPoolNotesButton")
            }

            PendingPlatformFundFromAssetLocksList(
                coordinator: walletManager.addressFundFromAssetLockCoordinator,
                walletId: wallet.walletId,
                assetLocks: walletAssetLocks,
                resumingAssetLock: $resumingAssetLock
            )
            .padding(.top, 8)

            Divider()
                .padding(.vertical, 8)

            // Transactions Section
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Text("Transactions")
                        .font(.headline)
                    Spacer()
                }
                .padding(.horizontal)

                NavigationLink(value: TransactionsRoute(walletId: wallet.walletId)) {
                    HStack {
                        Label("View All Transactions", systemImage: "list.bullet.rectangle")
                            .font(.subheadline)

                        Spacer()

                        if transactionCount > 0 {
                            Text("\(transactionCount)")
                                .font(.caption)
                                .foregroundColor(.secondary)
                                .padding(.horizontal, 8)
                                .padding(.vertical, 4)
                                .background(Color(UIColor.secondarySystemBackground))
                                .cornerRadius(8)
                        }

                        Image(systemName: "chevron.right")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                    .padding(.horizontal)
                    .padding(.vertical, 12)
                    .background(Color(UIColor.secondarySystemBackground))
                    .cornerRadius(10)
                }
                .buttonStyle(.plain)
                .padding(.horizontal)

                // Shielded Activity row — the derived private-operation
                // history (shields, sends, unshields, withdrawals,
                // identity-creates). Reads `PersistentShieldedActivity`
                // from SwiftData; same value-based push as Transactions.
                NavigationLink(value: ShieldedActivityRoute(walletId: wallet.walletId)) {
                    HStack {
                        Label("Shielded Activity", systemImage: "lock.rectangle.stack")
                            .font(.subheadline)
                        Spacer()
                        Image(systemName: "chevron.right")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                    .padding(.horizontal)
                    .padding(.vertical, 12)
                    .background(Color(UIColor.secondarySystemBackground))
                    .cornerRadius(10)
                }
                .buttonStyle(.plain)
                .padding(.horizontal)
                .padding(.top, 8)
                .accessibilityIdentifier("walletDetail.shieldedActivityLink")
            }

            Divider()
                .padding(.vertical, 8)

            // Section header
            HStack {
                Text("Accounts")
                    .font(.headline)
                    .padding(.horizontal)
                Spacer()
            }
            .padding(.top)

            // Account List
            AccountListView(wallet: wallet)
        }
        .navigationTitle(wallet.label)
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .navigationBarTrailing) {
                Button {
                    showWalletInfo = true
                } label: {
                    Image(systemName: "info.circle")
                }
                .accessibilityIdentifier("walletDetail.infoButton")
            }
        }
        .sheet(isPresented: $showReceiveAddress) {
            ReceiveAddressView(wallet: wallet)
        }
        .sheet(isPresented: $showSendTransaction) {
            SendTransactionView(wallet: wallet)
        }
        .sheet(isPresented: $showWalletInfo) {
            WalletInfoView(wallet: wallet) {
                dismiss()
            }
        }
        .sheet(isPresented: $showFundPlatformAddress) {
            FundFromAssetLockPlatformAddressView(wallet: wallet)
        }
        .sheet(isPresented: $showTransferPlatformAddress) {
            TransferPlatformAddressView(wallet: wallet)
        }
        .sheet(isPresented: $showWithdrawPlatformAddress) {
            WithdrawPlatformAddressView(wallet: wallet)
        }
        .sheet(item: $resumingAssetLock) { lock in
            FundFromAssetLockPlatformAddressView(wallet: wallet, resumeFromLock: lock)
        }
        .sheet(isPresented: $showShieldFromAssetLock) {
            ShieldedFundFromAssetLockView(wallet: wallet)
        }
        .sheet(isPresented: $showSeedShieldedPool) {
            SeedShieldedPoolView(wallet: wallet)
        }
        .onAppear {
            appUIState.showWalletsSyncDetails = false
            // Repoint the singleton ShieldedService at THIS wallet —
            // the app-level bind only attaches it to `firstWallet`,
            // so without this every detail screen would show the
            // first-bound wallet's shielded balance regardless of
            // which wallet the user opened.
            shieldedService.switchTo(walletId: wallet.walletId)
        }
        .onChange(of: wallet.walletId) { _, newId in
            shieldedService.switchTo(walletId: newId)
        }
    }
}

// MARK: - Wallet Info View

struct WalletInfoView: View {
    @Environment(\.dismiss) var dismiss
    @Environment(\.modelContext) var modelContext
    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var walletManagerStore: WalletManagerStore
    let wallet: PersistentWallet
    var onWalletDeleted: () -> Void = {}

    @State private var editedName: String = ""
    @State private var isEditingName = false
    @State private var mainnetEnabled: Bool = false
    @State private var testnetEnabled: Bool = false
    @State private var regtestEnabled: Bool = false
    @State private var devnetEnabled: Bool = false
    @State private var isUpdatingNetworks = false
    @State private var errorMessage: String?
    @State private var showError = false
    @State private var showDeleteConfirmation = false
    @State private var isDeleting = false
    @State private var mainnetAccountCount: Int? = nil
    @State private var testnetAccountCount: Int? = nil
    @State private var devnetAccountCount: Int? = nil
    @State private var regtestAccountCount: Int? = nil

    // "View Seed Phrase" flow.
    @State private var isAuthorizingSeedPhrase = false
    @State private var revealedMnemonic: String?

    // "Verify Identity Keys" diagnostic sheet — runs the per-key
    // health check + offers re-derive / delete-orphan repair actions.
    // See `WalletKeyHealthSheet`.
    @State private var showKeyHealthSheet = false

    // Account counts come from SwiftData now.
    @Query private var accounts: [PersistentAccount]
    /// Identities owned by this wallet — passed to the key-health
    /// sheet so it can iterate them.
    @Query private var walletIdentities: [PersistentIdentity]

    init(wallet: PersistentWallet, onWalletDeleted: @escaping () -> Void = {}) {
        self.wallet = wallet
        self.onWalletDeleted = onWalletDeleted
        let walletId = wallet.walletId
        _accounts = Query(filter: #Predicate<PersistentAccount> { $0.wallet.walletId == walletId })
        _walletIdentities = Query(
            filter: #Predicate<PersistentIdentity> { $0.wallet?.walletId == walletId }
        )
    }

    var body: some View {
        NavigationView {
            Form {
                // Wallet Name Section
                Section("Wallet Name") {
                    if isEditingName {
                        HStack {
                            TextField("Wallet Name", text: $editedName)
                                .textFieldStyle(.plain)

                            Button("Cancel") {
                                editedName = wallet.label
                                isEditingName = false
                            }
                            .buttonStyle(.bordered)

                            Button("Save") {
                                saveWalletName()
                            }
                            .buttonStyle(.borderedProminent)
                            .disabled(editedName.isEmpty)
                        }
                    } else {
                        HStack {
                            Text(wallet.label)
                            Spacer()
                            Button("Edit") {
                                editedName = wallet.label
                                isEditingName = true
                            }
                        }
                    }
                }

                // Networks Section
                Section("Networks") {
                    HStack {
                        Text("Mainnet")
                        Spacer()
                        if mainnetEnabled {
                            Image(systemName: "checkmark.circle.fill")
                                .foregroundColor(.green)
                        } else {
                            Button(action: {
                                Task {
                                    await enableNetwork(.mainnet)
                                }
                            }) {
                                Image(systemName: "plus.circle")
                                    .foregroundColor(.blue)
                            }
                            .disabled(isUpdatingNetworks)
                        }
                    }

                    HStack {
                        Text("Testnet")
                        Spacer()
                        if testnetEnabled {
                            Image(systemName: "checkmark.circle.fill")
                                .foregroundColor(.green)
                        } else {
                            Button(action: {
                                Task {
                                    await enableNetwork(.testnet)
                                }
                            }) {
                                Image(systemName: "plus.circle")
                                    .foregroundColor(.blue)
                            }
                            .disabled(isUpdatingNetworks)
                        }
                    }

                    HStack {
                        Text("Devnet")
                        Spacer()
                        if devnetEnabled {
                            Image(systemName: "checkmark.circle.fill")
                                .foregroundColor(.green)
                        } else {
                            Button(action: {
                                Task {
                                    await enableNetwork(.devnet)
                                }
                            }) {
                                Image(systemName: "plus.circle")
                                    .foregroundColor(.blue)
                            }
                            .disabled(isUpdatingNetworks)
                        }
                    }

                    HStack {
                        Text("Local (Regtest)")
                        Spacer()
                        if regtestEnabled {
                            Image(systemName: "checkmark.circle.fill")
                                .foregroundColor(.green)
                        } else {
                            Button(action: {
                                Task {
                                    await enableNetwork(.regtest)
                                }
                            }) {
                                Image(systemName: "plus.circle")
                                    .foregroundColor(.blue)
                            }
                            .disabled(isUpdatingNetworks)
                        }
                    }
                }

                Section {
                    Text("Once a network is enabled, it cannot be removed. Tap + to add a network.")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }

                // Wallet Info Section
                Section("Information") {
                    HStack {
                        Text("Created")
                        Spacer()
                        Text(AppDate.formatted(wallet.createdAt, dateStyle: .abbreviated, timeStyle: .omitted))
                            .foregroundColor(.secondary)
                    }

                    HStack {
                        Text("Wallet ID")
                        Spacer()
                        Text(wallet.walletId.toHexString())
                            .font(.system(.footnote, design: .monospaced))
                            .foregroundColor(.secondary)
                            .textSelection(.enabled)
                            .multilineTextAlignment(.trailing)
                    }

                    if mainnetEnabled {
                        HStack {
                            Text("Mainnet Accounts")
                            Spacer()
                            Text(mainnetAccountCount.map(String.init) ?? "–")
                                .foregroundColor(.secondary)
                        }
                    }
                    if testnetEnabled {
                        HStack {
                            Text("Testnet Accounts")
                            Spacer()
                            Text(testnetAccountCount.map(String.init) ?? "–")
                                .foregroundColor(.secondary)
                        }
                    }
                    if devnetEnabled {
                        HStack {
                            Text("Devnet Accounts")
                            Spacer()
                            Text(devnetAccountCount.map(String.init) ?? "–")
                                .foregroundColor(.secondary)
                        }
                    }
                    if regtestEnabled {
                        HStack {
                            Text("Regtest Accounts")
                            Spacer()
                            Text(regtestAccountCount.map(String.init) ?? "–")
                                .foregroundColor(.secondary)
                        }
                    }
                }

                // View Seed Phrase Section — above Delete so the
                // destructive action stays at the bottom.
                Section {
                    Button(action: {
                        Task { await authorizeAndRevealMnemonic() }
                    }) {
                        HStack {
                            Spacer()
                            if isAuthorizingSeedPhrase {
                                ProgressView()
                                    .progressViewStyle(CircularProgressViewStyle())
                                    .scaleEffect(0.8)
                            } else {
                                Label("View Seed Phrase", systemImage: "eye")
                            }
                            Spacer()
                        }
                    }
                    .disabled(isAuthorizingSeedPhrase)
                }

                // Verify Identity Keys Section — diagnostic that
                // walks every identity's PersistentPublicKey rows,
                // re-derives the canonical key from this wallet's
                // mnemonic, and confirms the stored pubkey + the
                // Keychain bytes match. Offers re-derive (for
                // wallet-owned keys with missing/wrong keychain
                // entries) and delete-identity (for orphan rows
                // whose pubkey doesn't match the wallet's mnemonic
                // at all). The keychain-collision bug between
                // wallets at identity_index=0 is the canonical
                // trigger for needing this.
                Section {
                    Button {
                        showKeyHealthSheet = true
                    } label: {
                        HStack {
                            Spacer()
                            Label("Verify Identity Keys", systemImage: "checkmark.shield")
                            Spacer()
                        }
                    }
                    .accessibilityIdentifier("walletInfo.verifyIdentityKeysButton")
                }

                // Delete Wallet Section
                Section {
                    Button(action: {
                        showDeleteConfirmation = true
                    }) {
                        HStack {
                            Spacer()
                            if isDeleting {
                                ProgressView()
                                    .progressViewStyle(CircularProgressViewStyle())
                                    .scaleEffect(0.8)
                            } else {
                                Label("Delete Wallet", systemImage: "trash")
                                    .foregroundColor(.white)
                            }
                            Spacer()
                        }
                    }
                    .disabled(isDeleting)
                    .accessibilityIdentifier("walletInfo.deleteWalletButton")
                    .listRowBackground(Color.red)
                }
            }
            .navigationTitle("Wallet Info")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Done") {
                        dismiss()
                    }
                }
            }
            .onAppear {
                loadNetworkStates()
                loadAccountCounts()
            }
            .onChange(of: accounts.count) { _, _ in
                loadAccountCounts()
            }
            .alert("Error", isPresented: $showError) {
                Button("OK") { }
            } message: {
                Text(errorMessage ?? "An error occurred")
            }
            .alert("Delete Wallet", isPresented: $showDeleteConfirmation) {
                Button("Cancel", role: .cancel) { }
                Button("Delete", role: .destructive) {
                    Task {
                        await deleteWallet()
                    }
                }
            } message: {
                Text("Are you sure you want to delete this wallet? This action cannot be undone and you will lose access to all funds unless you have backed up your recovery phrase.")
            }
            .sheet(
                isPresented: Binding(
                    get: { revealedMnemonic != nil },
                    set: { if !$0 { revealedMnemonic = nil } }
                )
            ) {
                if let phrase = revealedMnemonic {
                    SeedPhraseRevealSheet(mnemonic: phrase)
                }
            }
            .sheet(isPresented: $showKeyHealthSheet) {
                if let managed = walletManager.wallet(for: wallet.walletId) {
                    WalletKeyHealthSheet(
                        wallet: managed,
                        walletId: wallet.walletId,
                        identities: walletIdentities,
                        network: wallet.network ?? .testnet
                    )
                } else {
                    // Wallet manager hasn't loaded this wallet —
                    // surface a placeholder rather than presenting an
                    // empty sheet that the user can't interpret.
                    NavigationView {
                        ContentUnavailableView(
                            "Wallet not loaded",
                            systemImage: "exclamationmark.triangle",
                            description: Text(
                                "Open the wallet detail view once before running the key health check."
                            )
                        )
                        .toolbar {
                            ToolbarItem(placement: .navigationBarTrailing) {
                                Button("Done") { showKeyHealthSheet = false }
                            }
                        }
                    }
                }
            }
        }
        // Progress overlay shown while `enableNetwork` runs
        // (`isUpdatingNetworks`) so the add-to-network create isn't silent.
        .overlay {
            if isUpdatingNetworks {
                ZStack {
                    Color.black.opacity(0.25)
                        .ignoresSafeArea()
                    VStack(spacing: 12) {
                        ProgressView()
                            .controlSize(.large)
                        Text("Adding to network…")
                            .font(.subheadline)
                            .foregroundStyle(.secondary)
                    }
                    .padding(24)
                    .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 16))
                }
                .transition(.opacity)
            }
        }
        .animation(.easeInOut(duration: 0.2), value: isUpdatingNetworks)
    }

    /// Prompt the user via biometric / passcode, then pull the
    /// wallet's mnemonic out of the keychain for display. On failure
    /// surfaces the error via `errorMessage`/`showError`.
    @MainActor
    private func authorizeAndRevealMnemonic() async {
        guard !isAuthorizingSeedPhrase else { return }
        isAuthorizingSeedPhrase = true
        defer { isAuthorizingSeedPhrase = false }

        let context = LAContext()
        context.localizedCancelTitle = "Cancel"
        var policyError: NSError?
        guard context.canEvaluatePolicy(
            .deviceOwnerAuthentication,
            error: &policyError
        ) else {
            errorMessage = "Authentication is unavailable on this device: "
                + (policyError?.localizedDescription ?? "unknown")
            showError = true
            return
        }

        do {
            let authorized = try await context.evaluatePolicy(
                .deviceOwnerAuthentication,
                localizedReason: "Reveal your wallet's recovery phrase."
            )
            guard authorized else { return }
        } catch {
            errorMessage = "Authorization failed: \(error.localizedDescription)"
            showError = true
            return
        }

        do {
            revealedMnemonic = try WalletStorage().retrieveMnemonic(for: wallet.walletId)
        } catch {
            errorMessage = "This wallet's recovery phrase isn't stored on this device."
            showError = true
        }
    }

    private func loadNetworkStates() {
        // A wallet now has one `PersistentWallet` row per network it
        // lives on. Since network-scoping, those rows have DISTINCT
        // `walletId`s (the network byte is folded into the digest), so
        // they can't be matched by `walletId` anymore. They share a
        // network-independent `walletGroupId` instead — group by that
        // to reflect the actual set of rows rather than the single
        // `wallet.network` this view was opened with.
        let groupId = wallet.walletGroupId
        let rows: [PersistentWallet]
        if groupId.isEmpty {
            // Legacy row (no group id): its `walletId` is the
            // network-independent digest that siblings stamp as their
            // `walletGroupId`, so find siblings by it instead of
            // collapsing to this single row.
            let siblings = (try? modelContext.fetch(FetchDescriptor<PersistentWallet>(
                predicate: PersistentWallet.predicate(walletGroupId: wallet.walletId)
            ))) ?? []
            rows = [wallet] + siblings
        } else {
            let descriptor = FetchDescriptor<PersistentWallet>(
                predicate: PersistentWallet.predicate(walletGroupId: groupId)
            )
            rows = (try? modelContext.fetch(descriptor)) ?? [wallet]
        }
        let networks = Set(rows.compactMap { $0.network })
        mainnetEnabled = networks.contains(.mainnet)
        testnetEnabled = networks.contains(.testnet)
        regtestEnabled = networks.contains(.regtest)
        devnetEnabled = networks.contains(.devnet)
    }

    private func loadAccountCounts() {
        let count = accounts.count
        mainnetAccountCount = mainnetEnabled ? count : nil
        testnetAccountCount = testnetEnabled ? count : nil
        devnetAccountCount = devnetEnabled ? count : nil
        regtestAccountCount = regtestEnabled ? count : nil
    }

    private func saveWalletName() {
        // `label` is a computed fallback; the writable backing
        // field is `name`. Empty-string means "unnamed"; the
        // computed `label` then falls back to the hex fingerprint.
        let trimmed = editedName.trimmingCharacters(in: .whitespacesAndNewlines)
        let newName: String? = trimmed.isEmpty ? nil : trimmed
        wallet.name = newName
        do {
            try modelContext.save()
            isEditingName = false
        } catch {
            errorMessage = "Failed to save wallet name: \(error.localizedDescription)"
            showError = true
            return
        }
        // Mirror the rename into the keychain metadata blob so a
        // future reinstall / orphan-recovery picks up the new
        // label instead of resurrecting the old one (or the
        // "Recovered Wallet" placeholder when the original name
        // was never written). Read the existing blob first so the
        // `networks` and `birthHeight` fields round-trip — those
        // get filled in at creation time and the rename UI has no
        // business overwriting them with stale values from the
        // SwiftData row. Falls back to a freshly-built blob if
        // none exists yet (older installs that predate the
        // metadata feature).
        let storage = WalletStorage()
        let walletId = wallet.walletId
        var metadata: WalletKeychainMetadata
        do {
            metadata = try storage.metadata(for: walletId)
                ?? WalletKeychainMetadata()
        } catch {
            metadata = WalletKeychainMetadata()
        }
        metadata.name = newName
        metadata.walletDescription = wallet.walletDescription
        // Backfill `networks` from the SwiftData row when the
        // existing blob is missing it. `PersistentWallet` is
        // currently single-network, so the best we can do here is
        // a one-element list. When multi-network support lands on
        // the Rust side this can be widened.
        if metadata.networks == nil, let net = wallet.network {
            metadata.networks = [net.networkName]
        }
        // Same backfill story for `birthHeight` — older blobs
        // missed it; we have the SwiftData copy on hand so push
        // it in once.
        if metadata.birthHeight == nil {
            metadata.birthHeight = wallet.birthHeight
        }
        do {
            try storage.setMetadata(metadata, for: walletId)
        } catch {
            // Non-fatal: SwiftData already has the new name; this
            // only affects orphan-recovery after a wipe. Surface
            // through the logger instead of blocking the UI.
            SDKLogger.error(
                "Failed to update wallet metadata in keychain: \(error.localizedDescription)"
            )
        }
    }

    private func enableNetwork(_ network: Network) async {
        isUpdatingNetworks = true
        defer { isUpdatingNetworks = false }

        // Add the existing wallet to another network by re-creating it
        // from the stored mnemonic in that network's manager. The
        // `walletId` is now network-scoped — the same mnemonic produces
        // a DIFFERENT id on the target network — so the freshly-created
        // wallet gets its OWN scoped id, and its mnemonic must be stored
        // under that new id (the source wallet's keychain entry is keyed
        // by the source network's id and won't be found when the new
        // network's wallet looks itself up). Reusing
        // `createWallet(mnemonic:)` keeps all derivation on the Rust side
        // (no Swift orchestration); the keychain write below is the
        // sanctioned Swift-owned persist step.
        let mnemonic: String
        do {
            mnemonic = try WalletStorage().retrieveMnemonic(for: wallet.walletId)
        } catch {
            errorMessage = "This wallet's recovery phrase isn't stored on this device, so it can't be added to another network."
            showError = true
            return
        }

        do {
            let mgr = try walletManagerStore.backgroundManager(for: network)
            // Enabling an existing wallet on another network: the mnemonic is
            // pre-existing and may already have on-chain history there — scan
            // from genesis (birthHeight 0) so prior funds/payments are seen.
            let created = try await mgr.createWallet(
                mnemonic: mnemonic,
                network: network,
                name: wallet.name ?? wallet.label,
                birthHeight: 0
            )
            // Persist the mnemonic AND the per-wallet metadata under the
            // newly-enabled network's scoped walletId so that wallet is
            // independently recoverable and its own keychain lookups
            // resolve. The metadata is load-bearing for orphan-recovery
            // and the post-launch warmup: `ContentView.recoverWallet`
            // and the bootstrap pre-warm pick the restore network from
            // `metadata.resolvedNetworks`, so without it a wiped wallet
            // falls back to whatever network is active and could be
            // recreated on the wrong chain. Mirror the same blob shape
            // `CreateWalletView` writes per network. Best-effort — a
            // failure here doesn't undo the successful create.
            let storage = WalletStorage()
            do {
                try storage.storeMnemonic(mnemonic, for: created.walletId)
            } catch {
                SDKLogger.error(
                    "Failed to persist mnemonic to keychain for \(network.displayName): \(error.localizedDescription)"
                )
            }
            do {
                // Birth height is a chain-block number, so it must come
                // from the TARGET network's freshly-created row — NOT the
                // source `wallet`, whose `birthHeight` belongs to the
                // network this detail screen was opened on. The persister
                // stamps the right value on the new row synchronously
                // during `createWallet`; read it back (same shape
                // `CreateWalletView` uses) so orphan-recovery rescans the
                // target chain from the correct height.
                let createdId = created.walletId
                let createdRow = try? modelContext.fetch(
                    FetchDescriptor<PersistentWallet>(
                        predicate: PersistentWallet.predicate(walletId: createdId)
                    )
                ).first
                let metadata = WalletKeychainMetadata(
                    name: wallet.name ?? wallet.label,
                    walletDescription: wallet.walletDescription,
                    networks: [network.networkName],
                    birthHeight: createdRow?.birthHeight
                )
                try storage.setMetadata(metadata, for: created.walletId)
            } catch {
                SDKLogger.error(
                    "Failed to persist wallet metadata to keychain for \(network.displayName): \(error.localizedDescription)"
                )
            }
        } catch {
            // A typed `walletAlreadyExists` throw means the wallet is
            // already on this network — a genuine no-op, so fall through to
            // refresh. Any other failure (SDK build error, Rust-side error,
            // etc.) must surface to the user instead of silently doing
            // nothing.
            guard case PlatformWalletError.walletAlreadyExists = error else {
                let description = error.localizedDescription
                SDKLogger.error(
                    "enableNetwork(\(network.displayName)) failed: \(description)"
                )
                errorMessage = "Failed to add \(network.displayName): \(description)"
                showError = true
                return
            }
            SDKLogger.error(
                "enableNetwork(\(network.displayName)) create returned benign already-exists"
            )
        }

        // Backfill a legacy row's group id (= its walletId) so it groups
        // with the sibling just created — in both directions and across
        // launches. Idempotent: only fires while empty.
        if wallet.walletGroupId.isEmpty {
            wallet.walletGroupId = wallet.walletId
            try? modelContext.save()
        }

        loadNetworkStates()
        loadAccountCounts()
    }

    private func deleteWallet() async {
        let walletId = wallet.walletId

        await MainActor.run { isDeleting = true }

        // `PlatformWalletManager.deleteWallet` handles the full wipe:
        // Rust manager-side drop, in-memory dict removal, SwiftData
        // cascade + orphan sweep (transactions / pending inputs /
        // identities the @Relationship rule doesn't reach), and the
        // Keychain mnemonic + metadata blobs.
        do {
            try walletManager.deleteWallet(walletId: walletId)
        } catch {
            SDKLogger.error(
                "Failed to fully delete wallet: \(error.localizedDescription)"
            )
            await MainActor.run {
                errorMessage = "Failed to delete wallet: \(error.localizedDescription)"
                showError = true
                isDeleting = false
            }
            return
        }

        await MainActor.run {
            isDeleting = false
            dismiss()
            onWalletDeleted()
        }
    }
}

struct BalanceCardView: View {
    let wallet: PersistentWallet
    /// Invoked when the user taps the "+" affordance next to the
    /// Platform Balance row. The parent owns the sheet presentation
    /// state, so we surface the intent rather than presenting here.
    /// `nil` hides the affordance entirely (e.g. for read-only
    /// surfaces).
    var onFundPlatform: (() -> Void)?
    /// Opens the wallet-signed Platform→Platform credit transfer sheet
    /// (`TransferPlatformAddressView`, ADDR-02).
    var onTransferPlatform: (() -> Void)?
    /// Opens the wallet-signed Platform→Core L1 withdrawal sheet
    /// (`WithdrawPlatformAddressView`, ADDR-04).
    var onWithdrawPlatform: (() -> Void)?
    /// Same shape as `onFundPlatform`, for the Shielded Balance row.
    /// Opens the Core L1 → shielded-pool funding sheet
    /// (`ShieldedFundFromAssetLockView`, Type 18).
    var onFundShielded: (() -> Void)?
    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var platformState: AppState
    @EnvironmentObject var shieldedService: ShieldedService
    @EnvironmentObject var platformBalanceSyncService: PlatformBalanceSyncService

    @Query private var addressBalances: [PersistentPlatformAddress]
    @Query private var syncStates: [PersistentPlatformAddressesSyncState]
    @Query private var shieldedNotes: [PersistentShieldedNote]

    init(
        wallet: PersistentWallet,
        onFundPlatform: (() -> Void)? = nil,
        onTransferPlatform: (() -> Void)? = nil,
        onWithdrawPlatform: (() -> Void)? = nil,
        onFundShielded: (() -> Void)? = nil
    ) {
        self.wallet = wallet
        self.onFundPlatform = onFundPlatform
        self.onTransferPlatform = onTransferPlatform
        self.onWithdrawPlatform = onWithdrawPlatform
        self.onFundShielded = onFundShielded
        let walletId = wallet.walletId
        let walletNetworkRaw = (wallet.network ?? .testnet).rawValue
        _addressBalances = Query(
            filter: #Predicate<PersistentPlatformAddress> { $0.walletId == walletId }
        )
        _syncStates = Query(
            filter: #Predicate<PersistentPlatformAddressesSyncState> { $0.networkRaw == walletNetworkRaw }
        )
        _shieldedNotes = Query(
            filter: PersistentShieldedNote.unspentPredicate(walletId: walletId)
        )
    }

    /// Per-wallet shielded balance: sum of this wallet's unspent
    /// `PersistentShieldedNote` values. Reads SwiftData (Rust pushes
    /// note rows via the shielded persister) rather than the single-mirror
    /// `shieldedService.shieldedBalance`, so the card is correct for a
    /// non-`firstWallet` wallet whose engine binding is live but whose UI
    /// mirror is pointed elsewhere.
    private var shieldedBalance: UInt64 {
        shieldedNotes.reduce(0) { $0 + $1.value }
    }

    /// Core-chain balance summed from one Rust in-memory account snapshot.
    /// Reading every component from the same FFI result keeps the card
    /// internally consistent while sync updates account state.
    private var coreBalance: WalletCoreBalance {
        walletManager.accountBalances(for: wallet.walletId)
            .reduce(into: WalletCoreBalance()) { total, account in
                total.confirmed += account.confirmed
                total.unconfirmed += account.unconfirmed
                total.immature += account.immature
            }
    }

    /// Platform balance from BLAST sync (preferred) or identity sum (fallback).
    ///
    /// Skips identities whose `modelContext` is nil — SwiftData's
    /// marker for an invalidated row. During a wallet delete the
    /// relationship array can briefly contain invalidated entries
    /// before SwiftUI rerenders past them; reading any persisted
    /// property on an invalidated model fatals with
    /// `BackingData.swift:866: This model instance was invalidated…`.
    var platformBalance: UInt64 {
        let blastBalance = addressBalances.reduce(0) { $0 + $1.balance }
        let hasSynced = syncStates.first.map { $0.syncHeight > 0 || $0.syncTimestamp > 0 }
            ?? false
        if blastBalance > 0 || hasSynced {
            return blastBalance
        }
        // Fall back to summing credits across the wallet's
        // identities (via the SwiftData relationship). Pre-BLAST-
        // sync state shows approximate credit balance aggregated
        // from the on-chain identities we know about.
        return wallet.identities
            .filter { $0.modelContext != nil }
            .reduce(UInt64(0)) { sum, identity in
                sum + UInt64(bitPattern: identity.balance)
            }
    }

    /// Trailing-menu items for the Platform Balance row. Built only
    /// when at least one of Transfer/Withdraw is wired (the editable
    /// Wallet Detail surface); empty otherwise so read-only surfaces and
    /// the legacy single-action `+` path stay intact. Top Up is included
    /// in the menu whenever it's present so all three live in one place.
    private var platformMenuItems: [WalletBalanceRow.TrailingMenuItem] {
        guard onTransferPlatform != nil || onWithdrawPlatform != nil else { return [] }
        var items: [WalletBalanceRow.TrailingMenuItem] = []
        if let fund = onFundPlatform {
            items.append(
                WalletBalanceRow.TrailingMenuItem(
                    title: "Top Up from Core",
                    systemImage: "plus.circle",
                    accessibilityIdentifier: "balanceCard.platform.topUp",
                    action: fund
                )
            )
        }
        if let transfer = onTransferPlatform {
            items.append(
                WalletBalanceRow.TrailingMenuItem(
                    title: "Transfer Credits",
                    systemImage: "arrow.left.arrow.right",
                    accessibilityIdentifier: "balanceCard.platform.transfer",
                    action: transfer
                )
            )
        }
        if let withdraw = onWithdrawPlatform {
            items.append(
                WalletBalanceRow.TrailingMenuItem(
                    title: "Withdraw to Core",
                    systemImage: "arrow.up.circle",
                    accessibilityIdentifier: "balanceCard.platform.withdraw",
                    action: withdraw
                )
            )
        }
        return items
    }

    var body: some View {
        let core = coreBalance
        let allZero = WalletBalanceCardState.isEmpty(
            confirmedCore: core.confirmed,
            unconfirmedCore: core.unconfirmed,
            immatureCore: core.immature,
            platform: platformBalance,
            shielded: shieldedBalance
        )

        VStack(spacing: 12) {
            if allZero {
                Text("Empty Wallet")
                    .font(.system(size: 28, weight: .medium, design: .rounded))
                    .foregroundColor(.secondary)
            } else {
                // Core Balance row
                WalletBalanceRow(
                    label: "Core Balance",
                    amount: core.confirmed,
                    incoming: core.unconfirmed,
                    immature: core.immature,
                    color: .primary,
                    unit: .duffs
                )

                // Platform Balance row — on the editable Wallet Detail
                // surface this exposes a trailing menu with Top Up
                // (Core→Platform), Transfer (Platform→Platform,
                // ADDR-02), and Withdraw (Platform→Core L1, ADDR-04).
                // Read-only call sites pass `nil` for all three and the
                // affordance disappears. A single Top Up closure with no
                // transfer/withdraw still renders the legacy `+` button.
                WalletBalanceRow(
                    label: "Platform Balance",
                    amount: platformBalance,
                    color: .blue,
                    unit: .credits,
                    showSyncIndicator: platformBalanceSyncService.isSyncing,
                    trailingAction: platformMenuItems.isEmpty
                        ? onFundPlatform.map { fund in
                            WalletBalanceRow.TrailingAction(
                                systemImage: "plus.circle.fill",
                                accessibilityLabel: "Top Up Platform Balance from Core",
                                action: fund
                            )
                        }
                        : nil,
                    trailingMenu: platformMenuItems.isEmpty
                        ? nil
                        : (
                            accessibilityLabel: "Platform Balance Actions",
                            items: platformMenuItems
                        )
                )

                // Shielded Balance row — mirrors the Platform
                // Balance row's trailing `+` affordance. When
                // `onFundShielded` is wired the user can open the
                // shielding sheet, which now lets them choose the
                // source: Core L1 → pool (Type 18,
                // `ShieldFromAssetLockTransition`) or Platform credits
                // → pool (Type 15, `shieldedShield`).
                WalletBalanceRow(
                    label: "Shielded Balance",
                    amount: shieldedBalance,
                    color: .purple,
                    unit: .credits,
                    showSyncIndicator: shieldedService.isSyncing,
                    trailingAction: onFundShielded.map { fund in
                        WalletBalanceRow.TrailingAction(
                            systemImage: "plus.circle.fill",
                            accessibilityLabel: "Add to Shielded Balance",
                            action: fund
                        )
                    }
                )
            }
        }
        .padding()
        .background(Color(UIColor.secondarySystemBackground))
        .cornerRadius(12)
    }
}

struct WalletCoreBalance {
    var confirmed: UInt64 = 0
    var unconfirmed: UInt64 = 0
    var immature: UInt64 = 0
}

/// Pure balance-card state used by the SwiftUI view and unit tests. Keeping
/// the empty-state decision here prevents a future UI refactor from silently
/// dropping non-spendable-but-owned Core value such as immature coinbase funds.
enum WalletBalanceCardState {
    static func isEmpty(
        confirmedCore: UInt64,
        unconfirmedCore: UInt64,
        immatureCore: UInt64,
        platform: UInt64,
        shielded: UInt64
    ) -> Bool {
        confirmedCore == 0
            && unconfirmedCore == 0
            && immatureCore == 0
            && platform == 0
            && shielded == 0
    }
}

/// A single balance row showing label, amount, and optional incoming amount.
private enum WalletBalanceUnit {
    case duffs
    case credits
}

private struct WalletBalanceRow: View {
    /// Tappable affordance shown at the trailing edge of the row.
    /// Used today by the Platform Balance row to surface a "fund
    /// from Core" entry point without crowding the action button
    /// strip at the top of the wallet detail screen.
    struct TrailingAction {
        let systemImage: String
        let accessibilityLabel: String
        let action: () -> Void
    }

    /// One entry in a trailing `Menu`. Used by the Platform Balance
    /// row to offer Top Up / Transfer / Withdraw without crowding the
    /// row with three separate glyph buttons.
    struct TrailingMenuItem: Identifiable {
        let id = UUID()
        let title: String
        let systemImage: String
        let accessibilityIdentifier: String
        let action: () -> Void
    }

    let label: String
    var amount: UInt64
    var incoming: UInt64 = 0
    var immature: UInt64 = 0
    var color: Color
    var unit: WalletBalanceUnit = .duffs
    var showSyncIndicator: Bool = false
    var trailingAction: TrailingAction? = nil
    /// When set, the trailing affordance is a `Menu` (ellipsis glyph)
    /// listing these items instead of a single `trailingAction` button.
    /// `trailingMenu` takes precedence over `trailingAction` if both
    /// are supplied.
    var trailingMenu: (accessibilityLabel: String, items: [TrailingMenuItem])? = nil

    var body: some View {
        HStack {
            VStack(alignment: .leading, spacing: 2) {
                HStack(spacing: 4) {
                    Text(label)
                        .font(.caption)
                        .foregroundColor(.secondary)
                    if showSyncIndicator {
                        ProgressView()
                            .scaleEffect(0.5)
                    }
                }
            }
            Spacer()
            VStack(alignment: .trailing, spacing: 2) {
                if amount > 0 {
                    Text(formatBalance(amount))
                        .font(.subheadline)
                        .fontWeight(.medium)
                        .foregroundColor(color)
                } else {
                    Text("—")
                        .font(.subheadline)
                        .foregroundColor(.secondary)
                }
                if incoming > 0 {
                    Text("(+\(formatBalance(incoming)) incoming)")
                        .font(.caption2)
                        .foregroundColor(.orange)
                }
                if immature > 0 {
                    Text("(\(formatBalance(immature)) immature)")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                }
            }
            if let menu = trailingMenu {
                Menu {
                    ForEach(menu.items) { item in
                        Button(action: item.action) {
                            Label(item.title, systemImage: item.systemImage)
                        }
                        .accessibilityIdentifier(item.accessibilityIdentifier)
                    }
                } label: {
                    Image(systemName: "ellipsis.circle.fill")
                        .font(.title3)
                        .foregroundColor(color)
                }
                .accessibilityLabel(menu.accessibilityLabel)
            } else if let trailing = trailingAction {
                Button(action: trailing.action) {
                    Image(systemName: trailing.systemImage)
                        .font(.title3)
                        .foregroundColor(color)
                }
                .buttonStyle(.plain)
                .accessibilityLabel(trailing.accessibilityLabel)
            }
        }
    }

    private func formatBalance(_ amount: UInt64) -> String {
        let dashDivisor: Double = switch unit {
        case .duffs:
            100_000_000.0
        case .credits:
            100_000_000_000.0
        }
        let dash = Double(amount) / dashDivisor
        let formatter = NumberFormatter()
        formatter.minimumFractionDigits = 0
        formatter.maximumFractionDigits = 8
        formatter.numberStyle = .decimal
        formatter.groupingSeparator = ","
        formatter.decimalSeparator = "."
        if let formatted = formatter.string(from: NSNumber(value: dash)) {
            return "\(formatted) DASH"
        }
        return String(format: "%.8f DASH", dash)
    }
}

// MARK: - Seed Phrase Reveal Sheet

/// Read-only reveal of the mnemonic, gated by biometric auth on the
/// caller side. Renders the 12-word phrase in a numbered grid with a
/// copy-to-clipboard convenience and a warning banner.
private struct SeedPhraseRevealSheet: View {
    let mnemonic: String
    @Environment(\.dismiss) private var dismiss
    @State private var copied = false

    private var words: [String] {
        mnemonic.split(separator: " ").map(String.init)
    }

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    Label(
                        "Never share this phrase. Anyone who sees it can spend your funds.",
                        systemImage: "exclamationmark.triangle.fill"
                    )
                    .font(.subheadline)
                    .foregroundColor(.white)
                    .padding()
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(Color.red)
                    .cornerRadius(10)

                    let columns = [GridItem(.flexible()), GridItem(.flexible())]
                    LazyVGrid(columns: columns, spacing: 8) {
                        ForEach(Array(words.enumerated()), id: \.offset) { idx, word in
                            HStack(spacing: 8) {
                                Text(String(format: "%2d.", idx + 1))
                                    .font(.body.monospacedDigit())
                                    .foregroundColor(.secondary)
                                    .frame(width: 28, alignment: .trailing)
                                Text(word)
                                    .font(.body)
                                    .textSelection(.enabled)
                                Spacer()
                            }
                            .padding(8)
                            .background(Color(.secondarySystemBackground))
                            .cornerRadius(8)
                        }
                    }

                    Button {
                        UIPasteboard.general.string = mnemonic
                        copied = true
                        Task {
                            try? await Task.sleep(nanoseconds: 2_000_000_000)
                            copied = false
                        }
                    } label: {
                        Label(
                            copied ? "Copied!" : "Copy to Clipboard",
                            systemImage: copied ? "checkmark" : "doc.on.doc"
                        )
                        .frame(maxWidth: .infinity)
                    }
                    .buttonStyle(.bordered)
                }
                .padding()
            }
            .navigationTitle("Recovery Phrase")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Done") { dismiss() }
                }
            }
        }
    }
}
