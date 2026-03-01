#!/bin/bash
# Runs on the HOST before container creation.
# Resolves git worktree paths so git works inside the container.
set -euo pipefail

test -f "$HOME/.gitconfig" || touch "$HOME/.gitconfig"
mkdir -p "$HOME/.claude"

# Copy Claude config into the workspace so post-create.sh can access it.
# Bind mounts of ~/.claude fail on some Docker Desktop setups (virtiofs issues).
# The workspace mount is reliable, so we copy here and clean up in post-create.
CLAUDE_STAGING=".devcontainer/.claude-host-config"
rm -rf "$CLAUDE_STAGING"
if [ -d "$HOME/.claude" ] && [ "$(ls -A "$HOME/.claude" 2>/dev/null)" ]; then
    mkdir -p "$CLAUDE_STAGING"
    cp -a "$HOME/.claude"/. "$CLAUDE_STAGING"/ 2>/dev/null || true
    # Also copy dotfiles
    for item in "$HOME/.claude"/.*; do
        base="$(basename "$item")"
        [ "$base" = "." ] || [ "$base" = ".." ] && continue
        cp -a "$item" "$CLAUDE_STAGING/$base" 2>/dev/null || true
    done
    # Also copy ~/.claude.json (onboarding state, outside ~/.claude/)
    [ -f "$HOME/.claude.json" ] && cp -a "$HOME/.claude.json" "$CLAUDE_STAGING/.claude.json.root" 2>/dev/null || true
fi

# Resolve main .git directory for worktree support.
# Docker follows symlinks in bind mount sources, so we create a symlink
# at a known path that always points to the real .git directory.
# This way the same devcontainer.json works for both worktrees and main repo.
RESOLVED=".devcontainer/.main-git-resolved"

if [ -f .git ]; then
    # Worktree: .git file contains "gitdir: /path/to/main/.git/worktrees/name"
    GITDIR=$(sed 's/gitdir: //' .git)
    # Strip /worktrees/name to get the main .git directory
    MAIN_GIT="${GITDIR%/worktrees/*}"
    if [ -d "$MAIN_GIT" ]; then
        ln -sfn "$MAIN_GIT" "$RESOLVED"
    else
        mkdir -p "$RESOLVED"
    fi
elif [ -d .git ]; then
    # Main repo: just point to our own .git
    ln -sfn "$(pwd)/.git" "$RESOLVED"
else
    # No git at all — empty dir so the mount doesn't fail
    mkdir -p "$RESOLVED"
fi
