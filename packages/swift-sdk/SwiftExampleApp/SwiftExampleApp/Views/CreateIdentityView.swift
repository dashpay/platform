// CreateIdentityView.swift
// SwiftExampleApp
//
// Stepped UI for spinning up a new Dash Platform identity. The
// workflow chooses a funding source in two passes:
//
//   1. Source Wallet — one of the local PersistentWallet rows, or
//      "Create without Wallet" for the advanced path where the caller
//      supplies a raw asset-lock proof.
//   2. When a wallet is chosen: either a PersistentAccount on that
//      wallet (any type — Core pools and Platform Payment both work),
//      "Fund from unused Asset Lock", or "Shielded Balance" (Type 20
//      IdentityCreateFromShieldedPool — funds the identity directly
//      from the wallet's bound Orchard pool).
//
// Funding paths wired in `submit()`: Platform Payment
// (`registerIdentityFromAddresses`), Core / CoinJoin
// (`registerIdentityWithFunding`), unused asset-lock resume
// (`resumeIdentityWithAssetLock`), and Shielded Balance
// (`shieldedIdentityCreateFromPool`). The walletless raw-proof path
// is still a stub pending its FFI entry point.
//
// The Shielded Balance pass differs from the others: it spends a FIXED
// protocol denomination (not a free-form amount) from the bound
// shielded (Orchard) pool, takes tens of seconds for the Halo 2 proof,
// and so routes through the RegistrationCoordinator-hosted controller
// (survives sheet dismissal, visible under Pending Registrations).

import SwiftUI
import SwiftDashSDK
import SwiftData

/// Minimum surface of a tracked asset-lock row needed by the
/// cross-wallet anti-join behind the Identities-tab "Resumable
/// Registrations" section. Exists so the pure filter
/// `IdentitiesContentView.crossWalletResumableLocks(in:usedSlots:)`
/// can be unit-tested with lightweight structs instead of forcing
/// tests to spin up a SwiftData `ModelContainer` just to construct
/// `PersistentAssetLock` `@Model` instances. `PersistentAssetLock`
/// conforms automatically because it already exposes all three
/// properties on its public surface.
protocol AssetLockResumeRow {
    var walletId: Data { get }
    var statusRaw: Int { get }
    var identityIndexRaw: Int32 { get }
}

extension PersistentAssetLock: AssetLockResumeRow {}

struct CreateIdentityView: View {
    @Environment(\.dismiss) private var dismiss
    @Environment(\.modelContext) private var modelContext
    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var platformState: AppState

    /// Default number of Platform identity keys to register in this
    /// first-pass flow. The Rust-owned per-slot policy
    /// (`dash_sdk_derive_and_persist_identity_keys`) maps these to:
    /// key 0 AUTHENTICATION/MASTER, key 1 AUTHENTICATION/CRITICAL,
    /// key 2 AUTHENTICATION/HIGH, key 3 TRANSFER/CRITICAL. The
    /// TRANSFER/CRITICAL key is required to sign IdentityCreditTransfer
    /// (ID-04) and IdentityCreditWithdrawal (ID-10) — without it those
    /// broadcasts are rejected on-chain with "no transfer public key".
    /// Advanced override is intentionally not exposed here yet.
    private static let defaultKeyCount: UInt32 = 4

    /// Number of extra keys added when `addDashPayKeys` is on
    /// (encryption + decryption).
    private static let dashPayExtraKeyCount: UInt32 = 2

    /// Per-key cost in credits (from
    /// `IdentityCreateTransition::calculate_min_required_fee_v1`).
    /// Used to scale the asset-lock minimum when extra keys are
    /// requested.
    private static let identityKeyCreationCostCredits: UInt64 = 6_500_000

    /// DashPay system contract id (32 bytes), source-of-truth
    /// `packages/dashpay-contract/src/lib.rs::ID_BYTES`. Mirrored
    /// here so the contract-bounds payload for the optional
    /// encryption/decryption keys can be built without a contract
    /// fetch round-trip. Also matches the registry in
    /// `AddIdentityKeyView.systemContractsAllowingKeyBounds`.
    private static let dashpayContractId = Data([
        162, 161, 180, 172, 111, 239, 34, 234,
        42, 26, 104, 232, 18, 54, 68, 179,
        87, 135, 95, 107, 65, 44, 24, 16,
        146, 129, 193, 70, 231, 178, 113, 188,
    ])

    /// DashPay document type these keys are bound to — the only
    /// one in the contract that declares
    /// `requiresIdentityEncryptionBoundedKey`.
    private static let dashpayContactRequestDocumentType = "contactRequest"

    /// Credits per DASH (1e11) — the divisor used for Platform-side
    /// credit amounts. Duplicated from `PersistentPlatformAddress`
    /// docstring; kept here so the conversion logic stays local.
    private static let creditsPerDash: UInt64 = 100_000_000_000

    /// Versioned fixed exit denominations (in CREDITS) a Type-20
    /// IdentityCreateFromShieldedPool transition may spend from the
    /// shielded pool — 0.1 / 0.3 / 0.5 / 1.0 DASH. Source of truth:
    /// `shielded_identity_create_denominations` in
    /// `packages/rs-platform-version/src/version/drive_abci_versions/`
    /// `drive_abci_validation_versions/v8.rs`. There is no FFI getter
    /// for this set, so it's mirrored here — same precedent as
    /// `identityKeyCreationCostCredits` / `dashpayContractId`. If the
    /// versioned set changes on the Rust side, update this constant to
    /// match (a submitted denomination not in the on-chain set is
    /// rejected at validation).
    private static let shieldedIdentityCreateDenominations: [UInt64] = [
        10_000_000_000,   // 0.1 DASH
        30_000_000_000,   // 0.3 DASH
        50_000_000_000,   // 0.5 DASH
        100_000_000_000,  // 1.0 DASH
    ]

    /// Duffs per DASH (1e8) — Core-side scale, used by the Core-funded
    /// identity path.
    private static let duffsPerDash: UInt64 = 100_000_000

    /// Protocol minimum asset-lock funding for an identity create.
    /// Mirrors `IdentityCreateTransition::calculate_min_required_fee_v1`
    /// in `rs-platform-version`:
    ///   identity_create_base_cost (2_000_000 credits)
    ///   + asset_lock_base (200_000 duffs * 1000 credits/duff = 200_000_000)
    ///   + identity_key_in_creation_cost (6_500_000) * defaultKeyCount (4)
    ///   = 228_000_000 credits / 1000 = 228_000 duffs (0.002280 DASH).
    /// The v0 floor was 200_000 duffs but with key_in_creation_cost
    /// dynamic at v1, the per-key surcharge has to be added. Submitting
    /// below this gets rejected by Platform with
    /// `IdentityAssetLockTransactionOutPointNotEnoughBalance`. Use
    /// `minFundingDuffs(forKeyCount:)` for the active per-flow value
    /// when extra keys (e.g. the DashPay encryption/decryption pair)
    /// bump the per-key surcharge.
    private static let minIdentityFundingDuffsForDefaultKeys: UInt64 = 228_000

    /// Recompute the minimum-funding floor for `keyCount` total
    /// identity keys. Formula above; result is in duffs (credits ÷ 1000).
    private static func minFundingDuffs(forKeyCount keyCount: UInt32) -> UInt64 {
        let base: UInt64 = 2_000_000
        let assetLockBaseCredits: UInt64 = 200_000_000
        let perKey = identityKeyCreationCostCredits * UInt64(keyCount)
        return (base + assetLockBaseCredits + perKey) / 1_000
    }

    /// Whether the DashPay-keys toggle ACTUALLY applies to the
    /// active submission, mirroring `dashpayKeysSection`'s
    /// visibility predicate. The Toggle UI is hidden for resume /
    /// walletless flows (those paths don't pre-derive at custom
    /// indices today), so an `@State` `addDashPayKeys = true` value
    /// would still be true when the user is in those flows. Routing
    /// every call site through this gate keeps the funding-minimum
    /// calculation and the actual `makeDashpayKeyPair` invocation
    /// in lock-step with what the user sees.
    private var shouldRegisterDashPayKeys: Bool {
        guard case .wallet = walletSelection else { return false }
        guard fundingSelection != .unusedAssetLock else { return false }
        return addDashPayKeys
    }

    /// Total identity keys this submission will register, given the
    /// current DashPay-keys toggle. Drives the asset-lock minimum
    /// + the on-screen min-funding hint.
    private var plannedIdentityKeyCount: UInt32 {
        Self.defaultKeyCount + (shouldRegisterDashPayKeys ? Self.dashPayExtraKeyCount : 0)
    }

    /// Per-submission minimum funding in duffs — scales with
    /// `plannedIdentityKeyCount`.
    private var currentMinFundingDuffs: UInt64 {
        Self.minFundingDuffs(forKeyCount: plannedIdentityKeyCount)
    }

    /// Default funding amount pre-filled into the field for the Core
    /// path. Sits 12.5% above the protocol minimum to give headroom
    /// for the resulting identity's initial credit balance after the
    /// processing fee is deducted.
    private static let defaultCoreFundingDuffs: UInt64 = 250_000

    /// All locally-persisted wallets. Drives the Source Wallet
    /// picker along with the synthetic "no wallet" sentinel.
    @Query(sort: \PersistentWallet.createdAt) private var wallets: [PersistentWallet]

    /// All persisted accounts across wallets. Filtered per-selection
    /// inside `accountOptions(for:)` so switching wallets doesn't
    /// re-fire a SwiftData query.
    @Query private var allAccounts: [PersistentAccount]

    /// All persisted identities. Used by `unusedIdentityIndices(for:)`
    /// as the truth source for which identity-registration slots are
    /// already taken — the per-`PersistentCoreAddress.isUsed` flag can
    /// be stale for *discovered* identities (gap-limit scan finds an
    /// existing identity on Platform; the persister callback writes
    /// `PersistentIdentity` but doesn't currently flip the matching
    /// Core-address `isUsed` flag).
    @Query private var allIdentities: [PersistentIdentity]

    /// All tracked asset locks across wallets. Used here only to
    /// resolve `selectedAssetLockId` (the resume-flow init seed
    /// from Path B) back to a concrete row so the submit gate can
    /// hand its outpoint to the FFI. The cross-wallet anti-join
    /// that drives the Identities-tab Resumable Registrations
    /// section lives in `IdentitiesContentView` and runs against
    /// its own `@Query`.
    @Query(sort: [SortDescriptor(\PersistentAssetLock.updatedAt, order: .reverse)])
    private var allAssetLocks: [PersistentAssetLock]

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

    /// Register the optional pair of DashPay encryption/decryption
    /// keys (kid=3 ENCRYPTION + kid=4 DECRYPTION, both MEDIUM
    /// security, both ECDSA-secp256k1, both bound to the DashPay
    /// system contract's `contactRequest` document type) at
    /// identity-creation time. Default-on because the rest of the
    /// app's DashPay flow (contact requests, payments, profile)
    /// needs these keys present before it'll work — without them
    /// the user would have to come back to "Add Identity Key" and
    /// register them as a follow-up step. The two extra keys add
    /// 13_000 duffs to the asset-lock minimum.
    @State private var addDashPayKeys: Bool = true

    /// Raw asset-lock proof text, used only in the walletless path.
    /// Accepted encoding is base64 or lowercase hex — the submit
    /// logic (future) will detect + decode.
    @State private var walletlessProof: String = ""

    /// Persistent-id of the asset-lock row the user picked under
    /// `.unusedAssetLock`. Nil until they tap one. Identifying via
    /// `PersistentIdentifier` (not `outPointHex`) keeps the picker
    /// resilient if the underlying row updates its status mid-pick.
    @State private var selectedAssetLockId: PersistentIdentifier? = nil

    /// Amount (in DASH) to fund the new identity with. Populated
    /// automatically from the selected account's balance; the user
    /// can lower it but not exceed the available balance.
    @State private var amountDash: String = ""

    /// Chosen fixed exit denomination (in credits) for the
    /// `.shieldedBalance` funding path. `nil` until the user picks one
    /// in the denomination picker. Reset on any wallet / funding-source
    /// change (same reset paths as `amountDash`). Only ever one of
    /// `Self.shieldedIdentityCreateDenominations`.
    @State private var selectedDenomination: UInt64? = nil

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

    /// Active registration controller for the Core-funded path.
    /// Stored only so `submitCoreFunded` has a local reference
    /// after spawning it; the canonical lifetime owner is
    /// `walletManager.registrationCoordinator`, which keeps the
    /// controller available to the "Pending Registrations" row
    /// on the Identities tab after this sheet dismisses.
    @State private var activeController: IdentityRegistrationController? = nil

    /// Optional pre-pick wired up by the Identities-tab "Resumable
    /// Registrations" section. When non-nil, the four selection
    /// `@State`s below are seeded in `init` so the form opens
    /// already-configured for the resume path — same final state
    /// the user would land on by hand-picking the wallet, choosing
    /// "Fund from unused Asset Lock", and tapping the lock in the
    /// picker. Seeding in `init` (via `_state = State(initialValue:
    /// …)`) — rather than mutating in `.onAppear` / `.task` —
    /// matters because the form's `.onChange(of: walletSelection)`
    /// handler resets `fundingSelection` and `identityIndex` on any
    /// runtime mutation; treating the preselect as the *initial*
    /// state instead never trips that reset cascade.
    /// Default `nil` keeps the original "Identities tab → + →
    /// Create Identity" entry point unchanged.
    let preselectedAssetLock: PersistentAssetLock?

    init(preselectedAssetLock: PersistentAssetLock? = nil) {
        self.preselectedAssetLock = preselectedAssetLock
        if let lock = preselectedAssetLock {
            // Explicit `Optional(…)` wraps so the `@State` initial
            // values match the underlying Optional types — Swift
            // can't infer a `T?` from a bare `.wallet(...)` literal
            // here because the inferred type lands on `T`.
            _walletSelection = State(
                initialValue: Optional(WalletSelection.wallet(id: lock.walletId))
            )
            _fundingSelection = State(
                initialValue: Optional(FundingSelection.unusedAssetLock)
            )
            _selectedAssetLockId = State(
                initialValue: Optional(lock.persistentModelID)
            )
            _identityIndex = State(
                initialValue: Optional(UInt32(bitPattern: lock.identityIndexRaw))
            )
        }
    }

    var body: some View {
        NavigationStack {
            Form {
                if let controller = activeController {
                    // While registration is in flight (or has just
                    // reached a terminal phase), swap the entire
                    // form for the 5-step progress section + the
                    // terminal banner (success "Done" / failure
                    // error). The form's input sections would be
                    // noise at this point — the user has already
                    // committed; the controller lives on
                    // `walletManager.registrationCoordinator` and
                    // continues to run if this sheet is dismissed.
                    RegistrationProgressSection(controller: controller)
                    terminalSection(for: controller)
                } else if let lock = preselectedAssetLock {
                    // Path B (Resumable Registrations → Resume):
                    // wallet + lock + slot are all pinned by the
                    // tapped row, so the form collapses to a
                    // read-only summary and a single submit button.
                    // The picker UI is intentionally absent — the
                    // user already chose the lock on the Identities
                    // tab; re-prompting would be noise.
                    resumeSummarySection(for: lock)
                    if canSubmit {
                        submitSection
                    }
                } else {
                    sourceWalletSection
                    fundingSection
                    amountSection
                    identityIndexSection
                    dashpayKeysSection
                    if canSubmit {
                        submitSection
                    }
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

    /// Inline terminal banner that appears under the progress
    /// section once the controller has reached `.completed` or
    /// `.failed`. On success, shows the new identity id + a
    /// "Done" button that dismisses the sheet (the new identity
    /// appears in the Identities tab's `@Query` automatically).
    /// On failure, shows the error message inline.
    @ViewBuilder
    private func terminalSection(for controller: IdentityRegistrationController) -> some View {
        switch controller.phase {
        case .completed(let identityId):
            Section {
                VStack(alignment: .leading, spacing: 8) {
                    Label("Identity created", systemImage: "checkmark.seal.fill")
                        .foregroundColor(.green)
                        .font(.headline)
                    Text(identityId.toBase58String())
                        .font(.system(.caption, design: .monospaced))
                        .foregroundColor(.secondary)
                        .textSelection(.enabled)
                    Button {
                        // User acknowledged success — drop the
                        // controller from the coordinator's map so
                        // the "Pending Registrations" row on the
                        // Identities tab clears immediately
                        // (otherwise it lingers ~30 s until the
                        // post-completion retention sweep runs).
                        walletManager.registrationCoordinator.dismiss(
                            walletId: controller.walletId,
                            identityIndex: controller.identityIndex
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
                    Label("Registration failed", systemImage: "xmark.octagon.fill")
                        .foregroundColor(.red)
                        .font(.headline)
                    Text(message)
                        .font(.callout)
                        .foregroundColor(.primary)
                        .textSelection(.enabled)
                }
            }
        case .unconfirmed(let identityId, let message):
            Section {
                VStack(alignment: .leading, spacing: 8) {
                    Label(
                        "Broadcast succeeded — confirmation pending",
                        systemImage: "exclamationmark.triangle.fill"
                    )
                    .foregroundColor(.orange)
                    .font(.headline)
                    Text(identityId.toBase58String())
                        .font(.system(.caption, design: .monospaced))
                        .foregroundColor(.secondary)
                        .textSelection(.enabled)
                    Text(message)
                        .font(.callout)
                        .foregroundColor(.primary)
                        .textSelection(.enabled)
                    Text(
                        "The transition was broadcast and accepted, but its "
                        + "execution-result proof couldn't be confirmed. The "
                        + "identity will appear in the Identities tab after the "
                        + "next sync. Do NOT re-register these keys — the slot "
                        + "is held to prevent burning funds."
                    )
                    .font(.caption)
                    .foregroundColor(.secondary)
                    // Plain sheet dismiss only — leave the controller on the
                    // coordinator so the entry stays in Pending Registrations
                    // until the identity row appears via sync and the user
                    // dismisses it there.
                    Button {
                        dismiss()
                    } label: {
                        Text("Close")
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

    // MARK: - Sections

    private var sourceWalletSection: some View {
        Section {
            Picker("Source Wallet", selection: $walletSelection) {
                Text("Select…")
                    .tag(Optional<WalletSelection>.none)
                    .accessibilityIdentifier("createIdentity.sourceWallet.none")
                ForEach(wallets) { wallet in
                    Text(walletLabel(for: wallet))
                        .tag(Optional(WalletSelection.wallet(id: wallet.walletId)))
                        .accessibilityIdentifier("createIdentity.sourceWallet.wallet.\(wallet.walletId.toHexString())")
                }
                Divider()
                Text("Create without Wallet")
                    .tag(Optional(WalletSelection.walletless))
                    .accessibilityIdentifier("createIdentity.sourceWallet.walletless")
            }
            .accessibleFormPicker("createIdentity.sourceWalletPicker")
            .onChange(of: walletSelection) { _, newValue in
                // Reset downstream selection whenever the wallet
                // changes so a stale account / proof / denomination
                // can't leak through.
                fundingSelection = nil
                walletlessProof = ""
                selectedDenomination = nil
                // Default the identity-registration index to one
                // past the highest already-used slot on the newly-
                // selected wallet (identities aren't gap-limited; we
                // can derive any index N from the mnemonic on demand).
                // Walletless / no-selection branches clear it.
                if case .wallet(let walletId) = newValue {
                    identityIndex = nextUnusedIdentityIndex(for: walletId)
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

    /// Read-only summary shown in place of the source-wallet /
    /// funding-source / amount / identity-index sections when the
    /// caller pre-pinned an asset lock via `init(preselectedAssetLock:)`
    /// (Path B — Identities tab → "Resumable Registrations" → Resume).
    /// The choices are already made; this view confirms them so the
    /// user can sanity-check before tapping Create Identity.
    @ViewBuilder
    private func resumeSummarySection(for lock: PersistentAssetLock) -> some View {
        let walletLabel = wallets
            .first(where: { $0.walletId == lock.walletId })?
            .label ?? "Wallet"
        Section {
            summaryRow("Wallet", value: walletLabel)
            summaryRow(
                "Asset Lock",
                value: lock.shortOutPointDisplay,
                monospaced: true
            )
            summaryRow(
                "Amount",
                value: Self.formatDash(
                    raw: UInt64(bitPattern: Int64(lock.amountDuffs)),
                    divisor: Double(Self.duffsPerDash)
                )
            )
            summaryRow(
                "Status",
                value: lock.statusLabel
            )
            summaryRow(
                "Identity Slot",
                value: "#\(UInt32(bitPattern: lock.identityIndexRaw))"
            )
        } header: {
            Text("Resuming Registration")
        } footer: {
            Text(
                "Tap Create Identity to complete this in-flight "
                + "registration. The wallet, slot, and asset lock are "
                + "fixed by the original registration."
            )
        }
    }

    /// One key/value row used by `resumeSummarySection`. Plain
    /// `HStack` rather than `LabeledContent` because the value should
    /// flow with the row's content style (so the txid prefix can be
    /// monospaced) and `LabeledContent` resists styling overrides
    /// on its value slot.
    @ViewBuilder
    private func summaryRow(
        _ label: String,
        value: String,
        monospaced: Bool = false
    ) -> some View {
        HStack {
            Text(label)
            Spacer()
            let valueText = Text(value)
                .foregroundColor(.secondary)
            if monospaced {
                valueText.font(.system(.body, design: .monospaced))
            } else {
                valueText
            }
        }
    }


    @ViewBuilder
    private func walletAccountSection(for walletId: Data) -> some View {
        let options = accountOptions(for: walletId)
        // Whether to surface the shielded-pool funding row. Computed
        // once so the picker body and the footer text stay in lockstep.
        let showShielded = shieldedOptionAvailable(for: walletId)
        Section {
            Picker("Funding Source", selection: $fundingSelection) {
                Text("Select…")
                    .tag(Optional<FundingSelection>.none)
                    .accessibilityIdentifier("createIdentity.fundingSource.none")
                ForEach(options) { option in
                    Text("\(option.label) — \(option.balanceText)")
                        .tag(Optional(FundingSelection.account(id: option.persistentId)))
                        .accessibilityIdentifier("createIdentity.fundingSource.account.\(option.accountType).\(option.accountIndex)")
                }
                if showShielded {
                    let shieldedText = Self.formatDash(
                        raw: shieldedPoolBalance(for: walletId),
                        divisor: Double(Self.creditsPerDash)
                    )
                    Text("Shielded Balance — \(shieldedText)")
                        .tag(Optional(FundingSelection.shieldedBalance))
                        .accessibilityIdentifier("createIdentity.fundingSource.shielded")
                }
            }
            .accessibleFormPicker("createIdentity.fundingSourcePicker")
            .onChange(of: fundingSelection) { _, newValue in
                // Pre-fill the amount with the full available balance
                // of the selected Platform Payment account so the
                // happy path is one tap. Users can dial it down. The
                // shielded path uses a fixed denomination, not this
                // free-form field, so clear both the amount and the
                // denomination on every funding-source change.
                amountDash = defaultAmountString(for: newValue)
                selectedDenomination = nil
            }
        } header: {
            Text("Funding Source")
        } footer: {
            Text(Self.fundingSourceFooterText(showShielded: showShielded))
        }
    }

    /// Footer copy for the funding-source section. Built out-of-line —
    /// inlining the concatenation chain (with a conditional in the
    /// middle) blows the SwiftUI type-checker's time budget.
    private static func fundingSourceFooterText(showShielded: Bool) -> String {
        var text = "Any account on the selected wallet with a balance can fund "
            + "the identity — Core or Platform Payment. Empty accounts are hidden. "
        if showShielded {
            text += "Shielded Balance funds the identity directly from this "
                + "wallet's shielded (Orchard) pool by spending a fixed denomination. "
        }
        text += "To resume a prior in-flight registration, "
            + "use the Resumable Registrations section on the Identities tab."
        return text
    }

    /// Amount (in DASH) to fund the new identity with. Shown for
    /// Platform Payment and Core / CoinJoin funding sources. For the
    /// `.shieldedBalance` path the free-form amount is replaced by a
    /// fixed-denomination picker (`shieldedDenominationSection`).
    @ViewBuilder
    private var amountSection: some View {
        if fundingSelection == .shieldedBalance {
            shieldedDenominationSection
        } else if let account = selectedPlatformAccount {
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
        } else if let account = selectedCoreAccount {
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
                    raw: coreAccountBalanceDuffs(account),
                    divisor: Double(Self.duffsPerDash)
                )
                let minimum = Self.formatDash(
                    raw: currentMinFundingDuffs,
                    divisor: Double(Self.duffsPerDash)
                )
                Text("Available: \(available). Minimum: \(minimum). Rust builds an asset-lock transaction from your Core UTXOs and the locked funds become the new identity's initial credit balance.")
            }
        }
    }

    /// Fixed-denomination picker for the `.shieldedBalance` funding
    /// path. The Type-20 transition spends one of the versioned
    /// denominations (`Self.shieldedIdentityCreateDenominations`), not
    /// a free-form amount — so this replaces the amount field. Only
    /// denominations the selected wallet's shielded pool can actually
    /// cover (`<= shieldedPoolBalance(for:)`) are offered.
    @ViewBuilder
    private var shieldedDenominationSection: some View {
        // Per-wallet pool balance for the selected source wallet. This
        // section only renders when `fundingSelection == .shieldedBalance`,
        // which requires a wallet selection, so `selectedWalletId` is
        // non-nil here; the `?? 0` is a defensive fallback that yields an
        // empty `affordable` list rather than a crash.
        let poolBalance = selectedWalletId.map { shieldedPoolBalance(for: $0) } ?? 0
        // Denominations the pool can cover. Computed off the live
        // per-wallet pool balance so the list shrinks as the pool
        // drains (e.g. after a prior shielded spend this session).
        let affordable = Self.shieldedIdentityCreateDenominations
            .filter { $0 <= poolBalance }
        Section {
            if affordable.isEmpty {
                // Defensive: the option is gated on `shieldedBalance > 0`,
                // but the smallest denomination (0.1 DASH) can still
                // exceed a small positive balance. Surface why no
                // denomination is selectable rather than showing an
                // empty picker.
                Text(
                    "The shielded balance is below the smallest "
                    + "denomination (\(Self.formatDash(raw: Self.shieldedIdentityCreateDenominations.first ?? 0, divisor: Double(Self.creditsPerDash)))). "
                    + "Shield more funds first."
                )
                .font(.caption)
                .foregroundColor(.secondary)
            } else {
                Picker("Denomination", selection: $selectedDenomination) {
                    Text("Select…")
                        .tag(Optional<UInt64>.none)
                        .accessibilityIdentifier("createIdentity.denomination.none")
                    ForEach(affordable, id: \.self) { denom in
                        Text(Self.formatDash(
                            raw: denom,
                            divisor: Double(Self.creditsPerDash)
                        ))
                        .tag(Optional(denom))
                        .accessibilityIdentifier("createIdentity.denomination.\(denom)")
                    }
                }
                .accessibleFormPicker("createIdentity.denominationPicker")
                .disabled(isCreating)
            }
        } header: {
            Text("Denomination")
        } footer: {
            let available = Self.formatDash(
                raw: poolBalance,
                divisor: Double(Self.creditsPerDash)
            )
            Text(
                "Available shielded: \(available). The whole denomination "
                + "leaves the pool; the metered fee is taken FROM it (the new "
                + "identity starts at denomination − fee), and any excess "
                + "spent value returns to the pool as change. The Halo 2 proof "
                + "takes tens of seconds — registration continues under "
                + "Pending Registrations if you dismiss this sheet."
            )
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
            // Resume path: the lock fixes the slot. Render a read-
            // only summary so the user can confirm which slot the
            // tracked lock is anchored to, but the stepper is
            // disabled.
            if fundingSelection == .unusedAssetLock {
                Section {
                    HStack {
                        Text("Identity Registration Index")
                        Spacer()
                        if let identityIndex = identityIndex {
                            Text("#\(identityIndex)")
                                .foregroundColor(.secondary)
                                .monospacedDigit()
                        } else {
                            Text("—")
                                .foregroundColor(.secondary)
                        }
                    }
                } header: {
                    Text("Identity Registration Index")
                } footer: {
                    Text(
                        "Resuming uses the asset lock's original slot — "
                        + "the index can't be overridden on this path."
                    )
                }
            } else {
                let used = usedIdentityIndices(for: walletId)
                let collision = identityIndex.map { used.contains($0) } ?? false
                Section {
                    Stepper(value: Binding(
                        get: { identityIndex ?? 0 },
                        set: { identityIndex = $0 }
                    ), in: 0...UInt32.max) {
                        HStack {
                            Text("Identity Registration Index")
                            Spacer()
                            Text("#\(identityIndex ?? 0)")
                                .foregroundColor(collision ? .red : .secondary)
                                .monospacedDigit()
                        }
                    }
                    if collision {
                        Text(
                            "Index #\(identityIndex ?? 0) is already taken on "
                            + "this wallet. Pick a different index."
                        )
                        .font(.caption)
                        .foregroundColor(.red)
                    }
                } header: {
                    Text("Identity Registration Index")
                } footer: {
                    Text(
                        "The DIP-9 identity-registration slot the new identity "
                        + "will consume "
                        + "(`m/9'/coin'/5'/0'/0'/N'/0'`). Defaults to one past "
                        + "the highest index already used on this wallet; "
                        + "override to pick any other unused index."
                    )
                }
            }
        }
    }

    /// Per-key cost note for the DashPay-keys footer. The "+N duffs"
    /// funding-minimum phrasing only applies to the asset-lock-style
    /// paths (Core / Platform Payment), where the per-key surcharge
    /// bumps the funding floor. The shielded path spends a FIXED
    /// denomination and meters the fee FROM it, so there's no duff
    /// minimum to add to — surface the per-key cost neutrally instead
    /// of as a misleading "asset-lock minimum" bump.
    private var dashpayKeysCostNote: String {
        if fundingSelection == .shieldedBalance {
            return "The two extra keys add their per-key creation cost to the metered fee taken from the chosen denomination."
        }
        let extraDuffs = currentMinFundingDuffs
            - Self.minFundingDuffs(forKeyCount: Self.defaultKeyCount)
        return "Adds \(extraDuffs) duffs to the asset-lock minimum."
    }

    /// Toggle for the optional DashPay encryption/decryption key
    /// pair. Default-on because DashPay is a first-class feature in
    /// this app and registering the keys after-the-fact requires
    /// another state transition (Add Identity Key).
    @ViewBuilder
    private var dashpayKeysSection: some View {
        // Only meaningful for wallet-backed paths — walletless /
        // resume paths don't pre-derive at custom indices today.
        if case .wallet = walletSelection,
           fundingSelection != .unusedAssetLock {
            Section {
                Toggle("Register DashPay keys", isOn: $addDashPayKeys)
            } header: {
                Text("DashPay Support")
            } footer: {
                Text(
                    addDashPayKeys
                    ? "Registers 2 additional keys at registration — one Encryption + one Decryption (both MEDIUM security, ECDSA secp256k1, bound to the DashPay system contract's `contactRequest` document type). Required for sending and accepting friend requests, sending payments to contacts, and DashPay profile flows. \(dashpayKeysCostNote)"
                    : "Identity will register with the default 3 authentication keys only. You can add DashPay encryption/decryption keys later via Add Identity Key on the identity detail screen — but flows like Add Friend won't work until those keys exist."
                )
            }
        }
    }

    private var submitSection: some View {
        // The active progress / success / error UI lives on the
        // pushed `RegistrationProgressView` destination, NOT
        // inline. Once `submit()` spawns a controller and flips
        // `showProgressDestination` to true, the user is navigated
        // there and this form steps off-screen.
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

    // `successSection` was removed when the dedicated
    // `RegistrationProgressView` destination took ownership of the
    // success / failure terminal UI (including the "View Identity"
    // navigation). The form here is input-only; submission pushes
    // to the progress destination, which renders the post-
    // registration state.

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
        case (.wallet(let walletId), .some):
            guard let identityIndex = identityIndex else { return false }
            // The resume path is bound to the lock's original
            // identity-index slot. Path B's init seeds
            // `identityIndex` to the lock's `identityIndexRaw`,
            // so the collision check below would always trip
            // (the lock's slot has been "taken" by the lock
            // itself). Resume specifically needs the slot to NOT
            // have a `PersistentIdentity` — the cross-wallet
            // anti-join in `IdentitiesContentView` already filtered
            // for that on the row the user tapped Resume on, so
            // we trust the seed.
            if fundingSelection == .unusedAssetLock {
                // Resolve the id to a live row before enabling
                // submit — the row could have been deleted between
                // Path B's init-seed and now (e.g. another flow
                // consumed the same outpoint, or SwiftData pruning
                // collapsed it). Submitting with a stale id would
                // fall into the dispatch's "not yet supported"
                // branch and show a confusing error.
                return selectedAssetLock != nil
            }
            // Block submit on collision with an existing identity's
            // registration index. The picker shows a red collision
            // hint, but the button stays disabled regardless so the
            // user can't punch through.
            if usedIdentityIndices(for: walletId).contains(identityIndex) {
                return false
            }
            if let account = selectedPlatformAccount {
                guard let credits = parsedAmountCredits else { return false }
                return credits > 0 && credits <= accountBalance(account)
            }
            if let account = selectedCoreAccount {
                guard let duffs = parsedAmountDuffs else { return false }
                let available = coreAccountBalanceDuffs(account)
                return duffs >= currentMinFundingDuffs && duffs <= available
            }
            if fundingSelection == .shieldedBalance {
                // Re-check availability (the bound wallet / balance could
                // have changed since the option was rendered), require a
                // chosen denomination, and that the pool still covers it.
                // The slot-collision check above already applies (the
                // shielded path isn't `.unusedAssetLock`).
                guard shieldedOptionAvailable(for: walletId) else { return false }
                guard let denomination = selectedDenomination else { return false }
                return denomination <= shieldedPoolBalance(for: walletId)
            }
            return false
        default:
            return false
        }
    }

    // MARK: - Submit

    /// Dispatches identity registration to the correct funding path.
    /// Platform-Payment funding uses `registerIdentityFromAddresses`;
    /// Core / CoinJoin funding uses `registerIdentityWithFunding`
    /// (asset-lock proof built Rust-side from wallet UTXOs); unused
    /// asset-lock resume uses `resumeIdentityWithAssetLock`; and the
    /// Shielded Balance path uses `shieldedIdentityCreateFromPool`
    /// (Type-20, spends a fixed denomination from the Orchard pool).
    /// The walletless raw-proof branch stays disabled via `canSubmit`
    /// until its FFI entry point lands.
    private func submit() {
        guard
            let identityIndex = identityIndex,
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

        // Identity registration goes through the Swift `KeychainSigner`
        // now: every state-transition signature crosses the FFI via
        // the signer's vtable, so the BIP-39 mnemonic stays in
        // Keychain (not handed across the FFI).
        //
        // We pre-persist ONLY the identity-key private keys, looked
        // up by pubkey bytes (`key_type < 5` dispatch). Identity keys
        // ARE meant to live in the Keychain as primary storage —
        // they are not seed-derived afresh per request, but read
        // back from the Keychain by the trampoline.
        //
        // Platform-address private keys (the `key_type == 0xFF`
        // dispatch) are NOT pre-persisted. The trampoline derives
        // them on demand per signing call from `(mnemonic, path)`
        // via `dash_sdk_sign_with_mnemonic_and_path` and zeroes the
        // buffers immediately. The mnemonic stays in Keychain; the
        // derivation path lives on `PersistentPlatformAddress`.
        let signer = KeychainSigner(modelContainer: modelContext.container)
        var identityPubkeys: [ManagedPlatformWallet.IdentityPubkey]
        do {
            // Single-FFI derive + persist. The Rust side owns the
            // per-key MASTER-vs-HIGH policy and ships back the
            // ready-to-use `IdentityPubkey` rows; the Swift caller
            // does not recreate the policy. See
            // swift-sdk/CLAUDE.md "no mnemonic round-tripping" —
            // both the mnemonic fetch and the per-key Keychain
            // write are now Rust → Swift callbacks orchestrated by
            // the Rust loop, not Swift-side glue.
            identityPubkeys = try managedWallet.prePersistIdentityKeysForRegistration(
                identityIndex: identityIndex,
                keyCount: Self.defaultKeyCount,
                network: platformState.currentNetwork
            )
        } catch {
            submitError = .init(
                message: "Could not pre-derive identity keys: \(error.localizedDescription)"
            )
            return
        }

        // Optional DashPay key pair — Encryption (kid=N) + Decryption
        // (kid=N+1) bound to DashPay's `contactRequest` document
        // type, MEDIUM security level, ECDSA-secp256k1. Registered
        // alongside the default auth keys when the user has
        // `addDashPayKeys` on (default) so the post-registration
        // DashPay flow (send contact request, accept, send payment)
        // is usable without a follow-up Add Identity Key round.
        //
        // The Rust pre-persist call above only knows about the
        // auth-key policy (master + HIGH). For the encryption /
        // decryption pair we re-use the same per-slot derivation
        // FFI the manual Add Identity Key flow uses, build the
        // matching `IdentityPubkey` rows here with explicit
        // `contractBounds`, and store the private bytes under the
        // walletId-namespaced keychain account so the trampoline
        // can find them when DashPay flows ask the new keys to
        // sign.
        if shouldRegisterDashPayKeys {
            do {
                identityPubkeys.append(contentsOf: try makeDashpayKeyPair(
                    managedWallet: managedWallet,
                    walletId: walletId,
                    identityIndex: identityIndex,
                    firstKeyId: Self.defaultKeyCount,
                    network: platformState.currentNetwork
                ))
            } catch {
                submitError = .init(
                    message: "Could not derive DashPay keys: \(error.localizedDescription)"
                )
                return
            }
        }

        let network: Network = platformState.currentNetwork

        // Dispatch by funding source.
        if let account = selectedPlatformAccount {
            submitPlatformPayment(
                account: account,
                walletId: walletId,
                identityIndex: identityIndex,
                identityPubkeys: identityPubkeys,
                signer: signer,
                managedWallet: managedWallet,
                network: network
            )
        } else if let account = selectedCoreAccount {
            submitCoreFunded(
                account: account,
                walletId: walletId,
                identityIndex: identityIndex,
                identityPubkeys: identityPubkeys,
                signer: signer,
                managedWallet: managedWallet,
                network: network
            )
        } else if fundingSelection == .unusedAssetLock,
                  let lock = selectedAssetLock {
            submitResumed(
                lock: lock,
                walletId: walletId,
                identityIndex: identityIndex,
                identityPubkeys: identityPubkeys,
                signer: signer,
                managedWallet: managedWallet,
                network: network
            )
        } else if fundingSelection == .shieldedBalance {
            submitShieldedFunded(
                walletId: walletId,
                identityIndex: identityIndex,
                identityPubkeys: identityPubkeys,
                signer: signer,
                managedWallet: managedWallet,
                network: network
            )
        } else {
            submitError = .init(message: "Selected funding source is not yet supported.")
        }
    }

    /// Resume registration against an existing tracked asset lock.
    /// Same shape as `submitCoreFunded` — the only differences are
    /// (a) the body closure invokes
    /// `resumeIdentityWithAssetLock(...)` instead of
    /// `registerIdentityWithFunding(amountDuffs:...)`, and (b) the
    /// outpoint is decoded from the lock row's display-order
    /// `outPointHex` to the raw 32-byte txid + u32 vout the FFI
    /// wants.
    private func submitResumed(
        lock: PersistentAssetLock,
        walletId: Data,
        identityIndex: UInt32,
        identityPubkeys: [ManagedPlatformWallet.IdentityPubkey],
        signer: KeychainSigner,
        managedWallet: ManagedPlatformWallet,
        network: Network
    ) {
        guard let parts = Self.parseOutPointHex(lock.outPointHex) else {
            submitError = .init(
                message: "Tracked asset lock has a malformed outpoint and can't be resumed: \(lock.outPointHex)"
            )
            return
        }
        let (txidRaw, vout) = parts

        isCreating = true

        let coordinator = walletManager.registrationCoordinator
        let controller = coordinator.startRegistration(
            walletId: walletId,
            identityIndex: identityIndex,
            body: {
                let (identityId, _) = try await managedWallet.resumeIdentityWithAssetLock(
                    outPointTxid: txidRaw,
                    outPointVout: vout,
                    identityIndex: identityIndex,
                    identityPubkeys: identityPubkeys,
                    signer: signer
                )
                return identityId
            }
        )

        self.activeController = controller
        observeController(
            controller,
            walletId: walletId,
            identityIndex: identityIndex,
            network: network
        )
    }

    /// Shielded-pool funded registration (Type-20
    /// IdentityCreateFromShieldedPool). Spends a fixed denomination from
    /// the wallet's bound Orchard pool to fund a brand-new identity.
    ///
    /// Same coordinator-hosted shape as `submitCoreFunded` /
    /// `submitResumed`: the body closure runs
    /// `walletManager.shieldedIdentityCreateFromPool` (returns the new
    /// identity id directly) and `observeController` drives the shared
    /// `persistCreatedIdentity` + `markIdentitySlotUsed` side-effects.
    /// The coordinator host matters MORE here than the other paths — the
    /// Halo 2 proof takes tens of seconds, so a controller that survives
    /// sheet dismissal (and shows under Pending Registrations) is the
    /// right shape.
    ///
    /// Uses ZIP-32 account 0 (the app's shielded flows are account-0
    /// default). The Rust wrapper registers the proof-verified identity
    /// at `identityIndex` on success, so the `PersistentIdentity` row is
    /// created by the persister callbacks before `persistCreatedIdentity`
    /// patches its UI-only fields — same as the address-funded path.
    private func submitShieldedFunded(
        walletId: Data,
        identityIndex: UInt32,
        identityPubkeys: [ManagedPlatformWallet.IdentityPubkey],
        signer: KeychainSigner,
        managedWallet: ManagedPlatformWallet,
        network: Network
    ) {
        guard let denomination = selectedDenomination else {
            submitError = .init(message: "Pick a shielded denomination first.")
            return
        }
        // The fallback failure address is REQUIRED for Type-20. Visibility
        // of the option is already gated on this being non-nil
        // (`shieldedOptionAvailable`), but re-resolve + guard here so the
        // submit path is force-unwrap-free if state shifted underneath us.
        guard let fallbackAddressBytes = shieldedFallbackAddressBytes(for: walletId) else {
            submitError = .init(
                message: "No Platform Payment address is available on this wallet to use as the required shielded creation-failure fallback. Generate one first."
            )
            return
        }

        isCreating = true

        let coordinator = walletManager.registrationCoordinator
        // Captured locally so the escaping body holds its own reference
        // rather than re-reading the view's `walletManager` property from
        // another isolation domain (mirrors how the other submit paths
        // capture `managedWallet`).
        let manager = walletManager
        let controller = coordinator.startRegistration(
            walletId: walletId,
            identityIndex: identityIndex,
            fundingKind: .shieldedPool,
            body: {
                // `shieldedIdentityCreateFromPool` lives on the manager
                // (it's wallet-id-routed, unlike the per-wallet
                // `ManagedPlatformWallet` registration methods) and
                // returns the new identity id directly. Account 0 = the
                // app's shielded default.
                let identityId = try await manager.shieldedIdentityCreateFromPool(
                    walletId: walletId,
                    account: 0,
                    identityIndex: identityIndex,
                    identityPubkeys: identityPubkeys,
                    denomination: denomination,
                    sendToAddressOnCreationFailure: fallbackAddressBytes,
                    identitySigner: signer
                )
                return identityId
            }
        )

        self.activeController = controller
        observeController(
            controller,
            walletId: walletId,
            identityIndex: identityIndex,
            network: network
        )
    }

    /// Platform-Payment funded registration. Spends credits from
    /// `PersistentPlatformAddress` rows on the selected account.
    private func submitPlatformPayment(
        account: PersistentAccount,
        walletId: Data,
        identityIndex: UInt32,
        identityPubkeys: [ManagedPlatformWallet.IdentityPubkey],
        signer: KeychainSigner,
        managedWallet: ManagedPlatformWallet,
        network: Network
    ) {
        guard let targetCredits = parsedAmountCredits, targetCredits > 0 else {
            submitError = .init(message: "Enter a positive amount.")
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

        Task {
            do {
                let created = try await managedWallet.registerIdentityFromAddresses(
                    inputs: inputs,
                    output: nil,
                    identityIndex: identityIndex,
                    identityPubkeys: identityPubkeys,
                    identitySigner: signer,
                    addressSigner: signer
                )

                try await MainActor.run {
                    try persistCreatedIdentity(
                        identityId: created.identityId,
                        network: network
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

    /// Core BIP44-standard funded registration. Rust builds an
    /// asset-lock transaction from the *selected* BIP44 standard
    /// account's UTXOs (the picker is restricted to BIP44 standard
    /// only — see `isFundingAccount` for the predicate), broadcasts
    /// it, waits for the instant-send lock, and submits the
    /// IdentityCreate state transition.
    ///
    /// Iter 3 swap (the previous iter-1 path was a single in-flight
    /// spinner inside this view): the FFI call moves to a
    /// `RegistrationCoordinator`-hosted `IdentityRegistrationController`
    /// hung off the `PlatformWalletManager`. The view binds the
    /// "Registering…" indicator to the controller's `phase` so
    /// dismissing the sheet doesn't abort the registration —
    /// `RegistrationProgressView` can be opened from the home /
    /// identities tab to follow the same controller through.
    private func submitCoreFunded(
        account: PersistentAccount,
        walletId: Data,
        identityIndex: UInt32,
        identityPubkeys: [ManagedPlatformWallet.IdentityPubkey],
        signer: KeychainSigner,
        managedWallet: ManagedPlatformWallet,
        network: Network
    ) {
        guard let amountDuffs = parsedAmountDuffs, amountDuffs > 0 else {
            submitError = .init(message: "Enter a positive amount.")
            return
        }

        // Pinned at submit time so the async body captures a stable
        // index rather than re-reading the (mutable) `PersistentAccount`
        // row from another isolation domain.
        let accountIndex = account.accountIndex

        isCreating = true

        // Reuse an existing controller for this slot if one is in
        // flight (the coordinator's single-flight gate makes the
        // re-submit a no-op at the controller layer), otherwise
        // spawn a fresh one. The persistence side-effects
        // (`persistCreatedIdentity`, `markIdentitySlotUsed`) live
        // here in the view so we still have access to
        // `modelContext` / `platformState`; the controller's body
        // is responsible only for the FFI call + the success
        // identifier.
        let coordinator = walletManager.registrationCoordinator
        let controller = coordinator.startRegistration(
            walletId: walletId,
            identityIndex: identityIndex,
            body: {
                let (identityId, _) = try await managedWallet.registerIdentityWithFunding(
                    amountDuffs: amountDuffs,
                    accountIndex: accountIndex,
                    identityIndex: identityIndex,
                    identityPubkeys: identityPubkeys,
                    signer: signer
                )
                return identityId
            }
        )

        // Stash the controller; setting it flips the body to the
        // inline progress + terminal section in place of the
        // form. The controller's canonical lifetime owner is
        // `walletManager.registrationCoordinator` — if the user
        // dismisses the sheet mid-flight, the same controller is
        // reachable via the "Pending Registrations" row on the
        // Identities tab.
        self.activeController = controller

        // Observe phase transitions to mirror onto this view's
        // local success / error state. The controller stays in the
        // coordinator independently of this view's lifetime, so
        // the same flow remains reachable from
        // `RegistrationProgressView` after dismissal.
        observeController(
            controller,
            walletId: walletId,
            identityIndex: identityIndex,
            network: network
        )
    }

    /// Bridge a controller's phase transitions to this view's
    /// `createdIdentityId` / `submitError` / `isCreating` state.
    /// The observer task auto-cancels when this view deallocates
    /// (Swift task lifecycle on the captured `self`), but the
    /// controller itself outlives the view.
    private func observeController(
        _ controller: IdentityRegistrationController,
        walletId: Data,
        identityIndex: UInt32,
        network: Network
    ) {
        Task {
            for await phase in phaseStream(controller: controller) {
                switch phase {
                case .completed(let identityId):
                    do {
                        try persistCreatedIdentity(
                            identityId: identityId,
                            network: network
                        )
                        markIdentitySlotUsed(
                            walletId: walletId,
                            identityIndex: identityIndex
                        )
                        try modelContext.save()
                        self.createdIdentityId = identityId
                    } catch {
                        self.submitError = .init(
                            message: error.localizedDescription
                        )
                    }
                    self.isCreating = false
                    // Do NOT clear `activeController` here — the
                    // pushed `RegistrationProgressView` destination
                    // is bound to `if let controller = activeController`,
                    // so nilling it tears the destination down and
                    // SwiftUI pops back to the form before the user
                    // sees the success state. The controller lives
                    // on `walletManager.registrationCoordinator`
                    // anyway and is purged by the coordinator's
                    // ~30 s post-completion retention.
                    return
                case .failed(let message):
                    self.submitError = .init(message: message)
                    self.isCreating = false
                    // Same rationale as `.completed`: keep the
                    // controller so the destination can render
                    // the inline failure state.
                    return
                case .unconfirmed:
                    // Broadcast landed but its result couldn't be confirmed;
                    // the identity is probably already live on chain. Mark the
                    // slot used (same as `.completed`) — `usedIdentityIndices`
                    // unions the persisted `isUsed` reservation with the
                    // `PersistentIdentity` rows, so this is the reservation
                    // that holds the slot across app restarts until the next
                    // sync writes the identity row. It is best-effort (silent
                    // no-op when the slot row is beyond the derived
                    // lookahead), so the live `.unconfirmed` controller and
                    // the dismissibility gate in `PendingRegistrationsList`
                    // remain as defense in depth. We do NOT persist a
                    // `PersistentIdentity` row here — the proof-verified
                    // identity wasn't returned; the next sync writes the row
                    // once the identity is confirmed on chain.
                    markIdentitySlotUsed(
                        walletId: walletId,
                        identityIndex: identityIndex
                    )
                    try? modelContext.save()
                    self.isCreating = false
                    return
                default:
                    continue
                }
            }
        }
    }

    /// Poll the controller's `phase` until it reaches a terminal
    /// state (`.completed` / `.failed`) and yield each transition.
    /// Combine's `$phase.values` would be more idiomatic, but
    /// `AnyCancellable` isn't `Sendable` and the @Sendable closure
    /// of `AsyncStream`'s builder rejects it. A small polling loop
    /// avoids the bridge entirely.
    private func phaseStream(
        controller: IdentityRegistrationController
    ) -> AsyncStream<IdentityRegistrationController.Phase> {
        AsyncStream { continuation in
            Task { @MainActor in
                var lastEmitted: IdentityRegistrationController.Phase? = nil
                while !Task.isCancelled {
                    let phase = controller.phase
                    if phase != lastEmitted {
                        continuation.yield(phase)
                        lastEmitted = phase
                    }
                    switch phase {
                    case .completed, .failed, .unconfirmed:
                        continuation.finish()
                        return
                    default:
                        break
                    }
                    try? await Task.sleep(nanoseconds: 100_000_000)
                }
                continuation.finish()
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

    /// Patch the UI-only fields on the newly-registered identity's
    /// `PersistentIdentity` row.
    ///
    /// `IdentityManager::add_identity` on the Rust side already
    /// emitted an `IdentityChangeSet` + `IdentityKeysChangeSet`
    /// during `registerIdentityFromAddresses`, so by the time we
    /// land here the `PersistentIdentity` + `PersistentPublicKey`
    /// rows already exist (written by the
    /// `on_persist_identities_fn` / `on_persist_identity_keys_fn`
    /// persister callbacks — see
    /// `PlatformWalletPersistenceHandler.persistIdentities` /
    /// `persistIdentityKeys`). What the Rust callback can't know:
    ///
    /// - `network` — the persister is wallet-id-keyed; the wallet's
    ///   network isn't threaded through the callback today, so the
    ///   row inherits whatever default the persister wrote. We stamp
    ///   the caller-supplied value here.
    /// - `identityType` — fresh identities from
    ///   `register_from_addresses` are always user identities, but
    ///   the Rust-side type system doesn't carry the enum; stamp
    ///   `.user` here explicitly.
    ///
    /// This replaces the prior full-insert path that also wrote
    /// balance/revision/keys. Those fields are now Rust-authoritative
    /// (the SDK returns the post-transition balance, which Rust
    /// persists — the old `initialCreditBalance` parameter was a
    /// stale pre-submit estimate).
    ///
    /// Private-key material stays out of the Keychain here. The
    /// authentication keys are seed-derived at DIP-9 paths, so the
    /// Rust persister emits them as `AtWalletDerivationPath`
    /// variants that flow through the callback as namespaced
    /// `"derived:<path>"` identifiers — no Keychain write needed.
    private func persistCreatedIdentity(
        identityId: Data,
        network: Network
    ) throws {
        let descriptor = FetchDescriptor<PersistentIdentity>(
            predicate: #Predicate { $0.identityId == identityId }
        )
        guard let row = try modelContext.fetch(descriptor).first else {
            // Row hasn't landed yet — shouldn't happen in
            // production (Rust emits the changeset before the
            // registration FFI returns), but if the persister
            // callback isn't wired the UI-side fields stay unset
            // until a subsequent refresh. Log + fall through so the
            // registration doesn't fail.
            print("⚠️ persistCreatedIdentity: no PersistentIdentity row for \(identityId.toHexString()) — persister callback likely not wired")
            return
        }
        row.network = network
        row.identityType = SwiftDashSDK.IdentityType.user.rawValue
        row.lastUpdated = Date()
    }

    /// Flip `isUsed` on the consumed identity-registration slot so
    /// `usedIdentityIndices` (which unions the flag with the
    /// `PersistentIdentity` rows) skips it. For confirmed registrations
    /// this is redundant with the identity row, but for `.unconfirmed`
    /// Type-20 broadcasts it is the ONLY persisted reservation holding
    /// the slot until the next sync writes the identity row — losing it
    /// would offer the same index for a duplicate submission. Silent
    /// no-op if the slot row isn't found (an index beyond the derived
    /// lookahead); the Rust side remains the on-chain source of truth.
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

    /// The tracked asset lock the user picked in the resume flow.
    /// Looked up via `persistentModelID` so it stays valid even if
    /// the row's mutable fields (status, updatedAt) tick after
    /// selection.
    private var selectedAssetLock: PersistentAssetLock? {
        guard let id = selectedAssetLockId else { return nil }
        return allAssetLocks.first { $0.persistentModelID == id }
    }

    /// The currently-selected Core BIP44 standard account, if any.
    /// Used for the Core-funded identity-creation path.
    ///
    /// Restricted to BIP44 standard accounts (`accountType == 0` AND
    /// `standardTag == 0`) — the Rust side
    /// (`create_funded_asset_lock_proof`) only supports BIP44 standard
    /// today, so CoinJoin (`accountType == 1`) and BIP32
    /// (`standardTag == 1`) are intentionally excluded as funding
    /// sources for new-identity registration.
    private var selectedCoreAccount: PersistentAccount? {
        guard
            case .account(let persistentId) = fundingSelection,
            let account = allAccounts.first(where: {
                $0.persistentModelID == persistentId
            }),
            account.accountType == 0,
            account.standardTag == 0
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

    /// REQUIRED Type-20 fallback failure address for `walletId`, as raw
    /// 21-byte `PlatformAddress` storage bytes (1-byte variant tag +
    /// 20-byte hash). If identity creation fails a stateful check (a
    /// pubkey hash already registered to another identity) the spend is
    /// still finalized and the value lands at this address minus a
    /// penalty — it's bound into the transition sighash, so it can't be
    /// redirected after signing.
    ///
    /// Built from the wallet's Platform Payment account (type tag 14):
    /// the lowest-`addressIndex` `PersistentPlatformAddress` row gives
    /// `Data([row.addressType]) + row.addressHash`. This is the exact
    /// `(addressType, hash)` pairing `buildInputs` feeds the
    /// address-funded FFI, so the encoding matches
    /// `PlatformAddress.toBytes()`. Returns `nil` when the wallet has no
    /// Platform Payment address yet — the `.shieldedBalance` option is
    /// gated on this being non-nil so `submit` can guard cleanly.
    private func shieldedFallbackAddressBytes(for walletId: Data) -> Data? {
        guard let account = allAccounts.first(where: {
            $0.wallet.walletId == walletId && $0.accountType == 14
        }) else {
            return nil
        }
        guard let row = account.platformAddresses
            .min(by: { $0.addressIndex < $1.addressIndex })
        else {
            return nil
        }
        return Data([row.addressType]) + row.addressHash
    }

    /// Whether the `.shieldedBalance` funding option should be offered
    /// for `walletId`. Requires ALL of:
    ///   - the wallet is engine-bound (its default Orchard address
    ///     resolves), so ANY loaded+bound wallet qualifies — not just
    ///     the single UI mirror (`boundWalletId`),
    ///   - this wallet's own unspent pool balance > 0,
    ///   - a fallback platform address exists (Type-20 requires it).
    /// Gating visibility on the fallback lets `submit` guard without a
    /// force-unwrap and surface a clear error path-free.
    private func shieldedOptionAvailable(for walletId: Data) -> Bool {
        // Engine-bound (address resolves) + this wallet's own unspent
        // pool balance > 0 + a Type-20 fallback platform address exists.
        // No longer gated on the single UI mirror (`boundWalletId`), so
        // ANY loaded+bound wallet can fund from its own pool.
        let engineBound = (try? walletManager.shieldedDefaultAddress(
            walletId: walletId,
            account: 0
        )) != nil
        return engineBound
            && shieldedPoolBalance(for: walletId) > 0
            && shieldedFallbackAddressBytes(for: walletId) != nil
    }

    /// Per-wallet shielded pool balance: sum of `walletId`'s unspent
    /// `PersistentShieldedNote` values, fetched from `modelContext`
    /// rather than the single-mirror `shieldedService.shieldedBalance`.
    /// Correct for ANY loaded wallet, not just the mirror's
    /// `firstWallet`.
    private func shieldedPoolBalance(for walletId: Data) -> UInt64 {
        let descriptor = FetchDescriptor<PersistentShieldedNote>(
            predicate: #Predicate { $0.walletId == walletId && $0.isSpent == false }
        )
        let notes = (try? modelContext.fetch(descriptor)) ?? []
        return notes.reduce(0) { $0 + $1.value }
    }

    /// The wallet id currently chosen in the source-wallet picker, or
    /// `nil` for the walletless / unselected states. Lets standalone
    /// funding sub-sections (`shieldedDenominationSection`) resolve the
    /// per-wallet shielded pool without each threading `walletId`
    /// through the `@ViewBuilder` call chain.
    private var selectedWalletId: Data? {
        if case .wallet(let walletId) = walletSelection { return walletId }
        return nil
    }

    /// Derive + Keychain-persist the DashPay encryption/decryption
    /// key pair (kid `firstKeyId` = ENCRYPTION, kid `firstKeyId+1` =
    /// DECRYPTION), build the matching `IdentityPubkey` rows with
    /// contract bounds pointing at the DashPay system contract's
    /// `contactRequest` document type, and return both. Throws on
    /// any derivation / keychain-write failure so the caller can
    /// surface the error inline and bail out of the registration.
    ///
    /// Conceptually this is the per-key body that
    /// `prePersistIdentityKeysForRegistration` would emit if Rust
    /// owned the DashPay-purpose policy too — we keep it Swift-side
    /// for now because the Rust function's per-slot purpose table
    /// is hardcoded to the auth pattern (MASTER + HIGH×N).
    private func makeDashpayKeyPair(
        managedWallet: ManagedPlatformWallet,
        walletId: Data,
        identityIndex: UInt32,
        firstKeyId: UInt32,
        network: Network
    ) throws -> [ManagedPlatformWallet.IdentityPubkey] {
        let purposes: [(keyId: UInt32, purpose: KeyPurpose)] = [
            (firstKeyId, .encryption),
            (firstKeyId + 1, .decryption),
        ]
        let bounds: ManagedPlatformWallet.ContractBounds = .singleContractDocumentType(
            id: Self.dashpayContractId,
            documentTypeName: Self.dashpayContactRequestDocumentType
        )
        let walletIdHex = walletId.toHexString()
        let identityIdPlaceholder = ""  // overwritten by persister callback
                                        // when the identity actually
                                        // lands on-chain; the metadata
                                        // we write here only needs to
                                        // satisfy the keychain
                                        // round-trip lookup by pubkey
                                        // hex.

        var rows: [ManagedPlatformWallet.IdentityPubkey] = []
        rows.reserveCapacity(purposes.count)

        for (keyId, purpose) in purposes {
            let preview = try managedWallet.deriveIdentityAuthKeyAtSlot(
                identityIndex: identityIndex,
                keyId: keyId,
                network: network
            )

            // Defence against derivation drift / FFI marshalling
            // bugs — mirrors `AddIdentityKeyView.submit`'s cross-
            // check. A mismatched DashPay key lands on Platform as
            // a key the trampoline can't sign with and surfaces as
            // an opaque "encrypted xpub" failure on the first
            // contact-request flow, which is much harder to debug
            // after the fact than failing fast here.
            guard
                KeyValidation.validatePrivateKeyForPublicKey(
                    privateKeyHex: preview.privateKeyData.toHexString(),
                    publicKeyHex: preview.publicKeyHex,
                    keyType: .ecdsaSecp256k1,
                    network: network
                )
            else {
                throw PlatformWalletError.walletOperation(
                    "Derived DashPay key (kid \(keyId), purpose \(purpose.name)) didn't match its public key — refusing to persist"
                )
            }

            let pubKeyHashHex = SwiftDashSDK.KeychainManager.computePublicKeyHashHex(
                preview.publicKeyData
            )
            let metadata = IdentityPrivateKeyMetadata(
                identityId: identityIdPlaceholder,
                keyId: keyId,
                walletId: walletIdHex,
                identityIndex: identityIndex,
                keyIndex: keyId,
                derivationPath: preview.derivationPath,
                publicKey: preview.publicKeyHex,
                publicKeyHash: pubKeyHashHex,
                keyType: KeyType.ecdsaSecp256k1.rawValue,
                purpose: purpose.rawValue,
                securityLevel: SecurityLevel.medium.rawValue
            )
            guard KeychainManager.shared.storeIdentityPrivateKey(
                preview.privateKeyData,
                derivationPath: preview.derivationPath,
                metadata: metadata
            ) != nil else {
                throw PlatformWalletError.walletOperation(
                    "Could not persist DashPay key (kid \(keyId), purpose \(purpose.name)) to Keychain"
                )
            }
            rows.append(
                ManagedPlatformWallet.IdentityPubkey(
                    keyId: keyId,
                    keyType: .ecdsaSecp256k1,
                    purpose: purpose,
                    securityLevel: .medium,
                    pubkeyBytes: preview.publicKeyData,
                    contractBounds: bounds
                )
            )
        }
        return rows
    }

    /// Spendable balance (duffs) for a Core / CoinJoin account.
    /// Reads live FFI-backed balance from the platform-wallet manager
    /// (the SwiftData per-account `balanceConfirmed` scalar isn't
    /// maintained by the persister — `AccountDetailView.balanceCard`
    /// uses the same live source). Returns 0 if no match.
    private func coreAccountBalanceDuffs(_ account: PersistentAccount) -> UInt64 {
        let balances = walletManager.accountBalances(for: account.wallet.walletId)
        guard let match = balances.first(where: { b in
            UInt32(b.typeTag) == account.accountType
                && b.standardTag == account.standardTag
                && b.index == account.accountIndex
        }) else {
            return 0
        }
        return match.confirmed + match.unconfirmed
    }

    /// Default amount string (DASH) for the amount field.
    /// Platform Payment → full account balance (one-tap happy path).
    /// Core / CoinJoin → 0.001 DASH (the asset-lock minimum-with-
    /// headroom default); user dials up if they want a larger
    /// initial credit balance.
    private func defaultAmountString(for funding: FundingSelection?) -> String {
        guard
            case .account(let persistentId) = funding,
            let account = allAccounts.first(where: {
                $0.persistentModelID == persistentId
            })
        else {
            return ""
        }
        switch account.accountType {
        case 14:
            let balance = accountBalance(account)
            if balance == 0 { return "" }
            let dash = Double(balance) / Double(Self.creditsPerDash)
            return String(format: "%g", dash)
        case 0, 1:
            let available = coreAccountBalanceDuffs(account)
            let defaultDuffs = min(available, Self.defaultCoreFundingDuffs)
            if defaultDuffs == 0 { return "" }
            let dash = Double(defaultDuffs) / Double(Self.duffsPerDash)
            return String(format: "%g", dash)
        default:
            return ""
        }
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

    /// Parse the amount text into Core duffs (1e8 per DASH). Used for
    /// the Core-funded identity path.
    private var parsedAmountDuffs: UInt64? {
        let trimmed = amountDash.trimmingCharacters(in: .whitespaces)
        guard let dash = Double(trimmed), dash.isFinite, dash > 0 else {
            return nil
        }
        let duffs = (dash * Double(Self.duffsPerDash)).rounded()
        guard duffs >= 1, duffs <= Double(UInt64.max) else { return nil }
        return UInt64(duffs)
    }

    // MARK: - Helpers

    private func walletLabel(for wallet: PersistentWallet) -> String {
        let trimmed = wallet.label.trimmingCharacters(in: .whitespaces)
        let base = trimmed.isEmpty ? shortWalletId(wallet.walletId) : trimmed
        return "\(base) (\(wallet.network?.displayName ?? "Unknown"))"
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
        // Live balances from the Rust side. The persister doesn't
        // maintain `PersistentAccount.balanceConfirmed`, so we read
        // from the platform-wallet manager (same source as
        // `AccountDetailView.balanceCard`).
        let liveBalances = walletManager.accountBalances(for: walletId)
        return allAccounts
            .filter { account in
                guard account.wallet.walletId == walletId else { return false }
                guard CreateIdentityView.isFundingAccount(account) else { return false }
                return CreateIdentityView.hasBalance(
                    account: account,
                    liveBalances: liveBalances
                )
            }
            .sorted { lhs, rhs in
                let lhsKey = CreateIdentityView.sortKey(for: lhs)
                let rhsKey = CreateIdentityView.sortKey(for: rhs)
                return lhsKey < rhsKey
            }
            .map { account in
                let balanceText = Self.balanceText(
                    for: account,
                    liveBalances: liveBalances
                )
                return FundingAccountOption(
                    persistentId: account.persistentModelID,
                    accountType: account.accountType,
                    accountIndex: account.accountIndex,
                    label: Self.fundingLabel(for: account),
                    balanceText: balanceText
                )
            }
    }

    /// Whether an account has any spendable balance for the picker
    /// filter. Same data sources as `balanceText` (live FFI for Core /
    /// CoinJoin, SwiftData for PlatformPayment).
    private static func hasBalance(
        account: PersistentAccount,
        liveBalances: [PlatformWalletManager.AccountBalance]
    ) -> Bool {
        switch account.accountType {
        case 14:
            return account.platformAddresses.contains { $0.balance > 0 }
        default:
            let match = liveBalances.first { b in
                UInt32(b.typeTag) == account.accountType
                    && b.standardTag == account.standardTag
                    && b.index == account.accountIndex
            }
            return (match?.confirmed ?? 0) + (match?.unconfirmed ?? 0) > 0
        }
    }

    /// Picker-row balance text for an account. Core / CoinJoin reads
    /// from `liveBalances` (FFI snapshot); PlatformPayment sums
    /// `platformAddresses[].balance` from SwiftData.
    private static func balanceText(
        for account: PersistentAccount,
        liveBalances: [PlatformWalletManager.AccountBalance]
    ) -> String {
        switch account.accountType {
        case 14:
            let credits = account.platformAddresses.reduce(0) { $0 + $1.balance }
            return credits > 0 ? formatDash(raw: credits, divisor: 100_000_000_000.0) : "empty"
        default:
            let match = liveBalances.first { b in
                UInt32(b.typeTag) == account.accountType
                    && b.standardTag == account.standardTag
                    && b.index == account.accountIndex
            }
            let duffs = (match?.confirmed ?? 0) + (match?.unconfirmed ?? 0)
            return duffs > 0 ? formatDash(raw: duffs, divisor: 100_000_000.0) : "empty"
        }
    }

    /// Identity-registration indices that must not be offered for a new
    /// registration on this wallet: every index claimed by an existing
    /// `PersistentIdentity`, UNIONED with the persisted
    /// `PersistentCoreAddress.isUsed` reservations on the
    /// identity-registration account.
    ///
    /// The union matters for `.unconfirmed` Type-20 registrations: the
    /// broadcast landed but no `PersistentIdentity` row exists until the
    /// next sync confirms the identity, so `markIdentitySlotUsed`'s
    /// persisted `isUsed` flag is the ONLY artifact holding the slot once
    /// the in-memory controller is gone. Without it the same index would
    /// be offered again and a duplicate submission would burn funds
    /// against the registered-key-hash stateful check.
    ///
    /// Identity rows stay authoritative for confirmed identities; the flag
    /// is monotonic extra coverage. Its historical drift (discovered
    /// identities never flipped it) only under-reports, which the identity
    /// rows cover, and over-reporting merely skips an index — registration
    /// indices aren't gap-limited, so a skipped index costs nothing.
    private func usedIdentityIndices(for walletId: Data) -> Set<UInt32> {
        var used = Set(
            allIdentities
                .filter { $0.wallet?.walletId == walletId }
                .map { $0.identityIndex }
        )
        if let account = identityRegistrationAccount(for: walletId) {
            used.formUnion(
                account.coreAddresses
                    .filter(\.isUsed)
                    .map(\.addressIndex)
            )
        }
        return used
    }

    /// One past the highest used registration index on this wallet,
    /// or `0` for a wallet that has never registered an identity.
    /// Identity-registration keys aren't gap-limited — any `N` is
    /// derivable from the mnemonic at `m/9'/coin'/5'/0'/0'/N'/0'` —
    /// so "next unused" is just `max + 1`.
    ///
    /// Uses checked addition: at `UInt32.max` we leave the picker at
    /// `UInt32.max` rather than wrapping to `0` (which would silently
    /// pre-fill a colliding slot). The user can still adjust the
    /// stepper, and `canSubmit` keeps the button disabled until the
    /// chosen index isn't already taken.
    private func nextUnusedIdentityIndex(for walletId: Data) -> UInt32 {
        let used = usedIdentityIndices(for: walletId)
        guard let highest = used.max() else { return 0 }
        return highest == UInt32.max ? UInt32.max : highest + 1
    }

    private func identityRegistrationAccount(for walletId: Data) -> PersistentAccount? {
        allAccounts.first { account in
            account.wallet.walletId == walletId && account.accountType == 2
        }
    }

    /// Accounts eligible to fund a new identity.
    ///
    /// Core funding restricted to **BIP44 standard** accounts only
    /// (`accountType == 0 && standardTag == 0`). CoinJoin
    /// (`accountType == 1`) and BIP32 standard accounts
    /// (`accountType == 0 && standardTag == 1`) are intentionally
    /// hidden — the Rust `create_funded_asset_lock_proof` path used
    /// by `registerIdentityWithFunding` only handles BIP44 standard
    /// today; surfacing the others would let the user pick a source
    /// that silently funds from BIP44 #0 instead.
    ///
    /// PlatformPayment (`accountType == 14`) stays eligible — it
    /// flows through `registerIdentityFromAddresses`, a separate
    /// path that doesn't go through `create_funded_asset_lock_proof`.
    private static func isFundingAccount(_ account: PersistentAccount) -> Bool {
        switch account.accountType {
        case 0: return account.standardTag == 0
        case 14: return true
        default: return false
        }
    }

    /// Decode the display-order `outPointHex` (`<txid hex>:<vout>`)
    /// stored on `PersistentAssetLock` back into `(rawTxidBytes,
    /// vout)`. The rawTxidBytes are 32 bytes in wire / little-
    /// endian order — what the FFI's `OutPointFFI.txid` field
    /// expects. Reverse of `PersistentAssetLock.encodeOutPoint`.
    /// Returns `nil` on any parse failure.
    ///
    /// Internal (not private) so the unit test target can pin the
    /// round-trip with `encodeOutPoint`. Wrong endianness on either
    /// side would silently address a different outpoint and produce
    /// confusing Platform-side proof-verification failures, so the
    /// round-trip invariant is worth pinning explicitly.
    nonisolated static func parseOutPointHex(_ hex: String) -> (Data, UInt32)? {
        let parts = hex.split(
            separator: ":",
            maxSplits: 1,
            omittingEmptySubsequences: false
        )
        guard parts.count == 2 else { return nil }
        let txidHex = String(parts[0])
        guard let vout = UInt32(parts[1]) else { return nil }
        guard txidHex.count == 64 else { return nil }
        var txidDisplay = Data(capacity: 32)
        var idx = txidHex.startIndex
        for _ in 0..<32 {
            let end = txidHex.index(idx, offsetBy: 2)
            guard let byte = UInt8(txidHex[idx..<end], radix: 16) else {
                return nil
            }
            txidDisplay.append(byte)
            idx = end
        }
        // Reverse to raw wire-order to match OutPointFFI.txid.
        let txidRaw = Data(txidDisplay.reversed())
        return (txidRaw, vout)
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
    /// Fund the new identity from the wallet's bound shielded (Orchard)
    /// pool via the Type-20 IdentityCreateFromShieldedPool transition.
    /// Only offered when the wallet is the one currently bound to
    /// `ShieldedService` and that pool has a positive balance.
    case shieldedBalance
}

private struct FundingAccountOption: Identifiable {
    let persistentId: PersistentIdentifier
    /// HD account type tag (e.g. 0 = BIP44 standard, 14 = Platform
    /// Payment). Paired with `accountIndex` in the row's accessibility
    /// identifier because the index alone is NOT unique across account
    /// types — this picker lists both Core and Platform Payment
    /// accounts, so a bare `#0` would collide (BIP44 #0 vs Platform
    /// Payment #0).
    let accountType: UInt32
    /// HD account index, carried only so the accessibility identifier
    /// for this row can be stable across launches (the
    /// `PersistentIdentifier` opaque id is not).
    let accountIndex: UInt32
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
