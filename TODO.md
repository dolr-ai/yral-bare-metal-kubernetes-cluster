- symlink the devpod config to the .dotfiles repo
- use ansible vault to manage secrets in the .dotfiles repo
- evaluate if kubernetes secrets are actually secure and consider if we should host hashicorp vault on the cluster to manage secrets instead. This is because I imagine secrets are stored in etcd in plaintext. Since etcd nodes have their disk without encryption, the kubernetes secrets are essentially insecure, right? Is there a way to encrypt etcd at rest? Confirm this:
The key never leaves the cluster and is encrypted at rest inside etcd (Kubernetes API server encrypts Secrets at the etcd layer by default in kubeadm clusters).
- Run this benchmark - https://github.com/aquasecurity/kube-bench
- No notifications received for the dead node. We should have an alert for this. Set up a Grafana alert that notifies us when a node goes down, so we can investigate and fix it as soon as possible. This is critical for maintaining the health and availability of our cluster.

- Update AGENTS.md after the migration is done and remove redundant sections
- For subsequent migrations, instead of copying over live data and turning off the app temporarily, the preffered mechanism is to instead create another replica of the backing store, be it kafka or postgres, and then do a controlled failover to the new replica once it's fully synced. This way we can minimize downtime and avoid the risks associated with copying live data. Once, we're sure that the data is migrated, we point the app to the new replica and decommission the old one. Note this migration preference in AGENTS.md for future reference and to ensure that we follow best practices for migrations going forward.
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
- Do DNS for atmz.ai like we did for saikat.dev. The domain is available on namecheap with the same credentials that we have currently saved in the ansible vault
- Move to a 7 node control plane with mixed nodes for better upgradeability and cross data failure isolation. Also, do a stacked control plane deployment where control planes live on the same nodes as workloads. No wasted nodes
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
- Add dbx to the self hosted database UI list and gate behind auth.
- Remove the other unused UIs
- https://github.com/dolr-ai/yral-bare-metal-kubernetes-cluster/blob/343ed901a2a25d701455d20e2d1d41b4253573a2/kubernetes/infrastructure/kafka/kafka.yaml#L107-L116 - Do we need these still?