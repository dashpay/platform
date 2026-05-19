// FundPlatformAddressView.swift
// SwiftExampleApp
//
// Stepped UI for funding a Platform payment address from a Core
// (SPV) wallet balance. Drives the new `ManagedPlatformAddressWallet
// .fundFromCoreAssetLock(...)` end-to-end:
//
//   1. Build an asset-lock tx from the chosen Core BIP44 account.
//   2. Wait for the IS-lock (or fall back to ChainLock on timeout).
//   3. Submit an `AddressFundingFromAssetLockTransition` against the
//      proof to credit the destination platform address.
//   4. Mark the asset lock `Consumed` on success.
//
// No private keys cross the FFI boundary on this path — both
// Core-side derivation (inside the wallet's asset-lock manager) and
// the outer state-transition signature route through a local
// `MnemonicResolver`, atomic per call.

import SwiftUI
import SwiftDashSDK
import SwiftData

struct FundPlatformAddressView: View {
    @Environment(\.dismiss) private var dismiss
    @Environment(\.modelContext) private var modelContext
    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var platformState: AppState

    /// Wallet to fund a platform address on. Drives both the picker
    /// scope (Core BIP44 accounts and Platform addresses on this
    /// wallet only) and the managed-wallet lookup at submit time.
    let wallet: PersistentWallet

    /// All persisted accounts. Filtered down to the wallet's Core
    /// BIP44 accounts inside `coreAccountOptions` and to its DIP-17
    /// platform-payment accounts inside `platformAccountOptions`.
    @Query private var allAccounts: [PersistentAccount]

    /// All persisted platform addresses. Filtered down to the
    /// chosen platform-payment account inside
    /// `recipientCandidates`.
    @Query private var allPlatformAddresses: [PersistentPlatformAddress]

    // MARK: - Selection state

    @State private var fundingCoreAccountIndex: UInt32? = nil
    @State private var platformAccountIndex: UInt32? = nil
    @State private var selectedRecipientHash: Data? = nil
    @State private var amountDash: String = "0.001"

    // MARK: - Submit state

    @State private var isSubmitting: Bool = false
    @State private var submitError: SubmitError? = nil
    @State private var newBalance: UInt64? = nil

    /// 1 DASH = 1e8 duffs (Core side). The asset-lock builder takes
    /// duffs; we convert here for display ergonomics only.
    private static let duffsPerDash: UInt64 = 100_000_000

    /// Conservative floor mirroring `CreateIdentityView` — the
    /// platform-side fee strategy `ReduceOutput(remainder_index)`
    /// also pays the on-chain fee out of the remainder, so the
    /// remainder needs to cover at least the asset-lock fee plus the
    /// platform-side fee. 1mDASH (~100k duffs) is well above both.
    private static let minDuffs: UInt64 = 100_000

    var body: some View {
        NavigationStack {
            Form {
                if newBalance != nil {
                    successSection
                } else {
                    walletSection
                    coreFundingSection
                    platformAccountSection
                    recipientSection
                    amountSection
                    if canSubmit {
                        submitSection
                    }
                }
            }
            .navigationTitle("Fund Platform Address")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") { dismiss() }
                        .disabled(isSubmitting)
                }
            }
            .alert(item: $submitError) { err in
                Alert(
                    title: Text("Could not fund address"),
                    message: Text(err.message),
                    dismissButton: .default(Text("OK"))
                )
            }
            .onAppear(perform: autoSelectDefaults)
        }
    }

    // MARK: - Sections

    private var walletSection: some View {
        Section {
            HStack {
                Label("Wallet", systemImage: "wallet.pass")
                Spacer()
                Text(wallet.name ?? hexShort(wallet.walletId))
                    .lineLimit(1)
                    .truncationMode(.middle)
                    .foregroundColor(.secondary)
            }
        } header: {
            Text("Source")
        }
    }

    @ViewBuilder
    private var coreFundingSection: some View {
        let options = coreAccountOptions
        Section {
            if options.isEmpty {
                Text("No funded Core (BIP44 standard) accounts on this wallet.")
                    .font(.caption)
                    .foregroundColor(.secondary)
            } else {
                Picker("Core Account", selection: $fundingCoreAccountIndex) {
                    Text("Select…").tag(Optional<UInt32>.none)
                    ForEach(options, id: \.accountIndex) { opt in
                        Text("Account #\(opt.accountIndex) — \(formatDuffs(opt.balanceDuffs))")
                            .tag(Optional(opt.accountIndex))
                    }
                }
            }
        } header: {
            Text("Core Funding Source")
        } footer: {
            Text("The selected Core account's UTXOs are locked into an asset lock; the locked DASH becomes Platform credits on the destination address.")
        }
    }

    @ViewBuilder
    private var platformAccountSection: some View {
        let options = platformAccountOptions
        Section {
            if options.isEmpty {
                Text("No DIP-17 Platform Payment accounts on this wallet yet.")
                    .font(.caption)
                    .foregroundColor(.secondary)
            } else {
                Picker("Platform Account", selection: $platformAccountIndex) {
                    Text("Select…").tag(Optional<UInt32>.none)
                    ForEach(options, id: \.accountIndex) { opt in
                        Text("Account #\(opt.accountIndex) — \(formatCredits(opt.totalCredits))")
                            .tag(Optional(opt.accountIndex))
                    }
                }
                .onChange(of: platformAccountIndex) { _, _ in
                    selectedRecipientHash = nil
                    autoSelectRecipient()
                }
            }
        } header: {
            Text("Destination Account")
        } footer: {
            Text("Platform Payment account that owns the destination address. Picker shows current credit balance.")
        }
    }

    @ViewBuilder
    private var recipientSection: some View {
        let options = recipientCandidates
        Section {
            if options.isEmpty {
                Text("No unused addresses available on this platform account. Sync first.")
                    .font(.caption)
                    .foregroundColor(.secondary)
            } else {
                Picker("Recipient", selection: $selectedRecipientHash) {
                    Text("Select…").tag(Optional<Data>.none)
                    ForEach(options, id: \.addressHash) { row in
                        Text("Addr #\(row.addressIndex) — \(row.address.prefix(12))…")
                            .tag(Optional(row.addressHash))
                    }
                }
            }
        } header: {
            Text("Destination Address")
        } footer: {
            Text("Defaults to the lowest-index unused address on the selected platform account.")
        }
    }

    @ViewBuilder
    private var amountSection: some View {
        Section {
            HStack {
                TextField("Amount", text: $amountDash)
                    .keyboardType(.decimalPad)
                    .textFieldStyle(.roundedBorder)
                    .disabled(isSubmitting)
                Text("DASH")
                    .foregroundColor(.secondary)
            }
        } header: {
            Text("Amount")
        } footer: {
            if let amount = parsedDuffs {
                Text("\(formatDuffs(amount)) duffs will be locked. Minimum: \(formatDuffs(Self.minDuffs)).")
            } else {
                Text("Minimum: \(formatDuffs(Self.minDuffs)) duffs.")
            }
        }
    }

    private var submitSection: some View {
        Section {
            Button {
                submit()
            } label: {
                HStack {
                    if isSubmitting {
                        ProgressView().controlSize(.small).tint(.white)
                        Text("Funding…")
                    } else {
                        Text("Fund Address")
                    }
                    Spacer()
                }
                .foregroundColor(.white)
            }
            .frame(maxWidth: .infinity)
            .listRowBackground(Color.accentColor)
            .disabled(isSubmitting)
        }
    }

    @ViewBuilder
    private var successSection: some View {
        Section {
            HStack {
                Image(systemName: "checkmark.circle.fill")
                    .foregroundColor(.green)
                Text("Address funded")
                    .fontWeight(.semibold)
            }
            if let bal = newBalance {
                HStack {
                    Text("New balance")
                    Spacer()
                    Text(formatCredits(bal))
                        .foregroundColor(.secondary)
                }
            }
            Button("Done") { dismiss() }
        } header: {
            Text("Success")
        }
    }

    // MARK: - Derived

    private struct CoreAccountOption {
        let accountIndex: UInt32
        let balanceDuffs: UInt64
    }

    private struct PlatformAccountOption {
        let accountIndex: UInt32
        let totalCredits: UInt64
    }

    private var coreAccountOptions: [CoreAccountOption] {
        // Surface Core BIP44 standard accounts (`standardTag == 0`).
        // Balance shown from `balanceConfirmed` so the user sees
        // what's spendable. UTXO selection itself is owned by the
        // Rust-side asset-lock builder.
        allAccounts
            .filter { $0.wallet.walletId == wallet.walletId }
            .filter { $0.standardTag == 0 }
            .sorted { $0.accountIndex < $1.accountIndex }
            .map {
                CoreAccountOption(
                    accountIndex: $0.accountIndex,
                    balanceDuffs: $0.balanceConfirmed
                )
            }
    }

    private var platformAccountOptions: [PlatformAccountOption] {
        // DIP-17 platform payment accounts. `accountType == 14` is
        // the PlatformPayment discriminant on PersistentAccount.
        let accounts = allAccounts
            .filter { $0.wallet.walletId == wallet.walletId }
            .filter { $0.accountType == 14 }
            .sorted { $0.accountIndex < $1.accountIndex }
        return accounts.map { acct in
            let total = allPlatformAddresses
                .filter {
                    $0.walletId == wallet.walletId && $0.accountIndex == acct.accountIndex
                }
                .reduce(into: UInt64(0)) { acc, addr in acc &+= addr.balance }
            return PlatformAccountOption(accountIndex: acct.accountIndex, totalCredits: total)
        }
    }

    private var recipientCandidates: [PersistentPlatformAddress] {
        guard let acctIdx = platformAccountIndex else { return [] }
        return allPlatformAddresses
            .filter { $0.walletId == wallet.walletId && $0.accountIndex == acctIdx && !$0.isUsed && $0.balance == 0 }
            .sorted { $0.addressIndex < $1.addressIndex }
    }

    private var parsedDuffs: UInt64? {
        let raw = amountDash.trimmingCharacters(in: .whitespacesAndNewlines)
        guard let dash = Double(raw), dash > 0 else { return nil }
        let duffsDouble = dash * Double(Self.duffsPerDash)
        guard duffsDouble.isFinite, duffsDouble <= Double(UInt64.max) else { return nil }
        return UInt64(duffsDouble.rounded(.toNearestOrAwayFromZero))
    }

    private var canSubmit: Bool {
        fundingCoreAccountIndex != nil
            && platformAccountIndex != nil
            && selectedRecipientHash != nil
            && (parsedDuffs ?? 0) >= Self.minDuffs
            && !isSubmitting
    }

    // MARK: - Actions

    private func autoSelectDefaults() {
        if fundingCoreAccountIndex == nil {
            fundingCoreAccountIndex = coreAccountOptions
                .first { $0.balanceDuffs > 0 }?.accountIndex
                ?? coreAccountOptions.first?.accountIndex
        }
        if platformAccountIndex == nil {
            platformAccountIndex = platformAccountOptions.first?.accountIndex
        }
        autoSelectRecipient()
    }

    private func autoSelectRecipient() {
        if selectedRecipientHash == nil {
            selectedRecipientHash = recipientCandidates.first?.addressHash
        }
    }

    private func submit() {
        guard
            let fundingAccountIndex = fundingCoreAccountIndex,
            let platformAcct = platformAccountIndex,
            let hash = selectedRecipientHash,
            let recipient = recipientCandidates.first(where: { $0.addressHash == hash }),
            let duffs = parsedDuffs
        else { return }

        let managedHolder = walletManager.wallet(for: wallet.walletId)
        guard let managedHolder else {
            submitError = SubmitError(message: "Wallet handle not found in the wallet manager.")
            return
        }
        let addressWallet: ManagedPlatformAddressWallet
        do {
            addressWallet = try managedHolder.platformAddressWallet()
        } catch {
            submitError = SubmitError(message: "Couldn't acquire platform-address wallet: \(error.localizedDescription)")
            return
        }
        let signer = KeychainSigner(modelContainer: modelContext.container)

        isSubmitting = true
        Task {
            defer { isSubmitting = false }
            do {
                let updates = try await addressWallet.fundFromCoreAssetLock(
                    amountDuffs: duffs,
                    fundingAccountIndex: fundingAccountIndex,
                    platformAccountIndex: platformAcct,
                    recipients: [
                        // Single recipient — the auto-picked unused
                        // address — gets the entire remainder after
                        // the on-chain fee. `credits = nil` is the
                        // canonical "receive remainder" marker.
                        ManagedPlatformAddressWallet.FundingRecipient(
                            addressType: recipient.addressType,
                            hash: recipient.addressHash,
                            credits: nil
                        )
                    ],
                    signer: signer
                )
                // The remainder recipient's balance reflects the
                // proof-attested credit total. Surface it so the
                // success row is meaningful.
                let credited = updates.first(where: { $0.hash == recipient.addressHash })?.balance ?? 0
                newBalance = credited
            } catch let err as PlatformWalletError {
                submitError = SubmitError(message: err.localizedDescription)
            } catch {
                submitError = SubmitError(message: "\(error)")
            }
        }
    }

    // MARK: - Helpers

    private func formatDuffs(_ duffs: UInt64) -> String {
        let dash = Double(duffs) / Double(Self.duffsPerDash)
        return String(format: "%.8f DASH", dash)
    }

    private func formatCredits(_ credits: UInt64) -> String {
        // Credit divisor matches CreateIdentityView (1e11 credits/DASH).
        let dash = Double(credits) / 100_000_000_000.0
        return String(format: "%.6f DASH (credits)", dash)
    }

    private func hexShort(_ data: Data) -> String {
        let hex = data.map { String(format: "%02x", $0) }.joined()
        return hex.count > 12 ? "\(hex.prefix(6))…\(hex.suffix(6))" : hex
    }
}

// MARK: - Submit error wrapper

private struct SubmitError: Identifiable {
    let id = UUID()
    let message: String
}
