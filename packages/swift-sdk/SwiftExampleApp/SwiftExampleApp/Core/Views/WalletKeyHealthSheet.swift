import SwiftUI
import SwiftData
import SwiftDashSDK

// MARK: - Reports

/// Per-key diagnosis. Status drives the row's icon and, through
/// re-derive, the parent identity's roll-up severity.
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
        /// The key isn't the one this wallet derives at its slot
        /// `(identityIndex, keyId)`, so the wallet can't sign with it.
        /// Whose identity it is stays Rust's call (the report's
        /// `ownership`); the key alone offers no repair.
        case notWalletKey(reason: String)
        /// A key with nothing to compare it with: a type the wallet
        /// doesn't derive at the identity's key slots (Rust decides
        /// which), a row whose key type the app can't read, or a check
        /// that failed (a locked Keychain, say), which says nothing
        /// about whose key it is. Shown, but it leaves the identity's
        /// severity alone and offers no repair.
        case unchecked(reason: String)
    }

    var iconName: String {
        switch status {
        case .healthy: return "checkmark.circle.fill"
        case .needsRederive: return "exclamationmark.triangle.fill"
        case .notWalletKey: return "xmark.circle"
        case .unchecked: return "questionmark.circle.fill"
        }
    }

    var iconColor: Color {
        switch status {
        case .healthy: return .green
        case .needsRederive: return .orange
        case .notWalletKey: return .orange
        case .unchecked: return .secondary
        }
    }

    var statusLabel: String {
        switch status {
        case .healthy: return "Healthy"
        case .needsRederive(let reason): return "Needs re-derive — \(reason)"
        case .notWalletKey(let reason): return "Not this wallet's key: \(reason)"
        case .unchecked(let reason): return "Not checked: \(reason)"
        }
    }

    var purposeName: String {
        KeyPurpose(rawValue: purposeRaw)?.name ?? "Purpose \(purposeRaw)"
    }

    var securityLevelName: String {
        SecurityLevel(rawValue: securityLevelRaw)?.name ?? "Level \(securityLevelRaw)"
    }
}

/// Per-identity rollup; drives which repair action is offered
/// (re-derive vs delete-identity). Whose identity it is comes from
/// Rust (`ownership`, decided from the identity's enabled master key),
/// and the sheet offers delete only when Rust says it isn't the
/// wallet's.
struct WalletIdentityKeyHealthReport: Identifiable {
    let id: Data  // identity id bytes
    let identityIdBase58: String
    let identityIndex: UInt32
    let keys: [WalletKeyHealth]
    /// Rust's verdict on whose identity this is.
    let ownership: ManagedPlatformWallet.IdentityWalletOwnership
    /// Held so the delete-identity action can cascade-remove the row.
    let identityRow: PersistentIdentity

    enum Severity { case healthy, needsRederive, orphan }

    var severity: Severity {
        if ownership == .notOwned {
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

    /// Ask Rust (`checkIdentityKeys`) which of each identity's keys are
    /// the wallet's keys at their slots and whether the identity is the
    /// wallet's, and for a key that is, compare the Keychain entry with
    /// its private bytes. Rust decides; this inspects the Keychain and
    /// renders. Pure read: does not mutate state.
    @MainActor
    static func runCheck(
        wallet: ManagedPlatformWallet,
        walletId: Data,
        identities: [PersistentIdentity],
        network: Network
    ) -> [WalletIdentityKeyHealthReport] {
        identities.map { identity in
            checkIdentity(identity, wallet: wallet, walletId: walletId, network: network)
        }
    }

    /// One identity's report from a single Rust check of all its keys.
    @MainActor
    private static func checkIdentity(
        _ identity: PersistentIdentity,
        wallet: ManagedPlatformWallet,
        walletId: Data,
        network: Network
    ) -> WalletIdentityKeyHealthReport {
        let sortedKeys = identity.publicKeys.sorted { $0.keyId < $1.keyId }

        // A row the app can't read is not handed to Rust under a guessed
        // type, only counted: Rust can't call the identity foreign while
        // a key it wasn't shown could be the wallet's.
        var inputs: [ManagedPlatformWallet.IdentityKeyCheckInput] = []
        var unreadableKeyCount = 0
        for row in sortedKeys {
            guard let keyType = row.keyTypeEnum,
                  let purpose = UInt8(row.purpose),
                  let securityLevel = UInt8(row.securityLevel)
            else {
                unreadableKeyCount += 1
                continue
            }
            inputs.append(
                ManagedPlatformWallet.IdentityKeyCheckInput(
                    keyId: UInt32(bitPattern: row.keyId),
                    keyType: keyType,
                    purpose: purpose,
                    securityLevel: securityLevel,
                    publicKeyData: row.publicKeyData,
                    disabledAt: row.disabledAt.map { UInt64(clamping: $0) }
                )
            )
        }

        var checks: [UInt32: ManagedPlatformWallet.IdentityKeyWalletCheck] = [:]
        var ownership = ManagedPlatformWallet.IdentityWalletOwnership.undetermined
        var failure: String?
        do {
            let result = try wallet.checkIdentityKeys(
                identityIndex: identity.identityIndex,
                keys: inputs,
                unreadableKeyCount: unreadableKeyCount,
                network: network
            )
            for (input, check) in zip(inputs, result.keys) {
                checks[input.keyId] = check
            }
            ownership = result.ownership
        } catch {
            // An error (a locked Keychain, say) says nothing about whose
            // keys these are, so it must not offer to delete the identity.
            failure = "couldn't check the keys: \(error.localizedDescription)"
        }

        let keyHealths = sortedKeys.map { row in
            let keyId = UInt32(bitPattern: row.keyId)
            let storedHex = row.publicKeyData.toHexString()
            let (status, derivedHex) = keyStatus(
                row: row,
                check: checks[keyId],
                failure: failure,
                storedHex: storedHex,
                walletId: walletId
            )
            return WalletKeyHealth(
                id: row.keyId,
                keyId: keyId,
                purposeRaw: UInt8(row.purpose) ?? 0,
                securityLevelRaw: UInt8(row.securityLevel) ?? 0,
                storedPublicKeyHex: storedHex,
                derivedPublicKeyHex: derivedHex,
                status: status,
                row: row
            )
        }
        return WalletIdentityKeyHealthReport(
            id: identity.identityId,
            identityIdBase58: identity.identityIdBase58,
            identityIndex: identity.identityIndex,
            keys: keyHealths,
            ownership: ownership,
            identityRow: identity
        )
    }

    /// One key's status from Rust's check of it (`nil` when it wasn't
    /// sent or the check failed), and the wallet's key at its slot in
    /// hex (empty when there is none to show).
    @MainActor
    private static func keyStatus(
        row: PersistentPublicKey,
        check: ManagedPlatformWallet.IdentityKeyWalletCheck?,
        failure: String?,
        storedHex: String,
        walletId: Data
    ) -> (WalletKeyHealth.Status, String) {
        guard let check else {
            if let failure {
                return (.unchecked(reason: failure), "")
            }
            return (
                .unchecked(
                    reason: "the stored key type \(row.keyType), purpose \(row.purpose) or security level \(row.securityLevel) isn't one the app knows"
                ),
                ""
            )
        }
        switch check {
        case .notDerived:
            let typeName = row.keyTypeEnum?.name ?? "type \(row.keyType)"
            return (
                .unchecked(
                    reason: "the wallet doesn't derive \(typeName) keys from the identity's key slots"
                ),
                ""
            )
        case .notCompared:
            return (
                .unchecked(
                    reason: "the check compares ECDSA keys only in their 33-byte compressed form"
                ),
                ""
            )
        case .mismatch(let derivationPath, let derivedPublicKeyData):
            let derivedHex = derivedPublicKeyData.toHexString()
            return (
                .notWalletKey(
                    reason: "Stored \(storedHex.prefix(12))… doesn't match the wallet's key at \(derivationPath) (\(derivedHex.prefix(12))…)"
                ),
                derivedHex
            )
        case .matches(let derivationPath, let privateKeyData, _):
            return (
                keychainStatus(
                    walletId: walletId,
                    derivationPath: derivationPath,
                    privateKeyData: privateKeyData
                ),
                storedHex
            )
        }
    }

    /// The Keychain side of a key that is the wallet's: whether the
    /// walletId-namespaced entry at `derivationPath` holds
    /// `privateKeyData`.
    ///
    /// We intentionally DON'T fall back to the metadata-scan lookup
    /// (`retrieveIdentityPrivateKey(publicKeyHex:)`) here: that path
    /// also finds legacy (non-namespaced) entries, which would mask
    /// migration debt. Reporting legacy-only keys as "needsRederive"
    /// surfaces them in the sheet so the user can migrate them
    /// explicitly.
    @MainActor
    private static func keychainStatus(
        walletId: Data,
        derivationPath: String,
        privateKeyData: Data
    ) -> WalletKeyHealth.Status {
        let expectedAccount = namespacedKeychainAccount(
            walletIdHex: walletId.toHexString(),
            derivationPath: derivationPath
        )
        guard let kcBytes = KeychainManager.shared.retrieveKeyData(identifier: expectedAccount) else {
            return .needsRederive(
                reason: "No new-format Keychain entry (legacy-only entries don't count)"
            )
        }
        if kcBytes == privateKeyData {
            return .healthy
        }
        return .needsRederive(
            reason: "Keychain bytes at \(expectedAccount.suffix(40)) don't match the derived private key"
        )
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
            // Ask again rather than hold private bytes in the report.
            guard let keyType = key.row.keyTypeEnum else {
                failures.append((key.keyId, "unknown key type \(key.row.keyType)"))
                continue
            }
            let check: ManagedPlatformWallet.IdentityKeyWalletCheck
            do {
                check = try wallet.checkIdentityKey(
                    identityIndex: report.identityIndex,
                    keyId: key.keyId,
                    keyType: keyType,
                    purpose: key.purposeRaw,
                    securityLevel: key.securityLevelRaw,
                    publicKeyData: key.row.publicKeyData,
                    network: network
                )
            } catch {
                failures.append((key.keyId, "couldn't check the key: \(error.localizedDescription)"))
                continue
            }
            guard case let .matches(derivationPath, privateKeyData, publicKeyHashHex) = check else {
                failures.append((key.keyId, "the wallet no longer derives this key at its slot"))
                continue
            }
            let metadata = IdentityPrivateKeyMetadata(
                identityId: report.identityIdBase58,
                keyId: key.keyId,
                walletId: walletId.toHexString(),
                identityIndex: report.identityIndex,
                keyIndex: key.keyId,
                derivationPath: derivationPath,
                // The key's on-chain form, which `KeychainSigner` matches
                // on (the 20-byte hash for an ECDSA_HASH160 key). On a
                // match it is the wallet's key.
                publicKey: key.row.publicKeyData.toHexString(),
                publicKeyHash: publicKeyHashHex,
                keyType: keyType.rawValue,
                purpose: key.purposeRaw,
                securityLevel: key.securityLevelRaw
            )
            guard let pkid = KeychainManager.shared.storeIdentityPrivateKey(
                privateKeyData,
                derivationPath: derivationPath,
                metadata: metadata
            ) else {
                failures.append((key.keyId, "Keychain write at \(derivationPath) returned nil"))
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
                derivationPath: derivationPath
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
    @EnvironmentObject private var walletManager: PlatformWalletManager

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
                    // An error (keys not checked) is shown below instead.
                    if errorMessage == nil {
                        Section {
                            Text("No identities to check.")
                                .foregroundColor(.secondary)
                        }
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
        // Every verdict below assumes the stored recovery phrase is this
        // wallet's. Another wallet's phrase would report every key as
        // not the wallet's and offer to delete identities the wallet
        // owns, so check the binding first (Rust decides). A watch-only
        // wallet passes and each key then reports unchecked.
        do {
            try walletManager.verifySeedBinding(wallet)
        } catch {
            reports = []
            errorMessage = Self.seedBindingFailure("Keys weren't checked", error)
            return
        }
        reports = WalletKeyHealthChecker.runCheck(
            wallet: wallet,
            walletId: walletId,
            identities: identities,
            network: network
        )
    }

    private func rederive(_ report: WalletIdentityKeyHealthReport) {
        Task { @MainActor in
            // A re-derive on another wallet's recovery phrase would write
            // that wallet's keys, so nothing is written unless the stored
            // phrase is this wallet's.
            do {
                try walletManager.verifySeedBinding(wallet)
            } catch {
                actionMessage = nil
                errorMessage = Self.seedBindingFailure("Nothing was re-derived", error)
                return
            }
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

    /// The message for a recovery phrase that doesn't bind to this
    /// wallet (or couldn't be checked).
    private static func seedBindingFailure(_ outcome: String, _ error: Error) -> String {
        "\(outcome) because the recovery phrase stored for this wallet doesn't derive it: \(error.localizedDescription)"
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
