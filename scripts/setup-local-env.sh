#!/bin/bash
# Local environment setup for macOS.
# Idempotent — safe to re-run after vault changes or on a new machine.
# Run from anywhere: bash scripts/setup-local-env.sh
#
# What this does:
#   - Installs system tools via brew (direnv, kubectl, helm, flux, sops, age, gh, rpk, uv)
#   - Syncs Python deps via uv (ansible-core, ansible-lint, yamllint) into .venv
#   - Installs Ansible collections (ansible-galaxy)
#   - Extracts secrets from vault: SSH key, kubeconfig, age key, env vars
#   - Generates .env (gitignored) with secrets; .envrc (committed) loads it via direnv

set -e

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ANSIBLE_DIR="$REPO_ROOT/ansible"

echo "========================================="
echo "Local Environment Setup"
echo "========================================="

# Require Homebrew — all system-level tools are managed via brew
echo ""
echo "Checking Homebrew..."
if ! command -v brew >/dev/null 2>&1; then
    echo "✗ Homebrew not found. Install from https://brew.sh and re-run."
    exit 1
fi
echo "✓ Homebrew available"

# System tools via brew
echo ""
echo "Installing brew packages..."
brew_install() {
    local pkg="$1"
    local cmd="${2:-$1}"
    if command -v "$cmd" >/dev/null 2>&1; then
        echo "✓ $pkg already installed"
    else
        brew install "$pkg"
        echo "✓ $pkg installed"
    fi
}

brew_install uv
brew_install direnv
brew_install kubectl
brew_install helm
brew_install fluxcd/tap/flux flux
brew_install sops
brew_install age age-keygen
brew_install gh

if command -v rpk >/dev/null 2>&1; then
    echo "✓ rpk already installed: $(rpk version 2>/dev/null | head -1)"
else
    brew install redpanda-data/tap/redpanda
    echo "✓ rpk installed"
fi

# Add direnv hook to shell profiles — idempotent via sentinel markers.
# Only the hook goes in the profile (no secrets); secrets live in .env.
echo ""
echo "Configuring direnv shell hook..."
for ENTRY in "zsh:.zshrc" "bash:.bashrc"; do
    SHELL_BIN="${ENTRY%%:*}"
    RC="$HOME/${ENTRY##*:}"
    touch "$RC"
    sed -i '' '/# BEGIN direnv-hook/,/# END direnv-hook/d' "$RC"
    printf '\n# BEGIN direnv-hook\neval "$(direnv hook %s)"\n# END direnv-hook\n' "$SHELL_BIN" >> "$RC"
done
echo "✓ direnv hook added to ~/.zshrc and ~/.bashrc"

# Python deps via uv — creates/updates .venv in repo root
echo ""
echo "Syncing Python dependencies via uv..."
uv sync --project "$REPO_ROOT"
echo "✓ Python dependencies synced into .venv"

# Activate the venv for the remainder of this script so that ansible-vault,
# ansible-galaxy, and python3 all resolve from the venv.
# shellcheck disable=SC1091
source "$REPO_ROOT/.venv/bin/activate"
echo "✓ .venv activated"

# Ansible collections — ecosystem-specific, use ansible-galaxy from the venv
echo ""
echo "Installing Ansible collections..."
ansible-galaxy collection install -r "$ANSIBLE_DIR/requirements.yml" -p "$ANSIBLE_DIR/collections"
echo "✓ Ansible collections installed"

# Everything below requires the vault password file
if [ ! -f "$ANSIBLE_DIR/.vault_pass" ]; then
    echo ""
    echo "⚠ Vault password file not found at $ANSIBLE_DIR/.vault_pass"
    echo "  Place the vault password there and re-run to complete setup."
    exit 0
fi

# Extract SSH private key to the default SSH identity path so ssh, git, and
# ansible all pick it up automatically without explicit -i flags or
# private_key_file config. Single key for infra access and git operations.
echo ""
echo "Setting up SSH key..."
SSH_KEY=$(ansible-vault view "$ANSIBLE_DIR/inventory/group_vars/all/vault.yml" | \
    python3 -c "
import sys, yaml
d = yaml.safe_load(sys.stdin)
print(d.get('vault_github_actions_ssh_private_key', ''))
")
if [ -n "$SSH_KEY" ]; then
    SSH_KEY_FILE="$HOME/.ssh/id_ed25519"
    mkdir -p "$(dirname "$SSH_KEY_FILE")"
    printf '%s\n' "$SSH_KEY" > "$SSH_KEY_FILE"
    chmod 600 "$SSH_KEY_FILE"
    echo "✓ SSH key extracted to $SSH_KEY_FILE"
    ssh-add "$SSH_KEY_FILE" 2>/dev/null && echo "✓ SSH key added to agent" || true
else
    echo "⚠ vault_github_actions_ssh_private_key not found in vault"
fi

# Extract kubeconfig — standard location used by kubectl
echo ""
echo "Setting up kubeconfig..."
mkdir -p ~/.kube
ansible-vault view "$ANSIBLE_DIR/inventory/group_vars/all/vault.yml" | \
    python3 -c "
import sys, yaml
d = yaml.safe_load(sys.stdin)
kubeconfig = d.get('vault_kubeconfig', '')
if kubeconfig:
    print(kubeconfig.rstrip())
" > ~/.kube/config
if [ -s ~/.kube/config ]; then
    chmod 600 ~/.kube/config
    echo "✓ Kubeconfig written to ~/.kube/config"
else
    rm -f ~/.kube/config
    echo "⚠ Kubeconfig not found in vault (cluster may not be initialized yet)"
fi

# Extract age private key — SOPS standard location; also applied to cluster for Flux
echo ""
echo "Setting up age key for SOPS..."
AGE_PRIVATE_KEY=$(ansible-vault view "$ANSIBLE_DIR/inventory/group_vars/all/vault.yml" | \
    python3 -c "
import sys, yaml
d = yaml.safe_load(sys.stdin)
print(d.get('vault_age_private_key', ''))
")
if [ -n "$AGE_PRIVATE_KEY" ]; then
    mkdir -p "$HOME/.config/sops/age"
    printf '%s\n' "$AGE_PRIVATE_KEY" > "$HOME/.config/sops/age/keys.txt"
    chmod 600 "$HOME/.config/sops/age/keys.txt"
    echo "✓ Age key written to ~/.config/sops/age/keys.txt"
    kubectl create secret generic sops-age \
        --namespace flux-system \
        --from-literal=age.agekey="$AGE_PRIVATE_KEY" \
        --dry-run=client -o yaml | kubectl apply -f - && \
        echo "✓ sops-age secret applied in flux-system namespace" || \
        echo "⚠ Could not apply sops-age secret (cluster may not be reachable yet)"
else
    echo "⚠ vault_age_private_key not found in vault"
fi

# Generate .env from vault — secrets live here; .envrc (committed) loads it via direnv.
# .env is gitignored; re-run this script after vault changes to refresh.
echo ""
echo "Generating .env from vault..."
GITHUB_FLUX_TOKEN=$(ansible-vault view "$ANSIBLE_DIR/inventory/group_vars/all/vault.yml" | \
    python3 -c "import sys,yaml; d=yaml.safe_load(sys.stdin); print(d.get('vault_github_flux_token',''))")
CF_TOKEN=$(ansible-vault view "$ANSIBLE_DIR/inventory/group_vars/all/vault.yml" | \
    python3 -c "import sys,yaml; d=yaml.safe_load(sys.stdin); print(d.get('vault_cloudflare_api_token',''))")
S3_SECRET_KEY=$(ansible-vault view "$ANSIBLE_DIR/inventory/group_vars/all/vault.yml" | \
    python3 -c "import sys,yaml; d=yaml.safe_load(sys.stdin); print(d.get('vault_hetzner_s3_secret_key',''))")

# .env — secrets only (loaded by direnv via dotenv_if_exists in .envrc)
{
    echo "# Generated by scripts/setup-local-env.sh — do not edit by hand, do not commit."
    echo "# Re-run the script to refresh after vault changes."
    echo ""
    printf "GITHUB_TOKEN=%q\n"           "$GITHUB_FLUX_TOKEN"
    printf "CLOUDFLARE_API_TOKEN=%q\n"   "$CF_TOKEN"
    printf "AWS_ACCESS_KEY_ID=%q\n"       "XO5X9A1W8AMHY3DSTKMS"
    printf "AWS_SECRET_ACCESS_KEY=%q\n"  "$S3_SECRET_KEY"
    printf "AWS_ENDPOINT_URL=%q\n"        "https://fsn1.your-objectstorage.com"
    printf "AWS_DEFAULT_REGION=%q\n"      "fsn1"
    printf "KUBECONFIG=%q\n"              "$HOME/.kube/config"
} > "$REPO_ROOT/.env"

direnv allow "$REPO_ROOT"
echo "✓ .env (secrets) generated; .envrc (committed) loads it via direnv"

# Authenticate Vast.ai CLI — writes API key to ~/.config/vastai/api_key once
echo ""
echo "Authenticating Vast.ai CLI..."
VASTAI_API_KEY=$(ansible-vault view "$ANSIBLE_DIR/inventory/group_vars/all/vault.yml" | \
    python3 -c "import sys,yaml; d=yaml.safe_load(sys.stdin); print(d.get('vault_vastai_api_key',''))")
if [ -n "$VASTAI_API_KEY" ]; then
    vastai set api-key "$VASTAI_API_KEY"
    echo "✓ Vast.ai API key configured"
else
    echo "⚠ vault_vastai_api_key not found in vault (add it when ready)"
fi

# Configure GitHub CLI to use SSH
echo ""
echo "Configuring GitHub CLI..."
gh config set git_protocol ssh --host github.com 2>/dev/null || true
echo "✓ GitHub CLI configured to use SSH protocol"

echo ""
echo "========================================="
echo "✓ Setup complete!"
echo "  Reopen your terminal (or run: source ~/.zshrc)"
echo "  Then cd into the repo — direnv will activate the venv and load secrets."
echo "========================================="
