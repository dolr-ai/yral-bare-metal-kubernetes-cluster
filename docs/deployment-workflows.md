# Deployment Workflows Guide

This repository contains two complementary GitHub Actions workflows for managing bare metal Kubernetes clusters on Hetzner servers.

## Workflows Overview

### 1. `provision-server.yml` - Individual Server Provisioning
**Purpose**: Provision a single bare metal server from rescue mode to Ubuntu 24.04 with base configuration.

**What it does**:
1. Activates Hetzner rescue mode with SSH keys
2. Reboots server into rescue system
3. Detects drive type (NVMe/SATA)
4. Installs Ubuntu 24.04 via installimage
5. Runs base system configuration:
   - system-setup (packages, timezone, NTP)
   - btrfs-expand (expand to second drive if available)
   - ssh-security (disable password auth)
   - containerd-setup (container runtime)
   - kubeadm-install (Kubernetes binaries)

**When to use**: 
- Provisioning a new server from scratch
- Rebuilding a server that needs OS reinstall
- Adding a new node to the cluster

**Usage**:
```bash
gh workflow run provision-server.yml -f target_host=worker-11
```

**Time**: ~20-30 minutes per server

---

### 2. `deploy-cluster.yml` - Kubernetes Cluster Deployment
**Purpose**: Deploy and manage Kubernetes cluster components on already-provisioned servers.

**Deployment Options**:

#### `base-system-setup`
Runs base configuration on all nodes (assumes Ubuntu is installed):
- system-setup
- ssh-security  
- containerd-setup
- kubeadm-install

**Use case**: After provisioning servers with `provision-server`, or when you need to reconfigure base system on all nodes.

#### `kubernetes-only`
Deploys complete Kubernetes stack (assumes base setup is done):
- Helm installation
- kube-vip (HA VIP)
- Cluster initialization (control planes + workers)
- Cilium CNI with Service Mesh
- Monitoring stack (Prometheus + Grafana)
- etcd backups
- Velero backups

**Use case**: Fresh Kubernetes deployment on prepared servers.

#### `full-deployment`
Complete end-to-end deployment:
- `base-system-setup` + `kubernetes-only`

**Use case**: Going from provisioned Ubuntu servers to full Kubernetes cluster in one run.

#### Individual Components
Run specific playbooks only:
- `helm-install`
- `kube-vip-deploy`
- `cluster-setup`
- `cilium-deploy`
- `monitoring-deploy`
- `etcd-backup`
- `velero-install`

**Use case**: Updating or reinstalling specific components.

**Time**: 60-90 minutes for full deployment

---

## Complete Deployment Workflow

### Scenario 1: Fresh Cluster from Bare Metal

**Step 1**: Provision all servers individually
```bash
# Provision each server (can run in parallel)
gh workflow run provision-server.yml -f target_host=control-plane-1
gh workflow run provision-server.yml -f target_host=control-plane-2
gh workflow run provision-server.yml -f target_host=control-plane-3
gh workflow run provision-server.yml -f target_host=worker-1
# ... repeat for all workers
```

**Step 2**: Deploy full Kubernetes cluster
```bash
gh workflow run deploy-cluster.yml -f playbook=full-deployment
```

**Total time**: 
- Provisioning: 20-30 min per server (can parallelize with multiple GitHub Actions runners)
- Kubernetes deployment: 60-90 minutes
- **Total for 16 nodes**: ~3-4 hours if done serially, ~2 hours if provisioning in parallel

### Scenario 2: Cluster Already Provisioned, Need to Redeploy Kubernetes

```bash
gh workflow run deploy-cluster.yml -f playbook=kubernetes-only
```

### Scenario 3: Need to Reconfigure Base System + Redeploy Kubernetes

```bash
gh workflow run deploy-cluster.yml -f playbook=full-deployment
```

### Scenario 4: Update Specific Component

```bash
# Update Cilium only
gh workflow run deploy-cluster.yml -f playbook=cilium-deploy

# Update monitoring stack only  
gh workflow run deploy-cluster.yml -f playbook=monitoring-deploy
```

---

## Workflow Comparison

| Feature | provision-server | deploy-cluster |
|---------|-----------------|----------------|
| **Scope** | Single server | All cluster nodes |
| **Starting Point** | Bare metal / Rescue mode | Ubuntu installed |
| **OS Installation** | ✅ Yes | ❌ No |
| **Base Config** | ✅ Yes (automated) | ✅ Yes (optional) |
| **Kubernetes** | ❌ No | ✅ Yes |
| **Parallelization** | One at a time | All nodes together |
| **Use Case** | New/rebuilt servers | Cluster deployment |

---

## No Duplication Strategy

The workflows are designed to complement each other without duplication:

1. **provision-server**: Handles OS installation and base configuration for ONE server
2. **deploy-cluster**: Handles Kubernetes deployment for ALL servers

If you need to:
- **Add a new node**: Run `provision-server` for that node, then run `cluster-setup` to join it
- **Fresh cluster**: Run `provision-server` for all nodes, then `full-deployment`
- **Redeploy Kubernetes**: Just run `kubernetes-only` (skips base setup)
- **Update component**: Run specific playbook (cilium-deploy, monitoring-deploy, etc.)

---

## Required GitHub Secrets

Both workflows require these secrets:
- `HETZNER_BARE_METAL_GITHUB_ACTIONS_SSH_PRIVATE_KEY`: SSH private key for root access
- `HETZNER_ROBOT_PASSWORD`: Hetzner Robot API password
- `HETZNER_S3_SECRET_KEY`: Hetzner Object Storage secret key for backups

---

## Monitoring Deployment

### Via GitHub Actions UI
```bash
# List recent runs
gh run list --workflow=deploy-cluster.yml --limit 5

# Watch a specific run
gh run watch <run-id>

# View logs
gh run view <run-id> --log
```

### Via SSH
```bash
# Check cluster status
ssh root@95.216.228.60 'kubectl get nodes -o wide'

# Check all pods
ssh root@95.216.228.60 'kubectl get pods -A'

# Check Cilium
ssh root@95.216.228.60 'kubectl -n kube-system exec ds/cilium -- cilium-dbg status'
```

---

## Troubleshooting

### provision-server fails at Ubuntu install
- Check rescue mode activation logs
- Verify SSH keys are added to Hetzner account
- Check drive detection output

### deploy-cluster fails at cluster-setup
- Verify all servers are reachable via SSH
- Check that containerd is running on all nodes
- Verify DNS resolution for kubernetes-api.yral.com

### Individual nodes not joining cluster
- Check kubeadm join command in cluster-setup logs
- Verify kube-vip is running on control planes
- Check network connectivity between nodes

---

## Best Practices

1. **Provision servers first**: Always run `provision-server` before `deploy-cluster`
2. **Test SSH access**: Verify you can SSH to all nodes before deployment
3. **Monitor deployments**: Watch GitHub Actions logs during deployment
4. **Use specific playbooks**: For updates, use targeted playbooks instead of full redeployment
5. **Backup before changes**: Ensure backups are working before major updates
