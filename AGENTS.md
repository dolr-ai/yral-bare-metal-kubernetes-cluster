# Agent Guidelines for Kubernetes Cluster Operations

This document defines architectural patterns and constraints specific to this repository. Follow these guidelines to maintain consistency and code quality.

## Core Architecture Principles

### 1. Immutable Infrastructure
- All operations must be additive or subtractive, never mutating existing nodes
- Adding/removing/upgrading nodes doesn't modify any other nodes
- Decommissioned nodes are removed and replaced, not repaired in-place
- Day-2 operations follow the same immutability principle as Day-1 setup

**Nodes are never patched in place. If a node is misconfigured or missing a component, it is reprovisioned from scratch — not fixed on top.**
Examples of what this means in practice:
- A control plane initialized without CNI is NOT fixed by running a CNI-only playbook on top of it. It is torn down and re-initialized with the corrected `init-control-plane.yml`.
- A node that got stuck mid-provisioning is NOT recovered by SSHing in and running the missing commands. The playbook is re-run from the start, which wipes and reinstalls.
- There is no "partial apply" or "resume from step N" workflow. Every node reaches its final state through a single clean run of its full provisioning playbook.

**No "already done" skip guards in provisioning roles.**
Roles like `provision` and `storage-setup` run unconditionally every time — they do not check whether the OS is already installed or the disk already configured and skip if so. A clean slate on every run is the guarantee. Shortcuts that detect existing state and bypass destructive steps shorten the feedback loop during debugging but silently break the clean-slate contract; do not add them.

The only legitimate guards in roles are:
- **Correctness guards**: prevent a command from erroring when it would be a no-op (e.g., don't run `btrfs device add` if the device is already in the filesystem).
- **Safety nets**: detect partial failures and recover (e.g., `kubeadm reset` when `admin.conf` exists but the API server is unreachable).

### 2. Role-Based Architecture
- **Mandatory**: ALL business logic must live in roles
- **Mandatory**: Roles must be atomic (single responsibility, no orchestration logic)
- **Mandatory**: Roles NEVER call other roles (no `include_role` within roles)
- **Mandatory**: Role orchestration (chaining multiple roles) happens ONLY in playbooks
- **Mandatory**: ALL playbooks must be thin wrappers (single play)
- **Mandatory**: Single play per playbook (no multiple plays)
- **Allowed**: Multiple role entries within the single play (playbooks can chain roles in sequence)
- **Allowed**: A minimal `tasks:` section containing **only** `include_role` calls with `tasks_from` — no other modules, logic, or variables. This lets a single role expose multiple task files (e.g. `pre-init.yml`, `post-init.yml`) without duplicating the role.
- **Pattern**: Playbooks orchestrate atomic role execution, roles contain only their specific logic

**Valid playbook structures (single play, roles-only or thin tasks):**
```yaml
---
- name: Operation description
  hosts: target
  gather_facts: true/false
  become: true/false
  serial: 1
  
  roles:
    - role: task-role-1
    - role: task-role-2
    - role: task-role-3
```

```yaml
# Also valid: include_role with tasks_from (thin wrapper only)
  tasks:
    - ansible.builtin.include_role:
        name: my-role
        tasks_from: pre-init
    - ansible.builtin.include_role:
        name: other-role
    - ansible.builtin.include_role:
        name: my-role
        tasks_from: post-init
```

**Note:** `tasks_from` does NOT work in the `roles:` list — it is silently ignored. Use `include_role` in a `tasks:` section instead.

**Invalid patterns:**
- ❌ Multiple plays in a playbook
- ❌ Any modules in `tasks:` other than `include_role` (no shell, command, debug, copy, etc.)
- ❌ Direct shell commands or modules in playbooks
- ❌ Conditional logic in playbooks (when:, if statements)
- ❌ Loops in playbooks (with_items, loop:)
- ❌ Variables defined inline in playbooks

### 3. Playbook Organization

#### Operations Playbooks (`ansible/playbooks/operations/`)
Used for cluster Day-2 operations. Each operation is a single play calling one role.

**Current operations** (must remain pure thin wrappers chaining atomic roles):
1. `init-control-plane.yml` - Bootstrap first control plane (hardcoded to control-plane-1, chains: provision → storage-setup → ssh-hardening → base-system → containerd → kubernetes → cluster-init → node-labels → helm → gateway-api-crds → cilium)
2. `add-control-plane.yml` - Add control plane for HA (chains: provision → storage-setup → ssh-hardening → base-system → containerd → kubernetes → control-plane-join → node-labels). Cilium DaemonSet deploys automatically via the existing installation.
3. `add-worker.yml` - Add worker node (chains: provision → storage-setup → ssh-hardening → base-system → containerd → kubernetes → worker-join → node-labels)
4. `remove-node.yml` - Remove node from cluster (calls: `node-remove` role)
5. `upgrade-control-plane.yml` - Upgrade control plane node(s): cordon → drain → kubeadm reset → reboot → rejoin → verify. Accepts `-e target_host=<node>` for single node or runs all serially.
6. `upgrade-worker.yml` - Upgrade worker node(s): cordon → drain → delete → reboot → rejoin → verify. Accepts `-e target_host=<node>` for single node or runs all serially.

**There is no "partial install" or "apply missing component" playbook.** If a node is missing something that should have been installed during init (e.g. CNI, a system package), the correct fix is to re-run its full provisioning playbook from scratch, not to create a targeted playbook that applies only the missing piece.

**Adding new playbooks requires explicit user confirmation.** Before creating any new playbook:
1. Check whether the new functionality fits into an existing playbook by adding a role to it.
2. If a new playbook is genuinely needed, propose it to the user and wait for explicit agreement before creating it.
3. Never create a new playbook unilaterally — not even as a "convenience" or "one-off" wrapper.

When adding new capability, the default path is: **create a new role → add it to an existing playbook**. A new playbook is only justified when the operation is structurally distinct from all existing ones (different host targeting, different lifecycle stage, etc.).

#### Utility Playbooks (`ansible/playbooks/`)
Used for cluster-wide utilities. Can have limited logic if orchestrating multiple roles for a single operation.
- `lint-check.yml` - YAML validation only

### 4. Atomic Task Roles

#### Role Responsibilities
- **Purpose**: Single, well-defined operational task
- **Example**: `provision`, `storage-setup`, `ssh-hardening`, `base-system`, `containerd`, `kubernetes`, `cluster-init`, `node-upgrade`
- **Contains**: Direct Ansible modules, commands, configuration only
- **Does NOT contain**: `include_role` calls, orchestration logic, multi-step workflows

#### Role Atomicity Rules
- Each role must be independently executable
- Roles must not call other roles
- Role orchestration happens in playbooks, not in roles
- Conditional logic specific to role functionality is allowed (e.g., check if reboot needed)
- Orchestration conditionals that decide between roles belong in playbooks

### 5. Role Naming Conventions
- Use hyphens in role names: `my-role-name` (not `my_role_name`)
- Task roles: simple descriptive names like `provision`, `containerd`, `kubernetes`, `node-upgrade`
- No special naming convention needed—all roles are atomic and task-focused

### 6. Playbook-Role Relationship Pattern

Playbooks orchestrate sequences of atomic task roles:

```
Playbook (thin wrapper, single play)
├→ Task Role 1 (provision)
├→ Task Role 2 (storage-setup)
├→ Task Role 3 (ssh-hardening)
├→ Task Role 4 (base-system)
├→ Task Role 5 (containerd)
├→ Task Role 6 (kubernetes)
└→ Task Role 7 (cluster-init)
```

No intermediate orchestrator roles between playbook and task roles.

### 7. Deployment Workflow for New Nodes

**Full initialization flow** (applies to control planes and workers):
1. **Provision** - Hetzner API: rescue mode → install Ubuntu
2. **Storage** - Expand btrfs/setup disk configuration
3. **SSH Hardening** - Disable password auth, key-only access
4. **Base System** - Apt updates, unattended-upgrades, reboot if needed
5. **Containerd** - Install container runtime
6. **Kubernetes** - Install kubeadm, kubelet, kubectl
7. **Cluster Init/Join** - Initialize or join cluster
8. **CNI/Add-ons** - Deploy network plugins, apply labels

**Reboot behavior**:
- `base-system` role checks `/var/run/reboot-required`
- If present: automatic reboot with timeout handling
- If absent: skip reboot, continue

### 8. Variable Management

- **Global vars**: `ansible/group_vars/all/vars.yml` (plaintext + vault indirection)
- **Secrets**: `ansible/group_vars/all/vault.yml` (encrypted)
- **Role defaults**: `roles/role-name/defaults/main.yml`
- **Runtime vars**: Passed via `-e target_host=node-name` (not in playbooks)

**Variable Naming Convention:**
- Plaintext variables: lowercase with underscores (`hetzner_robot_api_user`)
- Vault-backed variables: `vault_` prefix (`vault_hetzner_robot_api_password`) - defined in vault.yml
- In vars.yml: Create indirection mappings to document the vault schema (reference vault variables with comments)
- In roles/tasks: Reference vault variables directly (vault variables auto-loaded via ansible.cfg vault_password_file)
- This allows operators to see vault schema in vars.yml without decrypting vault.yml

**Benefits of this pattern:**
- Tasks reference vault variables directly (no indirection lookup overhead)
- vars.yml documents which vault secrets exist in vault.yml (shows the mapping)
- Operators can discover what secrets are defined without decrypting: grep for `vault_` in vars.yml
- Vault variables automatically available (ansible.cfg vault_password_file handles loading)
- No need for pre_tasks in playbooks - vault is always accessible
- Single source of truth: vault.yml has secrets, vars.yml documents schema

**Example in vars.yml:**
```yaml
# Plaintext values
hetzner_robot_api_user: "#ws+hEJX77Pr"

# Indirection/documentation - shows what vault secrets exist
hetzner_robot_api_password: "{{ vault_hetzner_robot_api_password }}"
hetzner_s3_secret_key: "{{ vault_hetzner_s3_secret_key }}"
```

**Important**: In role task args, always reference the indirection variable (e.g. `hetzner_robot_api_password`) rather than the raw `vault_` variable directly. Ansible evaluates task args before vault group_vars are merged when the `vault_` variable is used directly, causing "undefined" errors. The indirection variable in `vars.yml` is resolved correctly at play startup.

**Example in role tasks:**
```yaml
- name: Call Hetzner API
  ansible.builtin.uri:
    url: "https://robot-ws.your-server.de/key"
    user: "{{ hetzner_robot_api_user }}"          # Plaintext from vars.yml
    password: "{{ vault_hetzner_robot_api_password }}"  # Vault variable (auto-loaded)
```

**Ansible.cfg configuration:**
```ini
vault_password_file = ansible/.vault_pass
```
This enables automatic loading of vault variables without pre_tasks.

### 9. Kubernetes Secrets Management (SOPS + Flux)

Kubernetes secrets that belong to cluster workloads (e.g. `cloudflare-api-token` for cert-manager) are managed via **SOPS-encrypted files committed to git**. Flux decrypts them at apply time using an age keypair.

**Two-tier secret strategy:**

| Secret | Stored in | Materialized by |
|--------|-----------|-----------------|
| Ansible/infra secrets (SSH keys, API passwords, kubeconfig, GitHub PAT) | `ansible-vault` (`vault.yml`) | `postCreate.sh` |
| Age private key (Flux decryption root of trust) | `ansible-vault` (`vault.yml`) | `postCreate.sh` → `sops-age` Secret in `flux-system` |
| Kubernetes workload secrets (cloudflare token, etc.) | SOPS-encrypted `*.sops.yaml` in `kubernetes/` | Flux (decrypts via `sops-age` key) |

**Why this split:**
- Ansible vault protects infrastructure-level secrets (pre-cluster-API)
- SOPS protects cluster workload secrets (post-cluster-API, fully GitOps)
- `postCreate.sh` only handles the bridge: it materializes the age key into the cluster so Flux can take over

**SOPS setup (one-time per cluster):**
```bash
# 1. Generate age keypair
age-keygen -o /tmp/age.key
# Output includes: # public key: age1...

# 2. Copy public key into .sops.yaml (replace placeholder)

# 3. Add private key to ansible vault
ansible-vault edit ansible/inventory/group_vars/all/vault.yml
# Add: vault_age_private_key: |
#        # created: ...
#        # public key: age1...
#        AGE-SECRET-KEY-...

# 4. Remove temp key file
rm /tmp/age.key

# 5. Source postCreate.sh to apply sops-age secret to cluster
source .devcontainer/postCreate.sh
```

**Adding a new Kubernetes secret (SOPS workflow):**
```bash
# 1. Create the secret YAML (must be named *.sops.yaml)
cat > kubernetes/path/to/my-secret.sops.yaml << 'EOF'
apiVersion: v1
kind: Secret
metadata:
  name: my-secret
  namespace: my-namespace
type: Opaque
stringData:
  key: value
EOF

# 2. Encrypt it (requires .sops.yaml to have correct public key)
sops --encrypt --in-place kubernetes/path/to/my-secret.sops.yaml

# 3. Add to the directory's kustomization.yaml resources list

# 4. Add decryption block to the Flux Kustomization that applies this directory
```

**Flux Kustomization decryption block:**
```yaml
spec:
  decryption:
    provider: sops
    secretRef:
      name: sops-age   # Secret in flux-system namespace
```

**Rules:**
- `*.sops.yaml` files MUST be encrypted before committing — never commit plaintext secrets
- Only the Flux Kustomization that applies the directory containing encrypted files needs the `decryption:` block
- The `sops-age` Secret in `flux-system` is the single root of trust; without it Flux cannot decrypt
- The age private key lives ONLY in ansible-vault and the ephemeral `sops-age` cluster secret — never in git

### 10. Lint and Validation

- Run from workspace root: `ansible-lint ansible/playbooks/operations/`
- All playbooks must pass lint checks
- Excluded rules (in `.ansible-lint`): 
  - `role-name` (hyphenated names allowed)
  - `syntax-check[specific]` (for `{{ target_host }}` runtime variables)

### 11. Error Handling and Validation

In roles:
- Use `ansible.builtin.fail` with clear messages
- Use `ansible.builtin.assert` for precondition checks
- Use `changed_when: false` for read-only operations
- Use `failed_when: false` + explicit checks for operations that might fail

Avoid in playbooks:
- Conditional logic
- Error handling
- Task registration and checking

## Checklist Before Approving Changes

When reviewing playbooks:
- [ ] All playbooks are single-play thin wrappers
- [ ] Zero `tasks:` sections in playbooks
- [ ] Zero conditional logic in playbooks (orchestration only)
- [ ] Zero loops in playbooks (orchestration only)
- [ ] Playbook calls atomic task roles in sequence
- [ ] Zero `include_role` calls in any roles (atomicity enforced)
- [ ] All operational logic in atomic task roles
- [ ] Operations follow immutability principle
- [ ] `base-system` role includes reboot detection logic
- [ ] Role-specific conditionals only (not orchestration)
- [ ] Playbook passes ansible-lint validation
- [ ] Documentation reflects the actual workflow

When reviewing roles:
- [ ] Role has only `include_role` in names/descriptions, not implementation
- [ ] Role contains zero orchestration logic
- [ ] Role does not call other roles (no `include_role` in tasks)
- [ ] Role is independently executable
- [ ] Role has clear single responsibility

## Common Patterns

### Adding a New Operation

1. Create/reuse atomic task roles: `roles/task-name/`
   - Each role handles one specific concern
   - `tasks/main.yml` - role logic only
   - `meta/main.yml` - role metadata (if needed)

2. Create thin playbook: `playbooks/operations/new-operation.yml`
   ```yaml
   ---
   - name: Operation description
     hosts: target
     gather_facts: true/false
     become: true/false
     serial: 1
     
     roles:
       - role: task-role-1
       - role: task-role-2
       - role: task-role-3
   ```

3. Run lint: `ansible-lint ansible/playbooks/operations/new-operation.yml`

### Chaining Multiple Roles in Playbooks

In playbook (NOT in roles):
```yaml
roles:
  - name: Step 1: Provision
    role: provision
  
  - name: Step 2: Setup storage
    role: storage-setup
  
  - name: Step 3: Harden SSH
    role: ssh-hardening
```

Note: Direct role entries in playbooks (not indented under `tasks:` or orchestrator roles)

### Handling Reboots

In `base-system` role (role-level logic only):
```yaml
- name: Check if reboot required
  ansible.builtin.stat:
    path: /var/run/reboot-required
  register: reboot_required

- name: Reboot if needed
  when: reboot_required.stat.exists
  ansible.builtin.reboot:
    msg: "Rebooting for kernel updates"
    reboot_timeout: 600
```

Multi-node upgrade orchestration (happens in playbook via `node-upgrade` role):
- For cluster-wide upgrades, the playbook calls `node-upgrade` role per target node
- The `node-upgrade` role handles single-node upgrade logic: drain → remove → upgrade → reboot → rejoin → verify
- Playbook controls loop across all nodes

## Component Versioning Policy

**Always ask the user which version to pin before adding or upgrading any component.**

When introducing a new component or upgrading an existing one:
1. Identify the component's releases page (e.g. `https://github.com/<org>/<repo>/releases`).
2. Share the releases URL with the user and ask which version to pin.
3. Wait for an explicit version number before writing any code or defaults.
4. Pin exactly that version in `roles/<role>/defaults/main.yml`.

Never silently pick "latest" or assume the most recent release — always get explicit user confirmation first.

- Only downgrade to an older version if the latest is confirmed incompatible with the current Kubernetes version, and document the reason in the role's `defaults/main.yml`.
- During Kubernetes version upgrades, upgrade other components **one at a time**, following each component's own upgrade guidance before moving to the next.
- Version pins live exclusively in `roles/<role>/defaults/main.yml` — never hardcoded in task files or playbooks.
- When a newer version is adopted after a downgrade workaround, remove the workaround comment from `defaults/main.yml`.

**Upgrade order for cluster upgrades:**
1. Upgrade one control plane node at a time (cordon → drain → upgrade → rejoin → verify)
2. Then upgrade worker nodes one at a time
3. Then upgrade cluster add-ons (Cilium, monitoring) individually

## Repository-Specific Patterns

### Hetzner Integration
- Provisioning uses Hetzner Robot API + installimage script
- SSH keys from vault, extracted by postCreate.sh
- Machine hostname passed to install script via environment variable

### DevContainer Setup (`postCreate.sh`)

`postCreate.sh` runs automatically on container creation and is the single source of truth for all environment bootstrapping. It handles:
- Installing Python packages and Ansible collections
- Extracting the SSH private key (`vault_github_actions_ssh_private_key`) from vault to `~/.ssh/hetzner-ansible-key`
- Writing `~/.kube/config` from `vault_kubeconfig`
- Injecting `GITHUB_TOKEN` into `~/.bashrc`
- Applying the `sops-age` secret to the cluster

**All secrets must be extracted in `postCreate.sh` using `yaml.safe_load` via Python** — not grep/sed. The vault file is at `ansible/inventory/group_vars/all/vault.yml`. Example pattern:
```bash
VALUE=$(ansible-vault view "$ANSIBLE_DIR/inventory/group_vars/all/vault.yml" 2>/dev/null | \
    python3 -c "
import sys, yaml
d = yaml.safe_load(sys.stdin)
print(d.get('vault_my_secret', ''))
")
```

**If the SSH key or any other secret is missing after a container rebuild, fix `postCreate.sh` — never extract secrets manually in the terminal.** The postCreate.sh script is the contract that makes the devcontainer self-contained; workarounds defeat that contract and will silently break for future agents.

### Kubernetes Cluster
- v1.35 with kubeadm
- Stacked etcd for HA control planes
- Odd number of control planes required (1, 3, 5...)
- 5 control planes in HEL1-DC2 (Helsinki)
- **Control plane HA**: DNS round-robin (`kubernetes-api.yral.com` → 5 A records, TTL=Auto, DNS-only at Cloudflare)
- **Cloudflare DNS lifecycle**: `node-remove` role deletes the outgoing CP's A record from Cloudflare **before** running `kubeadm reset`. `control-plane-join` role adds the new CP's A record after a successful join. This prevents worker kubelets from resolving to a decommissioned API server IP during the removal window. Config: `cloudflare_zone_id` and `cloudflare_api_token` in `vars.yml`/`vault.yml`.
- Cilium v1.19.1 CNI with WireGuard encryption
- Serial: 1 for node operations (one node at a time)

**Deployment order:**
1. `init-control-plane.yml` — bootstrap CP-1 (includes Cilium CNI install at end)
2. `add-control-plane.yml -e target_host=control-plane-N` — for CP-2 through CP-5 (one at a time; Cilium DaemonSet auto-deploys; DNS A record registered automatically)
3. `add-worker.yml -e target_host=worker-N` — add worker nodes (Cilium DaemonSet auto-deploys)

### CoreDNS Topology and Cross-DC VXLAN DNS Issue

**The problem — cross-datacenter UDP over VXLAN is unreliable:**

Hetzner nodes span multiple datacenters (HEL1-DC2 for control planes, FSN1-DC1 and FSN1-DC8 for workers). Cilium's overlay uses VXLAN encapsulation. DNS queries are UDP, and UDP over VXLAN has **no retransmission mechanism** — cross-DC packet loss produces intermittent DNS `i/o timeout` errors visible in all pods whose node is in a different datacenter zone from the CoreDNS replica they hit.

**Symptom:** Pods on worker nodes see sporadic DNS failures for any lookup — both internal (`svc.cluster.local`) and external. The failures are transient and hard to reproduce locally but frequent enough to cause real application errors.

**Root cause identified:** kubeadm's default CoreDNS Deployment has 2 replicas with no scheduling constraints. Both defaulted to control-plane-1 (HEL1-DC2). Worker nodes in FSN1 routed all DNS over VXLAN cross-DC. Cilium's `topology-mode: Auto` was not set on the `kube-dns` Service so no zone-local routing was applied.

**Fix already applied (must be maintained):**

Two manifest files are committed to `kubernetes/infrastructure/coredns/` and applied with `kubectl apply -f` (not Flux-managed — kubeadm owns these resources):

- `coredns-deployment-topology.yaml` — scales CoreDNS to **3 replicas** and adds `topologySpreadConstraints` by `topology.kubernetes.io/zone` (`whenUnsatisfiable: ScheduleAnyway`), ensuring one CoreDNS pod per zone.
- `coredns-service-topology.yaml` — adds `service.kubernetes.io/topology-mode: "Auto"` to the `kube-dns` Service so Cilium's EndpointSlice controller sets topology hints, routing pods to a same-zone CoreDNS endpoint.

To re-apply after any cluster change:
```bash
kubectl apply -f kubernetes/infrastructure/coredns/coredns-deployment-topology.yaml
kubectl apply -f kubernetes/infrastructure/coredns/coredns-service-topology.yaml
```

**Why not Flux-managed:** The CoreDNS Deployment and kube-dns Service are owned by kubeadm. Reconciling them via a Flux Kustomization risks ownership conflicts during `kubeadm upgrade`. The manifests are committed to git for auditability but applied imperatively.

**When adding a new worker node in a new zone:**

Whenever a worker is added in a **new Hetzner zone** (e.g., NBG1, a new FSN1 DC, etc.) that has no CoreDNS replica yet:

1. Verify zone coverage after the node joins:
   ```bash
   kubectl get pods -n kube-system -l k8s-app=kube-dns -o wide
   # Check the NODE column — there must be a CoreDNS pod in each zone that has worker nodes
   ```

2. If a zone is uncovered, increase `spec.replicas` in `coredns-deployment-topology.yaml` to match the total number of zones with worker nodes, then:
   ```bash
   # Commit the change to git first
   git add kubernetes/infrastructure/coredns/coredns-deployment-topology.yaml
   git commit -m "fix: increase CoreDNS replicas for new zone coverage"
   git push
   # Then apply
   kubectl apply -f kubernetes/infrastructure/coredns/coredns-deployment-topology.yaml
   ```

3. `topology-mode: Auto` has a **fallback**: if any zone has zero ready CoreDNS endpoints, Kubernetes automatically falls back to cross-zone routing for pods in that zone. This is safe but reintroduces the intermittent DNS timeout problem. The fix is always to restore per-zone CoreDNS coverage.

**Rule:** `replicas` in `coredns-deployment-topology.yaml` must always be `>=` the number of distinct `topology.kubernetes.io/zone` values across all cluster nodes.

### Longhorn CSI and btrfs Storage

Longhorn is the cluster's default CSI provider (`storageClassName: longhorn`). All PVCs that don't request a specific StorageClass will be provisioned by Longhorn. Prometheus persistent storage uses it; all future stateful workloads should too.

**btrfs CoW conflict — critical:**

Every Hetzner node uses btrfs as the root filesystem (expanded to two drives via `storage-setup`). btrfs applies Copy-on-Write (CoW) to all files by default. Longhorn implements its own CoW replication model. Running both simultaneously causes **double-CoW**:
- Every Longhorn write triggers a btrfs CoW as well
- Significant write amplification and I/O overhead
- btrfs metadata fragmentation grows rapidly under Longhorn workloads
- Apparent "out of space" errors even when free capacity exists (btrfs metadata exhaustion)

**The fix: `nodatacow` on `/var/lib/longhorn`**

The `chattr +C` attribute disables btrfs CoW for a directory tree. It must be set on `/var/lib/longhorn` while the directory is **empty** — before Longhorn's DaemonSet ever writes data there. Once set, all files created inside inherit nodatacow.

This is applied in the `storage-setup` role, which runs during `add-worker.yml` and `init-control-plane.yml` — before the node joins the cluster and before Longhorn's DaemonSet schedules on it.

**Verify on any node:**
```bash
lsattr -d /var/lib/longhorn
# Expected: ---------------C-- /var/lib/longhorn
```

**Existing workers (worker-1, worker-2):** Were provisioned before this fix was added. Their `/var/lib/longhorn` directories do **not** have nodatacow set. Per the immutability principle, the correct fix is to reprovision these nodes (drain Longhorn volumes, run `remove-node.yml`, then `add-worker.yml`). Until then, they are running with suboptimal btrfs CoW behaviour.

**btrfs data profile:**

`btrfs filesystem df /` shows `Data, single` on both workers — data is allocated to one device at a time (JBOD-style span), not RAID0-striped. The `storage-setup` role runs `btrfs balance start / --full-balance` without a profile flag, so the default `single` is preserved. Longhorn is not affected by the btrfs allocation profile — it only sees a directory path.

**Longhorn replica count:**

`defaultReplicaCount: 2` in `kubernetes/infrastructure/longhorn/helmrelease.yaml` — matches the 2 worker nodes. Each Longhorn volume stores 2 replicas across the 2 workers, surviving a single-node failure. Increase this value if more worker nodes are added.

**NFS / RWX volumes:**

`nfs-common` is not installed on nodes. RWX (ReadWriteMany) via Longhorn's NFS share feature requires it. For RWO (ReadWriteOnce) storage — the common case for databases and Prometheus — `nfs-common` is not needed. Install it via the `base-system` role if RWX support is required.

### Inventory Structure
- `control_plane` group: control plane hosts
- `worker_nodes` group: worker hosts
- `k8s_cluster` meta-group: all cluster nodes
- Target node specified via `-e target_host=node-name`

### Kubernetes Manifests (`kubernetes/`)

**Strict separation rule: if it runs as a pod, it belongs in `kubernetes/`, never in Ansible.**

Ansible manages infrastructure *below* the Kubernetes API: OS provisioning, kubelet, containerd, kubeadm cluster initialization, and the few components Kubernetes itself needs to start (CNI, Gateway API CRDs). Once the cluster API is available, **all further workloads** — cert-manager, monitoring, Flux, application services — are expressed as Kubernetes resources in `kubernetes/`.

| Location | Contents | Applied by |
|----------|----------|------------|
| `ansible/manifests/` | Helm values files consumed by Ansible roles | Ansible roles (via `copy` module) |
| `kubernetes/` | All K8s objects: Gateways, HTTPRoutes, Kustomizations, HelmReleases, workloads | Flux (GitOps) — see note below about `kubectl apply` |

**Examples of what goes where:**

| Component | Location | Reason |
|-----------|----------|--------|
| Cilium CNI | Ansible role | Must exist before any pod can schedule (Day-1 bootstrap only) |
| Cilium Day-2 (config/version) | `kubernetes/infrastructure/cilium/` | HelmRelease managed by Flux after bootstrap |
| Gateway API CRDs | Ansible role | Cilium needs them at CNI startup |
| cert-manager | `kubernetes/infrastructure/cert-manager/` | Runs as pods, managed declaratively |
| cert-manager ClusterIssuers | `kubernetes/infrastructure/cert-manager-issuers/` | Separate dir — Flux enforces ordering via dependsOn |
| Monitoring stack | `kubernetes/infrastructure/monitoring/` | Runs as pods, managed declaratively |
| oauth2-proxy | `kubernetes/infrastructure/oauth2-proxy/` | Auth proxy for internal tools with no built-in auth (one Deployment per protected app) |
| Gateway / HTTPRoute | `kubernetes/networking/` | All services — internal tools go via oauth2-proxy backend, user-facing services directly |
| Application workloads | `kubernetes/apps/` | Pure K8s objects |

**NetworkPolicy and the cilium-envoy proxy — source IP constraint:**

The Cilium Gateway API implementation uses a `cilium-envoy` DaemonSet that runs with `hostNetwork: true`. External traffic flows:
```
Client IP → node:80/443 (cilium-envoy, hostNetwork) → backend pod
```
Because cilium-envoy runs with `hostNetwork: true`, traffic it forwards to backend pods arrives at the pod's network policy enforcement point as a **node IP**, not as a pod/namespace-scoped identity. This affects **both** `CiliumNetworkPolicy` and standard Kubernetes `NetworkPolicy`:

- **`CiliumNetworkPolicy` with `fromCIDRSet`**: enforcement is against the cilium-envoy node IP, not the original client IP — `fromCIDRSet` with Cloudflare CIDRs will deny all legitimate traffic.
- **Standard `NetworkPolicy` with `namespaceSelector` / `podSelector`**: also does NOT match traffic from cilium-envoy, because node IPs have no Kubernetes namespace association and are not matched by pod/namespace selectors.

**Consequence:** Neither CiliumNetworkPolicy nor standard NetworkPolicy selectors can be used to restrict (or reliably allow) gateway-originated traffic to backend pods based on IP or namespace identity. To allow gateway traffic through a NetworkPolicy, add an explicit ingress rule with **no `from:` clause** scoped to the relevant port — security is then enforced by the application layer (HMAC secrets, auth proxies, etc.), not by IP.

**Pattern for allowing gateway traffic through NetworkPolicy:**
```yaml
# Allows gateway (cilium-envoy hostNetwork) traffic on a specific port.
# No from: clause = allow from any source. Scope with port to limit exposure.
ingress:
  - ports:
      - port: 9292
        protocol: TCP
```

The correct enforcement point for access control in this cluster is an in-cluster auth proxy (oauth2-proxy) or application-level authentication, not pod-level network policy IP rules.

**Exposure model — all services go via the Cilium Gateway:**

All services are exposed through the Cilium Gateway API. Auth is layered in front of services that lack their own:

| Service type | Backend in HTTPRoute | Auth |
|---|---|---|
| **User-facing apps** (public APIs, frontends) | App Service directly | App's own auth (JWT, session, etc.) |
| **Internal tools without auth** (Hubble UI, etc.) | `oauth2-proxy` Service → app upstream | oauth2-proxy enforces Google Workspace SSO |

**oauth2-proxy pattern for internal tools:**
- One `oauth2-proxy` Deployment + Service per protected app, in `kubernetes/infrastructure/oauth2-proxy/`
- HTTPRoute points to the `oauth2-proxy` Service; oauth2-proxy proxies upstream to the real app
- A single `oauth2-proxy-secrets` Secret (SOPS-encrypted) holds the shared Google OAuth client ID/secret and cookie secret
- Each Deployment gets a unique `--cookie-name` to avoid session cookie collisions between apps
- Auth cannot be bypassed by hitting node IPs directly — enforcement happens inside the cluster, after cilium-envoy

**Adding a new internally-protected service:**
1. Add a new `deployment-<appname>.yaml` and `service-<appname>.yaml` in `kubernetes/infrastructure/oauth2-proxy/` (copy existing pattern, change `--upstream` and `--redirect-url` and `--cookie-name`)
2. Add both to `kubernetes/infrastructure/oauth2-proxy/kustomization.yaml`
3. Update the app's HTTPRoute to point to the new oauth2-proxy Service (add a `ReferenceGrant` if the HTTPRoute and Service are in different namespaces)
4. Add the new `https://<hostname>/oauth2/callback` to the authorized redirect URIs in the Google Cloud OAuth app
5. Commit and push — Flux reconciles

**The shared `oauth2-proxy-secrets` Secret** contains `client-id`, `client-secret`, and `cookie-secret`. All oauth2-proxy Deployments reference this same Secret via env vars. It lives in the `oauth2-proxy` namespace.

**Flux readiness**: `kubernetes/` is structured as a Flux Kustomization source. Bootstrap with:
```bash
flux bootstrap github \
  --owner=dolr-ai \
  --repository=yral-bare-metal-kubernetes-cluster \
  --branch=main \
  --path=./kubernetes/clusters/yral-k8s
```
Flux reconciliation order is enforced via `dependsOn` in `kubernetes/clusters/yral-k8s/`:
- `infrastructure-cilium` (no deps)
- `infrastructure-cert-manager` (no deps)
- `infrastructure-cert-manager-issuers` → dependsOn: cert-manager
- `infrastructure-oauth2-proxy` (no deps — only needs cluster API)
- `networking` → dependsOn: cilium + cert-manager-issuers + oauth2-proxy

**Manual apply order** (before Flux is bootstrapped, after cluster has 3 CPs + 1 worker):
```bash
# 1. Install cert-manager (controller + CRDs)
kubectl apply -k kubernetes/infrastructure/cert-manager
kubectl wait --for=condition=available -n cert-manager deployment/cert-manager --timeout=120s
kubectl wait --for=condition=available -n cert-manager deployment/cert-manager-webhook --timeout=120s

# 2. Apply Cilium Day-2 HelmRelease (requires Flux CRDs — skip until Flux is bootstrapped)
# kubectl apply -k kubernetes/infrastructure/cilium

# 3. Apply ClusterIssuers (requires cert-manager CRDs to exist)
kubectl apply -k kubernetes/infrastructure/cert-manager-issuers

# 4. Apply Gateway and routes
kubectl apply -k kubernetes/networking
```

**Do NOT embed `kubectl apply` of these manifests inside Ansible roles.** Ansible provisions infrastructure; Kubernetes manages its own workloads declaratively.

**`kubectl apply` is only valid before Flux is bootstrapped.** Once Flux is running (i.e., the Flux Kustomizations exist and are reconciling), all changes to `kubernetes/` must go through git — commit, push, and let Flux reconcile. Never use `kubectl apply` or `kubectl delete` to imperatively push changes that belong to the Flux-managed state. Flux's `prune: true` will delete resources removed from git; there is no need to `kubectl delete` them manually.

## Deployment Execution Principle

**All mutations to cluster nodes must go through Ansible roles — never via ad-hoc terminal commands or SSH.**

- ✅ Validation and status checks (read-only): run freely in the terminal — `kubectl get nodes`, `ssh root@<ip> "systemctl status kubelet"`, `ssh root@<ip> "journalctl -u kubelet -n 50"`, etc.
- ❌ Mutations via SSH: strictly prohibited — no `ssh root@<ip> "apt install ..."`, no `ssh root@<ip> "systemctl restart ..."`, no `ssh root@<ip> "kubeadm ..."`, no copying files to nodes by hand.
- ❌ Mutations via `kubectl exec`: strictly prohibited for making changes to node state.
- ❌ Mutations (installs, config changes, reboots, kubeadm operations): must live in a role task and be executed via a playbook.
- ❌ Direct `helm` CLI mutations from the terminal: strictly prohibited — no `helm upgrade`, `helm install`, `helm uninstall`, or `helm repo add` as standalone terminal mutations. Helm operations that change cluster state must be embedded in a role (the role runs `helm` on the target node via Ansible) and invoked through a playbook. The only exception is read-only Helm commands like `helm list` or `helm status`.
- ❌ `kubectl apply` / `kubectl delete` for Flux-managed resources: strictly prohibited once Flux is bootstrapped. Commit the desired state to git and let Flux reconcile. For urgent rollbacks, remove the resource from git (Flux prunes it); do not delete it imperatively.
- ⚠️ `kubectl apply -f <file>` for **non-Flux-managed** resources (e.g., kubeadm-owned objects like the CoreDNS Deployment or kube-dns Service that Flux cannot own): acceptable **only** when Flux reconciliation is structurally impossible. Always commit the manifest file to git first so the repo reflects cluster state, then run `kubectl apply -f <path>`. Never run `kubectl apply` with inline flags or heredocs — always from a committed file. Prefer Flux as the primary mechanism; fall back to `kubectl apply -f` only when necessary.

This ensures every change is:
  - **Idempotent**: re-running the playbook reaches the same end state
  - **Auditable**: changes are tracked in version control
  - **Repeatable**: the same playbook can bootstrap any equivalent node

**SSH is a diagnostic tool only.** If you find yourself about to run a mutating command over SSH, stop. Instead:
1. Identify which role/task the mutation belongs to.
2. Implement or fix it in that role.
3. Re-run the playbook from scratch (immutability: full clean run, not resume).

**Running playbooks — always foreground, never background:**

Always run `ansible-playbook` synchronously (foreground, `isBackground=false`). Never pipe to `tee` or redirect to a log file and tail it separately.

```bash
# ✅ Correct — blocks until done, full output in one shot
ansible-playbook ansible/playbooks/operations/init-control-plane.yml -v

# ❌ Wrong — background process, output truncates, timeouts, invisible failures
ansible-playbook ... -v 2>&1 | tee /tmp/some.log &
```

Rationale: background execution causes output buffer overflows, silent truncation, and read timeouts. Foreground execution streams output directly, blocks until the play finishes, and makes failures immediately visible. Long playbooks (30–60 min) are fine to run foreground — the terminal stays open for the duration.

**When a deployment step fails:**
1. Investigate with read-only terminal commands to diagnose.
2. Fix the role/task that corresponds to the failing step.
3. Re-run the full provisioning playbook from scratch — not from the failing step.
4. Do NOT apply the fix manually on the node and skip updating the role.

## Questions for Agents

When in doubt:
1. Is this logic in a role? → If no, move it to a role.
2. Is this playbook a thin wrapper? → If no, extract logic to role.
3. Does this mutate existing nodes? → If yes, reconsider immutability — reprovision from scratch instead.
4. Could this reboot? → If yes, ensure `base-system` handles it.
5. Can this be tested independently? → If no, it's too coupled.
6. Is this a mutation? → If yes, it must be in a role run via a playbook, never via SSH or terminal.
7. Am I about to create a new playbook? → Stop. Can this fit into an existing playbook via a new role? If yes, do that. If no, ask the user first.
8. Am I about to SSH into a node and run a command that changes state? → Stop. Implement it in a role and run the full playbook instead.

## Active Deployment Handoff

If a `HANDOFF.md` file exists in the repository root, **read it before starting any work**, then **delete it** once the context has been ingested and work is resumed.

`HANDOFF.md` is created exclusively by the agent-handoff prompt — never by agents during normal workflows. Do not create or update `HANDOFF.md` as part of regular cluster operations.

## Maintaining AGENTS.md Over Time

This document is a living guide. As you work on cluster operations and discover new patterns, valid practices, or constraints:

**Add to AGENTS.md when:**
- You establish a new orchestration pattern (e.g., reboot handling, multi-step workflows)
- You discover a clarification that prevents future agents from repeating the same corrections
- You identify repository-specific constraints or best practices (e.g., "roles must be atomic")
- You create new core operations that should be documented
- You resolve architectural issues that required significant refactoring (e.g., orchestrator role extraction)
- You recognize a pattern violation that should be prevented going forward

**Update existing sections when:**
- Current guidance conflicts with evolved practice
- Examples need updating to reflect current codebase state
- Guidelines prove too restrictive or need nuance added
- New valid patterns emerge that differ from documented constraints

**Do NOT add:**
- One-off fixes or workarounds
- Temporary solutions that will be removed
- Comments about specific deployments or incidents
- Non-architectural operational notes (those belong in individual role READMEs)

**Process for updates:**
1. Identify the section(s) needing update
2. Update AGENTS.md with the new or clarified guidance
