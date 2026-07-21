#!/usr/bin/env python3
"""Verify that fork pull requests cannot reach the Swift self-hosted runner."""

import re
import sys
from pathlib import Path


CALLER_PATH = Path(".github/workflows/tests.yml")
CALLEE_PATH = Path(".github/workflows/swift-sdk-build.yml")
JOB_NAME = "swift-sdk-build"
CALLEE_POLICY = (
    "github.event_name!='pull_request'||"
    "github.event.pull_request.head.repo.full_name==github.repository"
)
CALLER_POLICY = (
    "always()&&("
    f"{CALLEE_POLICY}"
    ")&&needs.changes.outputs.swift-sdk-changed=='true'"
    "&&needs.changes.result!='failure'"
    "&&needs.rs-workspace-tests.result!='failure'"
    "&&needs.rs-wallet-tests.result!='failure'"
)


def job_block(workflow: str, job_name: str) -> str:
    """Return one top-level job block without accepting sibling job guards."""
    lines = workflow.splitlines(keepends=True)
    start_pattern = re.compile(rf"^  {re.escape(job_name)}:\s*(?:#.*)?$")
    sibling_pattern = re.compile(r"^  [A-Za-z0-9_-]+:\s*(?:#.*)?$")

    start = next(
        (
            index
            for index, line in enumerate(lines)
            if start_pattern.match(line.rstrip("\n"))
        ),
        None,
    )
    if start is None:
        return ""

    end = len(lines)
    for index in range(start + 1, len(lines)):
        if sibling_pattern.match(lines[index].rstrip("\n")):
            end = index
            break
    return "".join(lines[start:end])


def job_property(block: str, property_name: str) -> str:
    """Return one job-level property, excluding step-level lookalikes."""
    lines = block.splitlines(keepends=True)
    start_pattern = re.compile(rf"^    {re.escape(property_name)}:\s*(?:.*)?$")
    sibling_pattern = re.compile(r"^    [A-Za-z0-9_-]+:\s*(?:.*)?$")

    start = next(
        (
            index
            for index, line in enumerate(lines)
            if start_pattern.match(line.rstrip("\n"))
        ),
        None,
    )
    if start is None:
        return ""

    end = len(lines)
    for index in range(start + 1, len(lines)):
        if sibling_pattern.match(lines[index].rstrip("\n")):
            end = index
            break
    return "".join(lines[start:end])


def normalized_if(block: str) -> str:
    """Normalize one job condition for an exact policy comparison."""
    condition = job_property(block, "if")
    if not condition:
        return ""

    first_line, *continuation = condition.splitlines()
    first_value = first_line.split(":", 1)[1].strip()
    if first_value in {">", ">-", "|", "|-"}:
        first_value = ""
    expression = first_value + "".join(line.strip() for line in continuation)
    if expression.startswith("$" + "{{") and expression.endswith("}}"):
        expression = expression[3:-2]
    return re.sub(r"\s+", "", expression)


def check_policy(root: Path) -> list[str]:
    caller = (root / CALLER_PATH).read_text(encoding="utf-8")
    callee = (root / CALLEE_PATH).read_text(encoding="utf-8")
    caller_job = job_block(caller, JOB_NAME)
    callee_job = job_block(callee, JOB_NAME)
    errors = []

    if not re.search(r"(?m)^  pull_request\s*:", caller):
        errors.append(f"{CALLER_PATH}: expected pull_request trigger")
    if not re.search(
        r"uses:\s*\./\.github/workflows/swift-sdk-build\.yml(?:\s|$)",
        job_property(caller_job, "uses"),
    ):
        errors.append(f"{CALLER_PATH}: {JOB_NAME} must call the Swift workflow")
    if normalized_if(caller_job) != CALLER_POLICY:
        errors.append(f"{CALLER_PATH}: {JOB_NAME} lacks the strict fork PR policy")
    if normalized_if(callee_job) != CALLEE_POLICY:
        errors.append(f"{CALLEE_PATH}: {JOB_NAME} lacks the strict fork PR policy")
    if not re.search(
        r"(?m)^\s*runs-on:.*\bself-hosted\b", job_property(callee_job, "runs-on")
    ):
        errors.append(f"{CALLEE_PATH}: expected self-hosted runner boundary")
    if not re.search(
        r"(?m)^\s*bash\s+packages/swift-sdk/run_tests\.sh\s*$", callee_job
    ):
        errors.append(f"{CALLEE_PATH}: expected Swift test entrypoint")

    return errors


def main() -> int:
    root = Path(sys.argv[1]) if len(sys.argv) > 1 else Path.cwd()
    try:
        errors = check_policy(root)
    except (OSError, UnicodeDecodeError) as error:
        print(f"self-hosted Swift runner policy check failed: {error}", file=sys.stderr)
        return 2

    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        return 1

    print("Self-hosted Swift runner policy OK: fork pull requests are rejected")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
