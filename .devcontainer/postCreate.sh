#!/bin/bash
# Post-create script for devcontainer setup
# Runs after the devcontainer is created

set -e

echo "========================================="
echo "DevContainer Post-Create Setup"
echo "========================================="

# Install system packages
echo ""
echo "Installing system packages..."
sudo apt-get update -qq && sudo apt-get install -y -qq dnsutils
echo "✓ Installed dnsutils (dig, nslookup, host)"

# Install Python packages
echo ""
echo "Installing Python packages..."
pip install --user -r requirements.txt
echo "✓ Installed Python packages from requirements.txt"

# Install required Ansible collections
echo ""
echo "Installing Ansible collections..."
ansible-galaxy collection install -r ansible/requirements.yml
echo "✓ Installed Ansible collections from requirements.yml"

# Extract SSH key from Ansible vault for Hetzner server access
echo ""
echo "Setting up SSH key from Ansible vault..."
ANSIBLE_DIR="/workspaces/yral-bare-metal-kubernetes-cluster/ansible"
SSH_KEY_FILE="$HOME/.ssh/hetzner-ansible-key"

# Extract the SSH key from vault
if [ -f "$ANSIBLE_DIR/.vault_pass" ]; then
    SSH_KEY=$(ansible-vault view "$ANSIBLE_DIR/inventory/group_vars/all/vault.yml" 2>/dev/null | \
        python3 -c "
import sys, yaml
d = yaml.safe_load(sys.stdin)
print(d.get('vault_github_actions_ssh_private_key', ''))
")
    if [ -n "$SSH_KEY" ]; then
        mkdir -p "$(dirname "$SSH_KEY_FILE")"
        printf '%s\n' "$SSH_KEY" > "$SSH_KEY_FILE"
        chmod 600 "$SSH_KEY_FILE"
        echo "✓ SSH key extracted from vault to $SSH_KEY_FILE"
        if [ -n "$SSH_AUTH_SOCK" ]; then
            ssh-add "$SSH_KEY_FILE" 2>/dev/null && echo "✓ SSH key added to agent" || echo "⚠ Could not add key to agent (may already be added)"
        fi
    else
        echo "⚠ Warning: vault_github_actions_ssh_private_key not found in vault"
    fi
else
    echo "⚠ Warning: Vault password file not found at $ANSIBLE_DIR/.vault_pass"
    echo "  SSH key setup skipped"
fi

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

# Extract cluster kubeconfig for local kubectl access
echo ""
echo "Setting up cluster kubeconfig..."
if [ -f "$ANSIBLE_DIR/.vault_pass" ]; then
    mkdir -p ~/.kube
    ansible-vault view "$ANSIBLE_DIR/inventory/group_vars/all/vault.yml" 2>/dev/null | \
        python3 -c "
import sys, yaml
d = yaml.safe_load(sys.stdin)
kubeconfig = d.get('vault_kubeconfig', '')
if kubeconfig:
    print(kubeconfig.rstrip())
" > ~/.kube/config
    if [ -s ~/.kube/config ]; then
        chmod 600 ~/.kube/config
        echo "✓ Cluster kubeconfig written to ~/.kube/config"
    else
        rm -f ~/.kube/config
        echo "⚠ Kubeconfig not found in vault (cluster may not be initialized yet)"
    fi
else
    echo "⚠ Vault password file not found — kubeconfig setup skipped"
fi

# Export GitHub PAT (Flux) into all shells via ~/.bashrc
echo ""
echo "Setting up GitHub token for Flux..."
if [ -f "$ANSIBLE_DIR/.vault_pass" ]; then
    GITHUB_FLUX_TOKEN=$(ansible-vault view "$ANSIBLE_DIR/inventory/group_vars/all/vault.yml" 2>/dev/null | \
        python3 -c "
import sys, yaml
d = yaml.safe_load(sys.stdin)
print(d.get('vault_github_flux_token', ''))
")
    if [ -n "$GITHUB_FLUX_TOKEN" ]; then
        # Remove any previous injection then append fresh (idempotent)
        sed -i '/# BEGIN flux-github-token/,/# END flux-github-token/d' ~/.bashrc
        printf '\n# BEGIN flux-github-token\nexport GITHUB_TOKEN=%s\n# END flux-github-token\n' "$GITHUB_FLUX_TOKEN" >> ~/.bashrc
        export GITHUB_TOKEN="$GITHUB_FLUX_TOKEN"
        echo "✓ GITHUB_TOKEN written to ~/.bashrc (available in all new shells)"
    else
        echo "⚠ vault_github_flux_token not found in vault (add it when ready)"
    fi
else
    echo "⚠ Vault password file not found — GitHub token setup skipped"
fi

# Create sops-age decryption key secret for Flux
# Flux uses this to decrypt SOPS-encrypted secrets (e.g. cloudflare-api-token) from git.
# The age private key is stored in vault and materialized into the cluster on every devcontainer start.
echo ""
echo "Setting up sops-age secret for Flux SOPS decryption..."
if [ -f "$ANSIBLE_DIR/.vault_pass" ]; then
    AGE_PRIVATE_KEY=$(ansible-vault view "$ANSIBLE_DIR/inventory/group_vars/all/vault.yml" 2>/dev/null | \
        python3 -c "
import sys, yaml
d = yaml.safe_load(sys.stdin)
print(d.get('vault_age_private_key', ''))
")
    if [ -n "$AGE_PRIVATE_KEY" ]; then
        # Write age key locally so `sops` CLI can decrypt/edit SOPS files in this devcontainer
        mkdir -p "$HOME/.config/sops/age"
        printf '%s\n' "$AGE_PRIVATE_KEY" > "$HOME/.config/sops/age/keys.txt"
        chmod 600 "$HOME/.config/sops/age/keys.txt"
        echo "✓ Age private key written to ~/.config/sops/age/keys.txt"

        kubectl create secret generic sops-age \
            --namespace flux-system \
            --from-literal=age.agekey="$AGE_PRIVATE_KEY" \
            --dry-run=client -o yaml | kubectl apply -f - && \
            echo "✓ sops-age secret applied in flux-system namespace" || \
            echo "⚠ Could not apply sops-age secret (cluster may not be reachable yet)"
    else
        echo "⚠ vault_age_private_key not found in vault — run SOPS setup first (see .sops.yaml)"
    fi
else
    echo "⚠ Vault password file not found — sops-age setup skipped"
fi

# Extract Cloudflare API token for DNS management (node-remove/control-plane-join roles)
echo ""
echo "Setting up Cloudflare API token..."
if [ -f "$ANSIBLE_DIR/.vault_pass" ]; then
    CF_TOKEN=$(ansible-vault view "$ANSIBLE_DIR/inventory/group_vars/all/vault.yml" 2>/dev/null | \
        python3 -c "
import sys, yaml
d = yaml.safe_load(sys.stdin)
print(d.get('vault_cloudflare_api_token', ''))
")
    if [ -n "$CF_TOKEN" ]; then
        printf '%s' "$CF_TOKEN" > /tmp/.cf
        chmod 600 /tmp/.cf
        echo "✓ Cloudflare API token written to /tmp/.cf"
    else
        echo "⚠ vault_cloudflare_api_token not found in vault"
    fi
else
    echo "⚠ Vault password file not found — Cloudflare token setup skipped"
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
