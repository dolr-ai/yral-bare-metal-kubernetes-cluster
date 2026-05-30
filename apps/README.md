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
| `apps/saikat-api/` | https://github.com/dolr-ai/saikat-api | Not yet present in `kubernetes/apps/` |
| `apps/yral-common/` | https://github.com/dolr-ai/yral-common | Not yet present in `kubernetes/apps/` |
| `apps/hot-or-not-web-leptos-ssr/` | https://github.com/dolr-ai/hot-or-not-web-leptos-ssr | Not yet present in `kubernetes/apps/` |
| `apps/website/` | https://github.com/dolr-ai/website | Not yet present in `kubernetes/apps/` |
| `apps/yral-billing/` | https://github.com/dolr-ai/yral-billing | Not yet present in `kubernetes/apps/` |
| `apps/yral-metadata/` | https://github.com/dolr-ai/yral-metadata | Not yet present in `kubernetes/apps/` |
| `apps/yral-auth-v2/` | https://github.com/dolr-ai/yral-auth-v2 | Not yet present in `kubernetes/apps/` |
| `apps/off-chain-agent/` | https://github.com/dolr-ai/off-chain-agent | Not yet present in `kubernetes/apps/` |
