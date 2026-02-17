# Day-2 Operations Playbooks

This directory contains operational playbooks for managing the Kubernetes cluster lifecycle after initial deployment.

## Overview

These playbooks handle common operational tasks:
- **Adding nodes**: Provision and join new worker or control plane nodes
- **Removing nodes**: Safely remove nodes from the cluster
- **Maintenance**: Drain nodes for maintenance windows

## Prerequisites

- Initial cluster setup completed (`kubernetes-only.yml` or `full-deployment.yml`)
- Ansible vault password configured at `ansible/.vault_pass`
- SSH access to all nodes
- Valid Hetzner API credentials in vault (for provisioning operations)

## Playbooks

### 1. Add Worker Node

**File**: `add-worker.yml`

**Purpose**: Provision a new worker node and join it to the cluster.

**Usage**:
```bash
cd ansible
ansible-playbook playbooks/operations/add-worker.yml -e target_host=worker-14
```

**Steps**:
1. Validates target host exists in inventory under `worker_nodes` group
2. Provisions server via Hetzner Robot API (if not already provisioned)
3. Installs Ubuntu 24.04, base system, containerd, kubeadm
4. Generates join token from existing control plane
5. Joins node to cluster as worker
6. Applies topology and workload labels
7. Verifies node is Ready

**Requirements**:
- Target host must be in `ansible/inventory/hosts.yml` under `worker_nodes` group
- Hetzner credentials in vault for provisioning
- Sufficient cluster capacity

**Example Inventory Entry**:
```yaml
worker_nodes:
  hosts:
    worker-14:
      ansible_host: 168.119.70.14
      datacenter: fsn1-dc14
      region: eu-central
      country: de
      disk_type: sata
```

---

### 2. Add Control Plane Node

**File**: `add-control-plane.yml`

**Purpose**: Add a new control plane node to the HA cluster.

**Usage**:
```bash
cd ansible
ansible-playbook playbooks/operations/add-control-plane.yml -e target_host=control-plane-4
```

**Steps**:
1. Validates target host exists in inventory under `control_plane` group
2. Provisions server and installs base system
3. Uploads control plane certificates from existing control plane
4. Joins node as control plane with etcd member
5. Deploys kube-vip for VIP failover
6. Verifies etcd cluster health
7. Tests API server on new control plane

**Requirements**:
- Target host must be in `ansible/inventory/hosts.yml` under `control_plane` group
- At least one existing control plane must be healthy
- VIP address must be configured in inventory (`vip_address`)
- Hetzner credentials in vault

**Important Notes**:
- ⚠️ Adding control planes changes etcd quorum requirements
- 3 control planes = 2 nodes required for quorum
- 5 control planes = 3 nodes required for quorum
- Always maintain odd number of control planes (3, 5, 7)

**Example Inventory Entry**:
```yaml
control_plane:
  hosts:
    control-plane-4:
      ansible_host: 168.119.70.100
      datacenter: hel1-dc2
      region: eu-north
      country: fi
```

---

### 3. Drain Node

**File**: `drain-node.yml`

**Purpose**: Safely drain a node for maintenance by evicting all pods.

**Usage**:
```bash
cd ansible
ansible-playbook playbooks/operations/drain-node.yml -e target_host=worker-5
```

**Steps**:
1. Validates target host exists in inventory
2. Cordons the node (marks as unschedulable)
3. Evicts all pods (except DaemonSets)
4. Waits for pods to terminate gracefully
5. Verifies drain completed successfully

**Options**:
- Grace period: 300 seconds (5 minutes)
- Timeout: 600 seconds (10 minutes)
- DaemonSets are ignored (they run on all nodes)
- EmptyDir data is deleted

**Important Notes**:
- ⚠️ This will cause service disruption!
- Ensure sufficient capacity on other nodes before draining
- StatefulSet pods will reschedule to other nodes (if storage allows)
- After maintenance, uncordon the node: `kubectl uncordon <node>`

**To Uncordon**:
```bash
kubectl uncordon worker-5
```

---

### 4. Remove Node

**File**: `remove-node.yml`

**Purpose**: Permanently remove a node (worker or control plane) from the cluster.

**Usage**:
```bash
cd ansible
# Remove worker
ansible-playbook playbooks/operations/remove-node.yml -e target_host=worker-5

# Remove control plane (requires explicit confirmation)
ansible-playbook playbooks/operations/remove-node.yml -e target_host=control-plane-3
```

**Steps**:
1. Validates target host and determines node type
2. Drains node (evicts all pods)
3. For control planes: Removes etcd member
4. Deletes node from Kubernetes cluster
5. Stops kubelet and cleans up local Kubernetes state
6. Verifies removal and cluster health

**Requirements**:
- For control planes: Must type `yes-remove-control-plane` to confirm
- For control planes: At least 2 control planes must remain for quorum
- Node must be in inventory

**Important Notes**:
- ⚠️ This is DESTRUCTIVE and PERMANENT!
- For control planes: Reduces HA capability
- Never remove more than 1 control plane at a time
- After removal, manually update `ansible/inventory/hosts.yml`
- Optionally return server to Hetzner if no longer needed

**Post-Removal Steps**:
1. Remove node from `ansible/inventory/hosts.yml`
2. Verify cluster health: `kubectl get nodes`
3. For control planes: Verify etcd health: `kubectl get pods -n kube-system | grep etcd`
4. Return server to Hetzner (optional): Use Hetzner Robot interface

---

## Workflows

### Adding Capacity

**Add multiple workers**:
```bash
cd ansible

# Add workers one at a time
ansible-playbook playbooks/operations/add-worker.yml -e target_host=worker-14
ansible-playbook playbooks/operations/add-worker.yml -e target_host=worker-15
ansible-playbook playbooks/operations/add-worker.yml -e target_host=worker-16

# Verify
kubectl get nodes -o wide
```

### Scaling Control Plane (3 → 5)

**Add control planes for higher availability**:
```bash
cd ansible

# Add control planes one at a time
ansible-playbook playbooks/operations/add-control-plane.yml -e target_host=control-plane-4
# Wait for node to be Ready
kubectl get nodes | grep control-plane-4

ansible-playbook playbooks/operations/add-control-plane.yml -e target_host=control-plane-5
# Wait for node to be Ready
kubectl get nodes | grep control-plane-5

# Verify etcd cluster
kubectl get pods -n kube-system | grep etcd
kubectl exec -n kube-system etcd-control-plane-1 -- etcdctl \
  --endpoints=https://127.0.0.1:2379 \
  --cacert=/etc/kubernetes/pki/etcd/ca.crt \
  --cert=/etc/kubernetes/pki/etcd/server.crt \
  --key=/etc/kubernetes/pki/etcd/server.key \
  member list
```

### Node Maintenance

**Perform maintenance on a worker node**:
```bash
cd ansible

# Step 1: Drain the node
ansible-playbook playbooks/operations/drain-node.yml -e target_host=worker-5

# Step 2: Perform maintenance on worker-5
ssh root@worker-5
apt-get update && apt-get upgrade -y
reboot

# Step 3: Wait for node to come back
# Step 4: Uncordon the node
kubectl uncordon worker-5

# Step 5: Verify
kubectl get nodes
kubectl get pods -A -o wide --field-selector spec.nodeName=worker-5
```

### Decommissioning Nodes

**Remove old workers and replace with new ones**:
```bash
cd ansible

# Step 1: Add new workers first
ansible-playbook playbooks/operations/add-worker.yml -e target_host=worker-14
ansible-playbook playbooks/operations/add-worker.yml -e target_host=worker-15

# Step 2: Wait for new workers to be Ready
kubectl get nodes

# Step 3: Remove old workers
ansible-playbook playbooks/operations/remove-node.yml -e target_host=worker-1
ansible-playbook playbooks/operations/remove-node.yml -e target_host=worker-2

# Step 4: Update inventory (remove worker-1 and worker-2)
vim ansible/inventory/hosts.yml

# Step 5: Verify cluster health
kubectl get nodes
kubectl get pods -A
```

---

## Safety Considerations

### Control Plane Operations

1. **Quorum Requirements**:
   - 3 control planes: Need 2 healthy for quorum
   - 5 control planes: Need 3 healthy for quorum
   - Losing quorum = cluster unusable

2. **Adding Control Planes**:
   - Add one at a time
   - Wait for each to be Ready before adding next
   - Verify etcd health after each addition

3. **Removing Control Planes**:
   - ⚠️ NEVER remove more than 1 at a time
   - Always maintain at least 3 control planes for HA
   - Verify etcd health before and after removal

### Worker Node Operations

1. **Capacity Planning**:
   - Ensure sufficient capacity before draining
   - Check pod resource requests: `kubectl describe node <node>`
   - Monitor cluster-wide resources: `kubectl top nodes`

2. **StatefulSets**:
   - Pods will reschedule if storage is available
   - LocalPV pods cannot reschedule (will be stuck Pending)
   - Check for LocalPV usage: `kubectl get pv -o wide`

3. **DaemonSets**:
   - DaemonSets are not evicted during drain
   - They will remain running on drained node
   - Examples: Cilium agents, kube-proxy, monitoring agents

### General Best Practices

- Always verify cluster health before and after operations
- Test in non-production first if possible
- Have backups ready (Velero, etcd snapshots)
- Perform operations during maintenance windows
- Document all changes in runbook or tickets
- Update inventory immediately after changes

---

## Troubleshooting

### Worker Node Won't Join

**Symptoms**: Join command fails or times out

**Diagnosis**:
```bash
# Check kubelet logs on worker
ssh root@worker-14
journalctl -u kubelet -f

# Check control plane API server
kubectl get cs
kubectl get pods -n kube-system
```

**Common Causes**:
- Network connectivity issues
- Firewall blocking ports (6443, 10250)
- Time sync issues (NTP)
- Token expired (tokens valid for 24 hours)

**Solution**:
```bash
# Regenerate join token
kubeadm token create --print-join-command

# Manually join
ssh root@worker-14
<paste join command>
```

### Control Plane Join Fails

**Symptoms**: Control plane join hangs or fails with certificate errors

**Diagnosis**:
```bash
# Check certificate key expiration (valid 2 hours)
ssh root@control-plane-1
kubeadm init phase upload-certs --upload-certs

# Check API server logs
kubectl logs -n kube-system kube-apiserver-control-plane-1
```

**Solution**:
```bash
# Regenerate certificate key and join command
ssh root@control-plane-1
kubeadm init phase upload-certs --upload-certs
# Copy certificate key output
kubeadm token create --print-join-command --certificate-key <certificate-key>

# Retry join on new control plane
ssh root@control-plane-4
<paste join command> --control-plane --apiserver-advertise-address=<node-ip>
```

### Drain Hangs or Fails

**Symptoms**: Drain command times out or pods won't evict

**Diagnosis**:
```bash
# Check which pods are stuck
kubectl get pods -A --field-selector spec.nodeName=worker-5

# Check PodDisruptionBudgets
kubectl get pdb -A

# Check for finalizers
kubectl get pods -A -o json | jq '.items[] | select(.metadata.finalizers != null) | {name: .metadata.name, finalizers: .metadata.finalizers}'
```

**Common Causes**:
- PodDisruptionBudget blocking eviction
- Pods with finalizers stuck
- LocalPV pods cannot reschedule

**Solution**:
```bash
# Force drain (use with caution)
kubectl drain worker-5 --ignore-daemonsets --delete-emptydir-data --force --grace-period=0

# Or manually delete stuck pods
kubectl delete pod <pod-name> -n <namespace> --force --grace-period=0
```

### etcd Member Won't Remove

**Symptoms**: etcd member remove fails or etcd cluster unhealthy

**Diagnosis**:
```bash
# Check etcd cluster health
kubectl exec -n kube-system etcd-control-plane-1 -- etcdctl \
  --endpoints=https://127.0.0.1:2379 \
  --cacert=/etc/kubernetes/pki/etcd/ca.crt \
  --cert=/etc/kubernetes/pki/etcd/server.crt \
  --key=/etc/kubernetes/pki/etcd/server.key \
  endpoint health --cluster

# List members
kubectl exec -n kube-system etcd-control-plane-1 -- etcdctl \
  --endpoints=https://127.0.0.1:2379 \
  --cacert=/etc/kubernetes/pki/etcd/ca.crt \
  --cert=/etc/kubernetes/pki/etcd/server.crt \
  --key=/etc/kubernetes/pki/etcd/server.key \
  member list
```

**Solution**:
```bash
# Remove member manually
kubectl exec -n kube-system etcd-control-plane-1 -- etcdctl \
  --endpoints=https://127.0.0.1:2379 \
  --cacert=/etc/kubernetes/pki/etcd/ca.crt \
  --cert=/etc/kubernetes/pki/etcd/server.crt \
  --key=/etc/kubernetes/pki/etcd/server.key \
  member remove <member-id>
```

---

## Quick Reference

### Common kubectl Commands

```bash
# List all nodes
kubectl get nodes -o wide

# Check node resource usage
kubectl top nodes

# List pods on specific node
kubectl get pods -A -o wide --field-selector spec.nodeName=worker-5

# Cordon/uncordon node
kubectl cordon worker-5
kubectl uncordon worker-5

# Check etcd members
kubectl exec -n kube-system etcd-control-plane-1 -- etcdctl \
  --endpoints=https://127.0.0.1:2379 \
  --cacert=/etc/kubernetes/pki/etcd/ca.crt \
  --cert=/etc/kubernetes/pki/etcd/server.crt \
  --key=/etc/kubernetes/pki/etcd/server.key \
  member list

# Verify API server endpoints
kubectl get endpoints kubernetes

# Check component health
kubectl get cs
```

### Ansible Vault Commands

```bash
# View encrypted variables
ansible-vault view ansible/group_vars/all/vault.yml

# Edit vault
ansible-vault edit ansible/group_vars/all/vault.yml

# Encrypt new file
ansible-vault encrypt ansible/group_vars/all/new-secrets.yml
```

---

## Further Reading

- [Kubernetes Node Management](https://kubernetes.io/docs/concepts/architecture/nodes/)
- [Safely Drain a Node](https://kubernetes.io/docs/tasks/administer-cluster/safely-drain-node/)
- [Operating etcd clusters](https://etcd.io/docs/v3.5/op-guide/)
- [kubeadm High Availability](https://kubernetes.io/docs/setup/production-environment/tools/kubeadm/high-availability/)
- [PodDisruptionBudgets](https://kubernetes.io/docs/concepts/workloads/pods/disruptions/)

---

**Last Updated**: January 2025  
**Kubernetes Version**: 1.35.1  
**Maintained By**: Yral Infrastructure Team
