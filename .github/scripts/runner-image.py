#!/usr/bin/env python3
"""Select an exact PR image and export the shared runner tool requirements."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time
import urllib.parse
import urllib.request

MANIFEST = ".github/runner-requirements.json"
REPO = "dashpay/platform"


def require(condition, message):
    if not condition:
        raise ValueError(message)


def read_manifest(path):
    data = Path(path).read_bytes()
    require(len(data) <= 256 * 1024, "Requirements manifest is too large")
    manifest = json.loads(data)
    require(set(manifest) == {"schema", "recipe_revision", "requirements"} and manifest["schema"] == 1,
            "Unsupported requirements manifest")
    require(re.fullmatch(r"[0-9a-f]{40}", manifest["recipe_revision"]), "Pin the image recipe to a full SHA")
    require(manifest["requirements"]["platform"] == "linux/amd64", "Unsupported image platform")
    return manifest


def fingerprint(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


def api(path):
    token = os.environ.get("GH_TOKEN", "")
    headers = {"Accept": "application/vnd.github+json", "User-Agent": "platform-runner-image"}
    if token:
        headers["Authorization"] = "Bearer " + token
    request = urllib.request.Request("https://api.github.com/repos/" + REPO + "/" + path, headers=headers)
    with urllib.request.urlopen(request, timeout=30) as response:
        return json.load(response)


def changed_requirements(pr):
    require(pr.get("changed_files", 0) <= 3000, "PR exceeds GitHub's file-list limit; requirements need explicit review")
    for page in range(1, 31):
        files = api(f"pulls/{pr['number']}/files?per_page=100&page={page}")
        if any(f["filename"] == MANIFEST or f.get("previous_filename") == MANIFEST for f in files):
            return True
        if len(files) < 100:
            return False
    return False


def export_environment(manifest, output):
    lock = manifest["requirements"]
    versions, android = lock["versions"], lock["android"]
    values = {
        "CI_CARGO_LLVM_COV_VERSION": versions["llvm_cov"],
        "CI_CARGO_NEXTEST_VERSION": versions["nextest"],
        "CI_CARGO_MACHETE_VERSION": versions["machete"],
        "CI_CARGO_NDK_VERSION": versions["cargo_ndk"],
        "CI_PROTOC_VERSION": versions["protoc"], "CI_JAVA_MAJOR": str(lock["java_major"]),
        "CI_ANDROID_API": str(android["api"]), "CI_ANDROID_NDK": android["ndk"],
        "CI_ANDROID_BUILD_TOOLS": android["build_tools"],
    }
    require(all(isinstance(value, str) and re.fullmatch(r"[0-9]+(?:[.][0-9]+){0,3}(?:[-+][A-Za-z0-9.-]+)?", value)
                for value in values.values()), "Versions must be version-pinned, newline-free values")
    with open(output, "a") as handle:
        for key, value in values.items():
            handle.write(f"{key}={value}\n")


def select(manifest, kind, output, wait_seconds):
    fallback = ["self-hosted", "rust-ci" if kind == "rust" else "kotlin-ci"]
    event = json.loads(Path(os.environ["GITHUB_EVENT_PATH"]).read_text())
    requested = event.get("pull_request")
    labels, changed = fallback, False
    if requested:
        pr = api(f"pulls/{requested['number']}")
        head = requested["head"]["sha"]
        require(pr["state"] == "open" and pr["head"]["sha"] == head, "This PR run has been superseded")
        changed = changed_requirements(pr)
        if changed:
            # Use the exact PR requirement, not an accidental merge-tree mix
            # after both branches edited this file. Rebase such a PR first.
            import base64
            remote = api("contents/" + MANIFEST + "?" + urllib.parse.urlencode({"ref": head}))
            expected = json.loads(base64.b64decode(remote["content"]))
            require(fingerprint(expected) == fingerprint(manifest),
                    "Merge-tree requirements differ from PR head; rebase before building a candidate")
            deadline = time.monotonic() + wait_seconds
            while True:
                statuses = api(f"commits/{head}/status")["statuses"]
                candidate = next((s for s in statuses if s["context"] == f"Runner image candidate / PR {pr['number']}"), None)
                if candidate and candidate["state"] == "success":
                    require(candidate.get("creator", {}).get("login") == "github-actions[bot]",
                            "Candidate status must come from the trusted publisher")
                    require(re.fullmatch(r"sha256:[0-9a-f]{64}", candidate.get("description", "")),
                            "Publisher did not record an immutable digest")
                    match = re.fullmatch(r"https://github[.]com/dashpay/platform/actions/runs/([0-9]+)",
                                         candidate.get("target_url", ""))
                    require(match, "Candidate status is not linked to its publishing workflow")
                    run = api(f"actions/runs/{match.group(1)}")
                    require(run["path"] == ".github/workflows/runner-image-candidate.yml"
                            and run["event"] == "pull_request_target", "Unexpected candidate publisher")
                    if run["conclusion"] == "success":
                        labels = ["self-hosted", "Linux", "X64",
                                  f"platform-image-pr-{pr['number']}-{head}-{candidate['description'][7:]}-{kind}"]
                        break
                require(time.monotonic() < deadline,
                        "Candidate image was not published in time. Check Runner image candidate CI and bootstrap setup.")
                current = api(f"pulls/{pr['number']}")
                require(current["state"] == "open" and current["head"]["sha"] == head, "PR changed while waiting")
                time.sleep(20)
    with open(output, "a") as handle:
        handle.write("labels=" + json.dumps(labels, separators=(",", ":")) + "\n")
        handle.write("image_changed=" + str(changed).lower() + "\n")
    print("Candidate runner required" if changed else "Using the ordinary provisioned runner pool")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["env", "verify", "select"])
    parser.add_argument("--manifest", default=MANIFEST)
    parser.add_argument("--env", default=os.environ.get("GITHUB_ENV"))
    parser.add_argument("--output", default=os.environ.get("GITHUB_OUTPUT"))
    parser.add_argument("--kind", choices=["rust", "kotlin"])
    parser.add_argument("--wait-seconds", type=int, default=2400)
    args = parser.parse_args()
    manifest = read_manifest(args.manifest)
    if args.command == "select":
        require(args.kind and args.output, "Runner selection needs kind and output")
        select(manifest, args.kind, args.output, args.wait_seconds)
        return
    if args.command == "verify":
        subprocess.run(["ci-image-contract", "verify", args.manifest], check=True)
    require(args.env, "Environment output file is required")
    export_environment(manifest, args.env)


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, TypeError, OSError) as error:
        print(f"::error::{error}", file=sys.stderr)
        sys.exit(1)
