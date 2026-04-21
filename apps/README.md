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
# 1. Add the submodule
git submodule add https://github.com/<org>/<repo-name>.git apps/<repo-name>
git add apps/<repo-name> .gitmodules
git commit -m "feat: add <repo-name> submodule"
```

## Populating submodules after a fresh clone

```bash
git submodule update --init --recursive
```

## Current submodules

| Submodule | Source repo | Deployed at |
|-----------|-------------|-------------|
| `apps/yral-mobile/` | https://github.com/dolr-ai/yral-mobile | Not yet present in `kubernetes/apps/` |
| `apps/yral-backend-canister/` | https://github.com/dolr-ai/yral-backend-canister | Not yet present in `kubernetes/apps/` |
