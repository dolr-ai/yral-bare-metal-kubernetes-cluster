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

**For Flux Kustomizations specifically:** Prefer committing the change to git and pushing — Flux's garbage collector prunes removed resources on the next reconcile (the webhook triggers this within seconds of a push; the 1m interval is the backstop). No manual `flux suspend` is needed for ordinary resource removals (HTTPRoutes, Deployments, Services, etc.). Suspend the Kustomization first only when: (a) removing the `Kustomization` CR itself (you want its controller stopped before its own deletion), or (b) touching resources an ImageUpdateAutomation pipeline writes back to (to avoid commit-overwrite races between manual reconcile and IUA). Never `kubectl delete` a Flux-managed resource directly — always remove from git and let Flux prune.

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

### Missing CLI tools
If a CLI tool is needed but not installed, do NOT use `sudo apt-get install` or any system package manager. Instead:
1. Check if mise provides it — add to `[tools]` in `mise.toml` (e.g. `dig`/`nslookup` via `"apt:bind9-dnsutils"` in `[bootstrap.packages]`).
2. If it's a system C library or OS package, add to `[bootstrap.packages]` in `mise.toml`.
3. Run `mise install` (for tools) or `mise bootstrap packages apply --yes` (for system packages).
4. If mise refuses to run (untrusted directory), run `mise trust` first, then retry.
This keeps the environment fully declarative and reproducible from `mise.toml` alone.

### Edit Tooling Preference (Hard Rule)
Never use terminal commands (`sed`, `awk`, `echo >`, `cat >`, `tee`, etc.) for text edits. Always use the VS Code edit tools (`replace_string_in_file`, `multi_replace_string_in_file`, `create_file`) so changes are visible in the diff editor for review. Terminal commands bypass the review workflow and can silently corrupt files.

### Default-First Configuration
Prefer component defaults. Add explicit config only when a concrete problem requires deviation. Explicitly matching the default creates maintenance burden and obscures intent.

### Docs-First for Tooling (Hard Rule)
When using a tool or library for the first time (or encountering a non-trivial configuration question), **always check the official documentation first** before trial-and-error. Find the canonical/prescribed way to do it, implement that, and record it in AGENTS.md under the relevant tooling section. Never guess at API shapes, configuration fields, or patterns — fetch the docs, read the prescribed approach, and follow it exactly. This prevents hours of wasted iteration on wrong assumptions.

**Binary search debugging for procedural code:** When stuck after a couple of failed iterations on Dockerfiles, scripts, or any multi-step procedural code, comment out everything except the first essential step. Verify it works (locally + in-cluster). Then uncomment one line/step at a time, verifying each works before proceeding to the next. This isolates the exact failing step without rewriting from scratch.

### Component Versioning
**Always ask the user for an explicit version number from the project's releases page before adding/upgrading.** Pin exactly in `roles/<role>/defaults/main.yml` only.

- Never "latest".
- Downgrade only if latest incompatible with current K8s (document reason).
- Upgrade order: one CP at a time → workers → add-ons, one component at a time.

### Latest Stable Preference
When making changes to a codebase (fixing bugs, migrating APIs, adding features), strive to bump all affected dependencies to their latest stable versions. If a change breaks a dependent, upgrade the dependent too — don't pin to an older version to avoid the migration. This keeps the codebase current and reduces accumulated technical debt. Always verify compilation locally before pushing.

**Never revert backwards.** When a dependency has already been bumped (even by an automated tool like Dependabot/Renovate), never revert it to an older version to work around a break. Instead, fix the breakage by upgrading the *downstream* consumer (e.g. if `syn` 3 breaks `build.rs`, upgrade `prettyplease` to a syn-3-compatible version and fix the API change — do not revert `syn` to 2.x). Pinning to old versions accumulates technical debt and defeats the purpose of automated dependency management. Only revert if the user explicitly requests it.

### Sweeping Changes — Per-Component Verification (Hard Rule)
When making sweeping changes (removing a feature, restructuring workspaces, bumping shared deps, etc.) that touch multiple components, **verify each affected component individually** before pushing:
1. **Compile** — `cargo check` / `cargo build` / `mise run <app>-build` for each affected component.
2. **Test** — `cargo test` / `mise run <app>-test`. Tests requiring external services (Redis, IC canisters) that fail with "Connection refused" are expected locally; verify no *new* test failures from the change.
3. **Run locally** — `mise run <app>-run` (via pitchfork) to verify the app starts and the health endpoint responds.
4. **Push** — once all components pass, push to git and let CI/CD handle deployment.
5. **Validate on prod** — after deployment, verify the service is healthy in production (read-only `kubectl get/describe/logs`, health endpoint, smoke test).
Never push sweeping changes without first verifying every affected component compiles and runs locally.

### Alphabetical Dependency Ordering
In all `Cargo.toml` (and other manifest) files, list dependencies **alphabetically by key** within each `[dependencies]` section. Merge third-party and local/path deps into a single list (no separator comments). This makes it easier for human reviewers to find and parse dependency lists, and avoids duplicate entries. Apply this to `[workspace.dependencies]`, `[dependencies]`, `[dev-dependencies]`, and `[build-dependencies]` sections alike.

### Error Handling
In roles: use `fail`, `assert`, `changed_when: false`, `failed_when: false` + explicit checks.
In playbooks: none (orchestration only).

### Lint
Run `ansible-lint ansible/playbooks/operations/` before changes. Playbooks must pass.

## Repository-Specific Patterns & Rules

### Hetzner + Provisioning
- `provision` role uses Hetzner Robot API + installimage (Ubuntu).
- OS partition 50G on workers (via `provision_os_partition_size`); full disk on CPs.
- `storage-setup` is the Ceph OSD path for workers (see `kubernetes/infrastructure/rook-ceph/cluster/cephcluster.yaml` for disk layout details).
- `cloudflare-dns` role (at end of add-* playbooks) and `node-remove` handle DNS automatically from `cloudflare_dns_records` in `hosts.yml`.
- `namecheap-dns` role + `update-namecheap-dns.yml` playbook manages custom nameserver glue records at Namecheap from `namecheap_nameserver` per-host annotations in `hosts.yml`.

**Full init flow (never partial):** provision → storage-setup → ssh-hardening → base-system (reboot if needed) → containerd → kubernetes → (init or join) → node-labels → cloudflare-dns.

### Kubernetes Cluster
- kubeadm, stacked etcd, odd # CPs (currently 5, one per Helsinki building for blast radius).
- Control plane HA via Cloudflare DNS round-robin (`kubernetes-api.yral.com`).
- Cilium + WireGuard encryption. Gateway API for exposure.
- Serial node operations.
- CoreDNS topology: see `kubernetes/infrastructure/coredns/coredns-*-topology.yaml` (non-Flux, kubeadm-owned). Run `kubectl apply -f` after adding workers in new zones.
- Storage (Rook/Ceph): see `kubernetes/infrastructure/rook-ceph/cluster/cephcluster.yaml` for versions, disk layout, and operational rules.
- PostgreSQL (CloudNativePG): see `kubernetes/infrastructure/cloudnative-pg/helmrelease.yaml` for the per-service model and conventions.

### Backups
Velero only (full cluster DR, 30-day self-managed `ttl: 720h`). Named prefix `velero/` in the bucket. No bucket lifecycle policy (Velero GC handles it).

### Networking & Exposure Model
- Everything exposed via Cilium Gateway (Gateway + HTTPRoutes or TLSRoutes).
- DNS: wildcard + all-node A records (all workers + CPs). Adding an HTTPRoute is sufficient; no DNS change needed. See `kubernetes/networking/gateway.yaml` and `kubernetes/infrastructure/external-dns/dnsendpoint-saikat-dev.yaml` for details.
- **No L2 Announcements:** Cilium L2 announcements are intentionally disabled. L2 ARP/NDP only works within a single L2 broadcast domain — our cluster spans multiple DCs and will include nodes from other providers. The only network requirement for adding nodes is internet connectivity with a static IP. Service exposure uses `externalIPs` (L3, cross-provider via VXLAN tunnel) and LoadBalancer services. Do not enable `l2announcements` without revisiting this constraint.
- **ReferenceGrant (cross-namespace routes):** When an HTTPRoute/TLSRoute in namespace A references a Service in namespace B, a `ReferenceGrant` must exist in the **target** namespace (B) allowing it. Without it, Cilium Gateway silently returns HTTP 500 for all requests (`ResolvedRefs: False, RefNotPermitted`). Always add the ReferenceGrant in the same commit as the HTTPRoute when they span namespaces.
- **NetworkPolicy rule:** Never rely on `fromCIDRSet`/`namespaceSelector`/`podSelector` for gateway-originated traffic (cilium-envoy runs hostNetwork; appears as node IP). Use an explicit ingress rule with only a `ports:` clause (app-level auth does the real enforcement).
- **SOPS encryption mechanism:** To encrypt `*.sops.yaml` files, extract the age key from Ansible vault (`ansible-vault view ansible/inventory/group_vars/all/vault.yml | grep AGE-SECRET-KEY`), write to temp file, then `SOPS_AGE_KEY_FILE=/tmp/sops-age.key sops --encrypt --in-place <file>.sops.yaml`. The `.sops.yaml` creation rules in repo root handle age key selection automatically — no need to pass `--age` explicitly. Always clean up the temp key file after.

### Local Environment & Parity
The repo uses a single monorepo-wide `mise.toml` at the repository root as the source of truth for tool versions and task orchestration. Avoid adding per-project `mise.toml` files unless there is a strong, documented reason.

**mise (not direnv)** for env var management and tool versioning. Plaintext environment values belong in the `mise.toml` `[env]` section; secrets are managed by **fnox** (age-encrypted, committed to git in `fnox.toml`). No `.env` file is generated or loaded — `_.file = '.env'` was removed.

**Secret management (fnox + age) — two separate keys:**

There are **two distinct age keys** in this repo. Do not confuse them:

**Key 1 — fnox (SSH ed25519 → age identity):**
- fnox uses the infra SSH ed25519 private key, which it internally converts to an age identity.
- Private key: `vault_github_actions_ssh_private_key` from Ansible vault, extracted to `./.yral-infra-ed25519` (repo-root, gitignored) by `mise run bootstrap`. **Not** `~/.ssh/id_ed25519` — that's the user's personal key.
- `FNOX_AGE_KEY_FILE = './.yral-infra-ed25519'` is set in `mise.toml [env]` — fnox picks it up automatically. No manual `export` needed.
- `fnox.toml` at repo root (and per-app `fnox.toml` files) define encrypted secret definitions. The encrypted ciphertext is safe to commit to git.
- Rotating a secret: `fnox set <KEY> --provider age` (re-encrypts in `fnox.toml`, commit the change). For multi-line values (PEM keys/certs), pipe via stdin: `printf '%s' "$VALUE" | fnox set <KEY> --provider age`.
- Running commands with secrets: `fnox exec -- <command>`, or `fnox export -f env -o .env` for tools that need a file (e.g. `podman --env-file`).
- `fnox.local.toml` is gitignored for machine-specific overrides.

**Key 2 — SOPS (native age key):**
- SOPS-encrypted `*.sops.yaml` files under `kubernetes/` use a **separate native age key** (not the SSH key).
- The age private key is stored in Ansible vault as `vault_age_private_key` (a standard age key with `AGE-SECRET-KEY-...` format).
- The corresponding public key (`age1pdqae3ffmt9rxtmn2758pxjqaaxnytg6mzx3wpepuhs7de2ttfkqk3z7hl`) is in `.sops.yaml`.
- To extract and use: `ansible-vault view ansible/inventory/group_vars/all/vault.yml | grep vault_age_private_key` → write the `AGE-SECRET-KEY-...` line to a temp file → `SOPS_AGE_KEY_FILE=/tmp/sops-age.key sops --decrypt <file>.sops.yaml`.
- **Never edit SOPS files outside of `sops`** — use `sops <file>.sops.yaml` to edit, or decrypt → edit → re-encrypt. Manual edits break the SOPS MAC integrity check.
- To populate fnox secrets from SOPS: decrypt the SOPS file, extract values, and `fnox set <KEY> --provider age` for each.

**Shared:**
- Ansible vault remains the source for cluster ops secrets (roles consume `vault_*` vars directly). fnox replaced the old `generate-env` vault→`.env` pipeline for local dev.

**Workflow tasks live in `mise.toml`, not bash scripts.** All setup, build, test, and run workflows are `mise` tasks (`mise tasks` to list). `scripts/setup-local-env.sh` is a thin `exec mise run setup` wrapper for compatibility only — do not add logic there. New workflow steps → new/updated `mise` task. Never create bespoke bash scripts for repo workflows.

**Prefer mise tasks over raw tooling commands.** Don't run `cargo leptos build`, `podman build`, `npm install`, etc. directly — use the corresponding `mise run` task instead (e.g. `mise run yral-auth-build`, `mise run yral-auth-image`). This ensures env vars from `[env]` and fnox secrets are loaded, `depends` chains run, and the workflow is reproducible. If a needed workflow doesn't exist as a mise task, create one rather than running the raw command.

**Long-running processes (dev servers, containers) managed by pitchfork.** `pitchfork.toml` at the repo root defines daemons with ready checks, restart policies, and automatic cleanup. Use `mise run yral-auth-run` (which calls `pitchfork start`) instead of running a server directly — pitchfork ensures the process is tracked, health-checked, and cleanly stopped when you exit. Daemons use `mise = true` (pitchfork's built-in mise integration wraps commands with `mise x --`) and `dir` to load per-app `mise.toml [env]`. Secrets are injected via `fnox exec` inside the daemon command. Stop with `mise run yral-auth-stop` or `pitchfork stop --all`.

**Version locking:** `mise.lock` is committed. Bump tool versions explicitly in `mise.toml`, then `mise lock` to refresh the lockfile. Never use floating `"latest"` without a lockfile entry.

**Declarative-only tooling (no imperative system installs):** The local environment must be fully reproducible from `mise.toml` alone — a Nix-like experience. Never run `sudo apt-get install`, `brew install`, or any imperative system package manager command to satisfy a build dependency. If a tool or library is needed:
1. Check if mise provides it (add to `[tools]` in `mise.toml`).
2. If it's a Rust/Cargo tool, install via `cargo binstall` in a mise task.
3. If it's a system C library, linker, or other OS package, declare it in `[bootstrap.packages]` (e.g. `"apt:musl-tools" = "latest"`). mise handles the install via `mise bootstrap packages apply` — OS-filtered so apt entries are ignored on macOS, brew entries on Linux, etc. List entries alphabetically by key for easy scanning and editing.

This ensures `mise bootstrap --yes && mise run setup` is the only command needed to go from a fresh machine to a working environment.

**Per-service local dev convention (all services):** Every service we host and maintain follows the same pattern for local development:
- **Tasks and plaintext env vars** → `mise.toml` (per-app `[tasks]` and `[env]` sections, or root `mise.toml` for shared values)
- **Secrets** → `fnox.toml` (age-encrypted, committed to git)
- **Long-running dev servers** → `pitchfork.toml` (with ready checks, restart policies, automatic cleanup)
- Reuse shared secrets (Dragonfly TLS certs, Harbor credentials, etc.) across services — don't duplicate values, reference the same fnox secret definitions
- For each new service onboarded, create mise tasks (`<app>-build`, `<app>-run`, `<app>-image`), add secrets to fnox, and add a pitchfork daemon entry

### SpacetimeDB Usage Rules
**No raw SQL from application code (Hard Rule).** SpacetimeDB supports raw SQL over its REST/WS APIs, but we avoid it in app code to keep data access typed and declarative:
- **Rust services** interact with a SpacetimeDB database via the **generated `spacetimedb-sdk` bindings** (`spacetime generate` → `src/bindings/`): typed reducer calls, typed procedure calls with typed `SpacetimeType` returns, and typed table accessors. Never send raw SQL strings from Rust.
- **Mobile / non-SDK clients** (no Kotlin/Swift SpacetimeDB SDK exists) call module **procedures** via REST (`POST /v1/database/{db}/call/:name`, JSON array body → typed JSON `SpacetimeType` return). The client never constructs SQL.
- **CLI `spacetimedb-cli sql` / `spacetimedb-cli call`** is acceptable for imperative debugging/one-off inspection only — never wire it into application code or scripts.
Rationale: SpacetimeDB REST `/sql` has no bind parameters (only `:sender` for RLS), so interpolating values risks SQL injection and loses type safety. The generated bindings + procedures are the typed, safe paths.

**Procedures vs HTTP handlers vs reducers** (SpacetimeDB module patterns, spacetimedb 2.6.1):
- **Reducers** (`#[spacetimedb::reducer]`) — transactional, can't return data to callers. Use for all mutations.
- **Procedures** (`#[spacetimedb::procedure]`, `features = ["unstable"]`) — non-transactional (open explicit `ctx.with_tx`), can return typed `SpacetimeType` values to the caller, `ctx.sender()` available, generated SDK bindings. **Prefer for per-user/typed-return reads.**
- **HTTP handlers** (`#[spacetimedb::http::handler]` + `#[spacetimedb::http::router]`, `features = ["unstable"]`, exposed at `/v1/database/{db}/route/*path`) — bypass SpacetimeDB auth by design (`sender` is `Identity::ZERO`), arbitrary `http::Response`. **Prefer for truly public/identity-agnostic endpoints** where "no auth" is the feature — webhook receivers, public config dumps, health checks, pre-signed URL issuers.
Both procedures and HTTP handlers require `features = ["unstable"]` in the module's `Cargo.toml` (unstable-gated in spacetimedb 2.6.1).
Module-specific syntax details (table index macro, accessor traits) belong as comments in the module source, not here.

### External Service Config & Secrets (SpacetimeDB + general)
For any external service the repo calls (SpacetimeDB Maincloud, third-party APIs, etc.):
- **Locally:** plaintext values (URLs, DB names, identities) → `mise.toml [env]`; secrets (tokens, admin keys) → `fnox` (age-encrypted, committed).
- **In-cluster:** plaintext → k8s `ConfigMap`; secrets → SOPS-encrypted `*.sops.yaml`. **Only provision cluster config if a service in our cluster actually calls the external service.** External clients (mobile apps, out-of-workspace backends like the Prakash video-storage service) carry their own config and do not consume our cluster ConfigMaps/SOPS.
- Wire config into the consuming app via its existing config mechanism (e.g. mobile `BuildConfig`/config module; Rust service `mise.toml [env]` + `fnox exec`).

### GPU (Vast.ai)
See `ansible/roles/vastai-provision/defaults/main.yml` for provisioning rules and constraints.

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
**No manual `flux reconcile` or forced reconciliation** — the webhook receiver triggers reconciliation within seconds of a git push. Forcing reconciliation with `fluxcd.io/reconcile-at` annotations creates race conditions with the ImageUpdateAutomation controller (which commits image tag updates back to git). These concurrent reconciliations can overwrite each other's commits, causing git history divergence and lost changes. **After pushing to git, wait. Do not force reconcile.** The webhook + interval (1m) will handle it. Only use `flux reconcile` if the webhook is confirmed down (check GitHub webhook delivery status) or for explicit debugging — never as a shortcut to speed things up.

**Post-push validation (Non-Negotiable):** After every push, watch the cluster until the change has reconciled and the desired effect is confirmed in production. Do not declare done and walk away on "pushed." Concretely: poll the affected Flux Kustomizations (`flux get kustomizations --watch` or read-only `kubectl get kustomization -n flux-system`) until `Ready=True` with the new revision; then verify the target resources are healthy (`kubectl get/describe`, readiness, recent logs, health endpoints, smoke tests). If anything is not green or not as expected, stop and flag it to the operator immediately — then iterate (read-only diagnosis → fix in git/role → push → re-validate) until production is in a confirmed-fixed state. Never report success without this verification cycle having completed.

**Health check stalls:** When a Deployment is in `Failed`/`CrashLoopBackOff` status, Flux's `apps` kustomization health check will report unhealthy and may not apply new revisions. This is by design. Do not force reconcile or suspend/resume kustomizations to work around it. Instead, fix the root cause (the crashing pod) — build a new image with the fix, let the ImagePolicy/IUA pipeline update the tag in git, and push. Once pods come up healthy, Flux resumes normal operation automatically.

### Image Registry (Harbor)
In-cluster Harbor at `harbor.yral.com` is the registry for custom-built app images (NOT bootstrap infra images — those stay on GHCR/quay.io for disaster recovery).

**Simplicity over fine-grained access control:** This is a single-contributor repo/cluster. Use the Harbor admin credentials directly for CI pushes, Flux image scanning, and pod image pulls. Store as a SOPS-encrypted `kubernetes.io/dockerconfigjson` Secret (e.g., `harbor-pull.sops.yaml`) in each app namespace. Admin password lives in `kubernetes/infrastructure/harbor/harbor-admin-secret.sops.yaml`.
- Every namespace pulling from Harbor must have an `imagePullSecrets` reference in the Deployment/StatefulSet spec pointing to that Secret.
- Use Flux `ImageRepository` + `ImagePolicy` + `ImageUpdateAutomation` for automatic tag updates from Harbor into git manifests.

### Image Building (Shipwright + BuildKit)
In-cluster image building via Shipwright (CRD-native, CNCF Sandbox) wrapping BuildKit (rootless). Tekton Pipelines is the execution engine. Triggered by Flux on git push — no external CI needed. See `kubernetes/infrastructure/shipwright/kustomization.yaml` and `kubernetes/infrastructure/tekton-pipelines/kustomization.yaml` for version pins and compatibility notes.

**Test locally before deploying (Hard Rule):** Always build and test code changes locally before pushing to git and deploying to production. The full end-to-end workflow uses mise tasks for every step — both human operators and agents must use the same mise tooling:

1. **Compile locally** — `mise run <app>-build` (release musl binary) or `mise run <app>-build-local` (debug, same code as production).
2. **Test** — `mise run <app>-test` (unit tests; external-service-dependent tests are removed — only pure-logic and production-endpoint tests remain).
3. **Run locally** — `mise run <app>-run` (release binary via pitchfork, fnox secrets injected) or `mise run <app>-run-local` (debug build via pitchfork).
4. **Build image locally** — `mise run <app>-image` (podman build from repo root, `.dockerignore` excludes `target/` and `node_modules/`).
5. **Run image locally** — `mise run <app>-image-run` (podman container via pitchfork, port forwarded).
6. **Push** — `git push` and let the CI/CD pipeline (Tekton → Shipwright → Harbor → Flux) build and deploy.
7. **Validate on prod** — `mise run <app>-validate` (read-only kubectl checks: pods, readiness, recent logs).

Never push code changes to git without first verifying they compile and run locally. Waiting for an in-cluster Shipwright build to discover a compile error or runtime failure wastes 10+ minutes per iteration.

**CI/CD Pipeline (Tekton Triggers → Shipwright → Harbor → Flux):** Git push → Tekton EventListener → BuildRun → Harbor image push → Flux ImageRepository/ImagePolicy/ImageUpdateAutomation → git commit updating deployment manifest → Flux Kustomization reconcile → new pods.

- Per-app Tekton Trigger resources in `kubernetes/apps/<app>/tekton-trigger.yaml`.
- GitHub webhook → EventListener Service (exposed via Cilium Gateway HTTPRoute).
- **Build webhook setup (per new app):** When adding a new app with Tekton build triggers:
  1. Create HTTPRoute for `<app>-build.yral.com` in `kubernetes/networking/routes/<app>-build.yaml` (HTTP→HTTPS redirect + HTTPS route to `el-<app>-build-listener` in the app namespace).
  2. Create ReferenceGrant in the app namespace (`kubernetes/apps/<app>/build-reference-grant.yaml`) allowing HTTPRoute from kube-system to reference the EventListener Service.
  3. Add both to their respective kustomization.yaml files.
  4. Create the GitHub webhook via `gh` CLI: `gh api repos/<owner>/<repo>/hooks --input - <<'EOF' { "name": "web", "active": true, "events": ["push"], "config": { "url": "https://<app>-build.yral.com/", "content_type": "json" } } EOF`
  5. Push to git and wait for Flux to apply the HTTPRoute.
- **Filter out fluxcdbot commits**: The CEL interceptor MUST filter `body.head_commit.author.username != 'fluxcdbot'` to prevent a feedback loop.

**Image tagging (timestamp-prefixed, NOT raw commit SHA):** Tags are `YYYYMMDDHHMMSS-<8-char-sha>`. The timestamp prefix ensures `Alphabetical:asc` in Flux ImagePolicy correctly selects the most recent build.

**Tekton Triggers CEL Interceptor (official pattern):**
- Overlay `key`: just the field name (e.g. `image_tag`), NOT prefixed with `body.` or `extensions.`
- Overlay `expression`: CEL expression evaluated against `body` (JSON payload), `header`, `extensions`, `requestURL`
- TriggerBinding access: `$(extensions.image_tag)` — NOT `$(body.extensions.image_tag)`
- Available CEL functions: `truncate(uint)`, `translate(regex, repl)`, `split(sep)`, `join(sep)`, `replace(old, new)`, `substring(start, end)`, `lowerAscii()`, `upperAscii()`, `parseJSON()`, `parseURL()`

**Flux ImagePolicy (official — only 3 policy types exist):**
- `SemVer` — semantic versioning range (e.g. `>=1.0.0`)
- `Alphabetical` — string sort, `asc` picks Z (last/highest), `desc` picks A (first/lowest) — use `asc` for timestamp-prefixed tags
- `Numerical` — numeric sort, same order semantics
- NO `Latest` policy type exists.

**Shared cluster-scoped resources (avoid kustomize "id matched 2 resources"):** When multiple app sub-kustomizations are included in the same parent Kustomization (`apps`), cluster-scoped resources (ClusterRole, ClusterRoleBinding) and shared namespace resources (e.g., `harbor-scan-secret` in `flux-system`) must be defined only ONCE. Duplicate definitions with the same name cause kustomize build failures.

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
10. About to `kubectl delete` a Flux Kustomization/resource? → Make the change in git and push (Flux prunes). `flux suspend` first only if removing the Kustomization CR itself or IUA-managed resources.
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

**AGENTS.md is the only memory to use.** Do not use agent-specific memory systems (e.g. `/memories/`, `.copilot/`, or similar) — that memory doesn't get shared between hosts and isn't checked into source control. It defeats the purpose of memory when we switch to another host. All persistent knowledge must live in AGENTS.md (committed to git) or in role/README files within the repo.

**Service-specific details belong colocated as comments on the manifests/code that implements them.** AGENTS.md contains only repo-wide rules and patterns. When a detail applies to one component (versions, disk layouts, flag values, API patterns), put it in a header comment block on the relevant YAML/Ansible file. This keeps AGENTS.md scannable and ensures the detail is visible exactly where it's needed.