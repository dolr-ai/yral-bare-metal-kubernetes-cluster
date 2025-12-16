# Run on bare metal

```bash
curl -sL https://raw.githubusercontent.com/dolr-ai/yral-bare-metal-kubernetes-cluster/refs/heads/main/hetzner-rescue/control-plane.sh | bash
```

## Project Overview
Hetzner Robot bare metal Kubernetes cluster using Fedora CoreOS, kubeadm, Cilium CNI, and Longhorn CSI.
- Focus: Maximum Kubernetes-native features, declarative GitOps.
- OS: Fedora CoreOS with auto-updates via zincati.
- Longhorn: Replication factor 2, using second disk (sdb/nvme1n1).
- Bootstrap: Use hetzner-rescue/control-plane.sh for initial setup.
- GitOps: FluxCD for automatic applies on commit (coming soon).

## Current Structure
- `butane/`: Butane configs for Ignition (e.g., node.bu).
- `ansible/`: Playbooks and inventory for post-provisioning.
- `hetzner-rescue/`: Scripts for Hetzner rescue mode bootstrap.
- `.github/workflows/`: CI for Ignition releases and Ansible deploys.

## Next: Adding GitOps
We'll add FluxCD directories and manifests for declarative management.
