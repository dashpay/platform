#!/usr/bin/env bash
# Post-create setup for Dash Platform devcontainer with Claude Code.
# Runs once after the container is created.
set -euo pipefail

WORKSPACE="/workspace/platform"

echo "=== Dash Platform devcontainer post-create setup ==="

# --- Git worktree support ---
# Git worktrees use a .git FILE pointing to an absolute host path like
# /Users/you/.../platform/.git/worktrees/v3. That path doesn't exist inside
# the container. init-host.sh mounted the main .git at /workspace/.host-main-git.
# We create a symlink from the absolute host path so git can follow it.
if [ -f "$WORKSPACE/.git" ] && [ -d "/workspace/.host-main-git" ]; then
    GITDIR=$(sed 's/gitdir: //' "$WORKSPACE/.git")
    if [ ! -d "$GITDIR" ]; then
        # e.g. /Users/you/Projects/dashpay/platform/.git
        MAIN_GIT_HOST_PATH="$(dirname "$(dirname "$GITDIR")")"
        sudo mkdir -p "$(dirname "$MAIN_GIT_HOST_PATH")"
        sudo ln -sfn /workspace/.host-main-git "$MAIN_GIT_HOST_PATH"
        echo "Git worktree: linked $MAIN_GIT_HOST_PATH -> /workspace/.host-main-git"
    fi
fi

# --- Git configuration ---
git config --global --add safe.directory "$WORKSPACE"

# --- Cargo permissions ---
sudo chown -R vscode:vscode /home/vscode/.cargo "$WORKSPACE/target" 2>/dev/null || true

# --- Enable corepack for yarn ---
corepack enable 2>/dev/null || true

# --- Claude Code: copy config staged by init-host.sh, then override for sandbox ---
# init-host.sh copies ~/.claude into .devcontainer/.claude-host-config/ on the host.
# The workspace bind mount makes it available here. We copy into the persistent
# volume, then clean up the staging copy (it contains credentials).
CLAUDE_DIR="${CLAUDE_CONFIG_DIR:-/home/vscode/.claude}"
HOST_CONFIG="$WORKSPACE/.devcontainer/.claude-host-config"

if [ -d "$HOST_CONFIG" ] && [ "$(ls -A "$HOST_CONFIG" 2>/dev/null)" ]; then
    echo "Copying Claude config from host..."
    cp -a "$HOST_CONFIG"/. "$CLAUDE_DIR"/ 2>/dev/null || true
    for item in "$HOST_CONFIG"/.*; do
        basename="$(basename "$item")"
        [ "$basename" = "." ] || [ "$basename" = ".." ] && continue
        cp -a "$item" "$CLAUDE_DIR/$basename" 2>/dev/null || true
    done
    chmod 600 "$CLAUDE_DIR/.credentials.json" 2>/dev/null || true
    # Place ~/.claude.json (onboarding state) at the correct path
    if [ -f "$HOST_CONFIG/.claude.json.root" ]; then
        cp -a "$HOST_CONFIG/.claude.json.root" /home/vscode/.claude.json
        chown vscode:vscode /home/vscode/.claude.json
    fi
    # Clean up staged credentials from the workspace
    rm -rf "$HOST_CONFIG"
    echo "Host Claude config copied."
else
    mkdir -p "$CLAUDE_DIR"
    echo "No host Claude config found. Use ANTHROPIC_API_KEY or 'claude login'."
fi

chown -R vscode:vscode "$CLAUDE_DIR"

# Force bypassPermissions on top of whatever settings came from host
SETTINGS_FILE="$CLAUDE_DIR/settings.json"
if [ -f "$SETTINGS_FILE" ]; then
    TMP=$(mktemp)
    jq '.permissions.defaultMode = "bypassPermissions" | .skipDangerousModePermissionPrompt = true' \
        "$SETTINGS_FILE" > "$TMP" 2>/dev/null && mv "$TMP" "$SETTINGS_FILE" || \
        echo '{"permissions":{"defaultMode":"bypassPermissions"},"skipDangerousModePermissionPrompt":true}' > "$SETTINGS_FILE"
else
    echo '{"permissions":{"defaultMode":"bypassPermissions"},"skipDangerousModePermissionPrompt":true}' > "$SETTINGS_FILE"
fi
chown vscode:vscode "$SETTINGS_FILE"

# --- Shell history (idempotent) ---
grep -q 'HISTFILE=/commandhistory/.zsh_history' /home/vscode/.zshrc 2>/dev/null || \
    echo 'export HISTFILE=/commandhistory/.zsh_history' >> /home/vscode/.zshrc
grep -q 'HISTFILE=/commandhistory/.bash_history' /home/vscode/.bashrc 2>/dev/null || \
    echo 'export HISTFILE=/commandhistory/.bash_history' >> /home/vscode/.bashrc

echo "=== Post-create setup complete ==="
echo "Claude Code is configured with bypassPermissions mode."
echo "Set ANTHROPIC_API_KEY in your host environment before opening this devcontainer."
