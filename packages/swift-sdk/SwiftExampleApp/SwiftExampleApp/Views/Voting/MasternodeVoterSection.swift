import SwiftData
import SwiftDashSDK
import SwiftUI

// MARK: - Who votes

/// The masternode a vote sheet votes for, and how its vote is signed.
///
/// Shared by the DPNS contest vote, the decision proposal vote and the
/// multiple choice proposal vote. The sheets use the same value to cast and to
/// read "this masternode's answer", so both always name the same masternode.
enum MasternodeVoter: Equatable {
    /// A masternode or evonode identity loaded on this device with its voting
    /// key (Identities, Load Identity), and the pro_tx_hash (wire order) its
    /// row records for the masternode (`PersistentIdentity.masternodeProTxHash`).
    /// The two differ for a masternode stored under its voter identity. The
    /// key stays in the Keychain and signs through `KeychainSigner`.
    case storedKey(identityId: Data, proTxHash: Data)
    /// A pro_tx_hash (wire order) typed into the sheet, with no key: enough
    /// to read any masternode's answer, never to cast. Available in every
    /// build.
    case typedProTxHash(Data)
    #if DEBUG
    /// Debug builds only: a pro_tx_hash (wire order) and a voting private key
    /// typed into the sheet, for testing with arbitrary credentials. The key
    /// is `nil` while its field is empty: reading the masternode's answer
    /// needs only the pro_tx_hash, casting needs both.
    case typedCredentials(proTxHash: Data, votingPrivateKey: Data?)
    #endif

    /// Whether a vote can be cast as this voter. A stored key always can;
    /// a typed pro_tx_hash never; typed credentials only with a voting
    /// private key.
    var canCast: Bool {
        switch self {
        case .storedKey:
            return true
        case .typedProTxHash:
            return false
        #if DEBUG
        case .typedCredentials(_, let votingPrivateKey):
            return votingPrivateKey != nil
        #endif
        }
    }

    /// The pro_tx_hash in the wire order the vote and the "read this
    /// masternode's answer" FFIs take.
    var wireOrderProTxHash: Data {
        switch self {
        case .storedKey(_, let proTxHash):
            return proTxHash
        case .typedProTxHash(let proTxHash):
            return proTxHash
        #if DEBUG
        case .typedCredentials(let proTxHash, _):
            return proTxHash
        #endif
        }
    }

    /// A pro_tx_hash typed as Platform names it, the hex Core displays, in
    /// the wire order the vote FFIs take: its reverse, which Rust reverses
    /// back (see "pro_tx_hash byte order" in
    /// `rs-sdk-ffi/src/voting/masternode_vote.rs`). Load Identity converts
    /// the typed pro_tx_hash once with it and stores the result on the row.
    static func wireOrderProTxHash(displayOrder proTxHash: Data) -> Data {
        Data(proTxHash.reversed())
    }
}

// MARK: - Casting

/// One masternode vote, as each sheet's own SDK calls. `run(as:...)` picks how
/// the vote is signed, so the sheets do not repeat the dispatch.
@MainActor
struct MasternodeVoteCall {
    /// Casts with a signer: the wire-order pro_tx_hash, the 20-byte voting
    /// address and the signer that holds the voting key for the call.
    let withSigner: (_ proTxHash: Data, _ votingAddress: Data, _ signer: KeychainSigner) async throws -> Void

    #if DEBUG
    /// Debug builds only: casts with typed credentials, the wire-order
    /// pro_tx_hash and the raw voting private key.
    private var withTypedKey: ((_ proTxHash: Data, _ votingPrivateKey: Data) async throws -> Void)?
    #endif

    init(
        withSigner: @escaping (_ proTxHash: Data, _ votingAddress: Data, _ signer: KeychainSigner) async throws -> Void
    ) {
        self.withSigner = withSigner
    }

    #if DEBUG
    /// Debug builds only: this call, also able to cast with typed
    /// credentials. Chain it under `#if DEBUG` so a Release build compiles
    /// no raw-key call into the sheet.
    func allowingTypedKey(
        _ cast: @escaping (_ proTxHash: Data, _ votingPrivateKey: Data) async throws -> Void
    ) -> MasternodeVoteCall {
        var call = self
        call.withTypedKey = cast
        return call
    }
    #endif

    /// Casts as `voter`. A stored key signs through a `KeychainSigner` over
    /// `modelContainer`: `withStoredMasternodeVotingKey` makes the identity's
    /// voting key signable for the call and gives its voting address, and
    /// zeroes the key when the call ends.
    func run(
        as voter: MasternodeVoter,
        modelContainer: ModelContainer,
        network: Network
    ) async throws {
        switch voter {
        case .storedKey(let identityId, let proTxHash):
            let signer = KeychainSigner(modelContainer: modelContainer, network: network)
            try await signer.withStoredMasternodeVotingKey(identityId: identityId) { votingAddress in
                try await withSigner(proTxHash, votingAddress, signer)
            }
            _ = signer  // keepalive: see KeychainSigner lifetime contract.
        case .typedProTxHash:
            throw MasternodeVoteCallError.typedProTxHashCannotCast
        #if DEBUG
        case .typedCredentials(let proTxHash, let votingPrivateKey):
            guard let withTypedKey else {
                throw MasternodeVoteCallError.typedCredentialsUnsupported
            }
            guard let votingPrivateKey else {
                throw MasternodeVoteCallError.typedCredentialsWithoutKey
            }
            try await withTypedKey(proTxHash, votingPrivateKey)
        #endif
        }
    }
}

enum MasternodeVoteCallError: LocalizedError {
    case typedProTxHashCannotCast
    #if DEBUG
    case typedCredentialsUnsupported
    case typedCredentialsWithoutKey
    #endif

    var errorDescription: String? {
        switch self {
        case .typedProTxHashCannotCast:
            return "A typed pro_tx_hash only reads a masternode's answer. Pick a loaded masternode to cast a vote."
        #if DEBUG
        case .typedCredentialsUnsupported:
            return "This vote cannot be cast with typed credentials."
        case .typedCredentialsWithoutKey:
            return "Enter the voting private key to cast a vote with typed credentials."
        #endif
        }
    }
}

// MARK: - Picking the voter

/// The form section a vote sheet uses to pick who votes: a loaded masternode
/// or evonode identity on the current network whose voting key is stored.
/// Writes the choice to `voter`, `nil` while nothing usable is picked.
///
/// `accessibilityPrefix` names the sheet (`contestVote`,
/// `decisionProposalVote`, `multipleChoiceProposalVote`); the picker is
/// `<prefix>.voter`.
///
/// With `allowsTypedProTxHash`, a sheet that reads a masternode's answer
/// also offers, in every build, a typed pro_tx_hash with no key
/// (`<prefix>.useTypedProTxHash`, field `<prefix>.proTxHash`), published as
/// `.typedProTxHash`: it reads any masternode's answer and casts nothing.
///
/// Rust decides from the check recorded on a candidate, with the address of
/// the key it was made of (`PersistentIdentity.recordedVotingKey`), whether it
/// may vote and whether its key is checked when picked
/// (`MasternodeRecordedVotingKeyUse`). A candidate
/// that may not vote is labelled and never published. A picked candidate
/// whose key is to be checked first is checked once when picked
/// (`<prefix>.voterKeyCheck` shows how it went), and what Rust says to record
/// is written on its row.
struct MasternodeVoterSection: View {
    let accessibilityPrefix: String
    @Binding var voter: MasternodeVoter?
    /// Offer a typed pro_tx_hash for reading an answer. Set by the sheets
    /// that have a "read this masternode's answer" action.
    var allowsTypedProTxHash: Bool = false

    @EnvironmentObject private var appState: AppState
    @Environment(\.modelContext) private var modelContext
    @Query private var identities: [PersistentIdentity]

    /// The identity whose stored voting key is being checked right now.
    @State private var checkingIdentityId: Data?
    /// The pick, with what its row recorded, that the last check ran for.
    /// A stored key Rust says to check before use is published only once the
    /// check for the current pick and record has run.
    @State private var checkedTrigger: VotingKeyCheckTrigger?
    /// Why the last check of the picked identity's key did not happen or was
    /// not saved, by identity.
    @State private var checkFailure: (identityId: Data, failure: VotingKeyCheckFailure)?

    @State private var selectedIdentityId: Data?
    @State private var useTypedProTxHash = false
    /// Shared by the typed pro_tx_hash and, in Debug, the typed credentials;
    /// their toggles are mutually exclusive.
    @State private var proTxHashHex = ""
    #if DEBUG
    @State private var useTypedCredentials = false
    @State private var votingKeyHex = ""
    #endif

    /// Whether a typed input, not the picked identity, names the voter.
    private var usesTypedInput: Bool {
        #if DEBUG
        if useTypedCredentials { return true }
        #endif
        return allowsTypedProTxHash && useTypedProTxHash
    }

    /// Loaded masternode and evonode identities on the current network that
    /// carry a stored voting key.
    private var votingIdentities: [PersistentIdentity] {
        let network = appState.currentNetwork
        return identities
            .filter { identity in
                identity.network == network
                    && identity.votingPrivateKeyIdentifier != nil
                    && (identity.identityTypeEnum == .masternode
                        || identity.identityTypeEnum == .evonode)
            }
            .sorted { $0.displayName < $1.displayName }
    }

    /// The stored verdict of the picked identity's voting key.
    private var selectedVotingKeyCheckRaw: String? {
        votingIdentities.first { $0.identityId == selectedIdentityId }?.votingKeyCheckRaw
    }

    /// The voting address the picked identity's stored verdict was made of.
    private var selectedVotingKeyCheckAddress: Data? {
        votingIdentities.first { $0.identityId == selectedIdentityId }?.votingKeyCheckAddress
    }

    var body: some View {
        Section {
            storedKeyRows
            if allowsTypedProTxHash {
                typedProTxHashRows
            }
            #if DEBUG
            typedCredentialRows
            #endif
        } header: {
            Text("Voting Masternode")
        } footer: {
            Text("When you vote with a loaded identity, the signer uses its stored voting key. You don’t need to enter the private key here.")
        }
        .onAppear(perform: selectDefaultIfNeeded)
        // `@Query` updates while the sheet is open can add or remove an
        // eligible identity; keep the selection on one that still is.
        .onChange(of: votingIdentities.map(\.identityId)) { _, _ in selectDefaultIfNeeded() }
        // A network switch changes which identities are eligible even when
        // the id list compares equal; re-check the selection.
        .onChange(of: appState.currentNetwork) { _, _ in selectDefaultIfNeeded() }
        .onChange(of: selectedIdentityId) { _, _ in publish() }
        // The picked row's verdict can change while the sheet is open (this
        // section's own check, or another window loading the identity again):
        // publish again so the sheet's voter follows it.
        .onChange(of: selectedVotingKeyCheckRaw) { _, _ in publish() }
        // A verdict recorded again for another key can keep its value and
        // change only its address.
        .onChange(of: selectedVotingKeyCheckAddress) { _, _ in publish() }
        // A row resolved by Rust's load rule records its pro_tx_hash.
        .onChange(of: votingIdentities.first { $0.identityId == selectedIdentityId }?.masternodeProTxHash) { _, _ in publish() }
        // Once per pick and per record of it: the key is checked when Rust
        // says to check it before use, also when its record changes while
        // the sheet is open (a load of another key for the masternode marks
        // it to be checked again).
        .task(id: currentTrigger) { await checkSelectedVotingKeyIfNeeded() }
        .onChange(of: checkedTrigger) { _, _ in publish() }
        .onChange(of: useTypedProTxHash) { _, isOn in
            #if DEBUG
            if isOn { useTypedCredentials = false }
            #endif
            publish()
        }
        .onChange(of: proTxHashHex) { _, _ in publish() }
        #if DEBUG
        .onChange(of: useTypedCredentials) { _, isOn in
            if isOn { useTypedProTxHash = false }
            publish()
        }
        .onChange(of: votingKeyHex) { _, _ in publish() }
        #endif
    }

    @ViewBuilder
    private var storedKeyRows: some View {
        let candidates = votingIdentities
        if candidates.isEmpty {
            Text("No masternode or evonode identity with a voting key is loaded on this network. Load one with its voting private key under Identities, Load Identity.")
                .font(.caption)
                .foregroundColor(.secondary)
                .accessibilityIdentifier("\(accessibilityPrefix).noVoter")
        } else {
            Picker("Masternode", selection: $selectedIdentityId) {
                ForEach(candidates, id: \.identityId) { identity in
                    voterLabel(identity, among: candidates)
                        .tag(Optional(identity.identityId))
                }
            }
            .accessibilityIdentifier("\(accessibilityPrefix).voter")
            .disabled(usesTypedInput)
            if !usesTypedInput, let selected = candidates.first(where: { $0.identityId == selectedIdentityId }) {
                VotingKeyCheckStatus(
                    check: selected.votingKeyCheck,
                    isChecking: checkingIdentityId == selected.identityId,
                    failure: checkFailure?.identityId == selected.identityId ? checkFailure?.failure : nil
                )
                .accessibilityIdentifier("\(accessibilityPrefix).voterKeyCheck")
            }
        }
    }

    /// The identity's name and type, plus the start of its id when another
    /// candidate shows the same name, since aliases need not be unique.
    private func voterLabel(_ identity: PersistentIdentity, among candidates: [PersistentIdentity]) -> some View {
        let name = identity.displayName
        let shared = candidates.contains { $0.identityId != identity.identityId && $0.displayName == name }
        let suffix = shared ? ", \(identity.identityId.toHexString().prefix(8))" : ""
        let mismatch = Self.isRefused(identity) ? ", voting key does not match" : ""
        return Text("\(name) (\(identity.identityTypeEnum.rawValue)\(suffix)\(mismatch))")
    }

    /// Whether the identity's stored voting key may not vote, as Rust
    /// decides from the check recorded for it. The stored key's address is
    /// not read here (no Keychain read per row), so the record is taken as it
    /// is; the check when the identity is picked reads it
    /// (`checkSelectedVotingKeyIfNeeded`).
    private static func isRefused(_ identity: PersistentIdentity) -> Bool {
        !MasternodeRecordedVotingKeyUse(recorded: identity.recordedVotingKey, storedVotingAddress: nil).canVote
    }

    /// A pro_tx_hash typed with no key, available in every build: reads
    /// any masternode's answer, casts nothing.
    @ViewBuilder
    private var typedProTxHashRows: some View {
        Toggle("Read another masternode's answer", isOn: $useTypedProTxHash)
            .accessibilityIdentifier("\(accessibilityPrefix).useTypedProTxHash")
        if useTypedProTxHash {
            proTxHashField
            Text("Reads the answer of the masternode with this pro_tx_hash; no key is needed and no vote can be cast. The pro_tx_hash goes in wire order, the reverse of the hex Core displays.")
                .font(.caption2)
                .foregroundColor(.secondary)
        }
    }

    private var proTxHashField: some View {
        TextField("pro_tx_hash, wire order (64 hex chars)", text: $proTxHashHex)
            .font(.system(.caption, design: .monospaced))
            .autocorrectionDisabled()
            .textInputAutocapitalization(.never)
            .accessibilityIdentifier("\(accessibilityPrefix).proTxHash")
    }

    /// Decode a 32-byte value typed as 64 hex characters. `Data(hexString:)`
    /// drops a trailing odd nibble, so the length is checked first.
    private static func decode32(_ hex: String) -> Data? {
        let trimmed = hex.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmed.count == 64, let data = Data(hexString: trimmed), data.count == 32 else {
            return nil
        }
        return data
    }

    #if DEBUG
    // Debug-only raw credential entry. Gated behind `#if DEBUG` so a
    // Release/TestFlight build cannot direct users to paste a raw voting
    // private key, bypassing the `KeychainSigner` boundary the stored-key
    // path enforces (the same rule as the raw address forms in
    // `TransitionCategoryView`). In Release neither the key field nor its
    // decoding in `publish()` is compiled.
    @ViewBuilder
    private var typedCredentialRows: some View {
        Toggle("Use typed test credentials (debug only)", isOn: $useTypedCredentials)
            .accessibilityIdentifier("\(accessibilityPrefix).useTypedCredentials")
        if useTypedCredentials {
            proTxHashField
            SecureField("voting private key (64 hex chars)", text: $votingKeyHex)
                .font(.system(.caption, design: .monospaced))
                .autocorrectionDisabled()
                .textInputAutocapitalization(.never)
                .accessibilityIdentifier("\(accessibilityPrefix).votingKey")
            Text("Debug builds only. The pro_tx_hash goes in wire order, the reverse of the hex Core displays.")
                .font(.caption2)
                .foregroundColor(.secondary)
        }
    }
    #endif

    /// The pick and what its row records, which decide whether its key is
    /// checked again.
    private var currentTrigger: VotingKeyCheckTrigger {
        let selected = votingIdentities.first { $0.identityId == selectedIdentityId }
        return VotingKeyCheckTrigger(
            identityId: selectedIdentityId,
            checkRaw: selected?.votingKeyCheckRaw,
            checkAddress: selected?.votingKeyCheckAddress,
            proTxHash: selected?.masternodeProTxHash
        )
    }

    private func selectDefaultIfNeeded() {
        let candidates = votingIdentities
        if selectedIdentityId == nil || !candidates.contains(where: { $0.identityId == selectedIdentityId }) {
            // Prefer an identity whose voting key may vote.
            selectedIdentityId = (candidates.first { !Self.isRefused($0) } ?? candidates.first)?.identityId
        }
        publish()
    }

    /// Checks the picked identity's stored voting key against Platform when
    /// Rust says to check it before use, and records on its row what Rust's
    /// outcome says to record, with the key's voting address. The check
    /// reads only the key's voting address
    /// (`KeychainSigner.storedMasternodeVotingAddress`, and once more before
    /// the record is written); the key is not made signable.
    ///
    /// A row with no recorded pro_tx_hash (stored before it was recorded) is
    /// not published until Rust's load rule (`SDK.loadMasternodeForVoting`)
    /// says, for the pro_tx_hash its id would be, that it is that
    /// masternode's owner identity; its pro_tx_hash and role are then
    /// recorded with the key's verdict.
    private func checkSelectedVotingKeyIfNeeded() async {
        let trigger = currentTrigger
        // Whatever the check found, or if it did not need to run, the pick
        // and record it ran for are checked; a superseded run marks nothing.
        defer {
            if !Task.isCancelled { checkedTrigger = trigger }
        }
        guard let identityId = selectedIdentityId,
              let identity = votingIdentities.first(where: { $0.identityId == identityId }),
              let sdk = appState.sdk else {
            return
        }
        let network = appState.currentNetwork
        let signer = KeychainSigner(modelContainer: modelContext.container, network: network)
        // The stored key's address first: Rust reads a record made of
        // another key as never checked. A key that cannot be read is
        // reported as such, before any rule, and its record is left alone.
        let checkedAddress: MasternodeVotingKeyAddress
        do {
            let data = try await signer.storedMasternodeVotingAddress(identityId: identityId)
            guard let address = MasternodeVotingKeyAddress(data) else {
                throw SDKError.internalError("The stored voting key gave a voting address of \(data.count) bytes")
            }
            checkedAddress = address
        } catch {
            guard !Task.isCancelled else { return }
            checkFailure = (identityId, .keyUnreadable(
                "The stored voting key could not be read: \(error.localizedDescription)"
            ))
            return
        }
        guard !Task.isCancelled else { return }
        // The key reads now, so an earlier read failure no longer holds.
        if let current = checkFailure, current.identityId == identityId,
           case .keyUnreadable = current.failure {
            checkFailure = nil
        }
        let keyUse = MasternodeRecordedVotingKeyUse(
            recorded: identity.recordedVotingKey,
            storedVotingAddress: checkedAddress
        )
        let knownProTxHash = identity.masternodeProTxHash
        guard keyUse.checkBeforeUse || knownProTxHash == nil else { return }
        checkingIdentityId = identityId
        defer {
            if checkingIdentityId == identityId { checkingIdentityId = nil }
        }
        let check: MasternodeVotingKeyCheck
        // Set when Rust's load rule resolved a row with no recorded
        // pro_tx_hash as the owner identity of this pro_tx_hash.
        var resolvedProTxHash: Data?
        do {
            if let knownProTxHash {
                check = try await sdk.checkMasternodeVotingKey(
                    proTxHash: knownProTxHash,
                    votingAddress: checkedAddress.data
                )
            } else {
                // The only pro_tx_hash this row can be the owner identity
                // of is its id as Platform names it; Rust decides whether
                // it is.
                let candidate = MasternodeVoter.wireOrderProTxHash(displayOrder: identityId)
                let load = try await sdk.loadMasternodeForVoting(
                    proTxHash: candidate,
                    votingAddress: checkedAddress
                )
                guard case .loaded(let loaded) = load, loaded.role == .owner,
                      loaded.identityId == identityId, let loadedCheck = loaded.votingKeyCheck else {
                    guard !Task.isCancelled else { return }
                    var reason = "Platform does not hold this identity as the owner identity of a masternode with this voting key."
                    if case .refused(let message, _) = load { reason = message }
                    checkFailure = (identityId, .notResolved(reason))
                    return
                }
                check = loadedCheck
                resolvedProTxHash = candidate
            }
        } catch {
            // Nothing was learned about the key; the row is left as it was.
            guard !Task.isCancelled else { return }
            let message = "The voting key could not be checked: \(error.localizedDescription)"
            // A row whose masternode is not known yet cannot vote either way.
            checkFailure = (identityId, knownProTxHash == nil ? .notResolved(message) : .checkFailed(message))
            return
        }
        // A superseded check (the pick changed, then maybe changed back, so a
        // newer check of the same identity runs) is not recorded: the query
        // resumes after cancellation and could finish out of order.
        guard !Task.isCancelled else { return }
        // An answer from another network, or for a row that is gone or no
        // longer holds a voting key, is not recorded.
        guard appState.sdk === sdk, appState.currentNetwork == network,
              let row = PersistentIdentity.fetch(in: modelContext, identityId: identityId),
              row.network == network, row.votingPrivateKeyIdentifier != nil else {
            return
        }
        // The verdict belongs to the key that was checked, and the Keychain
        // identifier names the identity and role, not the key. So the stored
        // key is read again here, and from this read to the save below there
        // is no suspension point: every writer of the voting key slot and of
        // its record (Load Identity's store and replacement, and this check)
        // runs on the main actor, so none can land in between. If another
        // window replaced the key during the check, this answer is about a key
        // that is gone and is dropped. A key that cannot be read now is
        // reported, and the answer is not saved: it cannot be tied to the key.
        let storedAddress: MasternodeVotingKeyAddress
        do {
            let data = try signer.storedMasternodeVotingAddressWithoutSuspending(identityId: identityId)
            guard let address = MasternodeVotingKeyAddress(data) else {
                throw SDKError.internalError("The stored voting key gave a voting address of \(data.count) bytes")
            }
            storedAddress = address
        } catch {
            checkFailure = (identityId, .keyUnreadable(
                "The stored voting key could not be read, so the check's result was not saved: \(error.localizedDescription)"
            ))
            return
        }
        guard storedAddress == checkedAddress else { return }
        if let resolvedProTxHash {
            row.masternodeProTxHash = resolvedProTxHash
            row.masternodeRole = .owner
        }
        // The stored key is the checked key, so the slot holds the same key
        // with what its row records now.
        let outcome = check.outcome(
            votingAddress: checkedAddress,
            slot: .sameKey(recorded: row.recordedVotingKey)
        )
        if let record = outcome.record {
            row.recordedVotingKey = record
        }
        do {
            try modelContext.save()
            if checkFailure?.identityId == identityId { checkFailure = nil }
        } catch {
            checkFailure = (identityId, .checkFailed(
                "The voting key check could not be saved: \(error.localizedDescription)"
            ))
        }
        publish()
    }

    private func publish() {
        #if DEBUG
        if useTypedCredentials {
            // The pro_tx_hash alone is enough to read the masternode's
            // answer, so the voter is published without a key while the key
            // field is empty or not a valid key; `canCast` keeps Submit
            // disabled until it is.
            guard let proTxHash = Self.decode32(proTxHashHex) else {
                voter = nil
                return
            }
            voter = .typedCredentials(
                proTxHash: proTxHash,
                votingPrivateKey: Self.decode32(votingKeyHex)
            )
            return
        }
        #endif
        if allowsTypedProTxHash && useTypedProTxHash {
            voter = Self.decode32(proTxHashHex).map { .typedProTxHash($0) }
            return
        }
        // An identity whose voting key may not vote, or whose masternode is
        // not known yet, is not published; nor is one whose key Rust says to
        // check before use until that check has run for its current record.
        guard let selected = votingIdentities.first(where: { $0.identityId == selectedIdentityId }),
              !Self.isRefused(selected),
              let proTxHash = selected.masternodeProTxHash else {
            voter = nil
            return
        }
        let keyUse = MasternodeRecordedVotingKeyUse(recorded: selected.recordedVotingKey, storedVotingAddress: nil)
        if keyUse.checkBeforeUse && checkedTrigger != currentTrigger {
            voter = nil
            return
        }
        voter = .storedKey(identityId: selected.identityId, proTxHash: proTxHash)
    }
}

/// A pick and what its row records for its voting key: a check runs again
/// when any of them changes.
private struct VotingKeyCheckTrigger: Equatable {
    let identityId: Data?
    let checkRaw: String?
    let checkAddress: Data?
    let proTxHash: Data?
}

/// How the check of the picked identity's stored voting key went, under the
/// picker. Nothing for a key Platform accepted.
/// Why the check of the picked identity's stored voting key did not happen
/// or was not saved.
private enum VotingKeyCheckFailure: Equatable {
    /// The stored key was read, but the check could not run or its result
    /// could not be saved.
    case checkFailed(String)
    /// The stored key could not be read, so it cannot sign either.
    case keyUnreadable(String)
    /// The row records no pro_tx_hash, and Rust's load rule did not resolve
    /// it as a masternode's owner identity, so it cannot vote.
    case notResolved(String)
}

private struct VotingKeyCheckStatus: View {
    let check: MasternodeVotingKeyCheck?
    let isChecking: Bool
    /// Why the check did not happen or was not saved, when that is so.
    let failure: VotingKeyCheckFailure?

    var body: some View {
        if isChecking {
            HStack(spacing: 8) {
                ProgressView()
                Text("Checking the voting key...")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        } else if case .notResolved(let message) = failure {
            Text(message + " Load the masternode again with its ProTxHash and voting key under Identities, Load Identity.")
                .font(.caption)
                .foregroundColor(.red)
        } else if case .keyUnreadable(let message) = failure {
            Text(message + " Unlock the device, or load the identity again with its voting key under Identities, Load Identity.")
                .font(.caption)
                .foregroundColor(.red)
        } else if case .invalid(let message) = check {
            Text("This voting key cannot vote for this masternode: \(message) Load the identity again with its voting key under Identities, Load Identity.")
                .font(.caption)
                .foregroundColor(.red)
        } else if case .checkFailed(let message) = failure {
            Text(message + " You can still vote with it.")
                .font(.caption)
                .foregroundColor(.secondary)
        } else if case .undetermined(let message) = check {
            Text(message + " You can still vote with it; it is checked again when you pick this identity.")
                .font(.caption)
                .foregroundColor(.secondary)
        }
    }
}
