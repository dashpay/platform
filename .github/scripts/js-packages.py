#!/usr/bin/env python3
"""Plan and run the JS package lint and test jobs of the Tests workflow.

`plan` picks the packages to lint and test from the paths-filter results and
groups them into jobs. Every job pays for a checkout, Node setup and the build
artifact download, which for most packages takes longer than their tests or
ESLint, so one job lints every package and the packages are tested in the
GROUPS below. A package in no group keeps a job of its own.

`run` runs each package's yarn script in turn and fails after the last one if
any of them failed, so a single run reports every failing package.
"""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[2]
FILTER = ROOT / ".github/package-filters/js-packages-no-workflows.yml"
NODE_ONLY = "Node-only packages"

# Each group's packages share one test job. Measured over October runs,
# dashmate's tests take about 300 s, more than any other package's (its
# integration tests need Docker, so it keeps a job of its own). The karma
# packages are split so that neither group outlasts it: the Evo SDK group's
# tests take about 290 s, the legacy SDK group's about 190 s, and the Node-only
# group's about 100 s.
GROUPS = (
    # Tests that run in Node alone: no karma browser run and no Docker.
    # tests/test_js_packages.py checks each one's scripts and dependencies.
    (NODE_ONLY, (
        "@dashevo/wallet-utils-contract",
        "@dashevo/token-history-contract",
        "@dashevo/document-history-contract",
        "@dashevo/keyword-search-contract",
        "@dashevo/app-connect-contract",
        "@dashevo/moderation-charters-contract",
        "@dashevo/decision-proposals-contract",
        "@dashevo/token-market-contract",
        "@dashevo/dashpay-contract",
        "@dashevo/masternode-reward-shares-contract",
        "@dashevo/withdrawals-contract",
        "@dashevo/dpns-contract",
        "@dashevo/grpc-common",
        "@dashevo/dapi-grpc",
        "@dashevo/dash-spv",
        "@dashevo/dapi",
        "@dashevo/platform-test-suite",
    )),
    ("Evo SDK packages", (
        "@dashevo/evo-sdk",
        "@dashevo/wasm-sdk",
        "@dashevo/wasm-dpp2",
    )),
    ("legacy SDK packages", (
        "dash",
        "@dashevo/wallet-lib",
        "@dashevo/dapi-client",
        "@dashevo/wasm-dpp",
    )),
)

TEST_COMMANDS = {
    # The default command also runs the E2E tests. The integration tests need
    # Docker, which the GitHub-hosted runner has, but not a live network.
    "dashmate": "test:ci",
    # The default command drives a live network, so it runs from the E2E jobs.
    # The unit tests need nothing.
    "@dashevo/platform-test-suite": "test:unit",
}

# The heaviest package: its tests run on the nightly schedule and on manual
# runs only. `yarn build` still compiles it for every pull request, and the
# packages built on it keep running their tests there.
NIGHTLY_ONLY = ("@dashevo/wasm-dpp",)


def packages_in_filter(path=FILTER):
    """Return the top-level package names of a paths-filter file, in order."""
    names = []
    for line in Path(path).read_text().splitlines():
        match = re.match(r"(?:'([^']+)'|([^\s#'-][^\s:]*)):", line)
        if match:
            names.append(match.group(1) or match.group(2))
    return names


def select(event, every, changed, changed_direct, known):
    """Return the packages to test and, of those, the packages to lint.

    `every` tests and lints every known package: on a manual run, and when a
    pull request changes these jobs themselves. Only packages whose own files
    changed are linted otherwise.
    """
    tested = list(known) if every else list(changed)
    direct = set(known) if every else set(changed_direct)
    if event == "pull_request":
        tested = [package for package in tested if package not in NIGHTLY_ONLY]
    return tested, [package for package in tested if package in direct]


def entry(package, command):
    return {"package": package, "command": command}


def test_jobs(tested):
    """Give each package outside GROUPS a job and each group with work one job.

    A group with a single package to test is named after that package.
    """
    grouped = {package for _, members in GROUPS for package in members}
    batches = [(package, [package]) for package in tested if package not in grouped]
    for name, members in GROUPS:
        batch = [package for package in tested if package in members]
        if batch:
            batches.append((batch[0] if len(batch) == 1 else name, batch))
    return [
        {"name": name, "packages": [entry(package, TEST_COMMANDS.get(package, "test")) for package in batch]}
        for name, batch in batches
    ]


def parse_list(value):
    return json.loads(value) if value else []


def plan(output):
    event = os.environ["EVENT_NAME"]
    every = event == "workflow_dispatch" or os.environ.get("WORKFLOWS_CHANGED") == "true"
    tested, linted = select(
        event,
        every,
        parse_list(os.environ.get("CHANGED")),
        parse_list(os.environ.get("CHANGED_DIRECT")),
        packages_in_filter(),
    )
    jobs = test_jobs(tested)
    compact = {"separators": (",", ":")}
    with open(output, "a", encoding="utf-8") as handle:
        handle.write("packages=" + json.dumps(tested, **compact) + "\n")
        handle.write("lint=" + json.dumps([entry(package, "lint") for package in linted], **compact) + "\n")
        handle.write("test-jobs=" + json.dumps(jobs, **compact) + "\n")
    print("Lint:", ", ".join(linted) or "nothing")
    for job in jobs:
        print(f"Test job {job['name']}:", ", ".join(item["package"] for item in job["packages"]))


def run(entries):
    if not entries:
        raise ValueError("No JS packages to run")
    failed = []
    for item in entries:
        package, command = item["package"], item["command"]
        print(f"::group::{package}: yarn {command}", flush=True)
        code = subprocess.run(["yarn", "workspace", package, command], stdin=subprocess.DEVNULL).returncode
        print("::endgroup::", flush=True)
        if code:
            failed.append(package)
            print(f"::error title=yarn {command} failed::{package} exited with {code}", flush=True)
    if failed:
        print("Failed:", ", ".join(failed))
        return 1
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["plan", "run"])
    parser.add_argument("--output", default=os.environ.get("GITHUB_OUTPUT"))
    args = parser.parse_args()
    if args.command == "plan":
        if not args.output:
            raise ValueError("Planning needs an output file")
        plan(args.output)
        return 0
    return run(parse_list(os.environ.get("PACKAGES")))


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (ValueError, KeyError, TypeError, OSError) as error:
        print(f"::error::{error}", file=sys.stderr)
        sys.exit(1)
