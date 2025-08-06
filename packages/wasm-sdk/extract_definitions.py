#!/usr/bin/env python3
"""
Extract query and state transition definitions from index.html
"""

import json
import re
from pathlib import Path

def manual_extract_definitions(html_file):
    """Manually extract definitions using targeted patterns"""
    
    with open(html_file, 'r', encoding='utf-8') as f:
        content = f.read()
    
    # Extract the complete queryDefinitions block (handle potential whitespace)
    import re
    query_match = re.search(r'\s*const queryDefinitions = \{', content)
    if not query_match:
        return {}, {}
    
    query_start = query_match.start()
    
    # Find the end by counting braces
    brace_count = 0
    pos = query_match.end() - 1  # Start at the opening brace
    query_end = pos
    
    for i, char in enumerate(content[pos:], pos):
        if char == '{':
            brace_count += 1
        elif char == '}':
            brace_count -= 1
            if brace_count == 0:
                query_end = i + 1
                break
    
    query_block = content[pos:query_end]
    
    # Extract state transition definitions
    trans_match = re.search(r'\s*const stateTransitionDefinitions = \{', content)
    if not trans_match:
        return {}, {}
    
    trans_start = trans_match.start()
    
    brace_count = 0
    pos = trans_match.end() - 1  # Start at the opening brace
    trans_end = pos
    
    for i, char in enumerate(content[pos:], pos):
        if char == '{':
            brace_count += 1
        elif char == '}':
            brace_count -= 1
            if brace_count == 0:
                trans_end = i + 1
                break
    
    trans_block = content[pos:trans_end]
    
    # Parse definitions
    query_defs = parse_definition_block(query_block, 'queries')
    trans_defs = parse_definition_block(trans_block, 'transitions')
    
    return query_defs, trans_defs

def parse_definition_block(block, item_type):
    """Parse a definition block into structured data"""
    definitions = {}
    
    # First, find all category blocks with the pattern: categoryName: { label: "...", queries/transitions: { ... } }
    # We need to match the full category structure, not just anything with a label
    cat_pattern = r'(\w+):\s*\{\s*label:\s*"([^"]+)"[^}]*' + item_type + r':\s*\{'
    
    for match in re.finditer(cat_pattern, block):
        cat_key = match.group(1)
        cat_label = match.group(2)
        
        # Skip if this looks like it's inside another structure
        # Check if the match is at the top level by ensuring no unclosed braces before it
        # We expect exactly 1 unclosed brace (the opening brace of the main object)
        prefix = block[:match.start()]
        if prefix.count('{') - prefix.count('}') != 1:
            continue
        
        # Find this category's content
        cat_start = match.start()
        
        # Find the full category block by counting braces
        brace_count = 0
        cat_end = cat_start
        in_category = False
        
        for i, char in enumerate(block[cat_start:], cat_start):
            if char == '{':
                brace_count += 1
                in_category = True
            elif char == '}':
                brace_count -= 1
                if in_category and brace_count == 0:
                    cat_end = i + 1
                    break
        
        cat_block = block[cat_start:cat_end]
        
        # Now extract just the queries/transitions section
        items_pattern = rf'{item_type}:\s*\{{'
        items_match = re.search(items_pattern, cat_block)
        
        if items_match:
            items_start = items_match.end()
            
            # Find the end of the items section by counting braces
            brace_count = 1
            items_end = items_start
            
            for i, char in enumerate(cat_block[items_start:], items_start):
                if char == '{':
                    brace_count += 1
                elif char == '}':
                    brace_count -= 1
                    if brace_count == 0:
                        items_end = i
                        break
            
            items_block = cat_block[items_start:items_end]
            
            # Extract individual items
            definitions[cat_key] = {
                'label': cat_label,
                item_type: parse_items(items_block)
            }
    
    return definitions

def parse_items(items_block):
    """Parse individual query/transition items"""
    items = {}
    
    # Find each item that has label and description structure
    # This pattern ensures we only match actual query/transition definitions
    item_pattern = r'(\w+):\s*\{\s*label:\s*"'
    
    matches = list(re.finditer(item_pattern, items_block))
    
    for match in matches:
        item_key = match.group(1)
        item_start = match.start()
        
        # Find the full item block by counting braces
        brace_count = 0
        item_end = item_start
        in_item = False
        
        for j, char in enumerate(items_block[item_start:], item_start):
            if char == '{':
                brace_count += 1
                in_item = True
            elif char == '}':
                brace_count -= 1
                if in_item and brace_count == 0:
                    item_end = j + 1
                    break
        
        item_content = items_block[item_start + match.end() - match.start():item_end - 1]
        
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
        
        # Extract inputs
        inputs_match = re.search(r'inputs:\s*\[', item_content)
        if inputs_match:
            inputs_start = inputs_match.end()
            
            # Find the end of inputs array
            bracket_count = 1
            inputs_end = inputs_start
            
            for j, char in enumerate(item_content[inputs_start:], inputs_start):
                if char == '[':
                    bracket_count += 1
                elif char == ']':
                    bracket_count -= 1
                    if bracket_count == 0:
                        inputs_end = j
                        break
            
            inputs_content = item_content[inputs_start:inputs_end]
            item_data['inputs'] = parse_inputs(inputs_content)
        else:
            item_data['inputs'] = []
        
        items[item_key] = item_data
    
    return items

def parse_inputs(inputs_content):
    """Parse input array"""
    inputs = []
    
    # Find each input object
    obj_starts = [m.start() for m in re.finditer(r'\{', inputs_content)]
    
    for start in obj_starts:
        # Find the matching closing brace
        brace_count = 1
        end = start + 1
        
        for i, char in enumerate(inputs_content[start + 1:], start + 1):
            if char == '{':
                brace_count += 1
            elif char == '}':
                brace_count -= 1
                if brace_count == 0:
                    end = i + 1
                    break
        
        obj_content = inputs_content[start:end]
        
        # Parse this input object
        input_data = {}
        
        # Extract fields
        fields = {
            'name': r'name:\s*"([^"]+)"',
            'type': r'type:\s*"([^"]+)"',
            'label': r'label:\s*"([^"]+)"',
            'placeholder': r'placeholder:\s*["\']([^"\']+)["\']',
        }
        
        for field, pattern in fields.items():
            match = re.search(pattern, obj_content)
            if match:
                input_data[field] = match.group(1)
        
        # Extract required
        req_match = re.search(r'required:\s*(true|false)', obj_content)
        if req_match:
            input_data['required'] = req_match.group(1) == 'true'
        
        # Extract options if present
        options_match = re.search(r'options:\s*\[', obj_content)
        if options_match:
            options_start = options_match.end()
            bracket_count = 1
            options_end = options_start
            
            for i, char in enumerate(obj_content[options_start:], options_start):
                if char == '[':
                    bracket_count += 1
                elif char == ']':
                    bracket_count -= 1
                    if bracket_count == 0:
                        options_end = i
                        break
            
            options_content = obj_content[options_start:options_end]
            input_data['options'] = []
            
            # Extract each option
            opt_pattern = r'\{\s*value:\s*"([^"]+)"[^}]*label:\s*"([^"]+)"[^}]*\}'
            for opt_match in re.finditer(opt_pattern, options_content):
                input_data['options'].append({
                    'value': opt_match.group(1),
                    'label': opt_match.group(2)
                })
        
        if input_data:
            inputs.append(input_data)
    
    return inputs

def main():
    index_file = Path(__file__).parent / 'index.html'
    
    if not index_file.exists():
        print(f"Error: {index_file} not found")
        return 1
    
    query_defs, trans_defs = manual_extract_definitions(index_file)
    
    # Save to JSON for inspection
    output = {
        'queries': query_defs,
        'transitions': trans_defs
    }
    
    with open('extracted_definitions.json', 'w') as f:
        json.dump(output, f, indent=2)
    
    print(f"Extracted {sum(len(cat.get('queries', {})) for cat in query_defs.values())} queries")
    print(f"Extracted {sum(len(cat.get('transitions', {})) for cat in trans_defs.values())} transitions")
    
    return 0

if __name__ == '__main__':
    exit(main())