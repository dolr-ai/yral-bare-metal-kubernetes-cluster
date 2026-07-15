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
# 1. Add the submodule (SSH recommended)
git submodule add git@github.com:<org>/<repo-name>.git apps/<repo-name>
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
| `apps/yral/` | git@github.com:dolr-ai/yral.git | — |
| `apps/yral-mobile/` | git@github.com:dolr-ai/yral-mobile.git | — |
| `apps/yral-backend-canister/` | git@github.com:dolr-ai/yral-backend-canister.git | — |
| `apps/yral-common/` | git@github.com:dolr-ai/yral-common.git | — |
| `apps/website/` | git@github.com:dolr-ai/website.git | — |
| `apps/yral-billing/` | git@github.com:dolr-ai/yral-billing.git | — |
| `apps/yral-metadata/` | git@github.com:dolr-ai/yral-metadata.git | — |
| `apps/off-chain-agent/` | git@github.com:dolr-ai/off-chain-agent.git | — |

## In-repo applications (not submodules)

| App | Deployed at |
|-----|-------------|
| `apps/yral-auth/` | `auth.yral.com` |
| `apps/yral-legacy/` | `legacy.yral.com` |
