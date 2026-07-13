import SwiftUI
import SwiftDashSDK
import SwiftData
import UIKit

/// Read-only detail page for one aggregated masternode. Pushed from the
/// Identities → Masternodes list via value-based navigation.
///
/// All fields are pre-aggregated in Rust and mirrored on
/// `PersistentMasternode`; this view only renders them. The key-ownership,
/// Core-payout, and (evonode) claimable-balance panels are layered on top
/// of the same Rust-sourced data.
struct MasternodeDetailView: View {
    let masternode: PersistentMasternode
    /// Platform SDK access for the evonode claimable-balance fetch.
    @EnvironmentObject var platformState: AppState

    /// Core payout coinbase TXOs paid to this node's payout address, if
    /// that address belongs to this wallet. Sourced from persisted
    /// `PersistentTxo` (not a Rust in-memory scan): coinbase payout txs
    /// aren't provider txs, so they're evicted from the wallet's in-memory
    /// set — the durable record is the TXO the payout created here.
    @Query private var payoutTxos: [PersistentTxo]

    /// Evonode claimable balance = the masternode identity's credit balance
    /// (identity id == proTxHash, no hashing — see dash-evo-tool). `nil`
    /// until fetched / when the identity isn't found.
    @State private var claimableCredits: UInt64?
    @State private var balanceLoading = false
    @State private var balanceError: String?

    init(masternode: PersistentMasternode) {
        self.masternode = masternode
        // Every TXO paid to the (dedicated) masternode payout address in
        // this wallet is a payout. We deliberately do NOT filter on
        // `isCoinbase` — masternode payouts arrive via coinbase, but the
        // SPV persister doesn't reliably flag it, so requiring it would
        // hide real payouts. When `payoutAddress` is nil the empty-string
        // key matches nothing (correct: no payouts to show).
        let addr = masternode.payoutAddress ?? ""
        let wid = masternode.walletId
        _payoutTxos = Query(
            filter: #Predicate<PersistentTxo> { txo in
                txo.address == addr && txo.walletId == wid
            },
            sort: [SortDescriptor(\PersistentTxo.height, order: .reverse)]
        )
    }

    private var statusColor: Color {
        switch masternode.status {
        case .active: return .green
        case .inactive: return .orange
        case .retired: return .red
        case .unknown: return .secondary
        }
    }

    var body: some View {
        List {
            Section {
                HStack {
                    Label(masternode.typeName, systemImage: "server.rack")
                        .font(.headline)
                    Spacer()
                    Text(masternode.statusName)
                        .font(.subheadline)
                        .fontWeight(.medium)
                        .foregroundColor(statusColor)
                }
                MasternodeDetailRow(label: "Service", value: masternode.serviceAddress ?? "—")
            }

            Section("Identity") {
                MasternodeCopyRow(label: "proTxHash", value: masternode.proTxHashHex)
                MasternodeDetailRow(
                    label: "Registration",
                    value: masternode.hasRegistration
                        ? "Height \(masternode.registrationHeight)"
                        : "Not in wallet history"
                )
                MasternodeDetailRow(label: "Provider TXs", value: "\(masternode.txCount)")
            }

            Section("Keys") {
                if let owner = masternode.ownerAddress {
                    MasternodeCopyRow(label: "Owner Address", value: owner)
                }
                if let ownerHash = masternode.ownerKeyHashHex {
                    MasternodeCopyRow(label: "Owner Key Hash", value: ownerHash)
                }
                if let voting = masternode.votingAddress {
                    MasternodeCopyRow(label: "Voting Address", value: voting)
                }
                if let votingHash = masternode.votingKeyHashHex {
                    MasternodeCopyRow(label: "Voting Key Hash", value: votingHash)
                }
                if let operatorKey = masternode.operatorPublicKeyHex {
                    MasternodeCopyRow(label: "Operator Key (BLS)", value: operatorKey)
                }
                if let nodeId = masternode.platformNodeIdHex {
                    MasternodeCopyRow(label: "Platform Node ID", value: nodeId)
                }
                if let payout = masternode.payoutAddress {
                    MasternodeCopyRow(label: "Payout Address", value: payout)
                }
            }

            Section("Key Ownership") {
                MasternodeDetailRow(
                    label: "Owner",
                    value: PersistentMasternode.keyOwnershipLabel(
                        inWallet: masternode.ownerInWallet,
                        accountType: masternode.ownerAccountType,
                        index: masternode.ownerKeyIndex
                    )
                )
                MasternodeDetailRow(
                    label: "Voting",
                    value: PersistentMasternode.keyOwnershipLabel(
                        inWallet: masternode.votingInWallet,
                        accountType: masternode.votingAccountType,
                        index: masternode.votingKeyIndex
                    )
                )
                if masternode.operatorPublicKey != nil {
                    MasternodeDetailRow(
                        label: "Operator",
                        value: PersistentMasternode.keyOwnershipLabel(
                            inWallet: masternode.operatorInWallet,
                            accountType: masternode.operatorAccountType,
                            index: masternode.operatorKeyIndex
                        )
                    )
                }
                if masternode.platformNodeId != nil {
                    MasternodeDetailRow(
                        label: "Platform Node",
                        value: PersistentMasternode.keyOwnershipLabel(
                            inWallet: masternode.platformInWallet,
                            accountType: masternode.platformAccountType,
                            index: masternode.platformKeyIndex
                        )
                    )
                }
            }

            if let collateral = masternode.collateralDisplay {
                Section("Collateral") {
                    MasternodeCopyRow(label: "Outpoint", value: collateral)
                }
            }

            Section("Core Payouts") {
                if payoutTxos.isEmpty {
                    Text("No payouts in this wallet's history")
                        .font(.caption)
                        .foregroundColor(.secondary)
                } else {
                    ForEach(payoutTxos) { txo in
                        MasternodeDetailRow(
                            label: "Height \(txo.height)",
                            value: Self.duffsAsDash(txo.amount)
                        )
                    }
                }
            }

            if masternode.revoked {
                Section("Revocation") {
                    MasternodeDetailRow(
                        label: "Reason",
                        value: "\(masternode.revocationReason)"
                    )
                }
            }

            // Platform credits accrue on the masternode's Platform identity
            // (evonodes only participate in Platform). Read-only display;
            // "claiming" (identity credit withdrawal) is a separate flow.
            if masternode.isEvonode {
                Section("Claimable Balance") {
                    if balanceLoading {
                        HStack(spacing: 8) {
                            ProgressView().scaleEffect(0.8)
                            Text("Fetching…").foregroundColor(.secondary)
                        }
                    } else if let credits = claimableCredits {
                        MasternodeDetailRow(
                            label: "Claimable",
                            value: "\(credits) credits"
                        )
                        MasternodeDetailRow(
                            label: "≈ DASH",
                            value: Self.creditsAsDash(credits)
                        )
                    } else if let err = balanceError {
                        Text(err)
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }

                    Button {
                        Task { await fetchClaimableBalance() }
                    } label: {
                        Label("Refresh", systemImage: "arrow.clockwise")
                    }
                    .disabled(balanceLoading)
                    .accessibilityIdentifier("masternode.refreshBalance")
                }
            }
        }
        .navigationTitle(masternode.displayTitle)
        .navigationBarTitleDisplayMode(.inline)
        .task {
            if masternode.isEvonode {
                await fetchClaimableBalance()
            }
        }
    }

    /// Fetch the masternode identity's credit balance. The identity id IS
    /// the proTxHash, but in **display (reversed) byte order** — the same
    /// orientation `proTxHashHex` decodes to and that dash-evo-tool feeds
    /// `Identifier::from_string` from the pasted block-explorer hex. Our
    /// stored `proTxHash` is raw wire order (`Txid::as_ref()`), so we
    /// reverse it before keying the balance fetch. The FFI call is
    /// blocking, so it runs off the main actor (matching `loadPreviewKeys`).
    @MainActor
    private func fetchClaimableBalance() async {
        guard let sdk = platformState.sdk else {
            balanceError = "Platform SDK not ready"
            return
        }
        // Identity id = display-order proTxHash (reverse of the stored wire
        // bytes).
        let identityId = Data(masternode.proTxHash.reversed())
        balanceLoading = true
        balanceError = nil
        do {
            let credits = try await Task.detached(priority: .userInitiated) {
                try sdk.identities.getBalance(id: identityId)
            }.value
            claimableCredits = credits
        } catch {
            claimableCredits = nil
            // Distinguish "no identity registered" from a transport failure
            // so the copy isn't misleading on a transient network error.
            let message = error.localizedDescription.lowercased()
            if message.contains("not found") || message.contains("no identity")
                || message.contains("does not exist")
            {
                balanceError = "This masternode has no Platform identity yet."
            } else {
                balanceError = "Couldn't fetch balance (network error). Try Refresh."
            }
        }
        balanceLoading = false
    }

    /// 1 DASH = 100,000,000,000 credits.
    private static func creditsAsDash(_ credits: UInt64) -> String {
        let dash = Double(credits) / 100_000_000_000.0
        return String(format: "%.8f DASH", dash)
    }

    /// 1 DASH = 100,000,000 duffs (Core amounts).
    private static func duffsAsDash(_ duffs: UInt64) -> String {
        let dash = Double(duffs) / 100_000_000.0
        return String(format: "%.8f DASH", dash)
    }
}

// MARK: - Rows

/// Label + value row, matching the app's detail-row convention.
struct MasternodeDetailRow: View {
    let label: String
    let value: String

    var body: some View {
        HStack {
            Text(label)
                .font(.subheadline)
                .foregroundColor(.secondary)
            Spacer()
            Text(value)
                .font(.subheadline)
                .fontWeight(.medium)
                .multilineTextAlignment(.trailing)
        }
    }
}

/// Caption + monospaced, tap-to-copy value block for hashes / addresses.
struct MasternodeCopyRow: View {
    let label: String
    let value: String
    @State private var copied = false

    var body: some View {
        Button {
            UIPasteboard.general.string = value
            withAnimation { copied = true }
            DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) {
                withAnimation { copied = false }
            }
        } label: {
            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Text(label)
                        .font(.caption)
                        .foregroundColor(.secondary)
                    Spacer()
                    Image(systemName: copied ? "checkmark" : "doc.on.doc")
                        .font(.caption)
                        .foregroundColor(copied ? .green : .blue)
                }
                Text(value)
                    .font(.system(.footnote, design: .monospaced))
                    .foregroundColor(.primary)
                    .lineLimit(nil)
                    .fixedSize(horizontal: false, vertical: true)
            }
        }
        .buttonStyle(.plain)
    }
}
