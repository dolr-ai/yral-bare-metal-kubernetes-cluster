# kubernetes/apps/

Kubernetes deployment manifests for application workloads.

Each subdirectory corresponds to one application and contains Pure K8s objects  
(Namespace, Deployment, Service, HTTPRoute, etc.) managed by Flux.

## Adding a new app

1. Create `kubernetes/apps/<app-name>/` with a `kustomization.yaml` and all needed manifests.
2. Add `- <app-name>` to the `resources:` list in `kubernetes/apps/kustomization.yaml`.
3. Commit and push — Flux reconciles the `apps` Kustomization and deploys.

The corresponding application source lives in `apps/<app-name>/` as a git submodule.  
Flux reconciles manifests from git; the submodule checkout state is independent.
