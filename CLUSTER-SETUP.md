# YRAL Bare Metal Kubernetes Cluster - HA Production Setup

Complete Ansible automation for a production-grade High Availability Kubernetes cluster on Hetzner bare metal infrastructure.

## Architecture

- **Control Plane**: 3 nodes in HEL1-DC2 (Helsinki, Finland) - HA with kube-vip ARP failover
- **Workers**: 10 nodes (9 in FSN1 Falkenstein, 1 in HEL1-DC2 Helsinki)
- **VIP**: 77.42.49.55 (Failover IP) for kubernetes-api.yral.com
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
- Cloudflare DNS configured: kubernetes-api.yral.com → 77.42.49.55 (DNS-only)

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

### 2. Provision Servers (if needed)

Provision servers with Ubuntu 24.04:
```bash
# Provision individual servers
ansible-playbook ansible/playbooks/provision-server.yml -e target_host=control-plane-1
ansible-playbook ansible/playbooks/provision-server.yml -e target_host=control-plane-2
ansible-playbook ansible/playbooks/provision-server.yml -e target_host=control-plane-3
```

### 3. Deploy Cluster

```bash
# Install Helm on control planes
ansible-playbook ansible/playbooks/helm-install.yml

# Deploy kube-vip on all control planes
ansible-playbook ansible/playbooks/kube-vip-deploy.yml

# Initialize cluster and join all nodes
ansible-playbook ansible/playbooks/cluster-setup.yml

# Deploy Cilium CNI
ansible-playbook ansible/playbooks/cilium-deploy.yml

# Deploy monitoring stack
ansible-playbook ansible/playbooks/monitoring-deploy.yml

# Setup automated backups
ansible-playbook ansible/playbooks/etcd-backup.yml
ansible-playbook ansible/playbooks/velero-install.yml
```

## Project Structure

```
.
├── inventory/
│   └── hosts.yml                     # Inventory with topology labels and credentials
├── ansible/
│   ├── playbooks/
│   ├── helm-install.yml              # Install Helm on control planes
│   ├── kube-vip-deploy.yml           # Deploy kube-vip static pods
│   ├── cluster-setup.yml             # Initialize cluster and join nodes
│   ├── cilium-deploy.yml             # Deploy Cilium CNI
│   ├── monitoring-deploy.yml         # Deploy Prometheus/Grafana
│   ├── etcd-backup.yml               # Setup etcd automated backups
│   ├── velero-install.yml            # Install Velero for cluster backups
│   ├── system-setup.yml              # System updates (existing)
│   ├── containerd-setup.yml          # Container runtime (existing)
│   └── kubeadm-install.yml           # Kubernetes installation (existing)
│   ├── roles/
│   ├── kube-vip/                     # kube-vip configuration role
│   ├── cluster-init/                 # Cluster initialization role
│   ├── node-labels/                  # Topology labels role
│   ├── cilium/                       # Cilium deployment role
│   └── monitoring/                   # Monitoring stack role
│   ├── manifests/
│   ├── cilium-values.yaml            # Cilium Helm values
│   └── monitoring-values.yaml        # kube-prometheus-stack Helm values
└── scripts/
    └── install-ubuntu.sh             # Ubuntu installation script (existing)
```

## Cluster Features

### High Availability
- **Control Plane VIP**: kube-vip with ARP mode for 2-10s failover
- **Failover subnet**: API-based failover for control-plane-1 server failure (90-110s)
- **Multi-datacenter workers**: Geographic distribution for resilience
- **Automated backups**: Daily etcd snapshots + Velero cluster backups

### Networking
- **CNI**: Cilium 1.16.5 with eBPF datapath
- **kube-proxy replacement**: Strict mode for better performance
- **Encryption**: WireGuard for pod-to-pod traffic
- **Ingress**: Cilium Ingress Controller with Gateway API
- **Service Mesh**: Envoy-based L7 proxy
- **LoadBalancer**: kube-vip with 30 IP pool

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

# Check kube-vip
kubectl get svc -A | grep LoadBalancer
kubectl get configmap -n kube-system kubevip

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

### kube-vip not working
```bash
# Check static pod
kubectl get pods -n kube-system -l component=kube-vip
kubectl logs -n kube-system -l component=kube-vip

# Verify manifest
cat /etc/kubernetes/manifests/kube-vip.yaml
```

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

### Adding Worker Nodes
1. Add to `ansible/inventory/hosts.yml` with topology labels
2. Run system setup playbooks
3. Run `cluster-setup.yml` with `--limit new-worker`

### Upgrading Kubernetes
1. Backup cluster with Velero
2. Drain nodes one by one
3. Update kubeadm, kubelet, kubectl
4. Upgrade control planes first, then workers

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
