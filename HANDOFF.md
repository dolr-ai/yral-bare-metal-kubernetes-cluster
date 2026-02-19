# Deployment Handoff

> **Purpose**: This file captures in-progress deployment state for resuming work in a new environment.
> **Delete this file** once the deployment is fully complete and all nodes are healthy.

---

## Current Objective

Bootstrap the first Kubernetes control plane (`control-plane-1`) by running `init-control-plane.yml` end-to-end, then add the remaining control planes and workers.

**Active playbook**: `ansible/playbooks/operations/init-control-plane.yml`

---

## Node State as of Handoff (2026-02-19)

### control-plane-1 — `95.216.228.60` — **PARTIALLY INITIALIZED**

| Component | Status |
|---|---|
| Ubuntu 24.04 | ✅ installed and running |
| SSH hardening | ✅ key-only auth |
| btrfs storage | ✅ expanded |
| containerd | ✅ running |
| kubeadm / kubelet / kubectl v1.35.1 | ✅ installed and held |
| kube-vip static pod manifest | ❌ NOT written yet |
| kubeadm init | ⚠️ ran but FAILED partway (partial state on disk) |
| admin.conf | ⚠️ EXISTS at `/etc/kubernetes/admin.conf` (from failed run) |
| Cluster healthy | ❌ NO — API server unreachable via VIP |

**Why it failed**: `kubeadm init` was run without kube-vip deployed first. kubeadm's `upload-config` phase calls `https://kubernetes-api.yral.com:6443` (the VIP `77.42.49.55`), which timed out because nothing was bound to the VIP yet.

**What will happen on next run**: The `cluster-init` role now detects this partial state. It probes port 6443 on the node IP (`95.216.228.60`), finds the API server unreachable (because the failed partial init left orphaned static pod manifests but no healthy cluster), runs `kubeadm reset --force`, then re-runs `kubeadm init` — this time with the kube-vip manifest already in place.

### control-plane-2, 3, 4, 5 — **NOT STARTED**
### All worker nodes — **NOT STARTED**

---

## What to Do Next

### Step 1 — Re-run init-control-plane.yml

```bash
cd /workspaces/yral-bare-metal-kubernetes-cluster
ansible-playbook ansible/playbooks/operations/init-control-plane.yml
```

The playbook is fully idempotent. It will:
1. **Skip** all already-complete steps (provision, storage, SSH, base-system, containerd, kubernetes) — those roles have idempotency guards
2. **Write** kube-vip static pod manifest (`kube-vip` role, `tasks_from: pre-init`)
3. **Detect** the partial kubeadm init via the API server probe in `cluster-init`
4. **Reset** the partial init with `kubeadm reset --force`
5. **Run** `kubeadm init` fresh — kube-vip will be running alongside the API server, VIP will be reachable before upload-config phase
6. **Apply** kube-vip RBAC/ConfigMap (`kube-vip` role, `tasks_from: post-init`)

### Step 2 — Verify control-plane-1 is healthy

```bash
ssh -i ~/.ssh/hetzner-ansible-key root@95.216.228.60 \
  "kubectl --kubeconfig=/etc/kubernetes/admin.conf get nodes"
```

Expected: `control-plane-1   Ready   control-plane`

### Step 3 — Add remaining control planes (HA)

The cluster requires an odd number of control planes. Add them one at a time:

```bash
ansible-playbook ansible/playbooks/operations/add-control-plane.yml -e target_host=control-plane-2
ansible-playbook ansible/playbooks/operations/add-control-plane.yml -e target_host=control-plane-3
```

(Add control-plane-4 and 5 only if 5 control planes are desired. 3 is sufficient for HA quorum.)

### Step 4 — Deploy Cilium CNI

```bash
ansible-playbook ansible/playbooks/cilium-deploy.yml
```

Nodes will be `NotReady` until Cilium is deployed (no CNI).

### Step 5 — Add worker nodes

```bash
ansible-playbook ansible/playbooks/operations/add-worker.yml -e target_host=<worker-name>
```

---

## Prerequisites for New Environment

Before running any playbook in a new dev environment:

### 1. SSH key must exist

```bash
ls ~/.ssh/hetzner-ansible-key
```

If missing, extract from vault:
```bash
# The postCreate.sh script handles this — check if it ran:
cat ansible/.vault_pass   # must exist and be non-empty
```

Manually extract if needed:
```bash
ansible-vault view ansible/inventory/group_vars/all/vault.yml
# Copy vault_github_actions_ssh_private_key value to ~/.ssh/hetzner-ansible-key
chmod 600 ~/.ssh/hetzner-ansible-key
```

### 2. Vault password file must exist

```bash
cat ansible/.vault_pass   # must be non-empty
```

### 3. Verify vault variables load correctly

```bash
ansible -m debug -a "var=hetzner_robot_api_password" control-plane-1
# Must return the actual password, not "{{ vault_hetzner_robot_api_password }}"
```

### 4. Verify SSH connectivity

```bash
ssh -i ~/.ssh/hetzner-ansible-key root@95.216.228.60 "hostname"
# Expected: control-plane-1
```

If you get `WARNING: REMOTE HOST IDENTIFICATION HAS CHANGED`, clear the old key:
```bash
ssh-keygen -R 95.216.228.60
```

---

## Architecture of the Fix Applied This Session

### Problem
`kubeadm init` uses `--control-plane-endpoint=kubernetes-api.yral.com:6443` (the VIP). Its `upload-config` phase POSTs to the VIP *while still initializing*. kube-vip wasn't deployed yet, so the VIP was unreachable → timeout.

### Solution — kube-vip split into pre-init / post-init task files

The `kube-vip` role now has three task files:

| File | When | What |
|---|---|---|
| `tasks/pre-init.yml` | Before `kubeadm init` | Writes static pod manifest to `/etc/kubernetes/manifests/kube-vip.yaml` |
| `tasks/post-init.yml` | After `kubeadm init` | Applies RBAC + ConfigMap via kubectl |
| `tasks/main.yml` | Used by `add-control-plane` | Includes both (cluster already running) |

`init-control-plane.yml` role chain:
```
provision → storage-setup → ssh-hardening → base-system → containerd → kubernetes
→ kube-vip (tasks_from: pre-init)    ← writes manifest; kubelet starts kube-vip during init
→ cluster-init                        ← kubeadm init; VIP reachable before upload-config
→ kube-vip (tasks_from: post-init)   ← applies RBAC/ConfigMap after cluster is up
```

### Partial-init idempotency in cluster-init

If `admin.conf` exists from a previous failed run, the `cluster-init` role:
1. Probes port 6443 on the **node IP** (not VIP) with a 10-second timeout
2. If unreachable → `kubeadm reset --force` + removes stale `admin.conf`
3. Re-runs `kubeadm init` cleanly

---

## Files Changed This Session

| File | Change |
|---|---|
| `ansible/roles/kube-vip/tasks/pre-init.yml` | **NEW** — manifest generation only |
| `ansible/roles/kube-vip/tasks/post-init.yml` | **NEW** — RBAC + ConfigMap apply |
| `ansible/roles/kube-vip/tasks/main.yml` | **UPDATED** — now includes pre-init + post-init |
| `ansible/roles/cluster-init/tasks/main.yml` | **UPDATED** — partial-init reset logic; removed kube-vip RBAC tasks |
| `ansible/playbooks/operations/init-control-plane.yml` | **UPDATED** — added kube-vip pre/post-init in role chain |
| `ansible/roles/provision/tasks/main.yml` | **UPDATED** (prior session) — `delegate_to: localhost` on all tasks; vault indirection var fix |
| `ansible/inventory/group_vars/` | **MOVED** (prior session) — from `ansible/group_vars/` to `ansible/inventory/group_vars/` |
| `ansible.cfg` | **UPDATED** (prior session) — comment added explaining group_vars location |
| `AGENTS.md` | **UPDATED** (prior session) — Deployment Execution Principle, vault indirection note, upgrade playbook split |

---

## Key Variables

| Variable | Value | Source |
|---|---|---|
| `control_plane_vip` | `77.42.49.55` | `hosts.yml` |
| `control_plane_endpoint` | `kubernetes-api.yral.com` | `hosts.yml` |
| `pod_network_cidr` | `10.244.0.0/16` | `hosts.yml` |
| `kube_vip_version` | `v0.8.5` | `roles/kube-vip/defaults/main.yml` |
| `hetzner_robot_api_user` | `#ws+hEJX77Pr` | `hosts.yml` |
| `hetzner_robot_api_password` | (vault) | `inventory/group_vars/all/vault.yml` |

DNS `kubernetes-api.yral.com → 77.42.49.55` ✅ resolves correctly.

---

*Generated: 2026-02-19*
