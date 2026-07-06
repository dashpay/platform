#!/usr/bin/env python3
"""Verify the wallet reverse-dependency closure.

The wallet-only CI fast path (.github/workflows/tests-rs-wallet.yml) compiles
and tests only the wallet crates plus their known dependents. That is sound
only while the set of workspace crates depending (transitively) on the wallet
crates is exactly EXPECTED_DEPENDENTS below.

This script runs on BOTH Rust CI paths: on the full workspace path it fails
the PR that introduces a new dependent (the moment the invariant breaks), and
on the wallet fast path it protects against a stale scoped package list.

On failure: extend the scoped --package lists in
.github/workflows/tests-rs-wallet.yml, add the new dependent to
EXPECTED_DEPENDENTS here, and reconsider whether the fast path is still sound.
"""

import json
import subprocess
import sys

WALLET_CRATES = {"platform-wallet", "platform-wallet-ffi", "platform-wallet-storage"}
EXPECTED_DEPENDENTS = {"rs-unified-sdk-ffi"}


def main() -> int:
    meta = json.loads(
        subprocess.check_output(
            ["cargo", "metadata", "--format-version", "1", "--no-deps", "--locked"]
        )
    )
    deps = {p["name"]: {d["name"] for d in p["dependencies"]} for p in meta["packages"]}

    # Transitive closure of workspace crates that depend on a wallet crate
    reaches_wallet = set(WALLET_CRATES)
    changed = True
    while changed:
        changed = False
        for name, d in deps.items():
            if name not in reaches_wallet and d & reaches_wallet:
                reaches_wallet.add(name)
                changed = True

    dependents = reaches_wallet - WALLET_CRATES
    if dependents != EXPECTED_DEPENDENTS:
        print(
            f"Wallet reverse-dependency closure changed: "
            f"expected {sorted(EXPECTED_DEPENDENTS)}, got {sorted(dependents)}"
        )
        print(
            "Update the scoped --package lists in "
            ".github/workflows/tests-rs-wallet.yml and EXPECTED_DEPENDENTS in "
            ".github/scripts/check-wallet-closure.py to cover the new dependents."
        )
        return 1

    print(f"Closure OK: only {sorted(EXPECTED_DEPENDENTS)} depends on the wallet crates")
    return 0


if __name__ == "__main__":
    sys.exit(main())
