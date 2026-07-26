# apps/

This directory contains application source code. Some apps are **git submodules** (separate repos pinned here), while others are **in-repo directories** (native to this cluster repo).

## Relationship to `kubernetes/apps/`

| Directory                     | Contents                                             |
| ----------------------------- | ---------------------------------------------------- |
| `apps/<app-name>/`            | Application source code (submodule or in-repo)       |
| `kubernetes/apps/<app-name>/` | Kubernetes deployment manifests for that application |

Source code and deployment manifests are intentionally separated. Flux reconciles from `kubernetes/apps/` independently of the submodule checkout state.

## Adding a new application

### As a submodule (separate repo)

```bash
# 1. Add the submodule (SSH recommended)
git submodule add git@github.com:<org>/<repo-name>.git apps/<repo-name>
git add apps/<repo-name> .gitmodules
git commit -m "feat: add <repo-name> submodule"
```

### As an in-repo directory

```bash
mkdir apps/<app-name>
# Create Cargo.toml, Dockerfile.buildkit, etc.
# Add to Cargo.toml root workspace exclude list
```

## Populating submodules after a fresh clone

```bash
git submodule update --init --recursive
```

## Current submodules

| Submodule                          | Source repo                                           |
| ---------------------------------- | ----------------------------------------------------- |
| `apps/yral/`                       | git@github.com:dolr-ai/yral.git                       |
| `apps/yral-mobile/`                | git@github.com:dolr-ai/yral-mobile.git                |
| `apps/yral-backend-canister/`      | git@github.com:dolr-ai/yral-backend-canister.git      |

| `apps/website/`                    | git@github.com:dolr-ai/website.git                    |
| `apps/my-website/`                 | https://github.com/saikatdas0790/my-website.git       |

## In-repo applications (not submodules)

| App                             | Deployed at                                                 |
| ------------------------------- | ----------------------------------------------------------- |
| `apps/yral-auth/`               | `auth.yral.com`                                             |
| `apps/yral-legacy/`             | `legacy.yral.com`                                           |
| `apps/yral-metadata/`           | `metadata.yral.com`                                         |
| `apps/off-chain-agent/`         | `offchain.yral.com`                                         |
| `apps/yral-common/`             | Shared library crates (consumed via git deps by other apps) |
| `apps/task-runner/`             | Workspace member (root Cargo.toml)                          |
| `apps/yral-database-spacetime/` | Workspace member (root Cargo.toml)                          |
