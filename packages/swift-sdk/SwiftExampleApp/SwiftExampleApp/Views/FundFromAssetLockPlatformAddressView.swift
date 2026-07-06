// FundFromAssetLockPlatformAddressView.swift
// SwiftExampleApp
//
// Stepped UI for funding a Platform payment address from a Core
// (SPV) wallet balance. Drives the new `ManagedPlatformAddressWallet
// .fundFromAssetLock(...)` end-to-end:
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

struct FundFromAssetLockPlatformAddressView: View {
    @Environment(\.dismiss) private var dismiss
    @Environment(\.modelContext) private var modelContext
    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var platformState: AppState

    /// Wallet to fund a platform address on. Drives both the picker
    /// scope (Core BIP44 accounts and Platform addresses on this
    /// wallet only) and the managed-wallet lookup at submit time.
    let wallet: PersistentWallet

    /// Optional asset lock to resume from. When non-nil the view
    /// hides the Core-funding-account + amount sections (the asset
    /// lock already exists, those choices were made at original
    /// build time) and routes Submit to
    /// `ManagedPlatformAddressWallet.resumeFundFromAssetLock` instead
    /// of building a fresh lock. The user still picks the recipient
    /// platform address because the orphan lock doesn't carry that
    /// information — it's set at ST-submission time.
    var resumeFromLock: PersistentAssetLock? = nil

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

    /// Pre-submit error (e.g. KeychainSigner / handle lookup failed
    /// synchronously before the FFI call). In-flight failures land
    /// on the controller's `.failed` phase and are rendered by
    /// `AddressFundFromAssetLockProgressView`'s terminal section instead.
    @State private var submitError: SubmitError? = nil

    /// Controller for the in-flight funding attempt. Non-nil swaps
    /// the form body for `AddressFundFromAssetLockProgressSection` + a
    /// terminal section that follows the controller's phase.
    /// Lifetime-owned by `walletManager.addressFundFromAssetLockCoordinator`
    /// so view dismissal mid-flight doesn't lose the work.
    @State private var activeController: AddressFundFromAssetLockController? = nil

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
                if let controller = activeController {
                    // Form sections inside a Form render as siblings,
                    // not nested; the progress section + terminal
                    // section follow the same shape as
                    // `RegistrationProgressView`.
                    AddressFundFromAssetLockProgressSection(controller: controller)
                    progressTerminalSection(controller: controller)
                } else if resumeFromLock != nil {
                    // Resume mode: the asset lock + amount + Core
                    // funding account were all decided at original
                    // build time. The user only re-picks the
                    // recipient since the orphan lock doesn't
                    // carry that — it's set at ST-submit time.
                    walletSection
                    resumeFromAssetLockSection
                    platformAccountSection
                    recipientSection
                    if canSubmit {
                        submitSection
                    }
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
            .navigationTitle("Top Up Platform Address")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") { dismiss() }
                        .disabled(activeController?.phase == .inFlight)
                }
            }
            .alert(item: $submitError) { err in
                Alert(
                    title: Text("Could not top up address"),
                    message: Text(err.message),
                    dismissButton: .default(Text("OK"))
                )
            }
            .onAppear(perform: autoSelectDefaults)
            .onChange(of: activeController?.phase) { _, newPhase in
                // On successful funding, stamp the recipient hash
                // onto the matching consumed asset-lock row so the
                // storage explorer can show which address received
                // the credits. The PersistentAssetLock row is
                // written by the persister callback in response to
                // Rust's changeset — Rust doesn't know the
                // recipient (it's chosen at ST-submit time), so we
                // back-fill on the Swift side after the FFI returns.
                if case .completed = newPhase {
                    backfillRecipientOnConsumedLock()
                }
            }
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
                    .disabled(activeController != nil)
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
                    Text(resumeFromLock == nil ? "Top Up" : "Resume Top Up")
                    Spacer()
                }
                .foregroundColor(.white)
            }
            .frame(maxWidth: .infinity)
            .listRowBackground(Color.accentColor)
        }
    }

    /// Read-only summary of the asset lock the user is resuming.
    /// Replaces both `coreFundingSection` (the lock already exists
    /// against a specific account) and `amountSection` (the locked
    /// amount is whatever the original build chose).
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
                // Status-aware: a proof-ready lock (InstantSendLocked /
                // ChainLocked) submits as soon as a recipient is picked; a
                // Broadcast lock still needs finality, so resuming it waits
                // for the ChainLock (guaranteed finality, however long it
                // takes) before crediting the address.
                if lock.canFundIdentity {
                    Text("The asset lock already reached a usable proof state. Pick a destination address to complete the funding.")
                } else {
                    Text("The asset lock is broadcast and still awaiting InstantSend / ChainLock finality. Pick a destination address; resuming will wait for finality, then credit the address.")
                }
            }
        }
    }

    /// Inline terminal section that follows the controller's
    /// `.completed` / `.failed` phase. Mirrors the
    /// `terminalSection` shape on `AddressFundFromAssetLockProgressView`,
    /// but embedded directly in this view's `Form` so the user
    /// gets the full result without a separate navigation push.
    @ViewBuilder
    private func progressTerminalSection(
        controller: AddressFundFromAssetLockController
    ) -> some View {
        switch controller.phase {
        case .completed(let newBalance):
            Section {
                VStack(alignment: .leading, spacing: 8) {
                    Label("Address funded", systemImage: "checkmark.seal.fill")
                        .foregroundColor(.green)
                        .font(.headline)
                    HStack {
                        Text("New balance")
                            .foregroundColor(.secondary)
                        Spacer()
                        Text(formatCredits(newBalance))
                            .font(.system(.body, design: .monospaced))
                    }
                    Button {
                        walletManager.addressFundFromAssetLockCoordinator.dismiss(
                            walletId: controller.walletId,
                            platformAccountIndex: controller.platformAccountIndex,
                            recipientHash: controller.recipientHash
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
                    Label("Top Up failed", systemImage: "xmark.octagon.fill")
                        .foregroundColor(.red)
                        .font(.headline)
                    Text(message)
                        .font(.callout)
                        .foregroundColor(.primary)
                        .textSelection(.enabled)
                    Button("Dismiss") {
                        walletManager.addressFundFromAssetLockCoordinator.dismiss(
                            walletId: controller.walletId,
                            platformAccountIndex: controller.platformAccountIndex,
                            recipientHash: controller.recipientHash
                        )
                        dismiss()
                    }
                }
            }
        default:
            EmptyView()
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
        // Surface Core BIP44 standard accounts with spendable
        // balance only. The compound filter
        // `typeTag == 0 && standardTag == 0` matches BIP44
        // (Standard, BIP44-tagged) — `standardTag` alone would
        // include PlatformPayment / CoinJoin / Identity* accounts
        // because those leave `standardTag` at its `0` default,
        // surfacing duplicate "Account #0" rows.
        //
        // Balance reads from the live FFI (`accountBalances(for:)`)
        // not `PersistentAccount.balanceConfirmed` — the SwiftData
        // field is populated by the persister callback and lags
        // the in-memory Rust state, so a freshly-synced wallet
        // would show zero here even with spendable Core funds.
        //
        // Zero-balance accounts are excluded so the picker can't
        // present a submit path that's guaranteed to fail at the
        // Rust-side UTXO selection stage.
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

    /// Confirmed balance of the currently-selected Core funding
    /// account, or `0` if no account is selected. Used by
    /// `canSubmit` to gate submission on `balance >= parsedDuffs`.
    private var selectedCoreAccountBalanceDuffs: UInt64 {
        guard let idx = fundingCoreAccountIndex else { return 0 }
        return coreAccountOptions.first(where: { $0.accountIndex == idx })?.balanceDuffs ?? 0
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
        if resumeFromLock != nil {
            // Resume only needs a recipient. The lock + amount +
            // funding account are fixed by the original build.
            return platformAccountIndex != nil
                && selectedRecipientHash != nil
                && activeController == nil
        }
        let amount = parsedDuffs ?? 0
        return fundingCoreAccountIndex != nil
            && platformAccountIndex != nil
            && selectedRecipientHash != nil
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
            let platformAcct = platformAccountIndex,
            let hash = selectedRecipientHash
        else { return }
        // Recipient resolution can race with SwiftData: between the
        // user tapping Fund Address and this body running, the
        // selected address may have flipped to `isUsed = true` (a
        // concurrent flow consumed it). Surface that as a fail-fast
        // error so the button isn't dead on tap.
        guard let recipient = recipientCandidates.first(where: { $0.addressHash == hash }) else {
            submitError = SubmitError(
                message: "The selected recipient address is no longer available (it may have been used by another funding). Pick a fresh address and try again."
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
        let walletId = wallet.walletId
        let recipientHash = recipient.addressHash
        let recipientType = recipient.addressType

        // FFI closure — captured into the coordinator so the same
        // controller-lifetime guarantees apply to both fresh and
        // resume flows. Returning the proof-attested credit
        // balance of the recipient so the terminal section can
        // surface a meaningful number.
        let body: () async throws -> UInt64
        if let lock = resumeFromLock {
            // Resume path: outpoint is decoded from the persisted
            // `outPointHex` (canonical `<txid display hex>:<vout>`
            // shape produced by `PersistentAssetLock.encodeOutPoint`).
            guard let parsed = parseOutPoint(lock.outPointHex) else {
                submitError = SubmitError(
                    message: "Could not parse asset lock outpoint: \(lock.outPointHex)"
                )
                return
            }
            body = {
                let updates = try await addressWallet.resumeFundFromAssetLock(
                    outPointTxid: parsed.txid,
                    outPointVout: parsed.vout,
                    platformAccountIndex: platformAcct,
                    recipients: [
                        ManagedPlatformAddressWallet.FundFromAssetLockRecipient(
                            addressType: recipientType,
                            hash: recipientHash,
                            credits: nil
                        )
                    ],
                    signer: signer
                )
                return updates
                    .first(where: { $0.hash == recipientHash })?.balance ?? 0
            }
        } else {
            // Fresh build path: needs the funding account + amount
            // gates that the resume path skips.
            guard
                let fundingAccountIndex = fundingCoreAccountIndex,
                let duffs = parsedDuffs
            else { return }
            body = {
                let updates = try await addressWallet.fundFromAssetLock(
                    amountDuffs: duffs,
                    fundingAccountIndex: fundingAccountIndex,
                    platformAccountIndex: platformAcct,
                    recipients: [
                        ManagedPlatformAddressWallet.FundFromAssetLockRecipient(
                            addressType: recipientType,
                            hash: recipientHash,
                            credits: nil
                        )
                    ],
                    signer: signer
                )
                return updates
                    .first(where: { $0.hash == recipientHash })?.balance ?? 0
            }
        }

        // Capture the set of currently-Consumed address-funding
        // outpoints on this wallet BEFORE the FFI fires. The
        // post-success back-fill uses the set-difference against
        // the new state to deterministically match this funding's
        // consumed lock — even when two concurrent fundings on the
        // same wallet land in close succession (the previous
        // newest-`updatedAt` heuristic mis-stamped in that race).
        let preSubmitOutpoints = capturePreSubmitConsumedOutpoints()

        // Single-flight gate via the coordinator. The same slot
        // re-presents the existing controller on a duplicate tap
        // so two FFI calls never race for the same asset lock.
        let coordinator = walletManager.addressFundFromAssetLockCoordinator
        let controller = coordinator.startFunding(
            walletId: walletId,
            platformAccountIndex: platformAcct,
            recipientHash: recipientHash,
            recipientType: recipientType,
            body: body
        )
        controller.preSubmitConsumedOutpoints = preSubmitOutpoints

        // Stash the controller; setting it flips the body to the
        // progress section in place of the form. The controller's
        // canonical lifetime owner is the coordinator — if the user
        // dismisses the sheet mid-flight, the same controller is
        // reachable via the "Pending Platform Top Ups" section on
        // the wallet detail screen.
        activeController = controller
    }

    /// Snapshot every outpoint currently marked Consumed for this
    /// wallet's address-funding asset locks. Used by the post-
    /// success back-fill to compute the "new since submission"
    /// delta. Pure read; no writes.
    private func capturePreSubmitConsumedOutpoints() -> Set<String> {
        let walletId = wallet.walletId
        let descriptor = FetchDescriptor<PersistentAssetLock>(
            predicate: #Predicate<PersistentAssetLock> { entry in
                entry.walletId == walletId
                    && entry.fundingTypeRaw == 4
                    && entry.statusRaw == 4
            }
        )
        let rows = (try? modelContext.fetch(descriptor)) ?? []
        return Set(rows.map { $0.outPointHex })
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
        // The display hex is reverse-of-wire order; flip to get the
        // raw 32-byte little-endian txid the FFI expects.
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

    // MARK: - Helpers

    /// Stamp the recipient hash + type onto the asset-lock row this
    /// funding's FFI call just Consumed. Called from `.onChange`
    /// after the controller flips to `.completed`.
    ///
    /// Match strategy: set-difference against the pre-submit
    /// Consumed-outpoint snapshot captured at `submit()` time. The
    /// new Consumed outpoint that wasn't in the snapshot is ours.
    ///
    /// Edge cases:
    /// - `preSubmitConsumedOutpoints` missing on controller: fall
    ///   back to a newest-unrecipiented heuristic.
    /// - Zero new outpoints: nothing to stamp; the funding
    ///   succeeded but no Consumed row appeared yet (persister
    ///   callback lag). The catch-up on the next funding will fix
    ///   it via the same delta logic.
    /// - Multiple new outpoints: two address-funding flows
    ///   completed Consumed in the same `.onChange` window.
    ///   Refuse to stamp either to avoid mis-attribution — better
    ///   to leave both showing "—" in the storage explorer than to
    ///   silently tag the wrong row.
    private func backfillRecipientOnConsumedLock() {
        guard let controller = activeController else { return }
        let walletId = wallet.walletId
        let recipientHash = controller.recipientHash
        let recipientType = controller.recipientType
        let preSubmitSet = controller.preSubmitConsumedOutpoints

        let descriptor = FetchDescriptor<PersistentAssetLock>(
            predicate: #Predicate<PersistentAssetLock> { entry in
                entry.walletId == walletId
                    && entry.fundingTypeRaw == 4
                    && entry.statusRaw == 4
                    && entry.recipientPlatformAddressHash == nil
            },
            sortBy: [SortDescriptor(\.updatedAt, order: .reverse)]
        )
        let matches: [PersistentAssetLock]
        do {
            matches = try modelContext.fetch(descriptor)
        } catch {
            // The funding succeeded; this fetch only feeds the
            // storage-explorer recipient column. Match the
            // surrounding app's `print`-with-emoji logging idiom
            // rather than introducing an OSLog dependency just for
            // this one call site.
            print("⚠️ backfillRecipient: fetch failed: \(error)")
            return
        }

        let target: PersistentAssetLock?
        if let preSubmit = preSubmitSet {
            // Deterministic snapshot-delta path. Filter the
            // unrecipiented Consumed rows down to those NOT in the
            // pre-submit set — those are the genuinely new rows.
            let newRows = matches.filter { !preSubmit.contains($0.outPointHex) }
            switch newRows.count {
            case 1:
                target = newRows.first
            case 0:
                // No new Consumed row visible yet (persister lag);
                // skip rather than stamp the wrong unrecipiented
                // row. The next funding's delta will pick this row
                // up via its own pre-submit snapshot.
                print("⚠️ backfillRecipient: no new Consumed outpoint since submission; skipping stamp")
                return
            default:
                // Multi-match: two address-funding flows resolved
                // Consumed in the same window. Refuse rather than
                // mis-attribute — better both rows show "—" in the
                // storage explorer than a wrong attribution.
                print("⚠️ backfillRecipient: \(newRows.count) new Consumed outpoints in delta; refusing to stamp ambiguous row")
                return
            }
        } else {
            // Snapshot wasn't captured (e.g. a future caller that
            // wires up the coordinator directly). Pick the
            // newest-unrecipiented row — has a race window when
            // multiple flows complete concurrently; see the doc
            // comment above.
            target = matches.first
        }

        guard let lock = target else { return }
        lock.recipientPlatformAddressHash = recipientHash
        lock.recipientPlatformAddressType = recipientType
        do {
            try modelContext.save()
        } catch {
            // SwiftData save failure is rare (typically only on
            // disk-full / store-corruption) but worth visible
            // surfacing. The funding itself succeeded so we don't
            // alert the user — just log.
            print("⚠️ backfillRecipient: save failed: \(error)")
        }
    }

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
