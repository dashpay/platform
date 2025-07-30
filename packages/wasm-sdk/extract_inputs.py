#!/usr/bin/env python3
"""
Extract input parameters from index.html and update fixed_definitions.json
"""

import json
import re
from pathlib import Path

def extract_inputs_from_html():
    """Extract input definitions from index.html"""
    index_file = Path(__file__).parent / 'index.html'
    
    with open(index_file, 'r', encoding='utf-8') as f:
        content = f.read()
    
    # Find the queryDefinitions and stateTransitionDefinitions objects
    query_match = re.search(r'const queryDefinitions = ({[^;]+});', content, re.DOTALL)
    trans_match = re.search(r'const stateTransitionDefinitions = ({[^;]+});', content, re.DOTALL)
    
    if not query_match or not trans_match:
        print("Could not find definitions in index.html")
        return {}
    
    # Extract the JavaScript objects
    query_js = query_match.group(1)
    trans_js = trans_match.group(1)
    
    # Parse inputs for each query/transition
    inputs_map = {}
    
    # Process queries
    for category_match in re.finditer(r'(\w+):\s*\{[^}]*queries:\s*\{([^}]+)\}', query_js, re.DOTALL):
        category = category_match.group(1)
        queries_block = category_match.group(2)
        
        for query_match in re.finditer(r'(\w+):\s*\{[^}]*inputs:\s*\[([^\]]*)\]', queries_block, re.DOTALL):
            query_name = query_match.group(1)
            inputs_str = query_match.group(2)
            
            # Parse inputs
            inputs = []
            for input_match in re.finditer(r'\{([^}]+)\}', inputs_str):
                input_str = input_match.group(1)
                # Extract properties
                name_match = re.search(r'name:\s*[\'"]([^\'\"]+)[\'"]', input_str)
                type_match = re.search(r'type:\s*[\'"]([^\'\"]+)[\'"]', input_str)
                label_match = re.search(r'label:\s*[\'"]([^\'\"]+)[\'"]', input_str)
                required_match = re.search(r'required:\s*(true|false)', input_str)
                
                if name_match:
                    input_def = {
                        'name': name_match.group(1),
                        'type': type_match.group(1) if type_match else 'text',
                        'label': label_match.group(1) if label_match else name_match.group(1),
                        'required': required_match.group(1) == 'true' if required_match else True
                    }
                    inputs.append(input_def)
            
            inputs_map[f'query.{query_name}'] = inputs
    
    # Process transitions
    for category_match in re.finditer(r'(\w+):\s*\{[^}]*transitions:\s*\{([^}]+)\}', trans_js, re.DOTALL):
        category = category_match.group(1)
        transitions_block = category_match.group(2)
        
        for trans_match in re.finditer(r'(\w+):\s*\{[^}]*inputs:\s*\[([^\]]*)\]', transitions_block, re.DOTALL):
            trans_name = trans_match.group(1)
            inputs_str = trans_match.group(2)
            
            # Parse inputs
            inputs = []
            for input_match in re.finditer(r'\{([^}]+)\}', inputs_str):
                input_str = input_match.group(1)
                # Extract properties
                name_match = re.search(r'name:\s*[\'"]([^\'\"]+)[\'"]', input_str)
                type_match = re.search(r'type:\s*[\'"]([^\'\"]+)[\'"]', input_str)
                label_match = re.search(r'label:\s*[\'"]([^\'\"]+)[\'"]', input_str)
                required_match = re.search(r'required:\s*(true|false)', input_str)
                
                if name_match:
                    input_def = {
                        'name': name_match.group(1),
                        'type': type_match.group(1) if type_match else 'text',
                        'label': label_match.group(1) if label_match else name_match.group(1),
                        'required': required_match.group(1) == 'true' if required_match else True
                    }
                    inputs.append(input_def)
            
            inputs_map[f'transition.{trans_name}'] = inputs
    
    return inputs_map

def update_fixed_definitions():
    """Update fixed_definitions.json with actual inputs"""
    # Load current definitions
    with open('fixed_definitions.json', 'r') as f:
        definitions = json.load(f)
    
    # Get inputs from HTML
    inputs_map = extract_inputs_from_html()
    
    # Update queries
    for category in definitions['queries'].values():
        for query_key, query in category.get('queries', {}).items():
            map_key = f'query.{query_key}'
            if map_key in inputs_map:
                query['inputs'] = inputs_map[map_key]
    
    # Update transitions
    for category in definitions['transitions'].values():
        for trans_key, trans in category.get('transitions', {}).items():
            map_key = f'transition.{trans_key}'
            if map_key in inputs_map:
                trans['inputs'] = inputs_map[map_key]
    
    # Save updated definitions
    with open('fixed_definitions.json', 'w') as f:
        json.dump(definitions, f, indent=2)
    
    print("Updated fixed_definitions.json with input parameters")

if __name__ == '__main__':
    update_fixed_definitions()