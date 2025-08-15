#!/usr/bin/env python3
"""
Check that all queries and state transitions in api-definitions.json are documented
"""

import os
import sys
import json
from pathlib import Path
from datetime import datetime, timezone

def check_documentation_completeness():
    """Check if documentation is up to date with api-definitions.json"""
    
    script_dir = Path(__file__).parent
    
    # Required files
    api_definitions_file = script_dir / 'api-definitions.json'
    manifest_file = script_dir / 'docs_manifest.json'
    docs_file = script_dir / 'docs.html'
    ai_ref_file = script_dir / 'AI_REFERENCE.md'
    
    errors = []
    warnings = []
    
    # Check if all required files exist
    if not api_definitions_file.exists():
        errors.append(f"ERROR: api-definitions.json not found at {api_definitions_file}")
        return errors, warnings
    
    if not manifest_file.exists():
        errors.append(f"ERROR: Documentation manifest not found at {manifest_file}. Run generate_docs.py first.")
        return errors, warnings
    
    if not docs_file.exists():
        errors.append(f"ERROR: User documentation not found at {docs_file}. Run generate_docs.py first.")
    
    if not ai_ref_file.exists():
        errors.append(f"ERROR: AI reference not found at {ai_ref_file}. Run generate_docs.py first.")
    
    # Load current definitions from api-definitions.json
    print("Loading definitions from api-definitions.json...")
    try:
        with open(api_definitions_file, 'r') as f:
            api_data = json.load(f)
        current_defs = {
            'queries': api_data.get('queries', {}),
            'transitions': api_data.get('transitions', {})
        }
    except (FileNotFoundError, json.JSONDecodeError) as e:
        errors.append(f"ERROR: Failed to load api-definitions.json: {e}")
        return errors, warnings
    
    # Load documentation manifest
    with open(manifest_file, 'r') as f:
        manifest = json.load(f)
    
    # Check if manifest is stale (older than 24 hours)
    if 'generated_at' in manifest:
        generated_time = datetime.fromisoformat(manifest['generated_at'])
        # Normalize to UTC timezone
        if generated_time.tzinfo is None:
            generated_time = generated_time.replace(tzinfo=timezone.utc)
        else:
            generated_time = generated_time.astimezone(timezone.utc)
        age_hours = (datetime.now(timezone.utc) - generated_time).total_seconds() / 3600
        if age_hours > 24:
            warnings.append(f"WARNING: Documentation was generated {age_hours:.1f} hours ago. Consider regenerating.")
    
    # Extract all current queries and transitions
    current_queries = set()
    current_transitions = set()
    
    for cat_key, category in current_defs.get('queries', {}).items():
        for query_key in category.get('queries', {}).keys():
            current_queries.add(query_key)
    
    for cat_key, category in current_defs.get('transitions', {}).items():
        for trans_key in category.get('transitions', {}).keys():
            current_transitions.add(trans_key)
    
    documented_queries = set(manifest.get('queries', {}).keys())
    documented_transitions = set(manifest.get('transitions', {}).keys())
    
    # Find undocumented items
    undocumented_queries = current_queries - documented_queries
    undocumented_transitions = current_transitions - documented_transitions
    
    # Find removed items (documented but no longer in code)
    removed_queries = documented_queries - current_queries
    removed_transitions = documented_transitions - current_transitions
    
    # Report findings
    if undocumented_queries:
        errors.append(f"ERROR: {len(undocumented_queries)} queries are not documented:")
        for q in sorted(undocumented_queries):
            errors.append(f"  - {q}")
    
    if undocumented_transitions:
        errors.append(f"ERROR: {len(undocumented_transitions)} state transitions are not documented:")
        for t in sorted(undocumented_transitions):
            errors.append(f"  - {t}")
    
    if removed_queries:
        warnings.append(f"WARNING: {len(removed_queries)} queries are documented but no longer exist:")
        for q in sorted(removed_queries):
            warnings.append(f"  - {q}")
    
    if removed_transitions:
        warnings.append(f"WARNING: {len(removed_transitions)} transitions are documented but no longer exist:")
        for t in sorted(removed_transitions):
            warnings.append(f"  - {t}")
    
    # Check file timestamps
    api_definitions_mtime = os.path.getmtime(api_definitions_file)
    
    if docs_file.exists():
        docs_mtime = os.path.getmtime(docs_file)
        if api_definitions_mtime > docs_mtime:
            warnings.append("WARNING: api-definitions.json has been modified after docs.html was generated")
    
    if ai_ref_file.exists():
        ai_mtime = os.path.getmtime(ai_ref_file)
        if api_definitions_mtime > ai_mtime:
            warnings.append("WARNING: api-definitions.json has been modified after AI_REFERENCE.md was generated")
    
    return errors, warnings

def main():
    """Main function"""
    
    errors, warnings = check_documentation_completeness()
    
    # Write report
    report_lines = []
    report_lines.append("=" * 80)
    report_lines.append("Documentation Completeness Check")
    report_lines.append("=" * 80)
    report_lines.append(f"Timestamp: {datetime.now().isoformat()}\n")
    
    if not errors and not warnings:
        report_lines.append("✅ All documentation is up to date!")
    else:
        if warnings:
            report_lines.append(f"⚠️  {len(warnings)} warnings found:\n")
            for warning in warnings:
                report_lines.append(warning)
            report_lines.append("")
        
        if errors:
            report_lines.append(f"❌ {len(errors)} errors found:\n")
            for error in errors:
                report_lines.append(error)
            report_lines.append("")
    
    report_lines.append("=" * 80)
    
    if errors:
        report_lines.append("\nTo fix these errors, run: python3 generate_docs.py")
    
    # Print report
    report = '\n'.join(report_lines)
    print(report)
    
    # Save report
    with open('documentation-check-report.txt', 'w') as f:
        f.write(report)
    
    # Exit with error if there are any errors
    return 1 if errors else 0

if __name__ == '__main__':
    sys.exit(main())