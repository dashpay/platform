// PrivateDocumentEntriesView.swift
// SwiftExampleApp
//
// Reads a private document type's store (protocol version 15) for one of
// the app's wallets and lists the entries its identities opened or sent.
// The read itself (which identities, which keys, how a payload reads) is
// Rust's: this screen calls `ManagedPlatformWallet.readPrivateDocumentEntries`
// once per tap and shows what came back. Nothing is persisted; the entries
// live in view state.
//
// The screen takes the contract id and type name as plain values, not a
// `PersistentDocumentType`: a contract-search preview presents its types
// from a throwaway in-memory store, while this screen reads the device's
// wallets and identities, so the caller hands it the device's model context
// and no model object of the preview store crosses into it.

import SwiftUI
import SwiftData
import SwiftDashSDK

struct PrivateDocumentEntriesView: View {
    let contractId: Data
    let documentTypeName: String

    @EnvironmentObject var appState: AppState
    @EnvironmentObject var walletManager: PlatformWalletManager
    /// Off `.active`, the screen is covered so the app-switcher snapshot
    /// shows no decrypted entry or identity.
    @Environment(\.scenePhase) private var scenePhase

    /// Every persisted wallet; filtered to the active network by `wallets`.
    @Query(sort: \PersistentWallet.createdAt) private var allWallets: [PersistentWallet]
    /// For naming the wallet's identities by alias or DPNS name.
    @Query private var identities: [PersistentIdentity]

    /// The store pages one Read or Read more asks for. One tap, one call:
    /// the screen never reads on by itself.
    static let pagesPerRead: UInt32 = 8

    @State private var selectedWalletId: Data?
    /// The entries read since the last Read (or the last read that
    /// restarted), in store order.
    @State private var entries: [PrivateDocumentEntry] = []
    /// The latest read's answer; nil before the first read.
    @State private var lastRead: PrivateDocumentEntriesRead?
    /// Pages and refused entries across this screen's reads since the last
    /// Read, or the last read that restarted.
    @State private var pagesReadTotal: UInt64 = 0
    @State private var rejectedTotal: UInt64 = 0
    @State private var isReading = false
    @State private var failure: ReadFailure?

    private enum ReadFailure: Equatable {
        /// The network does not serve private stores yet.
        case notActive
        /// Shown as given.
        case message(String)
    }

    var body: some View {
        Form {
            walletSection
            readSection
            if let failure {
                failureSection(failure)
            }
            if let lastRead {
                resultSection(lastRead)
                statusSection(lastRead)
            }
        }
        .overlay {
            if scenePhase != .active {
                privacyCover
            }
        }
        .navigationTitle("\(documentTypeName) Entries")
        .navigationBarTitleDisplayMode(.inline)
        .onAppear(perform: ensureDefaultSelection)
        .onChange(of: wallets) {
            ensureDefaultSelection()
        }
    }

    // MARK: - Sections

    /// Opaque cover over the form while the scene is inactive or in the
    /// background. It only hides the entries: the read state stays.
    private var privacyCover: some View {
        ZStack {
            Color(.systemBackground)
            VStack(spacing: 12) {
                Image(systemName: "lock.fill")
                    .font(.largeTitle)
                    .foregroundColor(.secondary)
                Text("Private entries hidden")
                    .font(.headline)
                    .foregroundColor(.secondary)
            }
        }
        .ignoresSafeArea()
        .accessibilityElement(children: .combine)
        .accessibilityIdentifier("privateEntries.privacyCover")
    }

    @ViewBuilder
    private var walletSection: some View {
        Section {
            if wallets.isEmpty {
                Text("No wallets on this network. Create or import a wallet first.")
                    .font(.caption)
                    .foregroundColor(.secondary)
            } else {
                Picker(
                    "Wallet",
                    selection: Binding<Data?>(
                        get: { selectedWalletId },
                        set: { newId in
                            selectedWalletId = newId
                            // Another wallet's identities open other entries
                            resetRead()
                        }
                    )
                ) {
                    ForEach(wallets, id: \.walletId) { wallet in
                        Text("\(wallet.label) \(fingerprint(wallet.walletId))")
                            .tag(Optional(wallet.walletId))
                    }
                }
                .pickerStyle(.menu)
                .disabled(isReading)
                .accessibilityIdentifier("privateEntries.walletPicker")
                if selectedWalletId != nil && selectedManagedWallet == nil {
                    Text("This wallet is not loaded yet.")
                        .font(.caption)
                        .foregroundColor(.orange)
                }
            }
        } header: {
            Text("Wallet")
        } footer: {
            Text("Entries name nobody: the wallet finds its identities' entries by trying to open each one with their keys, and recovers the ones their keys sealed.")
        }
    }

    @ViewBuilder
    private var readSection: some View {
        Section {
            Button {
                read(from: nil)
            } label: {
                HStack {
                    if isReading {
                        ProgressView().controlSize(.small)
                    }
                    Text(lastRead == nil ? "Read" : "Read from the start")
                }
                .frame(maxWidth: .infinity)
            }
            .disabled(isReading || selectedManagedWallet == nil)
            .accessibilityIdentifier("privateEntries.readButton")

            if let lastRead, !lastRead.complete, lastRead.identityCount > 0 {
                Button {
                    read(from: lastRead.cursor)
                } label: {
                    Text("Read more")
                        .frame(maxWidth: .infinity)
                }
                .disabled(isReading || selectedManagedWallet == nil)
                .accessibilityIdentifier("privateEntries.readMoreButton")
            }
        } footer: {
            Text("Each read covers at most \(Self.pagesPerRead) pages of the store.")
        }
    }

    @ViewBuilder
    private func failureSection(_ failure: ReadFailure) -> some View {
        Section {
            switch failure {
            case .notActive:
                Label(
                    "This network does not serve private document stores yet: they arrive with protocol version 15.",
                    systemImage: "clock.badge.exclamationmark"
                )
                .font(.caption)
                .foregroundColor(.secondary)
                .accessibilityIdentifier("privateEntries.notActive")
            case .message(let message):
                Text(message)
                    .font(.caption)
                    .foregroundColor(.red)
                    .textSelection(.enabled)
                    .accessibilityIdentifier("privateEntries.error")
            }
        }
    }

    @ViewBuilder
    private func resultSection(_ read: PrivateDocumentEntriesRead) -> some View {
        if read.identityCount == 0 {
            Section {
                Text("No identity of this wallet can open entries, so nothing was read. An identity needs an encryption or decryption key to be sent entries.")
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .accessibilityIdentifier("privateEntries.noIdentity")
            }
        } else if entries.isEmpty {
            Section {
                Text("Nothing for this wallet in the entries read.")
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .accessibilityIdentifier("privateEntries.empty")
            }
        } else {
            Section("Entries (\(entries.count))") {
                ForEach(entries, id: \.rowId) { entry in
                    PrivateDocumentEntryRow(
                        entry: entry,
                        identityName: identityName(for:)
                    )
                }
            }
        }
    }

    private func statusSection(_ read: PrivateDocumentEntriesRead) -> some View {
        Section("Status") {
            VStack(alignment: .leading, spacing: 4) {
                InfoRow(label: "Pages read:", value: "\(pagesReadTotal)")
                InfoRow(
                    label: "Store size:",
                    value: read.cursor.provedTotalCount.map { "\($0)" } ?? "Unknown"
                )
                InfoRow(label: "Next position:", value: "\(read.cursor.position)")
                // Entries that opened for the wallet but whose envelope was
                // refused; a payload the schema refused shows on its row.
                InfoRow(label: "Refused envelopes:", value: "\(rejectedTotal)")
                InfoRow(label: "Read to the end:", value: read.complete ? "Yes" : "No")
            }
            .font(.caption)
            .accessibilityElement(children: .combine)
            .accessibilityIdentifier("privateEntries.status")

            if read.unscannedKeyCount > 0 {
                VStack(alignment: .leading, spacing: 4) {
                    InfoRow(label: "Keys not scanned:", value: "\(read.unscannedKeyCount)")
                    Text("Entries sealed to these keys (such as a DashPay Connect app key) are not read.")
                        .foregroundColor(.secondary)
                }
                .font(.caption)
                .accessibilityElement(children: .combine)
                .accessibilityIdentifier("privateEntries.unscannedKeys")
            }
        }
    }

    // MARK: - Reading

    /// One read of at most `pagesPerRead` pages: from the store's start when
    /// `cursor` is nil, else on from the cursor the last read returned.
    private func read(from cursor: PrivateDocumentReadCursor?) {
        guard let wallet = selectedManagedWallet else { return }
        let contractId = self.contractId
        let walletId = selectedWalletId
        let typeName = documentTypeName
        isReading = true
        failure = nil
        Task {
            defer { isReading = false }
            do {
                let result = try await wallet.readPrivateDocumentEntries(
                    contractId: contractId,
                    documentTypeName: typeName,
                    from: cursor,
                    maxPages: Self.pagesPerRead
                )
                // The user picked another wallet while this read ran
                guard walletId == selectedWalletId else { return }
                // A read that restarted (the wallet's keys changed since the
                // cursor) read from the store's start: it replaces the list.
                if cursor != nil && !result.restarted {
                    entries.append(contentsOf: result.entries)
                    pagesReadTotal += UInt64(result.pagesRead)
                    rejectedTotal += UInt64(result.rejectedCount)
                } else {
                    entries = result.entries
                    pagesReadTotal = UInt64(result.pagesRead)
                    rejectedTotal = UInt64(result.rejectedCount)
                }
                lastRead = result
            } catch {
                guard walletId == selectedWalletId else { return }
                failure = Self.failure(for: error)
            }
        }
    }

    /// How a failed read is shown: a network before protocol version 15 as
    /// that, everything else as the error says.
    private static func failure(for error: Error) -> ReadFailure {
        if let walletError = error as? PlatformWalletError {
            switch walletError {
            case .protocolVersionNotActive:
                return .notActive
            case .sdkPinnedBelowProtocolVersion:
                return .message("This build's SDK is pinned below protocol version 15, which private document types need. Raise or remove the SDK's protocol version pin.")
            default:
                break
            }
        }
        return .message(error.localizedDescription)
    }

    private func resetRead() {
        entries = []
        lastRead = nil
        pagesReadTotal = 0
        rejectedTotal = 0
        failure = nil
    }

    // MARK: - Derived state

    private var wallets: [PersistentWallet] {
        let raw = appState.currentNetwork.rawValue
        return allWallets.filter { $0.networkRaw == raw }
    }

    private var selectedManagedWallet: ManagedPlatformWallet? {
        guard let id = selectedWalletId else { return nil }
        return walletManager.wallet(for: id)
    }

    private func ensureDefaultSelection() {
        if let current = selectedWalletId, wallets.contains(where: { $0.walletId == current }) {
            return
        }
        selectedWalletId = wallets.first?.walletId
        resetRead()
    }

    /// A known identity's `displayName` on the current network: its alias, or
    /// a DPNS name it still owns (a sold name kept as the stored main pick is
    /// skipped), else its own short id. An identity this device does not
    /// know shows its base58 id shortened.
    private func identityName(for id: Data) -> String {
        if let identity = identities.first(where: {
            $0.identityId == id && $0.networkRaw == appState.currentNetwork.rawValue
        }) {
            return identity.displayName
        }
        let base58 = id.toBase58String()
        guard base58.count > 14 else { return base58 }
        return "\(base58.prefix(8))…\(base58.suffix(4))"
    }

    private func fingerprint(_ walletId: Data) -> String {
        let hex = walletId.prefix(4).map { String(format: "%02x", $0) }.joined()
        return hex.isEmpty ? "" : "(\(hex))"
    }
}

private extension PrivateDocumentEntry {
    /// Unique per row: an entry between two of the wallet's identities is
    /// listed once each way.
    var rowId: String { "\(position).\(directionName)" }

    var directionName: String {
        switch direction {
        case .received: return "received"
        case .sent: return "sent"
        }
    }
}

/// One opened entry: which way it went, where and when it was appended, who
/// it is for, who it claims to be from, and what it holds.
private struct PrivateDocumentEntryRow: View {
    let entry: PrivateDocumentEntry
    let identityName: (Data) -> String

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack {
                Label(
                    entry.direction == .received ? "Received" : "Recovered (outgoing)",
                    systemImage: entry.direction == .received ? "tray.and.arrow.down" : "paperplane"
                )
                .font(.headline)
                .foregroundColor(entry.direction == .received ? .green : .blue)
                Spacer()
                Text("#\(entry.position)")
                    .font(.caption.monospaced())
                    .foregroundColor(.secondary)
            }

            Text("Appended "
                + entry.blockTime.formatted(date: .abbreviated, time: .standard)
                + " · block \(entry.blockHeight)")
                .font(.caption2)
                .foregroundColor(.secondary)

            detail(
                // Recovery with an outgoing viewing key shows only that one
                // of the wallet's keys can recover this ciphertext, not that
                // the wallet appended it: a later copy recovers too.
                entry.direction == .received ? "Opened by" : "Recovered with",
                "\(identityName(entry.walletIdentityId)) (key \(entry.walletKeyId))"
            )
            detail("Recipient", identityName(entry.recipientIdentityId))
            detail("Authentication", authModeName)
            if let sender = entry.claimedSenderIdentityId {
                detail(
                    "Claimed sender",
                    "\(identityName(sender))"
                        + (entry.claimedSenderKeyId.map { " (key \($0))" } ?? "")
                        + ", not verified"
                )
                .foregroundColor(.orange)
            }

            switch entry.payload {
            case .properties(let json):
                ForEach(Self.propertyRows(json), id: \.key) { row in
                    HStack(alignment: .top) {
                        Text(row.key)
                            .font(.caption.weight(.medium))
                        Spacer()
                        Text(row.value)
                            .font(.caption.monospaced())
                            .multilineTextAlignment(.trailing)
                            .textSelection(.enabled)
                    }
                }
            case .refused(let reason):
                Label("Payload refused: \(reason)", systemImage: "exclamationmark.triangle")
                    .font(.caption)
                    .foregroundColor(.red)
            }
        }
        .padding(.vertical, 4)
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("privateEntries.entry.\(entry.rowId)")
    }

    private var authModeName: String {
        switch entry.authMode {
        case .anonymous: return "Anonymous"
        case .deniableMac: return "Deniable MAC"
        case .signature: return "Signature"
        }
    }

    private func detail(_ label: String, _ value: String) -> some View {
        HStack(alignment: .top) {
            Text(label)
                .foregroundColor(.secondary)
            Spacer()
            Text(value)
                .multilineTextAlignment(.trailing)
        }
        .font(.caption)
    }

    /// The JSON object's members as text, sorted by name: a string as it is,
    /// any other value as its JSON.
    private static func propertyRows(_ json: String) -> [(key: String, value: String)] {
        guard let data = json.data(using: .utf8),
              let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        else {
            return [("properties", json)]
        }
        return object.keys.sorted().map { key in
            let value = object[key]
            if let text = value as? String {
                return (key, text)
            }
            if let value,
               let data = try? JSONSerialization.data(
                   withJSONObject: value,
                   options: [.fragmentsAllowed, .sortedKeys]
               ),
               let text = String(data: data, encoding: .utf8) {
                return (key, text)
            }
            return (key, "\(value ?? "null")")
        }
    }
}
