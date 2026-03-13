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

# Configure git to use HTTPS with GH_TOKEN/GITHUB_TOKEN if available.
# This allows git push/pull and gh CLI to work without SSH keys.
if [ -n "${GH_TOKEN:-}" ] || [ -n "${GITHUB_TOKEN:-}" ]; then
    gh auth setup-git 2>/dev/null || true
    echo "GitHub auth configured via GH_TOKEN (HTTPS git operations enabled)."
fi

# --- Cargo permissions ---
sudo chown -R vscode:vscode /home/vscode/.cargo "$WORKSPACE/target" 2>/dev/null || true

# --- Enable corepack for yarn ---
corepack enable 2>/dev/null || true

# --- Claude Code: copy config staged by init-host.sh ---
# Credentials are always copied. Agents and skills are copied only if listed
# in .devcontainer/.env (CLAUDE_AGENTS, CLAUDE_SKILLS). Plugins are not copied
# automatically — use the project's .claude/settings.json instead.
CLAUDE_DIR="${CLAUDE_CONFIG_DIR:-/home/vscode/.claude}"
HOST_CONFIG="$WORKSPACE/.devcontainer/.claude-host-config"

mkdir -p "$CLAUDE_DIR"

if [ -f "$HOST_CONFIG/.credentials.json" ]; then
    echo "Copying Claude credentials from host..."
    cp -a "$HOST_CONFIG/.credentials.json" "$CLAUDE_DIR/.credentials.json"
    chmod 600 "$CLAUDE_DIR/.credentials.json"
    echo "Host Claude credentials copied."
else
    echo "No host Claude credentials found. Use ANTHROPIC_API_KEY or 'claude login'."
fi

# Write a clean settings.json with bypassPermissions
cat > "$CLAUDE_DIR/settings.json" <<'SETTINGS'
{
  "permissions": {
    "defaultMode": "bypassPermissions"
  },
  "skipDangerousModePermissionPrompt": true
}
SETTINGS

# Copy host agent definitions (opt-in via CLAUDE_AGENTS in .env)
if [ -d "$HOST_CONFIG/agents" ] && [ "$(ls -A "$HOST_CONFIG/agents" 2>/dev/null)" ]; then
    mkdir -p "$CLAUDE_DIR/agents"
    cp -a "$HOST_CONFIG/agents/"* "$CLAUDE_DIR/agents/"
    echo "Host Claude agents copied."
fi

# Copy host skill definitions (opt-in via CLAUDE_SKILLS in .env)
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
