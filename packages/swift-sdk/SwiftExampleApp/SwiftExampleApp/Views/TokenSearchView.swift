import SwiftUI
import SwiftData

struct TokenSearchView: View {
    @Query private var allTokens: [PersistentToken]
    @State private var selectedFilter: TokenFilter = .all
    @State private var searchText = ""
    
    enum TokenFilter: String, CaseIterable {
        case all = "All Tokens"
        case mintable = "Can Mint"
        case burnable = "Can Burn"
        case freezable = "Can Freeze"
        case hasDistribution = "Has Distribution"
        case paused = "Paused"
        
        var predicate: Predicate<PersistentToken>? {
            switch self {
            case .all:
                return nil
            case .mintable:
                return PersistentToken.mintableTokensPredicate()
            case .burnable:
                return PersistentToken.burnableTokensPredicate()
            case .freezable:
                return PersistentToken.freezableTokensPredicate()
            case .hasDistribution:
                return PersistentToken.distributionTokensPredicate()
            case .paused:
                return PersistentToken.pausedTokensPredicate()
            }
        }
    }
    
    var filteredTokens: [PersistentToken] {
        var tokens = allTokens
        
        // Apply control rule filter
        switch selectedFilter {
        case .mintable:
            tokens = tokens.filter { $0.canManuallyMint }
        case .burnable:
            tokens = tokens.filter { $0.canManuallyBurn }
        case .freezable:
            tokens = tokens.filter { $0.canFreeze }
        case .hasDistribution:
            tokens = tokens.filter { $0.hasDistribution }
        case .paused:
            tokens = tokens.filter { $0.isPaused }
        case .all:
            break
        }
        
        // Apply text search
        if !searchText.isEmpty {
            tokens = tokens.filter { token in
                token.name.localizedCaseInsensitiveContains(searchText) ||
                token.displayName.localizedCaseInsensitiveContains(searchText) ||
                (token.tokenDescription ?? "").localizedCaseInsensitiveContains(searchText)
            }
        }
        
        return tokens
    }
    
    var body: some View {
        VStack(spacing: 0) {
            // Search and Filter
            VStack(spacing: 12) {
                HStack {
                    Image(systemName: "magnifyingglass")
                        .foregroundColor(.secondary)
                    TextField("Search tokens...", text: $searchText)
                        .textFieldStyle(RoundedBorderTextFieldStyle())
                }
                .padding(.horizontal)
                
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 8) {
                        ForEach(TokenFilter.allCases, id: \.self) { filter in
                            FilterChip(
                                title: filter.rawValue,
                                isSelected: selectedFilter == filter,
                                action: { selectedFilter = filter }
                            )
                        }
                    }
                    .padding(.horizontal)
                }
            }
            .padding(.vertical)
            .background(Color(UIColor.systemBackground))
            
            // Results
            if filteredTokens.isEmpty {
                VStack(spacing: 20) {
                    Image(systemName: "magnifyingglass.circle")
                        .font(.system(size: 60))
                        .foregroundColor(.secondary)
                    
                    Text("No tokens found")
                        .font(.title2)
                        .fontWeight(.semibold)
                    
                    Text("Try adjusting your search or filters")
                        .font(.subheadline)
                        .foregroundColor(.secondary)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .padding()
            } else {
                List(filteredTokens) { token in
                    NavigationLink(destination: TokenDetailsView(token: token)) {
                        TokenSearchRow(token: token)
                    }
                }
                .listStyle(PlainListStyle())
            }
        }
        .navigationTitle("Token Search")
        .navigationBarTitleDisplayMode(.inline)
    }
}

struct FilterChip: View {
    let title: String
    let isSelected: Bool
    let action: () -> Void
    
    var body: some View {
        Button(action: action) {
            Text(title)
                .font(.subheadline)
                .padding(.horizontal, 16)
                .padding(.vertical, 8)
                .background(isSelected ? Color.blue : Color(UIColor.secondarySystemBackground))
                .foregroundColor(isSelected ? .white : .primary)
                .cornerRadius(20)
        }
        .buttonStyle(PlainButtonStyle())
    }
}

struct TokenSearchRow: View {
    let token: PersistentToken
    
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                VStack(alignment: .leading) {
                    Text(token.getPluralForm() ?? token.displayName)
                        .font(.headline)
                    
                    if let contract = token.dataContract {
                        Text(contract.name)
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                }
                
                Spacer()
                
                // Show capabilities
                HStack(spacing: 4) {
                    if token.canManuallyMint {
                        CapabilityBadge(icon: "plus.circle.fill", color: .green)
                    }
                    if token.canManuallyBurn {
                        CapabilityBadge(icon: "flame.fill", color: .orange)
                    }
                    if token.canFreeze {
                        CapabilityBadge(icon: "snowflake", color: .blue)
                    }
                    if token.hasDistribution {
                        CapabilityBadge(icon: "arrow.clockwise", color: .purple)
                    }
                    if token.isPaused {
                        CapabilityBadge(icon: "pause.circle.fill", color: .red)
                    }
                }
            }
            
            // Token info
            HStack {
                Text("Supply: \(token.formattedBaseSupply)")
                    .font(.caption)
                    .foregroundColor(.secondary)
                
                Spacer()
                
                if let maxSupply = token.maxSupply, maxSupply != "0" {
                    Text("Max: \(formatTokenAmount(maxSupply, decimals: token.decimals))")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
        }
        .padding(.vertical, 4)
    }
    
    private func formatTokenAmount(_ amount: String, decimals: Int) -> String {
        guard let value = Double(amount) else { return amount }
        let divisor = pow(10.0, Double(decimals))
        let actualAmount = value / divisor
        let formatter = NumberFormatter()
        formatter.numberStyle = .decimal
        formatter.maximumFractionDigits = decimals
        formatter.minimumFractionDigits = 0
        return formatter.string(from: NSNumber(value: actualAmount)) ?? amount
    }
}

struct CapabilityBadge: View {
    let icon: String
    let color: Color
    
    var body: some View {
        Image(systemName: icon)
            .font(.caption)
            .foregroundColor(color)
    }
}

// Example of using the predicate in a query
struct MintableTokensView: View {
    @Query(filter: PersistentToken.mintableTokensPredicate())
    private var mintableTokens: [PersistentToken]
    
    var body: some View {
        List(mintableTokens) { token in
            VStack(alignment: .leading) {
                Text(token.displayName)
                    .font(.headline)
                Text("Can mint new tokens")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        }
    }
}

#Preview {
    NavigationStack {
        TokenSearchView()
    }
}