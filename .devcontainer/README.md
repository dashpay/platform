# Dev Container

Sandboxed development environment for Dash Platform with Claude Code pre-configured for autonomous work.

## What's Included

- **Rust 1.92** with `wasm32-unknown-unknown` target
- **Node.js 24** with yarn 4.12.0 (via corepack)
- **Docker-in-Docker** for dashmate
- **Claude Code** with `bypassPermissions` mode
- protoc 32.0, wasm-bindgen-cli 0.2.108, wasm-pack, cargo-binstall
- Developer tools: git-delta, ripgrep, fd, fzf

## Prerequisites

### SSH keys (for git push/pull)

VS Code forwards your host's SSH agent into the container automatically. Make sure your key is loaded:

```bash
ssh-add --apple-use-keychain ~/.ssh/id_rsa   # macOS
ssh-add ~/.ssh/id_rsa                         # Linux
```

Without this, `git push`/`git pull` will fail with `Permission denied (publickey)`.

### Claude Code authentication

Authenticate using **one or both** methods.

### Option A: OAuth (recommended)

Run on your **host machine** before opening the devcontainer:

```bash
claude login
```

Your OAuth credentials (`~/.claude/.credentials.json`) are copied into the container. Optionally, personal agents and skills listed in `.devcontainer/.env` are also copied. No conversation history, project memories, settings, or plugins are transferred. If tokens expire, re-run `claude login` on the host and rebuild.

You can also log in from inside the container using the print-link flow (no browser redirect needed):

```bash
claude login --print-link
```

### Option B: OAuth Token

```bash
export CLAUDE_CODE_OAUTH_TOKEN=<your-token>
```

Set this in your shell profile. The token is forwarded into the container automatically.

### Option C: API Key

```bash
export ANTHROPIC_API_KEY=sk-ant-...
```

Set this in your shell profile so it's available when VS Code launches.

## Usage with VS Code

1. Install the [Dev Containers](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers) extension
2. Open this repository in VS Code
3. Press `Ctrl+Shift+P` (or `Cmd+Shift+P` on macOS) and select **Dev Containers: Reopen in Container**
4. Wait for the build (first time takes a while — Rust toolchain, etc.)
5. Claude Code is ready in the integrated terminal:

   ```bash
   claude  # runs with full permissions, no prompts
   ```

### Personal extensions

The `devcontainer.json` includes shared team extensions (rust-analyzer, eslint, Claude Code, etc.). To add your own extensions to every dev container, set this in your **host** VS Code settings (`Cmd+,` → search "defaultExtensions"):

```json
{
  "dev.containers.defaultExtensions": [
    "github.copilot",
    "vscodevim.vim"
  ]
}
```

## Usage with CLI (no VS Code)

You can use the [devcontainer CLI](https://github.com/devcontainers/cli) directly:

```bash
# Install the CLI
npm install -g @devcontainers/cli

# Build the container
devcontainer build --workspace-folder .

# Start and enter the container
devcontainer up --workspace-folder .
devcontainer exec --workspace-folder . bash

# Run Claude Code directly
devcontainer exec --workspace-folder . claude --dangerously-skip-permissions
```

Or use Docker Compose / `docker exec` if you prefer:

```bash
# Build
devcontainer build --workspace-folder .

# Start in background
devcontainer up --workspace-folder .

# Run Claude in headless mode (for CI/automation)
devcontainer exec --workspace-folder . claude -p "run the test suite" --dangerously-skip-permissions
```

## Claude Code customization

### Plugins

Plugins are **not** copied from your host. Use `.claude/settings.local.json` inside the container to enable personal plugins — this file is automatically gitignored by Claude Code:

```json
{
  "enabledPlugins": {
    "my-plugin@my-marketplace": true
  }
}
```

### Agents & skills

Personal agents and skills are **not** copied automatically. To bring specific ones from your `~/.claude/` into the container, create a `.env` file:

```bash
cp .devcontainer/.env.example .devcontainer/.env
```

Edit `.env` with comma-separated names:

```bash
# Agents from ~/.claude/agents/ (without .md extension)
CLAUDE_AGENTS=blockchain-security-auditor,rust-engineer

# Skills from ~/.claude/skills/ (directory names)
CLAUDE_SKILLS=my-custom-skill
```

The `.env` file is gitignored — each developer configures their own.

### Project-level settings

The project's `.claude/` directory is available inside the container via the workspace bind mount. Project-level agents (`.claude/agents/`) and skills (`.claude/skills/`) are automatically loaded by Claude Code.

## Security Model

Claude Code runs with `bypassPermissions` inside the container — it can read, write, and execute anything. The container is the sandbox boundary. To minimize exposure:

- **Only OAuth credentials** are copied from the host (`~/.claude/.credentials.json`). No conversation history, project memories, settings, plugins, hooks, scripts, or debug logs are transferred.
- **Agents/skills** are only copied if explicitly listed in `.devcontainer/.env` — nothing personal leaks in by default.
- **A clean `settings.json`** is generated inside the container with `bypassPermissions` — your host's permission allowlists, MCP server configs, and hooks are not copied.
- **No shell history** is persisted or shared with the container.
- **The `.git` directory** is mounted read-write (required for commits/pushes). This is the main trust boundary — Claude can push code.

## Network Firewall (optional)

By default, the container has unrestricted network access. To enable a restrictive firewall that only allows whitelisted services, add the following to `devcontainer.json`:

```jsonc
"runArgs": ["--cap-add=NET_ADMIN", "--cap-add=NET_RAW"],
"postStartCommand": "sudo /usr/local/bin/init-firewall.sh",
"waitFor": "postStartCommand"
```

You'll also need to add `iptables ipset iproute2 dnsutils` to the `apt-get install` in the Dockerfile and uncomment the firewall COPY/sudoers block. See `init-firewall.sh` for the domain whitelist.

## Persistent Data

These items survive container rebuilds (stored in Docker named volumes):

- `~/.cargo/registry` and `~/.cargo/git` — Rust dependency cache
- `target/` — Rust build artifacts
- `~/.claude/` — Claude Code credentials, settings, and optionally agents/skills from host

## Troubleshooting

### Git worktrees

Git worktrees are supported automatically. The `init-host.sh` script (runs on the host) detects whether you opened a worktree or the main repo and mounts the main `.git` directory into the container. The `post-create.sh` script creates the necessary symlinks so git resolves the worktree paths correctly. Commits and pushes from inside the container work as expected.

### Claude says "not authenticated"

- Ensure `ANTHROPIC_API_KEY` is set in your host shell, **or**
- Run `claude login` on your host and rebuild the container, **or**
- Run `claude login --print-link` inside the container (no browser redirect needed)

### `yarn install` fails

- Run `corepack enable` first (should be done by `post-create.sh`)

### Docker commands fail inside the container

- Docker-in-Docker starts automatically. If it didn't, check `docker info`.

### Firewall too restrictive (if enabled)

- Edit `.devcontainer/init-firewall.sh` to add domains
- Or temporarily flush rules: `sudo iptables -F OUTPUT`
