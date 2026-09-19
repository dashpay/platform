#!/usr/bin/env python3
"""Compare two determinism artifacts recorded on different architectures.

The artifact is written by the drive-abci strategy test
`determinism_tests::should_replay_identically_from_saved_state_across_the_protocol_upgrade`
(schema in packages/rs-drive-abci/tests/strategy_tests/determinism_artifact.rs).
It has three sections with three rules:

  consensus   must be byte-identical between the two recordings; any
              difference (an application hash, a transition code or fee, a
              protocol version, a balance, a credit total) rejects the pair.
  profile     the protocol versions must match, the target architecture must
              differ (unless --allow-same-architecture is given for a local
              two-process check), and the CPU features and engine pin are
              printed so a mismatch can be explained.
  diagnostic  printed side by side, never compared.

Exit codes: 0 match, 1 consensus or profile mismatch, 2 malformed input or
unknown schema.

`--self-test` runs the comparator against artifacts built in memory and
proves that it accepts an identical pair and rejects each kind of
consensus-visible difference. The workflow runs it before every real
comparison so the comparison step cannot pass by comparing nothing.
"""

import argparse
import copy
import json
import sys

KNOWN_SCHEMAS = {1}

EXIT_MATCH = 0
EXIT_MISMATCH = 1
EXIT_MALFORMED = 2


class Malformed(Exception):
    """The artifact cannot be compared at all."""


def load(path):
    try:
        with open(path, "r", encoding="utf-8") as handle:
            artifact = json.load(handle)
    except (OSError, ValueError) as error:
        raise Malformed(f"{path}: cannot read artifact: {error}") from error
    validate_shape(artifact, path)
    return artifact


HEX_HASH_LENGTH = 64

PROFILE_FIELDS = {
    "target_arch": str,
    "target_os": str,
    "pointer_width": int,
    "endian": str,
    "cpu_features": list,
    "protocol_version_start": int,
    "protocol_version_end": int,
}
CONSENSUS_FIELDS = {
    "workload_seed": int,
    "reopened_at_height": int,
    "blocks": list,
    "final_root_hash": str,
    "total_credits": dict,
    "identity_balances": dict,
}
TOTAL_CREDITS_FIELDS = (
    "total_credits_in_platform",
    "total_in_pools",
    "total_identity_balances",
    "total_specialized_balances",
    "total_in_addresses",
    "total_in_shielded_balances",
)
BLOCK_FIELDS = {"height": int, "protocol_version": int, "app_hash": str, "transitions": list}
TRANSITION_FIELDS = {"name": str, "code": int, "fee": int}
DIAGNOSTIC_FIELDS = {"elapsed_ms": int, "block_count": int}


def _require_fields(obj, fields, label):
    if not isinstance(obj, dict):
        raise Malformed(f"{label} is not an object")
    for key, kind in fields.items():
        if key not in obj:
            raise Malformed(f"{label} lacks {key!r}")
        value = obj[key]
        # bool is an int subclass in Python; a boolean where a number belongs is malformed.
        if not isinstance(value, kind) or (kind is int and isinstance(value, bool)):
            raise Malformed(f"{label}.{key} is not {kind.__name__}: {value!r}")


def _require_hash(value, label):
    if len(value) != HEX_HASH_LENGTH or any(c not in "0123456789abcdef" for c in value):
        raise Malformed(f"{label} is not a lowercase 32-byte hex hash: {value!r}")


def validate_shape(artifact, label):
    """Reject anything that is not a complete schema-1 artifact.

    An incomplete recording (a missing fee, an empty block list, a null
    block) must be malformed rather than silently comparable: two
    recordings that both lack a field would otherwise match on nothing.
    """
    if not isinstance(artifact, dict):
        raise Malformed(f"{label}: artifact is not an object")
    schema = artifact.get("schema")
    # bool is an int subclass and 1.0 == 1 in Python; only a real int counts.
    if type(schema) is not int or schema not in KNOWN_SCHEMAS:
        raise Malformed(
            f"{label}: unknown artifact schema {schema!r}; this comparator knows {sorted(KNOWN_SCHEMAS)}"
        )
    for section in ("profile", "consensus", "diagnostic"):
        if not isinstance(artifact.get(section), dict):
            raise Malformed(f"{label}: missing or malformed section {section!r}")

    _require_fields(artifact["profile"], PROFILE_FIELDS, f"{label}: profile")
    _require_fields(artifact["diagnostic"], DIAGNOSTIC_FIELDS, f"{label}: diagnostic")

    consensus = artifact["consensus"]
    _require_fields(consensus, CONSENSUS_FIELDS, f"{label}: consensus")
    _require_hash(consensus["final_root_hash"], f"{label}: consensus.final_root_hash")
    _require_fields(
        consensus["total_credits"],
        {key: int for key in TOTAL_CREDITS_FIELDS},
        f"{label}: consensus.total_credits",
    )
    for identity, balance in consensus["identity_balances"].items():
        _require_hash(identity, f"{label}: consensus.identity_balances key")
        if not isinstance(balance, int) or isinstance(balance, bool):
            raise Malformed(f"{label}: consensus.identity_balances[{identity}] is not int")

    blocks = consensus["blocks"]
    if not blocks:
        raise Malformed(f"{label}: consensus.blocks is empty; an empty recording is not evidence")
    previous_height = 0
    for index, block in enumerate(blocks):
        block_label = f"{label}: consensus.blocks[{index}]"
        _require_fields(block, BLOCK_FIELDS, block_label)
        if block["height"] != previous_height + 1:
            raise Malformed(
                f"{block_label}.height is {block['height']}, expected {previous_height + 1}"
            )
        previous_height = block["height"]
        _require_hash(block["app_hash"], f"{block_label}.app_hash")
        for position, transition in enumerate(block["transitions"]):
            _require_fields(transition, TRANSITION_FIELDS, f"{block_label}.transitions[{position}]")
    if blocks[-1]["app_hash"] != consensus["final_root_hash"]:
        raise Malformed(
            f"{label}: consensus.final_root_hash does not equal the last block's app_hash"
        )
    if artifact["diagnostic"]["block_count"] != len(blocks):
        raise Malformed(
            f"{label}: diagnostic.block_count is {artifact['diagnostic']['block_count']} "
            f"but {len(blocks)} blocks are recorded"
        )


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True)


def first_consensus_difference(left, right):
    """Name the first field that differs, block by block, or None."""
    left_blocks = left["blocks"]
    right_blocks = right["blocks"]
    for index, (lb, rb) in enumerate(zip(left_blocks, right_blocks)):
        height = lb.get("height", index + 1)
        if lb.get("height") != rb.get("height"):
            return f"block order differs at index {index}: heights {lb.get('height')} and {rb.get('height')}"
        for key in ("protocol_version", "app_hash"):
            if lb.get(key) != rb.get(key):
                return f"block {height}: {key} {lb.get(key)!r} != {rb.get(key)!r}"
        lt = lb.get("transitions", [])
        rt = rb.get("transitions", [])
        if len(lt) != len(rt):
            return f"block {height}: {len(lt)} transitions kept versus {len(rt)}"
        for position, (ltx, rtx) in enumerate(zip(lt, rt)):
            for key in ("name", "code", "fee"):
                if ltx.get(key) != rtx.get(key):
                    return (
                        f"block {height}, transition {position} ({ltx.get('name')}): "
                        f"{key} {ltx.get(key)!r} != {rtx.get(key)!r}"
                    )
    if len(left_blocks) != len(right_blocks):
        return f"{len(left_blocks)} blocks recorded versus {len(right_blocks)}"
    for key in sorted(set(left) | set(right)):
        if key == "blocks":
            continue
        if canonical(left.get(key)) != canonical(right.get(key)):
            return f"consensus.{key}: {canonical(left.get(key))} != {canonical(right.get(key))}"
    if canonical(left) != canonical(right):
        return "consensus sections differ in a field this comparator does not name"
    return None


def compare(left, right, allow_same_architecture=False, out=sys.stdout):
    """Compare two validated artifacts. Returns (exit_code, report_lines).

    Raises `Malformed` if either artifact fails `validate_shape`.
    """
    validate_shape(left, "left")
    validate_shape(right, "right")
    lines = []
    failures = []
    lp, rp = left["profile"], right["profile"]
    ld, rd = left["diagnostic"], right["diagnostic"]

    lines.append("| field | left | right |")
    lines.append("|---|---|---|")
    for key in sorted(set(lp) | set(rp)):
        lines.append(f"| profile.{key} | `{canonical(lp.get(key))}` | `{canonical(rp.get(key))}` |")
    for key in sorted(set(ld) | set(rd)):
        lines.append(f"| diagnostic.{key} | `{canonical(ld.get(key))}` | `{canonical(rd.get(key))}` |")

    if lp["target_arch"] == rp["target_arch"] and not allow_same_architecture:
        failures.append(
            f"both artifacts were recorded on {lp['target_arch']!r}; a cross-architecture "
            "comparison needs two different targets (pass --allow-same-architecture for a "
            "local two-process check)"
        )
    for key in ("protocol_version_start", "protocol_version_end"):
        if lp[key] != rp[key]:
            failures.append(
                f"profile.{key} differs ({lp[key]} versus {rp[key]}): the recordings ran "
                "under different protocol profiles and are not comparable"
            )

    if not failures:
        difference = first_consensus_difference(left["consensus"], right["consensus"])
        if difference is not None:
            failures.append(f"consensus mismatch: {difference}")

    for key in sorted(set(ld) | set(rd)):
        if canonical(ld.get(key)) != canonical(rd.get(key)):
            lines.append(f"note: diagnostic.{key} differs; diagnostics are recorded, not compared")

    if failures:
        for failure in failures:
            lines.append(f"REJECT: {failure}")
        code = EXIT_MISMATCH
    else:
        lines.append(
            f"MATCH: consensus sections are identical across {lp['target_arch']} and "
            f"{rp['target_arch']} ({len(left['consensus']['blocks'])} blocks, protocol "
            f"{lp['protocol_version_start']} to {lp['protocol_version_end']})"
        )
        code = EXIT_MATCH
    for line in lines:
        print(line, file=out)
    return code, lines


def sample_artifact(arch="x86_64"):
    return {
        "schema": 1,
        "profile": {
            "target_arch": arch,
            "target_os": "linux",
            "pointer_width": 64,
            "endian": "little",
            "cpu_features": ["sse4.2"] if arch == "x86_64" else ["neon"],
            "protocol_version_start": 13,
            "protocol_version_end": 14,
            "engine": None,
        },
        "consensus": {
            "workload_seed": 7,
            "reopened_at_height": 70,
            "blocks": [
                {
                    "height": 1,
                    "protocol_version": 13,
                    "app_hash": "01" * 32,
                    "transitions": [{"name": "IdentityCreate", "code": 0, "fee": 1000}],
                },
                {
                    "height": 2,
                    "protocol_version": 14,
                    "app_hash": "02" * 32,
                    "transitions": [],
                },
            ],
            "final_root_hash": "02" * 32,
            "total_credits": {
                "total_credits_in_platform": 10,
                "total_in_pools": 1,
                "total_identity_balances": 9,
                "total_specialized_balances": 0,
                "total_in_addresses": 0,
                "total_in_shielded_balances": 0,
            },
            "identity_balances": {"09" * 32: 9},
        },
        "diagnostic": {"elapsed_ms": 1, "block_count": 2, "engine_fuel": None},
    }


class _Sink:
    def write(self, _text):
        return None

    def flush(self):
        return None


def self_test():
    """Prove the comparator accepts an identical pair and rejects each
    consensus-visible difference. Returns the number of failed checks."""
    sink = _Sink()
    checks = []

    def expect(name, code, expected, lines=None, needle=None):
        ok = code == expected
        if ok and needle is not None:
            ok = any(needle in line for line in lines)
        checks.append((name, ok))
        status = "ok  " if ok else "FAIL"
        print(f"self-test {status} {name} (exit {code}, expected {expected})")

    left = sample_artifact("x86_64")
    right = sample_artifact("aarch64")

    code, lines = compare(left, right, out=sink)
    expect("identical consensus on two architectures matches", code, EXIT_MATCH, lines, "MATCH")

    mutated = copy.deepcopy(right)
    mutated["consensus"]["blocks"][0]["app_hash"] = "03" * 32
    code, lines = compare(left, mutated, out=sink)
    expect("one differing app hash is rejected", code, EXIT_MISMATCH, lines, "block 1: app_hash")

    mutated = copy.deepcopy(right)
    mutated["consensus"]["blocks"][1]["app_hash"] = "03" * 32
    mutated["consensus"]["final_root_hash"] = "03" * 32
    code, lines = compare(left, mutated, out=sink)
    expect("a differing final root is rejected", code, EXIT_MISMATCH, lines, "block 2: app_hash")

    mutated = copy.deepcopy(right)
    mutated["consensus"]["blocks"][0]["transitions"][0]["fee"] += 1
    code, lines = compare(left, mutated, out=sink)
    expect("one differing transition fee is rejected", code, EXIT_MISMATCH, lines, "fee 1000 != 1001")

    mutated = copy.deepcopy(right)
    mutated["consensus"]["blocks"][0]["transitions"][0]["code"] = 10000
    code, lines = compare(left, mutated, out=sink)
    expect("one differing transition code is rejected", code, EXIT_MISMATCH, lines, "code 0 != 10000")

    mutated = copy.deepcopy(right)
    mutated["consensus"]["blocks"][1]["protocol_version"] = 13
    code, lines = compare(left, mutated, out=sink)
    expect("a differing per-block protocol version is rejected", code, EXIT_MISMATCH, lines, "protocol_version 14 != 13")

    mutated = copy.deepcopy(right)
    mutated["consensus"]["total_credits"]["total_in_pools"] += 1
    code, lines = compare(left, mutated, out=sink)
    expect("a differing credit total is rejected", code, EXIT_MISMATCH, lines, "consensus.total_credits")

    mutated = copy.deepcopy(right)
    mutated["consensus"]["identity_balances"]["09" * 32] -= 1
    code, lines = compare(left, mutated, out=sink)
    expect("a differing identity balance is rejected", code, EXIT_MISMATCH, lines, "consensus.identity_balances")

    mutated = copy.deepcopy(right)
    mutated["consensus"]["blocks"].pop()
    mutated["consensus"]["final_root_hash"] = mutated["consensus"]["blocks"][-1]["app_hash"]
    mutated["diagnostic"]["block_count"] = 1
    code, lines = compare(left, mutated, out=sink)
    expect("a missing block is rejected", code, EXIT_MISMATCH, lines, "2 blocks recorded versus 1")

    mutated = copy.deepcopy(right)
    mutated["diagnostic"]["elapsed_ms"] = 999_999
    code, lines = compare(left, mutated, out=sink)
    expect("a differing diagnostic timing still matches, with a note", code, EXIT_MATCH, lines, "note: diagnostic.elapsed_ms differs")

    mutated = copy.deepcopy(right)
    mutated["profile"]["cpu_features"] = ["neon", "sve"]
    code, lines = compare(left, mutated, out=sink)
    expect("differing recorded CPU features still match", code, EXIT_MATCH, lines, "MATCH")

    code, lines = compare(left, copy.deepcopy(left), out=sink)
    expect("the same architecture twice is rejected without the flag", code, EXIT_MISMATCH, lines, "both artifacts were recorded on")

    code, lines = compare(left, copy.deepcopy(left), allow_same_architecture=True, out=sink)
    expect("the same architecture twice matches with the flag", code, EXIT_MATCH, lines, "MATCH")

    mutated = copy.deepcopy(right)
    mutated["profile"]["protocol_version_end"] = 15
    code, lines = compare(left, mutated, out=sink)
    expect("differing protocol profiles are rejected", code, EXIT_MISMATCH, lines, "profile.protocol_version_end differs")

    def malformed(name, mutate):
        mutated = copy.deepcopy(right)
        mutate(mutated)
        try:
            compare(left, mutated, out=sink)
            code = EXIT_MATCH
        except Malformed:
            code = EXIT_MALFORMED
        expect(name, code, EXIT_MALFORMED)

    def drop_fees(artifact):
        for block in artifact["consensus"]["blocks"]:
            for transition in block["transitions"]:
                del transition["fee"]

    def null_block(artifact):
        artifact["consensus"]["blocks"][0] = None

    def empty_blocks(artifact):
        artifact["consensus"]["blocks"] = []
        artifact["diagnostic"]["block_count"] = 0

    def drop_total_credits_field(artifact):
        del artifact["consensus"]["total_credits"]["total_in_pools"]

    def bad_hash(artifact):
        artifact["consensus"]["blocks"][0]["app_hash"] = "not hex"

    def gap_in_heights(artifact):
        artifact["consensus"]["blocks"][1]["height"] = 3

    def stale_final_root(artifact):
        artifact["consensus"]["final_root_hash"] = "0f" * 32

    malformed("an unknown schema is malformed", lambda a: a.__setitem__("schema", 99))
    for value in ([], {}, True, 1.0, "1", None):
        malformed(
            f"a schema of {value!r} is malformed, not compared",
            lambda a, value=value: a.__setitem__("schema", value),
        )
    malformed("a missing section is malformed", lambda a: a.__delitem__("diagnostic"))
    malformed("a missing transition fee is malformed", drop_fees)
    malformed("a null block is malformed", null_block)
    malformed("an empty recording is malformed", empty_blocks)
    malformed("a missing credit total field is malformed", drop_total_credits_field)
    malformed("a non-hex application hash is malformed", bad_hash)
    malformed("a gap in block heights is malformed", gap_in_heights)
    malformed("a final root that is not the last app hash is malformed", stale_final_root)

    # Both sides lacking the same field must still be malformed, not a match on nothing.
    both = copy.deepcopy(left)
    drop_fees(both)
    other = copy.deepcopy(right)
    drop_fees(other)
    try:
        compare(both, other, out=sink)
        code = EXIT_MATCH
    except Malformed:
        code = EXIT_MALFORMED
    expect("two recordings both missing fees are malformed, not a match", code, EXIT_MALFORMED)

    failed = [name for name, ok in checks if not ok]
    print(f"self-test: {len(checks) - len(failed)} of {len(checks)} checks passed")
    return len(failed)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("left", nargs="?", help="artifact recorded on the first architecture")
    parser.add_argument("right", nargs="?", help="artifact recorded on the second architecture")
    parser.add_argument(
        "--allow-same-architecture",
        action="store_true",
        help="accept two artifacts from one target (local two-process check)",
    )
    parser.add_argument(
        "--self-test",
        action="store_true",
        help="exercise the comparator on in-memory artifacts and exit",
    )
    args = parser.parse_args(argv)

    if args.self_test:
        return EXIT_MISMATCH if self_test() else EXIT_MATCH

    if not args.left or not args.right:
        parser.error("two artifact paths are required unless --self-test is given")

    try:
        left = load(args.left)
        right = load(args.right)
    except Malformed as error:
        print(f"MALFORMED: {error}", file=sys.stderr)
        return EXIT_MALFORMED

    code, _lines = compare(left, right, allow_same_architecture=args.allow_same_architecture)
    return code


if __name__ == "__main__":
    sys.exit(main())
