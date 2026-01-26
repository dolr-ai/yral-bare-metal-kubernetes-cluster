#!/usr/bin/env bash
set -euo pipefail

# YRAL Bare Metal Kubernetes - Local Deployment Script
# This script helps run Ansible playbooks locally with vault support

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
ANSIBLE_DIR="$PROJECT_ROOT/ansible"
VAULT_PASS_FILE="$ANSIBLE_DIR/.vault_pass"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_info() {
    echo -e "${BLUE}ℹ ${NC}$1"
}

print_success() {
    echo -e "${GREEN}✓ ${NC}$1"
}

print_warning() {
    echo -e "${YELLOW}⚠ ${NC}$1"
}

print_error() {
    echo -e "${RED}✗ ${NC}$1"
}

# Check for vault password file
check_vault_password() {
    if [ ! -f "$VAULT_PASS_FILE" ]; then
        print_error "Vault password file not found: $VAULT_PASS_FILE"
        echo ""
        echo "To set up vault access:"
        echo "  1. Create the password file: echo 'your-vault-password' > $VAULT_PASS_FILE"
        echo "  2. Secure the file: chmod 600 $VAULT_PASS_FILE"
        echo ""
        echo "See docs/vault-setup.md for more information."
        exit 1
    fi
    
    # Check file permissions
    PERMS=$(stat -c %a "$VAULT_PASS_FILE" 2>/dev/null || stat -f %A "$VAULT_PASS_FILE" 2>/dev/null)
    if [ "$PERMS" != "600" ]; then
        print_warning "Vault password file has incorrect permissions: $PERMS"
        print_info "Fixing permissions..."
        chmod 600 "$VAULT_PASS_FILE"
    fi
    
    print_success "Vault password file found"
}

# Run ansible playbook
run_playbook() {
    local playbook=$1
    local extra_args="${2:-}"
    
    print_info "Running playbook: $playbook"
    
    cd "$ANSIBLE_DIR"
    
    if [ -n "$extra_args" ]; then
        ansible-playbook -i inventory/hosts.yml "playbooks/$playbook" $extra_args
    else
        ansible-playbook -i inventory/hosts.yml "playbooks/$playbook"
    fi
}

# Display menu
show_menu() {
    echo ""
    echo "=========================================="
    echo "  YRAL Kubernetes Deployment Menu"
    echo "=========================================="
    echo ""
    echo "Orchestration Playbooks:"
    echo "  1) Full Deployment (base + kubernetes)"
    echo "  2) Base System Setup (system + ssh + containerd + kubeadm)"
    echo "  3) Kubernetes Only (helm → velero)"
    echo ""
    echo "Individual Components:"
    echo "  4) System Setup"
    echo "  5) SSH Security"
    echo "  6) Containerd Setup"
    echo "  7) Kubeadm Install"
    echo "  8) Helm Install"
    echo "  9) kube-vip Deploy"
    echo "  10) Cluster Setup"
    echo "  11) Cilium Deploy"
    echo "  12) Monitoring Deploy"
    echo "  13) etcd Backup"
    echo "  14) Velero Install"
    echo ""
    echo "Utility:"
    echo "  15) Test Connectivity (ping all hosts)"
    echo "  16) View Vault Contents"
    echo "  17) Edit Vault"
    echo ""
    echo "  0) Exit"
    echo ""
}

# Main script
main() {
    echo "=========================================="
    echo " YRAL Bare Metal Kubernetes Cluster"
    echo " Local Deployment Tool"
    echo "=========================================="
    echo ""
    
    # Check for vault password
    check_vault_password
    
    # Check for ansible
    if ! command -v ansible-playbook &> /dev/null; then
        print_error "Ansible not found. Please install Ansible first."
        echo "  Ubuntu/Debian: sudo apt install ansible"
        echo "  macOS: brew install ansible"
        echo "  pip: pip install ansible"
        exit 1
    fi
    
    print_success "Ansible found: $(ansible-playbook --version | head -n1)"
    
    # Interactive menu or direct playbook
    if [ $# -eq 0 ]; then
        while true; do
            show_menu
            read -p "Select option: " choice
            
            case $choice in
                1) run_playbook "full-deployment.yml" "-v" ;;
                2) run_playbook "base-system-setup.yml" "-v" ;;
                3) run_playbook "kubernetes-only.yml" "-v" ;;
                4) run_playbook "system-setup.yml" "-v" ;;
                5) run_playbook "ssh-security.yml" "-v" ;;
                6) run_playbook "containerd-setup.yml" "-v" ;;
                7) run_playbook "kubeadm-install.yml" "-v" ;;
                8) run_playbook "helm-install.yml" "-v" ;;
                9) run_playbook "kube-vip-deploy.yml" "-v" ;;
                10) run_playbook "cluster-setup.yml" "-v" ;;
                11) run_playbook "cilium-deploy.yml" "-v" ;;
                12) run_playbook "monitoring-deploy.yml" "-v" ;;
                13) run_playbook "etcd-backup.yml" "-v" ;;
                14) run_playbook "velero-install.yml" "-v" ;;
                15) 
                    print_info "Testing connectivity to all hosts..."
                    cd "$ANSIBLE_DIR"
                    ansible all -i inventory/hosts.yml -m ping
                    ;;
                16)
                    print_info "Viewing vault contents..."
                    ansible-vault view "$ANSIBLE_DIR/group_vars/all/vault.yml"
                    ;;
                17)
                    print_info "Opening vault for editing..."
                    ansible-vault edit "$ANSIBLE_DIR/group_vars/all/vault.yml"
                    ;;
                0) 
                    print_info "Exiting..."
                    exit 0
                    ;;
                *)
                    print_error "Invalid option: $choice"
                    ;;
            esac
            
            echo ""
            read -p "Press Enter to continue..."
        done
    else
        # Direct playbook execution
        run_playbook "$1" "${2:-}"
    fi
}

main "$@"
