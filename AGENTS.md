# Agent Guidelines for Kubernetes Cluster Operations

Concise architectural rules and constraints for this repository. Follow to maintain consistency.

## Immutable GitOps & Ansible Separation (Hard Rule)

**All mutations to cluster state must go through Git (Flux) or Ansible playbooks/roles. Never use imperative tools on a live cluster.**

### Strictly Prohibited (no exceptions once cluster is up)
- `kubectl delete` (namespace/deployment/service/configmap/secret/pvc/kustomization/pod — except CrashLoopBackOff)
- `kubectl apply/create/patch/edit/replace/expose/autoscale/scale/set` on Flux-managed resources
- `kubectl rollout restart / set image / set env`
- `kubectl exec` (mutating), `kubectl cp`, `kubectl port-forward` (prod), `kubectl proxy`, `kubectl debug` (mutating)
- `kubectl drain/cordon/uncordon/taint/label/annotate` (use Ansible `node-*` roles or git)
- `kubectl run`
- Direct `helm` CLI mutations (install/upgrade/uninstall/repo add) — must be inside Ansible roles
- SSH mutating commands (`apt`, `systemctl restart`, `kubeadm`, manual file copies)

**Allowed read-only only:** `kubectl get/describe/logs/events/top/diff/auth/can-i`, `kubectl version`, `kubectl api-*`, etc.

**For Flux Kustomizations specifically:** Always `flux suspend kustomization <name>` first, then remove from git. Flux will prune. Never `kubectl delete` directly.

**Non-Flux exceptions (CoreDNS manifests only):** `kubectl apply -f` the committed `coredns-*-topology.yaml` files after node changes. Commit first.

## Core Architecture Principles

### 0. Kubernetes-Native First
Built-in primitives > ecosystem projects > third-party. Flux > Argo CD. CRD-native operators > agent-based tools. Gateway API over Ingress. Prefer `kubernetes/` and `kubernetes-sigs/` projects.

**Decision rule:** "Does Kubernetes/CNCF already provide this?" before choosing third-party. Document rationale for exceptions.

### 1. Immutable Infrastructure
Nodes are **never patched in place**. Reprovision from a clean slate on every run of a provisioning playbook/role.

- No "already done" guard logic in roles (provision, storage-setup, etc. always run fully).
- Correctness/safety nets only (e.g. don't double-add a btrfs device).
- A node that gets stuck is re-provisioned from scratch — never "finish the remaining steps" over SSH.

### 2. Immutable Data Operations (Stateful Stores)
Never `UPDATE`/`DELETE` on live production tables or in-place schema changes that lose data.

All migrations/enrichments (ClickHouse, Kafka topics, object storage, DB schemas) use the 4-step pattern:
1. Dual-write (new writes go to both old and new versioned object)
2. Backfill historical data
3. Validate (counts, spot checks)
4. Swap read paths + archive old (rename to `_v1`/`_archived`; drop only after confirmed production period)

Naming convention for versioned stateful objects: `<name><delimiter><version><delimiter><status>` (e.g. `events_v2_staging`, `events-v2-staging`).

### 2b. Service Component Colocalization
Every microservice owns one dedicated namespace containing:
- Its Deployment/StatefulSet
- Its database (CNPG Cluster)
- Its Secrets/ConfigMaps
- Its NetworkPolicy/ReferenceGrant + routes
- Any sidecars/init jobs specific to it

Cross-namespace is the exception (shared infra like Cilium/cert-manager/monitoring; oauth2-proxy for internal tools).

### 3–6. Role & Playbook Discipline (Atomicity)
- **Playbooks** (`ansible/playbooks/operations/`): Thin single-play wrappers only. `roles:` list or minimal `include_role` for pre/post task files. Zero conditional logic, loops, or non-role modules in playbooks.
- **Roles**: Atomic (single responsibility). No `include_role` calls inside roles. No orchestration logic. Must be independently executable. Role-specific conditionals only.
- Playbook = orchestration sequence of atomic roles (e.g. `provision` → `storage-setup` → `ssh-hardening` → `base-system` → `containerd` → `kubernetes` → `cluster-init` → ...).
- New capability → new role first, added to existing playbook. New playbook only when structurally distinct (different host target or lifecycle stage). Ask before creating.

**Reboot handling:** Only in `base-system` role (detect `/var/run/reboot-required`, `ansible.builtin.reboot`). Playbooks orchestrate across nodes.

**Multi-node ops:** Always serial (one node complete + verified before next). Use `include_tasks` loops (inherently serial) when touching multiple nodes inside a role.

## Policies

### Default-First Configuration
Prefer component defaults. Add explicit config only when a concrete problem requires deviation. Explicitly matching the default creates maintenance burden and obscures intent.

**Binary search debugging for procedural code:** When stuck after a couple of failed iterations on Dockerfiles, scripts, or any multi-step procedural code, comment out everything except the first essential step. Verify it works (locally + in-cluster). Then uncomment one line/step at a time, verifying each works before proceeding to the next. This isolates the exact failing step without rewriting from scratch.

### Component Versioning
**Always ask the user for an explicit version number from the project's releases page before adding/upgrading.** Pin exactly in `roles/<role>/defaults/main.yml` only.

- Never "latest".
- Downgrade only if latest incompatible with current K8s (document reason).
- Upgrade order: one CP at a time → workers → add-ons, one component at a time.

### Error Handling
In roles: use `fail`, `assert`, `changed_when: false`, `failed_when: false` + explicit checks.
In playbooks: none (orchestration only).

### Lint
Run `ansible-lint ansible/playbooks/operations/` before changes. Playbooks must pass.

## Repository-Specific Patterns & Rules

### Hetzner + Provisioning
- `provision` role uses Hetzner Robot API + installimage (Ubuntu).
- OS partition 50G on workers (via `provision_os_partition_size`); full disk on CPs.
- `storage-setup` is the Ceph OSD path for workers (GPT partition `rook-osd` label on nvme0n1p3 + wipe nvme1n1 + `dd` for bluestore signatures).
- `cloudflare-dns` role (at end of add-* playbooks) and `node-remove` handle DNS automatically from `cloudflare_dns_records` in `hosts.yml`.

**Full init flow (never partial):** provision → storage-setup → ssh-hardening → base-system (reboot if needed) → containerd → kubernetes → (init or join) → node-labels → cloudflare-dns.

### Kubernetes Cluster
- v1.35 kubeadm, stacked etcd, odd # CPs (currently 5, one per Helsinki building for blast radius).
- Control plane HA via Cloudflare DNS round-robin (`kubernetes-api.yral.com`).
- Cilium v1.19.1 + WireGuard encryption. Gateway API for exposure.
- Serial node operations.

**CoreDNS topology (cross-DC VXLAN unreliability):**
Maintain `kubernetes/infrastructure/coredns/coredns-*-topology.yaml` (non-Flux, kubeadm-owned). Replicas must be `>=` number of distinct zones with workers. Always run the two `kubectl apply -f` commands after adding workers in new zones. `topology-mode: "Auto"` on the Service.

### Storage (Rook/Ceph)
Rook v1.19.3 + Ceph v20.2.1 (Tentacle). Default SC: `ceph-block` (2× replication, host failure domain, ~16.5 TB usable).

**Worker disk layout (post-reprovision):** 50G btrfs OS | Ceph partition (`rook-osd` label) + whole-disk OSD.
- `storage-setup` always runs the destructive path for workers.
- `wait: false` on the cluster Kustomization (OSD updates are long-running; do not block dependent workloads).
- Monitor with `ceph status` / `ceph osd status` / `ceph df` via toolbox.
- **Never** put "ceph" in PARTLABEL. Always run the `dd` wipe step for bluestore signatures.

Kafka is the explicit exception (`ceph-block-1replica`).

### PostgreSQL
**CloudNativePG operator only** (CRD-native, per-service model). No shared cluster-wide Postgres.

- One `postgresql.cnpg.io/v1 Cluster` per microservice in its own namespace.
- App credentials as `*.sops.yaml`.
- Use `-rw` endpoint.
- Same-region podAffinity for latency-sensitive app+DB pairs (requiredDuringSchedulingIgnoredDuringExecution on `cnpg.io/cluster` label + `topology.kubernetes.io/region`).
- Operator in `kubernetes/infrastructure/cloudnative-pg/`.

### Backups
Velero only (full cluster DR, 30-day self-managed `ttl: 720h`). Named prefix `velero/` in the bucket. No bucket lifecycle policy (Velero GC handles it).

### Networking & Exposure Model
- Everything exposed via Cilium Gateway (Gateway + HTTPRoutes or TLSRoutes).
- User-facing: direct to app Service.
- Internal tools (no built-in auth): one `oauth2-proxy` Deployment/Service per app (in `oauth2-proxy` namespace). Shared `oauth2-proxy-secrets`.
- Wildcard `*.yral.com` (orange cloud) + all-node A records = adding an HTTPRoute is sufficient; no DNS change needed.
- Kafka: TLSRoute (SNI passthrough, per-broker hostnames), grey-cloud DNS only.
- **ReferenceGrant (cross-namespace routes):** When an HTTPRoute/TLSRoute in namespace A references a Service in namespace B, a `ReferenceGrant` must exist in the **target** namespace (B) allowing it. Without it, Cilium Gateway silently returns HTTP 500 for all requests (`ResolvedRefs: False, RefNotPermitted`). Always add the ReferenceGrant in the same commit as the HTTPRoute when they span namespaces.

- **NetworkPolicy rule:** Never rely on `fromCIDRSet`/`namespaceSelector`/`podSelector` for gateway-originated traffic (cilium-envoy runs hostNetwork; appears as node IP). Use an explicit ingress rule with only a `ports:` clause (app-level auth does the real enforcement).

**Dashboard:** Update `kubernetes/apps/dashboard/index.html` only for internal tools (oauth2-proxy-gated or otherwise internal). Public/user-facing services are NOT listed on the dashboard.

### Local Environment & Parity
The repo uses a single monorepo-wide `mise.toml` at the repository root as the source of truth for tool versions and task orchestration. Avoid adding per-project `mise.toml` files unless there is a strong, documented reason.

**mise (not direnv)** for env var management and tool versioning. The root `mise.toml` loads `.env` (gitignored, generated from vault) via `_.file = '.env'`. Plaintext environment values belong in the `mise.toml` `[env]` section; secrets stay out of version control and should be provided through `.env`, `mise.local.toml`, or a secret manager such as fnox.

**Workflow tasks live in `mise.toml`, not bash scripts.** All setup, build, test, and run workflows are `mise` tasks (`mise tasks` to list). `scripts/setup-local-env.sh` is a thin `exec mise run setup` wrapper for compatibility only — do not add logic there. New workflow steps → new/updated `mise` task. Never create bespoke bash scripts for repo workflows.

**Version locking:** `mise.lock` is committed. Bump tool versions explicitly in `mise.toml`, then `mise lock` to refresh the lockfile. Never use floating `"latest"` without a lockfile entry.

**Declarative-only tooling (no imperative system installs):** The local environment must be fully reproducible from `mise.toml` alone — a Nix-like experience. Never run `sudo apt-get install`, `brew install`, or any imperative system package manager command to satisfy a build dependency. If a tool or library is needed:
1. Check if mise provides it (add to `[tools]` in `mise.toml`).
2. If it's a Rust/Cargo tool, install via `cargo binstall` in a mise task.
3. If it's a system C library, linker, or other OS package, declare it in `[bootstrap.packages]` (e.g. `"apt:musl-tools" = "latest"`). mise handles the install via `mise bootstrap packages apply` — OS-filtered so apt entries are ignored on macOS, brew entries on Linux, etc. List entries alphabetically by key for easy scanning and editing.

This ensures `mise bootstrap --yes && mise run setup` is the only command needed to go from a fresh machine to a working environment.

### GPU (Vast.ai)
`vastai-provision` role + playbook (not Kubernetes). Always ≥2 replicas on distinct offers. Shared infra SSH key attached. Override vars at invocation (never new playbook).

### Inventory
`control_plane` / `worker_nodes` / `k8s_cluster`. Target via `-e target_host=...`.

### Ansible / Playbook Execution
- `become` is globally false; remote plays SSH as root; localhost plays as vscode user.
- Always run playbooks in foreground.
- **Never truncate or filter terminal output during runs** — do not use `tail`, `head`, `grep`, pipes, or similar tools that cut off output. Run commands directly and let the full output stream so we can follow along together. `tail`/`head` only for post-hoc analysis after a run completes. This applies to `docker build`, `kubectl logs`, and all other long-running commands — stream full output, don't pipe through filters.
- Short poll loops (≤10s sleep) when waiting. Never use long sleeps (e.g. 120s) — instead poll with short intervals and re-check, or stop and let the user reinitiate.
- Lint before PRs.

### Flux
Bootstrap with `--token-auth --components-extra=image-reflector-controller,image-automation-controller`.
Webhook receiver for near-real-time reconcile on push.
`dependsOn` for ordering.
Manual apply order only before bootstrap (and only for non-Flux resources).
**No manual `flux reconcile` needed** — the webhook receiver triggers reconciliation within seconds of a git push. Only use `flux reconcile` if the webhook is down or for debugging.

### Image Registry (Harbor)
In-cluster Harbor at `harbor.yral.com` is the registry for custom-built app images (NOT bootstrap infra images — those stay on GHCR/quay.io for disaster recovery).

**Simplicity over fine-grained access control:** This is a single-contributor repo/cluster. Use the Harbor admin credentials directly for CI pushes, Flux image scanning, and pod image pulls. Store as a SOPS-encrypted `kubernetes.io/dockerconfigjson` Secret (e.g., `harbor-pull.sops.yaml`) in each app namespace. Admin password lives in `kubernetes/infrastructure/harbor/harbor-admin-secret.sops.yaml`.
- Every namespace pulling from Harbor must have an `imagePullSecrets` reference in the Deployment/StatefulSet spec pointing to that Secret.
- Use Flux `ImageRepository` + `ImagePolicy` + `ImageUpdateAutomation` for automatic tag updates from Harbor into git manifests.

### Image Building (Shipwright + BuildKit)
In-cluster image building via Shipwright (CRD-native operator, CNCF Sandbox) wrapping BuildKit (rootless). Tekton Pipelines is the execution engine. Triggered by Flux on git push — no external CI needed for building app images.

- Shipwright operator in `kubernetes/infrastructure/shipwright/`, Tekton in `kubernetes/infrastructure/tekton-pipelines/` (raw release YAML committed to repo, gotk-components.yaml style).
- Define a `Build` + `BuildRun` CR per app (source: git, strategy: buildkit ClusterBuildStrategy, output: Harbor).
- BuildKit runs rootless (non-privileged, UID 1000) — no `privileged: true` needed.
- Pin versions: Tekton v1.12.2 LTS (Shipwright v0.20.3 supports v1.3/v1.6/v1.9/v1.12 — NOT v1.14+), Shipwright v0.20.3, BuildKit v0.31.1.

**Test locally before deploying:** Always build and run container images locally to validate the binary starts and the health endpoint responds before triggering a Shipwright BuildRun. This dramatically reduces the dev loop time compared to waiting for a full in-cluster build to discover runtime failures.

**Container runtime:** Use `podman` (managed by the root `mise.toml`) for local container tasks — `podman build`, `podman run`, `podman logs`. The `mise run yral-auth-image` and `mise run yral-auth-image-run` tasks wrap the standard build/run flow. Use `--platform linux/amd64` with `podman run` to run the exact same x86_64 images that run in the cluster — this ensures parity between local testing and production. Log into Harbor locally with `echo "$HARBOR_PASS" | podman login harbor.yral.com -u admin --password-stdin`.

## Questions for Agents (When in Doubt)

1. Is this logic in a role? → Move it.
2. Is the playbook a thin wrapper? → Extract to role.
3. Does this mutate existing nodes? → Reprovision from scratch (immutability).
4. Could this trigger a reboot? → Ensure `base-system` handles it.
5. Can this be tested independently? → Refactor if coupled.
6. Is this a mutation? → Role + playbook. Never SSH, `kubectl exec`, terminal `helm`/`kubectl apply` (Flux resources), etc.
7. About to create a new playbook? → Fit into existing via a new atomic role, or ask first.
8. About to SSH-mutate a node? → Stop. Role it.
9. About to touch multiple nodes? → Stop. Serial only; complete + verify before next.
10. About to `kubectl delete` a Flux Kustomization? → Suspend first, then git rm.
11. About to `kubectl apply/delete` a Flux-managed resource? → Make the change in git and push.

## Deployment / Upgrade Workflow

Full clean playbook run (no partial apply). On failure: diagnose read-only, fix the role, re-run the entire playbook from scratch.

**When a step fails:**
1. Read-only investigation only.
2. Fix the corresponding role/task.
3. Re-run full playbook.
4. Never "apply the fix by hand on the node".

## Maintaining This Document

Add to / update AGENTS.md only for new patterns, clarifications that prevent repeated mistakes, or new core constraints.

Do **not** add one-off workarounds, temporary fixes, or incident-specific notes (those belong in role READMEs).

Process: identify section, make minimal prescriptive update, remove obsolete guidance immediately.

---

**Original document was verbose with heavy duplication (especially forbidden kubectl lists, repeated YAML role/playbook examples, and long diagnostic narratives). This version collapses redundancy while preserving every hard rule, decision question, and operational constraint that actually drives behavior.**