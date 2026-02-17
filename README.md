# YRAL Bare Metal Kubernetes Cluster

Ansible automation for managing a bare-metal Kubernetes cluster on Hetzner infrastructure.

## Prerequisites

- Ansible 2.9+ installed locally
- SSH access to bare-metal servers
- Python 3.x on target hosts

## Project Structure

```
.
├── ansible.cfg                  # Ansible configuration (at root)
├── ansible/
│   ├── inventory/
│   │   └── hosts.yml            # Inventory file with host definitions
│   ├── playbooks/               # Ansible playbooks for deployment
│   │   ├── helm-install.yml
│   │   ├── kube-vip-deploy.yml
│   │   ├── cluster-setup.yml
│   │   ├── cilium-deploy.yml
│   │   ├── monitoring-deploy.yml
│   │   ├── etcd-backup.yml
│   │   └── velero-install.yml
│   ├── roles/                   # Ansible roles
│   │   ├── kube-vip/
│   │   ├── cluster-init/
│   │   ├── worker-join/
│   │   ├── node-labels/
│   │   ├── cilium/
│   │   └── monitoring/
│   └── manifests/               # Kubernetes manifests and Helm values
│       ├── cilium-values.yaml
│       └── monitoring-values.yaml
└── scripts/
    └── install-ubuntu.sh        # Ubuntu installation script
```

## Inventory Configuration

The inventory is organized into groups for different Kubernetes node types:

- **control_plane**: Kubernetes master/control plane nodes
- **worker_nodes**: Kubernetes worker nodes
- **k8s_cluster**: Parent group containing all nodes

### Adding Hosts

Edit [ansible/inventory/hosts.yml](ansible/inventory/hosts.yml) and replace the placeholder IPs with your actual Hetzner server IPs:

```yaml
control_plane:
  hosts:
    master-01:
      ansible_host: YOUR_CONTROL_PLANE_IP  # Replace 10.0.0.1

worker_nodes:
  hosts:
    worker-01:
      ansible_host: YOUR_WORKER_IP         # Replace 10.0.0.2
```

To add more nodes, simply add additional entries:

```yaml
worker_nodes:
  hosts:
    worker-01:
      ansible_host: 10.0.0.2
    worker-02:
      ansible_host: 10.0.0.3
    worker-03:
      ansible_host: 10.0.0.4
```

## Running Ansible Locally

### Provisioning Workflow

The recommended provisioning sequence for new bare-metal servers:

1. **System Setup**: Update packages and configure unattended-upgrades
   ```bash
   ansible-playbook ansible/playbooks/system-setup.yml
   ```

2. **Btrfs Expansion** (if multiple drives): Expand filesystem across drives
   ```bash
   ansible-playbook ansible/playbooks/btrfs-expand.yml
   ```

3. **SSH Security**: Harden SSH configuration
   ```bash
   ansible-playbook ansible/playbooks/ssh-security.yml
   ```

4. **Container Runtime**: Install and configure containerd (pinned to 1.7.x)
   ```bash
   ansible-playbook ansible/playbooks/containerd-setup.yml
   ```

5. **Kubernetes Installation**: Install kubeadm, kubelet, and kubectl (pinned to v1.35)
   ```bash
   ansible-playbook ansible/playbooks/kubeadm-install.yml
   ```

After these steps, the cluster is ready for Kubernetes initialization (control plane setup and worker node joins).

### Test Connectivity (Ping)

```bash
ansible-playbook ansible/playbooks/ping.yml
```

With verbose output for debugging:

```bash
ansible-playbook ansible/playbooks/ping.yml -vvv
```

### Test Specific Host Groups

```bash
# Ping only control plane nodes
ansible-playbook ansible/playbooks/ping.yml --limit control_plane

# Ping only worker nodes
ansible-playbook ansible/playbooks/ping.yml --limit worker_nodes
```

### Ad-hoc Commands

```bash
# Check uptime on all hosts
ansible k8s_cluster -m shell -a "uptime"

# Check disk space
ansible k8s_cluster -m shell -a "df -h"

# List all hosts in inventory
ansible-inventory --list
```

## Day-2 Operations

For cluster operations like adding/removing nodes, see the operational playbooks in [ansible/playbooks/operations/](ansible/playbooks/operations/README.md)

### Viewing Results

Check the workflow run logs to see:
- Connection details for each host
- SSH debugging information (due to `-vvv` flag)
- Success/failure status for each node

## Configuration

### ansible.cfg

The [ansible.cfg](ansible.cfg) file at the root of the repository contains important settings:

- **inventory**: Points to `ansible/inventory/hosts.yml`
- **vault_password_file**: Points to `ansible/.vault_pass` for encrypted secrets
- **host_key_checking = False**: Disables SSH host key verification (suitable for automation)
- **pipelining = True**: Improves performance by reducing SSH connections
- **forks = 10**: Allows parallel execution on up to 10 hosts
- **gathering = smart**: Only gathers facts when needed

All ansible commands can be run from the repository root without specifying inventory paths.

### SSH Authentication

All connections use:
- **User**: `root` (configured in inventory)
- **Authentication**: SSH key-based (no passwords)
- **Host key checking**: Disabled for automation convenience

## Troubleshooting

### Connection Issues

If the ping test fails:

1. **Verify SSH access manually**:
   ```bash
   ssh root@YOUR_SERVER_IP
   ```

2. **Check if the SSH key is correct**:
   - Ensure the public key is in `/root/.ssh/authorized_keys` on target hosts
   - Verify the private key secret in GitHub matches your public key

3. **Test with verbose output**:
   ```bash
   ansible-playbook -i inventory/hosts.yml playbooks/ping.yml -vvv
   ```

4. **Verify inventory syntax**:
   ```bash
   ansible-inventory -i inventory/hosts.yml --list
   ```

### Common Issues

- **Permission denied**: SSH key not properly configured on target hosts
- **Host unreachable**: Check firewall rules and IP addresses
- **Python not found**: Install Python 3 on target hosts: `apt-get install python3`

## Next Steps

### Validation Commands

After running the provisioning playbooks, verify the setup on a control plane node (e.g., control-plane-1):

```bash
# Check containerd is running
ssh root@95.216.228.60 "systemctl status containerd"

# Verify containerd socket exists
ssh root@95.216.228.60 "ls -la /run/containerd/containerd.sock"

# Check systemd cgroup configuration
ssh root@95.216.228.60 "grep SystemdCgroup /etc/containerd/config.toml"

# Verify no swap is enabled
ssh root@95.216.228.60 "swapon --show"

# Check kernel modules are loaded
ssh root@95.216.228.60 "lsmod | grep -E 'overlay|br_netfilter'"

# Verify sysctl parameters
ssh root@95.216.228.60 "sysctl net.ipv4.ip_forward net.bridge.bridge-nf-call-iptables"

# Check kubeadm version
ssh root@95.216.228.60 "kubeadm version"

# Verify kubectl version
ssh root@95.216.228.60 "kubectl version --client"

# Check kubelet service (will be in crashloop until kubeadm init/join)
ssh root@95.216.228.60 "systemctl status kubelet"

# Test crictl (containerd CLI)
ssh root@95.216.228.60 "crictl --runtime-endpoint unix:///run/containerd/containerd.sock version"
```

### Next Steps for Cluster Setup

After successful validation, you can proceed with:

1. **Initialize Control Plane**: Run `kubeadm init` on the first control plane node
2. **Set Up HA Control Plane**: Join additional control plane nodes
3. **Install CNI Plugin**: Deploy a network plugin (Calico, Flannel, Cilium, etc.)
4. **Join Worker Nodes**: Add worker nodes to the cluster
5. **Configure kubectl**: Set up kubeconfig for cluster access
6. **Deploy Applications**: Start deploying workloads

After successful ping tests, you can:

1. Add more playbooks for Kubernetes cluster setup
2. Configure roles for different node types
3. Add playbooks for system updates and maintenance
4. Implement deployment workflows

## Security Notes

- SSH keys are stored securely in GitHub Secrets
- Host key checking is disabled for automation (understand MITM risks)
- All connections use root user (consider creating dedicated ansible user with sudo access)
- Review and rotate SSH keys regularly
