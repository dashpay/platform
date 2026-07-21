#!/usr/bin/env python3
"""Verify that untrusted fork pull requests cannot reach the Swift self-hosted runner.

Defense in depth for DS-CAND-103. For pull_request events GitHub evaluates
workflow YAML from the PR merge commit, so a malicious fork can edit the
guards this script checks — and this script itself. The authoritative trust
boundary is therefore server-side: the repository Actions setting that
requires approval before outside-collaborator workflow runs. This check
exists to catch accidental regressions on trusted runs (same-repo branches,
pushes, schedules), where the merge commit is not attacker-controlled.

The workflows are parsed structurally (PyYAML, preinstalled on GitHub's
Ubuntu runner images), so harmless reformatting does not break the check
while any semantic change to the guard expressions still fails it.
"""

import re
import sys
from pathlib import Path

try:
    import yaml
except ImportError:
    print(
        "self-hosted Swift runner policy check requires PyYAML "
        "(python3 -m pip install pyyaml)",
        file=sys.stderr,
    )
    sys.exit(2)

CALLER_PATH = Path(".github/workflows/tests.yml")
CALLEE_PATH = Path(".github/workflows/swift-sdk-build.yml")
JOB_NAME = "swift-sdk-build"

# Same trusted-fork policy as the sibling self-hosted workflows
# (tests-rs-wallet.yml, tests-rs-workspace.yml): same-repository pull
# requests plus thepastaclaw-owned forks.
CALLEE_POLICY = (
    "github.event_name!='pull_request'"
    "||github.event.pull_request.head.repo.full_name==github.repository"
    "||github.event.pull_request.head.repo.owner.login=='thepastaclaw'"
)
CALLER_POLICY = (
    "always()"
    f"&&({CALLEE_POLICY})"
    "&&needs.changes.outputs.swift-sdk-changed=='true'"
    "&&needs.changes.result!='failure'"
    "&&needs.rs-workspace-tests.result!='failure'"
    "&&needs.rs-wallet-tests.result!='failure'"
)


def load_workflow(path: Path) -> dict:
    document = yaml.safe_load(path.read_text(encoding="utf-8"))
    if not isinstance(document, dict):
        raise ValueError(f"{path}: expected a mapping at the document root")
    return document


def workflow_triggers(document: dict):
    # PyYAML resolves a bare `on` key as boolean True (YAML 1.1).
    return document.get("on", document.get(True))


def job(document: dict, job_name: str) -> dict:
    jobs = document.get("jobs")
    found = jobs.get(job_name) if isinstance(jobs, dict) else None
    return found if isinstance(found, dict) else {}


def normalized_if(job_mapping: dict) -> str:
    """Strip the optional ${{ }} wrapper and all whitespace for comparison."""
    condition = job_mapping.get("if")
    if not isinstance(condition, str):
        return ""
    expression = condition.strip()
    if expression.startswith("${{") and expression.endswith("}}"):
        expression = expression[3:-2]
    return re.sub(r"\s+", "", expression)


def check_policy(root: Path) -> list[str]:
    caller = load_workflow(root / CALLER_PATH)
    callee = load_workflow(root / CALLEE_PATH)
    caller_job = job(caller, JOB_NAME)
    callee_job = job(callee, JOB_NAME)
    errors = []

    triggers = workflow_triggers(caller)
    if not (isinstance(triggers, dict) and "pull_request" in triggers):
        errors.append(f"{CALLER_PATH}: expected pull_request trigger")
    if caller_job.get("uses") != "./.github/workflows/swift-sdk-build.yml":
        errors.append(f"{CALLER_PATH}: {JOB_NAME} must call the Swift workflow")
    if normalized_if(caller_job) != CALLER_POLICY:
        errors.append(f"{CALLER_PATH}: {JOB_NAME} lacks the strict fork PR policy")
    if normalized_if(callee_job) != CALLEE_POLICY:
        errors.append(f"{CALLEE_PATH}: {JOB_NAME} lacks the strict fork PR policy")

    runs_on = callee_job.get("runs-on")
    labels = runs_on if isinstance(runs_on, list) else [runs_on]
    if "self-hosted" not in labels:
        errors.append(f"{CALLEE_PATH}: expected self-hosted runner boundary")

    steps = callee_job.get("steps")
    commands = "\n".join(
        step["run"]
        for step in (steps if isinstance(steps, list) else [])
        if isinstance(step, dict) and isinstance(step.get("run"), str)
    )
    if not re.search(r"(?m)^\s*bash\s+packages/swift-sdk/run_tests\.sh\s*$", commands):
        errors.append(f"{CALLEE_PATH}: expected Swift test entrypoint")

    return errors


def main() -> int:
    root = Path(sys.argv[1]) if len(sys.argv) > 1 else Path.cwd()
    try:
        errors = check_policy(root)
    except (OSError, UnicodeDecodeError, ValueError, yaml.YAMLError) as error:
        print(f"self-hosted Swift runner policy check failed: {error}", file=sys.stderr)
        return 2

    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        return 1

    print("Self-hosted Swift runner policy OK: untrusted fork PRs are rejected")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
