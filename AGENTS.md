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
1. `init-control-plane.yml` - Bootstrap first control plane (hardcoded to control-plane-1, chains: provision → storage-setup → ssh-hardening → base-system → containerd → kubernetes → kube-vip[pre-init] → cluster-init → kube-vip[post-init] → helm → cilium)
2. `add-control-plane.yml` - Add control plane for HA (chains: provision → storage-setup → ssh-hardening → base-system → containerd → kubernetes → control-plane-join → kube-vip → node-labels). Cilium DaemonSet deploys automatically via the existing installation.
3. `add-worker.yml` - Add worker node (chains: provision → storage-setup → ssh-hardening → base-system → containerd → kubernetes → worker-join → node-labels)
4. `remove-node.yml` - Remove node from cluster (calls: `node-remove` role)
5. `upgrade-control-plane.yml` - Upgrade control plane node(s): cordon → drain → kubeadm reset → reboot → rejoin → verify. Accepts `-e target_host=<node>` for single node or runs all serially.
6. `upgrade-worker.yml` - Upgrade worker node(s): cordon → drain → delete → reboot → rejoin → verify. Accepts `-e target_host=<node>` for single node or runs all serially.

**There is no "partial install" or "apply missing component" playbook.** If a node is missing something that should have been installed during init (e.g. CNI, kube-vip, a system package), the correct fix is to re-run its full provisioning playbook from scratch, not to create a targeted playbook that applies only the missing piece.

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

### 9. Lint and Validation

- Run from workspace root: `ansible-lint ansible/playbooks/operations/`
- All playbooks must pass lint checks
- Excluded rules (in `.ansible-lint`): 
  - `role-name` (hyphenated names allowed)
  - `syntax-check[specific]` (for `{{ target_host }}` runtime variables)

### 10. Error Handling and Validation

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

**Default: always start with the latest released version of each component.**

- When adding or upgrading any component (kube-vip, Cilium, Helm, containerd, etc.), use the latest stable release.
- Only downgrade to an older version if the latest is confirmed incompatible with the current Kubernetes version, and document the reason in the role's `defaults/main.yml`.
- During Kubernetes version upgrades, upgrade other components **one at a time**, following each component's own upgrade guidance before moving to the next.
- Version pins live exclusively in `roles/<role>/defaults/main.yml` — never hardcoded in task files or playbooks.
- When a newer version is adopted after a downgrade workaround, remove the workaround comment from `defaults/main.yml`.

**Upgrade order for cluster upgrades:**
1. Upgrade one control plane node at a time (cordon → drain → upgrade → rejoin → verify)
2. Then upgrade worker nodes one at a time
3. Then upgrade cluster add-ons (kube-vip, Cilium, monitoring) individually

## Repository-Specific Patterns

### Hetzner Integration
- Provisioning uses Hetzner Robot API + installimage script
- SSH keys from vault, extracted by postCreate.sh
- Machine hostname passed to install script via environment variable

### Kubernetes Cluster
- v1.35 with kubeadm
- Stacked etcd for HA control planes
- Odd number of control planes required (1, 3, 5...)
- 5 control planes in HEL1-DC2 (Helsinki)
- kube-vip v1.0.4 for virtual IP failover
- Cilium v1.19.1 CNI with WireGuard encryption
- Serial: 1 for node operations (one node at a time)

**Deployment order:**
1. `init-control-plane.yml` — bootstrap CP-1 (includes Cilium CNI install at end)
2. `add-control-plane.yml -e target_host=control-plane-N` — for CP-2 through CP-5 (one at a time; Cilium DaemonSet auto-deploys)
3. `add-worker.yml -e target_host=worker-N` — add worker nodes (Cilium DaemonSet auto-deploys)

### Inventory Structure
- `control_plane` group: control plane hosts
- `worker_nodes` group: worker hosts
- `k8s_cluster` meta-group: all cluster nodes
- Target node specified via `-e target_host=node-name`

## Deployment Execution Principle

**All mutations to cluster nodes must go through Ansible roles — never via ad-hoc terminal commands or SSH.**

- ✅ Validation and status checks (read-only): run freely in the terminal — `kubectl get nodes`, `ssh root@<ip> "systemctl status kubelet"`, `ssh root@<ip> "journalctl -u kubelet -n 50"`, etc.
- ❌ Mutations via SSH: strictly prohibited — no `ssh root@<ip> "apt install ..."`, no `ssh root@<ip> "systemctl restart ..."`, no `ssh root@<ip> "kubeadm ..."`, no copying files to nodes by hand.
- ❌ Mutations via `kubectl exec`: strictly prohibited for making changes to node state.
- ❌ Mutations (installs, config changes, reboots, kubeadm operations): must live in a role task and be executed via a playbook.

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
