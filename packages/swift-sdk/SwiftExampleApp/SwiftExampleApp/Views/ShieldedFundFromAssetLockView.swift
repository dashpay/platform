// ShieldedFundFromAssetLockView.swift
// SwiftExampleApp
//
// Stepped UI for shielding credits from a Core (SPV) wallet balance
// into a wallet's Orchard (Type 18) pool. Drives
// `PlatformWalletManager.shieldedFundFromAssetLock(...)` end-to-end:
//
//   1. Build an asset-lock tx from the chosen Core BIP44 account.
//   2. Wait for the IS-lock (or fall back to ChainLock on timeout).
//   3. Build a Halo 2 proof (~30s) and submit a
//      `ShieldFromAssetLockTransition` against the asset-lock proof.
//   4. Mark the asset lock `Consumed` on success.
//
// No private keys cross the FFI boundary on this path — both the
// Core-side derivation (inside the wallet's asset-lock manager) and
// the outer state-transition signature route through a local
// `MnemonicResolver`, atomic per call.
//
// Differences vs. `FundFromAssetLockPlatformAddressView`:
//   * No platform-account picker — shielded recipients are external
//     Orchard payment addresses, not allocated from a wallet
//     account.
//   * The recipient defaults to the wallet's own bound shielded
//     default address (the natural demo case). The user can paste a
//     43-byte raw Orchard address (display hex) to override.
//   * Both `amountDuffs` (L1) and `shieldAmountCredits` (what enters
//     the pool) are exposed — the Rust orchestration takes both
//     because Type 18's Orchard `value_balance` is baked into the
//     Halo 2 proof at build time and can't be derived by Platform.
//   * No post-success recipient back-fill — shielded asset-lock
//     rows don't carry a per-account recipient stamp (the recipient
//     is an external Orchard address, not allocated from the wallet).

import SwiftUI
import SwiftDashSDK
import SwiftData

struct ShieldedFundFromAssetLockView: View {
    @Environment(\.dismiss) private var dismiss
    @Environment(\.modelContext) private var modelContext
    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var platformState: AppState

    /// Wallet to shield from / to. Drives both the picker scope
    /// (Core BIP44 accounts on this wallet only) and the managed-
    /// wallet lookup at submit time. The recipient defaults to
    /// this wallet's own bound shielded default address.
    let wallet: PersistentWallet

    /// Optional asset lock to resume from. When non-nil the view
    /// hides the Core-funding-account + amount sections (the asset
    /// lock already exists, those choices were made at original
    /// build time) and routes Submit to
    /// `PlatformWalletManager.shieldedResumeFundFromAssetLock`
    /// instead of building a fresh lock. The user can still adjust
    /// the recipient + shield amount since the orphan lock doesn't
    /// carry that — both are set at ST-submission time.
    var resumeFromLock: PersistentAssetLock? = nil

    // MARK: - Selection state

    @State private var fundingCoreAccountIndex: UInt32? = nil
    /// 43-byte raw recipient Orchard address. Defaults to the
    /// wallet's bound shielded default in `autoSelectDefaults`.
    @State private var recipientRaw43: Data? = nil
    /// User-facing display hex for the recipient (86 chars). Bound
    /// to the override text field so the user can paste a different
    /// raw address. Kept separate from `recipientRaw43` so we don't
    /// fight the formatter on partial input.
    @State private var recipientHex: String = ""
    @State private var amountDash: String = "0.001"
    /// Caller-supplied shielded credits (what enters the Orchard
    /// pool). String-backed so the user can edit it directly; the
    /// view also auto-fills it from the L1 amount when blank.
    @State private var shieldAmountCreditsText: String = ""

    // MARK: - Submit state

    @State private var submitError: SubmitError? = nil
    @State private var activeController: ShieldedFundFromAssetLockController? = nil

    /// 1 DASH = 1e8 duffs (Core side).
    private static let duffsPerDash: UInt64 = 100_000_000
    /// 1 duff = 1e3 credits (Platform side). Same scale every
    /// other duff→credits conversion in this app uses.
    private static let creditsPerDuff: UInt64 = 1_000
    /// 1 DASH ≈ 100,000 duffs ≈ 100,000,000 credits. The asset-lock
    /// floor mirrors `FundFromAssetLockPlatformAddressView` (1mDASH).
    private static let minDuffs: UInt64 = 100_000

    var body: some View {
        NavigationStack {
            Form {
                if let controller = activeController {
                    ShieldedFundFromAssetLockProgressSection(controller: controller)
                    progressTerminalSection(controller: controller)
                } else if resumeFromLock != nil {
                    walletSection
                    resumeFromAssetLockSection
                    recipientSection
                    shieldAmountSection
                    if canSubmit {
                        submitSection
                    }
                } else {
                    walletSection
                    coreFundingSection
                    recipientSection
                    amountSection
                    shieldAmountSection
                    if canSubmit {
                        submitSection
                    }
                }
            }
            .navigationTitle("Shield from Asset Lock")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") { dismiss() }
                        .disabled(activeController?.phase == .inFlight)
                }
            }
            .alert(item: $submitError) { err in
                Alert(
                    title: Text("Could not shield"),
                    message: Text(err.message),
                    dismissButton: .default(Text("OK"))
                )
            }
            .onAppear(perform: autoSelectDefaults)
            .onChange(of: amountDash) { _, _ in autoFillShieldAmount() }
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
                Text("No spendable Core (BIP44 standard) accounts on this wallet.")
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
            Text("Core Source")
        } footer: {
            Text(
                "The selected Core account's UTXOs are locked into an asset lock; "
                    + "the locked DASH becomes shielded credits on the destination "
                    + "Orchard address."
            )
        }
    }

    @ViewBuilder
    private var recipientSection: some View {
        Section {
            HStack {
                Label("Recipient", systemImage: "lock.shield")
                Spacer()
                if let r = recipientRaw43 {
                    Text(hexShort(r))
                        .font(.system(.body, design: .monospaced))
                        .foregroundColor(.secondary)
                        .lineLimit(1)
                        .truncationMode(.middle)
                } else {
                    Text("None")
                        .foregroundColor(.secondary)
                }
            }
            TextField(
                "Recipient (86-char hex, leave blank for self)",
                text: $recipientHex
            )
            .font(.system(.caption, design: .monospaced))
            .textInputAutocapitalization(.never)
            .autocorrectionDisabled(true)
            .onChange(of: recipientHex) { _, newValue in
                applyRecipientOverride(newValue)
            }
        } header: {
            Text("Destination Orchard Address")
        } footer: {
            Text(
                "Defaults to this wallet's own shielded default address (\"shield "
                    + "to self\"). Paste an 86-character raw hex Orchard address to send "
                    + "to a different recipient. Bech32m parsing isn't wired into the "
                    + "example app yet — use the raw shape that "
                    + "`platform_wallet_manager_shielded_default_address` returns."
            )
        }
    }

    @ViewBuilder
    private var amountSection: some View {
        Section {
            HStack {
                TextField("Amount", text: $amountDash)
                    .keyboardType(.decimalPad)
                    .textFieldStyle(.roundedBorder)
                    .disabled(activeController != nil)
                Text("DASH")
                    .foregroundColor(.secondary)
            }
        } header: {
            Text("L1 Asset-Lock Amount")
        } footer: {
            if let amount = parsedDuffs {
                Text(
                    "\(formatDuffs(amount)) duffs will be locked. Minimum: "
                        + "\(formatDuffs(Self.minDuffs)). Must cover shield amount + "
                        + "Platform min fee."
                )
            } else {
                Text("Minimum: \(formatDuffs(Self.minDuffs)) duffs.")
            }
        }
    }

    @ViewBuilder
    private var shieldAmountSection: some View {
        Section {
            HStack {
                TextField("Shield amount", text: $shieldAmountCreditsText)
                    .keyboardType(.numberPad)
                    .textFieldStyle(.roundedBorder)
                    .disabled(activeController != nil)
                Text("credits")
                    .foregroundColor(.secondary)
            }
        } header: {
            Text("Shielded Credits (Orchard value_balance)")
        } footer: {
            Text(
                "Credits that enter the shielded pool. Auto-filled from the L1 amount "
                    + "minus a conservative fee; override to claim less than the full lock. "
                    + "The wallet refuses obviously-undersized configurations before "
                    + "broadcasting the asset lock or building the ~30s Halo 2 proof."
            )
        }
    }

    private var submitSection: some View {
        Section {
            Button {
                submit()
            } label: {
                HStack {
                    Text(resumeFromLock == nil ? "Shield" : "Resume Shield")
                    Spacer()
                }
                .foregroundColor(.white)
            }
            .frame(maxWidth: .infinity)
            .listRowBackground(Color.accentColor)
        }
    }

    @ViewBuilder
    private var resumeFromAssetLockSection: some View {
        if let lock = resumeFromLock {
            Section {
                HStack {
                    Label("Asset Lock", systemImage: "lock.fill")
                    Spacer()
                    Text(lock.shortOutPointDisplay)
                        .font(.system(.body, design: .monospaced))
                        .foregroundColor(.secondary)
                }
                HStack {
                    Label("Amount Locked", systemImage: "dollarsign.circle")
                    Spacer()
                    Text(formatDuffs(UInt64(bitPattern: Int64(lock.amountDuffs))))
                        .foregroundColor(.secondary)
                }
                HStack {
                    Label("Status", systemImage: "info.circle")
                    Spacer()
                    Text(lock.statusLabel)
                        .foregroundColor(.secondary)
                }
            } header: {
                Text("Resuming")
            } footer: {
                Text(
                    "The asset lock was already built and reached a usable proof state. "
                        + "Pick a recipient + shield amount to complete the funding."
                )
            }
        }
    }

    /// Inline terminal section that follows the controller's phase.
    /// Same idea as the address-funding sibling's `progressTerminalSection`.
    @ViewBuilder
    private func progressTerminalSection(
        controller: ShieldedFundFromAssetLockController
    ) -> some View {
        switch controller.phase {
        case .completed:
            Section {
                VStack(alignment: .leading, spacing: 8) {
                    Label("Shielded", systemImage: "checkmark.seal.fill")
                        .foregroundColor(.green)
                        .font(.headline)
                    HStack {
                        Text("Amount shielded")
                            .foregroundColor(.secondary)
                        Spacer()
                        Text(formatCredits(controller.shieldAmountCredits))
                            .font(.system(.body, design: .monospaced))
                    }
                    Text(
                        "The shielded note appears in your balance after the next "
                            + "sync pass."
                    )
                    .font(.caption)
                    .foregroundColor(.secondary)
                    Button {
                        walletManager.shieldedFundFromAssetLockCoordinator.dismiss(
                            walletId: controller.walletId,
                            recipientRaw43: controller.recipientRaw43
                        )
                        dismiss()
                    } label: {
                        Text("Done")
                            .frame(maxWidth: .infinity)
                    }
                    .buttonStyle(.borderedProminent)
                    .padding(.top, 4)
                }
            }
        case .failed(let message):
            Section {
                VStack(alignment: .leading, spacing: 8) {
                    Label("Shield failed", systemImage: "xmark.octagon.fill")
                        .foregroundColor(.red)
                        .font(.headline)
                    Text(message)
                        .font(.callout)
                        .foregroundColor(.primary)
                        .textSelection(.enabled)
                    Button {
                        walletManager.shieldedFundFromAssetLockCoordinator.dismiss(
                            walletId: controller.walletId,
                            recipientRaw43: controller.recipientRaw43
                        )
                        dismiss()
                    } label: {
                        Text("Dismiss")
                            .frame(maxWidth: .infinity)
                    }
                    .buttonStyle(.bordered)
                    .padding(.top, 4)
                }
            }
        default:
            EmptyView()
        }
    }

    // MARK: - Submit

    private func submit() {
        guard let recipient = recipientRaw43 else { return }
        guard let shieldAmount = parsedShieldAmount, shieldAmount > 0 else { return }

        let walletId = wallet.walletId
        let manager = walletManager

        let body: () async throws -> Void
        if let lock = resumeFromLock {
            guard let parsed = parseOutPoint(lock.outPointHex) else {
                submitError = SubmitError(
                    message: "Could not parse asset lock outpoint: \(lock.outPointHex)"
                )
                return
            }
            body = {
                try await manager.shieldedResumeFundFromAssetLock(
                    walletId: walletId,
                    outPointTxid: parsed.txid,
                    outPointVout: parsed.vout,
                    recipients: [
                        ShieldedFundFromAssetLockRecipient(
                            recipientRaw43: recipient,
                            credits: shieldAmount
                        )
                    ]
                )
            }
        } else {
            guard
                let fundingAccountIndex = fundingCoreAccountIndex,
                let duffs = parsedDuffs
            else { return }
            body = {
                try await manager.shieldedFundFromAssetLock(
                    walletId: walletId,
                    fundingAccountIndex: fundingAccountIndex,
                    amountDuffs: duffs,
                    recipients: [
                        ShieldedFundFromAssetLockRecipient(
                            recipientRaw43: recipient,
                            credits: shieldAmount
                        )
                    ]
                )
            }
        }

        // Single-flight gate via the coordinator. Two levels:
        //   - Same recipient + in-flight: returns the existing
        //     controller (the user sees the same progress view).
        //   - Different recipient but another shielded funding in
        //     flight on this wallet: surfaces a typed "wait"
        //     error pointing at the in-flight recipient. Mirrors
        //     the Rust-side `shield_guard` mutex that serializes
        //     all shield-class ops per wallet.
        let coordinator = walletManager.shieldedFundFromAssetLockCoordinator
        switch coordinator.startFunding(
            walletId: walletId,
            recipientRaw43: recipient,
            shieldAmountCredits: shieldAmount,
            body: body
        ) {
        case .started(let controller):
            activeController = controller
        case .blockedByOtherWalletFunding(let blocker):
            submitError = SubmitError(
                message: "Another shielded funding is already in progress on this wallet "
                    + "(recipient \(hexShort(blocker.recipientRaw43))). Shield-class operations "
                    + "are serialised wallet-wide by the Rust runtime — try again after that "
                    + "one finishes."
            )
        }
    }

    /// Parse `<txid display hex>:<vout>` back into (32-byte raw
    /// little-endian txid, vout). Inverse of
    /// `PersistentAssetLock.encodeOutPoint(rawBytes:)`'s display
    /// formatting. Returns `nil` on any malformed input.
    private func parseOutPoint(_ hex: String) -> (txid: Data, vout: UInt32)? {
        let parts = hex.split(separator: ":", maxSplits: 1, omittingEmptySubsequences: false)
        guard parts.count == 2 else { return nil }
        let txidDisplay = String(parts[0])
        guard let vout = UInt32(parts[1]) else { return nil }
        guard txidDisplay.count == 64 else { return nil }
        var bytes = [UInt8]()
        bytes.reserveCapacity(32)
        var idx = txidDisplay.startIndex
        while idx < txidDisplay.endIndex {
            let next = txidDisplay.index(idx, offsetBy: 2)
            guard let b = UInt8(txidDisplay[idx..<next], radix: 16) else { return nil }
            bytes.append(b)
            idx = next
        }
        let txid = Data(bytes.reversed())
        return (txid, vout)
    }

    // MARK: - Derived

    private struct CoreAccountOption {
        let accountIndex: UInt32
        let balanceDuffs: UInt64
    }

    private var coreAccountOptions: [CoreAccountOption] {
        walletManager.accountBalances(for: wallet.walletId)
            .filter { $0.typeTag == 0 && $0.standardTag == 0 && $0.confirmed > 0 }
            .sorted { $0.index < $1.index }
            .map {
                CoreAccountOption(
                    accountIndex: $0.index,
                    balanceDuffs: $0.confirmed
                )
            }
    }

    private var selectedCoreAccountBalanceDuffs: UInt64 {
        guard let idx = fundingCoreAccountIndex else { return 0 }
        return coreAccountOptions.first(where: { $0.accountIndex == idx })?.balanceDuffs ?? 0
    }

    private var parsedDuffs: UInt64? {
        let raw = amountDash.trimmingCharacters(in: .whitespacesAndNewlines)
        guard let dash = Double(raw), dash > 0 else { return nil }
        let duffsDouble = dash * Double(Self.duffsPerDash)
        guard duffsDouble.isFinite, duffsDouble <= Double(UInt64.max) else { return nil }
        return UInt64(duffsDouble.rounded(.toNearestOrAwayFromZero))
    }

    private var parsedShieldAmount: UInt64? {
        UInt64(shieldAmountCreditsText.trimmingCharacters(in: .whitespacesAndNewlines))
    }

    private var canSubmit: Bool {
        if resumeFromLock != nil {
            return recipientRaw43 != nil
                && (parsedShieldAmount ?? 0) > 0
                && activeController == nil
        }
        let amount = parsedDuffs ?? 0
        return fundingCoreAccountIndex != nil
            && recipientRaw43 != nil
            && (parsedShieldAmount ?? 0) > 0
            && amount >= Self.minDuffs
            && selectedCoreAccountBalanceDuffs >= amount
            && activeController == nil
    }

    // MARK: - Actions

    private func autoSelectDefaults() {
        if fundingCoreAccountIndex == nil {
            fundingCoreAccountIndex = coreAccountOptions
                .first { $0.balanceDuffs > 0 }?.accountIndex
                ?? coreAccountOptions.first?.accountIndex
        }
        if recipientRaw43 == nil {
            // Default to the wallet's own bound shielded address —
            // the natural demo case ("shield to self"). The lookup
            // can fail if the wallet hasn't been bound yet; in that
            // case the user has to paste an external recipient.
            if let own = try? walletManager.shieldedDefaultAddress(walletId: wallet.walletId) {
                recipientRaw43 = own
            }
        }
        autoFillShieldAmount()
    }

    /// Apply a user-typed hex override to the recipient. Accepts
    /// empty (revert to wallet's own default) and 86-char hex
    /// (= 43 raw bytes). Anything in between is treated as
    /// in-progress entry: leave `recipientRaw43` unchanged so the
    /// submit button doesn't oscillate.
    private func applyRecipientOverride(_ raw: String) {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty {
            // Revert to the wallet's own default.
            recipientRaw43 = (try? walletManager.shieldedDefaultAddress(walletId: wallet.walletId))
                ?? nil
            return
        }
        guard trimmed.count == 86 else { return }
        var bytes = [UInt8]()
        bytes.reserveCapacity(43)
        var idx = trimmed.startIndex
        while idx < trimmed.endIndex {
            let next = trimmed.index(idx, offsetBy: 2)
            guard let b = UInt8(trimmed[idx..<next], radix: 16) else { return }
            bytes.append(b)
            idx = next
        }
        recipientRaw43 = Data(bytes)
    }

    /// Auto-fill the shield-amount field from the L1 amount when
    /// the user hasn't touched it manually. Conservative estimate:
    /// L1 credits minus a small fee buffer. Final precision is
    /// caller-controlled — the field stays editable.
    private func autoFillShieldAmount() {
        // Skip auto-fill if the user has typed a custom value the
        // L1 amount doesn't naturally produce.
        if let existing = parsedShieldAmount, existing > 0,
           let autoComputed = autoComputedShieldCredits(),
           existing != autoComputed,
           !shieldAmountCreditsText.isEmpty
        {
            // User has manually edited — leave alone.
            return
        }
        if let auto = autoComputedShieldCredits() {
            shieldAmountCreditsText = String(auto)
        }
    }

    /// Default shield amount = L1 credits − minFee (a safe lower
    /// bound). Returns `nil` when the L1 amount isn't valid yet.
    private func autoComputedShieldCredits() -> UInt64? {
        guard let duffs = parsedDuffs else { return nil }
        let lockCredits = duffs.multipliedReportingOverflow(by: Self.creditsPerDuff)
        guard !lockCredits.overflow else { return nil }
        // Mirrors `required_asset_lock_duff_balance_for_processing_start_for_address_funding`
        // (Type 14 / Type 18 share this constant in the platform
        // version). 1 million duffs = 1e9 credits, a comfortable
        // conservative buffer for the example app — Rust-side
        // preflight catches the precise value if we under-shoot.
        let minFeeCredits: UInt64 = 1_000_000_000
        guard lockCredits.partialValue > minFeeCredits else { return nil }
        return lockCredits.partialValue - minFeeCredits
    }

    // MARK: - Formatting

    private func formatDuffs(_ duffs: UInt64) -> String {
        let dash = Double(duffs) / Double(Self.duffsPerDash)
        return String(format: "%.8f DASH", dash)
    }

    private func formatCredits(_ credits: UInt64) -> String {
        let dash = Double(credits) / 100_000_000_000.0
        return String(format: "%.6f DASH (credits)", dash)
    }

    private func hexShort(_ data: Data) -> String {
        let hex = data.map { String(format: "%02x", $0) }.joined()
        if hex.count <= 16 { return hex }
        let prefix = hex.prefix(8)
        let suffix = hex.suffix(8)
        return "\(prefix)…\(suffix)"
    }

    // MARK: - SubmitError

    private struct SubmitError: Identifiable {
        let id = UUID()
        let message: String
    }
}
