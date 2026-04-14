# DevPod Onboarding

This cluster exposes a `devpod` namespace with pre-configured RBAC and resource quotas for development workspaces.

## Prerequisites

- [DevPod CLI](https://devpod.sh/docs/getting-started/install) or DevPod Desktop installed
- A kubeconfig with access to the cluster (or the shared service account token — see cluster admin)

## Initial Setup

Configure the Kubernetes provider once:

```sh
devpod provider add kubernetes
devpod provider configure kubernetes \
  --option KUBERNETES_NAMESPACE=devpod \
  --option INACTIVITY_TIMEOUT=60m \
  --option DISK_SIZE=100Gi \
  --option RESOURCES="requests.cpu=6,requests.memory=24Gi,limits.cpu=6,limits.memory=24Gi" \
  --option KUBERNETES_PULL_SECRETS_ENABLED=true \
  --option POD_TIMEOUT=10m \
  --option STRICT_SECURITY=false
```

### INACTIVITY_TIMEOUT — Required

`INACTIVITY_TIMEOUT` **must** be set. When configured, the DevPod agent injected into the workspace pod monitors the SSH/tunnel connection. After the specified duration with no active connection, the agent deletes only the pod — the PVC remains intact. The next `devpod up` rebinds to the existing PVC allowing a fast restart.

Without this setting pods run indefinitely, consuming CPU and memory even when you are not connected.

Recommended value: `60m` (adjust to taste — `10m` is the minimum practical value).

## Creating a Workspace

```sh
devpod up <git-repo-url> --provider kubernetes --ide vscode
```

## Lifecycle

| Action | Command |
|--------|---------|
| Connect to workspace | `devpod up <name>` |
| Stop workspace pod (PVC kept) | `devpod stop <name>` |
| Delete workspace completely | `devpod delete <name>` |
| List workspaces | `devpod list` |

## Storage and Scheduling

Workspace PVCs are provisioned by Ceph (`ceph-block`). Pods can restart on any worker node, and the volume is remounted by the RBD CSI driver.

## Resource Quotas

The `devpod` namespace has a `ResourceQuota` (`devpod-quota`) enforced cluster-side. If workspace creation fails with a quota error, contact the cluster admin to review limits.
