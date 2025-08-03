import SwiftUI

struct PlatformQueriesView: View {
    @EnvironmentObject var appState: UnifiedAppState
    
    enum QueryCategory: String, CaseIterable {
        case identity = "Identity"
        case dataContract = "Data Contract"
        case documents = "Documents"
        case dpns = "DPNS"
        case voting = "Voting & Contested Resources"
        case protocolVersion = "Protocol & Version"
        case epoch = "Epoch & Block"
        case token = "Token"
        case group = "Group"
        case system = "System & Utility"
        case diagnostics = "Diagnostics"
        
        var systemImage: String {
            switch self {
            case .identity: return "person.circle"
            case .dataContract: return "doc.badge.gearshape"
            case .documents: return "doc.text"
            case .dpns: return "at"
            case .voting: return "checkmark.seal"
            case .protocolVersion: return "gearshape.2"
            case .epoch: return "clock"
            case .token: return "dollarsign.circle"
            case .group: return "person.3"
            case .system: return "gear"
            case .diagnostics: return "stethoscope"
            }
        }
        
        var description: String {
            switch self {
            case .identity: return "Fetch and manage identity information"
            case .dataContract: return "Query data contracts and their history"
            case .documents: return "Search and retrieve documents"
            case .dpns: return "Dash Platform Name Service operations"
            case .voting: return "Contested resources and voting data"
            case .protocolVersion: return "Protocol version and upgrade info"
            case .epoch: return "Epoch and block information"
            case .token: return "Token balances and information"
            case .group: return "Group management queries"
            case .system: return "System status and utilities"
            case .diagnostics: return "Test and diagnose platform queries"
            }
        }
    }
    
    var body: some View {
        List {
            ForEach(QueryCategory.allCases, id: \.self) { category in
                NavigationLink(destination: QueryCategoryDetailView(category: category)) {
                    HStack(spacing: 15) {
                        Image(systemName: category.systemImage)
                            .font(.title2)
                            .foregroundColor(.blue)
                            .frame(width: 40)
                        
                        VStack(alignment: .leading, spacing: 4) {
                            Text(category.rawValue)
                                .font(.headline)
                            Text(category.description)
                                .font(.caption)
                                .foregroundColor(.secondary)
                                .lineLimit(2)
                        }
                    }
                    .padding(.vertical, 4)
                }
            }
        }
        .navigationTitle("Queries")
        .navigationBarTitleDisplayMode(.large)
    }
}

struct QueryCategoryDetailView: View {
    let category: PlatformQueriesView.QueryCategory
    @EnvironmentObject var appState: UnifiedAppState
    
    var body: some View {
        List {
            ForEach(queries(for: category), id: \.name) { query in
                if query.name == "runAllQueries" {
                    NavigationLink(destination: DiagnosticsView()) {
                        VStack(alignment: .leading, spacing: 4) {
                            Text(query.label)
                                .font(.headline)
                            Text(query.description)
                                .font(.caption)
                                .foregroundColor(.secondary)
                                .lineLimit(2)
                        }
                        .padding(.vertical, 4)
                    }
                } else if query.name == "testDPNSQueries" {
                    NavigationLink(destination: DPNSTestView()) {
                        VStack(alignment: .leading, spacing: 4) {
                            Text(query.label)
                                .font(.headline)
                            Text(query.description)
                                .font(.caption)
                                .foregroundColor(.secondary)
                                .lineLimit(2)
                        }
                        .padding(.vertical, 4)
                    }
                } else {
                    NavigationLink(destination: QueryDetailView(query: query)) {
                        VStack(alignment: .leading, spacing: 4) {
                            Text(query.label)
                                .font(.headline)
                            Text(query.description)
                                .font(.caption)
                                .foregroundColor(.secondary)
                                .lineLimit(2)
                        }
                        .padding(.vertical, 4)
                    }
                }
            }
        }
        .navigationTitle(category.rawValue)
        .navigationBarTitleDisplayMode(.inline)
    }
    
    private func queries(for category: PlatformQueriesView.QueryCategory) -> [QueryDefinition] {
        switch category {
        case .identity:
            return [
                QueryDefinition(name: "getIdentity", label: "Get Identity", description: "Fetch an identity by its identifier"),
                QueryDefinition(name: "getIdentityKeys", label: "Get Identity Keys", description: "Retrieve keys associated with an identity"),
                QueryDefinition(name: "getIdentitiesContractKeys", label: "Get Identities Contract Keys", description: "Get keys for multiple identities related to a specific contract"),
                QueryDefinition(name: "getIdentityNonce", label: "Get Identity Nonce", description: "Get the current nonce for an identity"),
                QueryDefinition(name: "getIdentityContractNonce", label: "Get Identity Contract Nonce", description: "Get the nonce for an identity in relation to a specific contract"),
                QueryDefinition(name: "getIdentityBalance", label: "Get Identity Balance", description: "Get the credit balance of an identity"),
                QueryDefinition(name: "getIdentitiesBalances", label: "Get Identities Balances", description: "Get balances for multiple identities"),
                QueryDefinition(name: "getIdentityBalanceAndRevision", label: "Get Identity Balance and Revision", description: "Get both balance and revision number for an identity"),
                QueryDefinition(name: "getIdentityByPublicKeyHash", label: "Get Identity by Public Key Hash", description: "Find an identity by its unique public key hash"),
                QueryDefinition(name: "getIdentityByNonUniquePublicKeyHash", label: "Get Identity by Non-Unique Public Key Hash", description: "Find identities by non-unique public key hash"),
            ]
            
        case .dataContract:
            return [
                QueryDefinition(name: "getDataContract", label: "Get Data Contract", description: "Fetch a data contract by its identifier"),
                QueryDefinition(name: "getDataContractHistory", label: "Get Data Contract History", description: "Get the version history of a data contract"),
                QueryDefinition(name: "getDataContracts", label: "Get Data Contracts", description: "Fetch multiple data contracts by their identifiers")
            ]
            
        case .documents:
            return [
                QueryDefinition(name: "getDocuments", label: "Get Documents", description: "Query documents from a data contract"),
                QueryDefinition(name: "getDocument", label: "Get Document", description: "Fetch a specific document by ID")
            ]
            
        case .dpns:
            return [
                QueryDefinition(name: "getDpnsUsername", label: "Get DPNS Usernames", description: "Get DPNS usernames for an identity"),
                QueryDefinition(name: "dpnsCheckAvailability", label: "DPNS Check Availability", description: "Check if a DPNS username is available"),
                QueryDefinition(name: "dpnsResolve", label: "DPNS Resolve Name", description: "Resolve a DPNS name to an identity ID"),
                QueryDefinition(name: "dpnsSearch", label: "DPNS Search", description: "Search for DPNS names by prefix")
            ]
            
        case .voting:
            return [
                QueryDefinition(name: "getContestedResources", label: "Get Contested Resources", description: "Get list of contested resources"),
                QueryDefinition(name: "getContestedResourceVoteState", label: "Get Contested Resource Vote State", description: "Get the current vote state for a contested resource"),
                QueryDefinition(name: "getContestedResourceVotersForIdentity", label: "Get Contested Resource Voters for Identity", description: "Get voters who voted for a specific identity in a contested resource"),
                QueryDefinition(name: "getContestedResourceIdentityVotes", label: "Get Contested Resource Identity Votes", description: "Get all votes cast by a specific identity"),
                QueryDefinition(name: "getVotePollsByEndDate", label: "Get Vote Polls by End Date", description: "Get vote polls within a time range")
            ]
            
        case .protocolVersion:
            return [
                QueryDefinition(name: "getProtocolVersionUpgradeState", label: "Get Protocol Version Upgrade State", description: "Get the current state of protocol version upgrades"),
                QueryDefinition(name: "getProtocolVersionUpgradeVoteStatus", label: "Get Protocol Version Upgrade Vote Status", description: "Get voting status for protocol version upgrades")
            ]
            
        case .epoch:
            return [
                QueryDefinition(name: "getEpochsInfo", label: "Get Epochs Info", description: "Get information about epochs"),
                QueryDefinition(name: "getCurrentEpoch", label: "Get Current Epoch", description: "Get information about the current epoch"),
                QueryDefinition(name: "getFinalizedEpochInfos", label: "Get Finalized Epoch Info", description: "Get information about finalized epochs"),
                QueryDefinition(name: "getEvonodesProposedEpochBlocksByIds", label: "Get Evonodes Proposed Epoch Blocks by IDs", description: "Get proposed blocks by evonode IDs"),
                QueryDefinition(name: "getEvonodesProposedEpochBlocksByRange", label: "Get Evonodes Proposed Epoch Blocks by Range", description: "Get proposed blocks by range")
            ]
            
        case .token:
            return [
                QueryDefinition(name: "getIdentityTokenBalances", label: "Get Identity Token Balances", description: "Get token balances for an identity"),
                QueryDefinition(name: "getIdentitiesTokenBalances", label: "Get Identities Token Balances", description: "Get token balance for multiple identities"),
                QueryDefinition(name: "getIdentityTokenInfos", label: "Get Identity Token Infos", description: "Get token information for an identity's tokens"),
                QueryDefinition(name: "getIdentitiesTokenInfos", label: "Get Identities Token Infos", description: "Get token information for multiple identities"),
                QueryDefinition(name: "getTokenStatuses", label: "Get Token Statuses", description: "Get status for multiple tokens"),
                QueryDefinition(name: "getTokenDirectPurchasePrices", label: "Get Token Direct Purchase Prices", description: "Get direct purchase prices for tokens"),
                QueryDefinition(name: "getTokenContractInfo", label: "Get Token Contract Info", description: "Get information about a token contract"),
                QueryDefinition(name: "getTokenPerpetualDistributionLastClaim", label: "Get Token Perpetual Distribution Last Claim", description: "Get last claim information for perpetual distribution"),
                QueryDefinition(name: "getTokenTotalSupply", label: "Get Token Total Supply", description: "Get total supply of a token")
            ]
            
        case .group:
            return [
                QueryDefinition(name: "getGroupInfo", label: "Get Group Info", description: "Get information about a group"),
                QueryDefinition(name: "getGroupInfos", label: "Get Group Infos", description: "Get information about multiple groups"),
                QueryDefinition(name: "getGroupActions", label: "Get Group Actions", description: "Get actions for a group"),
                QueryDefinition(name: "getGroupActionSigners", label: "Get Group Action Signers", description: "Get signers for a group action")
            ]
            
        case .system:
            return [
                QueryDefinition(name: "getStatus", label: "Get Status", description: "Get system status"),
                QueryDefinition(name: "getTotalCreditsInPlatform", label: "Get Total Credits in Platform", description: "Get total credits in the platform"),
                QueryDefinition(name: "getCurrentQuorumsInfo", label: "Get Current Quorums Info", description: "Get information about current validator quorums"),
                QueryDefinition(name: "getPrefundedSpecializedBalance", label: "Get Prefunded Specialized Balance", description: "Get balance of a prefunded specialized account")
            ]
            
        case .diagnostics:
            return [
                QueryDefinition(name: "runAllQueries", label: "Run All Queries", description: "Execute all platform queries with test data to verify connectivity and functionality"),
                QueryDefinition(name: "testDPNSQueries", label: "Test DPNS Native Queries", description: "Test the new native DPNS FFI query functions")
            ]
        }
    }
}

struct QueryDefinition {
    let name: String
    let label: String
    let description: String
}