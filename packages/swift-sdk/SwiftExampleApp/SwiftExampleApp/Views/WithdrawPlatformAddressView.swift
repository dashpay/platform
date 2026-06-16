// WithdrawPlatformAddressView.swift
// SwiftExampleApp
//
// Production (wallet-signed) UI for ADDR-04: withdraw a Platform
// payment account's credits back to a Core L1 address. Mirrors
// `FundFromAssetLockPlatformAddressView`'s shape and drives
// `ManagedPlatformAddressWallet.withdraw(accountIndex:coreAddress:
// coreFeePerByte:signer:)` with a `KeychainSigner`.
//
// Withdrawals consume the FULL funded balance of the account (no
// per-address amount, no change output), so this view shows the
// computed total rather than an amount field. The Core destination
// can be one of the wallet's own receive addresses ("My Wallet") or a
// pasted/scanned external address; the address is network-checked on
// the Rust side. No private keys are entered here — contrast with the
// raw `WithdrawAddressFundsView` debug form.

import SwiftUI
import SwiftDashSDK
import SwiftData

struct WithdrawPlatformAddressView: View {
    @Environment(\.dismiss) private var dismiss
    @Environment(\.modelContext) private var modelContext
    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var platformBalanceSyncService: PlatformBalanceSyncService

    let wallet: PersistentWallet

    @Query private var allAccounts: [PersistentAccount]
    @Query private var allPlatformAddresses: [PersistentPlatformAddress]

    // MARK: - Selection state

    private enum DestinationMode: String, CaseIterable, Identifiable {
        case myWallet = "My Wallet"
        case external = "External"
        var id: String { rawValue }
    }

    @State private var sourceAccountIndex: UInt32? = nil
    @State private var destinationMode: DestinationMode = .myWallet
    /// Core L1 address derived from this wallet's Core receive pool
    /// (mode == .myWallet). Resolved lazily in `resolveMyWalletAddress`.
    @State private var myWalletAddress: String? = nil
    /// Pasted/scanned external Core address (mode == .external).
    @State private var externalAddress: String = ""
    @State private var coreFeePerByte: String = "1"

    // MARK: - Core readiness

    /// nil = not yet checked, true/false = Core wallet usable.
    @State private var coreReady: Bool? = nil
    @State private var coreNotReadyReason: String? = nil

    // MARK: - Submit state

    @State private var submitError: SubmitError? = nil
    @State private var isSubmitting = false
    @State private var didSucceed = false

    private static let creditsPerDash: Double = 100_000_000_000.0

    var body: some View {
        NavigationStack {
            Form {
                if didSucceed {
                    successSection
                } else if coreReady == false {
                    coreNotReadySection
                } else {
                    walletSection
                    sourceAccountSection
                    destinationSection
                    feeSection
                    summarySection
                    if canSubmit {
                        submitSection
                    }
                }
            }
            .navigationTitle("Withdraw to Core")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") { dismiss() }
                        .disabled(isSubmitting)
                }
            }
            .alert(item: $submitError) { err in
                Alert(
                    title: Text("Could not withdraw"),
                    message: Text(err.message),
                    dismissButton: .default(Text("OK"))
                )
            }
            .onAppear {
                checkCoreReady()
                autoSelectDefaults()
            }
            .onChange(of: destinationMode) { _, mode in
                if mode == .myWallet { resolveMyWalletAddress() }
            }
        }
    }

    // MARK: - Sections

    private var coreNotReadySection: some View {
        Section {
            VStack(alignment: .leading, spacing: 8) {
                Label("Core wallet not ready", systemImage: "exclamationmark.triangle.fill")
                    .foregroundColor(.orange)
                    .font(.headline)
                Text(coreNotReadyReason
                    ?? "The Core (SPV) wallet must be initialized before you can withdraw to an L1 address. Sync the Core wallet and try again.")
                    .font(.callout)
                    .foregroundColor(.secondary)
                Button("Close") { dismiss() }
                    .padding(.top, 4)
            }
        }
        .accessibilityIdentifier("withdrawPlatform.coreNotReadySection")
    }

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
    private var sourceAccountSection: some View {
        let options = platformAccountOptions
        Section {
            if options.isEmpty {
                Text("No DIP-17 Platform Payment accounts on this wallet yet.")
                    .font(.caption)
                    .foregroundColor(.secondary)
            } else {
                Picker("Source Account", selection: $sourceAccountIndex) {
                    Text("Select…").tag(Optional<UInt32>.none)
                    ForEach(options, id: \.accountIndex) { opt in
                        Text("Account #\(opt.accountIndex) — \(formatCredits(opt.totalCredits))")
                            .tag(Optional(opt.accountIndex))
                    }
                }
                .accessibilityIdentifier("withdrawPlatform.sourceAccountPicker")
            }
        } header: {
            Text("Source Account")
        } footer: {
            Text("The full credit balance of this account is withdrawn — there is no partial amount.")
        }
    }

    @ViewBuilder
    private var destinationSection: some View {
        Section {
            Picker("Destination", selection: $destinationMode) {
                ForEach(DestinationMode.allCases) { mode in
                    Text(mode.rawValue).tag(mode)
                }
            }
            .pickerStyle(.segmented)
            .accessibilityIdentifier("withdrawPlatform.destinationModePicker")

            switch destinationMode {
            case .myWallet:
                if let addr = myWalletAddress {
                    HStack {
                        Label("Receive Address", systemImage: "arrow.down.circle")
                        Spacer()
                        Text("\(addr.prefix(10))…\(addr.suffix(6))")
                            .font(.system(.body, design: .monospaced))
                            .foregroundColor(.secondary)
                    }
                } else {
                    Text("Resolving a Core receive address…")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            case .external:
                VStack(alignment: .leading, spacing: 4) {
                    TextField("Core L1 address", text: $externalAddress)
                        .textFieldStyle(.roundedBorder)
                        .autocapitalization(.none)
                        .disableAutocorrection(true)
                        .monospaced()
                        .accessibilityIdentifier("withdrawPlatform.externalAddressField")
                    Text("The address is validated for this network on submit.")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
        } header: {
            Text("Core L1 Destination")
        } footer: {
            Text("Withdraw to one of your own Core receive addresses, or paste an external Core address.")
        }
    }

    @ViewBuilder
    private var feeSection: some View {
        Section {
            HStack {
                TextField("Fee per byte", text: $coreFeePerByte)
                    .keyboardType(.numberPad)
                    .textFieldStyle(.roundedBorder)
                    .disabled(isSubmitting)
                    .accessibilityIdentifier("withdrawPlatform.feePerByteField")
                Text("duffs/byte")
                    .foregroundColor(.secondary)
            }
        } header: {
            Text("Core Fee Rate")
        } footer: {
            Text("Fee rate for the eventual L1 payout transaction. Default is 1.")
        }
    }

    private var summarySection: some View {
        Section {
            HStack {
                Label("Total to Withdraw", systemImage: "dollarsign.circle")
                Spacer()
                Text(formatCredits(selectedSourceAccountCredits))
                    .foregroundColor(.secondary)
            }
        } header: {
            Text("Summary")
        } footer: {
            Text("The platform-side fee is deducted from these inputs. The full remaining balance is converted to Core duffs and paid out on L1 (minus the L1 fee).")
        }
    }

    private var submitSection: some View {
        Section {
            Button {
                submit()
            } label: {
                HStack {
                    if isSubmitting {
                        ProgressView()
                        Text("Withdrawing…")
                    } else {
                        Text("Withdraw")
                    }
                    Spacer()
                }
                .foregroundColor(.white)
            }
            .frame(maxWidth: .infinity)
            .listRowBackground(Color.accentColor)
            .disabled(isSubmitting)
            .accessibilityIdentifier("withdrawPlatform.submitButton")
        }
    }

    private var successSection: some View {
        Section {
            VStack(alignment: .leading, spacing: 8) {
                Label("Withdrawal submitted", systemImage: "checkmark.seal.fill")
                    .foregroundColor(.green)
                    .font(.headline)
                Text("The withdrawal was submitted. Credits will arrive on L1 once the payout is processed; balances are resyncing.")
                    .font(.callout)
                    .foregroundColor(.secondary)
                Button {
                    dismiss()
                } label: {
                    Text("Done").frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)
                .padding(.top, 4)
            }
        }
    }

    // MARK: - Derived

    private struct PlatformAccountOption {
        let accountIndex: UInt32
        let totalCredits: UInt64
    }

    private var platformAccountOptions: [PlatformAccountOption] {
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

    private var selectedSourceAccountCredits: UInt64 {
        guard let idx = sourceAccountIndex else { return 0 }
        return platformAccountOptions.first(where: { $0.accountIndex == idx })?.totalCredits ?? 0
    }

    private var parsedFeePerByte: UInt32? {
        let raw = coreFeePerByte.trimmingCharacters(in: .whitespacesAndNewlines)
        guard let v = UInt32(raw), v > 0 else { return nil }
        return v
    }

    /// Resolved Core destination address for the current mode.
    private var resolvedCoreAddress: String? {
        switch destinationMode {
        case .myWallet:
            return myWalletAddress
        case .external:
            let trimmed = externalAddress.trimmingCharacters(in: .whitespacesAndNewlines)
            return trimmed.isEmpty ? nil : trimmed
        }
    }

    private var canSubmit: Bool {
        guard
            !isSubmitting,
            coreReady == true,
            sourceAccountIndex != nil,
            selectedSourceAccountCredits > 0,
            parsedFeePerByte != nil,
            let addr = resolvedCoreAddress, !addr.isEmpty
        else { return false }
        return true
    }

    // MARK: - Actions

    private func autoSelectDefaults() {
        if sourceAccountIndex == nil {
            sourceAccountIndex = platformAccountOptions
                .first(where: { $0.totalCredits > 0 })?.accountIndex
                ?? platformAccountOptions.first?.accountIndex
        }
        if destinationMode == .myWallet {
            resolveMyWalletAddress()
        }
    }

    /// Gate the whole flow on the Core (SPV) wallet being usable.
    /// `coreWallet()` throws if the Core side isn't initialized; we
    /// also probe `nextReceiveAddress` so a half-initialized wallet
    /// surfaces here rather than at submit time.
    private func checkCoreReady() {
        guard let managedHolder = walletManager.wallet(for: wallet.walletId) else {
            coreReady = false
            coreNotReadyReason = "Wallet handle not found in the wallet manager."
            return
        }
        do {
            let core = try managedHolder.coreWallet()
            _ = try core.nextReceiveAddress(accountIndex: 0)
            coreReady = true
        } catch {
            coreReady = false
            coreNotReadyReason = "Core wallet is not ready: \(error.localizedDescription)"
        }
    }

    private func resolveMyWalletAddress() {
        guard myWalletAddress == nil else { return }
        guard let managedHolder = walletManager.wallet(for: wallet.walletId) else { return }
        do {
            let core = try managedHolder.coreWallet()
            myWalletAddress = try core.nextReceiveAddress(accountIndex: 0)
        } catch {
            // Leave nil; the destination section shows the resolving
            // placeholder and Core-readiness gating handles the rest.
            myWalletAddress = nil
        }
    }

    private func submit() {
        guard !isSubmitting else { return }
        guard
            let sourceAccount = sourceAccountIndex,
            let feePerByte = parsedFeePerByte,
            let coreAddress = resolvedCoreAddress
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
                _ = try await addressWallet.withdraw(
                    accountIndex: sourceAccount,
                    coreAddress: coreAddress,
                    coreFeePerByte: feePerByte,
                    signer: signer
                )
                await platformBalanceSyncService.performSync()
                didSucceed = true
            } catch {
                submitError = SubmitError(message: error.localizedDescription)
            }
        }
    }

    // MARK: - Helpers

    private func formatCredits(_ credits: UInt64) -> String {
        let dash = Double(credits) / Self.creditsPerDash
        return String(format: "%.6f DASH", dash)
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
