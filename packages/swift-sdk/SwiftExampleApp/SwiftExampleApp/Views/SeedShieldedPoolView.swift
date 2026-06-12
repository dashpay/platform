// SeedShieldedPoolView.swift
// SwiftExampleApp
//
// Devnet/testnet-only utility: seed the shielded pool's anonymity set so
// outgoing shielded transitions (transfer / unshield / withdrawal /
// identity-create-from-pool) clear the consensus 250-note minimum.
//
// After a devnet reset WITHOUT the `DRIVE_SHIELDED_SNAPSHOT` genesis
// ingest, the pool starts empty and every outgoing shielded transition is
// rejected with "pool has N notes but minimum 250 required". This sheet
// drives `PlatformWalletManager.seedShieldedPoolNotes(...)`, which submits
// a series of `ShieldFromAssetLock` (Type 18) batches — each adding up to
// 6 notes (1 real note to the wallet's own default shielded address + up
// to 5 zero-value anonymity-set fillers) — until the on-chain pool note
// count reaches the target. 6 is the most actions that fit the 20 KiB
// transaction-size limit (the Halo 2 proof grows ~2.7 KB per action); see
// MAX_ACTIONS_PER_BATCH in rs-platform-wallet's seed_pool.rs.
//
// Batches run serially and each waits for proven execution, so a 250-note
// seed is ~42 batches and can take an hour or more; the live progress
// counter (driven by the FFI progress callback) keeps the UI honest.
//
// The Rust side hard-errors on mainnet (`Network.mainnet`); this sheet is
// only surfaced for non-mainnet wallets, but the guard is defence in depth.

import SwiftUI
import SwiftDashSDK
import SwiftData

struct SeedShieldedPoolView: View {
    @Environment(\.dismiss) private var dismiss
    @EnvironmentObject var walletManager: PlatformWalletManager

    /// Wallet that funds the seeding and owns the real notes.
    let wallet: PersistentWallet

    /// 1 DASH = 1e8 duffs (Core side) — used only to display the picked
    /// account's balance.
    private static let duffsPerDash: UInt64 = 100_000_000

    // MARK: - Selection state

    @State private var fundingCoreAccountIndex: UInt32? = nil
    @State private var targetNotesText: String = "250"

    // MARK: - Run state

    private enum Phase: Equatable {
        case idle
        case inFlight
        case completed
        case failed(String)
    }
    @State private var phase: Phase = .idle
    @State private var progress: SeedShieldedPoolProgress? = nil

    var body: some View {
        NavigationStack {
            Form {
                switch phase {
                case .idle:
                    walletSection
                    fundingSection
                    targetSection
                    if canSubmit {
                        submitSection
                    }
                case .inFlight:
                    progressSection
                case .completed:
                    completedSection
                case .failed(let message):
                    failedSection(message)
                }
            }
            .navigationTitle("Seed Pool Notes")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") { dismiss() }
                        .disabled(phase == .inFlight)
                }
            }
            .onAppear(perform: autoSelectDefaults)
            // The seeding Task keeps running if the sheet goes away; with
            // no progress/failure surface the user could also start a
            // second concurrent run. Block swipe-to-dismiss while a run
            // is in flight (the toolbar Cancel is disabled for the same
            // reason).
            .interactiveDismissDisabled(phase == .inFlight)
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
            Text("Funding Wallet")
        } footer: {
            Text(
                "Seeding burns asset-lock value (one L1 lock + the shielded fee "
                    + "per batch) purely to grow the on-chain note count. This is a "
                    + "devnet/testnet utility — the mainnet pool is seeded at genesis."
            )
        }
    }

    @ViewBuilder
    private var fundingSection: some View {
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
                "Each batch builds one asset lock from this account's UTXOs. Make "
                    + "sure it holds enough DASH to cover roughly one lock per 6 notes."
            )
        }
    }

    private var targetSection: some View {
        Section {
            HStack {
                TextField("Target notes", text: $targetNotesText)
                    .keyboardType(.numberPad)
                    .textFieldStyle(.roundedBorder)
                Text("notes")
                    .foregroundColor(.secondary)
            }
        } header: {
            Text("Target Pool Size")
        } footer: {
            if let target = parsedTarget {
                // Mirrors MAX_ACTIONS_PER_BATCH (6) in seed_pool.rs — the
                // most actions that fit the 20 KiB transition-size limit.
                // Ceiling division without `target + 5`, which would trap
                // on a pasted UInt64.max target.
                let batches = target / 6 + (target % 6 == 0 ? 0 : 1)
                Text(
                    "Drive the pool up to \(target) notes — about \(batches) "
                        + "ShieldFromAssetLock batches. Already-present notes count toward "
                        + "the target."
                )
            } else {
                Text("Consensus minimum for outgoing shielded transitions is 250.")
            }
        }
    }

    private var submitSection: some View {
        Section {
            Button {
                submit()
            } label: {
                HStack {
                    Text("Seed Pool")
                    Spacer()
                }
                .foregroundColor(.white)
            }
            .frame(maxWidth: .infinity)
            .listRowBackground(Color.accentColor)
        }
    }

    private var progressSection: some View {
        Section {
            VStack(alignment: .leading, spacing: 12) {
                HStack(spacing: 12) {
                    ProgressView()
                    Text("Seeding pool…")
                        .foregroundColor(.secondary)
                }
                if let p = progress {
                    ProgressView(value: progressFraction(p))
                    // `batchIndex` counts COMPLETED batches (the Rust side
                    // emits it before and after each batch), so present it
                    // as a completed count, not a 1-based "current batch".
                    Text(
                        "\(p.batchIndex)/~\(p.batchesTotalEstimate) batches completed · "
                            + "\(p.poolNotesNow)/\(p.target) notes"
                    )
                    .font(.caption)
                    .foregroundColor(.secondary)
                } else {
                    Text("Building proof for the first batch (~30s)…")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                Text(
                    "Each batch waits for proven execution before the next — this can "
                        + "take tens of minutes. Keep the app foregrounded."
                )
                .font(.caption2)
                .foregroundColor(.secondary)
            }
        }
    }

    private var completedSection: some View {
        Section {
            VStack(alignment: .leading, spacing: 8) {
                Label("Pool seeded", systemImage: "checkmark.seal.fill")
                    .foregroundColor(.green)
                    .font(.headline)
                if let p = progress {
                    Text("Pool now has \(p.poolNotesNow) notes (target \(p.target)).")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                Text("Outgoing shielded transitions should now clear the 250-note minimum.")
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
    }

    private func failedSection(_ message: String) -> some View {
        Section {
            VStack(alignment: .leading, spacing: 8) {
                Label("Seeding failed", systemImage: "xmark.octagon.fill")
                    .foregroundColor(.red)
                    .font(.headline)
                if let p = progress {
                    Text("Reached \(p.poolNotesNow)/\(p.target) notes before failing.")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
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
    }

    // MARK: - Submit

    private func submit() {
        guard
            let fundingAccountIndex = fundingCoreAccountIndex,
            let target = parsedTarget
        else { return }

        let manager = walletManager
        let walletId = wallet.walletId
        phase = .inFlight
        progress = nil

        Task {
            do {
                try await manager.seedShieldedPoolNotes(
                    walletId: walletId,
                    account: 0,
                    targetTotalNotes: target,
                    fundingAccountIndex: fundingAccountIndex,
                    progress: { p in
                        // The FFI callback fires on a background worker
                        // thread; hop to the main actor before touching
                        // SwiftUI state.
                        Task { @MainActor in
                            self.progress = p
                        }
                    }
                )
                await MainActor.run { phase = .completed }
            } catch {
                await MainActor.run {
                    phase = .failed(error.localizedDescription)
                }
            }
        }
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
                CoreAccountOption(accountIndex: $0.index, balanceDuffs: $0.confirmed)
            }
    }

    private var parsedTarget: UInt64? {
        let raw = targetNotesText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard let value = UInt64(raw), value > 0 else { return nil }
        return value
    }

    private var canSubmit: Bool {
        fundingCoreAccountIndex != nil && parsedTarget != nil && phase == .idle
    }

    private func progressFraction(_ p: SeedShieldedPoolProgress) -> Double {
        guard p.target > 0 else { return 0 }
        return min(1.0, Double(p.poolNotesNow) / Double(p.target))
    }

    // MARK: - Actions

    private func autoSelectDefaults() {
        if fundingCoreAccountIndex == nil {
            fundingCoreAccountIndex = coreAccountOptions
                .first { $0.balanceDuffs > 0 }?.accountIndex
                ?? coreAccountOptions.first?.accountIndex
        }
    }

    // MARK: - Formatting

    private func formatDuffs(_ duffs: UInt64) -> String {
        let dash = Double(duffs) / Double(Self.duffsPerDash)
        return String(format: "%.8f DASH", dash)
    }

    private func hexShort(_ data: Data) -> String {
        let hex = data.map { String(format: "%02x", $0) }.joined()
        if hex.count <= 16 { return hex }
        let prefix = hex.prefix(8)
        let suffix = hex.suffix(8)
        return "\(prefix)…\(suffix)"
    }
}
