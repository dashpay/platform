#!/usr/bin/env python3
"""
Check that all NEW gRPC queries from platform.proto are implemented in rs-sdk.
Uses a cache to track known queries and only checks new ones.
"""

import os
import re
import sys
import json
from pathlib import Path
from datetime import datetime

# Queries that should be excluded from the check
EXCLUDED_QUERIES = {
    'broadcastStateTransition',  # Explicitly excluded as per requirement
}

# Mapping of proto query names to their expected SDK implementations
# This can be extended if the SDK uses different naming conventions
QUERY_MAPPINGS = {
    # Identity queries
    'getIdentity': ['Identity::fetch', 'fetch.*identity', 'GetIdentityRequest'],
    'getIdentityKeys': ['identity.*keys', 'GetIdentityKeysRequest'],
    'getIdentitiesContractKeys': ['identities.*contract.*keys', 'GetIdentitiesContractKeysRequest'],
    'getIdentityNonce': ['identity.*nonce', 'GetIdentityNonceRequest'],
    'getIdentityContractNonce': ['identity.*contract.*nonce', 'GetIdentityContractNonceRequest'],
    'getIdentityBalance': ['identity.*balance', 'GetIdentityBalanceRequest'],
    'getIdentitiesBalances': ['identities.*balances', 'GetIdentitiesBalancesRequest'],
    'getIdentityBalanceAndRevision': ['identity.*balance.*revision', 'GetIdentityBalanceAndRevisionRequest'],
    'getIdentityByPublicKeyHash': ['identity.*public.*key.*hash', 'GetIdentityByPublicKeyHashRequest'],
    'getIdentityByNonUniquePublicKeyHash': ['identity.*non.*unique.*public.*key.*hash', 'GetIdentityByNonUniquePublicKeyHashRequest'],
    
    # Data Contract queries
    'getDataContract': ['DataContract::fetch', 'fetch.*data.*contract', 'GetDataContractRequest'],
    'getDataContractHistory': ['data.*contract.*history', 'GetDataContractHistoryRequest'],
    'getDataContracts': ['data.*contracts', 'GetDataContractsRequest'],
    
    # Document queries
    'getDocuments': ['Document::fetch', 'fetch.*documents?', 'GetDocumentsRequest'],
    
    # Epoch/Evonode queries
    'getEvonodesProposedEpochBlocksByIds': ['evonodes.*proposed.*epoch.*blocks.*ids', 'GetEvonodesProposedEpochBlocksByIdsRequest'],
    'getEvonodesProposedEpochBlocksByRange': ['evonodes.*proposed.*epoch.*blocks.*range', 'GetEvonodesProposedEpochBlocksByRangeRequest'],
    'getEpochsInfo': ['epochs.*info', 'GetEpochsInfoRequest'],
    'getFinalizedEpochInfos': ['finalized.*epoch.*infos', 'GetFinalizedEpochInfosRequest'],
    
    # State transition queries
    'waitForStateTransitionResult': ['wait.*state.*transition', 'WaitForStateTransitionResultRequest'],
    
    # Protocol/consensus queries
    'getConsensusParams': ['consensus.*params', 'GetConsensusParamsRequest', 'get_consensus_params'],
    'getProtocolVersionUpgradeState': ['protocol.*version.*upgrade.*state', 'GetProtocolVersionUpgradeStateRequest'],
    'getProtocolVersionUpgradeVoteStatus': ['protocol.*version.*upgrade.*vote.*status', 'GetProtocolVersionUpgradeVoteStatusRequest'],
    
    # Contested resource queries
    'getContestedResources': ['contested.*resources', 'GetContestedResourcesRequest'],
    'getContestedResourceVoteState': ['contested.*resource.*vote.*state', 'GetContestedResourceVoteStateRequest'],
    'getContestedResourceVotersForIdentity': ['contested.*resource.*voters.*identity', 'GetContestedResourceVotersForIdentityRequest'],
    'getContestedResourceIdentityVotes': ['contested.*resource.*identity.*votes', 'GetContestedResourceIdentityVotesRequest'],
    
    # Vote queries
    'getVotePollsByEndDate': ['vote.*polls.*end.*date', 'GetVotePollsByEndDateRequest'],
    
    # System queries
    'getPrefundedSpecializedBalance': ['prefunded.*specialized.*balance', 'GetPrefundedSpecializedBalanceRequest'],
    'getTotalCreditsInPlatform': ['total.*credits.*platform', 'GetTotalCreditsInPlatformRequest'],
    'getPathElements': ['path.*elements', 'GetPathElementsRequest'],
    'getStatus': ['get.*status', 'GetStatusRequest'],
    'getCurrentQuorumsInfo': ['current.*quorums.*info', 'GetCurrentQuorumsInfoRequest'],
    
    # Token queries
    'getIdentityTokenBalances': ['identity.*token.*balances', 'GetIdentityTokenBalancesRequest'],
    'getIdentitiesTokenBalances': ['identities.*token.*balances', 'GetIdentitiesTokenBalancesRequest'],
    'getIdentityTokenInfos': ['identity.*token.*infos', 'GetIdentityTokenInfosRequest'],
    'getIdentitiesTokenInfos': ['identities.*token.*infos', 'GetIdentitiesTokenInfosRequest'],
    'getTokenStatuses': ['token.*statuses', 'GetTokenStatusesRequest'],
    'getTokenDirectPurchasePrices': ['token.*direct.*purchase.*prices', 'GetTokenDirectPurchasePricesRequest'],
    'getTokenContractInfo': ['token.*contract.*info', 'GetTokenContractInfoRequest', 'get_token_contract_info'],
    'getTokenPreProgrammedDistributions': ['token.*pre.*programmed.*distributions', 'GetTokenPreProgrammedDistributionsRequest', 'get_token_pre_programmed_distributions'],
    'getTokenPerpetualDistributionLastClaim': ['token.*perpetual.*distribution.*last.*claim', 'GetTokenPerpetualDistributionLastClaimRequest'],
    'getTokenTotalSupply': ['token.*total.*supply', 'GetTokenTotalSupplyRequest'],
    
    # Group queries
    'getGroupInfo': ['group.*info', 'GetGroupInfoRequest'],
    'getGroupInfos': ['group.*infos', 'GetGroupInfosRequest'],
    'getGroupActions': ['group.*actions', 'GetGroupActionsRequest'],
    'getGroupActionSigners': ['group.*action.*signers', 'GetGroupActionSignersRequest'],
}


def extract_grpc_queries(proto_file):
    """Extract all RPC method names from the platform.proto file."""
    queries = []
    
    with open(proto_file, 'r') as f:
        content = f.read()
    
    # Find service Platform block
    service_match = re.search(r'service\s+Platform\s*{(.*?)^}', content, re.DOTALL | re.MULTILINE)
    if not service_match:
        print("ERROR: Could not find 'service Platform' in proto file")
        sys.exit(1)
    
    service_content = service_match.group(1)
    
    # Extract all rpc methods
    rpc_pattern = r'rpc\s+(\w+)\s*\('
    for match in re.finditer(rpc_pattern, service_content):
        query_name = match.group(1)
        if query_name not in EXCLUDED_QUERIES:
            queries.append(query_name)
    
    return queries


def check_query_implementation(query_name, sdk_path):
    """Check if a query is implemented in the SDK."""
    patterns = QUERY_MAPPINGS.get(query_name, [query_name])
    
    # Search for any of the patterns in the SDK code
    for root, dirs, files in os.walk(sdk_path):
        # Skip test directories
        if 'tests' in root or 'test' in root:
            continue
            
        for file in files:
            if file.endswith('.rs'):
                file_path = os.path.join(root, file)
                try:
                    with open(file_path, 'r') as f:
                        content = f.read()
                        
                    for pattern in patterns:
                        if re.search(pattern, content, re.IGNORECASE):
                            return True, file_path
                except Exception as e:
                    print(f"Warning: Could not read {file_path}: {e}")
    
    return False, None


def load_cache(cache_file):
    """Load the query cache from file."""
    if cache_file.exists():
        with open(cache_file, 'r') as f:
            return json.load(f)
    return {"known_queries": {}, "last_updated": None}


def save_cache(cache_file, cache_data):
    """Save the query cache to file."""
    cache_data["last_updated"] = datetime.now().isoformat()
    with open(cache_file, 'w') as f:
        json.dump(cache_data, f, indent=2)


def main():
    """Main function to check gRPC coverage."""
    # Get paths
    script_dir = Path(__file__).parent
    project_root = script_dir.parent.parent
    proto_file = project_root / 'packages' / 'dapi-grpc' / 'protos' / 'platform' / 'v0' / 'platform.proto'
    sdk_path = project_root / 'packages' / 'rs-sdk' / 'src'
    cache_file = project_root / '.github' / 'grpc-queries-cache.json'
    
    # Check if files exist
    if not proto_file.exists():
        print(f"ERROR: Proto file not found: {proto_file}")
        sys.exit(1)
    
    if not sdk_path.exists():
        print(f"ERROR: SDK path not found: {sdk_path}")
        sys.exit(1)
    
    # Load cache
    cache_data = load_cache(cache_file)
    known_queries = cache_data.get("known_queries", {})
    
    # Extract queries from proto file
    print("Extracting gRPC queries from platform.proto...")
    all_queries = extract_grpc_queries(proto_file)
    all_queries.extend(EXCLUDED_QUERIES)  # Add excluded queries to all queries
    
    # Find new queries
    new_queries = []
    for query in all_queries:
        if query not in known_queries:
            new_queries.append(query)
    
    print(f"Found {len(all_queries)} total queries")
    print(f"Known queries: {len(known_queries)}")
    print(f"New queries to check: {len(new_queries)}")
    
    # Check only new queries
    missing_new_queries = []
    implemented_new_queries = []
    report_lines = []
    
    report_lines.append("=" * 80)
    report_lines.append("gRPC Query Coverage Report - NEW QUERIES ONLY")
    report_lines.append("=" * 80)
    report_lines.append(f"\nTotal queries in proto: {len(all_queries)}")
    report_lines.append(f"Previously known queries: {len(known_queries)}")
    report_lines.append(f"New queries found: {len(new_queries)}\n")
    
    if new_queries:
        report_lines.append("=" * 80)
        report_lines.append("\nNew Query Implementation Status:")
        report_lines.append("-" * 80)
        
        for query in sorted(new_queries):
            # Skip excluded queries
            if query in EXCLUDED_QUERIES:
                print(f"Checking {query}... EXCLUDED")
                known_queries[query] = {"status": "excluded", "reason": "Explicitly excluded as per requirement"}
                report_lines.append(f"⊘ {query:<45} EXCLUDED")
                continue
            
            print(f"Checking {query}...", end=' ')
            implemented, location = check_query_implementation(query, sdk_path)
            
            if implemented:
                print("✓")
                implemented_new_queries.append(query)
                known_queries[query] = {"status": "implemented"}
                report_lines.append(f"✓ {query:<45} {location}")
            else:
                print("✗")
                missing_new_queries.append(query)
                known_queries[query] = {"status": "not_implemented"}
                report_lines.append(f"✗ {query:<45} NOT FOUND")
    
    # Check if previously not_implemented queries are now implemented
    updated_queries = []
    for query, info in known_queries.items():
        if info.get("status") == "not_implemented" and query not in new_queries:
            implemented, location = check_query_implementation(query, sdk_path)
            if implemented:
                updated_queries.append(query)
                known_queries[query] = {"status": "implemented"}
    
    if updated_queries:
        report_lines.append("\n" + "=" * 80)
        report_lines.append("Previously Missing Queries Now Implemented:")
        report_lines.append("-" * 80)
        for query in sorted(updated_queries):
            report_lines.append(f"✓ {query}")
    
    # Summary
    report_lines.append("\n" + "=" * 80)
    report_lines.append("Summary:")
    report_lines.append("-" * 80)
    
    if new_queries:
        excluded_count = len([q for q in new_queries if q in EXCLUDED_QUERIES])
        checkable_new = len(new_queries) - excluded_count
        if checkable_new > 0:
            report_lines.append(f"New queries implemented: {len(implemented_new_queries)} ({len(implemented_new_queries)/checkable_new*100:.1f}%)")
            report_lines.append(f"New queries missing: {len(missing_new_queries)} ({len(missing_new_queries)/checkable_new*100:.1f}%)")
        else:
            report_lines.append("All new queries are excluded")
    else:
        report_lines.append("No new queries found")
    
    if updated_queries:
        report_lines.append(f"Previously missing queries now implemented: {len(updated_queries)}")
    
    # Count totals
    total_not_implemented = len([q for q, info in known_queries.items() if info.get("status") == "not_implemented"])
    total_implemented = len([q for q, info in known_queries.items() if info.get("status") == "implemented"])
    total_excluded = len([q for q, info in known_queries.items() if info.get("status") == "excluded"])
    
    report_lines.append(f"\nTotal known queries: {len(known_queries)}")
    report_lines.append(f"  - Implemented: {total_implemented}")
    report_lines.append(f"  - Not implemented: {total_not_implemented}")
    report_lines.append(f"  - Excluded: {total_excluded}")
    
    # List all not implemented queries
    not_implemented_queries = [q for q, info in known_queries.items() if info.get("status") == "not_implemented"]
    if not_implemented_queries:
        report_lines.append(f"\nNot implemented queries:")
        for query in sorted(not_implemented_queries):
            report_lines.append(f"  - {query}")
    
    if missing_new_queries:
        report_lines.append(f"\nMissing NEW queries:")
        for query in missing_new_queries:
            report_lines.append(f"  - {query}")
    
    # Save updated cache
    save_cache(cache_file, {"known_queries": known_queries})
    
    # Write report
    report_content = '\n'.join(report_lines)
    with open('grpc-coverage-report.txt', 'w') as f:
        f.write(report_content)
    
    print("\n" + report_content)
    
    # Exit with error only if there are missing NEW queries
    if missing_new_queries:
        print(f"\nERROR: {len(missing_new_queries)} NEW queries are not implemented in rs-sdk")
        sys.exit(1)
    else:
        print("\nSUCCESS: All NEW gRPC queries are implemented in rs-sdk (or excluded)")
        sys.exit(0)


if __name__ == '__main__':
    main()