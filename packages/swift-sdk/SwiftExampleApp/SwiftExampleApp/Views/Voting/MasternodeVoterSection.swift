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
    /// key (Identities, Load Identity). The key stays in the Keychain and
    /// signs through `KeychainSigner`.
    case storedKey(identityId: Data)
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
        case .storedKey(let identityId):
            return Self.wireOrderProTxHash(identityId: identityId)
        case .typedProTxHash(let proTxHash):
            return proTxHash
        #if DEBUG
        case .typedCredentials(let proTxHash, _):
            return proTxHash
        #endif
        }
    }

    /// A loaded masternode identity's id is the pro_tx_hash as Platform names
    /// it, the hex Core displays. The vote FFIs take the wire order, its
    /// reverse, and reverse it back in Rust (see "pro_tx_hash byte order" in
    /// `rs-sdk-ffi/src/voting/masternode_vote.rs`). This is the only place
    /// the app converts between the two.
    static func wireOrderProTxHash(identityId: Data) -> Data {
        Data(identityId.reversed())
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
        case .storedKey(let identityId):
            let proTxHash = voter.wireOrderProTxHash
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
struct MasternodeVoterSection: View {
    let accessibilityPrefix: String
    @Binding var voter: MasternodeVoter?
    /// Offer a typed pro_tx_hash for reading an answer. Set by the sheets
    /// that have a "read this masternode's answer" action.
    var allowsTypedProTxHash: Bool = false

    @EnvironmentObject private var appState: AppState
    @Query private var identities: [PersistentIdentity]

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
            Text("The vote is signed with the voting key stored when the identity was loaded. The key stays in the Keychain.")
        }
        .onAppear(perform: selectDefaultIfNeeded)
        // `@Query` updates while the sheet is open can add or remove an
        // eligible identity; keep the selection on one that still is.
        .onChange(of: votingIdentities.map(\.identityId)) { _, _ in selectDefaultIfNeeded() }
        // A network switch changes which identities are eligible even when
        // the id list compares equal; re-check the selection.
        .onChange(of: appState.currentNetwork) { _, _ in selectDefaultIfNeeded() }
        .onChange(of: selectedIdentityId) { _, _ in publish() }
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
                    voterLabel(identity)
                        .tag(Optional(identity.identityId))
                }
            }
            .accessibilityIdentifier("\(accessibilityPrefix).voter")
            .disabled(usesTypedInput)
        }
    }

    private func voterLabel(_ identity: PersistentIdentity) -> some View {
        Text("\(identity.displayName) (\(identity.identityTypeEnum.rawValue))")
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

    private func selectDefaultIfNeeded() {
        let candidates = votingIdentities
        if selectedIdentityId == nil || !candidates.contains(where: { $0.identityId == selectedIdentityId }) {
            selectedIdentityId = candidates.first?.identityId
        }
        publish()
    }

    private func publish() {
        #if DEBUG
        if useTypedCredentials {
            // The pro_tx_hash alone is enough to read the masternode's
            // answer, so the voter is published without a key while the key
            // field is empty. A key that is typed but invalid publishes
            // nothing.
            guard let proTxHash = Self.decode32(proTxHashHex) else {
                voter = nil
                return
            }
            if votingKeyHex.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                voter = .typedCredentials(proTxHash: proTxHash, votingPrivateKey: nil)
            } else if let votingKey = Self.decode32(votingKeyHex) {
                voter = .typedCredentials(proTxHash: proTxHash, votingPrivateKey: votingKey)
            } else {
                voter = nil
            }
            return
        }
        #endif
        if allowsTypedProTxHash && useTypedProTxHash {
            voter = Self.decode32(proTxHashHex).map { .typedProTxHash($0) }
            return
        }
        voter = selectedIdentityId.map { .storedKey(identityId: $0) }
    }
}
