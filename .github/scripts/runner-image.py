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
ARM64_MANIFEST = ".github/runner-requirements.arm64.json"
REPO = "dashpay/platform"
# Only this already-provisioned AMD64 contract may use legacy generic labels.
LEGACY_AMD64_FINGERPRINT = "d272d01bcf3dfa620bbab1e9f31c3e1987862d33ec876bbad83c80f29de41a58"


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
    require(manifest["requirements"]["platform"] in ("linux/amd64", "linux/arm64"),
            "Unsupported image platform")
    if manifest["requirements"]["platform"] == "linux/arm64":
        require(manifest["requirements"].get("profile") == "rust", "ARM64 requires the Rust-only profile")
    return manifest


def runtime_manifest():
    # Hosted selector jobs must not select a manifest for the eventual runner.
    # Only env/verify use the actual job runner's OS and architecture.
    return ARM64_MANIFEST if (os.environ.get("RUNNER_OS") == "Linux"
                              and os.environ.get("RUNNER_ARCH") == "ARM64") else MANIFEST


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


def changed_requirements(pr, manifest_path=MANIFEST):
    require(pr.get("changed_files", 0) <= 3000, "PR exceeds GitHub's file-list limit; requirements need explicit review")
    for page in range(1, 31):
        files = api(f"pulls/{pr['number']}/files?per_page=100&page={page}")
        if any(f["filename"] == manifest_path or f.get("previous_filename") == manifest_path for f in files):
            return True
        if len(files) < 100:
            return False
    return False


def latest_status(head, context):
    # The combined /status endpoint omits creator. Full statuses are newest
    # first: select before validating, never fall back to an older success.
    page = 1
    while True:
        statuses = api(f"commits/{head}/statuses?per_page=100&page={page}")
        # GitHub contexts are case-insensitive; a case variant must shadow
        # older canonical statuses even though it cannot be trusted below.
        candidate = next((s for s in statuses if s["context"].casefold() == context.casefold()), None)
        if candidate is not None:
            return candidate
        if len(statuses) < 100:
            return None
        page += 1


def export_environment(manifest, output):
    lock = manifest["requirements"]
    versions = lock["versions"]
    values = {
        "CI_CARGO_LLVM_COV_VERSION": versions["llvm_cov"],
        "CI_CARGO_NEXTEST_VERSION": versions["nextest"],
        "CI_CARGO_MACHETE_VERSION": versions["machete"],
        "CI_PROTOC_VERSION": versions["protoc"], "CI_JAVA_MAJOR": str(lock["java_major"]),
    }
    if lock.get("profile", "full") == "full":
        android = lock["android"]
        values.update({
            "CI_CARGO_NDK_VERSION": versions["cargo_ndk"],
            "CI_ANDROID_API": str(android["api"]), "CI_ANDROID_NDK": android["ndk"],
            "CI_ANDROID_BUILD_TOOLS": android["build_tools"],
        })
    require(all(isinstance(value, str) and re.fullmatch(r"[0-9]+(?:[.][0-9]+){0,3}(?:[-+][A-Za-z0-9.-]+)?", value)
                for value in values.values()), "Versions must be version-pinned, newline-free values")
    with open(output, "a") as handle:
        for key, value in values.items():
            handle.write(f"{key}={value}\n")


def ordinary_labels(manifest, kind, arch=None, validation=False):
    # Linux describes the runner process, not the physical host: ARM64 Linux
    # containers on Macs remain in this pool; native macOS stays for Swift.
    fallback = ["self-hosted"] + (["Linux"] if kind == "rust" else [])
    if arch:
        fallback.append(arch)
    fallback.append("rust-ci-validation" if validation else
                    {"rust": "rust-ci", "kotlin": "kotlin-ci", "npm": "npm-pr"}[kind])
    if arch != "ARM64" and fingerprint(manifest) != LEGACY_AMD64_FINGERPRINT:
        # New AMD64 pools must not carry rust-ci/kotlin-ci/npm-pr: old branches
        # still request those labels and must keep using their exact old image.
        require(manifest["requirements"]["platform"] == "linux/amd64",
                "Versioned ordinary pool requires an AMD64 manifest")
        return ["self-hosted", "Linux", "X64",
                f"platform-image-manifest-{fingerprint(manifest)}-{kind}"]
    return fallback


def select(manifest, kind, output, wait_seconds, arch=None, validation=False):
    require(arch in (None, "", "X64", "ARM64"), "Unsupported runner architecture")
    require(not arch or kind == "rust", "Architecture selection is only supported for Rust")
    require(not validation or (kind == "rust" and arch == "ARM64"),
            "The validation-only pool is for explicitly selected ARM64 Rust jobs")
    fallback = ordinary_labels(manifest, kind, arch, validation)
    event = json.loads(Path(os.environ["GITHUB_EVENT_PATH"]).read_text())
    requested = event.get("pull_request")
    labels, changed = fallback, False
    if requested:
        pr = api(f"pulls/{requested['number']}")
        head = requested["head"]["sha"]
        require(pr["state"] == "open" and pr["head"]["sha"] == head, "This PR run has been superseded")
        changed = changed_requirements(pr, ARM64_MANIFEST if arch == "ARM64" else MANIFEST)
        if not arch and kind == "rust" and changed_requirements(pr, ARM64_MANIFEST):
            # Only validation capacity has the new ARM64 image before rollout.
            # Keep this PR's ordinary job on unchanged AMD64 capacity while its
            # separate ARM64 job proves the new manifest on the validation pool.
            labels = fallback = ordinary_labels(manifest, kind, arch="X64")
        if arch == "ARM64":
            # ARM64 is explicitly provisioned from a published immutable image.
            # The AMD64/KVM candidate publisher is not ARM64 validation. The
            # separate ARM64 job verifies its exact lock/recipe on real capacity.
            if changed:
                import base64
                remote = api("contents/" + ARM64_MANIFEST + "?" + urllib.parse.urlencode({"ref": head}))
                expected = json.loads(base64.b64decode(remote["content"]))
                require(fingerprint(expected) == fingerprint(read_manifest(ARM64_MANIFEST)),
                        "Merge-tree ARM64 requirements differ from PR head; rebase before validation")
            with open(output, "a") as handle:
                handle.write("labels=" + json.dumps(fallback, separators=(",", ":")) + "\n")
                handle.write("image_changed=" + str(changed).lower() + "\n")
            print("Using explicitly provisioned ARM64 image capacity; exact runtime verification is required")
            return
        if changed:
            # Use the exact PR requirement, not an accidental merge-tree mix
            # after both branches edited this file. Rebase such a PR first.
            import base64
            remote = api("contents/" + MANIFEST + "?" + urllib.parse.urlencode({"ref": head}))
            expected = json.loads(base64.b64decode(remote["content"]))
            require(fingerprint(expected) == fingerprint(manifest),
                    "Merge-tree requirements differ from PR head; rebase before building a candidate")
            deadline = time.monotonic() + wait_seconds
            context = f"Runner image candidate / PR {pr['number']}"
            while True:
                candidate = latest_status(head, context)
                if candidate and candidate["state"] == "success":
                    require(candidate["context"] == context, "Candidate status must use the exact publisher context")
                    creator = candidate.get("creator")
                    require(isinstance(creator, dict) and creator.get("login") == "github-actions[bot]",
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
                        # Publication may finish during the retry sleep after
                        # this PR was updated/closed. Do not queue a stale label
                        # that the allocator must refuse to service.
                        current = api(f"pulls/{pr['number']}")
                        require(current["state"] == "open" and current["head"]["sha"] == head,
                                "PR changed before selecting its candidate")
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
    parser.add_argument("--manifest")
    parser.add_argument("--env", default=os.environ.get("GITHUB_ENV"))
    parser.add_argument("--output", default=os.environ.get("GITHUB_OUTPUT"))
    parser.add_argument("--kind", choices=["rust", "kotlin", "npm"])
    parser.add_argument("--arch", choices=["", "X64", "ARM64"])
    parser.add_argument("--validation", action="store_true")
    parser.add_argument("--wait-seconds", type=int, default=7200)
    args = parser.parse_args()
    manifest_path = args.manifest or (MANIFEST if args.command == "select" else runtime_manifest())
    manifest = read_manifest(manifest_path)
    if args.command == "select":
        require(args.kind and args.output, "Runner selection needs kind and output")
        select(manifest, args.kind, args.output, args.wait_seconds, args.arch, args.validation)
        return
    if args.command == "verify":
        subprocess.run(["ci-image-contract", "verify", manifest_path], check=True)
    require(args.env, "Environment output file is required")
    export_environment(manifest, args.env)


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, TypeError, OSError) as error:
        print(f"::error::{error}", file=sys.stderr)
        sys.exit(1)
