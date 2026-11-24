import SwiftUI

/// The state of a proposal's poll in a listing row, as Platform reported it
/// on the row's own read. The listings carry no poll state, and the device
/// clock does not decide it either: only a read of the poll does.
enum ProposalPollBadgeState: Equatable {
    /// A multiple choice proposal no choice was entered for: its first
    /// choice opens the vote, so nothing runs yet.
    case awaitingFirstChoice
    /// Platform reports no result yet.
    case voting
    /// Platform reports a result; `detail` adds how it ended, when known.
    case closed(detail: String?)
}

/// A small capsule showing ``ProposalPollBadgeState``; nothing while the
/// state is unknown (loading, failed, or a poll that was never opened).
/// It is the state at the row's last read: the rows read again when the
/// list is refreshed, not on a timer.
struct ProposalPollStateBadge: View {
    let state: ProposalPollBadgeState?
    let accessibilityIdentifier: String

    var body: some View {
        if let state {
            Label(text(for: state), systemImage: icon(for: state))
                // In a List row the automatic style drops the title and
                // leaves a tall, icon-only capsule; the words are the badge.
                .labelStyle(.titleAndIcon)
                .font(.caption2.weight(.semibold))
                .padding(.horizontal, 6)
                .padding(.vertical, 2)
                .background(color(for: state).opacity(0.15))
                .foregroundColor(color(for: state))
                .clipShape(Capsule())
                .fixedSize()
                .accessibilityIdentifier(accessibilityIdentifier)
        }
    }

    private func text(for state: ProposalPollBadgeState) -> String {
        switch state {
        case .awaitingFirstChoice:
            return "No choices yet"
        case .voting:
            return "Voting"
        case .closed(let detail?):
            return "Closed: \(detail)"
        case .closed(nil):
            return "Closed"
        }
    }

    private func icon(for state: ProposalPollBadgeState) -> String {
        switch state {
        case .awaitingFirstChoice:
            return "tray"
        case .voting:
            return "hourglass"
        case .closed:
            return "lock.fill"
        }
    }

    private func color(for state: ProposalPollBadgeState) -> Color {
        switch state {
        case .voting:
            return .blue
        case .awaitingFirstChoice, .closed:
            return .secondary
        }
    }
}

/// A proposal listing row's title with its ``ProposalPollStateBadge``:
/// the badge sits beside the title, and below it at accessibility text
/// sizes, where a wide badge such as "Closed: did not pass" would leave
/// the two-line title too little room to identify the proposal.
struct ProposalRowHeader: View {
    let title: String
    let pollState: ProposalPollBadgeState?
    let badgeIdentifier: String

    @Environment(\.dynamicTypeSize) private var dynamicTypeSize

    var body: some View {
        if dynamicTypeSize.isAccessibilitySize {
            VStack(alignment: .leading, spacing: 4) {
                titleText
                badge
            }
        } else {
            HStack(alignment: .firstTextBaseline) {
                titleText
                Spacer(minLength: 8)
                badge
            }
        }
    }

    private var titleText: some View {
        Text(title)
            .font(.headline)
            .lineLimit(2)
    }

    private var badge: some View {
        ProposalPollStateBadge(state: pollState, accessibilityIdentifier: badgeIdentifier)
    }
}
