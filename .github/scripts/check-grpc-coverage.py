#!/usr/bin/env python3
"""
Check that all gRPC queries from platform.proto are either implemented in rs-sdk
or explicitly annotated with @sdk-ignore in the proto file.

Cache stores only 'implemented' queries (auto-managed).
Unimplemented queries MUST have '// @sdk-ignore: <reason>' in the proto file.
"""

import os
import re
import sys
import json
from pathlib import Path
from datetime import datetime


# Optional overrides for auto-derived search patterns.
# Format: {"rpcMethodName": ["pattern1", "pattern2", ...]}
# By default, the script derives the pattern by capitalizing the first letter
# and appending "Request" (e.g., getIdentity -> GetIdentityRequest).
QUERY_MAPPINGS = {}


def extract_grpc_queries(proto_file):
    """Extract all RPC method names and their line numbers from platform.proto."""
    queries = []

    with open(proto_file, "r") as f:
        content = f.read()

    service_match = re.search(r"service\s+Platform\s*\{", content)
    if not service_match:
        print("ERROR: Could not find 'service Platform' in proto file")
        sys.exit(1)

    start_pos = service_match.end()
    brace_count = 1
    pos = start_pos

    while pos < len(content) and brace_count > 0:
        if content[pos] == "{":
            brace_count += 1
        elif content[pos] == "}":
            brace_count -= 1
        pos += 1

    if brace_count != 0:
        print("ERROR: Unmatched braces in service Platform block")
        sys.exit(1)

    service_content = content[start_pos : pos - 1]
    service_start_line = content[:start_pos].count("\n") + 1

    rpc_pattern = r"rpc\s+(\w+)\s*\("
    for match in re.finditer(rpc_pattern, service_content):
        query_name = match.group(1)
        line_in_service = service_content[: match.start()].count("\n")
        line_number = service_start_line + line_in_service
        queries.append((query_name, line_number))

    return queries


def extract_sdk_ignore_annotations(proto_file):
    """Parse @sdk-ignore annotations from proto file.

    Returns dict mapping query name to ignore reason.
    """
    ignored = {}

    with open(proto_file, "r") as f:
        lines = f.readlines()

    for i, line in enumerate(lines):
        stripped = line.strip()
        ignore_match = re.match(r"//\s*@sdk-ignore:\s*(.+)", stripped)
        if not ignore_match:
            continue

        reason = ignore_match.group(1).strip()
        if not reason:
            continue

        for j in range(i + 1, len(lines)):
            rpc_match = re.match(r"\s*rpc\s+(\w+)\s*\(", lines[j])
            if rpc_match:
                ignored[rpc_match.group(1)] = reason
                break
            if lines[j].strip() and not lines[j].strip().startswith("//"):
                break

    return ignored


def index_sdk_sources(sdk_path):
    """Read all .rs files once and return {path: content} dict."""
    sources = {}
    for root, dirnames, filenames in os.walk(sdk_path):
        dirnames[:] = [d for d in dirnames if d not in {"tests", "test"}]
        for file in filenames:
            if file.endswith(".rs"):
                file_path = os.path.join(root, file)
                try:
                    with open(file_path, "r") as f:
                        sources[file_path] = f.read()
                except Exception as e:
                    print(f"Warning: Could not read {file_path}: {e}")
    return sources


def check_query_implementation(query_name, sdk_sources):
    """Check if a query is implemented in the indexed SDK sources."""
    if query_name in QUERY_MAPPINGS:
        patterns = QUERY_MAPPINGS[query_name]
    else:
        patterns = [query_name[0].upper() + query_name[1:] + "Request"]

    for file_path, content in sdk_sources.items():
        for pattern in patterns:
            if pattern in content:
                return True, file_path

    return False, None


def load_cache(cache_file):
    """Load the query cache from file."""
    if cache_file.exists():
        with open(cache_file, "r") as f:
            return json.load(f)
    return {"known_queries": {}, "last_updated": None}


def save_cache(cache_file, cache_data):
    """Save the query cache to file."""
    cache_data["last_updated"] = datetime.now().isoformat()
    with open(cache_file, "w") as f:
        json.dump(cache_data, f, indent=2)
        f.write("\n")


def check_wasm_sdk_documentation():
    """Check WASM SDK documentation completeness."""
    wasm_sdk_path = Path(__file__).parent.parent.parent / "packages" / "wasm-sdk"
    check_script = wasm_sdk_path / "check_documentation.py"

    if not check_script.exists():
        return True, []

    import subprocess

    result = subprocess.run(
        [sys.executable, str(check_script)],
        cwd=wasm_sdk_path,
        capture_output=True,
        text=True,
    )

    errors = []
    if result.returncode != 0:
        for line in result.stdout.strip().split("\n"):
            if line.startswith("ERROR:"):
                errors.append(line)
        for line in result.stderr.strip().split("\n"):
            if line.strip():
                errors.append(line)

    return result.returncode == 0, errors


def main():
    """Main function to check gRPC coverage."""
    script_dir = Path(__file__).parent
    project_root = script_dir.parent.parent
    proto_file = project_root / "packages" / "dapi-grpc" / "protos" / "platform" / "v0" / "platform.proto"
    sdk_path = project_root / "packages" / "rs-sdk" / "src"
    cache_file = project_root / ".github" / "grpc-queries-cache.json"

    if not proto_file.exists():
        print(f"ERROR: Proto file not found: {proto_file}")
        sys.exit(1)

    if not sdk_path.exists():
        print(f"ERROR: SDK path not found: {sdk_path}")
        sys.exit(1)

    cache_data = load_cache(cache_file)
    known_queries = cache_data.get("known_queries", {})

    # Remove any non-implemented entries from cache (migration from old format)
    known_queries = {
        q: info
        for q, info in known_queries.items()
        if info.get("status") == "implemented"
    }

    print("Extracting gRPC queries from platform.proto...")
    all_queries = extract_grpc_queries(proto_file)
    ignored_queries = extract_sdk_ignore_annotations(proto_file)

    query_names = [name for name, _ in all_queries]

    print(f"Found {len(all_queries)} total queries")
    print(f"Queries with @sdk-ignore: {len(ignored_queries)}")
    print(f"Cached implemented queries: {len(known_queries)}")

    # Index SDK sources once for all lookups
    sdk_sources = index_sdk_sources(sdk_path)

    # Re-check previously implemented queries (detect removed implementations)
    removed_queries = []
    for query in list(known_queries.keys()):
        if query not in query_names:
            del known_queries[query]
            removed_queries.append(query)
            continue
        if query in ignored_queries:
            del known_queries[query]
            continue
        implemented, _ = check_query_implementation(query, sdk_sources)
        if not implemented:
            del known_queries[query]
            removed_queries.append(query)

    if removed_queries:
        print(f"Removed {len(removed_queries)} queries from cache (no longer implemented)")

    # Check all queries
    implemented_queries = []
    missing_queries = []

    for query_name, line_number in all_queries:
        if query_name in ignored_queries:
            continue

        if query_name in known_queries:
            implemented_queries.append(query_name)
            continue

        print(f"Checking {query_name}...", end=" ")
        implemented, _ = check_query_implementation(query_name, sdk_sources)

        if implemented:
            print("IMPLEMENTED")
            implemented_queries.append(query_name)
            known_queries[query_name] = {"status": "implemented"}
        else:
            print("MISSING")
            missing_queries.append((query_name, line_number))

    # Build Markdown report (used as GitHub PR comment)
    report_lines = []
    report_lines.append(f"**Total:** {len(all_queries)} queries — "
                        f"{len(implemented_queries)} implemented, "
                        f"{len(ignored_queries)} ignored, "
                        f"{len(missing_queries)} missing")

    if missing_queries:
        report_lines.append("\n#### ❌ Missing")
        for q, line in sorted(missing_queries):
            report_lines.append(f"- `{q}` (line {line})")

    if ignored_queries:
        report_lines.append("\n<details>\n<summary>⏭️ Ignored (@sdk-ignore) "
                            f"({len(ignored_queries)})</summary>\n")
        for q in sorted(ignored_queries):
            report_lines.append(f"- `{q}` — {ignored_queries[q]}")
        report_lines.append("\n</details>")

    if implemented_queries:
        report_lines.append("\n<details>\n<summary>✅ Implemented "
                            f"({len(implemented_queries)})</summary>\n")
        for q in sorted(implemented_queries):
            report_lines.append(f"- `{q}`")
        report_lines.append("\n</details>")

    # Save cache (only implemented queries)
    save_cache(cache_file, {"known_queries": known_queries})

    # Write report
    report_content = "\n".join(report_lines)
    with open("grpc-coverage-report.txt", "w") as f:
        f.write(report_content)

    print("\n" + report_content)

    # Check WASM SDK documentation
    print("\n" + "=" * 80)
    print("Checking WASM SDK Documentation...")
    print("=" * 80)

    wasm_docs_ok, wasm_errors = check_wasm_sdk_documentation()

    if wasm_docs_ok:
        print("WASM SDK documentation is up to date")
    else:
        print(f"WASM SDK documentation has {len(wasm_errors)} errors:")
        for error in wasm_errors:
            print(f"  {error}")
        print("\nTo fix WASM SDK documentation errors, run:")
        print("  cd packages/wasm-sdk && python3 generate_docs.py")

    # Fail if missing queries or documentation errors
    has_failures = False

    if missing_queries:
        has_failures = True
        proto_rel = "packages/dapi-grpc/protos/platform/v0/platform.proto"
        print(f"\nERROR: {len(missing_queries)} gRPC queries are not implemented "
              f"in rs-sdk and not ignored.\n")
        print("To ignore a query, add this comment above the rpc definition in")
        print(f"{proto_rel}:\n")
        print("  // @sdk-ignore: <your reason here>")
        print("  rpc <QueryName>(...) returns (...);\n")
        print("Missing queries:")
        for q, line in sorted(missing_queries):
            print(f"  - {q} (line {line})")

    if not wasm_docs_ok:
        has_failures = True

    if has_failures:
        sys.exit(1)
    else:
        print("\nSUCCESS: All gRPC queries are implemented in rs-sdk or annotated "
              "with @sdk-ignore, and documentation is up to date")
        sys.exit(0)


if __name__ == "__main__":
    main()
