# GitHub Actions Workflows

This directory contains CI/CD workflows for deploying and managing the Kubernetes cluster.

## Workflows

### 1. `validate-ansible.yml` - Continuous Validation

**Trigger**: Automatically runs on pull requests and pushes to main branch

**Purpose**: Validates all Ansible playbooks, roles, and configurations

**Checks**:
- ✅ YAML syntax validation with `yamllint`
- ✅ Ansible playbook syntax with `ansible-playbook --syntax-check`
- ✅ Best practices with `ansible-lint`
- ✅ Inventory structure validation
- ✅ Placeholder detection in inventory

**Manual Trigger**:
```bash
gh workflow run validate-ansible.yml
```

### 2. `deploy-cluster.yml` - Cluster Deployment

**Trigger**: Manual via `workflow_dispatch`

**Purpose**: Deploy and manage Kubernetes cluster via Ansible playbooks

**Available Playbooks**:
- `helm-install` - Install Helm on control planes
- `kube-vip-deploy` - Deploy kube-vip for HA
- `cluster-setup` - Initialize cluster and join nodes
- `cilium-deploy` - Deploy Cilium CNI with Service Mesh
- `monitoring-deploy` - Deploy Prometheus/Grafana stack
- `etcd-backup` - Setup automated etcd backups
- `velero-install` - Install Velero for cluster backups
- `all` - Run all playbooks in sequence

**Manual Trigger**:
```bash
# Deploy entire cluster
gh workflow run deploy-cluster.yml -f playbook=all

# Deploy specific playbook
gh workflow run deploy-cluster.yml -f playbook=cilium-deploy

# Check workflow status
gh run list --workflow=deploy-cluster.yml

# View logs
gh run view <run-id> --log
```

## Required Secrets

Configure these in your GitHub repository settings (`Settings` → `Secrets and variables` → `Actions` → `New repository secret`):

### `HETZNER_ROBOT_PASSWORD`
Hetzner Robot API password for server management
```bash
gh secret set HETZNER_ROBOT_PASSWORD
# Paste password when prompted
```

### `HETZNER_S3_SECRET_KEY`
Hetzner Object Storage secret key for backups
```bash
gh secret set HETZNER_S3_SECRET_KEY
# Paste secret key when prompted
```

### `HETZNER_BARE_METAL_GITHUB_ACTIONS_SSH_PRIVATE_KEY`
SSH private key for accessing bare metal servers
```bash
gh secret set HETZNER_BARE_METAL_GITHUB_ACTIONS_SSH_PRIVATE_KEY < ~/.ssh/id_ed25519
```

## Required Inventory Updates

Before running deployment workflows, update these placeholders in `inventory/hosts.yml`:

```yaml
hetzner_s3_access_key: "your-actual-access-key"  # Replace PLACEHOLDER_UPDATE_ME
hetzner_s3_bucket: "yral-k8s-backups"            # Replace PLACEHOLDER_UPDATE_ME
```

## Deployment Workflow

1. **Prepare Infrastructure**:
   ```bash
   # Ensure DNS is configured
   # kubernetes-api.yral.com → 77.42.49.55 (DNS-only, not proxied)
   ```

2. **Validate Configuration**:
   ```bash
   gh workflow run validate-ansible.yml
   gh run watch
   ```

3. **Deploy Prerequisites**:
   ```bash
   gh workflow run deploy-cluster.yml -f playbook=helm-install
   gh run watch
   ```

4. **Deploy HA Layer**:
   ```bash
   gh workflow run deploy-cluster.yml -f playbook=kube-vip-deploy
   gh run watch
   ```

5. **Initialize Cluster**:
   ```bash
   gh workflow run deploy-cluster.yml -f playbook=cluster-setup
   gh run watch
   ```

6. **Deploy CNI**:
   ```bash
   gh workflow run deploy-cluster.yml -f playbook=cilium-deploy
   gh run watch
   ```

7. **Deploy Monitoring**:
   ```bash
   gh workflow run deploy-cluster.yml -f playbook=monitoring-deploy
   gh run watch
   ```

8. **Setup Backups**:
   ```bash
   gh workflow run deploy-cluster.yml -f playbook=etcd-backup
   gh workflow run deploy-cluster.yml -f playbook=velero-install
   gh run watch
   ```

**Or deploy everything at once**:
```bash
gh workflow run deploy-cluster.yml -f playbook=all
gh run watch
```

## Monitoring Workflows

### View Recent Runs
```bash
gh run list --limit 10
```

### Watch Live Run
```bash
gh run watch
```

### View Run Logs
```bash
gh run view --log
```

### View Run Summary
```bash
gh run view
```

### Cancel Running Workflow
```bash
gh run cancel <run-id>
```

### Re-run Failed Workflow
```bash
gh run rerun <run-id>
```

## Troubleshooting

### Authentication Issues
```bash
# Login to GitHub CLI
gh auth login

# Verify authentication
gh auth status

# Set default repository
gh repo set-default
```

### Secret Management
```bash
# List all secrets
gh secret list

# Update a secret
gh secret set SECRET_NAME

# Remove a secret
gh secret remove SECRET_NAME
```

### Workflow Debugging
```bash
# Enable debug logging (add these secrets)
gh secret set ACTIONS_RUNNER_DEBUG --body "true"
gh secret set ACTIONS_STEP_DEBUG --body "true"

# Then re-run the workflow to see detailed logs
```

### Common Errors

#### "Missing required secrets"
**Solution**: Configure all required secrets listed above

#### "Placeholder found in inventory"
**Solution**: Update `inventory/hosts.yml` with actual values for:
- `hetzner_s3_access_key`
- `hetzner_s3_bucket`

#### "SSH connection failed"
**Solution**: Ensure SSH private key secret is correctly configured and matches the public key on servers

#### "Playbook syntax check failed"
**Solution**: Run validation workflow locally:
```bash
ansible-playbook --syntax-check -i inventory/hosts.yml playbooks/YOUR_PLAYBOOK.yml
```

## Local Development

### Run Validation Locally
```bash
# Install dependencies
pip install ansible ansible-lint yamllint

# Validate YAML
yamllint playbooks/ roles/ inventory/ manifests/

# Check Ansible syntax
ansible-playbook --syntax-check -i inventory/hosts.yml playbooks/*.yml

# Run ansible-lint
ansible-lint playbooks/ roles/

# Validate inventory
ansible-inventory -i inventory/hosts.yml --list
```

### Test Playbook Locally
```bash
# Dry run (check mode)
ansible-playbook -i inventory/hosts.yml playbooks/YOUR_PLAYBOOK.yml --check

# Run with verbose output
ansible-playbook -i inventory/hosts.yml playbooks/YOUR_PLAYBOOK.yml -v

# Run specific tags
ansible-playbook -i inventory/hosts.yml playbooks/YOUR_PLAYBOOK.yml --tags "YOUR_TAG"
```

## Security Best Practices

1. **Never commit secrets** to repository
2. **Rotate secrets regularly** (every 90 days recommended)
3. **Use SSH key authentication** instead of passwords where possible
4. **Review workflow logs** for sensitive data before making repository public
5. **Limit workflow permissions** to minimum required
6. **Enable branch protection** rules for main branch

## Additional Resources

- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Ansible Documentation](https://docs.ansible.com/)
- [Kubernetes Documentation](https://kubernetes.io/docs/)
- [Cilium Documentation](https://docs.cilium.io/)
- [kube-vip Documentation](https://kube-vip.io/)
