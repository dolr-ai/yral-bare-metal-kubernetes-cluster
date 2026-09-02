# AGENTS.md — Conventions & Guidance for AI Agents

This document is the authoritative reference for AI agents and contributors working on the
Hetzner bare-metal fleet automation. It captures hard-won conventions, architectural decisions,
and repo-specific constraints that must be respected to keep the fleet automation consistent,
idempotent, and safe.

This directory (`apps/hetzner-bare-metal-fleet`) is an **in-repo directory** of the
yral-bare-metal-kubernetes-cluster repository (inlined 2026-09-02 from the archived standalone
repo `dolr-ai/hetzner-bare-metal-fleet`). There is no per-directory `mise.toml`, `fnox.toml`,
`pyproject.toml`, or `setup.sh` — all tooling is centralized in the parent repo's root
`mise.toml` (`fleet-*` tasks) and root `fnox.toml`, per the parent repo's conventions.
The fleet shares the parent repo's Ansible vault (same password, same infra SSH key):
`ansible.cfg` points its `vault_password_file` at `../../ansible/.vault_pass` and its
`private_key_file` at `../../.yral-infra-ed25519` (the fleet dir is 2 levels below the repo
root; both created by the parent repo's `mise run bootstrap`). The fleet keeps its own `.venv`
(gitignored, created by `mise run fleet-bootstrap`) and its own `ansible.cfg` (inventory,
roles_path, vault password).

---

## Overview

This directory manages a fleet of Hetzner bare-metal servers using Ansible.
All servers run Ubuntu 26.04, are reachable as `root` via SSH key, and belong to the `bare_metal`
inventory group.  All hosts are treated equally — there are no sub-groups with special behaviour.

| Group | Purpose |
|---|---|
| `bare_metal` | All 27 bare-metal hosts — primary target for all plays |

Monitoring hub: the Beszel hub runs on the yral-bare-metal-kubernetes-cluster as a Flux-managed Deployment, accessible at `https://beszel.yral.com`. Agents on each fleet host connect to this hub.

---

## Directory Layout

```
ansible/
  ansible.cfg                   # roles_path = ansible/roles; vault/SSH key point at the parent repo root
  inventory/
    hosts.yml                   # single source of truth for hosts and IPs
    group_vars/all/vars.yml     # non-secret shared variables
    group_vars/all/vault.yml    # encrypted vault (ansible-vault AES256, parent repo's vault password)
    group_vars/bare_metal.yml   # connection defaults (ansible_user: root, key path at parent repo root)
  roles/                        # all task logic lives here
  playbooks/                    # thin wrappers and orchestrator playbooks
  files/                        # (legacy) — authoritative files now live inside roles
  .vault_pass                   # NOT used — vault password comes from ../../ansible/.vault_pass (parent repo)
../../mise.toml                 # fleet-* tasks (fleet-bootstrap, fleet-maintenance, fleet-provision,
                                #   fleet-ssh-access, fleet-lint, fleet-validate) live in the parent
                                #   repo's root mise.toml, with dir = apps/hetzner-bare-metal-fleet
../../fnox.toml                 # secrets live in the parent repo's root fnox.toml
../../.yral-infra-ed25519       # infra SSH key (gitignored at parent root, created by parent's mise run bootstrap)
```

---

## Core Architecture: Roles + Thin Playbooks

**All task logic lives in roles under `ansible/roles/`.  Playbooks are thin wrappers.**

### Roles

| Role | Purpose |
|---|---|
| `hetzner_rescue` | Activates Hetzner rescue mode via Robot API, reboots, waits for SSH |
| `bare_metal_provision` | Runs installimage in rescue, reboots into OS, sets up btrfs RAID 0 |
| `system_update` | `apt full-upgrade`, autoremove, optional reboot |
| `ssh_security` | Hardens sshd config; resets `authorized_keys` to canonical set |
| `docker` | Idempotent Docker CE install via official upstream repo |
| `beszel_agent` | Upserts `beszel-agent` service in `/root/docker-compose.yml` |
| `ssh_key_grant` | Adds a single team member's key — temporary; revoked on next weekly run |

Each role follows the standard Ansible structure:

```
roles/<name>/
  defaults/main.yml   # overridable defaults
  tasks/main.yml      # all task logic
  handlers/main.yml   # only if role needs handlers (currently: ssh_security)
  files/              # static assets copied to remote (currently: ssh_security, beszel_agent)
```

**Roles must be atomic** — a role does exactly one concern.  Do not combine unrelated operations
(e.g., do not mix Docker install with system update in one role).

### Primary Playbooks

Three playbooks cover all intended operations — there are no other playbooks in `ansible/playbooks/`:

| Playbook | Purpose | Key flags | Invocation |
|---|---|---|---|
| `provision.yml` | Full idempotent bootstrap of a new host | `skip_rescue_activation=true`, `force_provision=true` | `mise run fleet-provision` (local only) |
| `ssh-access.yml` | Grant temporary SSH access to a team member | `team_member_name=<name>` (required) | `mise run fleet-ssh-access` (local only) |
| `weekly-update.yml` | Weekly maintenance: update → agent refresh → rolling reboot → key reset | `enable_reboot=true` | `mise run fleet-maintenance` (local) + weekly GitHub Actions cron (`.github/workflows/fleet-maintenance.yml` at the parent repo root) |

---

## Variable Conventions

### Vault Pattern

- **Group-level secrets** live in `ansible/inventory/group_vars/all/vault.yml` (encrypted).
- All vault variables are prefixed `vault_*`.
- Plain `vars.yml` files only contain `key: "{{ vault_key }}"` references — never raw secrets.
- The vault password file is the **parent repo's** `ansible/.vault_pass` (600 permissions,
  gitignored) — referenced from this directory as `../../ansible/.vault_pass`.
- Per-host `vault.yml` files are not used — all secrets are group-level.

Example pattern:
```yaml
# group_vars/all/vars.yml
beszel_agent_token: "{{ vault_beszel_agent_token }}"

# group_vars/all/vault.yml  (encrypted)
vault_beszel_agent_token: "actual-token-here"
```

### Role Defaults

- Each role keeps its own overridable defaults in `defaults/main.yml`.
- Variables already defined in `group_vars/all/vars.yml` (e.g., `beszel_hub_url`,
  `beszel_listen_port`, `beszel_ssh_key`) are **not** duplicated in role `defaults/main.yml` —
  the group vars take precedence and roles inherit them automatically.
- Role `defaults/main.yml` is for role-private knobs only (paths, retry counts, feature flags).

### Target Override

All playbooks accept an optional `target` variable to scope execution without using `--limit`:

```yaml
hosts: "{{ target | default('all') }}"
```

Standard invocation still uses `--limit`; `target` is an alternative for programmatic callers.

---

## SSH and authorized_keys Management

**Canonical set** (always present on every host):

- `github-actions@yral.com` — CI/CD pipeline key
- `saikatdas0790@gmail.com` — admin key

These are stored in `ansible/roles/ssh_security/files/authorized_keys`.

**Team member keys** are defined in `group_vars/all/vars.yml` under `team_members`.
They are added temporarily via the `ssh_key_grant` role and **automatically expelled** the next time
`ssh_security` runs (weekly-update play 4, or manual `weekly-update.yml` run).

> **Never add team member keys to the canonical `authorized_keys` file.**
> Temporary access is the intentional model; canonical access requires adding keys to the file
> in the role's `files/` directory and committing the change.

---

## Idempotency Requirements

Every role and playbook must be safe to re-run without side effects.

Key patterns used:

| Concern | Pattern |
|---|---|
| Already-provisioned host | Marker file `/root/.provisioned`; fail unless `force_provision=true` |
| Docker already installed | `docker --version` check; all install tasks are `when: docker_check.rc != 0` |
| Beszel agent config | `blockinfile` with named marker; Python cleanup script removes stale blocks first |
| SSH keys | `lineinfile` + grep-based existence check before inserting |
| `authorized_keys` reset | Content diff check; only copies when canonical ≠ existing |
| apt upgrades | `cache_valid_time: 3600`; skips upgrade when `upgradable_packages == 0` |

---

## Execution Patterns

### Execution and Serial Behaviour

- `weekly-update.yml` **Play 1** (apt upgrade + beszel agent): runs **parallel** across all hosts — no `serial`
- `weekly-update.yml` **Play 2** (reboot): `serial: 1`, `max_fail_percentage: 50` — rolls one host at a time; skipped per-host when not required
- `ssh_security` play: `max_fail_percentage: 50` — tolerates up to half failing
- All other plays run parallel (no `serial`)

### Connection reliability

Every role that touches a remote host via apt/systemd starts with:

```yaml
- name: Wait for system to be ready
  wait_for_connection:
    timeout: 60
    delay: "{{ retry_delay }}"   # default: 10
  retries: "{{ retry_count }}"   # default: 3
  register: connection_result
  until: connection_result is succeeded

- name: Gather facts
  setup:

- name: Check if system is Debian/Ubuntu
  fail:
    msg: "This role is designed for Debian/Ubuntu systems only"
  when: ansible_os_family != "Debian"
```

Roles that only run locally (e.g., `hetzner_rescue`) or only use raw/shell before
`gather_facts` omit this block.

### Become

`become: false` is the default — we SSH directly as root.  Only use `become: true` in the
`ssh_security` play wrapper where legacy usage required it; individual role tasks do not need it.

---

## Beszel Monitoring

The Beszel hub runs on the yral-bare-metal-kubernetes-cluster as a Flux-managed
Deployment (see `kubernetes/infrastructure/beszel/` in that repo). This repo only
manages the agents that run on each fleet host.

- **Agent token**: `beszel_agent_token` — universal token from `group_vars/all/vault.yml`
  (`vault_beszel_agent_token`). Allows agents to auto-register without prior system creation.
  No per-host token overrides are used.
- **Hub URL**: `beszel_hub_url` — `https://beszel.yral.com` (defined in `group_vars/all/vars.yml`).
  Unchanged from when the hub ran on `uptime-monitor-1`; now served by the k8s cluster.
- The agent compose block uses `blockinfile` with marker `ANSIBLE MANAGED BLOCK - BESZEL AGENT`
  and the cleanup script `roles/beszel_agent/files/cleanup_beszel.py` removes stale entries.

---

## Adding a New Host

1. Add host entry to `ansible/inventory/hosts.yml` under `bare_metal`.
2. Run provisioning (from the parent repo root — mise tasks set the fleet directory as cwd):
   ```bash
   mise run fleet-provision -- --limit <hostname>
   ```

The universal `beszel_agent_token` from `group_vars/all/vault.yml` is used automatically.
No per-host `vars.yml` or `vault.yml` is needed unless the host requires host-specific overrides.

---

## Adding a Team Member

1. Add entry to `team_members` dict in `ansible/inventory/group_vars/all/vars.yml`:
   ```yaml
   team_members:
     newperson:
       email: "newperson@gobazzinga.io"
       ssh_key: "ssh-ed25519 AAAA... newperson@gobazzinga.io"
   ```
2. Grant temporary access:
   ```bash
   mise run fleet-ssh-access -- --limit <hostname|group> -e team_member_name=newperson
   ```
3. Access is automatically revoked on the next `weekly-update.yml` run.

---

## Local Development Setup (mise + fnox — centralized in the parent repo)

All dev tooling is managed **declaratively by mise** in the parent repo's root `mise.toml`
(per the parent repo's conventions — no per-directory config files). The fleet's tasks there
run with `dir = apps/hetzner-bare-metal-fleet` so the fleet's `ansible.cfg` is picked up:

- `mise run fleet-bootstrap` — create the fleet `.venv` (ansible-core + ansible-lint,
  pip-installed) and extract the infra SSH key to `../../.yral-infra-ed25519` (idempotent)
- `mise run fleet-maintenance -- --limit <host|bare_metal> [-e enable_reboot=false]` — weekly maintenance
- `mise run fleet-provision -- --limit <host>` — provision a host
- `mise run fleet-ssh-access -- --limit <host|group> -e team_member_name=<name>` — temporary access
- `mise run fleet-lint` — ansible-lint on this directory
- `mise run fleet-validate` — read-only inventory decryption + graph validation
- `mise run fleet-vault-decrypt` / `fleet-vault-encrypt` — decrypt/re-encrypt the fleet
  vault for editing (same pattern as the parent repo's `ansible-vault-*` tasks)
- `mise run fleet-vault-view -- <yaml key>` — extract a single value from the fleet vault

Secrets live in the parent repo's root `fnox.toml` (age-encrypted, safe to commit); the age
provider key is the infra SSH key extracted from the fleet vault to
`../../.yral-infra-ed25519` (gitignored at the parent root) by `mise run fleet-bootstrap`
(the root `bootstrap` task extracts the same key material from the parent vault).

**First-time setup (run in the parent repo root):**
```bash
# 1. Activate mise in your shell (once — managed by `mise bootstrap` in ~/.dotfiles)
eval "$(mise activate bash)"   # or zsh/fish

# 2. Create the parent repo's vault password file (placeholder — replace with real password)
echo 'real-vault-password' > ansible/.vault_pass
chmod 600 ansible/.vault_pass

# 3. Bootstrap the parent repo (tools, .venv, infra SSH key, kubeconfig, submodules)
mise run setup

# 4. Bootstrap the fleet's own .venv + infra SSH key (ansible-core + ansible-lint)
mise run fleet-bootstrap
```

### Auto-setup

The fleet's `.venv` is created on demand: every `fleet-*` mise task declares
`depends = ["fleet-bootstrap"]`, so the venv and infra SSH key are (re)installed
automatically before the playbook runs if missing or stale. `fleet-bootstrap` is
idempotent — mise skips it when its `sources` (`ansible/requirements.yml`,
`../../ansible/.vault_pass`) are unchanged and `outputs` (`.venv`,
`../../.yral-infra-ed25519`) exist.

---

## Naming Conventions

| Item | Convention | Example |
|---|---|---|
| Roles | `snake_case` | `beszel_agent`, `ssh_security` |
| Playbooks | `kebab-case.yml` | `weekly-update.yml`, `ssh-access.yml` |
| Variables | `snake_case` | `beszel_agent_token`, `enable_reboot` |
| Vault vars | `vault_` prefix | `vault_beszel_agent_token` |
| Host names | `kebab-case` | `clickhouse-keeper-1`, `uptime-monitor-1` |
| Role file assets | `snake_case` or `kebab-case` matching their purpose | `cleanup_beszel.py`, `authorized_keys` |

---

## Declarative over Imperative

**Always prefer declarative configuration over imperative scripts.**

- Tools, versions, env vars, and tasks are declared in the parent repo's root `mise.toml`
  (`fleet-*` tasks) — not installed via ad-hoc shell commands.
- Secrets are declared in the parent repo's root `fnox.toml` — not injected via `.env` files or runtime scripts.
- When adding a new tool or dependency, add it to the parent repo's `mise.toml` `[tools]` or the
  fleet's `ansible/requirements.yml` rather than calling `brew install`, `pip install`, or
  `apt install` in a script.
- When adding a new secret, use `fnox set` to encrypt it into the parent repo's root
  `fnox.toml` rather than creating `.env` files.

---

## What NOT to Do

- **Do not put task logic in playbooks.** Playbooks call roles; roles contain tasks.
- **Do not duplicate group_vars variables in role defaults.** Use `defaults/main.yml` only for
  role-local overrides.
- **Do not add team members to the canonical `authorized_keys` file** unless they are permanent
  service accounts.
- **Do not run `provision.yml` without `--limit`** unless intentionally reprovisioning the whole
  fleet (destructive).
- **Do not remove `serial: 1` from system update plays** — rolling updates protect the fleet from
  simultaneous reboots.
- **Do not commit vault password files** (`ansible/.vault_pass` anywhere in the parent
  repo) — they are gitignored by design.
- **Do not inline task logic in `when:` conditions that belong in role `defaults/main.yml`** —
  keep booleans in defaults, reference them in tasks.
