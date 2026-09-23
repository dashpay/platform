#!/usr/bin/env python3
"""Audit the DashVM engine dependency set against the RustSec advisory database.

The engine (Wasmtime, Cranelift and the wasm-tools crates that validate and
encode modules) decides consensus results, so an advisory against one of its
crates is not one line in the base-wide nightly audit: it is a signal that a
node release or a protocol upgrade is due, classified by the rule in
book/src/architecture/engine-dependencies-and-hotfixes.md.

The set is declared in the root Cargo.toml under
[workspace.metadata.dashvm.engine]:

  crates        engine crate names; every version the lockfile resolves is
                audited, a crate the lockfile does not contain is reported as
                planned and skipped
  acknowledged  scoped exceptions, each with an advisory id, a reason and an
                ISO 8601 expiry date
  patches       declared temporary patches, each with the crate, the upstream
                fix it carries and an expiry date

cargo-audit runs from a fresh temporary directory so the workspace ignore list
in .cargo/audit.toml cannot hide an engine advisory. The audit fails on:

  * a vulnerability against an engine crate without a valid acknowledgement
  * an acknowledgement that has expired, lacks a reason or an expiry, or runs
    more than MAX_EXCEPTION_DAYS from today (provisional bound)
  * an unmaintained, unsound or yanked engine crate (the policy is pinned
    upstream-maintained releases, so these are policy failures, not advisories
    to defer)
  * a [patch.*] entry for an engine crate in the root manifest without a
    declaration, or with an expired or over-long declaration

Exit codes: 0 clean, 1 findings, 2 the audit could not run (cargo-audit
missing or failing, unreadable manifest, lockfile or report).

`--self-test` evaluates manifests, lockfiles and reports built in memory and
proves that the checker accepts a clean set and rejects each kind of finding.
The workflow runs it before every real audit so a green run means the set was
audited, not that nothing was compared. `--report <json>` evaluates a saved
`cargo audit --json` report offline, which reproduces a CI failure locally.
"""

import argparse
import datetime
import json
import os
import subprocess
import sys
import tempfile
import tomllib

EXIT_CLEAN = 0
EXIT_FINDINGS = 1
EXIT_ERROR = 2

# Provisional bound (no value in the issue register): neither an acknowledgement
# nor a temporary patch may run more than this many days from the day the audit
# runs. Wasmtime patch releases on a supported line have shipped at least monthly,
# so a quarter is long enough to bump and short enough to be noticed.
MAX_EXCEPTION_DAYS = 90

# Warning kinds that fail the audit on an engine crate. `notice` is printed only.
FAILING_WARNING_KINDS = ("unmaintained", "unsound", "yanked")

REPO_ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))


class AuditError(Exception):
    """The audit cannot run at all (exit 2)."""


# --------------------------------------------------------------------------- inputs


def parse_manifest(text, label="Cargo.toml"):
    """Return the engine set, its exceptions and every [patch.*] entry of a manifest."""
    try:
        manifest = tomllib.loads(text)
    except tomllib.TOMLDecodeError as error:
        raise AuditError(f"{label}: cannot parse manifest: {error}") from error

    engine = manifest.get("workspace", {}).get("metadata", {}).get("dashvm", {}).get("engine")
    if not isinstance(engine, dict) or not isinstance(engine.get("crates"), list):
        raise AuditError(
            f"{label}: [workspace.metadata.dashvm.engine] with a `crates` list is required"
        )
    crates = engine["crates"]
    if not crates or not all(isinstance(name, str) and name for name in crates):
        raise AuditError(f"{label}: `crates` must be a non-empty list of crate names")

    acknowledged = engine.get("acknowledged", [])
    patches = engine.get("patches", [])
    for field, value in (("acknowledged", acknowledged), ("patches", patches)):
        if not isinstance(value, list) or not all(isinstance(entry, dict) for entry in value):
            raise AuditError(f"{label}: `{field}` must be a list of tables")

    patched_crates = set()
    for entries in manifest.get("patch", {}).values():
        if isinstance(entries, dict):
            patched_crates.update(entries.keys())

    return {
        "crates": list(crates),
        "acknowledged": acknowledged,
        "patches": patches,
        "patched_crates": patched_crates,
    }


def parse_lockfile(text, label="Cargo.lock"):
    """Return {crate name: sorted list of resolved versions}."""
    try:
        lock = tomllib.loads(text)
    except tomllib.TOMLDecodeError as error:
        raise AuditError(f"{label}: cannot parse lockfile: {error}") from error
    versions = {}
    for package in lock.get("package", []):
        name = package.get("name")
        version = package.get("version")
        if isinstance(name, str) and isinstance(version, str):
            versions.setdefault(name, []).append(version)
    return {name: sorted(found) for name, found in versions.items()}


def parse_report(text, label="cargo-audit report"):
    try:
        report = json.loads(text)
    except ValueError as error:
        raise AuditError(f"{label}: cannot parse report: {error}") from error
    if not isinstance(report, dict) or "vulnerabilities" not in report:
        raise AuditError(f"{label}: not a cargo-audit JSON report")
    return report


def read_text(path, label):
    try:
        with open(path, "r", encoding="utf-8") as handle:
            return handle.read()
    except OSError as error:
        raise AuditError(f"{label}: cannot read {path}: {error}") from error


def run_cargo_audit(lockfile, out=sys.stdout):
    """Run `cargo audit --json` from a fresh directory so no audit.toml is found."""
    lockfile = os.path.abspath(lockfile)
    command = ["cargo", "audit", "--json", "--file", lockfile]
    with tempfile.TemporaryDirectory(prefix="engine-audit-") as scratch:
        try:
            completed = subprocess.run(
                command,
                cwd=scratch,
                capture_output=True,
                text=True,
                check=False,
            )
        except OSError as error:
            raise AuditError(f"cannot run cargo-audit: {error}") from error
    if completed.returncode not in (0, 1):
        raise AuditError(
            f"cargo-audit exited {completed.returncode}: {completed.stderr.strip()[-2000:]}"
        )
    report = parse_report(completed.stdout, "cargo-audit output")
    database = report.get("database", {})
    print(
        "advisory database: {} advisories, commit {}, updated {}".format(
            database.get("advisory-count", "?"),
            database.get("last-commit", "?"),
            database.get("last-updated", "?"),
        ),
        file=out,
    )
    return report


# --------------------------------------------------------------------------- evaluation


def parse_date(value):
    """ISO 8601 date string to date, or None when absent or malformed."""
    if isinstance(value, datetime.date):
        return value
    if not isinstance(value, str):
        return None
    try:
        return datetime.date.fromisoformat(value)
    except ValueError:
        return None


def check_expiry(kind, name, entry, today, failures):
    """Validate the `expires` field of an exception; return True when it is usable."""
    expires = parse_date(entry.get("expires"))
    if expires is None:
        failures.append(f"{kind} {name}: `expires` must be an ISO 8601 date")
        return False
    if expires < today:
        failures.append(f"{kind} {name}: expired on {expires.isoformat()}")
        return False
    if (expires - today).days > MAX_EXCEPTION_DAYS:
        failures.append(
            f"{kind} {name}: expires {expires.isoformat()}, more than "
            f"{MAX_EXCEPTION_DAYS} days out (provisional bound); shorten it"
        )
        return False
    return True


def valid_acknowledgements(manifest, today, failures):
    """Return the set of advisory ids with a currently valid acknowledgement."""
    valid = set()
    for entry in manifest["acknowledged"]:
        advisory_id = entry.get("id")
        if not isinstance(advisory_id, str) or not advisory_id:
            failures.append("acknowledgement without an advisory `id`")
            continue
        reason = entry.get("reason")
        usable = True
        if not isinstance(reason, str) or not reason.strip():
            failures.append(f"acknowledgement {advisory_id}: `reason` is required")
            usable = False
        if not check_expiry("acknowledgement", advisory_id, entry, today, failures):
            usable = False
        if usable:
            valid.add(advisory_id)
    return valid


def valid_patch_declarations(manifest, today, failures):
    """Return the set of engine crates with a currently valid temporary-patch declaration."""
    valid = set()
    for entry in manifest["patches"]:
        crate = entry.get("crate")
        if not isinstance(crate, str) or not crate:
            failures.append("patch declaration without a `crate`")
            continue
        upstream = entry.get("upstream")
        usable = True
        if not isinstance(upstream, str) or not upstream.startswith("https://"):
            failures.append(f"patch {crate}: `upstream` must be the URL of the upstream fix")
            usable = False
        if not check_expiry("patch", crate, entry, today, failures):
            usable = False
        if usable:
            valid.add(crate)
    return valid


def describe(finding):
    advisory = finding.get("advisory") or {}
    package = finding.get("package") or {}
    patched = (finding.get("versions") or {}).get("patched") or []
    parts = [f"{package.get('name')} {package.get('version')}"]
    if advisory.get("id"):
        parts.append(advisory["id"])
    if advisory.get("title"):
        parts.append(advisory["title"])
    if patched:
        parts.append("patched in " + ", ".join(patched))
    if advisory.get("url"):
        parts.append(advisory["url"])
    return ": ".join(parts[:2]) + (" (" + "; ".join(parts[2:]) + ")" if len(parts) > 2 else "")


def evaluate(manifest, lock, report, today):
    """Pure core: returns (exit code, output lines)."""
    failures = []
    notices = []
    engine = set(manifest["crates"])

    for name in manifest["crates"]:
        if name in lock:
            notices.append(f"engine crate {name}: auditing {', '.join(lock[name])}")
        else:
            notices.append(f"engine crate {name}: planned, not in the lockfile")

    acknowledged = valid_acknowledgements(manifest, today, failures)
    declared_patches = valid_patch_declarations(manifest, today, failures)

    seen_advisories = set()
    for finding in report.get("vulnerabilities", {}).get("list", []):
        package = (finding.get("package") or {}).get("name")
        if package not in engine:
            continue
        advisory_id = (finding.get("advisory") or {}).get("id")
        seen_advisories.add(advisory_id)
        if advisory_id in acknowledged:
            notices.append("acknowledged vulnerability " + describe(finding))
        else:
            failures.append("vulnerability " + describe(finding))

    for kind, findings in (report.get("warnings") or {}).items():
        for finding in findings or []:
            package = (finding.get("package") or {}).get("name")
            if package not in engine:
                continue
            advisory_id = (finding.get("advisory") or {}).get("id")
            if advisory_id:
                seen_advisories.add(advisory_id)
            if kind in FAILING_WARNING_KINDS:
                failures.append(f"{kind} engine crate " + describe(finding))
            else:
                notices.append(f"{kind} on engine crate " + describe(finding))

    for entry in manifest["acknowledged"]:
        advisory_id = entry.get("id")
        if isinstance(advisory_id, str) and advisory_id not in seen_advisories:
            notices.append(f"acknowledgement {advisory_id}: no longer reported, remove it")

    for crate in sorted(manifest["patched_crates"] & engine):
        if crate not in declared_patches:
            failures.append(
                f"[patch] entry for engine crate {crate} without a valid declaration in "
                "[workspace.metadata.dashvm.engine].patches (upstream fix and expiry)"
            )
    for crate in sorted(declared_patches - manifest["patched_crates"]):
        notices.append(f"patch declaration {crate}: no [patch] entry, remove the declaration")

    lines = [f"FAIL {line}" for line in failures] + [f"note {line}" for line in notices]
    summary = "engine audit: {} crate(s) in the set, {} failure(s), {} notice(s)".format(
        len(engine), len(failures), len(notices)
    )
    lines.append(summary)
    return (EXIT_FINDINGS if failures else EXIT_CLEAN), lines


# --------------------------------------------------------------------------- self-test


def sample_manifest(extra="", patch_section=""):
    return (
        "[workspace]\nmembers = []\n\n"
        "[workspace.metadata.dashvm.engine]\n"
        'crates = ["wasmtime", "cranelift-codegen", "wasmparser"]\n'
        + extra
        + patch_section
    )


SAMPLE_LOCK = """
version = 4

[[package]]
name = "wasmtime"
version = "36.0.6"
source = "registry+https://github.com/rust-lang/crates.io-index"

[[package]]
name = "cranelift-codegen"
version = "0.123.6"
source = "registry+https://github.com/rust-lang/crates.io-index"

[[package]]
name = "rustls"
version = "0.23.40"
source = "registry+https://github.com/rust-lang/crates.io-index"
"""


def finding(name, version, advisory_id, title, patched=None):
    return {
        "advisory": {
            "id": advisory_id,
            "package": name,
            "title": title,
            "url": f"https://rustsec.org/advisories/{advisory_id}",
        },
        "versions": {"patched": patched or []},
        "package": {"name": name, "version": version},
    }


def sample_report(vulnerabilities=(), warnings=None):
    return {
        "database": {"advisory-count": 1, "last-commit": "deadbeef", "last-updated": "today"},
        "vulnerabilities": {
            "found": bool(vulnerabilities),
            "count": len(vulnerabilities),
            "list": list(vulnerabilities),
        },
        "warnings": warnings or {},
    }


WASMTIME_VULN = finding(
    "wasmtime", "36.0.6", "RUSTSEC-2026-0096", "aarch64 load miscompile", [">=36.0.7"]
)
RUSTLS_VULN = finding("rustls", "0.23.40", "RUSTSEC-2026-0285", "not an engine crate")


def self_test(out=sys.stdout):
    today = datetime.date(2026, 9, 22)
    checks = []

    def expect(name, manifest_text, report, expected, needle=None, lock_text=SAMPLE_LOCK):
        try:
            code, lines = evaluate(
                parse_manifest(manifest_text, name),
                parse_lockfile(lock_text, name),
                report,
                today,
            )
        except AuditError as error:
            code, lines = EXIT_ERROR, [str(error)]
        joined = "\n".join(lines)
        passed = code == expected and (needle is None or needle in joined)
        checks.append(passed)
        status = "ok  " if passed else "FAIL"
        print(f"self-test {status} {name} (exit {code}, expected {expected})", file=out)
        if not passed:
            for line in lines:
                print(f"    {line}", file=out)

    expect("clean set", sample_manifest(), sample_report(), EXIT_CLEAN, "0 failure(s)")
    expect(
        "engine crate absent from the lockfile is a notice",
        sample_manifest(),
        sample_report(),
        EXIT_CLEAN,
        "wasmparser: planned, not in the lockfile",
    )
    expect(
        "vulnerability on wasmtime fails",
        sample_manifest(),
        sample_report([WASMTIME_VULN]),
        EXIT_FINDINGS,
        "FAIL vulnerability wasmtime 36.0.6: RUSTSEC-2026-0096",
    )
    expect(
        "vulnerability on a non-engine crate is out of scope",
        sample_manifest(),
        sample_report([RUSTLS_VULN]),
        EXIT_CLEAN,
        "0 failure(s)",
    )
    valid_ack = (
        'acknowledged = [{ id = "RUSTSEC-2026-0096", '
        'reason = "aarch64 lowering not compiled in", expires = "2026-10-31" }]\n'
    )
    expect(
        "valid acknowledgement passes with a notice",
        sample_manifest(valid_ack),
        sample_report([WASMTIME_VULN]),
        EXIT_CLEAN,
        "note acknowledged vulnerability wasmtime",
    )
    expect(
        "expired acknowledgement fails",
        sample_manifest(
            'acknowledged = [{ id = "RUSTSEC-2026-0096", reason = "x", expires = "2026-09-21" }]\n'
        ),
        sample_report([WASMTIME_VULN]),
        EXIT_FINDINGS,
        "expired on 2026-09-21",
    )
    expect(
        "acknowledgement beyond the provisional cap fails",
        sample_manifest(
            'acknowledged = [{ id = "RUSTSEC-2026-0096", reason = "x", expires = "2027-06-01" }]\n'
        ),
        sample_report([WASMTIME_VULN]),
        EXIT_FINDINGS,
        f"more than {MAX_EXCEPTION_DAYS} days out",
    )
    expect(
        "acknowledgement without a reason fails",
        sample_manifest('acknowledged = [{ id = "RUSTSEC-2026-0096", expires = "2026-10-31" }]\n'),
        sample_report([WASMTIME_VULN]),
        EXIT_FINDINGS,
        "`reason` is required",
    )
    expect(
        "stale acknowledgement is a notice",
        sample_manifest(valid_ack),
        sample_report(),
        EXIT_CLEAN,
        "RUSTSEC-2026-0096: no longer reported, remove it",
    )
    expect(
        "unmaintained cranelift-codegen fails",
        sample_manifest(),
        sample_report(
            warnings={
                "unmaintained": [
                    finding("cranelift-codegen", "0.123.6", "RUSTSEC-2099-0001", "unmaintained")
                ]
            }
        ),
        EXIT_FINDINGS,
        "FAIL unmaintained engine crate cranelift-codegen",
    )
    expect(
        "yanked wasmtime fails",
        sample_manifest(),
        sample_report(
            warnings={"yanked": [{"advisory": None, "package": {"name": "wasmtime", "version": "36.0.6"}}]}
        ),
        EXIT_FINDINGS,
        "FAIL yanked engine crate wasmtime 36.0.6",
    )
    expect(
        "notice on an engine crate does not fail",
        sample_manifest(),
        sample_report(
            warnings={"notice": [finding("wasmtime", "36.0.6", "RUSTSEC-2099-0002", "notice only")]}
        ),
        EXIT_CLEAN,
        "note notice on engine crate wasmtime",
    )
    expect(
        "unsound warning on a non-engine crate is out of scope",
        sample_manifest(),
        sample_report(warnings={"unsound": [finding("rustls", "0.23.40", "RUSTSEC-2099-0003", "x")]}),
        EXIT_CLEAN,
        "0 failure(s)",
    )
    patch_section = '\n[patch.crates-io]\nwasmtime = { git = "https://github.com/dashpay/wasmtime", rev = "abc" }\n'
    expect(
        "undeclared [patch] on an engine crate fails",
        sample_manifest(patch_section=patch_section),
        sample_report(),
        EXIT_FINDINGS,
        "[patch] entry for engine crate wasmtime without a valid declaration",
    )
    expect(
        "declared temporary patch passes",
        sample_manifest(
            'patches = [{ crate = "wasmtime", '
            'upstream = "https://github.com/bytecodealliance/wasmtime/pull/1", expires = "2026-11-30" }]\n',
            patch_section,
        ),
        sample_report(),
        EXIT_CLEAN,
        "0 failure(s)",
    )
    expect(
        "declared patch with an expired date fails",
        sample_manifest(
            'patches = [{ crate = "wasmtime", '
            'upstream = "https://github.com/bytecodealliance/wasmtime/pull/1", expires = "2026-01-01" }]\n',
            patch_section,
        ),
        sample_report(),
        EXIT_FINDINGS,
        "patch wasmtime: expired on 2026-01-01",
    )
    expect(
        "declared patch without an upstream fix fails",
        sample_manifest(
            'patches = [{ crate = "wasmtime", expires = "2026-11-30" }]\n', patch_section
        ),
        sample_report(),
        EXIT_FINDINGS,
        "`upstream` must be the URL of the upstream fix",
    )
    expect(
        "[patch] on a non-engine crate is out of scope",
        sample_manifest(patch_section='\n[patch.crates-io]\nrustls = { path = "vendor/rustls" }\n'),
        sample_report(),
        EXIT_CLEAN,
        "0 failure(s)",
    )
    expect(
        "retired patch declaration is a notice",
        sample_manifest(
            'patches = [{ crate = "wasmtime", '
            'upstream = "https://github.com/bytecodealliance/wasmtime/pull/1", expires = "2026-11-30" }]\n'
        ),
        sample_report(),
        EXIT_CLEAN,
        "patch declaration wasmtime: no [patch] entry, remove the declaration",
    )
    expect(
        "manifest without the engine table is an error",
        "[workspace]\nmembers = []\n",
        sample_report(),
        EXIT_ERROR,
        "[workspace.metadata.dashvm.engine]",
    )

    failed = len(checks) - sum(checks)
    print(f"self-test: {len(checks) - failed} of {len(checks)} checks passed", file=out)
    return failed == 0


# --------------------------------------------------------------------------- entry point


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--manifest",
        default=os.path.join(REPO_ROOT, "Cargo.toml"),
        help="root Cargo.toml carrying [workspace.metadata.dashvm.engine]",
    )
    parser.add_argument(
        "--lockfile",
        default=os.path.join(REPO_ROOT, "Cargo.lock"),
        help="lockfile to audit (passed to cargo audit --file)",
    )
    parser.add_argument(
        "--report",
        help="evaluate a saved `cargo audit --json` report instead of running cargo-audit",
    )
    parser.add_argument(
        "--today",
        help="evaluate expiries against this ISO 8601 date instead of the current date",
    )
    parser.add_argument(
        "--self-test",
        action="store_true",
        help="evaluate in-memory manifests and reports and check every exit code",
    )
    args = parser.parse_args(argv)

    if args.self_test:
        return EXIT_CLEAN if self_test() else EXIT_FINDINGS

    today = parse_date(args.today) if args.today else datetime.date.today()
    if today is None:
        parser.error("--today must be an ISO 8601 date")

    try:
        manifest = parse_manifest(read_text(args.manifest, "manifest"), args.manifest)
        lock = parse_lockfile(read_text(args.lockfile, "lockfile"), args.lockfile)
        if args.report:
            report = parse_report(read_text(args.report, "report"), args.report)
        else:
            report = run_cargo_audit(args.lockfile)
    except AuditError as error:
        print(f"engine audit error: {error}")
        return EXIT_ERROR

    code, lines = evaluate(manifest, lock, report, today)
    for line in lines:
        print(line)
    return code


if __name__ == "__main__":
    sys.exit(main())
