#!/usr/bin/env python3
import argparse
import os
import re
import sys
from typing import Optional
import subprocess


DESC = """
dash_core_version_switcher.py: switch all Cargo.toml dashcore deps between local path and git (rev/branch).

Usage:
  dash_core_version_switcher.py local
  dash_core_version_switcher.py rev <rev>
  dash_core_version_switcher.py branch <branch>

This edits inline-table or simple dependencies like:
  dashcore = { path = "../../../rust-dashcore/dash", features = [ ... ], default-features = false }
  dashcore = { git = "https://github.com/dashpay/rust-dashcore", rev = "<rev>", features = [ ... ], default-features = false }
  dashcore = "0.40"

It preserves existing features/default-features and only switches path/git+rev/branch or version key.
Commented lines are not modified.
"""


GIT_URL = "https://github.com/dashpay/rust-dashcore"

# Dependency names we switch and their local paths
DEP_LOCAL_PATHS = {
    "dashcore": "../../../rust-dashcore/dash",
    "key-wallet": "../../../rust-dashcore/key-wallet",
    "key-wallet-manager": "../../../rust-dashcore/key-wallet-manager",
    "dash-spv": "../../../rust-dashcore/dash-spv",
    "dashcore-rpc": "../../../rust-dashcore/rpc-client",
    "key-wallet-ffi": "../../../rust-dashcore/key-wallet-ffi",
    "dash-spv-ffi": "../../../rust-dashcore/dash-spv-ffi",
}


def find_cargo_tomls(root: str):
    for dirpath, dirnames, filenames in os.walk(root):
        # skip typical build dirs
        skip = any(part in dirpath for part in ("/target/", "/.git/", "/node_modules/", "/.build/"))
        if skip:
            continue
        if "Cargo.toml" in filenames:
            yield os.path.join(dirpath, "Cargo.toml")


def iter_dep_blocks(text: str):
    dep_names = "|".join(map(re.escape, DEP_LOCAL_PATHS.keys()))
    # Inline tables
    pattern_inline = re.compile(rf"(^|\n)(?P<indent>\s*)(?P<name>{dep_names})\s*=\s*\{{[^}}]*\}}", re.S)
    for m in pattern_inline.finditer(text):
        block_start = m.start() + (0 if text[m.start()] != '\n' else 1)
        block_end = m.end()
        # Skip commented lines
        line_start = text.rfind('\n', 0, block_start) + 1
        line_end = text.find('\n', line_start)
        if line_end == -1:
            line_end = len(text)
        if text[line_start:line_end].lstrip().startswith('#'):
            continue
        dep_name = m.group('name')
        yield (block_start, block_end, dep_name, 'inline')

    # Simple string dependencies: name = "x.y.z"
    pattern_simple = re.compile(rf"(^|\n)(?P<indent>\s*)(?P<name>{dep_names})\s*=\s*\"[^\"]*\"", re.S)
    for m in pattern_simple.finditer(text):
        block_start = m.start() + (0 if text[m.start()] != '\n' else 1)
        block_end = m.end()
        line_start = text.rfind('\n', 0, block_start) + 1
        line_end = text.find('\n', line_start)
        if line_end == -1:
            line_end = len(text)
        if text[line_start:line_end].lstrip().startswith('#'):
            continue
        dep_name = m.group('name')
        yield (block_start, block_end, dep_name, 'simple')


def parse_inline_table(s: str):
    brace_open = s.find('{')
    brace_close = s.rfind('}')
    inner = s[brace_open + 1:brace_close]
    parts = []
    buf = []
    depth = 0
    for ch in inner:
        if ch == '[':
            depth += 1
        elif ch == ']':
            depth -= 1
        if ch == ',' and depth == 0:
            parts.append(''.join(buf).strip())
            buf = []
        else:
            buf.append(ch)
    if buf:
        parts.append(''.join(buf).strip())
    kv = []
    for p in parts:
        if not p or '=' not in p:
            continue
        k, v = p.split('=', 1)
        kv.append((k.strip(), v.strip()))
    return kv


def serialize_inline_table(prefix: str, pairs):
    body = ', '.join(f"{k} = {v}" for k, v in pairs)
    return f"{prefix}{{ {body} }}"


def get_default_branch(remote_url: str) -> str:
    try:
        out = subprocess.check_output(["git", "ls-remote", "--symref", remote_url, "HEAD"], text=True)
        for line in out.splitlines():
            line = line.strip()
            if line.startswith("ref:") and "refs/heads/" in line:
                ref = line.split()[1]
                return ref.split("/")[-1]
        raise RuntimeError(f"Could not determine default branch from: {out}")
    except subprocess.CalledProcessError as e:
        raise RuntimeError(f"git ls-remote failed: {e}")


def get_branch_head_sha(remote_url: str, branch: str) -> str:
    try:
        ref = f"refs/heads/{branch}"
        out = subprocess.check_output(["git", "ls-remote", remote_url, ref], text=True)
        sha = out.strip().split()[0]
        if not sha:
            raise RuntimeError(f"Unexpected ls-remote output: {out}")
        return sha
    except subprocess.CalledProcessError as e:
        raise RuntimeError(f"git ls-remote failed: {e}")


def switch_dep(block_text: str, dep_name: str, mode: str, value: Optional[str]):
    if '{' in block_text:
        prefix = block_text[:block_text.find('{')]
        pairs = parse_inline_table(block_text)
        keys = [k for k, _ in pairs]
        d = {k: v for k, v in pairs}

        for k in ("git", "rev", "branch", "path", "version"):
            if k in d:
                del d[k]
                if k in keys:
                    keys.remove(k)

        if mode == 'local':
            keys.insert(0, 'path')
            d['path'] = f'"{DEP_LOCAL_PATHS[dep_name]}"'
        elif mode == 'rev':
            keys.insert(0, 'git')
            d['git'] = f'"{GIT_URL}"'
            keys.insert(1, 'rev')
            d['rev'] = f'"{value}"'
        elif mode == 'branch':
            keys.insert(0, 'git')
            d['git'] = f'"{GIT_URL}"'
            keys.insert(1, 'branch')
            d['branch'] = f'"{value}"'
        else:
            raise RuntimeError(f"Unknown mode {mode}")

        ordered_pairs = []
        for k in keys:
            if k in d:
                ordered_pairs.append((k, d[k]))
        for k, v in d.items():
            if k not in keys:
                ordered_pairs.append((k, v))

        return serialize_inline_table(prefix, ordered_pairs)
    else:
        # simple: name = "x.y.z" -> upgrade to inline form on switches
        name, _, _ = block_text.partition('=')
        name_prefix = name + '= '
        if mode == 'local':
            body = f'{{ path = "{DEP_LOCAL_PATHS[dep_name]}" }}'
        elif mode == 'rev':
            body = f'{{ git = "{GIT_URL}", rev = "{value}" }}'
        elif mode == 'branch':
            body = f'{{ git = "{GIT_URL}", branch = "{value}" }}'
        else:
            raise RuntimeError(f"Unknown mode {mode}")
        return name_prefix + body


def process_file(path: str, mode: str, value: Optional[str]) -> bool:
    with open(path, 'r', encoding='utf-8') as f:
        text = f.read()

    blocks = list(iter_dep_blocks(text))
    if not blocks:
        return False

    changed = False
    for start, end, dep_name, _kind in reversed(blocks):
        block_text = text[start:end]
        new_block = switch_dep(block_text, dep_name, mode, value)
        if new_block != block_text:
            text = text[:start] + new_block + text[end:]
            changed = True

    if changed:
        with open(path, 'w', encoding='utf-8', newline='\n') as f:
            f.write(text)
    return changed


def main():
    parser = argparse.ArgumentParser(description=DESC)
    sub = parser.add_subparsers(dest='cmd', required=True)
    sub.add_parser('local')
    p_rev = sub.add_parser('rev')
    p_rev.add_argument('rev')
    p_branch = sub.add_parser('branch')
    p_branch.add_argument('branch')
    sub.add_parser('main_branch_latest')
    args = parser.parse_args()

    mode = args.cmd
    val = None
    resolved = None
    if mode == 'rev':
        val = args.rev
    elif mode == 'branch':
        val = args.branch
    elif mode == 'main_branch_latest':
        branch = get_default_branch(GIT_URL)
        sha = get_branch_head_sha(GIT_URL, branch)
        mode = 'rev'
        val = sha
        resolved = (branch, sha)

    repo_root = os.getcwd()
    edited = []
    for cargo in find_cargo_tomls(repo_root):
        if process_file(cargo, mode, val):
            edited.append(cargo)

    if edited:
        print(f"Updated rust-dashcore dependencies in {len(edited)} file(s):")
        for p in edited:
            print(f" - {os.path.relpath(p, repo_root)}")
        if resolved:
            print(f"Resolved default branch '{resolved[0]}' at {resolved[1]}")
    else:
        print("No Cargo.toml files with dashcore dependency found to update.")


if __name__ == '__main__':
    try:
        main()
    except KeyboardInterrupt:
        sys.exit(130)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)
