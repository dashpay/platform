#!/usr/bin/env python3
"""
Documentation generator for WASM JS SDK
Reads query and state transition definitions from api-definitions.json
and generates both user documentation (HTML) and AI reference (Markdown)
"""

import json
import html as html_lib
from pathlib import Path
from datetime import datetime, timezone

# Module-level constants extracted for maintainability

# Test data for various query types - using a known testnet values
TESTNET_TEST_DATA = {
    'identity_id': '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk',
    'specialized_balance_id': 'AzaU7zqCT7X1kxh8yWxkT9PxAgNqWDu4Gz13emwcRyAT',
    'contract_id': 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
    'data_contract_id': 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
    'data_contract_history_id': 'HLY575cNazmc5824FxqaEMEBuzFeE4a98GDRNKbyJqCM',
    'token_contract_id': 'ALybvzfcCwMs7sinDwmtumw17NneuW7RgFtFHgjKmF3A',
    'group_contract_id': '49PJEnNx7ReCitzkLdkDNr4s6RScGsnNexcdSZJ1ph5N',
    'public_key_hash_unique': 'b7e904ce25ed97594e72f7af0e66f298031c1754',
    'public_key_hash_non_unique': '518038dc858461bcee90478fd994bba8057b7531',
    'pro_tx_hash': '143dcd6a6b7684fde01e88a10e5d65de9a29244c5ecd586d14a342657025f113',
    'token_id': 'Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv',
    'document_type': 'domain',
    'document_id': '7NYmEKQsYtniQRUmxwdPGeVcirMoPh5ZPyAKz8BWFy3r',
    'username': 'alice',
    'epoch': 8635
}

# Function name mappings from API names to SDK function names
FUNCTION_NAME_MAP = {
    'getIdentity': 'identity_fetch',
    'getIdentityKeys': 'get_identity_keys',
    'getIdentityNonce': 'get_identity_nonce',
    'getIdentityContractNonce': 'get_identity_contract_nonce',
    'getIdentityBalance': 'get_identity_balance',
    'getIdentitiesBalances': 'get_identities_balances',
    'getIdentityBalanceAndRevision': 'get_identity_balance_and_revision',
    'getIdentityByPublicKeyHash': 'get_identity_by_public_key_hash',
    'getIdentityByNonUniquePublicKeyHash': 'get_identity_by_non_unique_public_key_hash',
    'getIdentitiesContractKeys': 'get_identities_contract_keys',
    'getIdentityTokenBalances': 'get_identity_token_balances',
    'getIdentitiesTokenBalances': 'get_identities_token_balances',
    'getIdentityTokenInfos': 'get_identity_token_infos',
    'getIdentitiesTokenInfos': 'get_identities_token_infos',
    'getDataContract': 'data_contract_fetch',
    'getDataContractHistory': 'get_data_contract_history',
    'getDataContracts': 'get_data_contracts',
    'getDocuments': 'get_documents',
    'getDocument': 'get_document',
    'getDpnsUsername': 'get_dpns_usernames',
    'dpnsCheckAvailability': 'dpns_is_name_available',
    'dpnsResolve': 'dpns_resolve_name',
    'dpnsSearch': 'get_documents',
    'getContestedResources': 'get_contested_resources',
    'getContestedResourceVoteState': 'get_contested_resource_vote_state',
    'getContestedResourceVotersForIdentity': 'get_contested_resource_voters_for_identity',
    'getContestedResourceIdentityVotes': 'get_contested_resource_identity_votes',
    'getVotePollsByEndDate': 'get_vote_polls_by_end_date',
    'getProtocolVersionUpgradeState': 'get_protocol_version_upgrade_state',
    'getProtocolVersionUpgradeVoteStatus': 'get_protocol_version_upgrade_vote_status',
    'getEpochsInfo': 'get_epochs_info',
    'getCurrentEpoch': 'get_current_epoch',
    'getFinalizedEpochInfos': 'get_finalized_epoch_infos',
    'getEvonodesProposedEpochBlocksByIds': 'get_evonodes_proposed_epoch_blocks_by_ids',
    'getEvonodesProposedEpochBlocksByRange': 'get_evonodes_proposed_epoch_blocks_by_range',
    'getTokenStatuses': 'get_token_statuses',
    'getTokenDirectPurchasePrices': 'get_token_direct_purchase_prices',
    'getTokenContractInfo': 'get_token_contract_info',
    'getTokenPerpetualDistributionLastClaim': 'get_token_perpetual_distribution_last_claim',
    'getTokenTotalSupply': 'get_token_total_supply',
    'getGroupInfo': 'get_group_info',
    'getGroupInfos': 'get_group_infos',
    'getGroupActions': 'get_group_actions',
    'getGroupActionSigners': 'get_group_action_signers',
    'getStatus': 'get_status',
    'getCurrentQuorumsInfo': 'get_current_quorums_info',
    'getPrefundedSpecializedBalance': 'get_prefunded_specialized_balance',
    'getTotalCreditsInPlatform': 'get_total_credits_in_platform',
    'getPathElements': 'get_path_elements',
    'waitForStateTransitionResult': 'wait_for_state_transition_result'
}

def load_api_definitions(api_definitions_file):
    """Load query and state transition definitions from api-definitions.json"""
    try:
        with open(api_definitions_file, 'r', encoding='utf-8') as f:
            api_data = json.load(f)
        
        query_definitions = api_data.get('queries', {})
        transition_definitions = api_data.get('transitions', {})
        
        return query_definitions, transition_definitions
        
    except (FileNotFoundError, json.JSONDecodeError) as e:
        print(f"Error loading API definitions: {e}")
        return {}, {}

def generate_example_code(query_key, inputs):
    """Generate example code for a query"""
    
    # Map input names to test values
    param_mapping = {
        'id': f"'{TESTNET_TEST_DATA['data_contract_history_id']}'" if 'getDataContractHistory' in query_key else f"'{TESTNET_TEST_DATA['data_contract_id']}'" if 'getDataContract' in query_key else f"'{TESTNET_TEST_DATA['identity_id']}'",
        'identityId': f"'{TESTNET_TEST_DATA['specialized_balance_id']}'" if 'getPrefundedSpecializedBalance' in query_key else "'5RG84o6KsTaZudDqS8ytbaRB8QP4YYQ2uwzb6Hj8cfjX'" if 'getTokenPerpetualDistributionLastClaim' in query_key else f"'{TESTNET_TEST_DATA['identity_id']}'",
        'ids': f"['{TESTNET_TEST_DATA['data_contract_id']}', '{TESTNET_TEST_DATA['token_contract_id']}']" if 'getDataContracts' in query_key else f"['{TESTNET_TEST_DATA['pro_tx_hash']}']" if 'Evonodes' in query_key else f"['{TESTNET_TEST_DATA['identity_id']}']",
        'identitiesIds': f"['{TESTNET_TEST_DATA['identity_id']}']",
        'identityIds': f"['{TESTNET_TEST_DATA['identity_id']}']",
        'contractId': f"'{TESTNET_TEST_DATA['group_contract_id']}'" if ('group' in query_key.lower() or 'Group' in query_key) else f"'{TESTNET_TEST_DATA['token_contract_id']}'" if ('token' in query_key.lower() or 'Token' in query_key) and 'TokenBalance' not in query_key and 'TokenInfo' not in query_key else f"'{TESTNET_TEST_DATA['contract_id']}'",
        'dataContractId': "'EETVvWgohFDKtbB3ejEzBcDRMNYkc9TtgXY6y8hzP3Ta'" if 'getTokenContractInfo' in query_key else f"'{TESTNET_TEST_DATA['data_contract_id']}'",
        'publicKeyHash': f"'{TESTNET_TEST_DATA['public_key_hash_unique']}'" if 'ByPublicKeyHash' in query_key and 'NonUnique' not in query_key else f"'{TESTNET_TEST_DATA['public_key_hash_non_unique']}'",
        'startProTxHash': f"'{TESTNET_TEST_DATA['pro_tx_hash']}'",
        'tokenId': "'HEv1AYWQfwCffXQgmuzmzyzUo9untRTmVr67n4e4PSWa'" if 'getTokenPerpetualDistributionLastClaim' in query_key else f"'{TESTNET_TEST_DATA['token_id']}'",
        'tokenIds': f"['{TESTNET_TEST_DATA['token_id']}', 'H7FRpZJqZK933r9CzZMsCuf1BM34NT5P2wSJyjDkprqy']" if 'getTokenStatuses' in query_key else "['H7FRpZJqZK933r9CzZMsCuf1BM34NT5P2wSJyjDkprqy']" if 'getTokenDirectPurchasePrices' in query_key else f"['{TESTNET_TEST_DATA['token_id']}']",
        'documentType': f"'{TESTNET_TEST_DATA['document_type']}'",
        'documentId': f"'{TESTNET_TEST_DATA['document_id']}'",
        'label': f"'{TESTNET_TEST_DATA['username']}'",
        'name': f"'{TESTNET_TEST_DATA['username']}'",
        'prefix': "'ali'",
        'epoch': '1000' if 'getEpochsInfo' in query_key else TESTNET_TEST_DATA['epoch'],
        'keyRequestType': "'all'",
        'limit': '10',
        'count': '100',
        'offset': '0',
        'startEpoch': '8635',
        'startTimeMs': 'Date.now() - 86400000', # 24 hours ago
        'endTimeMs': 'Date.now()',
        'ascending': 'true',
        'orderAscending': 'true',
        'resultType': "'documents'",
        'documentTypeName': "'domain'",
        'indexName': "'parentNameAndLabel'",
        'contestantId': f"'{TESTNET_TEST_DATA['identity_id']}'",
        'amount': '1000000',
        'recipientId': f"'{TESTNET_TEST_DATA['identity_id']}'",
        'toAddress': "'yNPbcFfabtNmmxKdGwhHomdYfVs6gikbPf'",
        'where': 'JSON.stringify([["normalizedParentDomainName", "==", "dash"]])',
        'orderBy': 'JSON.stringify([["normalizedLabel", "asc"]])',
        'groupContractPosition': '0',
        'groupContractPositions': '[0]',
        'startAtGroupContractPosition': '0',
        'startGroupContractPositionIncluded': 'false',
        'status': "'ACTIVE'",
        'startActionId': "'0'",
        'startActionIdIncluded': 'false',
        'actionId': "'6XJzL6Qb8Zhwxt4HFwh8NAn7q1u4dwdoUf8EmgzDudFZ'",
        'path': "['96']",
        'keys': f"['{TESTNET_TEST_DATA['identity_id']}']",
        'stateTransitionHash': "'0000000000000000000000000000000000000000000000000000000000000000'",
        'allowIncludeLockedAndAbstainingVoteTally': 'null',
        'startAtValue': 'null',
        'startAtIdentifierInfo': 'null',
        'indexValues': "['dash', 'alice']",
        'startAtVoterInfo': 'null',
        'startAtVotePollIdInfo': 'null',
        'startTimeInfo': '(Date.now() - 86400000).toString()',
        'endTimeInfo': 'Date.now().toString()',
        'startAfter': 'null'
    }
    
    # Handle special cases for functions with structured parameters
    if query_key == 'getGroupInfos':
        # getGroupInfos expects: sdk, contractId, startAtInfo (object or null), count
        params = [f"'{TESTNET_TEST_DATA['group_contract_id']}'", "null", "100"]
    elif query_key == 'getGroupActions':
        # getGroupActions expects: sdk, contractId, groupContractPosition, status, startAtInfo (object or null), count
        params = [f"'{TESTNET_TEST_DATA['group_contract_id']}'", "0", "'ACTIVE'", "null", "100"]
    elif query_key == 'getDataContractHistory':
        # getDataContractHistory expects: sdk, id, limit, offset, startAtMs
        # Use the specific contract ID for getDataContractHistory examples
        params = ["'HLY575cNazmc5824FxqaEMEBuzFeE4a98GDRNKbyJqCM'", "10", "0", "'0'"]
    elif query_key == 'dpnsSearch':
        # dpnsSearch is implemented as get_documents with DPNS-specific parameters
        # get_documents expects: sdk, contractId, documentType, whereClause, orderBy, limit
        dpns_contract_id = "'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec'"
        document_type = "'domain'"
        where_clause = 'JSON.stringify([["normalizedLabel", "startsWith", "ali"], ["normalizedParentDomainName", "==", "dash"]])'
        order_by = 'JSON.stringify([["normalizedLabel", "asc"]])'
        limit = "10"
        params = [dpns_contract_id, document_type, where_clause, order_by, limit]
    else:
        # Generate parameters normally
        params = []
        for input_def in inputs:
            name = input_def.get('name', '')
            if name in param_mapping:
                params.append(str(param_mapping[name]))
            elif input_def.get('required', False):
                # Generate a default value based on type
                if input_def.get('type') == 'array':
                    params.append('[]')
                elif input_def.get('type') == 'number':
                    params.append('100')
                elif input_def.get('type') == 'checkbox':
                    params.append('false')
                else:
                    params.append('""')
    
    # Use module-level function name mapping
    func_name = FUNCTION_NAME_MAP.get(query_key, query_key)
    
    # Add sdk as first parameter for all functions
    all_params = ['sdk'] + params
    
    # Generate the function call (functions are imported directly, not methods on sdk)
    return f'return await window.wasmFunctions.{func_name}({", ".join(all_params)});'


def generate_sidebar_entries(definitions, type_prefix, section_class=""):
    """Generate sidebar entries for queries or transitions"""
    html_content = ""
    for cat_key, category in definitions.items():
        html_content += f'            <li class="category">{category.get("label", cat_key)}</li>\n'
        items = category.get('queries' if type_prefix == 'query' else 'transitions', {})
        for item_key in items:
            item = items[item_key]
            html_content += f'            <li style="margin-left: 20px;"><a href="#{type_prefix}-{item_key}">{item.get("label", item_key)}</a></li>\n'
    return html_content

def generate_operation_docs(definitions, type_name, type_prefix):
    """Generate documentation for operations (queries or transitions)"""
    html_content = ""
    for cat_key, category in definitions.items():
        html_content += f'''\n    <div class="category">
        <h3>{category.get('label', cat_key)}</h3>
'''
        
        items_key = 'queries' if type_prefix == 'query' else 'transitions'
        items = category.get(items_key, {})
        for item_key, item in items.items():
            html_content += generate_operation_entry(item_key, item, type_prefix)
        
        html_content += '    </div>'
    return html_content

def generate_operation_entry(operation_key, operation, type_prefix):
    """Generate documentation for a single operation"""
    html_content = f'''        <div class="operation">
            <h4 id="{type_prefix}-{operation_key}">{operation.get('label', operation_key)}</h4>
            <p class="description">{operation.get('description', 'No description available')}</p>
            
            <div class="parameters">
                <h5>Parameters:</h5>
'''
    
    # Use sdk_params if available (for state transitions), otherwise use inputs
    sdk_params = operation.get('sdk_params', [])
    inputs = operation.get('inputs', [])
    params_to_use = sdk_params if sdk_params else inputs
    
    if not params_to_use:
        html_content += '                <p class="param-optional">No parameters required</p>'
    else:
        for param in params_to_use:
            html_content += generate_parameter_entry(param)
    
    html_content += '''            </div>
            
            <div class="example-container">
                <h5>Example</h5>
'''
    
    if type_prefix == 'query':
        example_code = generate_example_code(operation_key, inputs)
        html_content += f'                <div class="example-code" id="code-{operation_key}">{example_code}</div>\n'
        
        # Special handling for certain operations
        if operation_key == 'waitForStateTransitionResult':
            html_content += '                <p class="info-note">This is an internal query used to wait for and retrieve the result of a previously submitted state transition. It requires a valid state transition hash from a prior operation.</p>'
        else:
            html_content += f'                <button class="run-button" id="run-{operation_key}" onclick="runExample(\'{operation_key}\')">Run</button>'
            if operation_key in ['getPathElements', 'getContestedResourceVotersForIdentity']:
                html_content += ' <span style="color: #f39c12; margin-left: 10px;">🚧 Work in Progress</span>'
        
        # Add special examples and info
        if operation_key == 'getIdentityKeys':
            html_content += f'''
                <div class="example-result" id="result-{operation_key}"></div>
            </div>
            
            <div class="example-container">
                <h5>Example 2 - Get Specific Keys</h5>
                <div class="example-code" id="code-getIdentityKeys2">return await window.wasmFunctions.get_identity_keys(sdk, \'5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk\', \'specific\', [0, 1, 2]);</div>
                <button class="run-button" id="run-getIdentityKeys2" onclick="runExample(\'getIdentityKeys2\')">Run</button>
                <div class="example-result" id="result-getIdentityKeys2"></div>
            </div>
            
            <div class="example-container">
                <h5>Example 3 - Search Keys by Purpose</h5>
                <div class="example-code" id="code-getIdentityKeys3">return await window.wasmFunctions.get_identity_keys(sdk, \'5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk\', \'search\', undefined, \'{{"0": {{"0": "current"}}, "1": {{"0": "current"}}}}\');</div>
                <button class="run-button" id="run-getIdentityKeys3" onclick="runExample(\'getIdentityKeys3\')">Run</button>
                <div class="example-result" id="result-getIdentityKeys3"></div>
            </div>
        </div>'''
            return html_content
        elif operation_key == 'getPathElements':
            html_content += generate_path_elements_info()
        
        html_content += f'\n                <div class="example-result" id="result-{operation_key}"></div>'
    else:
        # State transitions don't have run buttons
        html_content += f'                <div class="example-code">{generate_transition_example(operation_key, operation)}</div>'
    
    html_content += '''            </div>
        </div>
'''
    return html_content

def generate_parameter_entry(param):
    """Generate documentation for a single parameter"""
    required_text = '<span class="param-required">(required)</span>' if param.get('required', False) else '<span class="param-optional">(optional)</span>'
    html_content = f'''                <div class="parameter">
                    <span class="param-name">{param.get('label', param.get('name', 'Unknown'))}</span>
                    <span class="param-type">{param.get('type', 'text')}</span>
                    {required_text}
'''
    if param.get('description'):
        html_content += f'                    <br><small>{html_lib.escape(param.get("description"))}</small>\n'
    elif param.get('placeholder'):
        html_content += f'                    <br><small>Example: {html_lib.escape(param.get("placeholder"))}</small>\n'
    elif param.get('name') == 'limit' and not param.get('required', False):
        html_content += '                    <br><small>Default: 100 (maximum items returned if not specified)</small>\n'
    if param.get('options'):
        html_content += '                    <br><small>Options: '
        opts = [f'{opt.get("label", opt.get("value"))}' for opt in param.get('options', [])]
        html_content += ', '.join(opts)
        html_content += '</small>\n'
    html_content += '                </div>\n'
    return html_content

def generate_transition_example(trans_key, transition=None):
    """Generate example code for state transitions"""
    # Check if there's a custom sdk_example
    if transition and transition.get('sdk_example'):
        return transition.get('sdk_example')
    
    if trans_key == 'documentCreate':
        return '''const result = await sdk.document_create(
    identityHex,
    contractId,
    "note",
    JSON.stringify({ message: "Hello!" }),
    privateKeyHex
);'''
    elif trans_key == 'tokenTransfer':
        return '''const result = await sdk.token_transfer(
    identityHex,
    contractId,
    tokenId,
    1000000, // amount
    recipientId,
    privateKeyHex
);'''
    else:
        return f'const result = await sdk.{trans_key}(identityHex, /* params */, privateKeyHex);'

def generate_path_elements_info():
    """Generate path elements documentation"""
    return '''\n                <div class="path-info">
                    <h6>Common Path Values:</h6>
                    <table class="path-table">
                        <tr><th>Path Value</th><th>Description</th></tr>
                        <tr><td><code>64</code></td><td>Data Contract Documents</td></tr>
                        <tr><td><code>32</code></td><td>Identities</td></tr>
                        <tr><td><code>24</code></td><td>Unique Public Key Hashes to Identities</td></tr>
                        <tr><td><code>8</code></td><td>Non-Unique Public Key Hashes to Identities</td></tr>
                        <tr><td><code>16</code></td><td>Tokens</td></tr>
                        <tr><td><code>48</code></td><td>Pools</td></tr>
                        <tr><td><code>40</code></td><td>Prefunded Specialized Balances</td></tr>
                        <tr><td><code>72</code></td><td>Spent Asset Lock Transactions</td></tr>
                        <tr><td><code>80</code></td><td>Withdrawal Transactions</td></tr>
                        <tr><td><code>88</code></td><td>Group Actions</td></tr>
                        <tr><td><code>96</code></td><td>Balances</td></tr>
                        <tr><td><code>104</code></td><td>Misc</td></tr>
                        <tr><td><code>112</code></td><td>Votes</td></tr>
                        <tr><td><code>120</code></td><td>Versions</td></tr>
                    </table>
                    <h6>Example Paths:</h6>
                    <ul>
                        <li><code>[32, identity_id]</code> - Access identity data</li>
                        <li><code>[96, identity_id]</code> - Access identity balance</li>
                        <li><code>[16, contract_id, token_id]</code> - Access token info</li>
                        <li><code>[64, contract_id, 1, document_type]</code> - Access documents by type</li>
                        <li><code>[88, contract_id, group_position]</code> - Access group actions</li>
                        <li><code>[112]</code> - Access votes tree</li>
                    </ul>
                </div>'''


def generate_docs_javascript():
    """Generate the main JavaScript module for docs.html"""
    
    # Generate function lists dynamically from FUNCTION_NAME_MAP
    wasm_functions = list(dict.fromkeys(FUNCTION_NAME_MAP.values()))
    
    # Additional functions not in FUNCTION_NAME_MAP
    additional_functions = ['prefetch_trusted_quorums_testnet']
    
    # For imports: init is default export, others are named exports
    named_imports = ['WasmSdkBuilder', *wasm_functions, *additional_functions]
    named_imports_str = ',\n            '.join(named_imports)
    
    # For window.wasmFunctions: only include the actual functions (not init/WasmSdkBuilder)
    window_functions = wasm_functions  # Note: excluding prefetch_trusted_quorums_testnet from window object
    window_assignments = ',\n            '.join(window_functions)
    
    return '''        import init, { 
            ''' + named_imports_str + '''
        } from './pkg/wasm_sdk.js';
        
        let sdk = null;
        let isInitialized = false;
        
        // Make functions available globally for the examples
        window.wasmFunctions = {
            ''' + window_assignments + '''
        };
        
        // Progress update function
        function updateProgress(percent, text) {
            const progressFill = document.getElementById('progressFill');
            const progressPercent = document.getElementById('progressPercent');
            const preloaderText = document.querySelector('.preloader-text');
            
            if (progressFill) progressFill.style.width = percent + '%';
            if (progressPercent) progressPercent.textContent = percent + '%';
            if (preloaderText && text) preloaderText.textContent = text;
        }
        
        async function checkWasmCached() {
            if ('caches' in window) {
                try {
                    const cache = await caches.open('wasm-sdk-cache-v7');
                    const wasmResponse = await cache.match('/pkg/wasm_sdk_bg.wasm');
                    const jsResponse = await cache.match('/pkg/wasm_sdk.js');
                    return wasmResponse && jsResponse;
                } catch (e) {
                    return false;
                }
            }
            return false;
        }
        
        async function initializeSdk() {
            if (isInitialized) return;
            
            const preloader = document.getElementById('preloader');
            const isCached = await checkWasmCached();
            
            // Only show preloader if not cached
            if (!isCached || window.location.search.includes('force-reload')) {
                preloader.classList.add('preloader--visible');
            }
            
            try {
                const startTime = performance.now();
                
                if (!isCached) {
                    updateProgress(10, 'Loading WASM module...');
                } else {
                    updateProgress(50, 'Loading from cache...');
                }
                
                await init();
                
                if (!isCached) {
                    updateProgress(50, 'Initializing SDK...');
                } else {
                    updateProgress(80, 'Initializing SDK...');
                }
                
                updateProgress(80, 'Prefetching quorum information...');
                await prefetch_trusted_quorums_testnet();
                
                updateProgress(85, 'Building SDK...');
                sdk = await WasmSdkBuilder.new_testnet_trusted().build();
                updateProgress(90, 'Finalizing...');
                
                isInitialized = true;
                const endTime = performance.now();
                const loadTime = endTime - startTime;
                console.log(`SDK initialized in ${loadTime.toFixed(2)}ms (${isCached ? 'from cache' : 'fresh load'})`);
                
                updateProgress(100, 'Ready!');
                
                // Hide preloader faster if cached
                setTimeout(() => {
                    preloader.classList.remove('preloader--visible');
                }, isCached ? 200 : 500);
            } catch (error) {
                console.error('Failed to initialize SDK:', error);
                sdk = null;
                isInitialized = false;
                preloader.classList.remove('preloader--visible');
                throw error;
            }
        }
        
        window.runExample = async function(exampleId) {
            const button = document.getElementById(`run-${exampleId}`);
            const result = document.getElementById(`result-${exampleId}`);
            
            button.disabled = true;
            button.innerHTML = '<span class="loading"></span> Running...';
            result.style.display = 'none';
            
            if (!isInitialized) {
                await initializeSdk();
            }
            
            if (!sdk) {
                result.className = 'example-result error';
                result.textContent = 'Error: SDK failed to initialize. Check console for details.';
                result.style.display = 'block';
                button.disabled = false;
                button.innerHTML = 'Run';
                return;
            }
            
            try {
                const code = document.getElementById(`code-${exampleId}`).textContent;
                const func = new Function('sdk', 'return (async () => { ' + code + ' })()');
                let output = await func(sdk);
                
                result.className = 'example-result success';
                result.textContent = JSON.stringify(output, null, 2);
                result.style.display = 'block';
            } catch (error) {
                // Check if it's a quorum cache error and retry once
                if (error.message && error.message.includes('Quorum not found in cache')) {
                    console.log('Quorum cache miss, retrying with fresh quorum prefetch...');
                    try {
                        // Reinitialize SDK to refresh quorum cache
                        sdk = null;
                        isInitialized = false;
                        
                        // Prefetch quorums again to ensure we have the latest
                        await prefetch_trusted_quorums_testnet();
                        await initializeSdk();
                        
                        if (sdk) {
                            const code = document.getElementById(`code-${exampleId}`).textContent;
                            const func = new Function('sdk', 'return (async () => { ' + code + ' })()');
                            const output = await func(sdk);
                            
                            result.className = 'example-result success';
                            result.textContent = JSON.stringify(output, null, 2);
                            result.style.display = 'block';
                        } else {
                            throw new Error('Failed to reinitialize SDK');
                        }
                    } catch (retryError) {
                        result.className = 'example-result error';
                        result.textContent = 'Error after retry: ' + retryError.message;
                        result.style.display = 'block';
                    }
                } else {
                    result.className = 'example-result error';
                    result.textContent = 'Error: ' + error.message;
                    result.style.display = 'block';
                }
            } finally {
                button.disabled = false;
                button.innerHTML = 'Run';
            }
        };
        
        // Register service worker for caching
        if ('serviceWorker' in navigator) {
            navigator.serviceWorker.register('/service-worker-simple.js')
                .then(registration => {
                    console.log('ServiceWorker registered:', registration.scope);
                })
                .catch(error => {
                    console.error('ServiceWorker registration failed:', error);
                });
        }
        
        // Initialize SDK when page loads
        window.addEventListener('DOMContentLoaded', async () => {
            console.log('[Performance] Starting WASM module load...');
            try {
                await initializeSdk();
            } catch (error) {
                console.error('Failed to initialize SDK on page load:', error);
            }
        });
        
        // Search functionality
        window.addEventListener('DOMContentLoaded', function() {
            const searchInput = document.getElementById('sidebar-search');
            const sidebarItems = document.querySelectorAll('.sidebar li');
            const noResults = document.getElementById('no-results');
            const categories = document.querySelectorAll('.sidebar .category');
            const sectionHeaders = document.querySelectorAll('.sidebar .section-header');
            
            searchInput.addEventListener('input', function(e) {
                const searchTerm = e.target.value.toLowerCase();
                let hasResults = false;
                
                // Hide all categories and section headers initially
                categories.forEach(cat => {
                    cat.style.display = searchTerm ? 'none' : 'block';
                });
                sectionHeaders.forEach(header => {
                    header.style.display = searchTerm ? 'none' : 'block';
                });
                
                sidebarItems.forEach(item => {
                    const link = item.querySelector('a');
                    if (link) {
                        const text = link.textContent.toLowerCase();
                        const matches = text.includes(searchTerm);
                        
                        if (searchTerm === '') {
                            item.classList.remove('hidden');
                            hasResults = true;
                        } else if (matches) {
                            item.classList.remove('hidden');
                            hasResults = true;
                            
                            // Show the category for this item
                            let prevSibling = item.previousElementSibling;
                            while (prevSibling) {
                                if (prevSibling.classList.contains('category')) {
                                    prevSibling.style.display = 'block';
                                    break;
                                }
                                if (prevSibling.querySelector('strong')) {
                                    // This is a category item
                                    prevSibling.style.display = 'block';
                                    break;
                                }
                                prevSibling = prevSibling.previousElementSibling;
                            }
                        } else {
                            item.classList.add('hidden');
                        }
                    }
                });
                
                // Show/hide no results message
                noResults.style.display = hasResults ? 'none' : 'block';
            });
        });'''


def generate_test_runner_javascript():
    """Generate the test runner JavaScript for the hidden testing feature"""
    return '''        // Hidden test runner feature
        
        // Create the test runner UI
        const testRunnerHTML = `
            <div id="test-runner" style="display: none; position: fixed; top: 50%; left: 50%; transform: translate(-50%, -50%); 
                 background: white; border: 2px solid #3498db; border-radius: 10px; padding: 20px; 
                 box-shadow: 0 4px 20px rgba(0,0,0,0.3); z-index: 10000; max-width: 80%; max-height: 80%; overflow: auto;">
                <h2 style="margin-top: 0; color: #2c3e50;">Test Runner</h2>
                <button id="close-test-runner" style="position: absolute; top: 10px; right: 10px; 
                        background: #e74c3c; color: white; border: none; padding: 5px 10px; 
                        border-radius: 5px; cursor: pointer;">✕</button>
                <button id="run-all-tests" style="background: #3498db; color: white; border: none; 
                        padding: 10px 20px; border-radius: 5px; cursor: pointer; font-size: 16px;">
                    Run All Tests
                </button>
                <div id="test-progress" style="margin-top: 20px; font-weight: bold;"></div>
                <div id="test-summary" style="margin-top: 10px; display: flex; gap: 20px;"></div>
                <div id="test-results" style="margin-top: 20px;"></div>
            </div>
        `;
        
        // Add test runner to body
        document.body.insertAdjacentHTML('beforeend', testRunnerHTML);
        
        // Get references to elements
        const queriesHeader = document.querySelector('.section-header');
        const testRunner = document.getElementById('test-runner');
        const closeButton = document.getElementById('close-test-runner');
        const runAllButton = document.getElementById('run-all-tests');
        const testProgress = document.getElementById('test-progress');
        const testSummary = document.getElementById('test-summary');
        const testResults = document.getElementById('test-results');
        
        // Show test runner
        function showTestRunner() {
            testRunner.style.display = 'block';
            document.body.style.overflow = 'hidden'; // Prevent scrolling
        }
        
        // Hide test runner
        function hideTestRunner() {
            testRunner.style.display = 'none';
            document.body.style.overflow = 'auto';
        }
        
        // Run a single test
        async function runSingleTest(buttonId) {
            try {
                const functionId = buttonId.replace('run-', '');
                
                // Call the existing runExample function
                await window.runExample(functionId);
                
                // Wait a bit for the result to be displayed
                await new Promise(resolve => setTimeout(resolve, 100));
                
                // Check the result element to see if it succeeded
                const resultDiv = document.getElementById(`result-${functionId}`);
                if (resultDiv && resultDiv.style.display !== 'none') {
                    const hasError = resultDiv.className && resultDiv.className.includes('error');
                    if (hasError) {
                        const errorText = resultDiv.textContent || 'Unknown error';
                        return { id: buttonId, success: false, error: errorText };
                    }
                    return { id: buttonId, success: true };
                }
                
                // If no result div or not displayed, assume it worked
                return { id: buttonId, success: true };
            } catch (error) {
                return { id: buttonId, success: false, error: error.message || error.toString() };
            }
        }
        
        // Run all tests
        async function runAllTests() {
            testProgress.textContent = 'Starting tests...';
            testResults.innerHTML = '';
            testSummary.textContent = '';
            
            // Find all run buttons
            const runButtons = document.querySelectorAll('.run-button');
            const totalTests = runButtons.length;
            let passed = 0;
            let failed = 0;
            let currentTest = 0;
            
            const results = [];
            
            for (const button of runButtons) {
                currentTest++;
                const testName = button.id.replace('run-', '');
                testProgress.textContent = `Running test ${currentTest} of ${totalTests}: ${testName}...`;
                
                const result = await runSingleTest(button.id);
                
                if (result.success) {
                    passed++;
                    results.push(`<div style="color: #27ae60; margin: 5px 0;">✅ ${testName}: PASSED</div>`);
                } else {
                    failed++;
                    results.push(`<div style="color: #e74c3c; margin: 5px 0;">❌ ${testName}: FAILED - ${result.error}</div>`);
                }
                
                // Update results in real-time
                testResults.innerHTML = results.join('');
            }
            
            testProgress.textContent = 'All tests completed!';
            testSummary.innerHTML = `
                <div style="color: #2c3e50;">Total: ${totalTests}</div>
                <div style="color: #27ae60;">Passed: ${passed}</div>
                <div style="color: #e74c3c;">Failed: ${failed}</div>
                <div style="color: #3498db;">Success Rate: ${((passed/totalTests) * 100).toFixed(1)}%</div>
            `;
        }
        
        // Set up triple-click detection on Queries header
        if (queriesHeader) {
            let clickCount = 0;
            let clickTimer = null;
            
            queriesHeader.addEventListener('click', () => {
                clickCount++;
                
                // Reset click count after 500ms
                if (clickTimer) {
                    clearTimeout(clickTimer);
                }
                
                clickTimer = setTimeout(() => {
                    clickCount = 0;
                }, 500);
                
                // Show test runner on triple click
                if (clickCount === 3) {
                    showTestRunner();
                    clickCount = 0;
                    if (clickTimer) {
                        clearTimeout(clickTimer);
                        clickTimer = null;
                    }
                }
            });
        }
        
        // Close button handler
        closeButton.addEventListener('click', hideTestRunner);
        
        // Run all tests button handler
        runAllButton.addEventListener('click', runAllTests);
        
        // Close on escape key
        document.addEventListener('keydown', (e) => {
            if (e.key === 'Escape' && testRunner.style.display !== 'none') {
                hideTestRunner();
            }
        });
        
        // Make runExample function available globally if it doesn't exist
        if (!window.runExample) {
            window.runExample = async function(exampleId) {
                const resultDiv = document.getElementById('result-' + exampleId);
                const codeElement = document.getElementById('code-' + exampleId);
                const button = document.getElementById('run-' + exampleId);
                
                if (!resultDiv || !codeElement || !button) return;
                
                // Disable button
                button.disabled = true;
                button.textContent = 'Running...';
                
                try {
                    // Clear previous results
                    resultDiv.innerHTML = '<div style="color: #3498db;">Executing...</div>';
                    
                    // Execute the example code
                    const code = codeElement.textContent;
                    const AsyncFunction = Object.getPrototypeOf(async function(){}).constructor;
                    const testFunc = new AsyncFunction('window', 'sdk', code);
                    
                    if (!window.sdk) {
                        throw new Error('SDK not initialized');
                    }
                    
                    const result = await testFunc(window, window.sdk);
                    
                    // Display result
                    resultDiv.className = 'example-result success';
                    resultDiv.innerHTML = '<pre>' + JSON.stringify(result, null, 2) + '</pre>';
                    resultDiv.style.display = 'block';
                } catch (error) {
                    resultDiv.className = 'example-result error';
                    resultDiv.textContent = 'Error: ' + (error.message || error);
                    resultDiv.style.display = 'block';
                } finally {
                    // Re-enable button
                    button.disabled = false;
                    button.textContent = 'Run';
                }
            };
        }'''

def generate_html_head():
    """Generate HTML head section with meta tags and scripts"""
    return '''    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Dash Platform WASM JS SDK Documentation</title>
    <link rel="icon" type="image/svg+xml" href="https://media.dash.org/wp-content/uploads/blue-d.svg">
    <link rel="alternate icon" type="image/png" href="https://media.dash.org/wp-content/uploads/blue-d-250.png">
    <link rel="stylesheet" href="docs.css">
    <script type="module">
''' + generate_docs_javascript() + '''
    </script>'''


def generate_preloader_html():
    """Generate preloader HTML section"""
    return '''    <!-- Preloader -->
    <div id="preloader">
        <div class="preloader-content">
            <div class="preloader-text">Loading WASM module...</div>
            <div class="preloader-progress">
                <div class="progress-bar">
                    <div class="progress-fill" id="progressFill"></div>
                </div>
                <div class="progress-percent" id="progressPercent">0%</div>
            </div>
        </div>
    </div>'''


def generate_sidebar_html(query_defs, transition_defs):
    """Generate sidebar HTML section with table of contents"""
    sidebar_html = '''    <!-- Sidebar -->
    <div class="sidebar">
        <h2>Table of Contents</h2>
        <div class="search-container">
            <input type="text" id="sidebar-search" class="search-input" placeholder="Search queries and transitions...">
        </div>
        <div id="no-results" class="no-results">No results found</div>
        <ul>
            <li><a href="#overview">Overview</a></li>
        </ul>
        
        <div class="section-header">Queries</div>
        <ul>
'''
    
    # Generate sidebar links for queries
    sidebar_html += generate_sidebar_entries(query_defs, 'query')
    
    sidebar_html += '''        </ul>
        
        <div class="section-header state-transitions">State Transitions</div>
        <ul>
'''
    
    # Generate sidebar links for transitions
    sidebar_html += generate_sidebar_entries(transition_defs, 'transition')
    
    sidebar_html += '''        </ul>
    </div>'''
    
    return sidebar_html


def generate_main_content_html(query_defs, transition_defs):
    """Generate main content HTML section with documentation"""
    main_content_html = '''    <!-- Main Content -->
    <div class="main-content">
        <nav class="nav">
            <ul>
                <li><a href="index.html">← Back to SDK</a></li>
                <li><a href="AI_REFERENCE.md">AI Reference</a></li>
                <li><a href="https://github.com/dashpay/platform" target="_blank">GitHub</a></li>
            </ul>
        </nav>
        
        <h1>Dash Platform WASM JS SDK Documentation</h1>
        
        <div class="category" id="overview">
            <h2>Overview</h2>
            <p>The Dash Platform WASM JS SDK provides a WebAssembly-based interface for interacting with the Dash Platform from JavaScript and TypeScript applications. 
            This documentation covers all available queries and state transitions.</p>
            
            <h3>Key Concepts</h3>
            <ul>
                <li><strong>Queries</strong>: Read-only operations that fetch data from the platform</li>
                <li><strong>State Transitions</strong>: Write operations that modify state on the platform</li>
                <li><strong>Proofs</strong>: Cryptographic proofs can be requested for most queries to verify data authenticity</li>
                <li><strong>Credits</strong>: The platform's unit of account for paying transaction fees</li>
                <li><strong>Default Limits</strong>: All queries with optional limit parameters default to a maximum of 100 items if not specified</li>
            </ul>
            
            <p><strong>Note:</strong> All examples below can be run directly in your browser using the testnet. Click the "Run" button to execute any example.</p>
            
            <div style="background-color: #e3f2fd; padding: 15px; border-radius: 5px; margin-top: 15px;">
                <strong>Test Identity:</strong> All examples use the testnet identity <code style="background-color: #fff; padding: 2px 6px; border-radius: 3px;">5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk</code>
                <br>This is a known identity with activity on testnet that you can use for testing.
            </div>
        </div>
        
        <h2 id="queries">Queries</h2>
'''
    
    # Add query documentation
    main_content_html += generate_operation_docs(query_defs, 'query', 'query')
    
    # Add state transition documentation
    main_content_html += '<h2 id="transitions">State Transitions</h2>'
    main_content_html += generate_operation_docs(transition_defs, 'transition', 'transition')
    
    # Close main content div
    main_content_html += '''
        <a href="#" class="back-to-top">↑ Top</a>
    </div>'''
    
    return main_content_html

def generate_user_docs_html(query_defs, transition_defs):
    """Generate user-friendly HTML documentation"""
    
    html_content = '''<!DOCTYPE html>
<html lang="en">
<head>
''' + generate_html_head() + '''
</head>
<body>
''' + generate_preloader_html() + '''
    
''' + generate_sidebar_html(query_defs, transition_defs) + '''
    
''' + generate_main_content_html(query_defs, transition_defs) + '''
    
    <script>
        // Smooth scrolling for anchor links
        document.querySelectorAll('a[href^="#"]').forEach(anchor => {
            anchor.addEventListener('click', function (e) {
                e.preventDefault();
                const target = document.querySelector(this.getAttribute('href'));
                if (target) {
                    target.scrollIntoView({
                        behavior: 'smooth',
                        block: 'start'
                    });
                }
            });
        });
    </script>
    
    <script>
''' + generate_test_runner_javascript() + '''
    </script>
</body>
</html>
'''
    
    return html_content

def generate_ai_reference_md(query_defs, transition_defs):
    """Generate AI-friendly markdown reference"""
    
    md_content = '''# Dash Platform WASM JS SDK - AI Reference

## Overview
The Dash Platform WASM JS SDK provides WebAssembly bindings for interacting with Dash Platform from JavaScript/TypeScript. This reference is optimized for AI understanding and quick implementation.

## Quick Setup
```javascript
// Import and initialize
import init, { WasmSdk } from './pkg/wasm_sdk.js';

await init();
const transport = { 
    url: "https://52.12.176.90:1443/", // testnet
    network: "testnet"
};
const proofs = true; // Enable proof verification
const sdk = await WasmSdk.new(transport, proofs);
```

## Authentication
Most state transitions require authentication:
```javascript
const identityHex = "hex_encoded_identity";
const privateKeyHex = "hex_encoded_private_key";
```

## Query Operations

### Pattern
All queries follow this pattern:
```javascript
const result = await sdk.{query_name}(param1, param2, ...);
```

### Available Queries
'''
    
    # Document all queries
    for cat_key, category in query_defs.items():
        md_content += f"\n#### {category.get('label', cat_key)}\n"
        
        queries = category.get('queries', {})
        for query_key, query in queries.items():
            md_content += f"\n**{query.get('label', query_key)}** - `{query_key}`\n"
            md_content += f"*{query.get('description', 'No description')}*\n\n"
            
            # Parameters
            inputs = query.get('inputs', [])
            if inputs:
                md_content += "Parameters:\n"
                for param in inputs:
                    req = "required" if param.get('required', False) else "optional"
                    md_content += f"- `{param.get('name', 'unknown')}` ({param.get('type', 'text')}, {req})"
                    
                    if param.get('label') and param.get('label') != param.get('name'):
                        md_content += f" - {param.get('label')}"
                    
                    if param.get('placeholder'):
                        md_content += f"\n  - Example: `{param.get('placeholder')}`"
                    
                    if param.get('options'):
                        md_content += "\n  - Options: "
                        opts = [f"`{opt.get('value')}` ({opt.get('label')})" for opt in param.get('options', [])]
                        md_content += ', '.join(opts)
                    
                    md_content += "\n"
            else:
                md_content += "No parameters required.\n"
            
            # Example
            md_content += f"\nExample:\n```javascript\n"
            
            # Generate example based on query type
            if query_key == 'getIdentity':
                md_content += 'const identity = await sdk.getIdentity("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec");'
            elif query_key == 'getDocuments':
                md_content += '''const docs = await sdk.getDocuments(
    contractId,
    "note",
    JSON.stringify([["$ownerId", "==", identityId]]),
    JSON.stringify([["$createdAt", "desc"]]),
    10
);'''
            elif query_key == 'getIdentityBalance':
                md_content += 'const balance = await sdk.getIdentityBalance(identityId);'
            elif query_key == 'getPathElements':
                md_content += '''// Access any data in the Dash Platform state tree
// Common root paths:
// - DataContractDocuments: 64
// - Identities: 32
// - UniquePublicKeyHashesToIdentities: 24
// - NonUniquePublicKeyKeyHashesToIdentities: 8
// - Tokens: 16
// - Pools: 48
// - PreFundedSpecializedBalances: 40
// - SpentAssetLockTransactions: 72
// - WithdrawalTransactions: 80
// - GroupActions: 88
// - Balances: 96
// - Misc: 104
// - Votes: 112
// - Versions: 120

// Example: Get identity balance
const result = await sdk.getPathElements(['96'], ['identityId']);

// Example: Get identity info
const identityKeys = await sdk.getPathElements(['32'], ['identityId']);

// Example: Get contract documents
const documents = await sdk.getPathElements(['64'], ['contractId', '1', 'documentType']);'''
            else:
                # Generic example
                params = []
                for inp in inputs:
                    if inp.get('required', False):
                        if inp.get('type') == 'array':
                            params.append('[]')
                        elif inp.get('type') == 'number':
                            params.append('100')
                        else:
                            params.append(f'"{inp.get("name", "value")}"')
                md_content += f'const result = await sdk.{query_key}({", ".join(params)});'
            
            md_content += "\n```\n"
    
    # Document state transitions
    md_content += "\n## State Transition Operations\n\n### Pattern\n"
    md_content += "All state transitions require authentication and follow this pattern:\n"
    md_content += "```javascript\nconst result = await sdk.{transition_name}(identityHex, ...params, privateKeyHex);\n```\n"
    
    md_content += "\n### Available State Transitions\n"
    
    for cat_key, category in transition_defs.items():
        md_content += f"\n#### {category.get('label', cat_key)}\n"
        
        transitions = category.get('transitions', {})
        for trans_key, transition in transitions.items():
            md_content += f"\n**{transition.get('label', trans_key)}** - `{trans_key}`\n"
            md_content += f"*{transition.get('description', 'No description')}*\n\n"
            
            # Parameters - use sdk_params if available, otherwise fall back to inputs
            sdk_params = transition.get('sdk_params', [])
            inputs = transition.get('inputs', [])
            params_to_use = sdk_params if sdk_params else inputs
            
            # Adjust parameter section header based on whether we're using SDK params
            if sdk_params:
                md_content += "Parameters:\n"
            elif inputs:
                md_content += "Parameters (in addition to identity/key):\n"
            
            if params_to_use:
                for param in params_to_use:
                    req = "required" if param.get('required', False) else "optional"
                    md_content += f"- `{param.get('name', 'unknown')}` ({param.get('type', 'text')}, {req})"
                    
                    if param.get('label') and param.get('label') != param.get('name'):
                        md_content += f" - {param.get('label')}"
                    
                    if param.get('description'):
                        md_content += f"\n  - {param.get('description')}"
                    elif param.get('placeholder'):
                        md_content += f"\n  - Example: `{param.get('placeholder')}`"
                    
                    md_content += "\n"
            
            # Example
            md_content += f"\nExample:\n```javascript\n"
            
            # Check if there's a custom sdk_example
            sdk_example = transition.get('sdk_example')
            if sdk_example:
                md_content += sdk_example
            elif trans_key == 'documentCreate':
                md_content += '''const result = await sdk.document_create(
    identityHex,
    contractId,
    "note",
    JSON.stringify({ message: "Hello!" }),
    privateKeyHex
);'''
            elif trans_key == 'tokenTransfer':
                md_content += '''const result = await sdk.token_transfer(
    identityHex,
    contractId,
    tokenId,
    1000000, // amount
    recipientId,
    privateKeyHex
);'''
            else:
                md_content += f'const result = await sdk.{trans_key}(identityHex, /* params */, privateKeyHex);'
            
            md_content += "\n```\n"
    
    # Add common patterns section
    md_content += '''
## Common Patterns

### Error Handling
```javascript
try {
    const result = await sdk.getIdentity(identityId);
    console.log(result);
} catch (error) {
    console.error("Query failed:", error);
}
```

### Working with Proofs
```javascript
// Enable proofs during SDK initialization
const sdk = await WasmSdk.new(transport, true);

// Query with proof verification
const identityWithProof = await sdk.getIdentity(identityId);
```

### Document Queries with Where/OrderBy
```javascript
// Where clause format: [[field, operator, value], ...]
const whereClause = JSON.stringify([
    ["$ownerId", "==", identityId],
    ["age", ">=", 18]
]);

// OrderBy format: [[field, direction], ...]
const orderBy = JSON.stringify([
    ["$createdAt", "desc"]
]);

const docs = await sdk.getDocuments(
    contractId,
    documentType,
    whereClause,
    orderBy,
    limit
);
```

### Batch Operations
```javascript
// Get multiple identities
const identityIds = ["id1", "id2", "id3"];
const balances = await sdk.getIdentitiesBalances(identityIds);
```

## Important Notes

1. **Network Endpoints**: 
   - Testnet: `https://52.12.176.90:1443/`
   - Mainnet: Update when available

2. **Identity Format**: Identity IDs and keys should be hex-encoded strings

3. **Credits**: All fees are paid in credits (1 credit = 1 satoshi equivalent)

4. **Nonces**: The SDK automatically handles nonce management for state transitions

5. **Proofs**: Enable proofs for production applications to ensure data integrity

## Troubleshooting

- **Connection errors**: Verify network endpoint and that SDK is initialized
- **Invalid parameters**: Check parameter types and required fields
- **Authentication failures**: Ensure correct identity/key format and key permissions
- **Query errors**: Validate contract IDs, document types, and field names exist
'''
    
    return md_content

def main():
    """Main function to generate documentation"""
    
    # Get paths
    script_dir = Path(__file__).parent
    
    # Load API definitions from api-definitions.json
    api_definitions_file = script_dir / 'api-definitions.json'
    
    if not api_definitions_file.exists():
        print(f"Error: api-definitions.json not found at {api_definitions_file}")
        return 1
    
    print("Loading API definitions from api-definitions.json...")
    try:
        query_defs, transition_defs = load_api_definitions(api_definitions_file)
        
        # Check if loading failed (returns empty dictionaries on error)
        if not query_defs or not transition_defs:
            print("Error: Failed to load API definitions or definitions are empty")
            return 1
            
        # Validate that we have actual data
        query_count = sum(len(cat.get('queries', {})) for cat in query_defs.values())
        transition_count = sum(len(cat.get('transitions', {})) for cat in transition_defs.values())
        
        if query_count == 0 and transition_count == 0:
            print("Error: No queries or state transitions found in API definitions")
            return 1
            
        print(f"Found {query_count} queries")
        print(f"Found {transition_count} state transitions")
        
    except Exception as e:
        print(f"Error: Failed to load API definitions: {e}")
        return 1
    
    # API definitions are already clean from JSON source - no cleanup needed
    
    # Generate user docs
    print("\nGenerating user documentation (docs.html)...")
    user_docs_html = generate_user_docs_html(query_defs, transition_defs)
    docs_file = script_dir / 'docs.html'
    with open(docs_file, 'w', encoding='utf-8') as f:
        f.write(user_docs_html)
    print(f"Written to {docs_file}")
    
    # Generate AI reference
    print("\nGenerating AI reference (AI_REFERENCE.md)...")
    ai_reference_md = generate_ai_reference_md(query_defs, transition_defs)
    ai_ref_file = script_dir / 'AI_REFERENCE.md'
    with open(ai_ref_file, 'w', encoding='utf-8') as f:
        f.write(ai_reference_md)
    print(f"Written to {ai_ref_file}")
    
    # Generate documentation manifest for CI checks
    manifest = {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "queries": {},
        "transitions": {}
    }
    
    for cat_key, category in query_defs.items():
        for query_key in category.get('queries', {}).keys():
            manifest["queries"][query_key] = {
                "category": cat_key,
                "documented": True
            }
    
    for cat_key, category in transition_defs.items():
        for trans_key in category.get('transitions', {}).keys():
            manifest["transitions"][trans_key] = {
                "category": cat_key,
                "documented": True
            }
    
    manifest_file = script_dir / 'docs_manifest.json'
    with open(manifest_file, 'w', encoding='utf-8') as f:
        json.dump(manifest, f, indent=2)
    print(f"\nGenerated documentation manifest at {manifest_file}")
    
    print("\nDocumentation generation complete!")
    return 0

if __name__ == "__main__":
    exit(main())