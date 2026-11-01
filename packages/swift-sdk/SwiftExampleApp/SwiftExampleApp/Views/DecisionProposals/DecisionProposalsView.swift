import SwiftUI
import SwiftData
import SwiftDashSDK

/// Decision proposals (protocol version 15): yes/no questions any identity
/// puts to the masternodes.
///
/// Lists them three ways, each one Platform listing read a page at a time:
/// the open ones closing soonest first, the ones a local identity filed, and
/// the newest. Drills into ``DecisionProposalDetailView`` for the poll state,
/// the masternode vote and the edit, and creates new ones with
/// ``CreateDecisionProposalView``. Every read goes through the `SDK`
/// decision proposal wrappers; nothing here decides what a proposal is.
struct DecisionProposalsView: View {
    @EnvironmentObject var appState: AppState
    @EnvironmentObject var walletManager: PlatformWalletManager
    @Query private var identities: [PersistentIdentity]

    private enum Listing: String, CaseIterable, Identifiable {
        case open = "Open"
        case mine = "Mine"
        case newest = "Newest"

        var id: String { rawValue }

        var footer: String {
            switch self {
            case .open:
                return "Proposals whose voting closes from now on, closing soonest first."
            case .mine:
                return "Proposals the selected identity filed, in the order it filed them."
            case .newest:
                return "Every proposal, newest first."
            }
        }
    }

    /// Proposals per page.
    private static let pageSize: UInt32 = 20

    @State private var listing: Listing = .open
    /// Base58 id of the identity whose proposals "Mine" lists.
    @State private var selectedOwnerId = ""
    @State private var proposals: [DecisionProposal] = []
    /// `false` once a page came back shorter than the page size (the last).
    @State private var hasMore = false
    @State private var isLoading = false
    @State private var errorMessage: String?
    /// Bumped on every fresh load so a page that arrives late for a previous
    /// listing is dropped instead of mixed in.
    @State private var loadGeneration = 0
    @State private var showCreate = false

    /// Identities on the active network, for the "Mine" listing.
    private var localIdentities: [PersistentIdentity] {
        identities.filter { $0.network == appState.currentNetwork }
    }

    var body: some View {
        List {
            Section {
                Picker("Listing", selection: $listing) {
                    ForEach(Listing.allCases) { value in
                        Text(value.rawValue).tag(value)
                    }
                }
                .pickerStyle(.segmented)
                .accessibilityIdentifier("decisionProposals.listing")

                if listing == .mine {
                    Picker("Identity", selection: $selectedOwnerId) {
                        Text("Select identity").tag("")
                        ForEach(localIdentities) { identity in
                            Text(identity.alias ?? identity.identityIdBase58)
                                .tag(identity.identityIdBase58)
                        }
                    }
                    .accessibilityIdentifier("decisionProposals.ownerPicker")
                }
            } footer: {
                Text(listing.footer)
            }

            if let errorMessage {
                Section {
                    Label(errorMessage, systemImage: "exclamationmark.triangle")
                        .font(.caption)
                        .foregroundColor(.orange)
                }
            }

            Section("Proposals") {
                if proposals.isEmpty {
                    if isLoading {
                        HStack {
                            ProgressView()
                            Text("Loading proposals…")
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                    } else if listing == .mine && selectedOwnerId.isEmpty {
                        Text("Select an identity to list its proposals.")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    } else {
                        Text("No proposals")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                } else {
                    ForEach(proposals) { proposal in
                        NavigationLink(
                            destination: DecisionProposalDetailView(proposal: proposal)
                        ) {
                            DecisionProposalRow(proposal: proposal)
                        }
                        .accessibilityIdentifier("decisionProposals.row.\(proposal.id)")
                    }

                    if hasMore {
                        Button {
                            Task { await loadNextPage() }
                        } label: {
                            HStack {
                                if isLoading {
                                    ProgressView().controlSize(.small)
                                }
                                Text("Load More")
                            }
                            .frame(maxWidth: .infinity)
                        }
                        .disabled(isLoading)
                        .accessibilityIdentifier("decisionProposals.loadMore")
                    }
                }
            }
        }
        .navigationTitle("Decision Proposals")
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .navigationBarTrailing) {
                Button {
                    showCreate = true
                } label: {
                    Image(systemName: "plus")
                }
                .disabled(appState.sdk == nil)
                .accessibilityLabel("New decision proposal")
                .accessibilityIdentifier("decisionProposals.create")
            }
        }
        .refreshable {
            await reload()
        }
        .task(id: "\(listing.rawValue)|\(selectedOwnerId)") {
            await reload()
        }
        .onChange(of: listing) { _, newListing in
            // Start "Mine" on the first local identity rather than empty.
            if newListing == .mine, selectedOwnerId.isEmpty,
               let first = localIdentities.first {
                selectedOwnerId = first.identityIdBase58
            }
        }
        .sheet(isPresented: $showCreate) {
            CreateDecisionProposalView { _ in
                Task { await reload() }
            }
            .environmentObject(appState)
            .environmentObject(walletManager)
        }
    }

    // MARK: - Loading

    /// Load the first page of the current listing.
    private func reload() async {
        loadGeneration += 1
        proposals = []
        hasMore = false
        errorMessage = nil
        if listing == .mine && selectedOwnerId.isEmpty { return }
        await load(startAfter: nil, generation: loadGeneration)
    }

    /// Load the page after the last proposal shown.
    private func loadNextPage() async {
        guard let last = proposals.last else { return }
        await load(startAfter: last.id, generation: loadGeneration)
    }

    private func load(startAfter: String?, generation: Int) async {
        guard let sdk = appState.sdk else {
            errorMessage = "SDK not initialized"
            return
        }
        isLoading = true
        defer { isLoading = false }

        do {
            let page: [DecisionProposal]
            switch listing {
            case .open:
                page = try await sdk.decisionProposalsByEndDate(
                    from: Date(),
                    limit: Self.pageSize,
                    startAfter: startAfter
                )
            case .mine:
                page = try await sdk.decisionProposalsByOwner(
                    ownerId: selectedOwnerId,
                    limit: Self.pageSize,
                    startAfter: startAfter
                )
            case .newest:
                page = try await sdk.decisionProposalsByCreatedAt(
                    limit: Self.pageSize,
                    startAfter: startAfter
                )
            }
            guard generation == loadGeneration else { return }
            proposals.append(contentsOf: page)
            // A page shorter than the page size is the last one.
            hasMore = page.count == Int(Self.pageSize)
        } catch {
            guard generation == loadGeneration else { return }
            errorMessage = "Loading failed: \(error.localizedDescription)"
        }
    }
}

/// One proposal in a listing.
private struct DecisionProposalRow: View {
    let proposal: DecisionProposal

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(proposal.title)
                .font(.headline)
                .lineLimit(2)
            HStack(spacing: 12) {
                Label {
                    Text(proposal.endDate, style: proposal.endDate > Date() ? .relative : .date)
                } icon: {
                    Image(systemName: "clock")
                }
                Label("\(proposal.passPercent)%", systemImage: "checkmark.circle")
                Label("\(proposal.minimumTurnoutPercent)%", systemImage: "person.3")
            }
            .font(.caption)
            .foregroundColor(.secondary)
            Text(proposal.id)
                .font(.system(.caption2, design: .monospaced))
                .foregroundColor(.secondary)
                .lineLimit(1)
                .truncationMode(.middle)
        }
        .padding(.vertical, 2)
    }
}
