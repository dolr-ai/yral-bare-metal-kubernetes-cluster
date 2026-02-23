# YRAL Bare Metal Kubernetes Cluster - HA Production Setup

Complete Ansible automation for a production-grade High Availability Kubernetes cluster on Hetzner bare metal infrastructure.

## Architecture

- **Control Plane**: 5 nodes in HEL1-DC2 (Helsinki, Finland) - HA with DNS round-robin
- **Workers**: 2+ nodes in FSN1/HEL1
- **DNS**: kubernetes-api.yral.com → 5 A records (one per control plane), TTL=60, DNS-only
- **LoadBalancer Pool**: 95.217.49.193-95.217.49.222 (Failover subnet /27, 30 IPs)
- **CNI**: Cilium with kube-proxy replacement, WireGuard encryption, Ingress, Service Mesh
- **Monitoring**: Prometheus + Grafana on workers with datacenter-aware scheduling
- **Backups**: Automated etcd snapshots + Velero to Hetzner Object Storage

## Prerequisites

- Ansible 2.9+ installed locally
- SSH access to all bare-metal servers
- Python 3.x on target hosts
- Hetzner Robot API credentials
- Hetzner Object Storage bucket and credentials
- Cloudflare DNS configured: kubernetes-api.yral.com → 5 A records (one per control plane IP), DNS-only

## Quick Start

### 1. Update Placeholders in Inventory

Edit `ansible/inventory/hosts.yml` and update:
- `hetzner_s3_access_key`: Your Hetzner Object Storage access key
- `hetzner_s3_bucket`: Your S3 bucket name

Set environment variables:
```bash
export HETZNER_ROBOT_PASSWORD="your-robot-password"
export HETZNER_S3_SECRET_KEY="your-s3-secret-key"
```

### 2. Initial Cluster Deployment

For the initial cluster bootstrap, all servers must be pre-provisioned with Ubuntu 24.04. Manually provision each server via Hetzner Robot by activating rescue mode and running `scripts/install-ubuntu.sh`.

```bash
# Deploy complete cluster stack (all in proper order)
ansible-playbook ansible/playbooks/full-deployment.yml
```

This playbook orchestrates:
- Helm installation on control planes
- Kubernetes cluster initialization and node joins
- Cilium CNI setup
- Monitoring stack (Prometheus/Grafana)
- Automated backup configuration (etcd + Velero)

### 3. Day-2 Operations: Add/Remove Nodes

The cluster is designed for immutable node operations. To add or remove nodes, use the operations playbooks:

```bash
# Add a new worker node (provisions + configures + joins)
ansible-playbook ansible/playbooks/operations/add-worker.yml -e target_host=worker-16

# Add a new control plane node
ansible-playbook ansible/playbooks/operations/add-control-plane.yml -e target_host=control-plane-4

# Drain a node for maintenance
ansible-playbook ansible/playbooks/operations/drain-node.yml -e target_host=worker-5

# Remove a node permanently
ansible-playbook ansible/playbooks/operations/remove-node.yml -e target_host=worker-5
```

To add capabilities to the cluster (new CNI features, monitoring updates, etc.), drain the node, remove it, and add it back to re-run all roles with updated configurations.

## Project Structure

```
.
├── inventory/
│   └── hosts.yml                     # Inventory with topology labels and credentials
├── ansible/
│   ├── playbooks/
│   │   ├── full-deployment.yml       # Complete deployment orchestrator
│   │   ├── kubernetes-only.yml       # Cluster bootstrap + Cilium + monitoring
│   │   ├── cluster-setup.yml         # Cluster init and node joins
│   │   ├── helm-install.yml          # Helm installation
│   │   ├── cilium-deploy.yml         # Cilium CNI deployment
│   │   ├── monitoring-deploy.yml     # Prometheus/Grafana stack
│   │   ├── etcd-backup.yml           # etcd backup automation
│   │   ├── velero-install.yml        # Velero backup system
│   │   ├── lint-check.yml            # Ansible linting
│   │   └── operations/
│   │       ├── add-worker.yml        # Add new worker to cluster
│   │       ├── add-control-plane.yml # Add new control plane
│   │       ├── drain-node.yml        # Drain node for maintenance
│   │       └── remove-node.yml       # Remove node from cluster
│   ├── roles/                        # Core roles (all logic lives here)
│   │   ├── provision/                # Hetzner provisioning
│   │   ├── base-system/              # System setup (updates, networking)
│   │   ├── containerd/               # Container runtime
│   │   ├── kubernetes/               # kubeadm installation
│   │   ├── cluster-init/             # First control plane bootstrap
│   │   ├── control-plane-join/       # Additional control planes
│   │   ├── worker-join/              # Worker node join
│   │   ├── cilium/                   # CNI deployment
│   │   ├── monitoring/               # Observability stack
│   │   ├── node-labels/              # Topology labeling
│   │   ├── storage-setup/            # Storage provisioning
│   │   ├── add-worker/               # Day-2: add worker operations
│   │   ├── add-control-plane/        # Day-2: add control plane operations
│   │   ├── node-drain/               # Day-2: drain node for maintenance
│   │   ├── node-remove/              # Day-2: remove node operations
│   │   ├── full-add-worker/          # Orchestrator: provision + setup + join
│   │   └── full-add-control-plane/   # Orchestrator: provision + setup + join + vip
│   ├── manifests/
│   │   ├── cilium-values.yaml        # Cilium Helm values
│   │   └── monitoring-values.yaml    # kube-prometheus-stack Helm values
│   ├── inventory/
│   │   └── hosts.yml                 # Node inventory and variables
│   └── group_vars/
│       └── all/
│           ├── vars.yml              # Cluster configuration
│           └── vault.yml             # Encrypted secrets
└── scripts/
    └── install-ubuntu.sh             # Hetzner bare-metal Ubuntu provisioning
```

## Cluster Features

### High Availability
- **Control Plane HA**: DNS round-robin (kubernetes-api.yral.com → 5 A records, TTL=Auto, DNS-only at Cloudflare)
- **Multi-datacenter workers**: Geographic distribution for resilience
- **Automated backups**: Daily etcd snapshots + Velero cluster backups

### Networking
- **CNI**: Cilium 1.16.5 with eBPF datapath
- **kube-proxy replacement**: Strict mode for better performance
- **Encryption**: WireGuard for pod-to-pod traffic
- **Ingress**: Cilium Ingress Controller with Gateway API
- **Service Mesh**: Envoy-based L7 proxy


### Observability
- **Hubble**: Network flow visibility (UI at 95.217.49.194)
- **Prometheus**: Metrics collection (2 replicas on workers)
- **Grafana**: Dashboards at 95.217.49.195
- **Pre-configured dashboards**: Kubernetes, Cilium, Hubble, Node Exporter

### Topology Awareness
- Nodes labeled with datacenter, region, country
- Monitoring stack prefers same datacenter for low latency
- Soft topology spread constraints (no hard requirements)

## Access Information

After deployment:

### Kubernetes API
- Endpoint: `https://kubernetes-api.yral.com:6443`
- Kubeconfig: `/root/.kube/config` on control planes

### Hubble UI
- URL: `http://95.217.49.194`
- Network flow visualization and metrics

### Grafana
- URL: `http://95.217.49.195`
- Credentials: Check `/root/grafana-access.txt` on control-plane-1
- Default username: `admin`

### Cilium Ingress
- First IP: `95.217.49.193`
- Automatically assigns IPs from pool to Ingress resources

## Backup & Recovery

### Etcd Backups
- **Schedule**: Daily at 2 AM UTC
- **Retention**: 10 snapshots (local + S3)
- **Location**: Hetzner Object Storage S3 bucket
- **Per-node backups**: Each control plane backs up independently
- **Manual backup**: `/usr/local/bin/etcd-backup.sh`
- **Logs**: `/var/log/etcd-backup.log`

### Velero Cluster Backups
- **Schedule**: Daily at 3 AM UTC
- **Retention**: 30 days
- **Scope**: All namespaces except velero, kube-system
- **Manual backup**: `velero backup create my-backup`
- **Restore**: `velero restore create --from-backup <name>`
- **Documentation**: `/root/velero-usage.txt`

## Validation Commands

```bash
# Check cluster status
kubectl get nodes -o wide
kubectl get nodes -L topology.kubernetes.io/zone,topology.kubernetes.io/region

# Verify Cilium
kubectl -n kube-system get pods -l k8s-app=cilium
kubectl -n kube-system exec ds/cilium -- cilium-dbg status
kubectl -n kube-system exec ds/cilium -- cilium-dbg encrypt status

# Check LoadBalancer services
kubectl get svc -A | grep LoadBalancer

# Verify monitoring
kubectl get pods -n monitoring -o wide
kubectl get svc -n monitoring

# Test VIP failover
# On control-plane-1: systemctl stop kubelet
# Watch: kubectl get pods -n kube-system -w
# VIP should move to another control plane in 2-10s

# Check backups
s3cmd ls s3://your-bucket/etcd-backups/
velero backup get
```

## Troubleshooting

### Cilium issues
```bash
# Check Cilium status
kubectl -n kube-system exec ds/cilium -- cilium-dbg status
kubectl -n kube-system logs ds/cilium

# Restart Cilium pods
kubectl -n kube-system rollout restart ds/cilium
```

### Monitoring not accessible
```bash
# Check Grafana service
kubectl get svc -n monitoring kube-prometheus-stack-grafana
kubectl describe svc -n monitoring kube-prometheus-stack-grafana

# Check LoadBalancer IP assignment
kubectl get svc -A | grep LoadBalancer
```

### Backup failures
```bash
# Check etcd backup logs
tail -f /var/log/etcd-backup.log

# Test S3 access
s3cmd ls s3://your-bucket/

# Verify Velero
kubectl logs -n velero deployment/velero
velero backup describe <backup-name>
```

## Security Notes

- SSH keys stored securely in GitHub Secrets
- Control planes tainted to prevent application workloads
- WireGuard encryption for all pod-to-pod traffic
- etcd client certificate authentication
- S3 credentials stored in environment variables (not committed)
- Grafana admin password should be changed after first login

## Maintenance

### Adding/Removing Worker Nodes (Immutable Operations)

The cluster uses immutable node operations—nodes are removed and re-added rather than modified:

```bash
# Add a new worker node (provisions + configures + joins)
ansible-playbook ansible/playbooks/operations/add-worker.yml -e target_host=worker-16

# Remove a worker node (graceful drain and removal)
ansible-playbook ansible/playbooks/operations/remove-node.yml -e target_host=worker-5

# Drain a node for maintenance without removal
ansible-playbook ansible/playbooks/operations/drain-node.yml -e target_host=worker-5

# To add capabilities to an existing node: drain → remove → add
ansible-playbook ansible/playbooks/operations/drain-node.yml -e target_host=worker-5
ansible-playbook ansible/playbooks/operations/remove-node.yml -e target_host=worker-5
ansible-playbook ansible/playbooks/operations/add-worker.yml -e target_host=worker-5  # Rejoins with updated config
```

### Adding Control Plane Nodes

```bash
# Add a new control plane (provisions + configures + joins)
ansible-playbook ansible/playbooks/operations/add-control-plane.yml -e target_host=control-plane-4

# Remove a control plane (drains, removes from etcd, deletes node)
ansible-playbook ansible/playbooks/operations/remove-node.yml -e target_host=control-plane-4
```

### Upgrading Kubernetes

1. Backup cluster with Velero
2. Drain nodes one by one (`drain-node.yml`)
3. Update kubeadm, kubelet, kubectl via new `provision` role
4. Upgrade control planes first, then workers (via `add-*` operations)
5. Verify cluster health

### Scaling Monitoring
```bash
# Increase Prometheus replicas
kubectl -n monitoring scale statefulset prometheus-kube-prometheus-stack-prometheus --replicas=3

# Add more storage
kubectl -n monitoring edit statefulset prometheus-kube-prometheus-stack-prometheus
```

## Cost Breakdown

- **Failover IP**: $5/month (control plane VIP)
- **Failover subnet /27**: $70/month (30 LoadBalancer IPs)
- **Total networking**: $75/month

## Contributing

This is infrastructure-as-code for the YRAL production cluster. Changes should be tested in a staging environment before applying to production.

## License

Internal infrastructure - YRAL/DOLR-AI

---

**Status**: ✅ Production-ready
**Last Updated**: 2026-01-26
