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
//      wallet (any type — Core pools and Platform Payment both work)
//      or "Fund from unused Asset Lock".
//
// The first-pass implementation only wires the Platform Payment
// funding path — see `submit()`. Core / CoinJoin / walletless paths
// are still stubs pending their respective FFI entry points.

import SwiftUI
import SwiftDashSDK
import SwiftData

struct CreateIdentityView: View {
    @Environment(\.dismiss) private var dismiss
    @Environment(\.modelContext) private var modelContext
    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var platformState: AppState

    /// Called when the user taps "View" on the success state.
    /// Carries the newly created identity id back to the parent
    /// (typically `IdentitiesContentView`), which is expected to
    /// (a) dismiss this sheet and (b) push `IdentityDetailView`
    /// in its own `NavigationStack`. Empty default makes the view
    /// usable in places that don't need post-create navigation.
    var onViewIdentity: (Data) -> Void = { _ in }

    /// Default number of Platform identity authentication keys to
    /// register in this first-pass flow. First key is MASTER, the
    /// rest are HIGH. Advanced override is intentionally not exposed
    /// here yet.
    private static let defaultKeyCount: UInt32 = 3

    /// Credits per DASH (1e11) — the divisor used for Platform-side
    /// credit amounts. Duplicated from `PersistentPlatformAddress`
    /// docstring; kept here so the conversion logic stays local.
    private static let creditsPerDash: UInt64 = 100_000_000_000

    /// Duffs per DASH (1e8) — Core-side scale, used by the Core-funded
    /// identity path.
    private static let duffsPerDash: UInt64 = 100_000_000

    /// Protocol minimum asset-lock funding for an identity create.
    /// Mirrors `IdentityCreateTransition::calculate_min_required_fee_v1`
    /// in `rs-platform-version`:
    ///   identity_create_base_cost (2_000_000 credits)
    ///   + asset_lock_base (200_000 duffs * 1000 credits/duff = 200_000_000)
    ///   + identity_key_in_creation_cost (6_500_000) * defaultKeyCount (3)
    ///   = 221_500_000 credits / 1000 = 221_500 duffs (0.002215 DASH).
    /// The v0 floor was 200_000 duffs but with key_in_creation_cost
    /// dynamic at v1, the per-key surcharge has to be added. Submitting
    /// below this gets rejected by Platform with
    /// `IdentityAssetLockTransactionOutPointNotEnoughBalance`. Keep this
    /// in sync if `defaultKeyCount` changes.
    private static let minIdentityFundingDuffs: UInt64 = 221_500

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

    /// Raw asset-lock proof text, used only in the walletless path.
    /// Accepted encoding is base64 or lowercase hex — the submit
    /// logic (future) will detect + decode.
    @State private var walletlessProof: String = ""

    /// Amount (in DASH) to fund the new identity with. Populated
    /// automatically from the selected account's balance; the user
    /// can lower it but not exceed the available balance.
    @State private var amountDash: String = ""

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

    /// Active registration controller for the Core-funded path
    /// while the registration is in flight. Set in
    /// `submitCoreFunded`; drives the pushed
    /// `RegistrationProgressView` destination. Cleared on
    /// terminal phase by `observeController`.
    @State private var activeController: IdentityRegistrationController? = nil

    /// Toggle for the pushed registration-progress destination.
    /// `submit()` sets this to `true` once the coordinator has
    /// spawned a controller; the dedicated screen then owns the
    /// rest of the user flow (progress steps + success / error +
    /// "View Identity" navigation).
    @State private var showProgressDestination: Bool = false

    var body: some View {
        NavigationStack {
            Form {
                sourceWalletSection
                fundingSection
                amountSection
                identityIndexSection
                if canSubmit {
                    submitSection
                }
            }
            .navigationTitle("Create Identity")
            .navigationBarTitleDisplayMode(.inline)
            .navigationDestination(isPresented: $showProgressDestination) {
                // Standalone progress screen for the spawned
                // controller. Owns the 5-step progress UI and the
                // terminal success / failure sections. The "View"
                // button on success closes the sheet AND tells
                // the parent which identity to navigate to via
                // `onViewIdentity`.
                if let controller = activeController {
                    RegistrationProgressView(
                        controller: controller,
                        onViewIdentity: { identityId in
                            // Tell the parent first (it stores
                            // the id and triggers its own
                            // navigation), then dismiss the
                            // sheet. SwiftUI processes the state
                            // update before the dismissal
                            // animation completes, so the parent
                            // is already primed to push.
                            onViewIdentity(identityId)
                            dismiss()
                        }
                    )
                }
            }
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

    // MARK: - Sections

    private var sourceWalletSection: some View {
        Section {
            Picker("Source Wallet", selection: $walletSelection) {
                Text("Select…")
                    .tag(Optional<WalletSelection>.none)
                ForEach(wallets) { wallet in
                    Text(walletLabel(for: wallet))
                        .tag(Optional(WalletSelection.wallet(id: wallet.walletId)))
                }
                Divider()
                Text("Create without Wallet")
                    .tag(Optional(WalletSelection.walletless))
            }
            .onChange(of: walletSelection) { _, newValue in
                // Reset downstream selection whenever the wallet
                // changes so a stale account / proof can't leak
                // through.
                fundingSelection = nil
                walletlessProof = ""
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

    @ViewBuilder
    private func walletAccountSection(for walletId: Data) -> some View {
        let options = accountOptions(for: walletId)
        Section {
            Picker("Funding Source", selection: $fundingSelection) {
                Text("Select…")
                    .tag(Optional<FundingSelection>.none)
                ForEach(options) { option in
                    Text("\(option.label) — \(option.balanceText)")
                        .tag(Optional(FundingSelection.account(id: option.persistentId)))
                }
                Divider()
                Text("Fund from unused Asset Lock")
                    .tag(Optional(FundingSelection.unusedAssetLock))
            }
            .onChange(of: fundingSelection) { _, newValue in
                // Pre-fill the amount with the full available balance
                // of the selected Platform Payment account so the
                // happy path is one tap. Users can dial it down.
                amountDash = defaultAmountString(for: newValue)
            }
        } header: {
            Text("Funding Source")
        } footer: {
            Text(
                "Any account on the selected wallet with a balance can fund "
                + "the identity — Core or Platform Payment. Empty accounts "
                + "are hidden. \"Fund from unused Asset Lock\" picks an "
                + "existing tracked asset lock instead."
            )
        }
    }

    /// Amount (in DASH) to fund the new identity with. Shown for
    /// Platform Payment and Core / CoinJoin funding sources.
    @ViewBuilder
    private var amountSection: some View {
        if let account = selectedPlatformAccount {
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
                    raw: Self.minIdentityFundingDuffs,
                    divisor: Double(Self.duffsPerDash)
                )
                Text("Available: \(available). Minimum: \(minimum). Rust builds an asset-lock transaction from your Core UTXOs and the locked funds become the new identity's initial credit balance.")
            }
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
                return duffs >= Self.minIdentityFundingDuffs && duffs <= available
            }
            // Remaining wallet-backed paths (unused asset lock,
            // future variants) are stubbed — submit stays disabled.
            return false
        default:
            return false
        }
    }

    // MARK: - Submit

    /// Dispatches identity registration to the correct funding path.
    /// Platform-Payment funding uses `registerIdentityFromAddresses`;
    /// Core / CoinJoin funding uses `registerIdentityWithFunding`
    /// (asset-lock proof built Rust-side from wallet UTXOs). Other
    /// funding branches (unused asset-lock, walletless) stay disabled
    /// via `canSubmit` until later iterations.
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
        let identityPubkeys: [ManagedPlatformWallet.IdentityPubkey]
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
        } else if selectedCoreAccount != nil {
            submitCoreFunded(
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

    /// Core / CoinJoin funded registration. Rust builds an asset-lock
    /// transaction from BIP44 account #0's UTXOs (mempool `account_index = 0`
    /// in `create_funded_asset_lock_proof`), broadcasts it, waits for
    /// the instant-send lock, and submits the IdentityCreate state
    /// transition.
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
                    identityIndex: identityIndex,
                    identityPubkeys: identityPubkeys,
                    signer: signer
                )
                return identityId
            }
        )

        // Capture the controller for the pushed
        // `RegistrationProgressView` destination, then trigger the
        // navigation. The destination owns the progress UI + the
        // success/failure terminal state from here on; this view
        // becomes a no-op until the user pops back.
        self.activeController = controller
        self.showProgressDestination = true

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
                    case .completed, .failed:
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
    /// the next call to `unusedIdentityIndices` skips it. Silent
    /// no-op if the slot isn't found — this is cosmetic bookkeeping
    /// and the Rust side is already the source of truth.
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

    /// The currently-selected Core / CoinJoin account, if any. Used
    /// for the Core-funded identity-creation path (Standard BIP44/BIP32
    /// or CoinJoin). The Rust function `create_funded_asset_lock_proof`
    /// reads UTXOs from BIP44 account #0 by convention; for a fresh
    /// wallet this matches what the user has.
    private var selectedCoreAccount: PersistentAccount? {
        guard
            case .account(let persistentId) = fundingSelection,
            let account = allAccounts.first(where: {
                $0.persistentModelID == persistentId
            }),
            account.accountType == 0 || account.accountType == 1
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

    /// Unused identity-registration key indices on the wallet's
    /// Identity Registration account (FFI type tag 2). Each
    /// `PersistentCoreAddress` under that account represents one
    /// registration slot keyed by `addressIndex`; `isUsed` flips to
    /// true once the slot has been consumed by a prior identity
    /// creation. Returns an ascending list of the remaining slots.
    /// Identity-registration indices currently claimed by an existing
    /// `PersistentIdentity` on this wallet. Single source of truth —
    /// the deprecated `PersistentCoreAddress.isUsed` flag was a
    /// denormalized cache that drifted (discovered identities never
    /// flipped it).
    private func usedIdentityIndices(for walletId: Data) -> Set<UInt32> {
        Set(
            allIdentities
                .filter { $0.wallet?.walletId == walletId }
                .map { $0.identityIndex }
        )
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

    /// Account types eligible to fund a new identity.
    private static func isFundingAccount(_ account: PersistentAccount) -> Bool {
        switch account.accountType {
        case 0, 1, 14: return true
        default: return false
        }
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
}

private struct FundingAccountOption: Identifiable {
    let persistentId: PersistentIdentifier
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
