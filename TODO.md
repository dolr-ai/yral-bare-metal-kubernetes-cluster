- Disable ceph ui auth because it is already tackled by oauth2-proxy. 
- Just to confirm, for control plane nodes, we are still using the entire 1 TB via joining 2 500GB drives using btrfs, right? No point wasting space on the control planes since they don't run Ceph OSDs
- Host yral.com to root domain. Take down about.yral.com
- refactor hetzner-bare-metal repo to use individual segmentation for the inventory file and commit to the individuals owning infra strategy completely
- Fix volumes still being placed across regions instead of within the same region. Also what is zone in longhorn terminology? What value is it set to for our nodes?
- Snowplow bad events are human readable but the enriched events are not. Figure out why and decide if we want to keep it this way
- We need to make it so that there are no entries in snowplow-bad. They should all go to snowplow-enriched. IF there are events in snowplow-bad, it means the enrichment process is failing for some reason and we need to investigate and fix it.
- existing Loki/Prometheus/ClickHouse/Metabase volumes retain their 2 Longhorn replicas (desired outcome; no migration needed). Each app comment in the PVC/HelmRelease now documents exactly what redundancy work is needed before it can drop to 1 replica. Figure out 1 at a time what work is needed to migrate each app to its desired state with 1 replica and then we execute it
- integrate snowplow sdk into yral-mobile to send events to the snowplow collector
- figure out a way to run devcontainers on the kubernetes cluster, so we can have a single environment for development and deployment
- github tasks assigned
- timer counter remove II auth and use oauth2-proxy to get the logged in user
- use realtime capabilities of spacetimedb to build a realtime persistence layer
- Verify that Longhorn backups are being taken
- We should make it so that Longhorn volumes are replicated across the same region, not across regions, to avoid cross-region latency
    - What v1.11.0 does add for topology control: StorageClass allowedTopologies support (#12261) — you can restrict replica provisioning to nodes matching specific Kubernetes topology labels (e.g. topology.kubernetes.io/region=falkenstein). This is the closest thing to region confinement available.
    - What we want is for stateful workloads to have their volumes in the same region. Ideally, if a stateful workload is provisioned, it uses a volume where one copy of the volume is on the same node where the workload is running (like a database, for example). The second copy of that volume should be in the same region. We don't specifically want FSN, but if the stateful workload is in HSN, then the volumes should also be there, and one of them on the same node
- Look at monthly copilot spend and upgrade to pro+ if makes sense
- Look at domains and see if still required
- Remove github org members who are no longer active
- Add new github members to the report script
- Remove cloudflare pages and workers projects on personal account
- Move personal website to github pages and remove from cloudflare
- kafka native schema validation via schema registry and redpanda console
- confirm receiving renovate notifications for kubernetes version changes
- metabase not working. Fix
- rewrite timer counter as a kotlin app to run on android and windows
- we can remove all the kubernetes packages unholding logic with `apt-mark unhold` since all the packages are now unheld. Confirm once before removing the code that unholds packages. Also, we have already removed the code that holds packages, right?
- are loki logs being cleaned up properly? Check retention policies and storage usage. What about prometheus and clickhouse? Does prometheus have a retention policy configured? Check storage usage and retention settings. Similarly, check clickhouse for retention policies and storage usage. We are dumping snowplow events into clickhouse via the snowplow stream collector and using clickhouse as the data warehouse for analytics. We should ensure that we have appropriate retention policies in place to manage storage and costs effectively. The snowplow data is also saved long term in object storage, so we can always re-ingest if needed
- archive all the unused org repos
- Confirm that all the new volumes being provisioned have 1 replica unless specifically configure to have 2
- How do we enforce schema on the events being ingested into our kafka topics? Should we use Snowplow's schema registry that uses iglu? Figure out if this is open and popular. Or should we use schema enforcement at Kafka's layer? This is probably the more popular and documented solution
- Clean up unused dns entries
- symlink the devpod config to the .dotfiles repo
- use ansible vault to manage secrets in the .dotfiles repo
- encrypted volumes for longhorn
- The lateral movement problem is the serious one. If any single pod is compromised (RCE via a vulnerable dependency, a misconfigured container, a supply chain attack), the attacker has unrestricted access to:

ClickHouse — can read/write all analytics data, port 8123 open to every pod
Kafka — can produce/consume any topic, including snowplow events
Longhorn manager API — can manipulate volumes or extract data from snapshots
All Prometheus/Grafana metrics — operational intelligence leakage
The Kubernetes API itself indirectly via any misconfigured RBAC
In your setup this is especially relevant because you have data infrastructure (ClickHouse, Kafka) and orchestration infrastructure (Longhorn, Flux) all on the same flat network as application pods.

The noisy neighbour / accidental misconfiguration problem: A misconfigured pod (wrong namespace, wrong labels) can accidentally reach production databases. There's no enforcement boundary.

Should you apply it now?
Honest answer: not all at once. Here's why this is hard for your specific cluster:

Monitoring scrapes everything — Prometheus needs ingress to every pod in every namespace on metrics ports. A blanket default-deny breaks this immediately if you don't enumerate every scrape target.

Longhorn is deeply cross-namespace — the CSI driver, manager, instance-manager, and share-manager pods communicate heavily across longhorn-system, kube-system, and default. Their dependency graph is not trivial to express in policies.

Strimzi explicitly disabled policy generation (generateNetworkPolicy: false) — meaning someone already decided the generated policies were problematic. You'd need to write Kafka network policies from scratch.

The cilium-envoy hostNetwork constraint (documented in AGENTS.md) — gateway traffic arrives as a node IP, so you can't write namespaceSelector-based ingress rules for any pod that receives external traffic. Every such pod needs a port-only rule with no from: clause, which is weaker than it looks.

CoreDNS — everything needs egress to kube-dns on UDP/TCP 53, or DNS breaks silently.

A practical approach if you do want to harden
The right way is incremental:

Start with namespace isolation for your highest-value targets — ClickHouse and Kafka first. An explicit allow-list for what actually needs to talk to them (Kafka Connect, Metabase, ClickHouse is reachable from… what exactly?) is achievable without a cluster-wide policy change.

Use Hubble (already deployed) to generate the actual traffic map before writing any policies. hubble observe --namespace clickhouse will show you exactly which pods are making connections to ClickHouse right now. Write policies to match reality, don't guess.

Don't do CiliumClusterwideNetworkPolicy default-deny yet — that's a single change that would silently break an unknown number of things. Namespace-scoped policies for specific high-value namespaces are lower risk and reversible.

In short: your current state is a real security gap worth closing, but "apply default-deny cluster-wide" is not the right first move. Hubble → traffic map → targeted policies for ClickHouse and Kafka → expand from there.

- evaluate if kubernetes secrets are actually secure and consider if we should host hashicorp vault on the cluster to manage secrets instead. This is because I imagine secrets are stored in etcd in plaintext. Since etcd nodes have their disk without encryption, the kubernetes secrets are essentially insecure, right? Is there a way to encrypt etcd at rest? Confirm this:
The key never leaves the cluster and is encrypted at rest inside etcd (Kubernetes API server encrypts Secrets at the etcd layer by default in kubeadm clusters).
- Run this benchmark - https://github.com/aquasecurity/kube-bench
- No notifications received for the dead node. We should have an alert for this. Set up a Grafana alert that notifies us when a node goes down, so we can investigate and fix it as soon as possible. This is critical for maintaining the health and availability of our cluster.
- Cleanup longhorn when done with the entire migration
    - Longhorn UI
    - Longhorn S3 backups

- MIGRATION STATUS: ceph-block is intentionally NOT set as the default StorageClass while Longhorn PVCs are being migrated.  Once all workloads have been moved to Ceph (see PVC migration procedure in AGENTS.md), set storageclass.kubernetes.io/is-default-class: "true" and patch the Longhorn StorageClass annotation to "false" in the Longhorn HelmRelease values.
After migration is complete, ensure Rook-Ceph is the default StorageClass and that the Longhorn StorageClass is not default, to ensure new volumes are provisioned on Ceph by default.

- Update AGENTS.md after the migration is done and remove redundant sections
- Ceph volume backups
- Reevaluate if it's possible to have same region replication with ceph that was not possible with Longhorn. So, the primary volume replica should be on the same node as the workload pod and the secondary replica should be in the same region but not necessarily on the same node. This is to ensure low latency for the primary replica while still having redundancy within the same region. Check if ceph supports this kind of topology-aware replication and if it can be configured to meet this requirement.
- Since we have removed Longhorn from the cluster, we can also remove the nfs-common package, right? Also the rpcbind handling that we were doing, right? Confirm that there are no dependencies on nfs-common or rpcbind before removing them from the cluster. This will help to reduce the attack surface and remove unnecessary components from the cluster, improving security and maintainability.
- Clean up the admin user added to Kafka for the migration from longhorn to ceph