#!/bin/bash
# Runs on the HOST before container creation.
# Resolves git worktree paths so git works inside the container.
set -euo pipefail

test -f "$HOME/.gitconfig" || touch "$HOME/.gitconfig"
mkdir -p "$HOME/.claude"

# Stage ONLY the minimum Claude config needed for authentication.
# We deliberately skip: conversation history, project memories, debug logs,
# shell snapshots, plans, scripts, plugins cache, and other data that could
# leak information from other projects into the sandboxed container.
CLAUDE_STAGING=".devcontainer/.claude-host-config"
rm -rf "$CLAUDE_STAGING"
if [ -d "$HOME/.claude" ]; then
    mkdir -p "$CLAUDE_STAGING"
    # Credentials (OAuth tokens) — required for authentication
    [ -f "$HOME/.claude/.credentials.json" ] && \
        cp -a "$HOME/.claude/.credentials.json" "$CLAUDE_STAGING/.credentials.json" 2>/dev/null || true
    # Onboarding state — prevents setup wizard
    [ -f "$HOME/.claude.json" ] && \
        cp -a "$HOME/.claude.json" "$CLAUDE_STAGING/.claude.json.root" 2>/dev/null || true

    # Plugins: always copy (just IDs, no secrets)
    if [ -f "$HOME/.claude/settings.json" ]; then
        jq '{enabledPlugins: .enabledPlugins}' "$HOME/.claude/settings.json" \
            > "$CLAUDE_STAGING/enabled-plugins.json" 2>/dev/null || true
    fi

    # Agents & skills: copy only what the user listed in .env
    CLAUDE_AGENTS=""
    CLAUDE_SKILLS=""
    ENV_FILE=".devcontainer/.env"
    [ -f "$ENV_FILE" ] && source "$ENV_FILE"

    if [ -n "$CLAUDE_AGENTS" ]; then
        mkdir -p "$CLAUDE_STAGING/agents"
        IFS=',' read -ra AGENT_LIST <<< "$CLAUDE_AGENTS"
        for agent in "${AGENT_LIST[@]}"; do
            agent=$(echo "$agent" | xargs)
            src="$HOME/.claude/agents/${agent}.md"
            [ -f "$src" ] && cp -a "$src" "$CLAUDE_STAGING/agents/" 2>/dev/null || true
        done
    fi

    if [ -n "$CLAUDE_SKILLS" ]; then
        mkdir -p "$CLAUDE_STAGING/skills"
        IFS=',' read -ra SKILL_LIST <<< "$CLAUDE_SKILLS"
        for skill in "${SKILL_LIST[@]}"; do
            skill=$(echo "$skill" | xargs)
            src="$HOME/.claude/skills/${skill}"
            [ -d "$src" ] && cp -a "$src" "$CLAUDE_STAGING/skills/" 2>/dev/null || true
        done
    fi
fi

# Resolve main .git directory for worktree support.
# Docker follows symlinks in bind mount sources, so we create a symlink
# at a known path that always points to the real .git directory.
# This way the same devcontainer.json works for both worktrees and main repo.
RESOLVED=".devcontainer/.main-git-resolved"
rm -rf "$RESOLVED"

if [ -f .git ]; then
    # Worktree: .git file contains "gitdir: /path/to/main/.git/worktrees/name"
    GITDIR=$(sed 's/gitdir: //' .git)
    # Strip /worktrees/name to get the main .git directory
    MAIN_GIT="${GITDIR%/worktrees/*}"
    if [ -d "$MAIN_GIT" ]; then
        ln -s "$MAIN_GIT" "$RESOLVED"
    else
        mkdir -p "$RESOLVED"
    fi
elif [ -d .git ]; then
    # Main repo: just point to our own .git
    ln -s "$(pwd)/.git" "$RESOLVED"
else
    # No git at all — empty dir so the mount doesn't fail
    mkdir -p "$RESOLVED"
fi
