#!/usr/bin/env python3
"""Validate and summarize the Kotlin/Swift executable parity manifest.

The manifest is JSON (and therefore valid YAML) so this checker depends only on
the Python standard library.  Run with ``--write-summary`` after an intentional
manifest change; CI runs in check mode and rejects a stale generated summary.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from collections import Counter
from pathlib import Path
from typing import Any


DEFAULT_MANIFEST = Path("docs/sdk/sdk-parity-manifest.json")
DEFAULT_SUMMARY = Path("packages/kotlin-sdk/PARITY_SUMMARY.md")
HOSTS = ("swift", "kotlin")
SURFACES = ("sdk", "example_app")
STATUSES = ("supported", "partial", "unsupported", "not-applicable")
RESTART_STATES = ("tested", "required", "not_applicable")
VERIFICATION_KINDS = ("unit", "integration", "device", "manual")
AUTOMATED_KINDS = ("unit", "integration", "device")
CAPABILITY_ID_RE = re.compile(r"^[a-z][a-z0-9_]*(?:\.[a-z][a-z0-9_]*)+$")
SYMBOL_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_:]*$")
BASELINE_RE = re.compile(r"^PR #[1-9][0-9]* @ [0-9a-f]{40}$")


class ManifestError(Exception):
    """Raised after collecting one or more actionable validation failures."""


def _validate_command_target(
    raw_path: Any,
    command: Any,
    kind: Any,
    field: str,
    errors: list[str],
) -> None:
    """Reject commands that cannot build the test target containing ``raw_path``.

    This is intentionally structural rather than a shell executor: host-specific
    CI jobs execute tests, while the manifest checker prevents a Swift Package
    command from claiming an Xcode-app test (and analogous Gradle target drift).
    """
    if kind not in AUTOMATED_KINDS or not isinstance(raw_path, str) \
            or not isinstance(command, str):
        return

    normalized = raw_path.replace("\\", "/")
    if "/SwiftExampleAppTests/" in normalized:
        required = (
            "xcodebuild",
            "packages/swift-sdk/SwiftExampleApp/SwiftExampleApp.xcodeproj",
            "-scheme SwiftExampleApp",
        )
        if any(token not in command for token in required):
            errors.append(
                f"{field}: SwiftExampleAppTests require xcodebuild with the "
                "SwiftExampleApp project and scheme"
            )
    elif "/SwiftTests/SwiftDashSDKTests/" in normalized or \
            "/SwiftTests/SwiftDashSDKIntegrationTests/" in normalized:
        swift_package_command = "swift test --package-path packages/swift-sdk" in command
        xcode_package_command = all(token in command for token in (
            "cd packages/swift-sdk",
            "xcodebuild test",
            "-scheme SwiftDashSDK",
        ))
        if not swift_package_command and not xcode_package_command:
            errors.append(
                f"{field}: Swift package tests require "
                "'swift test --package-path packages/swift-sdk' or xcodebuild "
                "with the SwiftDashSDK package scheme"
            )
    elif "/KotlinExampleApp/app/src/test/" in normalized:
        if "./gradlew" not in command or ":app:test" not in command:
            errors.append(f"{field}: Kotlin example-app tests require the :app:test Gradle target")
    elif "/kotlin-sdk/sdk/src/androidTest/" in normalized:
        if "./gradlew" not in command or ":sdk:connected" not in command:
            errors.append(
                f"{field}: Android instrumented SDK tests require a :sdk:connected Gradle target"
            )
    elif "/kotlin-sdk/sdk/src/test/" in normalized or \
            "/kotlin-sdk/sdk/src/testDebug/" in normalized:
        if "./gradlew" not in command or ":sdk:test" not in command:
            errors.append(f"{field}: Kotlin SDK unit tests require the :sdk:test Gradle target")


def _is_plain_int(value: Any) -> bool:
    return isinstance(value, int) and not isinstance(value, bool)


def _repo_file(repo_root: Path, raw_path: Any, field: str, errors: list[str]) -> Path | None:
    if not isinstance(raw_path, str) or not raw_path:
        errors.append(f"{field}: expected a non-empty repository-relative path")
        return None
    relative = Path(raw_path)
    if relative.is_absolute() or ".." in relative.parts:
        errors.append(f"{field}: path must stay inside the repository: {raw_path!r}")
        return None
    resolved = (repo_root / relative).resolve()
    try:
        resolved.relative_to(repo_root.resolve())
    except ValueError:
        errors.append(f"{field}: path escapes repository: {raw_path!r}")
        return None
    if not resolved.is_file():
        errors.append(f"{field}: referenced file does not exist: {raw_path}")
        return None
    return resolved


def load_manifest(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError as exc:
        raise ManifestError(f"manifest does not exist: {path}") from exc
    except json.JSONDecodeError as exc:
        raise ManifestError(f"{path}:{exc.lineno}:{exc.colno}: invalid JSON: {exc.msg}") from exc
    if not isinstance(value, dict):
        raise ManifestError("manifest root must be a JSON object")
    return value


def validate_manifest(manifest: dict[str, Any], repo_root: Path) -> None:
    errors: list[str] = []
    allowed_root = {
        "schema_version",
        "baseline",
        "description",
        "declared_persistence_capabilities",
        "shared_symbols",
        "capabilities",
    }
    unknown_root = sorted(set(manifest) - allowed_root)
    if unknown_root:
        errors.append(f"root: unknown fields: {', '.join(unknown_root)}")

    if (
        not _is_plain_int(manifest.get("schema_version"))
        or manifest.get("schema_version") != 1
    ):
        errors.append("schema_version: expected integer 1")
    if not isinstance(manifest.get("baseline"), str) or not BASELINE_RE.fullmatch(
        manifest.get("baseline", "")
    ):
        errors.append("baseline: expected 'PR #<number> @ <40-character lowercase git SHA>'")
    if not isinstance(manifest.get("description"), str) or not manifest.get("description"):
        errors.append("description: expected a non-empty string")

    declared = manifest.get("declared_persistence_capabilities")
    if not isinstance(declared, list) or not all(isinstance(item, str) and item for item in declared):
        errors.append("declared_persistence_capabilities: expected a list of non-empty strings")
        declared_set: set[str] = set()
    else:
        declared_set = set(declared)
        if len(declared_set) != len(declared):
            errors.append("declared_persistence_capabilities: duplicate entries")
        if declared != sorted(declared):
            errors.append("declared_persistence_capabilities: entries must be sorted")

    symbols = manifest.get("shared_symbols")
    if not isinstance(symbols, dict):
        errors.append("shared_symbols: expected an object mapping symbol to source file")
        symbols = {}
    for symbol, raw_path in symbols.items():
        field = f"shared_symbols.{symbol}"
        if not isinstance(symbol, str) or not SYMBOL_RE.fullmatch(symbol):
            errors.append(f"{field}: invalid symbol name")
            continue
        path = _repo_file(repo_root, raw_path, field, errors)
        if path is not None:
            try:
                contents = path.read_text(encoding="utf-8", errors="replace")
            except OSError as exc:
                errors.append(f"{field}: cannot read {raw_path}: {exc}")
            else:
                if symbol not in contents:
                    errors.append(f"{field}: symbol is not present in {raw_path}")

    capabilities = manifest.get("capabilities")
    if not isinstance(capabilities, list) or not capabilities:
        errors.append("capabilities: expected a non-empty list")
        capabilities = []

    seen_ids: set[str] = set()
    used_symbols: set[str] = set()
    for index, capability in enumerate(capabilities):
        prefix = f"capabilities[{index}]"
        if not isinstance(capability, dict):
            errors.append(f"{prefix}: expected an object")
            continue
        allowed_capability = {
            "id",
            "title",
            "area",
            "shared_apis",
            "required_persistence_capabilities",
            "hosts",
            "verification",
        }
        unknown = sorted(set(capability) - allowed_capability)
        if unknown:
            errors.append(f"{prefix}: unknown fields: {', '.join(unknown)}")

        capability_id = capability.get("id")
        if not isinstance(capability_id, str) or not CAPABILITY_ID_RE.fullmatch(capability_id):
            errors.append(f"{prefix}.id: expected dotted lower_snake_case identifier")
            capability_id = prefix
        elif capability_id in seen_ids:
            errors.append(f"{prefix}.id: duplicate capability id {capability_id!r}")
        else:
            seen_ids.add(capability_id)
        for field in ("title", "area"):
            if not isinstance(capability.get(field), str) or not capability.get(field):
                errors.append(f"{prefix}.{field}: expected a non-empty string")

        shared_apis = capability.get("shared_apis", [])
        if not isinstance(shared_apis, list) or not all(isinstance(item, str) for item in shared_apis):
            errors.append(f"{prefix}.shared_apis: expected a list of symbol strings")
            shared_apis = []
        if len(shared_apis) != len(set(shared_apis)):
            errors.append(f"{prefix}.shared_apis: duplicate symbols")
        for symbol in shared_apis:
            used_symbols.add(symbol)
            if symbol not in symbols:
                errors.append(f"{prefix}.shared_apis: undeclared shared symbol {symbol!r}")

        requirements = capability.get("required_persistence_capabilities", [])
        if not isinstance(requirements, list) or not all(isinstance(item, str) for item in requirements):
            errors.append(
                f"{prefix}.required_persistence_capabilities: expected a list of strings"
            )
            requirements = []
        if len(requirements) != len(set(requirements)):
            errors.append(f"{prefix}.required_persistence_capabilities: duplicate entries")
        for requirement in requirements:
            if requirement not in declared_set:
                errors.append(
                    f"{prefix}.required_persistence_capabilities: undeclared capability "
                    f"{requirement!r}"
                )

        hosts = capability.get("hosts")
        if not isinstance(hosts, dict):
            errors.append(f"{prefix}.hosts: expected an object")
            hosts = {}
        if set(hosts) != set(HOSTS):
            errors.append(f"{prefix}.hosts: expected exactly {', '.join(HOSTS)}")

        verification = capability.get("verification", [])
        if not isinstance(verification, list):
            errors.append(f"{prefix}.verification: expected a list")
            verification = []
        verification_hosts: set[str] = set()
        restart_hosts: set[str] = set()
        for v_index, entry in enumerate(verification):
            v_prefix = f"{prefix}.verification[{v_index}]"
            if not isinstance(entry, dict):
                errors.append(f"{v_prefix}: expected an object")
                continue
            allowed_verification = {"host", "kind", "file", "id", "command", "covers_restart"}
            unknown_v = sorted(set(entry) - allowed_verification)
            if unknown_v:
                errors.append(f"{v_prefix}: unknown fields: {', '.join(unknown_v)}")
            host = entry.get("host")
            if host not in (*HOSTS, "shared"):
                errors.append(f"{v_prefix}.host: expected swift, kotlin, or shared")
            else:
                verification_hosts.add(host)
            kind = entry.get("kind")
            if kind not in VERIFICATION_KINDS:
                errors.append(f"{v_prefix}.kind: invalid verification kind {kind!r}")
            command = entry.get("command")
            if kind in AUTOMATED_KINDS and (not isinstance(command, str) or not command.strip()):
                errors.append(f"{v_prefix}.command: automated verification requires a command")
            if command is not None and not isinstance(command, str):
                errors.append(f"{v_prefix}.command: expected a string")
            _validate_command_target(
                entry.get("file"), command, kind, f"{v_prefix}.command", errors
            )
            test_id = entry.get("id")
            if not isinstance(test_id, str) or not test_id:
                errors.append(f"{v_prefix}.id: expected a non-empty stable test id")
            path = _repo_file(repo_root, entry.get("file"), f"{v_prefix}.file", errors)
            if path is not None and isinstance(test_id, str) and test_id:
                text = path.read_text(encoding="utf-8", errors="replace")
                if test_id not in text:
                    errors.append(f"{v_prefix}.id: {test_id!r} is not present in {entry.get('file')}")
            covers_restart = entry.get("covers_restart", False)
            if not isinstance(covers_restart, bool):
                errors.append(f"{v_prefix}.covers_restart: expected boolean")
            elif covers_restart and host in HOSTS:
                restart_hosts.add(host)

        for host in HOSTS:
            h_prefix = f"{prefix}.hosts.{host}"
            value = hosts.get(host)
            if not isinstance(value, dict):
                errors.append(f"{h_prefix}: expected an object")
                continue
            if set(value) != {"sdk", "example_app", "restart", "reason"}:
                errors.append(
                    f"{h_prefix}: expected exactly sdk, example_app, restart, reason"
                )
            sdk_status = value.get("sdk")
            app_status = value.get("example_app")
            restart = value.get("restart")
            reason = value.get("reason")
            if sdk_status not in STATUSES:
                errors.append(f"{h_prefix}.sdk: invalid status {sdk_status!r}")
            if app_status not in STATUSES:
                errors.append(f"{h_prefix}.example_app: invalid status {app_status!r}")
            if restart not in RESTART_STATES:
                errors.append(f"{h_prefix}.restart: invalid restart state {restart!r}")
            has_gap = sdk_status in ("partial", "unsupported") or app_status in (
                "partial",
                "unsupported",
            )
            if has_gap and (not isinstance(reason, str) or not reason.strip()):
                errors.append(f"{h_prefix}.reason: partial/unsupported status requires a reason")
            if not has_gap and reason is not None:
                errors.append(f"{h_prefix}.reason: fully supported/not-applicable host uses null")
            if app_status == "supported" and sdk_status != "supported":
                errors.append(f"{h_prefix}: example app cannot be supported when SDK is not")
            if (sdk_status in ("supported", "partial") or app_status in ("supported", "partial")) \
                    and host not in verification_hosts:
                errors.append(f"{h_prefix}: supported/partial implementation requires verification")
            if restart == "tested" and host not in restart_hosts:
                errors.append(f"{h_prefix}.restart: tested requires covers_restart verification")
            if restart == "required" and sdk_status == "supported" and app_status in (
                "supported",
                "not-applicable",
            ):
                errors.append(
                    f"{h_prefix}.restart: capability cannot be fully supported while restart is required"
                )

    unused_symbols = sorted(set(symbols) - used_symbols)
    if unused_symbols:
        errors.append(f"shared_symbols: declarations are unused: {', '.join(unused_symbols)}")

    if errors:
        raise ManifestError("\n".join(f"- {error}" for error in errors))


def compute_counts(manifest: dict[str, Any]) -> dict[str, dict[str, Counter[str]]]:
    counts: dict[str, dict[str, Counter[str]]] = {}
    for host in HOSTS:
        counts[host] = {surface: Counter() for surface in SURFACES}
        counts[host]["restart"] = Counter()
    for capability in manifest["capabilities"]:
        for host in HOSTS:
            host_value = capability["hosts"][host]
            for surface in SURFACES:
                counts[host][surface][host_value[surface]] += 1
            counts[host]["restart"][host_value["restart"]] += 1
    return counts


def _cell(value: Any) -> str:
    return str(value).replace("|", "\\|").replace("\n", " ")


def render_summary(manifest: dict[str, Any]) -> str:
    counts = compute_counts(manifest)
    lines = [
        "<!-- Generated by scripts/check_sdk_parity_manifest.py; do not edit by hand. -->",
        "# Kotlin/Swift executable parity summary",
        "",
        f"Audit baseline: `{_cell(manifest['baseline'])}`",
        f"Capabilities tracked: **{len(manifest['capabilities'])}**",
        "",
        "## Status counts",
        "",
        "| Host | Surface | Supported | Partial | Unsupported | Not applicable |",
        "| --- | --- | ---: | ---: | ---: | ---: |",
    ]
    labels = {"sdk": "SDK", "example_app": "Example app"}
    for host in HOSTS:
        for surface in SURFACES:
            counter = counts[host][surface]
            lines.append(
                f"| {host.title()} | {labels[surface]} | {counter['supported']} | "
                f"{counter['partial']} | {counter['unsupported']} | "
                f"{counter['not-applicable']} |"
            )
    lines.extend(
        [
            "",
            "## Restart coverage",
            "",
            "| Host | Tested | Required | Not applicable |",
            "| --- | ---: | ---: | ---: |",
        ]
    )
    for host in HOSTS:
        counter = counts[host]["restart"]
        lines.append(
            f"| {host.title()} | {counter['tested']} | {counter['required']} | "
            f"{counter['not_applicable']} |"
        )
    lines.extend(
        [
            "",
            "## Capability status",
            "",
            "| Capability | Swift SDK / app / restart | Kotlin SDK / app / restart |",
            "| --- | --- | --- |",
        ]
    )
    for capability in manifest["capabilities"]:
        cells = []
        for host in HOSTS:
            value = capability["hosts"][host]
            cells.append(f"{value['sdk']} / {value['example_app']} / {value['restart']}")
        lines.append(
            f"| `{_cell(capability['id'])}` | {_cell(cells[0])} | {_cell(cells[1])} |"
        )
    lines.extend(
        [
            "",
            "Source: [`docs/sdk/sdk-parity-manifest.json`](../../docs/sdk/sdk-parity-manifest.json).",
            "Detailed legacy view mapping remains in [`PARITY.md`](PARITY.md), but its manual totals are not authoritative.",
            "",
        ]
    )
    return "\n".join(lines)


def check_summary(summary_path: Path, expected: str) -> None:
    try:
        actual = summary_path.read_text(encoding="utf-8")
    except FileNotFoundError as exc:
        raise ManifestError(
            f"generated summary is missing: {summary_path}; run with --write-summary"
        ) from exc
    if actual != expected:
        raise ManifestError(
            f"generated summary is stale: {summary_path}; run with --write-summary"
        )


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    parser.add_argument("--summary", type=Path, default=DEFAULT_SUMMARY)
    parser.add_argument("--write-summary", action="store_true")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    repo_root = args.repo_root.resolve()
    manifest_path = args.manifest if args.manifest.is_absolute() else repo_root / args.manifest
    summary_path = args.summary if args.summary.is_absolute() else repo_root / args.summary
    try:
        manifest = load_manifest(manifest_path)
        validate_manifest(manifest, repo_root)
        expected = render_summary(manifest)
        if args.write_summary:
            summary_path.parent.mkdir(parents=True, exist_ok=True)
            summary_path.write_text(expected, encoding="utf-8")
        else:
            check_summary(summary_path, expected)
    except (ManifestError, OSError) as exc:
        print(f"SDK parity manifest check failed:\n{exc}", file=sys.stderr)
        return 1
    print(
        f"SDK parity manifest OK: {len(manifest['capabilities'])} capabilities; "
        f"summary {'written' if args.write_summary else 'current'}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
