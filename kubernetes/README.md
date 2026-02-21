# Kubernetes Manifests

All Kubernetes resources that run as pods or control cluster workloads live here.
Ansible provisions the infrastructure; this directory manages everything above the Kubernetes API.

## Separation of concerns

| Location | Contents | Applied by |
|----------|----------|------------|
| `ansible/manifests/` | Helm values files consumed by Ansible roles | Ansible roles |
| `kubernetes/` | All K8s objects: Gateways, HTTPRoutes, Kustomizations, workloads | `kubectl apply` → Flux |

**Rule**: If it runs as a pod, it belongs here — not in Ansible. See `AGENTS.md` for the full policy.

## Flux readiness

This directory is structured as a Flux Kustomization source. When Flux is bootstrapped, point it at `kubernetes/` and it will reconcile everything with no restructuring needed.

## Directory structure

```
kubernetes/
  infrastructure/
    cert-manager/
      kustomization.yaml        # cert-manager install (v1.19.3) via Kustomize
      cluster-issuers.yaml      # Let's Encrypt staging + prod ClusterIssuers
  networking/
    gateway.yaml                # web-gateway: shared Gateway for all services (ports 80 + 443)
    routes/
      hubble-ui.yaml            # Hubble UI HTTPRoutes (HTTP→HTTPS redirect + HTTPS backend)
```

## Ingress architecture

External traffic reaches services via **DNS round-robin + Cilium Gateway API**:

1. `*.yral.com` DNS A records → all 5 control-plane public IPv4 addresses
2. Cilium Envoy handles the `web-gateway` (NodePort 80/443, set via `service-node-port-range`)
3. cert-manager automatically provisions and renews TLS certificates via Let's Encrypt HTTP-01
4. HTTPRoutes in any namespace attach to `web-gateway` — no Gateway changes needed per service

| CP | IPv4 |
|----|------|
| control-plane-1 | 95.216.228.60 |
| control-plane-2 | 95.216.6.238 |
| control-plane-3 | 95.216.24.51 |
| control-plane-4 | 95.217.35.190 |
| control-plane-5 | 95.216.16.50 |

## Manual apply order (before Flux)

Apply after the cluster has 3 control planes + at least 1 worker:

```bash
# 1. Install cert-manager (CRDs + controller pods)
kubectl apply -k kubernetes/infrastructure/cert-manager
kubectl wait --for=condition=available -n cert-manager deployment/cert-manager --timeout=120s
kubectl wait --for=condition=available -n cert-manager deployment/cert-manager-webhook --timeout=120s

# 2. Create ClusterIssuers (depends on cert-manager CRDs existing)
kubectl apply -f kubernetes/infrastructure/cert-manager/cluster-issuers.yaml

# 3. Create the shared Gateway (Cilium allocates it on NodePort 80/443)
kubectl apply -f kubernetes/networking/gateway.yaml

# 4. Create service HTTPRoutes
kubectl apply -f kubernetes/networking/routes/
```

## TLS workflow

cert-manager handles TLS automatically via the gateway-shim controller:
- The `web-gateway` is annotated with `cert-manager.io/cluster-issuer: letsencrypt-staging`
- cert-manager creates a per-hostname `Secret` in `kube-system` (e.g. `hubble-ui-yral-com-tls`) based on the `hostname` set on each HTTPS listener
- HTTP-01 challenge: cert-manager creates a temporary HTTPRoute on port 80; Let's Encrypt validates via any CP IP
- To switch to production certs: change the Gateway annotation to `letsencrypt-prod`, delete the TLS Secret — cert-manager re-issues it

**Adding a new hostname** requires adding a new HTTPS listener to the Gateway with the correct `hostname` and a new `certificateRefs` Secret name.

## Adding a new service

1. Deploy the service (Deployment + Service) in `kubernetes/apps/<service-name>/`
2. Create an HTTPRoute in `kubernetes/networking/routes/<service-name>.yaml`:

```yaml
apiVersion: gateway.networking.k8s.io/v1
kind: HTTPRoute
metadata:
  name: my-service
  namespace: my-namespace
spec:
  parentRefs:
    - name: web-gateway
      namespace: kube-system
      sectionName: https
  hostnames:
    - my-service.yral.com
  rules:
    - backendRefs:
        - name: my-service
          port: 8080
```

3. Add a redirect rule on the `http` sectionName (copy from `routes/hubble-ui.yaml`)
4. cert-manager automatically adds `my-service.yral.com` to the TLS certificate — no other changes needed
