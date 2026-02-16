#!/bin/bash
# Post-create script for devcontainer setup
# Runs after the devcontainer is created

set -e

echo "========================================="
echo "DevContainer Post-Create Setup"
echo "========================================="

# Install Python linting tools in user packages
echo ""
echo "Installing Python linting tools..."
pip install --user --upgrade ansible-lint yamllint
echo "✓ Installed ansible-lint and yamllint"

# Add ~/.local/bin to PATH if not already there
if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
    echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
    export PATH="$HOME/.local/bin:$PATH"
    echo "✓ Added ~/.local/bin to PATH"
fi

# Install required Ansible collections
echo ""
echo "Installing Ansible collections..."
ansible-galaxy collection install community.general
echo "✓ Installed community.general collection"

# Check SSH Agent accessibility
echo ""
echo "Checking SSH Agent..."
if ssh-add -l >/dev/null 2>&1; then
    echo "✓ SSH Agent is accessible"
    ssh-add -l | sed 's/^/  /'
elif [ $? -eq 1 ]; then
    echo "✓ SSH Agent is running but has no identities"
    echo "  Run 'ssh-add ~/.ssh/id_ed25519' in WSL to add your key"
else
    echo "⚠ Warning: SSH agent not accessible"
    echo "  Ensure agent is running in WSL with fixed socket at ~/.ssh/agent.sock"
    echo "  See: https://github.com/dolr-ai/yral-bare-metal-kubernetes-cluster/blob/main/README.md#ssh-setup"
fi

# Configure GitHub CLI to use SSH
echo ""
echo "Configuring GitHub CLI..."
gh config set git_protocol ssh --host github.com 2>/dev/null || true
gh config set git_protocol ssh 2>/dev/null || true
echo "✓ GitHub CLI configured to use SSH protocol"

# Check gh authentication status
echo ""
echo "Checking GitHub CLI authentication..."
if gh auth status >/dev/null 2>&1; then
    echo "✓ GitHub CLI is authenticated"
else
    echo "⚠ GitHub CLI not authenticated (expected in CI)"
    echo "  For local development: ensure gh is authenticated in WSL"
fi

echo ""
echo "========================================="
echo "✓ DevContainer setup complete!"
echo "========================================="
