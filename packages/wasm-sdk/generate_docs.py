#!/usr/bin/env python3
"""
Documentation generator for WASM JS SDK
Extracts query and state transition definitions from index.html
and generates both user documentation (HTML) and AI reference (Markdown)
"""

import os
import re
import json
import html as html_lib
import html
from pathlib import Path
from datetime import datetime

def extract_definitions_from_html(html_content):
    """Extract query and state transition definitions from index.html"""
    
    # Extract queryDefinitions
    query_match = re.search(r'const queryDefinitions = ({[\s\S]*?});(?=\s*(?:const|//|$))', html_content)
    query_definitions = {}
    if query_match:
        # Clean up the JavaScript object to make it JSON-parseable
        query_str = query_match.group(1)
        # Convert JS object to JSON format
        query_str = re.sub(r'(\w+):', r'"\1":', query_str)  # Add quotes to keys
        query_str = re.sub(r',\s*}', '}', query_str)  # Remove trailing commas
        query_str = re.sub(r',\s*]', ']', query_str)  # Remove trailing commas in arrays
        # Handle functions and complex values
        query_str = re.sub(r'dependsOn:\s*{[^}]+}', '"dependsOn": {}', query_str)
        query_str = re.sub(r'action:\s*"[^"]*"', '"action": ""', query_str)
        query_str = re.sub(r'dynamic:\s*true', '"dynamic": true', query_str)
        query_str = re.sub(r'defaultValue:\s*true', '"defaultValue": true', query_str)
        query_str = re.sub(r'validateOnType:\s*true', '"validateOnType": true', query_str)
        
        try:
            query_definitions = json.loads(query_str)
        except json.JSONDecodeError as e:
            print(f"Warning: Could not parse query definitions: {e}")
            # Fallback to regex extraction
            query_definitions = extract_definitions_regex(html_content, 'queryDefinitions')
    
    # Extract stateTransitionDefinitions
    transition_match = re.search(r'const stateTransitionDefinitions = ({[\s\S]*?});(?=\s*(?:const|//|$))', html_content)
    transition_definitions = {}
    if transition_match:
        trans_str = transition_match.group(1)
        trans_str = re.sub(r'(\w+):', r'"\1":', trans_str)
        trans_str = re.sub(r',\s*}', '}', trans_str)
        trans_str = re.sub(r',\s*]', ']', trans_str)
        trans_str = re.sub(r'dependsOn:\s*{[^}]+}', '"dependsOn": {}', trans_str)
        trans_str = re.sub(r'action:\s*"[^"]*"', '"action": ""', trans_str)
        trans_str = re.sub(r'dynamic:\s*true', '"dynamic": true', trans_str)
        trans_str = re.sub(r'defaultValue:\s*true', '"defaultValue": true', trans_str)
        trans_str = re.sub(r'validateOnType:\s*true', '"validateOnType": true', trans_str)
        
        try:
            transition_definitions = json.loads(trans_str)
        except json.JSONDecodeError as e:
            print(f"Warning: Could not parse transition definitions: {e}")
            transition_definitions = extract_definitions_regex(html_content, 'stateTransitionDefinitions')
    
    return query_definitions, transition_definitions

def extract_definitions_regex(html_content, definition_name):
    """Robust regex extraction method for JavaScript object definitions"""
    definitions = {}
    
    # Find the complete definition block
    pattern = rf'const {definition_name} = (\{{(?:[^{{}}]|{{[^{{}}]*}})*\}});'
    match = re.search(pattern, html_content, re.DOTALL)
    if not match:
        return definitions
    
    full_def = match.group(1)
    
    # Extract each category
    cat_pattern = r'(\w+):\s*\{[^}]*label:\s*"([^"]+)"[^}]*(?:queries|transitions):\s*\{([^}]+(?:\{[^}]*\}[^}]*)*)\}'
    
    for cat_match in re.finditer(cat_pattern, full_def, re.DOTALL):
        cat_key = cat_match.group(1)
        cat_label = cat_match.group(2)
        items_block = cat_match.group(3)
        
        items_key = 'queries' if 'query' in definition_name else 'transitions'
        definitions[cat_key] = {
            'label': cat_label,
            items_key: {}
        }
        
        # Extract individual queries/transitions
        item_pattern = r'(\w+):\s*\{([^}]+(?:\{[^}]*\}[^}]*)*)\}'
        
        for item_match in re.finditer(item_pattern, items_block):
            item_key = item_match.group(1)
            item_content = item_match.group(2)
            
            # Extract item details
            item_data = {}
            
            # Extract label
            label_match = re.search(r'label:\s*"([^"]+)"', item_content)
            if label_match:
                item_data['label'] = label_match.group(1)
            
            # Extract description
            desc_match = re.search(r'description:\s*"([^"]+)"', item_content)
            if desc_match:
                item_data['description'] = desc_match.group(1)
            
            # Extract inputs array
            inputs_match = re.search(r'inputs:\s*\[([^\]]+(?:\[[^\]]*\][^\]]*)*)\]', item_content, re.DOTALL)
            if inputs_match:
                inputs_str = inputs_match.group(1)
                item_data['inputs'] = extract_inputs(inputs_str)
            else:
                item_data['inputs'] = []
            
            definitions[cat_key][items_key][item_key] = item_data
    
    return definitions

def generate_example_code(query_key, inputs):
    """Generate example code for a query"""
    
    # Test data for various query types
    # Using a known testnet identity with activity
    test_data = {
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
    
    # Map input names to test values
    param_mapping = {
        'id': f"'{test_data['data_contract_history_id']}'" if 'getDataContractHistory' in query_key else f"'{test_data['data_contract_id']}'" if 'getDataContract' in query_key else f"'{test_data['identity_id']}'",
        'identityId': f"'{test_data['specialized_balance_id']}'" if 'getPrefundedSpecializedBalance' in query_key else "'5RG84o6KsTaZudDqS8ytbaRB8QP4YYQ2uwzb6Hj8cfjX'" if 'getTokenPerpetualDistributionLastClaim' in query_key else f"'{test_data['identity_id']}'",
        'ids': f"['{test_data['data_contract_id']}', '{test_data['token_contract_id']}']" if 'getDataContracts' in query_key else f"['{test_data['pro_tx_hash']}']" if 'Evonodes' in query_key else f"['{test_data['identity_id']}']",
        'identitiesIds': f"['{test_data['identity_id']}']",
        'identityIds': f"['{test_data['identity_id']}']",
        'contractId': f"'{test_data['group_contract_id']}'" if ('group' in query_key.lower() or 'Group' in query_key) else f"'{test_data['token_contract_id']}'" if ('token' in query_key.lower() or 'Token' in query_key) and 'TokenBalance' not in query_key and 'TokenInfo' not in query_key else f"'{test_data['contract_id']}'",
        'dataContractId': "'EETVvWgohFDKtbB3ejEzBcDRMNYkc9TtgXY6y8hzP3Ta'" if 'getTokenContractInfo' in query_key else f"'{test_data['data_contract_id']}'",
        'publicKeyHash': f"'{test_data['public_key_hash_unique']}'" if 'ByPublicKeyHash' in query_key and 'NonUnique' not in query_key else f"'{test_data['public_key_hash_non_unique']}'",
        'startProTxHash': f"'{test_data['pro_tx_hash']}'",
        'tokenId': "'HEv1AYWQfwCffXQgmuzmzyzUo9untRTmVr67n4e4PSWa'" if 'getTokenPerpetualDistributionLastClaim' in query_key else f"'{test_data['token_id']}'",
        'tokenIds': f"['{test_data['token_id']}', 'H7FRpZJqZK933r9CzZMsCuf1BM34NT5P2wSJyjDkprqy']" if 'getTokenStatuses' in query_key else "['H7FRpZJqZK933r9CzZMsCuf1BM34NT5P2wSJyjDkprqy']" if 'getTokenDirectPurchasePrices' in query_key else f"['{test_data['token_id']}']",
        'documentType': f"'{test_data['document_type']}'",
        'documentId': f"'{test_data['document_id']}'",
        'label': f"'{test_data['username']}'",
        'name': f"'{test_data['username']}'",
        'epoch': '1000' if 'getEpochsInfo' in query_key else test_data['epoch'],
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
        'contestantId': f"'{test_data['identity_id']}'",
        'amount': '1000000',
        'recipientId': f"'{test_data['identity_id']}'",
        'toAddress': "'yNPbcFfabtNmmxKdGwhHomdYfVs6gikbPf'",
        'whereClause': 'JSON.stringify([["normalizedParentDomainName", "==", "dash"]])',
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
        'keys': f"['{test_data['identity_id']}']",
        'stateTransitionHash': "'0000000000000000000000000000000000000000000000000000000000000000'",
        'allowIncludeLockedAndAbstainingVoteTally': 'null',
        'startAtValue': 'null',
        'startAtIdentifierInfo': 'null',
        'indexValues': "['dash', 'alice']",
        'startAtVoterInfo': 'null',
        'startAtVotePollIdInfo': 'null',
        'startTimeInfo': '(Date.now() - 86400000).toString()',
        'endTimeInfo': 'Date.now().toString()'
    }
    
    # Handle special cases for functions with structured parameters
    if query_key == 'getGroupInfos':
        # getGroupInfos expects: sdk, contractId, startAtInfo (object or null), count
        params = [f"'{test_data['group_contract_id']}'", "null", "100"]
    elif query_key == 'getGroupActions':
        # getGroupActions expects: sdk, contractId, groupContractPosition, status, startAtInfo (object or null), count
        params = [f"'{test_data['group_contract_id']}'", "0", "'ACTIVE'", "null", "100"]
    elif query_key == 'getDataContractHistory':
        # getDataContractHistory expects: sdk, id, limit, offset, startAtMs
        # Use the specific contract ID for getDataContractHistory examples
        params = ["'HLY575cNazmc5824FxqaEMEBuzFeE4a98GDRNKbyJqCM'", "10", "0"]
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
    
    # Handle special function name mappings
    func_name_map = {
        'getIdentity': 'identity_fetch',
        'getIdentityKeys': 'get_identity_keys',
        'getIdentitiesContractKeys': 'get_identities_contract_keys',
        'getIdentityNonce': 'get_identity_nonce',
        'getIdentityContractNonce': 'get_identity_contract_nonce',
        'getIdentityBalance': 'get_identity_balance',
        'getIdentitiesBalances': 'get_identities_balances',
        'getIdentityBalanceAndRevision': 'get_identity_balance_and_revision',
        'getIdentityByPublicKeyHash': 'get_identity_by_public_key_hash',
        'getIdentityByNonUniquePublicKeyHash': 'get_identity_by_non_unique_public_key_hash',
        'getDataContract': 'data_contract_fetch',
        'getDataContractHistory': 'get_data_contract_history',
        'getDataContracts': 'get_data_contracts',
        'getDocuments': 'get_documents',
        'getDocument': 'get_document',
        'getDpnsUsername': 'get_dpns_usernames',
        'dpnsCheckAvailability': 'dpns_is_name_available',
        'dpnsResolve': 'dpns_resolve_name',
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
        'getIdentityTokenBalances': 'get_identity_token_balances',
        'getIdentitiesTokenBalances': 'get_identities_token_balances',
        'getIdentityTokenInfos': 'get_identity_token_infos',
        'getIdentitiesTokenInfos': 'get_identities_token_infos',
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
    
    func_name = func_name_map.get(query_key, query_key)
    
    # Add sdk as first parameter for all functions
    all_params = ['sdk'] + params
    
    # Generate the function call (functions are imported directly, not methods on sdk)
    return f'return await window.wasmFunctions.{func_name}({", ".join(all_params)});'

def extract_inputs(inputs_str):
    """Extract input definitions from JavaScript array string"""
    inputs = []
    
    # Match individual input objects
    input_pattern = r'\{([^}]+(?:\{[^}]*\}[^}]*)*)\}'
    
    for match in re.finditer(input_pattern, inputs_str):
        input_content = match.group(1)
        input_data = {}
        
        # Extract name
        name_match = re.search(r'name:\s*"([^"]+)"', input_content)
        if name_match:
            input_data['name'] = name_match.group(1)
        
        # Extract type
        type_match = re.search(r'type:\s*"([^"]+)"', input_content)
        if type_match:
            input_data['type'] = type_match.group(1)
        
        # Extract label
        label_match = re.search(r'label:\s*"([^"]+)"', input_content)
        if label_match:
            input_data['label'] = label_match.group(1)
        
        # Extract required
        req_match = re.search(r'required:\s*(true|false)', input_content)
        if req_match:
            input_data['required'] = req_match.group(1) == 'true'
        
        # Extract placeholder
        placeholder_match = re.search(r'placeholder:\s*["\']([^"\']+)["\']', input_content)
        if placeholder_match:
            input_data['placeholder'] = placeholder_match.group(1)
        
        # Extract options if present
        options_match = re.search(r'options:\s*\[([^\]]+)\]', input_content)
        if options_match:
            options_str = options_match.group(1)
            input_data['options'] = []
            
            # Extract each option
            opt_pattern = r'\{\s*value:\s*"([^"]+)"[^}]*label:\s*"([^"]+)"[^}]*\}'
            for opt_match in re.finditer(opt_pattern, options_str):
                input_data['options'].append({
                    'value': opt_match.group(1),
                    'label': opt_match.group(2)
                })
        
        if input_data:
            inputs.append(input_data)
    
    return inputs

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
    
    inputs = operation.get('inputs', [])
    if not inputs:
        html_content += '                <p class="param-optional">No parameters required</p>'
    else:
        for param in inputs:
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
        html_content += f'                <div class="example-code">{generate_transition_example(operation_key)}</div>'
    
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
    if param.get('placeholder'):
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

def generate_transition_example(trans_key):
    """Generate example code for state transitions"""
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

def generate_user_docs_html(query_defs, transition_defs):
    """Generate user-friendly HTML documentation"""
    
    html_content = '''<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Dash Platform WASM JS SDK Documentation</title>
    <link rel="icon" type="image/svg+xml" href="https://media.dash.org/wp-content/uploads/blue-d.svg">
    <link rel="alternate icon" type="image/png" href="https://media.dash.org/wp-content/uploads/blue-d-250.png">
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
            line-height: 1.6;
            color: #333;
            margin: 0;
            padding: 0;
            background-color: #f5f5f5;
            display: flex;
        }
        
        /* Sidebar styles */
        .sidebar {
            width: 280px;
            background-color: white;
            box-shadow: 2px 0 4px rgba(0,0,0,0.1);
            position: fixed;
            height: 100vh;
            overflow-y: auto;
            padding: 20px;
        }
        
        .sidebar h2 {
            font-size: 1.2em;
            margin-bottom: 10px;
            color: #2c3e50;
        }
        
        .sidebar ul {
            list-style: none;
            padding: 0;
            margin: 0 0 20px 0;
        }
        
        .sidebar li {
            margin-bottom: 5px;
        }
        
        .sidebar a {
            color: #34495e;
            text-decoration: none;
            font-size: 0.9em;
            display: block;
            padding: 5px 10px;
            border-radius: 3px;
            transition: background-color 0.2s;
        }
        
        .sidebar a:hover {
            background-color: #ecf0f1;
            color: #2c3e50;
        }
        
        .sidebar .section-header {
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            padding: 12px 20px;
            margin: 20px -20px 15px -20px;
            font-weight: 600;
            font-size: 0.9em;
            text-transform: uppercase;
            letter-spacing: 0.5px;
            position: relative;
            overflow: hidden;
        }
        
        .sidebar .section-header:before {
            content: '';
            position: absolute;
            top: 0;
            left: 0;
            right: 0;
            bottom: 0;
            background: rgba(255, 255, 255, 0.1);
            transform: translateX(-100%);
            transition: transform 0.6s ease;
        }
        
        .sidebar .section-header:hover:before {
            transform: translateX(0);
        }
        
        .sidebar .section-header.state-transitions {
            background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
        }
        
        .sidebar .category {
            font-weight: 600;
            color: #34495e;
            margin-top: 15px;
            margin-bottom: 8px;
            font-size: 0.85em;
            padding-left: 10px;
            border-left: 3px solid #3498db;
        }
        
        /* Search box styles */
        .search-container {
            padding: 0 20px 20px 20px;
            border-bottom: 1px solid #ecf0f1;
        }
        
        .search-input {
            width: 100%;
            padding: 8px 12px;
            border: 1px solid #ddd;
            border-radius: 4px;
            font-size: 0.9em;
            outline: none;
            transition: border-color 0.2s;
        }
        
        .search-input:focus {
            border-color: #3498db;
        }
        
        .search-input::placeholder {
            color: #95a5a6;
        }
        
        .sidebar li.hidden {
            display: none;
        }
        
        .sidebar .no-results {
            text-align: center;
            color: #95a5a6;
            padding: 20px;
            font-size: 0.9em;
            display: none;
        }
        
        /* Main content styles */
        .main-content {
            margin-left: 320px;
            padding: 20px 40px;
            max-width: 900px;
        }
        
        h1, h2, h3, h4 {
            color: #2c3e50;
        }
        
        h1 {
            border-bottom: 3px solid #3498db;
            padding-bottom: 10px;
        }
        
        h2 {
            border-bottom: 2px solid #ecf0f1;
            padding-bottom: 8px;
            margin-top: 30px;
        }
        
        h3 {
            color: #34495e;
            margin-top: 25px;
        }
        
        .nav {
            background-color: white;
            padding: 15px;
            border-radius: 8px;
            margin-bottom: 30px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }
        
        .nav ul {
            list-style: none;
            padding: 0;
            margin: 0;
        }
        
        .nav li {
            display: inline-block;
            margin-right: 20px;
        }
        
        .nav a {
            color: #3498db;
            text-decoration: none;
            font-weight: 500;
        }
        
        .nav a:hover {
            text-decoration: underline;
        }
        
        .category {
            background-color: white;
            padding: 20px;
            border-radius: 8px;
            margin-bottom: 20px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }
        
        .operation {
            border-left: 4px solid #3498db;
            padding-left: 20px;
            margin-bottom: 30px;
        }
        
        .description {
            color: #7f8c8d;
            font-style: italic;
            margin-bottom: 15px;
        }
        
        .parameters {
            background-color: #ecf0f1;
            padding: 15px;
            border-radius: 5px;
            margin-top: 10px;
        }
        
        .parameter {
            margin-bottom: 10px;
            padding: 5px 0;
            border-bottom: 1px solid #bdc3c7;
        }
        
        .parameter:last-child {
            border-bottom: none;
        }
        
        .param-name {
            font-weight: bold;
            color: #2c3e50;
        }
        
        .param-type {
            color: #e74c3c;
            font-family: monospace;
            font-size: 0.9em;
        }
        
        .param-required {
            color: #e74c3c;
            font-weight: bold;
        }
        
        .param-optional {
            color: #95a5a6;
        }
        
        .code-example {
            background-color: #2c3e50;
            color: #ecf0f1;
            padding: 15px;
            border-radius: 5px;
            overflow-x: auto;
            font-family: monospace;
            margin-top: 10px;
        }
        
        /* Interactive example styles */
        .example-container {
            background-color: #f8f9fa;
            border: 1px solid #dee2e6;
            border-radius: 5px;
            padding: 15px;
            margin-top: 15px;
        }
        
        .example-code {
            background-color: #2c3e50;
            color: #ecf0f1;
            padding: 10px;
            border-radius: 3px;
            font-family: monospace;
            font-size: 0.9em;
            margin-bottom: 10px;
            position: relative;
        }
        
        .run-button {
            background-color: #3498db;
            color: white;
            border: none;
            padding: 8px 16px;
            border-radius: 3px;
            cursor: pointer;
            font-weight: 500;
            transition: background-color 0.2s;
        }
        
        .run-button:hover {
            background-color: #2980b9;
        }
        
        .run-button:disabled {
            background-color: #95a5a6;
            cursor: not-allowed;
        }
        
        .example-result {
            margin-top: 10px;
            padding: 10px;
            border-radius: 3px;
            font-family: monospace;
            font-size: 0.85em;
            display: none;
        }
        
        .example-result.success {
            background-color: #d4edda;
            border: 1px solid #c3e6cb;
            color: #155724;
        }
        
        .example-result.error {
            background-color: #f8d7da;
            border: 1px solid #f5c6cb;
            color: #721c24;
        }
        
        .loading {
            display: inline-block;
            width: 20px;
            height: 20px;
            border: 3px solid rgba(255,255,255,.3);
            border-radius: 50%;
            border-top-color: #fff;
            animation: spin 1s ease-in-out infinite;
        }
        
        @keyframes spin {
            to { transform: rotate(360deg); }
        }
        
        .back-to-top {
            position: fixed;
            bottom: 20px;
            right: 20px;
            background-color: #3498db;
            color: white;
            padding: 10px 15px;
            border-radius: 5px;
            text-decoration: none;
            box-shadow: 0 2px 4px rgba(0,0,0,0.2);
        }
        
        .back-to-top:hover {
            background-color: #2980b9;
        }
        
        .info-note {
            background-color: #e3f2fd;
            color: #1565c0;
            padding: 12px 16px;
            border-radius: 4px;
            font-size: 0.9em;
            margin: 10px 0;
            border-left: 4px solid #1976d2;
        }
        
        .path-info {
            background-color: #f5f7fa;
            border: 1px solid #e1e5eb;
            border-radius: 4px;
            padding: 15px;
            margin-top: 15px;
        }
        
        .path-info h6 {
            margin-top: 15px;
            margin-bottom: 10px;
            color: #2c3e50;
            font-size: 0.95em;
        }
        
        .path-info h6:first-child {
            margin-top: 0;
        }
        
        .path-table {
            width: 100%;
            border-collapse: collapse;
            margin-bottom: 15px;
        }
        
        .path-table th {
            background-color: #e9ecef;
            padding: 8px 12px;
            text-align: left;
            font-weight: 600;
            border: 1px solid #dee2e6;
        }
        
        .path-table td {
            padding: 8px 12px;
            border: 1px solid #dee2e6;
        }
        
        .path-table code {
            background-color: #fff;
            padding: 2px 6px;
            border-radius: 3px;
            font-family: monospace;
        }
        
        .path-info ul {
            margin: 0;
            padding-left: 25px;
        }
        
        .path-info li {
            margin-bottom: 5px;
            line-height: 1.6;
        }
        
        .path-info li code {
            background-color: #fff;
            padding: 2px 6px;
            border-radius: 3px;
            font-family: monospace;
        }
        
        /* Preloader styles */
        #preloader {
            display: none;
            position: fixed;
            top: 0;
            left: 0;
            width: 100%;
            height: 100%;
            background-color: rgba(0, 0, 0, 0.8);
            z-index: 9999;
            display: flex;
            justify-content: center;
            align-items: center;
        }
        
        .preloader-content {
            text-align: center;
            background: white;
            padding: 30px 50px;
            border-radius: 10px;
            box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
        }
        
        .preloader-text {
            font-size: 16px;
            margin-bottom: 15px;
            color: #333;
        }
        
        .preloader-progress {
            margin-top: 20px;
        }
        
        .progress-bar {
            width: 300px;
            height: 20px;
            background-color: #f0f0f0;
            border-radius: 10px;
            overflow: hidden;
            margin-bottom: 10px;
        }
        
        .progress-fill {
            height: 100%;
            background: linear-gradient(90deg, #4CAF50, #45a049);
            width: 0%;
            transition: width 0.3s ease;
        }
        
        .progress-percent {
            font-size: 14px;
            font-weight: bold;
            color: #333;
        }
    </style>
    <script type="module">
        import init, { 
            WasmSdkBuilder,
            identity_fetch,
            get_identity_keys,
            get_identity_nonce,
            get_identity_contract_nonce,
            get_identity_balance,
            get_identities_balances,
            get_identity_balance_and_revision,
            get_identity_by_public_key_hash,
            get_identity_by_non_unique_public_key_hash,
            get_identities_contract_keys,
            get_identity_token_balances,
            get_identities_token_balances,
            get_identity_token_infos,
            get_identities_token_infos,
            data_contract_fetch,
            get_data_contract_history,
            get_data_contracts,
            get_documents,
            get_document,
            get_dpns_usernames,
            dpns_is_name_available,
            dpns_resolve_name,
            get_contested_resources,
            get_contested_resource_vote_state,
            get_contested_resource_voters_for_identity,
            get_contested_resource_identity_votes,
            get_vote_polls_by_end_date,
            get_protocol_version_upgrade_state,
            get_protocol_version_upgrade_vote_status,
            get_epochs_info,
            get_current_epoch,
            get_finalized_epoch_infos,
            get_evonodes_proposed_epoch_blocks_by_ids,
            get_evonodes_proposed_epoch_blocks_by_range,
            get_token_statuses,
            get_token_direct_purchase_prices,
            get_token_contract_info,
            get_token_perpetual_distribution_last_claim,
            get_token_total_supply,
            get_group_info,
            get_group_infos,
            get_group_actions,
            get_group_action_signers,
            get_status,
            get_current_quorums_info,
            get_prefunded_specialized_balance,
            get_total_credits_in_platform,
            get_path_elements,
            wait_for_state_transition_result,
            prefetch_trusted_quorums_testnet
        } from './pkg/wasm_sdk.js';
        
        let sdk = null;
        let isInitialized = false;
        
        // Make functions available globally for the examples
        window.wasmFunctions = {
            identity_fetch,
            get_identity_keys,
            get_identity_nonce,
            get_identity_contract_nonce,
            get_identity_balance,
            get_identities_balances,
            get_identity_balance_and_revision,
            get_identity_by_public_key_hash,
            get_identity_by_non_unique_public_key_hash,
            get_identities_contract_keys,
            get_identity_token_balances,
            get_identities_token_balances,
            get_identity_token_infos,
            get_identities_token_infos,
            data_contract_fetch,
            get_data_contract_history,
            get_data_contracts,
            get_documents,
            get_document,
            get_dpns_usernames,
            dpns_is_name_available,
            dpns_resolve_name,
            get_contested_resources,
            get_contested_resource_vote_state,
            get_contested_resource_voters_for_identity,
            get_contested_resource_identity_votes,
            get_vote_polls_by_end_date,
            get_protocol_version_upgrade_state,
            get_protocol_version_upgrade_vote_status,
            get_epochs_info,
            get_current_epoch,
            get_finalized_epoch_infos,
            get_evonodes_proposed_epoch_blocks_by_ids,
            get_evonodes_proposed_epoch_blocks_by_range,
            get_token_statuses,
            get_token_direct_purchase_prices,
            get_token_contract_info,
            get_token_perpetual_distribution_last_claim,
            get_token_total_supply,
            get_group_info,
            get_group_infos,
            get_group_actions,
            get_group_action_signers,
            get_status,
            get_current_quorums_info,
            get_prefunded_specialized_balance,
            get_total_credits_in_platform,
            get_path_elements,
            wait_for_state_transition_result
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
                preloader.style.display = 'flex';
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
                    preloader.style.display = 'none';
                }, isCached ? 200 : 500);
            } catch (error) {
                console.error('Failed to initialize SDK:', error);
                sdk = null;
                isInitialized = false;
                preloader.style.display = 'none';
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
        });
    </script>
</head>
<body>
    <!-- Preloader -->
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
    </div>
    
    <!-- Sidebar -->
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
    html_content += generate_sidebar_entries(query_defs, 'query')
    
    html_content += '''        </ul>
        
        <div class="section-header state-transitions">State Transitions</div>
        <ul>
'''
    
    # Generate sidebar links for transitions
    html_content += generate_sidebar_entries(transition_defs, 'transition')
    
    html_content += '''        </ul>
    </div>
    
    <!-- Main Content -->
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
    html_content += generate_operation_docs(query_defs, 'query', 'query')
    
    # Add state transition documentation
    html_content += '<h2 id="transitions">State Transitions</h2>'
    html_content += generate_operation_docs(transition_defs, 'transition', 'transition')
    
    # Close main content div
    html_content += '''
        <a href="#" class="back-to-top">↑ Top</a>
    </div>
    
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
        // Hidden test runner feature
        
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
        }
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
            
            # Parameters
            inputs = transition.get('inputs', [])
            if inputs:
                md_content += "Parameters (in addition to identity/key):\n"
                for param in inputs:
                    req = "required" if param.get('required', False) else "optional"
                    md_content += f"- `{param.get('name', 'unknown')}` ({param.get('type', 'text')}, {req})"
                    
                    if param.get('label') and param.get('label') != param.get('name'):
                        md_content += f" - {param.get('label')}"
                    
                    if param.get('placeholder'):
                        md_content += f"\n  - Example: `{param.get('placeholder')}`"
                    
                    md_content += "\n"
            
            # Example
            md_content += f"\nExample:\n```javascript\n"
            
            # Generate specific examples
            if trans_key == 'documentCreate':
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
    index_file = script_dir / 'index.html'
    
    if not index_file.exists():
        print(f"Error: index.html not found at {index_file}")
        return 1
    
    # First check if we have manually fixed definitions
    fixed_file = script_dir / 'fixed_definitions.json'
    extracted_file = script_dir / 'extracted_definitions.json'
    
    if fixed_file.exists():
        print("Using manually fixed definitions...")
        definitions_file = fixed_file
    else:
        # Extract definitions using the extraction script
        print("Extracting definitions from index.html...")
        import subprocess
        result = subprocess.run(['python3', 'extract_definitions.py'], cwd=script_dir, capture_output=True, text=True)
        if result.returncode != 0:
            print(f"Error extracting definitions: {result.stderr}")
            return 1
        
        if not extracted_file.exists():
            print("Error: Could not find extracted definitions")
            return 1
        definitions_file = extracted_file
    
    with open(definitions_file, 'r') as f:
        extracted = json.load(f)
    
    query_defs = extracted.get('queries', {})
    transition_defs = extracted.get('transitions', {})
    
    # Clean up the extracted data (remove invalid entries like 'dependsOn')
    for cat_key, category in list(query_defs.items()):
        queries = category.get('queries', {})
        for q_key in list(queries.keys()):
            if q_key in ['dependsOn', 'offset', 'limit'] or not queries[q_key].get('label'):
                del queries[q_key]
    
    for cat_key, category in list(transition_defs.items()):
        transitions = category.get('transitions', {})
        for t_key in list(transitions.keys()):
            if t_key in ['dependsOn'] or not transitions[t_key].get('label'):
                del transitions[t_key]
    
    print(f"Found {sum(len(cat.get('queries', {})) for cat in query_defs.values())} queries")
    print(f"Found {sum(len(cat.get('transitions', {})) for cat in transition_defs.values())} state transitions")
    
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
        "generated_at": datetime.now().isoformat(),
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