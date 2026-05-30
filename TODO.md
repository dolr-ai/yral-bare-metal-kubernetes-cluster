- Unable to log into Ceph UI with the credentials. Did something change?
- Just to confirm, for control plane nodes, we are still using the entire 1 TB via joining 2 500GB drives using btrfs, right? No point wasting space on the control planes since they don't run Ceph OSDs
- Snowplow bad events are human readable but the enriched events are not. Figure out why and decide if we want to keep it this way
- We need to make it so that there are no entries in snowplow-bad. They should all go to snowplow-enriched. IF there are events in snowplow-bad, it means the enrichment process is failing for some reason and we need to investigate and fix it.
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