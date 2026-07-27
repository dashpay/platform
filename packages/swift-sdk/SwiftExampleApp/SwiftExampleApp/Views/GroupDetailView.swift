import SwiftUI
import SwiftData
import SwiftDashSDK

/// One member of a contract group, rendered as a row in
/// `GroupDetailView`. Carries the raw base58 id so the lookup into
/// `PersistentIdentity` can run inside the row view (driven by
/// `@Query`, which can't be parameterised at the parent level
/// without rebuilding the whole list on every contract switch).
struct GroupMember: Identifiable, Hashable {
    let memberIdBase58: String
    let power: Int

    var id: String { memberIdBase58 }
}

/// Renders one group definition pulled off
/// `PersistentDataContract.groups`. The parent (`DataContractDetailsView`)
/// extracts position + member list + required power and hands them
/// over flat — this view doesn't reach back into the contract row.
struct GroupDetailView: View {
    let contractId: Data
    let position: Int
    let members: [GroupMember]
    let requiredPower: Int

    var body: some View {
        List {
            Section("Group \(position)") {
                InfoRow(label: "Required Power:", value: "\(requiredPower)")
                InfoRow(label: "Members:", value: "\(members.count)")
            }

            Section("Members") {
                ForEach(members.sorted(by: { $0.memberIdBase58 < $1.memberIdBase58 })) { member in
                    GroupMemberRow(member: member)
                }
            }
        }
        .navigationTitle("Group \(position)")
        .navigationBarTitleDisplayMode(.inline)
    }
}

/// One member row. Resolves the base58 id against
/// `PersistentIdentity` via a targeted `@Query` so the row updates
/// reactively if the identity is loaded into the local store after
/// this view is mounted (e.g. via `LoadIdentityView`).
private struct GroupMemberRow: View {
    let member: GroupMember
    @Query private var matches: [PersistentIdentity]

    init(member: GroupMember) {
        self.member = member
        // The `@Query` predicate captures the member id bytes if we
        // can resolve them; otherwise it filters to nothing so the
        // row falls into the "Not in local store" path below.
        let memberData = Data.identifier(fromBase58: member.memberIdBase58) ?? Data()
        _matches = Query(
            filter: #Predicate<PersistentIdentity> { identity in
                identity.identityId == memberData
            }
        )
    }

    private var truncatedId: String {
        let s = member.memberIdBase58
        guard s.count > 12 else { return s }
        return "\(s.prefix(8))..\(s.suffix(4))"
    }

    var body: some View {
        if let identity = matches.first {
            NavigationLink(destination: IdentityDetailView(identityId: identity.identityId)) {
                HStack {
                    VStack(alignment: .leading, spacing: 2) {
                        Text(identity.displayName)
                            .font(.body)
                        Text(truncatedId)
                            .font(.caption2)
                            .foregroundColor(.secondary)
                            .lineLimit(1)
                            .truncationMode(.middle)
                    }
                    Spacer()
                    Text("Power \(member.power)")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
        } else {
            HStack {
                VStack(alignment: .leading, spacing: 2) {
                    Text(truncatedId)
                        .font(.body)
                        .lineLimit(1)
                        .truncationMode(.middle)
                    Text("Not in local store")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                }
                Spacer()
                Text("Power \(member.power)")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        }
    }
}
