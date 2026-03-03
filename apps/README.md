# apps/

This directory contains **git submodules** pointing to application source code repositories.

Application code lives in separate repositories. This directory acts as the integration point — submodules are added here on demand so that the cluster repo can reference a specific snapshot of each application while keeping the source repos independent.

## Relationship to `kubernetes/apps/`

| Directory | Contents |
|-----------|----------|
| `apps/<repo-name>/` | Application source code (git submodule) |
| `kubernetes/apps/<repo-name>/` | Kubernetes deployment manifests for that application |

Source code and deployment manifests are intentionally separated. The submodule here is a reference for human operators and CI pipelines; Flux reconciles from `kubernetes/apps/` independently of the submodule checkout state.

## Adding a new application

```bash
# 1. Add the submodule (use SSH remote for write access)
git submodule add git@github.com:dolr-ai/<repo-name>.git apps/<repo-name>

# 2. Commit the submodule pointer
git add apps/<repo-name> .gitmodules
git commit -m "feat: add <repo-name> submodule"

# 3. Create Kubernetes deployment manifests
mkdir -p kubernetes/apps/<repo-name>
# Add Namespace, Deployment, Service, HTTPRoute, etc.

# 4. Add a kustomization.yaml in kubernetes/apps/<repo-name>/

# 5. Register the app in kubernetes/apps/kustomization.yaml
#    Add: - <repo-name>  under resources:

# 6. Push — Flux deploys automatically via the apps Kustomization
git add kubernetes/apps/
git commit -m "feat: deploy <repo-name>"
git push
```

## Populating submodules after a fresh clone

```bash
git submodule update --init --recursive
```

## Current submodules

| Submodule | Source repo | Deployed at |
|-----------|-------------|-------------|
| `apps/timer-counter` | https://github.com/saikatdas0790/timer-counter | https://timer-counter.saikat.yral.com |
