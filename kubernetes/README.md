# Kubernetes Manifests

Pure Kubernetes declarative objects that are applied to the cluster after
infrastructure provisioning is complete. These are not Ansible artifacts —
they have no dependency on Ansible roles or inventory.

## Separation of concerns

| Location | Contents | Applied by |
|----------|----------|------------|
| `ansible/manifests/` | Helm values files | Ansible roles (consumed via `copy` module) |
| `kubernetes/` | K8s objects, CRDs | `kubectl apply` (this directory) |

When a GitOps tool (ArgoCD/Flux) is added later, this directory becomes the
app-of-apps source — no restructuring needed.

## Apply order

Apply **after** the cluster has 3 control planes + at least 1 worker node:

```bash
# 1. Define the LoadBalancer IP pool (must be first — other services depend on it)
kubectl apply -f kubernetes/networking/cilium-lb-pool.yaml

# 2. Expose Hubble UI (pod schedules on workers; IP assigned from pool)
kubectl apply -f kubernetes/networking/hubble-ui-service.yaml

# Verify IP assignment
kubectl get svc -n kube-system hubble-ui-loadbalancer
```

## IP assignments (subnet 95.216.82.96/27, routed to CP-1 at 95.216.228.60)

| IP | Service | Source |
|----|---------|--------|
| 95.216.82.97 | Cilium Ingress Controller | `ansible/manifests/cilium-values.yaml` |
| 95.216.82.98 | Hubble UI | `kubernetes/networking/hubble-ui-service.yaml` |
| 95.216.82.99 | Grafana | `ansible/manifests/monitoring-values.yaml` |
| 95.216.82.100+ | Available | — |

## Directory structure

```
kubernetes/
  networking/
    cilium-lb-pool.yaml        # CiliumLoadBalancerIPPool — defines the IP pool
    hubble-ui-service.yaml     # Hubble UI LoadBalancer Service
```
