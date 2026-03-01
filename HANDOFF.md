# Agent Handoff

## Session Summary

Productive session. Fixed two operational issues and cleaned up one-off playbooks.

## Completed This Session

### 1. DNSConfigForming warnings — FIXED (commit `66a02d7`)
- **Root cause**: Hetzner netplan ships 4 nameservers (2x IPv4 + 2x IPv6). kubeadm sets kubelet `--resolv-conf=/run/systemd/resolve/resolv.conf`, which reflects all 4. kubelet prepends CoreDNS → 5 total → Linux/glibc truncates to 3 → `DNSConfigForming` on every pod.
- **Fix**: `resolv-cleanup` role removes the 2 redundant Hetzner IPv6 nameservers (`2a01:4ff:ff00::add:1`, `2a01:4ff:ff00::add:2`) from netplan, then runs `netplan apply` + restarts `systemd-resolved`. All 40 nodes now have exactly 2 upstream nameservers (`185.12.64.1`, `185.12.64.2`).
- **Baked in**: `resolv-cleanup` role added after `base-system` in `init-control-plane.yml`, `add-control-plane.yml`, `add-worker.yml` — all future nodes get it automatically.
- **Status**: Fix deployed. Stale warning events (152) are aging out; no new ones being generated. They will fully clear as DaemonSet pods recycle over the next few hours.

### 2. One-off playbook cleanup
- Deleted `ansible/playbooks/operations/fix-node-resolv.yml` (DNS fix applied to all nodes)
- Deleted `ansible/playbooks/operations/tune-node-sysctls.yml` (sysctl tuning applied to all 40 nodes, baked into provisioning playbooks)

## Current Cluster State

- Kubernetes v1.35, 5 control planes (HEL1-DC2/3/4/6/7), 35 workers
- Cilium 1.19.1, Flux GitOps, Longhorn CSI, Snowplow → Kafka → S3 pipeline active
- Headlamp UI live at https://headlamp.yral.com (behind oauth2-proxy, Google SSO, auto-login)
- S3 bucket `yral-events-data-lake` on Hetzner FSN1 — pipeline healthy, awaiting real app events

## Pending TODO Items (from TODO.md)

Work through these in order:

### 1. Add Capacitor UI for Flux
- Weave GitOps (formerly Capacitor) is the standard Flux UI — https://github.com/weaveworks/weave-gitops
- Deploy pattern: same as Headlamp — HelmRelease in `kubernetes/infrastructure/`, oauth2-proxy in front, HTTPRoute at e.g. `gitops.yral.com`
- Check latest chart version before deploying: https://github.com/weaveworks/weave-gitops/releases
- Ask user which version to pin before writing any code (per AGENTS.md versioning policy)

### 2. Remove Goldilocks
- User confirmed it's not useful
- Files to remove: `kubernetes/infrastructure/goldilocks/` (entire directory), `kubernetes/clusters/yral-k8s/infrastructure-goldilocks.yaml`
- Remove from `kubernetes/clusters/yral-k8s/kustomization.yaml` if referenced there
- Check for any HTTPRoute or oauth2-proxy resources for Goldilocks in `kubernetes/networking/routes/` and `kubernetes/infrastructure/oauth2-proxy/`
- Commit and push — Flux `prune: true` will clean up cluster resources

### 3. Fix weekly system update failing
- Source unknown — check `unattended-upgrades` logs on nodes and/or any Kubernetes CronJob
- Investigate: `ssh root@<node> 'journalctl -u unattended-upgrades --since "7 days ago" | tail -50'`
- May be related to apt lock contention with kubelet or containerd updates

### 4. Integrate Snowplow SDK into yral-mobile (Android/Kotlin)
- No Snowplow SDK installed yet in the mobile app
- Collector endpoint: `https://events.yral.com`
- Official Snowplow Android SDK: https://github.com/snowplow/snowplow-android-tracker
- Gradle dependency: `com.snowplowanalytics.snowplow:snowplow-android-tracker:<version>`
- Standard setup: initialize tracker with collector URL, send structured/self-describing events

## Key Infrastructure Details

| Item | Value |
|------|-------|
| S3 bucket | `yral-events-data-lake`, endpoint `fsn1.your-objectstorage.com` |
| S3 access key | `XO5X9A1W8AMHY3DSTKMS` (secret in vault as `vault_hetzner_s3_secret_key`) |
| Snowplow collector | `https://events.yral.com` |
| Kafka topic (raw) | `snowplow-raw` |
| Kafka topic (enriched) | `snowplow-enriched` |
| Headlamp | `https://headlamp.yral.com` |
| Cluster API | `https://kubernetes-api.yral.com` |
| Flux receiver | `https://flux-receiver.yral.com` |
| oauth2-proxy namespace | `oauth2-proxy` |
| Shared OAuth secret | `oauth2-proxy-secrets` in `oauth2-proxy` ns |
