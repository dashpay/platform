import SwiftUI
import SwiftDashSDK

struct ContestDetailView: View {
    let contestName: String
    let contestInfo: [String: Any]
    let currentIdentityId: String
    
    @State private var contenders: [(id: String, votes: String, isCurrentIdentity: Bool)] = []
    @State private var abstainVotes: Int? = nil
    @State private var lockVotes: Int? = nil
    @State private var endTime: Date? = nil
    
    var body: some View {
        List {
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
                // Show special message if this is a newly registered contest (only one contender and it's us)
                if contenders.count == 1 && contenders.first?.isCurrentIdentity == true {
                    VStack(alignment: .leading, spacing: 8) {
                        HStack {
                            Image(systemName: "sparkles")
                                .foregroundColor(.yellow)
                            Text("Newly Registered Contest")
                                .font(.headline)
                                .foregroundColor(.primary)
                        }
                        Text("You just started this contest! Other users can join as contenders until voting begins.")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                    .padding(.vertical, 4)
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
            
            // Vote Tallies Section
            if abstainVotes != nil || lockVotes != nil {
                Section("Other Votes") {
                    if let abstain = abstainVotes {
                        HStack {
                            Label("Abstain Votes", systemImage: "minus.circle")
                            Spacer()
                            Text("\(abstain)")
                                .foregroundColor(.secondary)
                        }
                    }
                    
                    if let lock = lockVotes {
                        HStack {
                            Label("Lock Votes", systemImage: "lock.fill")
                            Spacer()
                            Text("\(lock)")
                                .foregroundColor(.red)
                        }
                    }
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
        // For contests, we don't know the start time, so we'll just show time remaining
        // Assume a 14-day contest for mainnet, 90-minute contest for testnet
        let totalDuration: TimeInterval = 14 * 24 * 60 * 60 // Default to 14 days
        let timeRemaining = max(0, endTime.timeIntervalSinceNow)
        let progress = min(1.0, timeRemaining / totalDuration)
        
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
}