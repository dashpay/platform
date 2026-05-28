import SwiftUI
import SwiftData
import SwiftDashSDK

// MARK: - Reports

/// Per-key diagnosis. Status drives the row's icon + the parent
/// identity's roll-up severity.
struct WalletKeyHealth: Identifiable {
    let id: Int32  // keyId (stable per identity)
    let keyId: UInt32
    let purposeRaw: UInt8
    let securityLevelRaw: UInt8
    let storedPublicKeyHex: String
    let derivedPublicKeyHex: String
    let status: Status
    /// Held so the re-derive action can write the new pkid back.
    let row: PersistentPublicKey

    enum Status {
        /// Stored pubkey matches the wallet's derivation AND the
        /// Keychain entry holds matching private bytes.
        case healthy
        /// Stored pubkey matches the derivation but the Keychain
        /// entry is missing or holds different bytes — a re-derive
        /// will repair it.
        case needsRederive(reason: String)
        /// Stored pubkey doesn't match what this wallet's mnemonic
        /// derives at `(identityIndex, keyId)`. The identity row
        /// belongs to a different wallet (or a stale mnemonic).
        /// Repair is destructive — only delete-identity.
        case orphan(reason: String)
    }

    var iconName: String {
        switch status {
        case .healthy: return "checkmark.circle.fill"
        case .needsRederive: return "exclamationmark.triangle.fill"
        case .orphan: return "xmark.circle.fill"
        }
    }

    var iconColor: Color {
        switch status {
        case .healthy: return .green
        case .needsRederive: return .orange
        case .orphan: return .red
        }
    }

    var statusLabel: String {
        switch status {
        case .healthy: return "Healthy"
        case .needsRederive(let reason): return "Needs re-derive — \(reason)"
        case .orphan(let reason): return "Orphan — \(reason)"
        }
    }

    var purposeName: String {
        KeyPurpose(rawValue: purposeRaw)?.name ?? "Purpose \(purposeRaw)"
    }

    var securityLevelName: String {
        SecurityLevel(rawValue: securityLevelRaw)?.name ?? "Level \(securityLevelRaw)"
    }
}

/// Per-identity rollup. Severity is the highest-severity child key
/// status; drives which repair action is offered (re-derive vs
/// delete-identity).
struct WalletIdentityKeyHealthReport: Identifiable {
    let id: Data  // identity id bytes
    let identityIdBase58: String
    let identityIndex: UInt32
    let keys: [WalletKeyHealth]
    /// Held so the delete-identity action can cascade-remove the row.
    let identityRow: PersistentIdentity

    enum Severity { case healthy, needsRederive, orphan }

    var severity: Severity {
        if keys.contains(where: { if case .orphan = $0.status { true } else { false } }) {
            return .orphan
        }
        if keys.contains(where: { if case .needsRederive = $0.status { true } else { false } }) {
            return .needsRederive
        }
        return .healthy
    }
}

// MARK: - Checker / repair

/// Construct the new (walletId-namespaced) keychain account name a
/// key SHOULD be stored under. Mirrors `KeychainManager.storeIdentityPrivateKey`.
/// Pulled out here so the health check can do strict direct lookups
/// instead of falling through to the metadata-scan path that also
/// matches legacy entries.
fileprivate func namespacedKeychainAccount(walletIdHex: String, derivationPath: String) -> String {
    "identity_privkey.\(walletIdHex).\(derivationPath)"
}

/// Construct the OLD (no-walletId) keychain account name. Used only
/// for the post-rederive cleanup sweep.
fileprivate func legacyKeychainAccount(derivationPath: String) -> String {
    "identity_privkey.\(derivationPath)"
}

/// Best-effort delete of the legacy (no-walletId) keychain entry for
/// `derivationPath`, gated on its metadata's `walletId` matching the
/// wallet we just migrated from. Skips silently if the metadata says
/// the row belongs to a different wallet — never clobber data we
/// don't own. No-op when the row doesn't exist.
///
/// Called after a successful re-derive in
/// `WalletKeyHealthChecker.rederive` so the keychain ends up with a
/// single new-format entry per `(wallet, path)`.
@MainActor
fileprivate func cleanupLegacyKeychainEntry(walletIdHex: String, derivationPath: String) {
    let account = legacyKeychainAccount(derivationPath: derivationPath)
    let serviceName = KeychainManager.shared.serviceName
    let lookupQuery: [String: Any] = [
        kSecClass as String: kSecClassGenericPassword,
        kSecAttrService as String: serviceName,
        kSecAttrAccount as String: account,
        kSecReturnAttributes as String: true,
        kSecMatchLimit as String: kSecMatchLimitOne,
    ]

    var result: AnyObject?
    let lookupStatus = SecItemCopyMatching(lookupQuery as CFDictionary, &result)
    guard lookupStatus == errSecSuccess, let attrs = result as? [String: Any] else {
        return
    }
    guard let metadataData = attrs[kSecAttrGeneric as String] as? Data,
          let metadata = try? JSONDecoder().decode(
            IdentityPrivateKeyMetadata.self,
            from: metadataData
          )
    else {
        return
    }
    guard metadata.walletId.caseInsensitiveCompare(walletIdHex) == .orderedSame else {
        // Different wallet's data sitting at the legacy account —
        // leave it. (Pathological since we'd have to have lost the
        // namespaced fix at some point, but defending it is cheap.)
        return
    }

    let deleteQuery: [String: Any] = [
        kSecClass as String: kSecClassGenericPassword,
        kSecAttrService as String: serviceName,
        kSecAttrAccount as String: account,
    ]
    _ = SecItemDelete(deleteQuery as CFDictionary)
}

/// Pure helpers — no UI state. Constructed lazily inside the sheet's
/// `.task { … }` so the heavy derivation + Keychain scans happen on a
/// background context.
enum WalletKeyHealthChecker {

    /// Derive each PersistentPublicKey's canonical key from the
    /// wallet's mnemonic and classify against the stored row +
    /// Keychain bytes. Pure read — does not mutate state.
    @MainActor
    static func runCheck(
        wallet: ManagedPlatformWallet,
        walletId: Data,
        identities: [PersistentIdentity],
        network: Network
    ) -> [WalletIdentityKeyHealthReport] {
        var reports: [WalletIdentityKeyHealthReport] = []
        for identity in identities {
            let sortedKeys = identity.publicKeys.sorted { $0.keyId < $1.keyId }
            var keyHealths: [WalletKeyHealth] = []
            for row in sortedKeys {
                let kid = UInt32(bitPattern: row.keyId)
                let purposeRaw = UInt8(row.purpose) ?? 0
                let levelRaw = UInt8(row.securityLevel) ?? 0
                let storedHex = row.publicKeyData.toHexString()

                let status: WalletKeyHealth.Status
                let derivedHex: String

                do {
                    let preview = try wallet.deriveIdentityAuthKeyAtSlot(
                        identityIndex: identity.identityIndex,
                        keyId: kid,
                        network: network
                    )
                    derivedHex = preview.publicKeyHex

                    if preview.publicKeyData == row.publicKeyData {
                        // pubkey matches the wallet's mnemonic →
                        // look up the keychain bytes at the expected
                        // walletId-namespaced account.
                        //
                        // We intentionally DON'T fall back to the
                        // metadata-scan lookup
                        // (`retrieveIdentityPrivateKey(publicKeyHex:)`)
                        // here — that path also finds legacy
                        // (non-namespaced) entries, which would
                        // mask migration debt. Reporting legacy-only
                        // keys as "needsRederive" surfaces them in
                        // the sheet so the user can migrate them
                        // explicitly.
                        let expectedAccount = namespacedKeychainAccount(
                            walletIdHex: walletId.toHexString(),
                            derivationPath: preview.derivationPath
                        )
                        if let kcBytes = KeychainManager.shared
                            .retrieveKeyData(identifier: expectedAccount)
                        {
                            if kcBytes == preview.privateKeyData {
                                status = .healthy
                            } else {
                                status = .needsRederive(
                                    reason: "Keychain bytes at \(expectedAccount.suffix(40)) don't match the derived private key"
                                )
                            }
                        } else {
                            status = .needsRederive(
                                reason: "No new-format Keychain entry (legacy-only entries don't count)"
                            )
                        }
                    } else {
                        status = .orphan(
                            reason: "Stored pubkey \(storedHex.prefix(12))… doesn't match wallet's derivation \(derivedHex.prefix(12))…"
                        )
                    }
                } catch {
                    derivedHex = ""
                    status = .orphan(
                        reason: "Derivation failed: \(error.localizedDescription)"
                    )
                }

                keyHealths.append(
                    WalletKeyHealth(
                        id: row.keyId,
                        keyId: kid,
                        purposeRaw: purposeRaw,
                        securityLevelRaw: levelRaw,
                        storedPublicKeyHex: storedHex,
                        derivedPublicKeyHex: derivedHex,
                        status: status,
                        row: row
                    )
                )
            }
            reports.append(
                WalletIdentityKeyHealthReport(
                    id: identity.identityId,
                    identityIdBase58: identity.identityIdBase58,
                    identityIndex: identity.identityIndex,
                    keys: keyHealths,
                    identityRow: identity
                )
            )
        }
        return reports
    }

    /// Re-derive every key in `report` whose status is
    /// `.needsRederive`, write fresh Keychain entries (at the new
    /// walletId-namespaced account), and update each
    /// `PersistentPublicKey.privateKeyKeychainIdentifier` to point
    /// at the new account. Returns the number of keys fixed.
    @MainActor
    static func rederive(
        report: WalletIdentityKeyHealthReport,
        wallet: ManagedPlatformWallet,
        walletId: Data,
        network: Network,
        modelContext: ModelContext
    ) throws -> Int {
        var fixed = 0
        for key in report.keys {
            guard case .needsRederive = key.status else { continue }
            let preview = try wallet.deriveIdentityAuthKeyAtSlot(
                identityIndex: report.identityIndex,
                keyId: key.keyId,
                network: network
            )
            let pubkeyHashHex = SwiftDashSDK.KeychainManager.computePublicKeyHashHex(preview.publicKeyData)
            let metadata = IdentityPrivateKeyMetadata(
                identityId: report.identityIdBase58,
                keyId: key.keyId,
                walletId: walletId.toHexString(),
                identityIndex: report.identityIndex,
                keyIndex: key.keyId,
                derivationPath: preview.derivationPath,
                publicKey: preview.publicKeyHex,
                publicKeyHash: pubkeyHashHex,
                keyType: UInt8(key.row.keyType) ?? 0,
                purpose: key.purposeRaw,
                securityLevel: key.securityLevelRaw
            )
            guard let pkid = KeychainManager.shared.storeIdentityPrivateKey(
                preview.privateKeyData,
                derivationPath: preview.derivationPath,
                metadata: metadata
            ) else {
                continue
            }
            key.row.privateKeyKeychainIdentifier = pkid

            // Sweep the legacy (no-walletId) account for the same
            // derivation path. Only delete if its metadata's
            // walletId matches THIS wallet — defensive guard against
            // ever clobbering another wallet's keys, in case a
            // future collision lands data we don't own at that
            // account.
            cleanupLegacyKeychainEntry(
                walletIdHex: walletId.toHexString(),
                derivationPath: preview.derivationPath
            )
            fixed += 1
        }
        if fixed > 0 {
            try modelContext.save()
        }
        return fixed
    }

    /// Cascade-delete an orphan identity from SwiftData. Safe to
    /// call now that the relationship inverses
    /// (`PersistentPublicKey.identity`, `PersistentDPNSName.identity`,
    /// `PersistentDashpayProfile.identity`, `PersistentDashpayContactRequest.owner`)
    /// are all Optional — see the doc comments on those models for
    /// the SwiftData cascade-on-non-optional crash this avoids.
    @MainActor
    static func deleteOrphan(
        identity: PersistentIdentity,
        modelContext: ModelContext
    ) throws {
        modelContext.delete(identity)
        try modelContext.save()
    }
}

// MARK: - View

/// Modal sheet that runs the health check on appear and renders the
/// per-identity / per-key result, with one-tap repair actions.
struct WalletKeyHealthSheet: View {
    let wallet: ManagedPlatformWallet
    let walletId: Data
    /// Snapshotted at construction so the sheet shows what was true
    /// when opened, not what changes underneath. Re-fetch by closing
    /// and re-opening.
    let identities: [PersistentIdentity]
    let network: Network

    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) private var dismiss

    @State private var reports: [WalletIdentityKeyHealthReport] = []
    @State private var isRunning = false
    @State private var actionMessage: String?
    @State private var errorMessage: String?
    @State private var pendingOrphanDelete: WalletIdentityKeyHealthReport?

    var body: some View {
        NavigationView {
            Form {
                if isRunning {
                    Section {
                        HStack {
                            ProgressView()
                            Text("Checking keys…")
                                .foregroundColor(.secondary)
                        }
                    }
                } else if reports.isEmpty {
                    Section {
                        Text("No identities to check.")
                            .foregroundColor(.secondary)
                    }
                } else {
                    summarySection
                    ForEach(reports) { report in
                        identitySection(report)
                    }
                }
                if let actionMessage {
                    Section {
                        Text(actionMessage)
                            .font(.caption)
                            .foregroundColor(.green)
                    }
                }
                if let errorMessage {
                    Section {
                        Text(errorMessage)
                            .font(.caption)
                            .foregroundColor(.red)
                    }
                }
            }
            .navigationTitle("Verify Identity Keys")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Done") { dismiss() }
                }
            }
            .task { await runCheck() }
            .alert(
                "Delete Identity",
                isPresented: Binding(
                    get: { pendingOrphanDelete != nil },
                    set: { if !$0 { pendingOrphanDelete = nil } }
                )
            ) {
                Button("Cancel", role: .cancel) { pendingOrphanDelete = nil }
                Button("Delete", role: .destructive) {
                    if let report = pendingOrphanDelete {
                        deleteIdentity(report)
                        pendingOrphanDelete = nil
                    }
                }
            } message: {
                if let report = pendingOrphanDelete {
                    Text(
                        "Identity \(report.identityIdBase58.prefix(12))… doesn't match this wallet's mnemonic. " +
                        "Deleting it removes the SwiftData row and all associated keys / DashPay state. " +
                        "The on-chain identity is unaffected."
                    )
                }
            }
        }
    }

    @ViewBuilder
    private var summarySection: some View {
        Section("Summary") {
            HStack {
                Text("Identities checked")
                Spacer()
                Text("\(reports.count)").foregroundColor(.secondary)
            }
            let healthy = reports.filter { $0.severity == .healthy }.count
            let needs = reports.filter { $0.severity == .needsRederive }.count
            let orphans = reports.filter { $0.severity == .orphan }.count
            HStack {
                Label("Healthy", systemImage: "checkmark.circle.fill")
                    .foregroundColor(.green)
                Spacer()
                Text("\(healthy)").foregroundColor(.secondary)
            }
            HStack {
                Label("Needs re-derive", systemImage: "exclamationmark.triangle.fill")
                    .foregroundColor(.orange)
                Spacer()
                Text("\(needs)").foregroundColor(.secondary)
            }
            HStack {
                Label("Orphan", systemImage: "xmark.circle.fill")
                    .foregroundColor(.red)
                Spacer()
                Text("\(orphans)").foregroundColor(.secondary)
            }
        }
    }

    @ViewBuilder
    private func identitySection(_ report: WalletIdentityKeyHealthReport) -> some View {
        Section {
            ForEach(report.keys) { key in
                VStack(alignment: .leading, spacing: 4) {
                    HStack {
                        Image(systemName: key.iconName)
                            .foregroundColor(key.iconColor)
                        Text("Key #\(key.keyId) — \(key.purposeName), \(key.securityLevelName)")
                            .font(.subheadline)
                            .fontWeight(.medium)
                        Spacer()
                    }
                    Text(key.statusLabel)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                .padding(.vertical, 2)
            }
            actionRow(for: report)
        } header: {
            HStack {
                Image(systemName: severityIcon(report.severity))
                    .foregroundColor(severityColor(report.severity))
                Text("Identity \(report.identityIdBase58.prefix(12))… (idx \(report.identityIndex))")
            }
        }
    }

    @ViewBuilder
    private func actionRow(for report: WalletIdentityKeyHealthReport) -> some View {
        switch report.severity {
        case .healthy:
            EmptyView()
        case .needsRederive:
            Button {
                rederive(report)
            } label: {
                Label("Re-derive missing keys", systemImage: "arrow.triangle.2.circlepath")
                    .foregroundColor(.blue)
            }
        case .orphan:
            Button(role: .destructive) {
                pendingOrphanDelete = report
            } label: {
                Label("Delete this identity", systemImage: "trash")
            }
        }
    }

    private func severityIcon(_ s: WalletIdentityKeyHealthReport.Severity) -> String {
        switch s {
        case .healthy: return "checkmark.circle.fill"
        case .needsRederive: return "exclamationmark.triangle.fill"
        case .orphan: return "xmark.circle.fill"
        }
    }

    private func severityColor(_ s: WalletIdentityKeyHealthReport.Severity) -> Color {
        switch s {
        case .healthy: return .green
        case .needsRederive: return .orange
        case .orphan: return .red
        }
    }

    // MARK: Actions

    @MainActor
    private func runCheck() async {
        isRunning = true
        defer { isRunning = false }
        actionMessage = nil
        errorMessage = nil
        reports = WalletKeyHealthChecker.runCheck(
            wallet: wallet,
            walletId: walletId,
            identities: identities,
            network: network
        )
    }

    private func rederive(_ report: WalletIdentityKeyHealthReport) {
        Task { @MainActor in
            do {
                let fixed = try WalletKeyHealthChecker.rederive(
                    report: report,
                    wallet: wallet,
                    walletId: walletId,
                    network: network,
                    modelContext: modelContext
                )
                actionMessage = "Re-derived \(fixed) key\(fixed == 1 ? "" : "s") for identity \(report.identityIdBase58.prefix(12))…"
                errorMessage = nil
                // Re-run the check so the report reflects the new
                // state (formerly-orange rows should turn green).
                await runCheck()
            } catch {
                errorMessage = "Re-derive failed: \(error.localizedDescription)"
            }
        }
    }

    private func deleteIdentity(_ report: WalletIdentityKeyHealthReport) {
        do {
            try WalletKeyHealthChecker.deleteOrphan(
                identity: report.identityRow,
                modelContext: modelContext
            )
            actionMessage = "Deleted orphan identity \(report.identityIdBase58.prefix(12))…"
            errorMessage = nil
            // Drop the now-deleted row from the report list so the
            // sheet doesn't try to render it again.
            reports.removeAll { $0.id == report.id }
        } catch {
            errorMessage = "Delete failed: \(error.localizedDescription)"
        }
    }
}
