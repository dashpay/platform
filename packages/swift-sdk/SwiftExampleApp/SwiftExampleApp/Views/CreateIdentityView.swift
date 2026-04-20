// CreateIdentityView.swift
// SwiftExampleApp
//
// Stepped UI for spinning up a new Dash Platform identity. The
// workflow chooses a funding source in two passes:
//
//   1. Source Wallet — one of the local HDWallet rows, or
//      "Create without Wallet" for the advanced path where the caller
//      supplies a raw asset-lock proof.
//   2. When a wallet is chosen: either a PersistentAccount on that
//      wallet (any type — Core pools and Platform Payment both work)
//      or "Fund from unused Asset Lock".
//
// The first-pass implementation only wires the Platform Payment
// funding path — see `submit()`. Core / CoinJoin / walletless paths
// are still stubs pending their respective FFI entry points.

import SwiftUI
import SwiftDashSDK
import SwiftData

struct CreateIdentityView: View {
    @Environment(\.dismiss) private var dismiss
    @Environment(\.modelContext) private var modelContext
    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var platformState: AppState

    /// Default number of Platform identity authentication keys to
    /// register in this first-pass flow. First key is MASTER, the
    /// rest are HIGH. Advanced override is intentionally not exposed
    /// here yet.
    private static let defaultKeyCount: UInt32 = 3

    /// Credits per DASH (1e11) — the divisor used for Platform-side
    /// credit amounts. Duplicated from `PersistentPlatformAddress`
    /// docstring; kept here so the conversion logic stays local.
    private static let creditsPerDash: UInt64 = 100_000_000_000

    /// All locally-persisted wallets. Drives the Source Wallet
    /// picker along with the synthetic "no wallet" sentinel.
    @Query(sort: \HDWallet.createdAt) private var wallets: [HDWallet]

    /// All persisted accounts across wallets. Filtered per-selection
    /// inside `accountOptions(for:)` so switching wallets doesn't
    /// re-fire a SwiftData query.
    @Query private var allAccounts: [PersistentAccount]

    // MARK: - Selection state

    /// The source wallet selection. `nil` encodes "pick nothing yet";
    /// `.walletless` encodes the explicit "Create without Wallet"
    /// choice that switches step 2 to the raw asset-lock path.
    @State private var walletSelection: WalletSelection? = nil

    /// Chosen funding source when a wallet is selected.
    @State private var fundingSelection: FundingSelection? = nil

    /// Identity registration key slot to consume (for wallet-backed
    /// paths). `nil` until a wallet selection populates it with the
    /// first unused index; the user can override via the picker.
    @State private var identityIndex: UInt32? = nil

    /// Raw asset-lock proof text, used only in the walletless path.
    /// Accepted encoding is base64 or lowercase hex — the submit
    /// logic (future) will detect + decode.
    @State private var walletlessProof: String = ""

    /// Amount (in DASH) to fund the new identity with. Populated
    /// automatically from the selected account's balance; the user
    /// can lower it but not exceed the available balance.
    @State private var amountDash: String = ""

    // MARK: - Submit state

    /// True while the FFI `registerIdentityFromAddresses` call is in
    /// flight. Used to swap the submit button for a progress
    /// indicator and block input.
    @State private var isCreating: Bool = false

    /// User-facing error surfaced via the `.alert` modifier.
    @State private var submitError: SubmitError? = nil

    /// Success payload. Populated after the identity is persisted;
    /// the submit section swaps to a success banner and auto-dismiss.
    @State private var createdIdentityId: Data? = nil

    var body: some View {
        NavigationStack {
            Form {
                sourceWalletSection
                fundingSection
                amountSection
                identityIndexSection
                if createdIdentityId != nil {
                    successSection
                } else if canSubmit {
                    submitSection
                }
            }
            .navigationTitle("Create Identity")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") { dismiss() }
                        .disabled(isCreating)
                }
            }
            .alert(item: $submitError) { err in
                Alert(
                    title: Text("Could not create identity"),
                    message: Text(err.message),
                    dismissButton: .default(Text("OK"))
                )
            }
        }
    }

    // MARK: - Sections

    private var sourceWalletSection: some View {
        Section {
            Picker("Source Wallet", selection: $walletSelection) {
                Text("Select…")
                    .tag(Optional<WalletSelection>.none)
                ForEach(wallets) { wallet in
                    Text(walletLabel(for: wallet))
                        .tag(Optional(WalletSelection.wallet(id: wallet.walletId)))
                }
                Divider()
                Text("Create without Wallet")
                    .tag(Optional(WalletSelection.walletless))
            }
            .onChange(of: walletSelection) { _, newValue in
                // Reset downstream selection whenever the wallet
                // changes so a stale account / proof can't leak
                // through.
                fundingSelection = nil
                walletlessProof = ""
                // Default the identity-registration index to the
                // first unused slot on the newly-selected wallet,
                // or clear it for the walletless / no-selection
                // branches.
                if case .wallet(let walletId) = newValue {
                    identityIndex = firstUnusedIdentityIndex(for: walletId)
                } else {
                    identityIndex = nil
                }
            }
        } header: {
            Text("Source Wallet")
        } footer: {
            Text(
                "Pick a wallet to fund the identity from one of its accounts, "
                + "or Create without Wallet to supply a raw asset-lock proof."
            )
        }
    }

    @ViewBuilder
    private var fundingSection: some View {
        switch walletSelection {
        case .none:
            EmptyView()
        case .walletless:
            walletlessSection
        case .wallet(let walletId):
            walletAccountSection(for: walletId)
        }
    }

    @ViewBuilder
    private func walletAccountSection(for walletId: Data) -> some View {
        let options = accountOptions(for: walletId)
        Section {
            Picker("Funding Source", selection: $fundingSelection) {
                Text("Select…")
                    .tag(Optional<FundingSelection>.none)
                ForEach(options) { option in
                    Text("\(option.label) — \(option.balanceText)")
                        .tag(Optional(FundingSelection.account(id: option.persistentId)))
                }
                Divider()
                Text("Fund from unused Asset Lock")
                    .tag(Optional(FundingSelection.unusedAssetLock))
            }
            .onChange(of: fundingSelection) { _, newValue in
                // Pre-fill the amount with the full available balance
                // of the selected Platform Payment account so the
                // happy path is one tap. Users can dial it down.
                amountDash = defaultAmountString(for: newValue)
            }
        } header: {
            Text("Funding Source")
        } footer: {
            Text(
                "Any account on the selected wallet with a balance can fund "
                + "the identity — Core or Platform Payment. Empty accounts "
                + "are hidden. \"Fund from unused Asset Lock\" picks an "
                + "existing tracked asset lock instead."
            )
        }
    }

    /// Amount (in DASH) to fund the new identity with. Only shown
    /// once the user has picked a funding source the current flow
    /// can actually spend from (Platform Payment account).
    @ViewBuilder
    private var amountSection: some View {
        if let account = selectedPlatformAccount {
            Section {
                HStack {
                    TextField("Amount", text: $amountDash)
                        .keyboardType(.decimalPad)
                        .textFieldStyle(.roundedBorder)
                        .disabled(isCreating)
                    Text("DASH")
                        .foregroundColor(.secondary)
                }
            } header: {
                Text("Amount")
            } footer: {
                let available = Self.formatDash(
                    raw: accountBalance(account),
                    divisor: Double(Self.creditsPerDash)
                )
                Text("Available: \(available). The new identity will start with this amount funded from the selected addresses.")
            }
        }
    }

    private var walletlessSection: some View {
        Section {
            TextEditor(text: $walletlessProof)
                .font(.system(.footnote, design: .monospaced))
                .frame(minHeight: 120)
                .autocorrectionDisabled()
                .textInputAutocapitalization(.never)
        } header: {
            Text("Asset Lock Proof")
        } footer: {
            Text("Paste the raw proof as base64 or hex.")
        }
    }

    @ViewBuilder
    private var identityIndexSection: some View {
        // Only relevant when a wallet is the source. Walletless
        // creations don't burn an identity-registration slot off our
        // HD tree.
        if case .wallet(let walletId) = walletSelection {
            let unused = unusedIdentityIndices(for: walletId)
            Section {
                if unused.isEmpty {
                    Text("No unused identity registration keys available.")
                        .font(.subheadline)
                        .foregroundColor(.secondary)
                } else {
                    Picker("Identity Registration Index", selection: $identityIndex) {
                        ForEach(unused, id: \.self) { index in
                            Text("#\(index)")
                                .tag(Optional(index))
                        }
                    }
                }
            } header: {
                Text("Identity Registration Index")
            } footer: {
                Text(
                    "The identity-registration key slot the new identity "
                    + "will consume. Defaults to the lowest unused slot in "
                    + "the wallet; override to pick any other unused index."
                )
            }
        }
    }

    private var submitSection: some View {
        Section {
            Button {
                submit()
            } label: {
                HStack {
                    if isCreating {
                        ProgressView()
                            .controlSize(.small)
                            .tint(.white)
                        Text("Creating Identity…")
                    } else {
                        Text("Create Identity")
                    }
                }
                .frame(maxWidth: .infinity)
            }
            .buttonStyle(.borderedProminent)
            .disabled(!canSubmit || isCreating)
        }
    }

    /// Success banner + "Done" button shown after the identity is
    /// registered and persisted. Replaces the submit section so
    /// the user can't accidentally double-submit.
    private var successSection: some View {
        Section {
            VStack(alignment: .leading, spacing: 8) {
                Label("Identity created", systemImage: "checkmark.seal.fill")
                    .foregroundColor(.green)
                    .font(.headline)
                if let id = createdIdentityId {
                    Text(id.toBase58String())
                        .font(.system(.caption, design: .monospaced))
                        .foregroundColor(.secondary)
                        .textSelection(.enabled)
                }
                Button {
                    dismiss()
                } label: {
                    Text("Done")
                        .frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)
                .padding(.top, 4)
            }
        }
    }

    // MARK: - Derived state

    /// Whether the current selection is complete enough that the
    /// submit button should light up. Non-empty hex / base64 content
    /// in the walletless path, or a concrete account + funding choice
    /// + an identity-registration index on the wallet path. For the
    /// Platform-payment path we additionally require a positive
    /// amount that doesn't exceed the account balance.
    private var canSubmit: Bool {
        switch (walletSelection, fundingSelection) {
        case (.walletless, _):
            return !walletlessProof
                .trimmingCharacters(in: .whitespacesAndNewlines)
                .isEmpty
        case (.wallet, .some):
            guard identityIndex != nil else { return false }
            if let account = selectedPlatformAccount {
                guard let credits = parsedAmountCredits else { return false }
                return credits > 0 && credits <= accountBalance(account)
            }
            // Non-Platform-payment wallet-backed paths are still
            // stubbed — don't light the button until they're wired.
            return false
        default:
            return false
        }
    }

    // MARK: - Submit

    /// Runs the Platform-payment-funded identity registration path.
    /// Other funding branches are intentionally stubbed — the button
    /// stays disabled for them via `canSubmit`.
    private func submit() {
        guard
            let account = selectedPlatformAccount,
            let identityIndex = identityIndex,
            let targetCredits = parsedAmountCredits,
            targetCredits > 0,
            case .wallet(let walletId) = walletSelection
        else {
            submitError = .init(message: "Selection is incomplete.")
            return
        }
        // The Rust manager holds every loaded wallet — route by id
        // rather than assuming a single active wallet. If the
        // selected wallet isn't loaded (create flow never ran /
        // restore failed) we surface a concrete error.
        guard let managedWallet = walletManager.wallet(for: walletId) else {
            submitError = .init(message: "The selected wallet isn't loaded. Try restoring it from the wallets list first.")
            return
        }

        // Greedy-select addresses to cover `targetCredits`. The
        // last address's credits field is capped to the remaining
        // amount so the total spent matches exactly.
        let inputs = buildInputs(
            from: account,
            targetCredits: targetCredits
        )
        guard !inputs.isEmpty else {
            submitError = .init(message: "No funded Platform addresses available on this account.")
            return
        }

        isCreating = true

        let networkRaw: String = platformState.currentNetwork.rawValue

        Task {
            do {
                let created = try await managedWallet.registerIdentityFromAddresses(
                    inputs: inputs,
                    output: nil,
                    identityIndex: identityIndex,
                    keyCount: Self.defaultKeyCount
                )

                try await MainActor.run {
                    try persistCreatedIdentity(
                        created,
                        walletId: walletId,
                        networkRaw: networkRaw,
                        initialCreditBalance: Int64(targetCredits)
                    )
                    markIdentitySlotUsed(
                        walletId: walletId,
                        identityIndex: identityIndex
                    )
                    try modelContext.save()
                    self.createdIdentityId = created.identityId
                    self.isCreating = false
                }
            } catch {
                await MainActor.run {
                    self.submitError = .init(
                        message: error.localizedDescription
                    )
                    self.isCreating = false
                }
            }
        }
    }

    /// Convert the selected Platform Payment account's
    /// `PersistentPlatformAddress` rows into the flat FFI input list,
    /// stopping once we have enough credits to cover `targetCredits`.
    /// Addresses are sorted by balance descending to minimize the
    /// number of spent inputs.
    private func buildInputs(
        from account: PersistentAccount,
        targetCredits: UInt64
    ) -> [ManagedPlatformWallet.IdentityAddressInput] {
        let candidates = account.platformAddresses
            .filter { $0.balance > 0 }
            .sorted { $0.balance > $1.balance }

        var remaining = targetCredits
        var inputs: [ManagedPlatformWallet.IdentityAddressInput] = []
        for addr in candidates {
            guard remaining > 0 else { break }
            let spend = min(addr.balance, remaining)
            inputs.append(
                ManagedPlatformWallet.IdentityAddressInput(
                    addressType: addr.addressType,
                    hash: addr.addressHash,
                    nonce: addr.nonce,
                    credits: spend
                )
            )
            remaining -= spend
        }
        // If we couldn't cover the target, return empty — `submit`
        // surfaces this as "no funded addresses", which matches the
        // practical cause (account underfunded by the time the user
        // tapped).
        return remaining == 0 ? inputs : []
    }

    /// Insert a fully-populated `PersistentIdentity` row together
    /// with one `PersistentPublicKey` per key registered on the
    /// identity. Pulls revision + keys off the returned
    /// `ManagedIdentity` via the FFI — the wallet's local identity
    /// manager stays the source of truth, this is the SwiftData
    /// mirror that drives the UI.
    ///
    /// Private-key material is intentionally not stashed here. The
    /// authentication keys were derived from the wallet seed at
    /// DIP-9 paths; re-deriving is cheaper and safer than
    /// duplicating them into the Keychain. A follow-up will add a
    /// `derivationPath` column so we can round-trip back to the
    /// seed when a signer is needed.
    private func persistCreatedIdentity(
        _ created: ManagedPlatformWallet.CreatedIdentity,
        walletId: Data,
        networkRaw: String,
        initialCreditBalance: Int64
    ) throws {
        let revision = (try? created.identity.getRevision()) ?? 0
        let publicKeys = (try? created.identity.getPublicKeys()) ?? []

        let identity = PersistentIdentity(
            identityId: created.identityId,
            balance: initialCreditBalance,
            revision: Int64(revision),
            isLocal: true,
            identityType: .user,
            network: networkRaw,
            walletId: walletId
        )

        let identityIdString = created.identityId.toHexString()
        for info in publicKeys {
            let persistent = PersistentPublicKey(
                keyId: info.keyId,
                purpose: info.purpose,
                securityLevel: info.securityLevel,
                keyType: info.keyType,
                publicKeyData: info.data,
                readOnly: info.readOnly,
                disabledAt: info.disabledAt,
                contractBounds: nil,
                identityId: identityIdString
            )
            identity.addPublicKey(persistent)
        }

        modelContext.insert(identity)
    }

    /// Flip `isUsed` on the consumed identity-registration slot so
    /// the next call to `unusedIdentityIndices` skips it. Silent
    /// no-op if the slot isn't found — this is cosmetic bookkeeping
    /// and the Rust side is already the source of truth.
    private func markIdentitySlotUsed(
        walletId: Data,
        identityIndex: UInt32
    ) {
        guard let account = identityRegistrationAccount(for: walletId) else {
            return
        }
        if let slot = account.coreAddresses.first(where: {
            $0.addressIndex == identityIndex
        }) {
            slot.isUsed = true
        }
    }

    /// The currently-selected Platform Payment account, if any.
    /// Everything downstream (amount section, inputs builder, submit
    /// gate) keys off this.
    private var selectedPlatformAccount: PersistentAccount? {
        guard
            case .account(let persistentId) = fundingSelection,
            let account = allAccounts.first(where: {
                $0.persistentModelID == persistentId
            }),
            account.accountType == 14
        else {
            return nil
        }
        return account
    }

    /// Raw credit balance across all addresses in a PlatformPayment
    /// account.
    private func accountBalance(_ account: PersistentAccount) -> UInt64 {
        account.platformAddresses.reduce(0) { $0 + $1.balance }
    }

    /// Default amount string (DASH) for the amount field — the full
    /// balance of the selected Platform Payment account.
    private func defaultAmountString(for funding: FundingSelection?) -> String {
        guard
            case .account(let persistentId) = funding,
            let account = allAccounts.first(where: {
                $0.persistentModelID == persistentId
            }),
            account.accountType == 14
        else {
            return ""
        }
        let balance = accountBalance(account)
        if balance == 0 { return "" }
        let dash = Double(balance) / Double(Self.creditsPerDash)
        return String(format: "%g", dash)
    }

    /// Parse the amount text back into credits. Returns `nil` on
    /// invalid / negative / overflow input.
    private var parsedAmountCredits: UInt64? {
        let trimmed = amountDash.trimmingCharacters(in: .whitespaces)
        guard let dash = Double(trimmed), dash.isFinite, dash > 0 else {
            return nil
        }
        // Round to nearest credit to avoid floating-point dust.
        let credits = (dash * Double(Self.creditsPerDash)).rounded()
        guard credits >= 1, credits <= Double(UInt64.max) else { return nil }
        return UInt64(credits)
    }

    // MARK: - Helpers

    private func walletLabel(for wallet: HDWallet) -> String {
        let trimmed = wallet.label.trimmingCharacters(in: .whitespaces)
        let base = trimmed.isEmpty ? shortWalletId(wallet.walletId) : trimmed
        return "\(base) (\(wallet.network.rawValue))"
    }

    private func shortWalletId(_ walletId: Data) -> String {
        let prefix = walletId.prefix(4).map { String(format: "%02x", $0) }.joined()
        return prefix.isEmpty ? "Wallet" : "Wallet \(prefix)…"
    }

    /// Turn a wallet's PersistentAccounts into the funding-picker
    /// rows. Restricted to accounts that actually hold spendable
    /// funds — Core Standard (BIP44 / BIP32), CoinJoin, and
    /// PlatformPayment — AND filtered to a non-zero balance.
    /// Identity / provider / asset-lock-topup accounts are
    /// intentionally excluded; they aren't sources of funds. Empty
    /// accounts are filtered out rather than greyed, because
    /// SwiftUI's menu-style Picker strips visual-dim modifiers on
    /// child rows. Ordering matches `AccountListView`: BIP44 →
    /// PlatformPayment → BIP32 → CoinJoin.
    private func accountOptions(for walletId: Data) -> [FundingAccountOption] {
        allAccounts
            .filter { account in
                guard account.wallet?.walletId == walletId else { return false }
                guard CreateIdentityView.isFundingAccount(account) else { return false }
                return CreateIdentityView.accountBalanceSummary(account).hasBalance
            }
            .sorted { lhs, rhs in
                let lhsKey = CreateIdentityView.sortKey(for: lhs)
                let rhsKey = CreateIdentityView.sortKey(for: rhs)
                return lhsKey < rhsKey
            }
            .map { account in
                let (_, balanceText) = Self.accountBalanceSummary(account)
                return FundingAccountOption(
                    persistentId: account.persistentModelID,
                    label: Self.fundingLabel(for: account),
                    balanceText: balanceText
                )
            }
    }

    /// Unused identity-registration key indices on the wallet's
    /// Identity Registration account (FFI type tag 2). Each
    /// `PersistentCoreAddress` under that account represents one
    /// registration slot keyed by `addressIndex`; `isUsed` flips to
    /// true once the slot has been consumed by a prior identity
    /// creation. Returns an ascending list of the remaining slots.
    private func unusedIdentityIndices(for walletId: Data) -> [UInt32] {
        guard let account = identityRegistrationAccount(for: walletId) else {
            return []
        }
        return account.coreAddresses
            .filter { !$0.isUsed }
            .map { $0.addressIndex }
            .sorted()
    }

    /// Lowest unused identity-registration index on a wallet, or
    /// `nil` if no slots remain. Drives the picker's default value.
    private func firstUnusedIdentityIndex(for walletId: Data) -> UInt32? {
        unusedIdentityIndices(for: walletId).first
    }

    private func identityRegistrationAccount(for walletId: Data) -> PersistentAccount? {
        allAccounts.first { account in
            account.wallet?.walletId == walletId && account.accountType == 2
        }
    }

    /// Account types eligible to fund a new identity.
    private static func isFundingAccount(_ account: PersistentAccount) -> Bool {
        switch account.accountType {
        case 0, 1, 14: return true
        default: return false
        }
    }

    /// Formatted balance for the picker row and the disabled flag.
    /// Core / CoinJoin use the SPV-maintained
    /// `balanceConfirmed + balanceUnconfirmed` duffs (1e8/DASH);
    /// PlatformPayment sums the BLAST-synced credit balances across
    /// its addresses (1e11/DASH).
    private static func accountBalanceSummary(
        _ account: PersistentAccount
    ) -> (hasBalance: Bool, balanceText: String) {
        switch account.accountType {
        case 14:
            let credits = account.platformAddresses.reduce(0) { $0 + $1.balance }
            return (
                credits > 0,
                credits > 0 ? formatDash(raw: credits, divisor: 100_000_000_000.0) : "empty"
            )
        default:
            let duffs = account.balanceConfirmed + account.balanceUnconfirmed
            return (
                duffs > 0,
                duffs > 0 ? formatDash(raw: duffs, divisor: 100_000_000.0) : "empty"
            )
        }
    }

    /// `"0.01 DASH"` — stripped of trailing zeros, uses up to 8 decimals.
    private static func formatDash(raw: UInt64, divisor: Double) -> String {
        let dash = Double(raw) / divisor
        let fmt = NumberFormatter()
        fmt.minimumFractionDigits = 0
        fmt.maximumFractionDigits = 8
        fmt.numberStyle = .decimal
        fmt.groupingSeparator = ","
        fmt.decimalSeparator = "."
        return (fmt.string(from: NSNumber(value: dash)) ?? String(format: "%.8f", dash)) + " DASH"
    }

    private static func fundingLabel(for account: PersistentAccount) -> String {
        "\(account.accountTypeName) #\(account.accountIndex)"
    }

    private static func sortKey(
        for account: PersistentAccount
    ) -> (UInt8, UInt32, UInt8, UInt32) {
        let group: UInt8
        switch account.accountType {
        case 0: group = account.standardTag == 0 ? 0 : 2
        case 14: group = 1
        case 1: group = 3
        default: group = 4
        }
        return (group, account.accountType, account.standardTag, account.accountIndex)
    }
}

// MARK: - Selection types

private enum WalletSelection: Hashable {
    case wallet(id: Data)
    case walletless
}

private enum FundingSelection: Hashable {
    case account(id: PersistentIdentifier)
    case unusedAssetLock
}

private struct FundingAccountOption: Identifiable {
    let persistentId: PersistentIdentifier
    let label: String
    /// Human-readable balance suffix (`"0.01 DASH"`). Zero-balance
    /// accounts are filtered out upstream so this is always a
    /// positive amount.
    let balanceText: String
    var id: PersistentIdentifier { persistentId }
}

/// Wrapper so SwiftUI's `.alert(item:)` can render a fresh alert each
/// time the error changes.
private struct SubmitError: Identifiable {
    let id = UUID()
    let message: String
}
