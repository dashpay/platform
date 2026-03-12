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

# --- Claude Code: copy config staged by init-host.sh ---
# init-host.sh stages credentials, plugin list, and optionally agents/skills.
# We copy into the persistent volume, create a minimal settings.json with
# plugins merged in, then clean up the staging copy.
CLAUDE_DIR="${CLAUDE_CONFIG_DIR:-/home/vscode/.claude}"
HOST_CONFIG="$WORKSPACE/.devcontainer/.claude-host-config"

mkdir -p "$CLAUDE_DIR"

if [ -d "$HOST_CONFIG" ] && [ "$(ls -A "$HOST_CONFIG" 2>/dev/null)" ]; then
    echo "Copying Claude config from host..."
    # OAuth credentials
    if [ -f "$HOST_CONFIG/.credentials.json" ]; then
        cp -a "$HOST_CONFIG/.credentials.json" "$CLAUDE_DIR/.credentials.json"
        chmod 600 "$CLAUDE_DIR/.credentials.json"
    fi
    echo "Host Claude credentials copied."
else
    echo "No host Claude credentials found. Use ANTHROPIC_API_KEY or 'claude login'."
fi

# Write a clean settings.json with bypassPermissions (no host settings leak)
SETTINGS_FILE="$CLAUDE_DIR/settings.json"
cat > "$SETTINGS_FILE" <<'SETTINGS'
{
  "permissions": {
    "defaultMode": "bypassPermissions"
  },
  "skipDangerousModePermissionPrompt": true
}
SETTINGS

# Merge host's enabledPlugins into settings (plugin IDs only, no secrets)
if [ -f "$HOST_CONFIG/enabled-plugins.json" ]; then
    TMP=$(mktemp)
    jq -s '.[0] * .[1]' "$SETTINGS_FILE" "$HOST_CONFIG/enabled-plugins.json" \
        > "$TMP" 2>/dev/null && mv "$TMP" "$SETTINGS_FILE" || true
fi

# Copy host agent definitions
if [ -d "$HOST_CONFIG/agents" ] && [ "$(ls -A "$HOST_CONFIG/agents" 2>/dev/null)" ]; then
    mkdir -p "$CLAUDE_DIR/agents"
    cp -a "$HOST_CONFIG/agents/"* "$CLAUDE_DIR/agents/"
    echo "Host Claude agents copied."
fi

# Copy host skill definitions
if [ -d "$HOST_CONFIG/skills" ] && [ "$(ls -A "$HOST_CONFIG/skills" 2>/dev/null)" ]; then
    mkdir -p "$CLAUDE_DIR/skills"
    cp -a "$HOST_CONFIG/skills/"* "$CLAUDE_DIR/skills/"
    echo "Host Claude skills copied."
fi

chown -R vscode:vscode "$CLAUDE_DIR"

# Clean up staged config from the workspace
rm -rf "$HOST_CONFIG"

echo "=== Post-create setup complete ==="
echo "Claude Code is configured with bypassPermissions mode."
echo "Set ANTHROPIC_API_KEY in your host environment before opening this devcontainer."
