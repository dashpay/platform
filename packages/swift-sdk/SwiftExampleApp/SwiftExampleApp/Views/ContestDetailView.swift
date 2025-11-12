import SwiftUI
import SwiftDashSDK

struct ContestDetailView: View {
    let contestName: String
    let contestInfo: [String: Any]
    let currentIdentityId: String
    
    @EnvironmentObject var appState: AppState
    @State private var contenders: [(id: String, votes: String, isCurrentIdentity: Bool)] = []
    @State private var abstainVotes: Int? = nil
    @State private var lockVotes: Int? = nil
    @State private var endTime: Date? = nil
    @State private var isRefreshing = false
    
    var body: some View {
        List {
            // Show refresh indicator if refreshing
            if isRefreshing {
                HStack {
                    Spacer()
                    ProgressView()
                        .progressViewStyle(CircularProgressViewStyle())
                    Text("Refreshing...")
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .padding(.leading, 8)
                    Spacer()
                }
                .padding(.vertical, 8)
            }
            
            // Contest Header
            Section("Contest Information") {
                HStack {
                    Label("Name", systemImage: "at")
                    Spacer()
                    Text(contestName)
                        .font(.headline)
                        .foregroundColor(.blue)
                }
                
                if let hasWinner = contestInfo["hasWinner"] as? Bool {
                    HStack {
                        Label("Status", systemImage: "flag.fill")
                        Spacer()
                        if hasWinner {
                            Text("Resolved")
                                .foregroundColor(.green)
                        } else {
                            Text("Voting Ongoing")
                                .foregroundColor(.orange)
                        }
                    }
                }
                
                if let endTime = endTime {
                    HStack {
                        Label("Voting Ends", systemImage: "clock")
                        Spacer()
                        VStack(alignment: .trailing, spacing: 2) {
                            Text(endTime, style: .relative)
                                .font(.caption)
                                .foregroundColor(.orange)
                            Text(endTime, format: .dateTime.month().day().hour().minute())
                                .font(.caption2)
                                .foregroundColor(.secondary)
                        }
                    }
                    
                    // Show time remaining as progress if contest is active
                    if let hasWinner = contestInfo["hasWinner"] as? Bool, !hasWinner {
                        VStack(spacing: 4) {
                            GeometryReader { geometry in
                                ZStack(alignment: .leading) {
                                    Rectangle()
                                        .fill(Color.gray.opacity(0.2))
                                        .frame(height: 4)
                                        .cornerRadius(2)
                                    
                                    Rectangle()
                                        .fill(timeRemainingColor(for: endTime))
                                        .frame(width: progressWidth(for: endTime, in: geometry.size.width), height: 4)
                                        .cornerRadius(2)
                                        .animation(.easeInOut, value: endTime)
                                }
                            }
                            .frame(height: 4)
                            
                            Text(timeRemainingText(for: endTime))
                                .font(.caption2)
                                .foregroundColor(.secondary)
                        }
                        .padding(.top, 4)
                    }
                }
            }
            
            // Contenders Section
            Section("Contenders") {
                // Show special message if this is a newly registered contest
                // Check: only one contender, it's us, AND the contest was started very recently
                if contenders.count == 1 && contenders.first?.isCurrentIdentity == true {
                    // Calculate how long the contest has been running
                    let totalDuration: TimeInterval = appState.currentNetwork == .mainnet ?
                        (14 * 24 * 60 * 60) : // 14 days for mainnet
                        (90 * 60) // 90 minutes for testnet
                    
                    let timeRemaining = endTime?.timeIntervalSinceNow ?? 0
                    let elapsedTime = totalDuration - timeRemaining
                    
                    // Only show "newly registered" if less than 5% of total time has elapsed
                    // For testnet (90 min): show if less than 4.5 minutes elapsed
                    // For mainnet (14 days): show if less than ~17 hours elapsed
                    let isNewlyRegistered = elapsedTime < (totalDuration * 0.05)
                    
                    if isNewlyRegistered {
                        VStack(alignment: .leading, spacing: 8) {
                            HStack {
                                Image(systemName: "sparkles")
                                    .foregroundColor(.yellow)
                                Text("Newly Registered Contest")
                                    .font(.headline)
                                    .foregroundColor(.primary)
                            }
                            Text("You just started this contest! Other users can join as contenders until the halfway point.")
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                        .padding(.vertical, 4)
                    } else {
                        // Show a different message for contests where you're the only contender but it's not new
                        VStack(alignment: .leading, spacing: 8) {
                            HStack {
                                Image(systemName: "person.fill")
                                    .foregroundColor(.blue)
                                Text("Only Contender")
                                    .font(.headline)
                                    .foregroundColor(.primary)
                            }
                            Text("You are currently the only contender for this name. Other users can still join until the halfway point.")
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                        .padding(.vertical, 4)
                    }
                }
                
                ForEach(contenders, id: \.id) { contender in
                    VStack(alignment: .leading, spacing: 8) {
                        HStack {
                            if contender.isCurrentIdentity {
                                Label("You", systemImage: "person.fill")
                                    .font(.caption)
                                    .foregroundColor(.blue)
                            }
                            Text(contender.id)
                                .font(.system(.caption, design: .monospaced))
                                .lineLimit(1)
                                .truncationMode(.middle)
                        }
                        
                        HStack {
                            Label("Votes", systemImage: "hand.thumbsup.fill")
                                .font(.caption)
                                .foregroundColor(.secondary)
                            Spacer()
                            Text(formatVotes(contender.votes))
                                .font(.caption)
                                .foregroundColor(.primary)
                        }
                    }
                    .padding(.vertical, 4)
                }
            }
            
            // Vote Tallies Section - Always show to give complete picture
            Section("Vote Summary") {
                HStack {
                    Label("Abstain Votes", systemImage: "minus.circle")
                        .foregroundColor(.gray)
                    Spacer()
                    Text("\(abstainVotes ?? 0)")
                        .font(.headline)
                        .foregroundColor(abstainVotes ?? 0 > 0 ? .orange : .secondary)
                }
                
                HStack {
                    Label("Lock Votes", systemImage: "lock.fill")
                        .foregroundColor(.red)
                    Spacer()
                    Text("\(lockVotes ?? 0)")
                        .font(.headline)
                        .foregroundColor(lockVotes ?? 0 > 0 ? .red : .secondary)
                }
                
                // Add a divider and total vote count
                Divider()
                
                HStack {
                    Label("Total Votes", systemImage: "sum")
                        .foregroundColor(.primary)
                        .font(.headline)
                    Spacer()
                    Text("\(getTotalVotes())")
                        .font(.headline)
                        .foregroundColor(.primary)
                }
            }
            
            // Info Section
            Section {
                VStack(alignment: .leading, spacing: 8) {
                    Text("About Contested Names")
                        .font(.headline)
                    Text("When multiple identities want the same DPNS username, masternodes vote to decide the winner. The identity with the most votes will be awarded the name when voting ends.")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                .padding(.vertical, 4)
            }
        }
        .navigationTitle("Contest Details")
        .navigationBarTitleDisplayMode(.inline)
        .refreshable {
            await refreshVoteState()
        }
        .onAppear {
            parseContestInfo()
        }
    }
    
    private func parseContestInfo() {
        // Parse contenders
        if let contendersArray = contestInfo["contenders"] as? [[String: Any]] {
            contenders = contendersArray.compactMap { contenderDict in
                guard let id = contenderDict["identifier"] as? String,
                      let votes = contenderDict["votes"] as? String else {
                    return nil
                }
                
                let isCurrentIdentity = contenderDict["isQueriedIdentity"] as? Bool ?? false ||
                                       id == currentIdentityId
                
                return (id: id, votes: votes, isCurrentIdentity: isCurrentIdentity)
            }
            
            // Sort contenders by vote count (if we can parse them)
            contenders.sort { first, second in
                // Try to extract numeric vote count for sorting
                let firstVotes = extractVoteCount(from: first.votes)
                let secondVotes = extractVoteCount(from: second.votes)
                return firstVotes > secondVotes
            }
        }
        
        // Parse vote tallies
        abstainVotes = contestInfo["abstainVotes"] as? Int
        lockVotes = contestInfo["lockVotes"] as? Int
        
        // Parse end time (milliseconds since epoch)
        // Check for various numeric types since it could be stored as UInt64, Double, or Int
        if let endTimeMillis = contestInfo["endTime"] as? UInt64 {
            endTime = Date(timeIntervalSince1970: Double(endTimeMillis) / 1000.0)
        } else if let endTimeMillis = contestInfo["endTime"] as? Double {
            endTime = Date(timeIntervalSince1970: endTimeMillis / 1000.0)
        } else if let endTimeMillis = contestInfo["endTime"] as? Int {
            endTime = Date(timeIntervalSince1970: Double(endTimeMillis) / 1000.0)
        }
        
        // Debug logging
        print("🔵 Contest endTime parsing - contestInfo[endTime]: \(String(describing: contestInfo["endTime"])), parsed date: \(String(describing: endTime))")
    }
    
    private func formatVotes(_ votesString: String) -> String {
        // The votes string comes in format like "ResourceVote { vote_choice: TowardsIdentity(...), strength: 1 }"
        // Try to extract the strength value
        if let strengthRange = votesString.range(of: "strength: "),
           let endRange = votesString[strengthRange.upperBound...].range(of: " }") {
            let strengthValue = String(votesString[strengthRange.upperBound..<endRange.lowerBound])
            return "\(strengthValue) vote\(strengthValue == "1" ? "" : "s")"
        }
        
        // If we can't parse it, just show a simplified version
        if votesString.contains("TowardsIdentity") {
            return "Supporting"
        } else if votesString.contains("Abstain") {
            return "Abstain"
        } else if votesString.contains("Lock") {
            return "Lock"
        }
        
        return "Unknown"
    }
    
    private func extractVoteCount(from votesString: String) -> Int {
        // Try to extract the strength value as an integer
        if let strengthRange = votesString.range(of: "strength: "),
           let endRange = votesString[strengthRange.upperBound...].range(of: " }") {
            let strengthValue = String(votesString[strengthRange.upperBound..<endRange.lowerBound])
            return Int(strengthValue) ?? 0
        }
        return 0
    }
    
    private func getTotalVotes() -> Int {
        // Sum up all votes: contender votes + abstain + lock
        let contenderVotes = contenders.reduce(0) { total, contender in
            total + extractVoteCount(from: contender.votes)
        }
        let abstain = abstainVotes ?? 0
        let lock = lockVotes ?? 0
        return contenderVotes + abstain + lock
    }
    
    private func timeRemainingColor(for endTime: Date) -> Color {
        let timeRemaining = endTime.timeIntervalSinceNow
        let oneDay: TimeInterval = 24 * 60 * 60
        
        if timeRemaining < 0 {
            return .red // Expired
        } else if timeRemaining < oneDay {
            return .orange // Less than 24 hours
        } else if timeRemaining < oneDay * 3 {
            return .yellow // Less than 3 days
        } else {
            return .green // More than 3 days
        }
    }
    
    private func progressWidth(for endTime: Date, in totalWidth: CGFloat) -> CGFloat {
        // Get total duration based on network
        let totalDuration: TimeInterval = appState.currentNetwork == .mainnet ?
            (14 * 24 * 60 * 60) : // 14 days for mainnet
            (90 * 60) // 90 minutes for testnet
        
        // Calculate elapsed time
        let timeRemaining = max(0, endTime.timeIntervalSinceNow)
        let elapsedTime = totalDuration - timeRemaining
        
        // Calculate progress (how much time has passed)
        let progress = min(1.0, max(0, elapsedTime / totalDuration))
        
        return totalWidth * CGFloat(progress)
    }
    
    private func timeRemainingText(for endTime: Date) -> String {
        let timeRemaining = endTime.timeIntervalSinceNow
        
        if timeRemaining < 0 {
            return "Contest has ended"
        }
        
        let formatter = DateComponentsFormatter()
        formatter.allowedUnits = [.day, .hour, .minute]
        formatter.unitsStyle = .abbreviated
        formatter.maximumUnitCount = 2
        
        if let formattedTime = formatter.string(from: timeRemaining) {
            return "Time remaining: \(formattedTime)"
        }
        
        return "Contest ending soon"
    }
    
    private func refreshVoteState() async {
        guard let sdk = appState.sdk else { return }
        
        // Don't refresh if already refreshing
        guard !isRefreshing else { return }
        
        await MainActor.run {
            isRefreshing = true
        }
        
        do {
            // Call the SDK to get the latest vote state for this contested name
            let voteState = try await sdk.dpnsGetContestedVoteState(name: contestName, limit: 100)
            
            await MainActor.run {
                // Parse the updated vote state
                var newContenders: [(id: String, votes: String, isCurrentIdentity: Bool)] = []
                
                if let contendersArray = voteState["contenders"] as? [[String: Any]] {
                    newContenders = contendersArray.compactMap { contenderDict in
                        guard let id = contenderDict["identifier"] as? String,
                              let votes = contenderDict["votes"] as? String else {
                            return nil
                        }
                        
                        let isCurrentIdentity = id == currentIdentityId
                        
                        return (id: id, votes: votes, isCurrentIdentity: isCurrentIdentity)
                    }
                    
                    // Sort contenders by vote count
                    newContenders.sort { first, second in
                        let firstVotes = extractVoteCount(from: first.votes)
                        let secondVotes = extractVoteCount(from: second.votes)
                        return firstVotes > secondVotes
                    }
                }
                
                // Update vote tallies
                if let abstain = voteState["abstainVotes"] as? Int {
                    abstainVotes = abstain
                }
                if let lock = voteState["lockVotes"] as? Int {
                    lockVotes = lock
                }
                
                // Update contenders
                contenders = newContenders
                
                // Update the identity's contested info if we have access
                if let identityIndex = appState.identities.firstIndex(where: { $0.idString == currentIdentityId }) {
                    var updatedIdentity = appState.identities[identityIndex]
                    
                    // Update the contest info for this name
                    var updatedContestInfo = updatedIdentity.contestedDpnsInfo[contestName] as? [String: Any] ?? [:]
                    updatedContestInfo["contenders"] = voteState["contenders"]
                    updatedContestInfo["abstainVotes"] = abstainVotes
                    updatedContestInfo["lockVotes"] = lockVotes
                    
                    // Check if there's a winner
                    if let winner = voteState["winner"] {
                        updatedContestInfo["hasWinner"] = !(winner is NSNull)
                    }
                    
                    updatedIdentity.contestedDpnsInfo[contestName] = updatedContestInfo
                    appState.identities[identityIndex] = updatedIdentity
                    
                    // Persist the update
                    appState.updateIdentityDPNSNames(
                        id: updatedIdentity.id,
                        dpnsNames: updatedIdentity.dpnsNames,
                        contestedNames: updatedIdentity.contestedDpnsNames,
                        contestedInfo: updatedIdentity.contestedDpnsInfo
                    )
                }
                
                isRefreshing = false
            }
        } catch {
            await MainActor.run {
                isRefreshing = false
                print("Failed to refresh vote state: \(error)")
                // Could show an error alert here if desired
            }
        }
    }
}