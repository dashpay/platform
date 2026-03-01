#!/usr/bin/env bash
# Network firewall for Claude Code devcontainer sandbox.
# Restricts outbound traffic to only necessary services.
# Based on Anthropic's official init-firewall.sh pattern.
set -euo pipefail

# Skip if not running as root
if [ "$(id -u)" -ne 0 ]; then
    echo "ERROR: init-firewall.sh must run as root (use sudo)"
    exit 1
fi

# Skip if iptables is not available
if ! command -v iptables &>/dev/null; then
    echo "WARNING: iptables not found, skipping firewall setup"
    exit 0
fi

echo "Configuring devcontainer firewall..."

# Flush existing rules
iptables -F OUTPUT 2>/dev/null || true
ipset destroy allowed_hosts 2>/dev/null || true

# Create ipset for allowed hosts
ipset create allowed_hosts hash:ip hashsize 4096

# --- Resolve and allow domains ---

# NOTE: DNS resolution is point-in-time. CDN-backed services rotate IPs.
# The ESTABLISHED,RELATED rule helps for long-lived connections.
# This script re-runs on every container start (postStartCommand).
resolve_and_allow() {
    local domain="$1"
    local ips
    ips=$(dig +short "$domain" A 2>/dev/null | grep -E '^[0-9]+\.' || true)
    for ip in $ips; do
        ipset add allowed_hosts "$ip" 2>/dev/null || true
    done
}

# Claude / Anthropic API
resolve_and_allow "api.anthropic.com"
resolve_and_allow "sentry.io"
resolve_and_allow "statsig.anthropic.com"
resolve_and_allow "statsig.com"
resolve_and_allow "featuregates.org"
resolve_and_allow "prodregistryv2.org"

# npm registry
resolve_and_allow "registry.npmjs.org"
resolve_and_allow "registry.yarnpkg.com"

# Rust / crates.io
resolve_and_allow "crates.io"
resolve_and_allow "static.crates.io"
resolve_and_allow "index.crates.io"
resolve_and_allow "static.rust-lang.org"
resolve_and_allow "sh.rustup.rs"

# GitHub (dynamic IP ranges - added as CIDR rules below since ipset doesn't support /16 etc.)
GITHUB_IPS=$(curl -s https://api.github.com/meta 2>/dev/null | jq -r '.web[], .api[], .git[], .actions[]' 2>/dev/null || true)
resolve_and_allow "github.com"
resolve_and_allow "api.github.com"
resolve_and_allow "raw.githubusercontent.com"
resolve_and_allow "objects.githubusercontent.com"
resolve_and_allow "codeload.github.com"
resolve_and_allow "ghcr.io"

# VS Code marketplace
resolve_and_allow "marketplace.visualstudio.com"
resolve_and_allow "vscode.blob.core.windows.net"
resolve_and_allow "update.code.visualstudio.com"
resolve_and_allow "az764295.vo.msecnd.net"

# Protobuf releases (via GitHub, already covered)

# Docker Hub (for dashmate Docker-in-Docker)
resolve_and_allow "registry-1.docker.io"
resolve_and_allow "auth.docker.io"
resolve_and_allow "production.cloudflare.docker.com"

# Dash-specific
resolve_and_allow "testnet.platform-explorer.com"

# --- Apply iptables rules ---

# Allow loopback
iptables -A OUTPUT -o lo -j ACCEPT

# Allow established/related connections
iptables -A OUTPUT -m state --state ESTABLISHED,RELATED -j ACCEPT

# Allow DNS (UDP/TCP 53)
iptables -A OUTPUT -p udp --dport 53 -j ACCEPT
iptables -A OUTPUT -p tcp --dport 53 -j ACCEPT

# Allow SSH
iptables -A OUTPUT -p tcp --dport 22 -j ACCEPT

# Allow private networks: Docker-in-Docker (172.x), dashmate local nodes, host services.
# Broad ranges are intentional — dashmate orchestrates multiple containers on Docker networks.
iptables -A OUTPUT -d 172.16.0.0/12 -j ACCEPT
iptables -A OUTPUT -d 10.0.0.0/8 -j ACCEPT
iptables -A OUTPUT -d 192.168.0.0/16 -j ACCEPT

# Allow resolved hosts
iptables -A OUTPUT -m set --match-set allowed_hosts dst -j ACCEPT

# Allow GitHub CIDR ranges directly
for cidr in $GITHUB_IPS; do
    iptables -A OUTPUT -d "$cidr" -j ACCEPT 2>/dev/null || true
done

# Default deny all other outbound
iptables -A OUTPUT -j REJECT --reject-with icmp-port-unreachable

echo "Firewall configured. Verifying..."

# Verify: allowed domain should work
if curl -sf --max-time 5 -o /dev/null "https://api.github.com" 2>/dev/null; then
    echo "  [OK] api.github.com is reachable"
else
    echo "  [WARN] api.github.com is not reachable - firewall may be too restrictive"
fi

# Verify: blocked domain should fail
if curl -sf --max-time 3 -o /dev/null "https://example.com" 2>/dev/null; then
    echo "  [WARN] example.com is reachable - firewall may not be working"
else
    echo "  [OK] example.com is blocked"
fi

echo "Firewall setup complete."
