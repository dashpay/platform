// TransferPlatformAddressView.swift
// SwiftExampleApp
//
// Production (wallet-signed) UI for ADDR-02: transfer credits between
// Platform (DIP-17) addresses. Mirrors the shape of
// `FundFromAssetLockPlatformAddressView` (Source → Destination → Amount
// → Submit) and drives `ManagedPlatformAddressWallet.transfer(...)`
// end-to-end with a `KeychainSigner`.
//
// No private keys are ever entered here. Input selection, change
// routing, fee strategy, nonce selection (Auto), and signing all
// happen inside the Rust `platform-wallet` crate via the FFI wrapper —
// the only thing this view decides is the source account, the amount,
// the destination address, and which (unused) wallet address to route
// change to. Contrast with the raw `TransferAddressFundsView` debug
// form, which pastes a 64-char private key.

import SwiftUI
import SwiftDashSDK
import SwiftData

struct TransferPlatformAddressView: View {
    @Environment(\.dismiss) private var dismiss
    @Environment(\.modelContext) private var modelContext
    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var platformBalanceSyncService: PlatformBalanceSyncService

    /// Wallet whose DIP-17 platform-payment accounts/addresses this
    /// transfer operates on.
    let wallet: PersistentWallet

    @Query private var allAccounts: [PersistentAccount]
    @Query private var allPlatformAddresses: [PersistentPlatformAddress]

    // MARK: - Selection state

    private enum DestinationMode: String, CaseIterable, Identifiable {
        case ownWallet = "My Wallet"
        case external = "External"
        var id: String { rawValue }
    }

    @State private var sourceAccountIndex: UInt32? = nil
    @State private var destinationMode: DestinationMode = .ownWallet
    /// Selected own-wallet recipient (20-byte hash) when mode == .ownWallet.
    @State private var selectedRecipientHash: Data? = nil
    /// Pasted/scanned external recipient hash, 40 hex chars (20 bytes).
    @State private var externalHashHex: String = ""
    @State private var amountDash: String = "0.0001"

    // MARK: - Submit state

    @State private var submitError: SubmitError? = nil
    @State private var isSubmitting = false
    @State private var didSucceed = false

    /// 1e11 credits per DASH. Matches `CreateIdentityView`.
    private static let creditsPerDash: Double = 100_000_000_000.0

    /// Mirror of `ManagedPlatformAddressWallet.feeBuffer` (held back so
    /// the change output survives the on-chain fee). Used here only to
    /// gate the submit button with the same accounting the wrapper
    /// enforces — the wrapper still throws if this is violated.
    private static let feeBuffer: UInt64 = 100_000_000

    var body: some View {
        NavigationStack {
            Form {
                if didSucceed {
                    successSection
                } else {
                    walletSection
                    sourceAccountSection
                    destinationSection
                    amountSection
                    if canSubmit {
                        submitSection
                    }
                }
            }
            .navigationTitle("Transfer Platform Credits")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") { dismiss() }
                        .disabled(isSubmitting)
                }
            }
            .alert(item: $submitError) { err in
                Alert(
                    title: Text("Could not transfer credits"),
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
                .accessibilityIdentifier("transferPlatform.sourceAccountPicker")
                .onChange(of: sourceAccountIndex) { _, _ in
                    selectedRecipientHash = nil
                    autoSelectRecipient()
                }
            }
        } header: {
            Text("Source Account")
        } footer: {
            Text("Platform Payment account funding the transfer. Picker shows its current credit balance.")
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
            .accessibilityIdentifier("transferPlatform.destinationModePicker")

            switch destinationMode {
            case .ownWallet:
                let options = ownWalletRecipientCandidates
                if options.isEmpty {
                    Text("No other addresses available on this wallet to receive credits. Sync first or add funds.")
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
                    .accessibilityIdentifier("transferPlatform.recipientPicker")
                }
            case .external:
                VStack(alignment: .leading, spacing: 4) {
                    TextField("Recipient hash (40 hex chars = 20 bytes)", text: $externalHashHex)
                        .textFieldStyle(.roundedBorder)
                        .autocapitalization(.none)
                        .disableAutocorrection(true)
                        .monospaced()
                        .accessibilityIdentifier("transferPlatform.externalHashField")
                    if !externalHashHex.isEmpty && parsedExternalHash == nil {
                        Text("Enter exactly 40 hexadecimal characters.")
                            .font(.caption)
                            .foregroundColor(.red)
                    }
                }
            }
        } header: {
            Text("Destination Address")
        } footer: {
            Text("Send to another address on this wallet, or paste a 20-byte P2PKH address hash. Change routes automatically to a fresh unused address.")
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
                    .accessibilityIdentifier("transferPlatform.amountField")
                Text("DASH")
                    .foregroundColor(.secondary)
            }
        } header: {
            Text("Amount")
        } footer: {
            if let credits = parsedCredits {
                let available = selectedSourceAccountCredits
                if credits + Self.feeBuffer > available {
                    Text("Insufficient balance: \(formatCredits(credits)) + fee exceeds the account's \(formatCredits(available)).")
                        .foregroundColor(.red)
                } else {
                    Text("\(formatCredits(credits)) will be transferred (plus a small on-chain fee held back from change).")
                }
            } else {
                Text("Enter an amount in DASH.")
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
                        ProgressView()
                        Text("Transferring…")
                    } else {
                        Text("Transfer")
                    }
                    Spacer()
                }
                .foregroundColor(.white)
            }
            .frame(maxWidth: .infinity)
            .listRowBackground(Color.accentColor)
            .disabled(isSubmitting)
            .accessibilityIdentifier("transferPlatform.submitButton")
        }
    }

    private var successSection: some View {
        Section {
            VStack(alignment: .leading, spacing: 8) {
                Label("Credits transferred", systemImage: "checkmark.seal.fill")
                    .foregroundColor(.green)
                    .font(.headline)
                Text("The transfer was submitted and your balances are resyncing.")
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

    /// Own-wallet recipients: any address on the wallet that is NOT the
    /// auto-selected change address and NOT a source-account input that
    /// holds balance. We surface unused (zero-balance) addresses on any
    /// platform-payment account so the user can send to a fresh address;
    /// the FFI wrapper rejects a recipient that collides with an input.
    private var ownWalletRecipientCandidates: [PersistentPlatformAddress] {
        let changeHash = autoChangeAddress?.addressHash
        return allPlatformAddresses
            .filter { $0.walletId == wallet.walletId }
            .filter { $0.addressHash != changeHash }
            .sorted { ($0.accountIndex, $0.addressIndex) < ($1.accountIndex, $1.addressIndex) }
    }

    /// Lowest-index unused, zero-balance address on the source account —
    /// the change destination. Picked internally; never exposed in the UI.
    private var autoChangeAddress: PersistentPlatformAddress? {
        guard let acctIdx = sourceAccountIndex else { return nil }
        return allPlatformAddresses
            .filter {
                $0.walletId == wallet.walletId
                    && $0.accountIndex == acctIdx
                    && !$0.isUsed
                    && $0.balance == 0
            }
            .sorted { $0.addressIndex < $1.addressIndex }
            .first
    }

    /// Parse the pasted external hash (40 hex chars → 20 bytes).
    private var parsedExternalHash: Data? {
        let raw = externalHashHex.trimmingCharacters(in: .whitespacesAndNewlines)
        guard raw.count == 40 else { return nil }
        var bytes = [UInt8]()
        bytes.reserveCapacity(20)
        var idx = raw.startIndex
        while idx < raw.endIndex {
            let next = raw.index(idx, offsetBy: 2)
            guard let b = UInt8(raw[idx..<next], radix: 16) else { return nil }
            bytes.append(b)
            idx = next
        }
        return Data(bytes)
    }

    /// Resolved destination (addressType, 20-byte hash) for the current mode.
    private var resolvedDestination: (addressType: UInt8, hash: Data)? {
        switch destinationMode {
        case .ownWallet:
            guard let hash = selectedRecipientHash,
                let row = allPlatformAddresses.first(where: { $0.addressHash == hash })
            else { return nil }
            return (row.addressType, row.addressHash)
        case .external:
            guard let hash = parsedExternalHash else { return nil }
            // External pasted hashes are treated as P2PKH (type 0).
            return (0, hash)
        }
    }

    private var parsedCredits: UInt64? {
        let raw = amountDash.trimmingCharacters(in: .whitespacesAndNewlines)
        guard let dash = Double(raw), dash > 0 else { return nil }
        let creditsDouble = dash * Self.creditsPerDash
        guard creditsDouble.isFinite, creditsDouble <= Double(UInt64.max) else { return nil }
        return UInt64(creditsDouble.rounded(.toNearestOrAwayFromZero))
    }

    private var canSubmit: Bool {
        guard
            !isSubmitting,
            sourceAccountIndex != nil,
            let credits = parsedCredits, credits > 0,
            let dest = resolvedDestination,
            autoChangeAddress != nil
        else { return false }
        // Reject change-address / recipient collision up front (the
        // wrapper rejects it too, but a dead button is worse UX).
        if dest.hash == autoChangeAddress?.addressHash { return false }
        // Gate on amount + fee buffer <= account balance.
        let needed = credits.addingReportingOverflow(Self.feeBuffer)
        if needed.overflow { return false }
        return selectedSourceAccountCredits >= needed.partialValue
    }

    // MARK: - Actions

    private func autoSelectDefaults() {
        if sourceAccountIndex == nil {
            sourceAccountIndex = platformAccountOptions
                .first(where: { $0.totalCredits > 0 })?.accountIndex
                ?? platformAccountOptions.first?.accountIndex
        }
        autoSelectRecipient()
    }

    private func autoSelectRecipient() {
        if destinationMode == .ownWallet && selectedRecipientHash == nil {
            selectedRecipientHash = ownWalletRecipientCandidates.first?.addressHash
        }
    }

    private func submit() {
        guard !isSubmitting else { return }
        guard
            let sourceAccount = sourceAccountIndex,
            let credits = parsedCredits,
            let dest = resolvedDestination,
            let change = autoChangeAddress
        else { return }

        guard dest.hash != change.addressHash else {
            submitError = SubmitError(
                message: "The destination collides with the auto-selected change address. Pick a different recipient."
            )
            return
        }

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
        let outputs = [
            ManagedPlatformAddressWallet.TransferOutput(
                addressType: dest.addressType,
                hash: dest.hash,
                credits: credits
            )
        ]
        let changeAddress = ManagedPlatformAddressWallet.ChangeAddress(
            addressType: change.addressType,
            hash: change.addressHash
        )

        isSubmitting = true
        Task {
            defer { isSubmitting = false }
            do {
                _ = try await addressWallet.transfer(
                    accountIndex: sourceAccount,
                    outputs: outputs,
                    changeAddress: changeAddress,
                    signer: signer
                )
                // Trigger a DIP-17 resync so balances + the unused-
                // address pool catch up after the transfer.
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
