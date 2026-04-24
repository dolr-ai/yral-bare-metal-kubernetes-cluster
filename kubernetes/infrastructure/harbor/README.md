# Harbor Container Registry Setup

Harbor v1.18.3 is the self-hosted private container registry for temporary applications without external GitHub repositories.

## Configuration

### Secrets (SOPS-encrypted)

Before deploying Harbor, update the encrypted Harbor admin secret:

```bash
sops kubernetes/infrastructure/harbor/harbor-admin-secret.sops.yaml
```

**Required value:**
- `admin-password`: Harbor admin account password (user: `admin`)

Harbor oauth2-proxy uses the existing shared secret in `kubernetes/infrastructure/oauth2-proxy/secret.sops.yaml`.
For Harbor access, only add the redirect URI `https://harbor.yral.com/oauth2/callback` to the existing Google OAuth app.

### Deployment

Harbor is deployed by the Flux Kustomization `infrastructure-harbor` in `kubernetes/clusters/yral-k8s/kustomization.yaml`.

```bash
# Check deployment status
kubectl get -n harbor statefulset,deployment
kubectl logs -n harbor deploy/harbor-core
kubectl logs -n oauth2-proxy deploy/oauth2-proxy-harbor
```

### Access

**Hostname:** `harbor.yral.com` (exposed via Gateway API with oauth2-proxy authentication)

**Login:**
1. Navigate to https://harbor.yral.com
2. Redirect through Google OAuth (configured for yral.com domain)
3. Create local projects or use default `library` project

**Docker CLI login:**
```bash
docker login harbor.yral.com
# Prompts for username (admin) and password (use admin-password from secrets)
```

## Building and Pushing Images

### For temporary validation apps (no external GitHub repo):

```bash
# 1. Build the image from Dockerfile
docker build -t <app-name>:v0.1.0 apps/<app-name>/

# 2. Log in to Harbor
docker login harbor.yral.com

# 3. Tag for Harbor registry
docker tag <app-name>:v0.1.0 harbor.yral.com/library/<app-name>:v0.1.0

# 4. Push to Harbor
docker push harbor.yral.com/library/<app-name>:v0.1.0

# 5. Reference the image in Kubernetes deployment manifests:
# image: harbor.yral.com/library/<app-name>:v0.1.0
```

## Cleanup Policy

When a temporary validation app is no longer needed:

1. Remove the app namespace, Deployment, Service, and HTTPRoute from git
2. Delete the image from Harbor's web UI or via API
3. Remove any oauth2-proxy configuration entries for that app
4. Commit changes with message: `cleanup: remove <app-name> validation app after verification`

Example cleanup after validation:
```bash
# Remove from git (after validation is complete)
rm -rf kubernetes/apps/<app-name>/
git rm kubernetes/networking/routes/<app-name>.yaml
# Update kubernetes/networking/routes/kustomization.yaml to remove <app-name>.yaml
# Commit and push — Flux will reconcile and delete resources
```

## Troubleshooting

**Harbor pod stuck in CrashLoopBackOff:**
- Check logs: `kubectl logs -n harbor deploy/harbor-core`
- Common issues: storage not ready, secret missing, permissions

**oauth2-proxy fails to authenticate:**
- Verify shared OAuth credentials in `kubernetes/infrastructure/oauth2-proxy/secret.sops.yaml`
- Check oauth2-proxy logs: `kubectl logs -n oauth2-proxy deploy/oauth2-proxy-harbor`
- Ensure redirect URL is registered in Google Cloud OAuth app settings

**Cannot push images:**
- Ensure docker login succeeded: `docker login harbor.yral.com`
- Check network connectivity to harbor.yral.com
- Verify image tag format: `harbor.yral.com/library/image:tag`

## Storage

Harbor uses ceph-block StorageClass for persistence:
- Registry data: 50Gi
- Job service: 10Gi
- Internal database: 10Gi
- Redis: 5Gi

Monitor usage with:
```bash
kubectl get pvc -n harbor
```

If storage approaches capacity, scale up PVC size in `kubernetes/infrastructure/harbor/helmrelease.yaml`.
