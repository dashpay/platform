// ShieldedFundFromAssetLockView.swift
// SwiftExampleApp
//
// Hosts TWO shield-into-pool entry points behind one sheet, chosen
// with an in-view "Source" segmented picker (fresh-build flow only):
//
//   * Core Balance  → the Type 18 asset-lock flow described below
//     (build lock → wait IS-lock → Halo 2 proof → ShieldFromAssetLock).
//   * Platform Balance → a Type 15 shield (`shieldedShield(...)`) that
//     spends Platform credits from a DIP-17 Platform Payment account
//     INTO this wallet's own shielded pool (always shield-to-self;
//     no recipient choice). Single awaited FFI call, no asset-lock
//     controller/coordinator — Halo 2 proof (~30s) runs inside.
//
// The resume-from-asset-lock path is Core-only and never shows the
// Source picker.
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
//   * Only one number is exposed in the UI ("Amount" in DASH = L1
//     lock size) — same shape as the address-funding sibling and the
//     identity-funding flow. The Rust wallet derives the shielded
//     credit amount internally (`lock_value − protocol_min_fee`)
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

    /// All persisted accounts. Filtered down to the wallet's DIP-17
    /// Platform Payment accounts inside `platformAccountOptions`
    /// (the Platform-source branch). Unused by the Core branch.
    @Query private var allAccounts: [PersistentAccount]

    /// All persisted platform addresses. Summed per Platform Payment
    /// account inside `platformAccountOptions` to show each account's
    /// spendable credit balance in the picker.
    @Query private var allPlatformAddresses: [PersistentPlatformAddress]

    // MARK: - Source selection

    /// Which balance the fresh-build flow shields from. Drives both
    /// the body branching and the live navigation title. The resume
    /// path ignores this (it's Core-only).
    private enum ShieldSource: String, CaseIterable, Identifiable {
        case coreBalance = "Core Balance"
        case platformBalance = "Platform Balance"
        var id: String { rawValue }
    }
    @State private var source: ShieldSource = .coreBalance

    // MARK: - Selection state

    @State private var fundingCoreAccountIndex: UInt32? = nil
    /// DIP-17 Platform Payment account index the Platform branch
    /// spends credits from. `paymentAccount` arg to `shieldedShield`.
    @State private var platformAccountIndex: UInt32? = nil
    /// 43-byte raw recipient Orchard address. Defaults to the
    /// wallet's bound shielded default in `autoSelectDefaults`.
    @State private var recipientRaw43: Data? = nil
    /// User-facing display hex for the recipient (86 chars). Bound
    /// to the override text field so the user can paste a different
    /// raw address. Kept separate from `recipientRaw43` so we don't
    /// fight the formatter on partial input.
    @State private var recipientHex: String = ""
    @State private var amountDash: String = "0.001"
    /// Platform-branch amount in DASH (parsed to credits, 1e11/DASH).
    /// Kept separate from `amountDash` so the Core (duffs) and
    /// Platform (credits) unit systems never share a text buffer.
    @State private var platformAmountDash: String = "0.001"

    // MARK: - Submit state

    @State private var submitError: SubmitError? = nil
    @State private var activeController: ShieldedFundFromAssetLockController? = nil

    /// Platform-branch submit lifecycle. The Platform path does NOT
    /// go through the asset-lock controller/coordinator — it's a
    /// single awaited `shieldedShield(...)` FFI call — so it tracks
    /// its own minimal phase here. Mirrors the Core controller's
    /// terminal-section shape visually only.
    private enum PlatformShieldPhase: Equatable {
        case idle
        case inFlight
        case completed
        case failed(String)
    }
    @State private var platformShieldPhase: PlatformShieldPhase = .idle

    /// 1 DASH = 1e8 duffs (Core side).
    private static let duffsPerDash: UInt64 = 100_000_000
    /// 1 DASH = 1e11 Platform credits. The Platform branch types an
    /// amount in DASH but `shieldedShield` takes credits, so the
    /// Platform path multiplies by this (vs. `duffsPerDash` on Core).
    private static let creditsPerDash: UInt64 = 100_000_000_000
    /// The asset-lock floor mirrors
    /// `FundFromAssetLockPlatformAddressView` (1mDASH).
    private static let minDuffs: UInt64 = 100_000

    var body: some View {
        NavigationStack {
            Form {
                if let controller = activeController {
                    // Core asset-lock flow in flight (Type 18).
                    ShieldedFundFromAssetLockProgressSection(controller: controller)
                    progressTerminalSection(controller: controller)
                } else if platformShieldPhase != .idle {
                    // Platform → pool flow in flight / terminal (Type
                    // 15). Single awaited call, so its progress lives
                    // entirely in `platformShieldPhase` — no controller.
                    platformShieldProgressSection
                } else if resumeFromLock != nil {
                    // Resume is Core-only and never shows the Source
                    // picker — the lock already exists.
                    walletSection
                    resumeFromAssetLockSection
                    recipientSection
                    if canSubmit {
                        submitSection
                    }
                } else {
                    // Fresh build: the user first picks a Source, then
                    // sees the matching funding/amount sections.
                    walletSection
                    sourcePickerSection
                    switch source {
                    case .coreBalance:
                        coreFundingSection
                        recipientSection
                        amountSection
                    case .platformBalance:
                        platformAccountSection
                        platformAmountSection
                        platformRecipientSection
                    }
                    if canSubmit {
                        submitSection
                    }
                }
            }
            .navigationTitle(navTitle)
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") { dismiss() }
                        .disabled(
                            activeController?.phase == .inFlight
                                || platformShieldPhase == .inFlight
                        )
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
        }
    }

    /// Live navigation title. Resume is Core-only ("Shield from Asset
    /// Lock"); the fresh-build flow tracks the Source picker so the
    /// title flips between Core / Platform as the user toggles it.
    private var navTitle: String {
        if resumeFromLock != nil { return "Shield from Asset Lock" }
        switch source {
        case .coreBalance: return "Shield from Core Balance"
        case .platformBalance: return "Shield from Platform Balance"
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

    /// Segmented Core/Platform source toggle (fresh-build flow only).
    /// Switching it re-renders the funding sections below and flips
    /// the navigation title via `navTitle`.
    private var sourcePickerSection: some View {
        Section {
            Picker("Source", selection: $source) {
                ForEach(ShieldSource.allCases) { src in
                    Text(src.rawValue).tag(src)
                }
            }
            .pickerStyle(.segmented)
        } footer: {
            Text(
                "Core Balance locks L1 DASH into an asset lock (Type 18). "
                    + "Platform Balance spends existing Platform credits straight "
                    + "into your shielded pool (Type 15)."
            )
        }
    }

    // MARK: - Platform-source sections

    /// Platform Payment account picker — the credit source for the
    /// Platform branch. Mirrors `FundFromAssetLockPlatformAddressView`'s
    /// `platformAccountSection`. The chosen index becomes the
    /// `paymentAccount` argument to `shieldedShield`.
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
            }
        } header: {
            Text("Platform Source")
        } footer: {
            Text(
                "Platform Payment account whose credits are spent into the "
                    + "shielded pool. Picker shows each account's current credit "
                    + "balance."
            )
        }
    }

    /// Amount field for the Platform branch. The number is in DASH
    /// for display ergonomics but converts to *credits* (1e11/DASH)
    /// — distinct from the Core branch's duffs (1e8/DASH). A
    /// separate field from `amountDash` keeps the two unit systems
    /// from leaking into each other.
    @ViewBuilder
    private var platformAmountSection: some View {
        Section {
            HStack {
                TextField("Amount", text: $platformAmountDash)
                    .keyboardType(.decimalPad)
                    .textFieldStyle(.roundedBorder)
                    .disabled(platformShieldPhase == .inFlight)
                Text("DASH")
                    .foregroundColor(.secondary)
            }
        } header: {
            Text("Amount")
        } footer: {
            if let credits = platformAmountCredits {
                Text(
                    "\(formatCredits(credits)) will be moved from the selected "
                        + "Platform account into your shielded pool."
                )
            } else {
                Text("Amount in DASH; converted to Platform credits (1e11/DASH).")
            }
        }
    }

    /// Read-only recipient row for the Platform branch. `shieldedShield`
    /// has no recipient parameter — it always shields into this
    /// wallet's own shielded pool (account 0). We surface the actual
    /// default address when it's resolvable, falling back to a static
    /// "your shielded address" line.
    @ViewBuilder
    private var platformRecipientSection: some View {
        Section {
            HStack {
                Label("Recipient", systemImage: "lock.shield")
                Spacer()
                if let own = try? walletManager.shieldedDefaultAddress(walletId: wallet.walletId) {
                    Text(hexShort(own))
                        .font(.system(.body, design: .monospaced))
                        .foregroundColor(.secondary)
                        .lineLimit(1)
                        .truncationMode(.middle)
                } else {
                    Text("your shielded address")
                        .foregroundColor(.secondary)
                }
            }
        } header: {
            Text("Destination")
        } footer: {
            Text(
                "Always shields to your own wallet's shielded pool (account 0). "
                    + "This path has no recipient choice."
            )
        }
    }

    /// Terminal/progress section for the Platform branch. Visual
    /// twin of `progressTerminalSection` but driven by
    /// `platformShieldPhase` (no controller). Dismiss on Done/Dismiss.
    @ViewBuilder
    private var platformShieldProgressSection: some View {
        switch platformShieldPhase {
        case .inFlight:
            Section {
                HStack(spacing: 12) {
                    ProgressView()
                    Text("Building proof (~30s)…")
                        .foregroundColor(.secondary)
                }
            }
        case .completed:
            Section {
                VStack(alignment: .leading, spacing: 8) {
                    Label("Shielded", systemImage: "checkmark.seal.fill")
                        .foregroundColor(.green)
                        .font(.headline)
                    Text(
                        "The shielded note appears in your balance after the next "
                            + "sync pass."
                    )
                    .font(.caption)
                    .foregroundColor(.secondary)
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
                        dismiss()
                    } label: {
                        Text("Dismiss")
                            .frame(maxWidth: .infinity)
                    }
                    .buttonStyle(.bordered)
                    .padding(.top, 4)
                }
            }
        case .idle:
            EmptyView()
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
            Text("Amount")
        } footer: {
            if let lockDuffs = parsedDuffs {
                Text(
                    "\(formatDuffs(lockDuffs)) will be locked on L1. "
                        + "The shielded pool receives lock value minus the Platform "
                        + "minimum fee. Minimum lock: \(formatDuffs(Self.minDuffs))."
                )
            } else {
                Text("Minimum: \(formatDuffs(Self.minDuffs)).")
            }
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
        // Platform branch: a single awaited `shieldedShield(...)` that
        // moves `credits` from the chosen Platform Payment account into
        // this wallet's own shielded pool (account 0). No recipient,
        // no asset-lock controller/coordinator — the Halo 2 proof runs
        // inside the call. Mirrors the `SendViewModel` self-shield path.
        if resumeFromLock == nil && source == .platformBalance {
            guard
                let paymentAccount = platformAccountIndex,
                let credits = platformAmountCredits
            else { return }
            let signer = KeychainSigner(modelContainer: modelContext.container)
            let manager = walletManager
            let walletId = wallet.walletId
            platformShieldPhase = .inFlight
            Task {
                do {
                    try await manager.shieldedShield(
                        walletId: walletId,
                        shieldedAccount: 0,
                        paymentAccount: paymentAccount,
                        amount: credits,
                        addressSigner: signer
                    )
                    await MainActor.run { platformShieldPhase = .completed }
                } catch {
                    await MainActor.run {
                        platformShieldPhase = .failed(error.localizedDescription)
                    }
                }
            }
            return
        }

        guard let recipient = recipientRaw43 else { return }

        let walletId = wallet.walletId
        let manager = walletManager

        // The operation id carries the resumed lock's outpoint: resumable
        // locks default to the same wallet-owned shielded recipient, so
        // the coordinator's slot key alone cannot tell two locks apart
        // and would silently reuse the first lock's controller.
        // A fresh shield mints a NEW id per submission: the coordinator
        // retains a completed controller for ~30s, and a fixed marker
        // would match it and rebind — reopening the old result instead
        // of running this submission's body. Minted here, at submission
        // time, so dismissing a completed sheet and shielding again is
        // always a new operation ("resume:<outpoint>" stays stable so a
        // re-tap of the SAME lock still rebinds).
        let operationId: String
        let body: () async throws -> Void
        if let lock = resumeFromLock {
            guard let parsed = parseOutPoint(lock.outPointHex) else {
                submitError = SubmitError(
                    message: "Could not parse asset lock outpoint: \(lock.outPointHex)"
                )
                return
            }
            operationId = "resume:\(lock.outPointHex)"
            body = {
                try await manager.shieldedResumeFundFromAssetLock(
                    walletId: walletId,
                    outPointTxid: parsed.txid,
                    outPointVout: parsed.vout,
                    recipients: [
                        ShieldedFundFromAssetLockRecipient(recipientRaw43: recipient)
                    ]
                )
            }
        } else {
            guard
                let fundingAccountIndex = fundingCoreAccountIndex,
                let duffs = parsedDuffs
            else { return }
            operationId = "shield:\(UUID().uuidString)"
            body = {
                try await manager.shieldedFundFromAssetLock(
                    walletId: walletId,
                    fundingAccountIndex: fundingAccountIndex,
                    amountDuffs: duffs,
                    recipients: [
                        ShieldedFundFromAssetLockRecipient(recipientRaw43: recipient)
                    ]
                )
            }
        }

        // Single-flight gate via the coordinator. Two levels:
        //   - Same operation (same recipient + operation id) +
        //     in-flight: returns the existing controller (the user
        //     sees the same progress view).
        //   - Any OTHER shielded funding in flight on this wallet —
        //     different recipient, or a different lock/fresh shield
        //     on the same recipient: surfaces a typed "wait" error
        //     pointing at the in-flight blocker. Mirrors the
        //     Rust-side `shield_guard` mutex that serializes all
        //     shield-class ops per wallet.
        let coordinator = walletManager.shieldedFundFromAssetLockCoordinator
        switch coordinator.startFunding(
            walletId: walletId,
            recipientRaw43: recipient,
            operationId: operationId,
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

    private struct PlatformAccountOption {
        let accountIndex: UInt32
        let totalCredits: UInt64
    }

    /// DIP-17 Platform Payment accounts on this wallet with their
    /// summed credit balance. Mirrors
    /// `FundFromAssetLockPlatformAddressView.platformAccountOptions`:
    /// `accountType == 14` is the PlatformPayment discriminant; per-
    /// account credits are summed from the wallet's platform addresses.
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

    /// Credit balance of the currently-selected Platform Payment
    /// account, or `0` if none is selected. Gates `canSubmit` on the
    /// Platform branch (`creditsWanted <= balance`).
    private var selectedPlatformAccountCredits: UInt64 {
        guard let idx = platformAccountIndex else { return 0 }
        return platformAccountOptions.first(where: { $0.accountIndex == idx })?.totalCredits ?? 0
    }

    /// Platform branch: the DASH the user typed converted to credits
    /// (1e11/DASH). Distinct from `parsedDuffs` (Core, 1e8/DASH).
    private var platformAmountCredits: UInt64? {
        let raw = platformAmountDash.trimmingCharacters(in: .whitespacesAndNewlines)
        guard let dash = Double(raw), dash > 0 else { return nil }
        let creditsDouble = dash * Double(Self.creditsPerDash)
        guard creditsDouble.isFinite, creditsDouble <= Double(UInt64.max) else { return nil }
        return UInt64(creditsDouble.rounded(.toNearestOrAwayFromZero))
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

    /// Fresh-build path: L1 duffs the user typed in the Amount field.
    /// Nil on resume (resume reads from the persisted lock).
    private var parsedDuffs: UInt64? {
        guard resumeFromLock == nil else { return nil }
        let raw = amountDash.trimmingCharacters(in: .whitespacesAndNewlines)
        guard let dash = Double(raw), dash > 0 else { return nil }
        let duffsDouble = dash * Double(Self.duffsPerDash)
        guard duffsDouble.isFinite, duffsDouble <= Double(UInt64.max) else { return nil }
        return UInt64(duffsDouble.rounded(.toNearestOrAwayFromZero))
    }

    private var canSubmit: Bool {
        if resumeFromLock != nil {
            return recipientRaw43 != nil && activeController == nil
        }
        // Platform branch: a chosen account, an idle phase (and no
        // Core controller running), a positive credit amount that
        // the account can cover. No min-floor — the Platform credits
        // already exist, unlike the L1 asset-lock floor.
        if source == .platformBalance {
            let credits = platformAmountCredits ?? 0
            return platformAccountIndex != nil
                && platformShieldPhase == .idle
                && activeController == nil
                && credits > 0
                && credits <= selectedPlatformAccountCredits
        }
        let amount = parsedDuffs ?? 0
        return fundingCoreAccountIndex != nil
            && recipientRaw43 != nil
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
        // Platform branch: default to the first Platform Payment
        // account (prefer one with a credit balance), mirroring the
        // Core auto-select. Cheap even when the Core source is active.
        if platformAccountIndex == nil {
            platformAccountIndex = platformAccountOptions
                .first { $0.totalCredits > 0 }?.accountIndex
                ?? platformAccountOptions.first?.accountIndex
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

    // MARK: - Formatting

    private func formatDuffs(_ duffs: UInt64) -> String {
        let dash = Double(duffs) / Double(Self.duffsPerDash)
        return String(format: "%.8f DASH", dash)
    }

    /// Credit divisor matches `FundFromAssetLockPlatformAddressView`
    /// / `CreateIdentityView` (1e11 credits/DASH).
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
