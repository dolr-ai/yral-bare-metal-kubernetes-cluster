# Agent Guidelines for Kubernetes Cluster Operations

This document defines architectural patterns and constraints specific to this repository. Follow these guidelines to maintain consistency and code quality.

## Working with Claude Code

**Git commits:** Never add a `Co-Authored-By: Claude` trailer to commit messages in this repository.

## Core Architecture Principles

### 0. Kubernetes-Native First

When choosing between tools or approaches, always prefer the option that is more Kubernetes-native:

- **Built-in > ecosystem > third-party**: Prefer Kubernetes built-in primitives (HPA, VPA, NetworkPolicy, Gateway API) over ecosystem projects, and ecosystem projects over proprietary third-party tools.
- **Declarative GitOps**: Flux over Argo CD — Flux is a CNCF project that extends Kubernetes API primitives directly; Argo CD introduces its own abstractions on top.
- **CRD-native operators over agents**: Prefer operators that extend the Kubernetes API via CRDs over tools that require a separate control plane or proprietary agents.
- **Standard APIs over custom ones**: Use Gateway API over Ingress, use standard Kubernetes Secrets over external secret stores where the security trade-off allows.
- **Upstream Kubernetes projects first**: If the feature exists in the `kubernetes/` or `kubernetes-sigs/` GitHub orgs, prefer it over third-party alternatives (e.g., VPA from `kubernetes/autoscaler` over custom resource recommenders).

**Decision rule**: When evaluating a new component, ask "does Kubernetes or the CNCF ecosystem already provide this?" before reaching for a third-party tool. Document the rationale when a non-native choice is made.

### 1. Immutable Data Operations

**Hard requirement.** Never run `UPDATE` or `DELETE` on a live production table, and never make in-place schema changes that lose data. All data migrations, enrichments, and schema changes on any stateful data store (ClickHouse tables, databases, Kafka topics, object storage prefixes) follow this 4-step versioned pattern:

1. **Dual-write**: Update the write path (Materialized View, application config, Kafka consumer) to write new incoming data to **both** the existing store **and** the new versioned store (e.g., `events_v2`). Do this before backfilling — it ensures the new table stays current during the migration window.
2. **Backfill**: Copy and transform historical data from the old store into the new store.
3. **Validate**: Verify the new store — row counts, fill rates, spot checks on both historical and newly-arrived data — before switching any read path.
4. **Swap and archive**: Switch read paths (dashboards, application queries) to the new store. Rename the old store to `_v1` or `_old` rather than dropping immediately. Drop only after the new store has been running in production for a confirmed period.

This mirrors the Immutable Infrastructure principle: changes are additive, the previous state is preserved until the new state is confirmed correct.

**When creating a new versioned table** (e.g., `events_v2`): add a second Materialized View targeting `events_v2` in `clickhouse-installation.yaml` and deploy it before the backfill job runs. This is the dual-write step.

**A mutation on a brand-new version table** (e.g., enriching `events_v2` before the swap) is acceptable because the old table is intact. A mutation on the currently-live production table is not.

### 1a. Stateful Object Naming Convention

**Mandatory for all changes to stateful data stores.**

All stateful objects — ClickHouse tables and materialized views, Kafka topics, Kubernetes PVCs, object storage prefixes, database schemas — must follow the pattern:

```
<name><delimiter><version><delimiter><status>
```

The delimiter follows each ecosystem's naming convention:

| Ecosystem | Convention | Example |
|-----------|-----------|---------|
| SQL / ClickHouse | snake_case | `events_v2_staging` |
| Kubernetes resources | kebab-case | `events-v2-staging` |
| Python / Go types | PascalCase | `EventsV2Staging` |
| Kafka topics | snake_case | `snowplow_events_v1_active` |

**Status values:**
- `active` — the currently-live production object (primary read/write path)
- `staging` — migration target being prepared; not yet the primary path
- `archived` — previous version retained for rollback safety after a swap
- `temp` — ephemeral; must be dropped after the operation that created it

**Examples for the current events migration:**
- `snowplow.events_v1_active` — live table (currently bare `events`)
- `snowplow.events_v2_staging` — migration target (currently bare `events_v2`)
- `snowplow.events_mv_v1_active` — primary write MV (currently `events_mv`)
- `snowplow.events_mv_v2_staging` — dual-write MV (currently `events_v2_mv`)

**Enforcement:** Apply this convention to any new stateful objects you create. When touching an existing object as part of a migration (e.g., the swap step), rename it to follow the convention at that time. Do not leave newly-created objects with bare names like `events_v3` or `events_new`.

### 2. Immutable Infrastructure
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
4. `remove-node.yml` - Remove node from cluster (chains: `longhorn-evict` → `node-remove`). `longhorn-evict` sets `evictionRequested=true` on all Longhorn disks and waits up to 30 min (polling every 30s) for all replicas to migrate off before `node-remove` drains and deletes the node. **Not applied in `upgrade-worker.yml`** — `upgrade-worker` does not delete the node object at all (see below), so there is nothing to evict from.
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

## Default-First Configuration Policy

**Always prefer component defaults over explicit configuration.** Only add configuration parameters when a specific default behaviour is causing a concrete problem that needs solving. Explicitly setting parameters that match the default:
- creates maintenance burden (the value must be kept in sync as defaults evolve)
- introduces typo/format bugs that would otherwise be impossible
- obscures intent (a reader cannot tell whether the value was set deliberately or cargo-culted)

This applies to all components: Helm values, StorageClass parameters, CephCluster settings, Ansible role vars, etc. If the component ships a sensible default for the current versions in use, omit the parameter.

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

### `become` usage

`ansible.cfg` sets `become = False` globally. This is intentional:

- **Remote plays** (`hosts: worker_nodes`, `hosts: control_plane`): `ansible_user: root` is set in `hosts.yml`, so Ansible already SSH-es in as root. A global `become: true` would run a redundant `sudo root → root` on every task.
- **Localhost plays** (`hosts: localhost`): the `vscode` user has `~/.kube/config`, the SSH key, and all vault-extracted credentials. No privilege escalation is needed.

**Playbooks that target remote nodes declare `become: true` at the play level** (e.g. `add-worker.yml`, `upgrade-worker.yml`). This is belt-and-suspenders for tasks that genuinely need it (package installs, systemd, etc.) and is harmless since the SSH session is already root.

**Localhost playbooks** (`remove-node.yml`, `etcd-backup.yml`) do **not** set `become: true` — they run kubectl, curl, and vault operations as `vscode`, which is correct.

Individual tasks within roles may declare `become: true` when `delegate_to: target_host` is used — these are explicit and intentional (e.g. stopping kubelet on the node being removed).

### Kubernetes Cluster
- v1.35 with kubeadm
- Stacked etcd for HA control planes
- Odd number of control planes required (1, 3, 5...)
- 5 control planes, 1 per Helsinki building for blast-radius isolation:
  - control-plane-1: HEL1-DC2 (running)
  - control-plane-2: HEL1-DC3
  - control-plane-3: HEL1-DC4
  - control-plane-4: HEL1-DC6
  - control-plane-5: HEL1-DC7
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

Hetzner nodes span multiple datacenters (5 Helsinki buildings for control planes, 13 distinct zones across Helsinki + Falkenstein for workers). Cilium's overlay uses VXLAN encapsulation. DNS queries are UDP, and UDP over VXLAN has **no retransmission mechanism** — cross-DC packet loss produces intermittent DNS `i/o timeout` errors visible in all pods whose node is in a different datacenter zone from the CoreDNS replica they hit.

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

### Cilium BPF Service Table Staleness After Worker Reprovision

**Symptom:** Kubernetes admission webhooks return `context deadline exceeded` intermittently — typically after a worker node is removed and reprovisioned. Only some API requests fail (those routed to affected control planes), making the root cause non-obvious.

**Root cause:** Cilium maintains per-node BPF service maps that translate ClusterIP → pod endpoint IPs. When a worker is reprovisioned, its pods get new IPs. In normal operation, Cilium on all nodes syncs from EndpointSlice events. Occasionally, one or more Cilium pods on control planes fail to reconcile, leaving stale pod IPs in their BPF maps. Requests routed through those API servers hit the dead pod IP → TCP timeout → webhook failure.

**Diagnosis:**
```bash
# 1. Check the webhook's EndpointSlice for current pod IPs
kubectl get endpointslices -n longhorn-system -l kubernetes.io/service-name=longhorn-admission-webhook

# 2. Inspect the Cilium service table on each control plane for the webhook's ClusterIP
# Get ClusterIP first:
kubectl get svc -n longhorn-system longhorn-admission-webhook -o jsonpath='{.spec.clusterIP}'

# On each CP, check the Cilium pod:
CILIUM_POD=$(kubectl get pod -n kube-system -o wide --no-headers | grep <cp-node> | grep ^cilium | awk '{print $1}')
kubectl exec -n kube-system $CILIUM_POD -- cilium service list | grep <ClusterIP>
# Then inspect that service ID:
kubectl exec -n kube-system $CILIUM_POD -- cilium service get <ID> --verbose
# Look for backends — any IP not in the current EndpointSlice is stale
```

**Fix:** Restart the Cilium pod on each affected control plane to force a BPF map re-sync from EndpointSlices:
```bash
kubectl delete pod -n kube-system <cilium-pod-on-affected-cp>
# Wait for it to become Running again, then verify backends are updated
```

**Key insight:** Cilium on worker nodes (where the workload pods live) is not the issue — the staleness is on the *control plane* Cilium pods that receive API traffic destinated for webhooks. Always check all CPs, not just the workers.

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

**Existing workers:** worker-2 was reprovisioned after this fix was added and has nodatacow correctly set. worker-1 was provisioned before this fix — it is being reprovisioned via `remove-node.yml` → `add-worker.yml` to apply nodatacow, RAID0, and the correct Longhorn storage reservation.

**btrfs data profile:**

The `storage-setup` role runs `btrfs balance start -dconvert=raid0 -mconvert=dup /` after adding the second drive:
- `Data, RAID0` — chunks are striped across both NVMe drives for maximum sequential throughput and full combined capacity. No local redundancy, but Longhorn's `defaultReplicaCount: 2` provides cross-node redundancy.
- `Metadata, DUP` — metadata is mirrored on both drives; a single bad metadata chunk does not corrupt the filesystem.

worker-2 and all subsequently provisioned nodes are on RAID0. worker-1 is being reprovisioned to correct this.

**Why not mdadm RAID0 + ext4?** Hetzner's `installimage` script supports configuring mdadm RAID0 before OS installation, which would give ext4-on-RAID0 with no CoW at all (Longhorn's preferred filesystem). The tradeoff: mdadm must be configured *in the installimage step* (the OS boots on the array), requiring a change to the `provision` role's installimage config. The current btrfs approach is post-install (OS is already on nvme0, storage-setup adds nvme1 to the pool), which is simpler. If all nodes are reprovisioned at the same time in the future, switching to mdadm RAID0 + ext4 + mounting `/var/lib/longhorn` on the array would eliminate the `nodatacow` workaround entirely and give marginally better raw I/O.

**Longhorn replica count and StorageClass policy:**

The default `longhorn` StorageClass uses 2 replicas (`defaultReplicaCount: 2`). **Always use `storageClassName: longhorn` (or omit it) for new PVCs.** This is the right choice for nearly all stateful workloads — databases, caches, object stores, and services that do not implement their own replication. Longhorn 2-replica tolerates a single node failure without data loss and without requiring the application to handle it.

The `longhorn-1replica` StorageClass (defined in `storageclass-kafka.yaml`) exists exclusively for workloads where app-level replication is not just present but **strictly stronger** than what Longhorn provides, making Longhorn replication purely wasteful. Currently only Kafka meets this bar: with 3 brokers and `replication.factor=3`, Kafka can tolerate 2 of 3 node failures — Longhorn 2-replica only tolerates 1. Layering both gives 6× physical disk writes per produce request (3 brokers × 2 Longhorn replicas) with no additional durability benefit. Kafka's replication is the write path itself, not an optional redundancy layer on top of it.

**This is not a pattern to copy.** All databases and stateful workloads that do not implement their own replication — including PostgreSQL, ClickHouse, Redis, and any future data stores — must use `longhorn` (2 replicas). These workloads rely entirely on Longhorn for durability; dropping to 1 replica gives zero fault tolerance for a single node failure.

Only use `longhorn-1replica` when **all three** of the following are true:
1. The application implements its own replication natively (not just backups or failover)
2. App-level replication is strictly stronger than Longhorn 2-replica
3. The write amplification from layering both is measurable and unacceptable

If in doubt, use `longhorn`. Document the specific reason when choosing `longhorn-1replica`.

**Storage reservation — two separate knobs:**

`storageReservedPercentageForDefaultDisk: 5` (5% of each disk, ~48 Gi on 954 Gi nodes). This is the headroom Longhorn leaves for the OS, kubelet, and containerd image cache. Kubelet's default image GC (`imageGCHighThresholdPercent: 85`) automatically evicts unused container images before the disk fills, making a 2% reservation safe in practice. 5% is a conservative buffer on top of that. The global setting propagates to newly-registered disks; existing nodes have their `storageReserved` field written by the Longhorn node controller and will be updated on the next reconciliation cycle.

`storageMinimalAvailablePercentage: 0` — a completely separate setting controlling the minimum *physical free space* that must remain after a volume expansion (enforced by the `validator.longhorn.io` admission webhook). The default is 25%, which conflicts with the 5% reservation strategy: a disk can have 452 GiB free yet still fail an expansion because 25% of 954 GiB = 238 GiB would remain below the floor. Since `storageReservedPercentageForDefaultDisk` already protects the host, this second guard is redundant and set to 0.

**NFS / RWX volumes:**

`nfs-common` is installed on all nodes via the `base-system` role. Longhorn RWX (ReadWriteMany) volumes work by creating a `longhorn-share-manager` pod that runs an in-cluster NFS server; the kubelet uses `nfs-common` userspace tools to mount that NFS export into pods. No conflicts with Cilium + WireGuard — pod-to-pod NFS traffic (port 2049) travels inside the encrypted Cilium mesh. The `storage-network-for-rwx-volume-enabled: false` Longhorn setting correctly routes RWX traffic over the standard pod network.

**Longhorn version and placement model:**

Current version: `1.11.1`.

**Post-upgrade: engine image must be manually upgraded per-volume.** When the Longhorn HelmRelease chart version is bumped, the control plane (manager, CSI driver) upgrades automatically but existing attached volumes keep running on the old engine image — Longhorn deliberately does not auto-live-upgrade them to avoid a fleet-wide I/O blip. New volumes get the new image automatically. After any chart version bump, run:
```bash
NEW_IMAGE="docker.io/longhornio/longhorn-engine:v<new-version>"
kubectl patch volumes.longhorn.io \
  $(kubectl get volumes.longhorn.io -n longhorn-system --no-headers \
    -o custom-columns='NAME:.metadata.name,IMAGE:.status.currentImage' \
    | grep -v "$NEW_IMAGE" | awk '{print $1}' | tr '\n' ' ') \
  -n longhorn-system --type=merge -p "{\"spec\":{\"image\":\"$NEW_IMAGE\"}}"
```
The correct field is `spec.image` (not `spec.engineImage`). The live upgrade causes a brief per-volume engine process restart but does not detach or interrupt I/O observable to pods.

**Placement strategy — same-node primary replica, free secondary:**

Longhorn's two replicas serve distinct purposes:
- **Replica 1 (local)**: `dataLocality: best-effort` migrates one replica to the pod's node in the background after scheduling. This eliminates the network hop for writes — I/O goes directly to local NVMe. The kube-scheduler tends to return pods to the same node after restarts (resource stability), and the local replica reinforces that tendency. `WaitForFirstConsumer` defers PV binding until the pod is scheduled, so the local replica starts on the pod's node rather than needing to migrate after the fact.
- **Replica 2 (remote)**: placed freely by Longhorn's scheduler — it may land on a different node, different zone, or different datacenter region. Cross-region replication is asynchronous and transparent to the application. The second replica provides datacenter-level durability at no cost to application write latency.

No topology constraints are configured (`csiAllowedTopologyKeys`, `allowedTopologies`, and Volume CR `spec.nodeSelector` are intentionally absent). Longhorn's default zone anti-affinity spreads the two replicas across zones when possible — this is desirable for durability and left at its default.

**Co-locating tightly-coupled pods (app + its database):**

When a workload has a dedicated database that it talks to constantly (e.g. Bytebase + its Postgres), both pods must land in the same `topology.kubernetes.io/region` (helsinki or falkenstein). Cross-region pod-to-pod traffic adds ~10ms latency on every DB call, which is unacceptable for synchronous request paths.

Use `podAffinity` with `requiredDuringSchedulingIgnoredDuringExecution` at the `topology.kubernetes.io/region` topologyKey. The database pod schedules first (wherever the scheduler places it); the app pod then follows to the same region.

Pattern — add to the app's pod spec (or HelmRelease `affinity` values key):
```yaml
affinity:
  podAffinity:
    requiredDuringSchedulingIgnoredDuringExecution:
      - labelSelector:
          matchLabels:
            app: <database-pod-label>
        topologyKey: topology.kubernetes.io/region
```

This ensures tightly-coupled pods (e.g. an app and its database) share a zone, minimising pod-to-pod latency on the synchronous request path. Longhorn's `WaitForFirstConsumer` then binds the PVC to whichever node the pod lands on.

`replicaAutoBalance: best-effort` is **permanently enabled** — this is required for Longhorn to remove excess replicas when `spec.numberOfReplicas` is reduced on an attached volume (without it, `cleanupAutoBalancedReplicas` returns immediately and extra replicas are never removed).

**`disableRevisionCounter` — critical HelmRelease format requirement (Longhorn v1.11+):**

Longhorn v1.11 changed the `disable-revision-counter` Setting's internal format from a plain boolean string to a per-engine-type JSON object: `{"v1":"true"}` or `{"v1":"false"}`. This change breaks naive HelmRelease values and causes a hard-to-diagnose salvage failure.

**The bug:** Setting `disableRevisionCounter: "false"` (quoted string) in the HelmRelease is a non-empty string, which is *truthy* in Go templates. The chart renders the ConfigMap as `{"v1":"true"}` regardless. The correct value is `disableRevisionCounter: '{"v1":"false"}'` — the explicit JSON form.

**Where each CR gets the flag:**
- **Engine CR** `spec.revisionCounterDisabled` — written by the volume controller from the **global Setting CR** (`disable-revision-counter`). This is what controls the `--disableRevCounter` flag on the engine process.
- **Replica** `--disableRevCounter` flag — derived from the **Volume CR** `spec.revisionCounterDisabled`, which is set at PVC creation time from the StorageClass parameter.

**The salvage failure mode:** When a volume is faulted and re-attaches, Longhorn starts the engine with `--salvageRequested`. In salvage mode the engine checks `RevisionCounterDisabled` parity between itself and each replica. If the engine has `--disableRevCounter` (from Setting) but replicas do not (from Volume CR spec), every replica is immediately marked ERR with `"Revision Counter Disabled setting mismatch"` → `"cannot find any replica for salvage"` → both replicas get `failedAt` set → volume stays faulted in a rapid cycle.

**Diagnosis commands:**
```bash
# Check global Setting (should be {"v1":"false"})
kubectl get setting.longhorn.io disable-revision-counter -n longhorn-system -o jsonpath='{.value}'

# Check Engine CRs (all should be False)
kubectl get engines.longhorn.io -n longhorn-system -o json | \
  python3 -c "import sys,json; d=json.load(sys.stdin); \
  bad=[e['metadata']['name'] for e in d['items'] if e['spec'].get('revisionCounterDisabled')!=False]; \
  print(f'{len(bad)} engines wrong'); [print(' ',n) for n in bad]"

# Check instance-manager logs for the mismatch message
kubectl logs -n longhorn-system <instance-manager-pod> --since=5m | grep "Revision Counter Disabled"
```

**After changing the global Setting, Engine CRs do NOT auto-reconcile immediately** — the volume controller reconciles them lazily. After any Setting change, patch all engines that still have the old value:
```bash
kubectl get engines.longhorn.io -n longhorn-system -o json | python3 -c "
import sys, json, subprocess
d = json.load(sys.stdin)
to_patch = [e['metadata']['name'] for e in d['items'] if e['spec'].get('revisionCounterDisabled') == True]
for name in to_patch:
    subprocess.run(['kubectl','patch','engines.longhorn.io',name,'-n','longhorn-system',
                    '--type=merge','-p','{\"spec\":{\"revisionCounterDisabled\":false}}'])
    print(f'patched {name}')
"
```

**Recovery if a volume is stuck in the salvage cycle:** See the "Cilium BPF Service Table Staleness" section pattern for diagnosis. Fix order: (1) patch the global Setting CR to `{"v1":"false"}`, (2) restart the longhorn-manager pod on the volume's ownerID node to flush its setting cache, (3) patch the Engine CR `spec.revisionCounterDisabled: false`, (4) clear `failedAt` on both replicas.

### Rook/Ceph — Distributed Block Storage

The cluster is migrating from Longhorn to **Rook/Ceph** as the primary CSI provider. Rook v1.19.3 + Ceph v20.2.1 (Tentacle). All new stateful workloads should use `storageClassName: ceph-block` once the migration is complete and Ceph is made the default StorageClass.

**Why Ceph instead of Longhorn:**
Longhorn caps each PVC at a single node's disk size. Ceph aggregates all OSD disks across the entire cluster into one logical pool; a PVC's maximum size is the total usable pool capacity (currently ~16.5 TB at 2× replication over 35 workers × 950 GB per worker).

**Disk layout per worker node (post-reprovision):**

| Device | Role | Size |
|--------|------|------|
| `nvme0n1p1` | EFI / BIOS boot | ~512 MB |
| `nvme0n1p2` | btrfs OS root | **50 GB** |
| `nvme0n1p3` | Raw Ceph OSD partition (created by `storage-setup`) | ~450 GB |
| `nvme1n1` | Raw Ceph OSD whole disk (wiped by `storage-setup`) | ~500 GB |

Each worker contributes **2 OSDs** (~950 GB total). Control planes keep the original full-disk btrfs RAID0 layout and are **not** Ceph OSD nodes (they are in Helsinki; all workers are in Falkenstein).

**How worker reprovisioning works with the new disk layout:**
- `provision` role passes `OS_PARTITION_SIZE=50G` to `install-ubuntu.sh` via `provision_os_partition_size` (auto-detected from `group_names`); control planes continue to use `all`.
- `storage-setup` role detects `'worker_nodes' in group_names` and runs the Ceph OSD path: creates the GPT partition on nvme0n1 free space with `sgdisk`, wipes nvme1n1, creates `/var/lib/rook`.
- Rook's `deviceFilter: "nvme"` auto-discovers both raw devices on the newly joined node.

**Rook/Ceph Kubernetes manifests — two-directory split:**
```
kubernetes/infrastructure/rook-ceph/
  operator/   ← Flux Kustomization: infrastructure-rook-ceph-operator
    namespace.yaml, helmrepository.yaml, helmrelease.yaml
  cluster/    ← Flux Kustomization: infrastructure-rook-ceph-cluster (dependsOn: operator)
    cephcluster.yaml, cephblockpool.yaml, storageclass-rbd.yaml
```
The split is necessary because the `CephCluster` CRD is installed by the Rook Helm chart. `wait: true` on the operator Kustomization ensures CRDs exist before the cluster CRs are applied.

**Pool configuration:**
- `replication.size: 2` — each block is written to 2 different OSD nodes.
- `failureDomain: host` — replicas must land on different physical nodes.
- Usable capacity: ~16.5 TB (33 TB raw ÷ 2).
- `ceph-block` StorageClass: RWO, `WaitForFirstConsumer`, `allowVolumeExpansion: true`.
- **OSD encryption**: Intentionally disabled. `encryptedDevice: "true"` restricts Rook to whole-disk OSDs only — ceph-volume unconditionally rejects partitions in encrypted mode. Each worker provides one ~450 GB partition (`nvme0n1p3`) and one ~477 GB whole disk (`nvme1n1`) as separate OSDs, so encryption would silently drop half the capacity per worker. In-transit encryption is provided by Cilium WireGuard cluster-wide.

**Default StorageClass transition:**
`ceph-block` is intentionally **not** the default during migration. Once all Longhorn PVCs are migrated, flip both annotations:
```bash
kubectl patch storageclass ceph-block -p '{"metadata":{"annotations":{"storageclass.kubernetes.io/is-default-class":"true"}}}'
kubectl patch storageclass longhorn -p '{"metadata":{"annotations":{"storageclass.kubernetes.io/is-default-class":"false"}}}'
```
Then update the Longhorn HelmRelease `persistence.defaultClass: false` and commit.

**Monitoring Ceph cluster health:**
```bash
# Overall health
kubectl exec -n rook-ceph deploy/rook-ceph-tools -- ceph status

# OSD status (expect 70 OSDs once all 35 workers are reprovisioned)
kubectl exec -n rook-ceph deploy/rook-ceph-tools -- ceph osd status

# Pool usage
kubectl exec -n rook-ceph deploy/rook-ceph-tools -- ceph df

# Placement group status (should be 100% active+clean at steady state)
kubectl exec -n rook-ceph deploy/rook-ceph-tools -- ceph pg stat
```

Deploy the Ceph toolbox for ad-hoc diagnostics:
```bash
kubectl apply -f https://raw.githubusercontent.com/rook/rook/v1.19.3/deploy/examples/toolbox.yaml
```

**Worker node migration sequence:**
Workers are reprovisioned **strictly in order of worker number** (worker-1, worker-2, … worker-35), one at a time. This ensures nothing is missed and progress is easy to track. No batching, no skipping ahead.

For each worker, follow this exact procedure:

1. **Check for Longhorn replicas** on the worker:
   ```bash
   kubectl get replicas.longhorn.io -n longhorn-system \
     -o jsonpath='{range .items[?(@.spec.nodeID=="worker-N")]}{.spec.volumeName}{"\n"}{end}' | sort -u
   ```

2. **If replicas exist** — migrate each PVC to `ceph-block` first (see PVC data migration procedure below), validate the migrated data, then delete the old Longhorn PVC and confirm the Longhorn volume is gone. Only proceed to step 3 once all replicas on this worker are migrated and removed.

3. **Remove and reprovision** using the existing playbooks:
   ```bash
   ansible-playbook ansible/playbooks/operations/remove-node.yml -e target_host=worker-N
   ansible-playbook ansible/playbooks/operations/add-worker.yml -e target_host=worker-N
   ```

4. **Verify OSDs are up** before moving to the next worker:
   ```bash
   kubectl exec -n rook-ceph deploy/rook-ceph-tools -- ceph osd tree | grep worker-N
   ```

The cluster is usable for PVC provisioning once **3 OSD nodes** are online (mons achieve quorum and the block pool has enough replicated copies). Wait for `ceph status` to show `HEALTH_OK` or `HEALTH_WARN` (not `HEALTH_ERR`) before provisioning test volumes.

**Reprovision status (as of April 2026):**
Workers 1–9 are reprovisioned with the correct disk layout (50 GB OS partition + Ceph OSD partition on nvme0n1 + nvme1n1 whole disk). Each has 2 OSDs. Workers 10–35 still need reprovisioning.

**ceph_bluestore signature wipe — important operational note:**
`wipefs -af` does not clear `ceph_bluestore` signatures. blkid detects bluestore by reading raw data at byte 0 of the device, not via the standard wipefs signature table. The `storage-setup` role now runs `dd if=/dev/zero of=<device> bs=4M count=10 conv=fsync` after wipefs on both OSD devices (nvme0n1p3 and nvme1n1), guaranteeing that udevadm/blkid see a blank device on every reprovision. If you see a fresh OSD prepare job log the device as "contains a filesystem ceph_bluestore" and skip it, this dd step was not run — reprovision the worker again.

**PARTLABEL must NOT contain "ceph" — important:**
ceph-volume inventory checks `'ceph' in partlabel` and marks any match as "Used by ceph-disk", causing the OSD partition to be unconditionally skipped. The GPT partition on nvme0n1p3 MUST be labelled `rook-osd` (not `ceph-osd` or anything with "ceph"). This is enforced in `storage-setup` — do not change this label.

**PVC data migration procedure (Longhorn → Ceph):**

**First, determine whether the data actually needs migrating:**

- **Regenerable data** (caches, downloaded files, indexes that rebuild on startup): just update `storageClassName: ceph-block` in git and let Flux do the work. The correct procedure is:
  1. Suspend the Flux kustomization (`flux suspend kustomization <name>`)
  2. Scale the workload to 0
  3. Delete the old Longhorn PVC imperatively (`kubectl delete pvc <name> -n <ns>`)
  4. Resume the Flux kustomization — Flux provisions a fresh ceph-block PVC and the workload repopulates it on startup
  5. Scale back up and validate

  Examples: `geoip-db` (re-downloaded by init container), any PVC whose data is derived from an external source.

- **Irreplaceable data** (databases, application state, user-uploaded content): use the rsync procedure below.

**CRITICAL — never pin `volumeName` in git manifests.** When Flux applies a PVC with `volumeName` set, it permanently couples the manifest to a specific PV. If that PV is ever replaced (node reprovision, migration, restore), the manifest must be updated again. Instead: after a migration, the git manifest must contain only `storageClassName` — Flux will dynamically provision a new PV. The only exception is an in-progress rebind step that is immediately followed by removing `volumeName` from git.

For workloads with irreplaceable data, perform this sequence. The procedure uses a temporary migration PVC and a rsync Job to avoid downtime. The hardest part is rebinding a StatefulSet's PVC to the new Ceph PV.

```bash
# Example: migrating Loki's 600 Gi PVC (storage-loki-0)
NAMESPACE=loki
OLD_PVC=storage-loki-0
NEW_SIZE=600Gi
WORKLOAD=loki  # StatefulSet name

# 1. Scale the workload to 0 to quiesce writes
kubectl scale statefulset $WORKLOAD -n $NAMESPACE --replicas=0
kubectl wait pod -n $NAMESPACE -l app.kubernetes.io/name=loki --for=delete --timeout=120s

# 2. Create a migration Ceph PVC
kubectl apply -f - <<EOF
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: ${OLD_PVC}-migrate
  namespace: $NAMESPACE
spec:
  storageClassName: ceph-block
  accessModes: [ReadWriteOnce]
  resources:
    requests:
      storage: $NEW_SIZE
EOF

# 3. Run rsync Job to copy data (replace image tag as needed)
kubectl apply -f - <<EOF
apiVersion: batch/v1
kind: Job
metadata:
  name: migrate-${OLD_PVC}
  namespace: $NAMESPACE
spec:
  template:
    spec:
      restartPolicy: Never
      containers:
        - name: rsync
          image: alpine:3.20
          command: [sh, -c, "apk add --no-cache rsync && rsync -avx --delete /src/ /dst/"]
          volumeMounts:
            - name: src
              mountPath: /src
            - name: dst
              mountPath: /dst
      volumes:
        - name: src
          persistentVolumeClaim:
            claimName: $OLD_PVC
        - name: dst
          persistentVolumeClaim:
            claimName: ${OLD_PVC}-migrate
EOF
kubectl wait job migrate-${OLD_PVC} -n $NAMESPACE --for=condition=complete --timeout=3600s
kubectl delete job migrate-${OLD_PVC} -n $NAMESPACE

# 4. Mark the Ceph PV as Retain so it survives PVC deletion
CEPH_PV=$(kubectl get pvc ${OLD_PVC}-migrate -n $NAMESPACE -o jsonpath='{.spec.volumeName}')
kubectl patch pv $CEPH_PV -p '{"spec":{"persistentVolumeReclaimPolicy":"Retain"}}'

# 5. Delete both PVCs (the Longhorn PVC is reclaimed; the Ceph PV stays due to Retain)
kubectl delete pvc $OLD_PVC -n $NAMESPACE
kubectl delete pvc ${OLD_PVC}-migrate -n $NAMESPACE

# After the second delete, the Ceph PV status goes to Released.
# 6. Patch the Ceph PV to clear its claimRef (makes it Available for rebinding)
kubectl patch pv $CEPH_PV -p '{"spec":{"claimRef":null}}'

# 7. Create a new PVC with the ORIGINAL name, binding explicitly to the Ceph PV
kubectl apply -f - <<EOF
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: $OLD_PVC
  namespace: $NAMESPACE
spec:
  storageClassName: ceph-block
  accessModes: [ReadWriteOnce]
  resources:
    requests:
      storage: $NEW_SIZE
  volumeName: $CEPH_PV
EOF
kubectl wait pvc $OLD_PVC -n $NAMESPACE --for=condition=Bound --timeout=60s

# 8. Scale the workload back up and verify it starts cleanly
kubectl scale statefulset $WORKLOAD -n $NAMESPACE --replicas=1
kubectl rollout status statefulset $WORKLOAD -n $NAMESPACE --timeout=300s

# 9. Restore the Ceph PV reclaim policy to Delete (normal behaviour)
kubectl patch pv $CEPH_PV -p '{"spec":{"persistentVolumeReclaimPolicy":"Delete"}}'
```

**PVCs to migrate (current inventory):**

| Namespace | PVC | Size | Workload | Notes |
|-----------|-----|------|----------|-------|
| loki | storage-loki-0 | 600 Gi | loki StatefulSet | Largest — migrate with care |
| kafka | data-kafka-cluster-combined-0/1/2 | 250 Gi × 3 | Kafka StatefulSet (3 brokers) | App-level replication → use `ceph-block` (single Rook replica acceptable only if using a 1-replica CephBlockPool variant) |
| clickhouse | data-volume-chi-analytics-events-0-0-0 | 200 Gi | ClickHouse | Follow immutable data ops pattern |
| monitoring | prometheus-kube-prometheus-stack-prometheus-db-... | 100 Gi | Prometheus StatefulSet | Losing metrics history OK temporarily |
| metabase | metabase-data | 10 Gi | Metabase Deployment | |
| bytebase | bytebase-postgres-data | 5 Gi | Bytebase StatefulSet | |
| snowplow | geoip-db | 1 Gi | Snowplow | Tiny — migrate last |

**Kafka note:** Kafka brokers use `longhorn-1replica` currently. After migration to Ceph, continue using a 1-replica pool. Either reuse the same `ceph-block` StorageClass with a separate `CephBlockPool` configured for `replicated.size: 1`, or apply Kafka's own replication factor guarantee directly. Decision: use `ceph-block` (2-replica pool) for simplicity — the write amplification concern (3 brokers × 2 Longhorn replicas) still applies, but at cluster scale the I/O is no longer cross-node per write (Ceph is in the same Falkenstein region as Kafka pods). If write amplification becomes measurable, create a `ceph-block-1replica` StorageClass pointing to a separate 1-replica pool.

**Decommissioning Longhorn (after all PVCs migrated):**
1. Make `ceph-block` the default StorageClass (flip annotations above).
2. Remove `infrastructure-longhorn` from `kubernetes/clusters/yral-k8s/kustomization.yaml`.
3. Suspend the Flux Kustomization first: `flux suspend kustomization infrastructure-longhorn`.
4. Remove `storage-setup`'s Longhorn section for control planes (the `chattr +C` block and `/var/lib/longhorn` directory creation) — they are marked with "Remove once Longhorn is removed" comments.
5. Remove `longhorn-evict` role invocation from `remove-node.yml` (no more Longhorn volumes to evict).

### Backup Strategy

Two complementary backup mechanisms run in parallel — use both, they are not redundant:

| System | Scope | Storage path | Retention | Best for |
|--------|-------|-------------|-----------|----------|
| **Velero** | All k8s resource manifests + PV filesystem data (kopia) | `velero/backups/` | `ttl: 720h` (30d, self-managed) | Full cluster DR — restore entire namespace or cluster state |
| **Longhorn native** | Incremental block-level volume snapshots | `longhorn/` | RecurringJob TTL (per-volume) | Per-volume point-in-time restores, faster than Velero for single-volume recovery |

**S3 bucket:** `yral-bare-metal-kubernetes-cluster-control-plane-backup` on `hel1.your-objectstorage.com`

**Bucket layout — named prefix per system:**
```
yral-bare-metal-kubernetes-cluster-control-plane-backup/
├── velero/      ← Velero (BSL prefix: velero)
└── longhorn/    ← Longhorn native backup target suffix
```

Each backup system uses its own named prefix in the bucket to prevent any possibility of collision. When adding a new backup system, always assign it a dedicated prefix.

**Retention — each system manages its own:**
- Velero: `schedule.ttl: 720h` (30 days) in the HelmRelease. Velero's GC controller deletes expired Backup objects *and* their S3 data automatically. No bucket lifecycle policy needed.
- Longhorn: retention is configured per-volume via RecurringJob TTL settings. Longhorn incremental backups share blocks across snapshots — a bucket-level expiry policy would silently corrupt the backup chain by deleting base blocks still referenced by newer incremental backups. **Never set a bucket lifecycle policy on the `longhorn/` prefix.**

No bucket lifecycle policy is configured on this bucket.

### Analytics Pipeline (Snowplow → Kafka → ClickHouse)

**ClickHouse service hostname — common footgun:**

The Altinity operator creates two services for the `analytics` installation:
- `clickhouse-analytics.clickhouse.svc.cluster.local` — the load-balancer service, use this for all connections
- `chi-analytics-events-0-0.clickhouse.svc.cluster.local` — the per-pod headless service, not for general use

**Always use `clickhouse-analytics.clickhouse.svc.cluster.local:8123`** when connecting to ClickHouse from within the cluster (Metabase, backfill jobs, any tooling). `chi-analytics-events.clickhouse.svc.cluster.local` does not exist and will fail DNS resolution.

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

**Application source code — `apps/` git submodules:**

Application source repositories live as git submodules under `apps/`. Deployment manifests for those apps live under `kubernetes/apps/`. The two are intentionally decoupled — Flux reconciles manifests from `kubernetes/apps/` independently of whether the submodule is checked out.

**Submodule operations must always be performed via `git submodule add` in the terminal — never via MCP tools or by creating/pushing files into the upstream repo.** Using MCP to mirror what `git submodule` does natively defeats the purpose of submodules and breaks standard git tooling.

```bash
# Correct: add a submodule using git directly
git submodule add https://github.com/<org>/<repo>.git apps/<repo>
git add apps/<repo> .gitmodules
git commit -m "feat: add <repo> submodule"
```

The same principle applies to all git submodule lifecycle operations (updating, removing, initialising): **use git CLI in the terminal**.

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

**Service DNS — wildcard covers all `*.yral.com` hostnames automatically:**

A single Cloudflare wildcard A record (`*.yral.com`) points to every node's public IP (all 5 CPs + all workers), DNS-only (not proxied — TLS is terminated inside the cluster at Cilium Gateway). This means **adding a new HTTPRoute for any `<name>.yral.com` hostname requires no DNS changes** — the wildcard resolves it immediately to all node IPs.

Rationale: Cilium runs a full WireGuard-encrypted mesh across all nodes. Traffic arriving at *any* node — worker or control plane — is correctly routed by Cilium's envoy to the pod running the service, regardless of which node it lands on. The wildcard + all-node IPs gives natural load distribution and fault tolerance: if a node is down, DNS clients retry other IPs.

**To expose a new service: just commit the HTTPRoute.** No Cloudflare DNS step required. Flux reconciles the route and it is immediately reachable at `https://<name>.yral.com`.

**The only hostname that is NOT covered by the wildcard** is `kubernetes-api.yral.com`, which uses explicit per-CP A records managed by the `node-remove` and `control-plane-join` roles. Do not add it to the wildcard — its records must track exactly which CP nodes are live.

**When adding or removing a node**, no service hostname DNS updates are needed (the wildcard covers all nodes automatically). Only `kubernetes-api.yral.com` requires role-managed A record updates, which the existing roles already handle.

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
3. Add an HTTPRoute in `kubernetes/networking/routes/` pointing to the oauth2-proxy Service (add a `ReferenceGrant` if the HTTPRoute and Service are in different namespaces)
4. Add the new `https://<hostname>/oauth2/callback` to the authorized redirect URIs in the Google Cloud OAuth app
5. **Add a card for the new URL in `kubernetes/apps/dashboard/configmap.yaml`** — the dashboard at `dashboard.yral.com` is the canonical list of all hosted services; every new visitable URL must appear there
6. Commit and push — Flux reconciles and the hostname is live immediately (wildcard DNS requires no Cloudflare changes)

**Dashboard — `dashboard.yral.com`:**
A dark-mode internal homepage listing every hosted UI on the cluster. It lives in `kubernetes/apps/dashboard/configmap.yaml` as a static HTML ConfigMap served by nginx. **Every time a new visitable URL is added to the cluster (or removed), update the dashboard ConfigMap in the same commit.**

**The shared `oauth2-proxy-secrets` Secret** contains `client-id`, `client-secret`, and `cookie-secret`. All oauth2-proxy Deployments reference this same Secret via env vars. It lives in the `oauth2-proxy` namespace.

**Flux readiness**: `kubernetes/` is structured as a Flux Kustomization source. Bootstrap with:
```bash
flux bootstrap github \
  --owner=dolr-ai \
  --repository=yral-bare-metal-kubernetes-cluster \
  --branch=main \
  --path=./kubernetes/clusters/yral-k8s \
  --components-extra=image-reflector-controller,image-automation-controller \
  --token-auth
```

**`--token-auth` is mandatory.** Without it, `flux bootstrap` defaults to SSH and rewrites `gotk-sync.yaml` with an `ssh://` URL. The `flux-system` Secret holds username/password credentials from the fine-grained PAT — SSH auth requires an identity key which that secret does not contain, causing the GitRepository to fail with `invalid 'ssh' auth option: 'identity' is required`. Always pass `--token-auth` to keep the URL as HTTPS and match the existing secret.

**`--components-extra=image-reflector-controller,image-automation-controller` is required** to support Flux image automation (auto-updating image tags in `deployment.yaml` when a new image is pushed to GHCR).

**`flux bootstrap` is the canonical — and only correct — way to update `gotk-components.yaml`.** It generates the manifest, commits it to git first, then applies it. This means it is git-driven under the hood (not purely imperative), and avoids the chicken-and-egg problem of controllers that don't exist yet being unable to self-reconcile a git change that adds them. Do not hand-edit `gotk-components.yaml`; re-run `flux bootstrap` instead.

**Flux reconciliation — GitHub webhook (not polling):**

Flux is configured with a GitHub webhook receiver (`kubernetes/infrastructure/flux-receiver/`). When a commit is pushed to `main`, GitHub immediately POSTs to the receiver, triggering reconciliation within seconds rather than waiting for the default 1h poll interval. This means: **a `git push` to main is sufficient to deploy a change** — no need to run `flux reconcile` manually under normal circumstances.

The receiver endpoint is exposed via an HTTPRoute on `flux-receiver.yral.com`. The webhook secret is SOPS-encrypted in `kubernetes/infrastructure/flux-receiver/`. If reconciliation does not trigger within ~30s of a push, check the receiver pod logs:
```bash
kubectl logs -n flux-system -l app=notification-controller --tail=50
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

**Removing a Flux Kustomization — always suspend first:**

Flux sets a `finalizers.fluxcd.io` finalizer on every Kustomization it manages. When a Kustomization is deleted, the kustomize-controller processes the finalizer to clean up all managed resources before the object disappears. If the Kustomization is currently in an error loop (e.g. `path not found`), the controller cannot build a resource inventory and the finalizer handler fails too — leaving the object stuck with `DeletionTimestamp` set but never completing.

**The correct procedure for removing a Kustomization from the cluster:**
```bash
# 1. Suspend it — stops reconciliation so the controller releases the finalizer cleanly
flux suspend kustomization <name> -n flux-system

# 2. Remove it from git (delete the file + its entry in kustomization.yaml) and push
# Flux prunes it on the next flux-system reconcile — no imperative delete needed
```

**Never `kubectl delete kustomization` directly.** Even with `prune: true` on the parent, a stuck child Kustomization in an error loop will not self-prune — the controller cannot process the finalizer without a valid path. Bypassing this with `kubectl delete` strips the finalizer forcibly, which can corrupt the flux-system namespace if the controller is simultaneously restarting (e.g. after a `flux bootstrap` push). This is exactly what caused the `flux-system` namespace to go Terminating in March 2026: an imperative `kubectl delete kustomization infrastructure-goldilocks` was run while the kustomize-controller was restarting after a bootstrap push, creating a finalizer deadlock that brought down the entire `flux-system` namespace and required a full re-bootstrap.

## Deployment Execution Principle

**All mutations to cluster nodes must go through Ansible roles — never via ad-hoc terminal commands or SSH.**

- ✅ Validation and status checks (read-only): run freely in the terminal — `kubectl get nodes`, `ssh root@<ip> "systemctl status kubelet"`, `ssh root@<ip> "journalctl -u kubelet -n 50"`, etc.
- ❌ Mutations via SSH: strictly prohibited — no `ssh root@<ip> "apt install ..."`, no `ssh root@<ip> "systemctl restart ..."`, no `ssh root@<ip> "kubeadm ..."`, no copying files to nodes by hand.
- ❌ Mutations via `kubectl exec`: strictly prohibited for making changes to node state.
- ❌ Mutations (installs, config changes, reboots, kubeadm operations): must live in a role task and be executed via a playbook.
- ❌ Direct `helm` CLI mutations from the terminal: strictly prohibited — no `helm upgrade`, `helm install`, `helm uninstall`, or `helm repo add` as standalone terminal mutations. Helm operations that change cluster state must be embedded in a role (the role runs `helm` on the target node via Ansible) and invoked through a playbook. The only exception is read-only Helm commands like `helm list` or `helm status`.
- ❌ `kubectl apply` / `kubectl delete` for Flux-managed resources: strictly prohibited once Flux is bootstrapped. Commit the desired state to git and let Flux reconcile. For urgent rollbacks, remove the resource from git (Flux prunes it); do not delete it imperatively. For removing a Flux **Kustomization** specifically: suspend it first (`flux suspend kustomization <name>`), then remove from git — see the "Removing a Flux Kustomization" section above.
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

**Never truncate command output:**

Never pipe terminal output through `tail`, `head`, `grep`, or any other filter that truncates it during normal command runs. The user follows along with full output in the terminal in real time — filtering hides context that matters for debugging.

```bash
# ✅ Correct — full output visible
ansible-playbook ansible/playbooks/operations/add-worker.yml -e target_host=worker-1 -v
kubectl logs -n rook-ceph job/rook-ceph-osd-prepare-worker-1

# ❌ Wrong — truncates output the user needs to follow
ansible-playbook ... -v 2>&1 | tail -5
kubectl logs ... | grep -E "configured|skip"
```

The one exception is post-hoc analysis of already-completed output (e.g. searching a stored log for a specific error after the fact), where filtering is appropriate.

**Multi-node operations are always serial — never parallel:**

Any operation that touches more than one cluster node must complete fully on each node before starting the next. This applies at every level — playbook invocations, role loops, and individual task loops within roles. Never run two operations against different nodes simultaneously.

Examples:
- CP redistribution: remove one old node → verify cluster healthy → add new node → verify → then move to next pair
- Worker provisioning: `add-worker.yml -e target_host=worker-3` → wait for completion + verify Ready → then worker-4, etc.
- Node upgrades: one node drained, upgraded, rejoined, and verified before touching the next
- Kubelet restarts after CP removal: restart worker-N, wait until Ready, then restart worker-N+1 (implemented via serial `include_tasks` loop in `node-remove`)

At the role level, whenever a loop touches multiple nodes (e.g., restarting kubelets on several workers), use `include_tasks` rather than a plain `loop:` on a single task — `include_tasks` loops are inherently serial: each iteration completes before the next begins.

Rationale: etcd quorum, Longhorn replica rebuilds, and DNS round-robin all require cluster stability between per-node operations. Running nodes in parallel risks quorum loss, data loss, or cascading failures.

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
9. Am I about to run operations against multiple nodes at the same time? → Stop. Any operation touching multiple nodes is always serial — complete each node fully (playbook done + verified Ready/healthy, or task loop iteration complete) before moving to the next. This applies at every level: playbook invocations, role loops, and task loops within roles.
10. Am I about to `kubectl delete` a Flux Kustomization? → Stop. Suspend it first (`flux suspend kustomization <name>`), then remove it from git. Never delete it imperatively — the finalizer deadlock this creates can bring down the entire `flux-system` namespace.
11. Am I about to `kubectl delete` or `kubectl apply` any Flux-managed resource? → Stop. Make the change in git and push. Flux reconciles it. Imperative API changes are either a no-op (Flux reverts them) or destructive (if the resource disappears from git inventory).

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
