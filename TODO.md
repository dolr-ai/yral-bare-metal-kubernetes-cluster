- ✅ Snowplow bad events: ROOT CAUSE FOUND & FIXED. Bad events were human-readable JSON; enriched events are TSV (131 tab-separated fields, text but dense). Raw topic is binary Thrift (not previewable in Redpanda Console). All 1,623 bad messages (from July 20) had the same error: `atomic_field_length_exceeded` — the mobile app put event properties as a JSON string into `se_property` (maxLength=1000), and events with large arrays (e.g. `influencer_cards_viewed` with 200+ items) exceeded this. Fix: truncated `se_property` in both Android and iOS `SnowplowAnalyticsProvider`. Also separated `snowplow-bad` into `snowplow-collector-bad` (bot traffic), `snowplow-enrich-bad` (schema violations), and `snowplow-enrich-failed` (enrichment failures) so enrichment failures are measurable independently. Added ClickHouse `snowplow.bad_rows` table for queryable bad-row analytics. Switched Iglu resolver from HTTP to HTTPS. Updated enrich config comment from 6.8.0 to 6.12.0.
- ✅ Snowplow bad topic separation: Done — collector bad rows → `snowplow-collector-bad`, enrichment bad rows → `snowplow-enrich-bad`, enrichment failed events → `snowplow-enrich-failed`. The old `snowplow-bad` topic is retired. `snowplow-enrich-bad` and `snowplow-enrich-failed` should be zero; non-zero means a real bug.
- kafka native schema validation via schema registry and redpanda console
- rewrite timer counter as a kotlin app to run on android and windows
- are loki logs being cleaned up properly? Check retention policies and storage usage. What about prometheus and clickhouse? Does prometheus have a retention policy configured? Check storage usage and retention settings. Similarly, check clickhouse for retention policies and storage usage. We are dumping snowplow events into clickhouse via the snowplow stream collector and using clickhouse as the data warehouse for analytics. We should ensure that we have appropriate retention policies in place to manage storage and costs effectively. The snowplow data is also saved long term in object storage, so we can always re-ingest if needed
- archive all the unused org repos
- Confirm that all the new volumes being provisioned have 1 replica unless specifically configure to have 2
- How do we enforce schema on the events being ingested into our kafka topics? Should we use Snowplow's schema registry that uses iglu? Figure out if this is open and popular. Or should we use schema enforcement at Kafka's layer? This is probably the more popular and documented solution
- Clean up unused dns entries
- symlink the devpod config to the .dotfiles repo
- use ansible vault to manage secrets in the .dotfiles repo
- The lateral movement problem is the serious one. If any single pod is compromised (RCE via a vulnerable dependency, a misconfigured container, a supply chain attack), the attacker has unrestricted access to:

ClickHouse — can read/write all analytics data, port 8123 open to every pod
Kafka — can produce/consume any topic, including snowplow events
All Prometheus/Grafana metrics — operational intelligence leakage
The Kubernetes API itself indirectly via any misconfigured RBAC

The noisy neighbour / accidental misconfiguration problem: A misconfigured pod (wrong namespace, wrong labels) can accidentally reach production databases. There's no enforcement boundary.

Should you apply it now?
Honest answer: not all at once. Here's why this is hard for your specific cluster:

Monitoring scrapes everything — Prometheus needs ingress to every pod in every namespace on metrics ports. A blanket default-deny breaks this immediately if you don't enumerate every scrape target.


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

- Update AGENTS.md after the migration is done and remove redundant sections
- Ceph volume backups
- Reevaluate if it's possible to have same region replication with ceph. So, the primary volume replica should be on the same node as the workload pod and the secondary replica should be in the same region but not necessarily on the same node. This is to ensure low latency for the primary replica while still having redundancy within the same region. Check if ceph supports this kind of topology-aware replication and if it can be configured to meet this requirement.
- Decide what to do with the ceph-block-1replica pool. If it needs separate disks to be allocated, it doesn't solve our need. What we need is the ability to have 1 replica for certain volumes that don't require high durability, but we still want to use the same underlying storage pool (ceph-block) that has 2 replicas for other volumes. Check if ceph allows us to have different replication factors for different volumes within the same pool, or if we need to create a separate pool with 1 replica and manage it separately. If it doesn't allow, we want to get rid of this pool and just use the ceph-block pool for all volumes, even those that only require 1 replica, since it will still provide redundancy and durability without the need for a separate pool.
- Depending on the above, the kafka storage class that is currently using the ceph-block-1replica pool may need to be updated to use the ceph-block pool instead, if we decide to get rid of the ceph-block-1replica pool. This will ensure that all Kafka volumes are using the same underlying storage pool with 2 replicas, providing better durability and redundancy for our Kafka data. If we keep the ceph-block-1replica pool, we can keep the Kafka storage class as is, but we need to ensure that it is properly documented which volumes should use which pool based on their durability requirements.
- Depending on the above, we should also reduce the number of replicas for kafka since replication is already handled at the volume level by ceph. Having 2 replicas at the storage level and 3 replicas at the application level is redundant and doesn't provide additional durability benefits, while it does increase storage costs. We can reduce the number of Kafka replicas to 1 or 2, depending on our durability requirements, while still using the ceph-block pool with 2 replicas for the underlying storage. For scaling write throughput, we can rely on partitioning in Kafka rather than having multiple replicas at the application level, since the storage layer is already providing redundancy. This will help to optimize our storage costs while still maintaining durability and performance for our Kafka workloads.
- For subsequent migrations, instead of copying over live data and turning off the app temporarily, the preffered mechanism is to instead create another replica of the backing store, be it kafka or postgres, and then do a controlled failover to the new replica once it's fully synced. This way we can minimize downtime and avoid the risks associated with copying live data. Once, we're sure that the data is migrated, we point the app to the new replica and decommission the old one. Note this migration preference in AGENTS.md for future reference and to ensure that we follow best practices for migrations going forward.
- Handle ceph snapshot backups to s3
- Check if Hetzner has Ubuntu 26.04 and use that everywhere
- cleanup the rust counter service when done testing cloudnative pg with it
- flux bootstrap instead of flux install. Currently flux tab on headlamp shows not bootstrapped
- setup flux alerts and provider to google chat
- move the self hosted beszel to kubernetes
- move the self hosted uptime kuma to kubernetes
- Instead of beszel move all bare metal servers to grafana and prometheus node exporter and use grafana to monitor them instead of having a separate monitoring solution for the bare metal servers
- Setup networking so that we specifically allowlist which pods can talk to each other
- Remove ffmpeg@7 after done with offchain changes
- Remove ci-e2e-reader KafkaUser if Kafka bridge serves our needs and have mobile app tests call the bridge instead of Kafka directly. Remove all associated Kafka native infrastructure that we added to support ci-e2e-reader
- Instead of TSV, should we move to json or something else?
- Remove goldilocks completely. I seem to have seen a goldilocks-alloy container
- Remove yral-auth specific task runner and use the global task runner for everything
- Remove the library/rust-counter image from harbor
- Move all env to mise instead of direnv
- Move task runners to mise? Check and decide
- Migrate DNS from Cloudflare to in-cluster CoreDNS to fix Reliance Jio DNS blocking. Jio intermittently blocks DNS queries to Cloudflare's resolvers, causing mobile app clients in India to fail with DNS resolution errors (offchain.yral.com, agent.rishi.yral.com, etc.). The fix is to stop using Cloudflare as the authoritative NS for yral.com and instead run CoreDNS in-cluster as the authoritative NS. This also eliminates the 40× per-node *.yral.com A records in Cloudflare and enables per-team-member wildcards (*.ansuman.yral.com, *.rishi.yral.com, etc.) with zero DNS provider changes. Steps: (1) transfer yral.com domain from Cloudflare Registrar to a registrar that allows custom NS (e.g. Hetzner Domains) — Cloudflare Registrar does NOT allow pointing NS to external nameservers, (2) set glue records at the new registrar: ns1-5.yral.com → 5 CP IPs (static, one-time), (3) deploy CoreDNS on the 5 CPs with hostNetwork:true on port 53 serving the yral.com zone with wildcards for *.yral.com, *.ansuman.yral.com, etc. → all cluster node IPs, (4) switch cert-manager from Cloudflare API to RFC-2136 (dynamic DNS updates to CoreDNS) for DNS-01 challenges, (5) delete all Cloudflare DNS records — no longer needed. Cloudflare stays as the domain registrar is NOT an option (forced Cloudflare NS coupling).
- Remove mixpanel from all source code completely. We don't use mixpanel as an analytics provider anymore
- Turn all of the rust apps in this repo as one unified workspace whose dependencies are managed by the root cargo toml
- remove ceph dashboard entry from dashboard.yral.com since it's been removed?
- ceph has health_warn on the headlamp dashboard. Figure out why and help fix?
- check rook/ceph to confirm if on pod creation with the default storage class that uses 2 replicas, the primary replica is always on the same node that has the pod. Subsequently, it's preferable if the second replica is in the same region as the primary so that replication over the same region network is faster. If this is not possible/available, it's still okay but the primary replica on the same node as the pod is non-negotiable
- Do DNS for atmz.ai like we did for saikat.dev. The domain is available on namecheap with the same credentials that we have currently saved in the ansible vault
- Move to a 7 node control plane with mixed nodes for better upgradeability and cross data failure isolation. Also, do a stacked control plane deployment where control planes live on the same nodes as workloads. No wasted nodes
- Move to Mayastor for data locality with primary replica on the same node as the pod. Secondary replica should be on a different node in the same region
- rewrite my-website with leptos builder syntax
- stop yral-auth from storing its data in naitik's redis infrastructure but move it over to call our spacetimedb
- The categorized scan completed. Here's the full breakdown of all 238,606 keys in the Redis/Dragonfly instance:

Prefix	Count	What it is
IC Principal keys (xxx-xxx-xxx-...:*)	193,023	Per-user data keyed by IC Principal — owned by other services (yral-legacy, off-chain-agent, etc.)
yral-auth:*	27,671	yral-auth auth KV store (session data, OAuth lookups, principal mappings)
yral-metadata:*	12,451	yral-metadata service data
impressions:*	5,322	Impression/view tracking
offchain:*	127	off-chain-agent data
test:*	12	Test keys (from TEST_KEY_PREFIX in dragonfly_kv.rs)
TOTAL	238,606	
We're correctly only migrating yral-auth:* (27,671 keys). The SpacetimeDB auth_kv table is the replacement for yral-auth's KV store specifically — all 23 calls to format_to_dragonfly_key() in yral-auth use KEY_PREFIX = "yral-auth".

The other prefixes belong to different services that share this Dragonfly instance and will have their own migration paths:

yral-metadata:* — the yral-metadata service has its own Redis-backed store
impressions:* — view/impression tracking, likely from the off-chain-agent or yral-legacy
offchain:* — off-chain-agent's own data
IC Principal keys — per-user canister data from yral-legacy
We should not bring everything into the auth_kv table — that would mix yral-auth's auth data with unrelated services' data in a single key-value store. Each service needs its own SpacetimeDB table or its own migration strategy.

Migrate the other useful bits