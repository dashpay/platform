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

/// Best-effort delete of the legacy (no-walletId) keychain entry for
/// `derivationPath`, gated on its metadata's `walletId` matching the
/// wallet we just migrated from. Delegates to
/// `KeychainManager.deleteLegacyKeychainEntryIfOwnedByWallet` so the
/// "don't clobber data we don't own" safety check is centralized —
/// shared with `KeychainManager.deleteIdentityPrivateKey`'s sweep.
///
/// Called after a successful re-derive in
/// `WalletKeyHealthChecker.rederive` so the keychain ends up with a
/// single new-format entry per `(wallet, path)`.
@MainActor
fileprivate func cleanupLegacyKeychainEntry(walletIdHex: String, derivationPath: String) {
    _ = KeychainManager.shared.deleteLegacyKeychainEntryIfOwnedByWallet(
        walletIdHex: walletIdHex,
        derivationPath: derivationPath
    )
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

                    // `row.publicKeyData` stores whatever shape was
                    // registered on Platform — 33-byte compressed
                    // pubkey for `.ecdsaSecp256k1`, 20-byte HASH160
                    // for `.ecdsaHash160`. The Rust-derived preview
                    // is always the raw 33-byte pubkey; hash it
                    // ourselves before comparing for HASH160 rows.
                    // (Other variants — BLS, BIP13 script-hash,
                    // EdDSA — aren't produced by this preview path
                    // today; treat them as `notSupported` so the
                    // diagnostic surfaces them instead of silently
                    // misclassifying them as orphans.)
                    let derivedComparableHex: String?
                    switch row.keyTypeEnum ?? .ecdsaSecp256k1 {
                    case .ecdsaSecp256k1:
                        derivedComparableHex = preview.publicKeyHex
                        derivedHex = preview.publicKeyHex
                    case .ecdsaHash160:
                        let hashHex = SwiftDashSDK.KeychainManager
                            .computePublicKeyHashHex(preview.publicKeyData)
                        derivedComparableHex = hashHex.isEmpty ? nil : hashHex
                        derivedHex = hashHex
                    case .bls12_381, .bip13ScriptHash, .eddsa25519Hash160:
                        derivedComparableHex = nil
                        derivedHex = preview.publicKeyHex
                    }

                    if let derivedComparableHex,
                       derivedComparableHex.caseInsensitiveCompare(storedHex) == .orderedSame {
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
                    } else if derivedComparableHex == nil {
                        // Key type isn't one the diagnostic knows
                        // how to compare against a Rust-derived
                        // pubkey today (BLS / BIP13 / EdDSA). Report
                        // it as orphan with a clear reason so the
                        // user knows we can't verify it, rather than
                        // silently passing.
                        let label = row.keyTypeEnum?.name ?? "type \(row.keyType)"
                        status = .orphan(
                            reason: "Key type \(label) isn't supported by the diagnostic — can't verify against derived pubkey"
                        )
                    } else {
                        status = .orphan(
                            reason: "Stored \(storedHex.prefix(12))… doesn't match wallet's derivation \(derivedHex.prefix(12))…"
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

    /// Per-key result of a rederive pass. `success` is the count of
    /// keys whose Keychain entry was rewritten; `failures` lists each
    /// key the pass tried and couldn't fix, with a reason — useful so
    /// the sheet can show the user *which* key is stuck (not just
    /// "Re-derived 0 keys").
    struct RederiveOutcome {
        let success: Int
        /// `(keyId, reason)` for each `.needsRederive` key the loop
        /// touched but did not fix.
        let failures: [(UInt32, String)]
    }

    /// Re-derive every key in `report` whose status is
    /// `.needsRederive`, write fresh Keychain entries (at the new
    /// walletId-namespaced account), and update each
    /// `PersistentPublicKey.privateKeyKeychainIdentifier` to point
    /// at the new account.
    ///
    /// Returns a `RederiveOutcome` with both the count fixed and a
    /// per-key list of failures. Individual key failures are
    /// collected rather than thrown so one bad key doesn't block
    /// repair of the rest of the identity's keys; throws only when
    /// the whole batch is unrecoverable (e.g. the SwiftData save at
    /// the end fails).
    @MainActor
    static func rederive(
        report: WalletIdentityKeyHealthReport,
        wallet: ManagedPlatformWallet,
        walletId: Data,
        network: Network,
        modelContext: ModelContext
    ) throws -> RederiveOutcome {
        var fixed = 0
        var failures: [(UInt32, String)] = []
        for key in report.keys {
            guard case .needsRederive = key.status else { continue }
            let preview: ManagedPlatformWallet.IdentityRegistrationKeyPreview
            do {
                preview = try wallet.deriveIdentityAuthKeyAtSlot(
                    identityIndex: report.identityIndex,
                    keyId: key.keyId,
                    network: network
                )
            } catch {
                failures.append((key.keyId, "derivation failed: \(error.localizedDescription)"))
                continue
            }
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
                failures.append((key.keyId, "Keychain write at \(preview.derivationPath) returned nil"))
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
        return RederiveOutcome(success: fixed, failures: failures)
    }

    /// Outcome of a single `deleteOrphan` call. Decouples the
    /// SwiftData side of the delete (the bit that affects the
    /// reactive UI — `@Query` results, the report list) from the
    /// Keychain side (which is observability / cleanup, not UI-
    /// reactive). The caller can drop the row from the report
    /// list as long as `swiftDataDeleted == true`, even when
    /// `keychainError` is non-nil.
    struct OrphanDeleteOutcome {
        let swiftDataDeleted: Bool
        let keychainError: Error?
    }

    /// Cascade-delete an orphan identity from SwiftData AND wipe its
    /// associated Keychain entries.
    ///
    /// Ordering:
    ///  1. Wipe Keychain first (`identity_privkey.*` rows whose
    ///     metadata matches this identity). Idempotent on retry.
    ///  2. Pre-delete the identity's cascade children that have
    ///     non-Optional inverses back to it — DPNS names, DashPay
    ///     profile, contact requests — and save. SwiftData fatals
    ///     on `save()` whenever it tries to null out a non-
    ///     Optional inverse, so the parent delete in step 3 cannot
    ///     be in the same save batch as these children; mirrors
    ///     the per-layer save pattern in
    ///     `PlatformWalletPersistenceHandler.deleteWalletData`.
    ///  3. Delete the identity itself + save.
    ///
    /// Returns a per-side outcome so the caller can distinguish
    /// "fully clean" from "SwiftData clean, Keychain warning" —
    /// the latter shouldn't prevent the sheet from dropping the
    /// now-deleted row from its in-memory `reports` list.
    /// Genuine SwiftData failures still throw (they're the ones
    /// that keep the row visible and actionable).
    @MainActor
    static func deleteOrphan(
        identity: PersistentIdentity,
        modelContext: ModelContext
    ) throws -> OrphanDeleteOutcome {
        // Snapshot the base58 id BEFORE the delete — once the row
        // is removed its computed accessor is invalid.
        let identityIdBase58 = identity.identityIdBase58
        var keychainError: Error?
        do {
            try KeychainManager.shared.deleteAllIdentityPrivateKeys(
                forIdentityIdBase58: identityIdBase58
            )
        } catch {
            keychainError = error
        }

        // PHASE 1: delete cascade children with non-Optional
        // inverses to identity. Same reasoning as
        // `PlatformWalletPersistenceHandler.deleteWalletData` —
        // PersistentPublicKey / Document / TokenBalance inverses
        // to identity are already Optional and don't need pre-
        // deletion; DPNS names, DashPay profile, contact requests
        // do.
        for name in Array(identity.dpnsNames) {
            modelContext.delete(name)
        }
        if let profile = identity.dashpayProfile {
            modelContext.delete(profile)
        }
        for cr in Array(identity.contactRequests) {
            modelContext.delete(cr)
        }
        try modelContext.save()

        // PHASE 2: delete the identity itself. Its problematic
        // cascade children are gone, so SwiftData has no non-
        // Optional inverse to null out during the merge phase.
        modelContext.delete(identity)
        try modelContext.save()

        return OrphanDeleteOutcome(
            swiftDataDeleted: true,
            keychainError: keychainError
        )
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
                let outcome = try WalletKeyHealthChecker.rederive(
                    report: report,
                    wallet: wallet,
                    walletId: walletId,
                    network: network,
                    modelContext: modelContext
                )
                let fixed = outcome.success
                var msg = "Re-derived \(fixed) key\(fixed == 1 ? "" : "s") for identity \(report.identityIdBase58.prefix(12))…"
                if !outcome.failures.isEmpty {
                    let detail = outcome.failures
                        .map { "kid \($0.0): \($0.1)" }
                        .joined(separator: "; ")
                    msg += " — \(outcome.failures.count) failed: \(detail)"
                }
                actionMessage = msg
                errorMessage = outcome.failures.isEmpty ? nil : msg
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
            let outcome = try WalletKeyHealthChecker.deleteOrphan(
                identity: report.identityRow,
                modelContext: modelContext
            )
            // SwiftData side succeeded → drop the now-deleted row
            // from the report list regardless of the keychain
            // side. Keeping it in `reports` after the SwiftData
            // delete leaves stale UI (the row's relationship
            // accessors are invalidated and any action against
            // them throws). A keychain-only failure becomes a
            // non-blocking warning instead of suppressing the
            // success state.
            if outcome.swiftDataDeleted {
                reports.removeAll { $0.id == report.id }
            }
            if let keychainError = outcome.keychainError {
                actionMessage = "Deleted orphan identity \(report.identityIdBase58.prefix(12))…"
                errorMessage = "Keychain cleanup warning: \(keychainError.localizedDescription) — re-running Verify Identity Keys can retry."
            } else {
                actionMessage = "Deleted orphan identity \(report.identityIdBase58.prefix(12))…"
                errorMessage = nil
            }
        } catch {
            // SwiftData delete failed (the in-memory `reports`
            // list is still authoritative); surface as a hard
            // error and leave the row visible so the user can
            // retry or escalate.
            errorMessage = "Delete failed: \(error.localizedDescription)"
        }
    }
}
