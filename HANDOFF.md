# Agent Handoff — Kafka Event Pipeline

## Current State

The Kafka-based event ingestion pipeline is **fully deployed and running** as of
2026-02-28. All Flux Kustomizations are reconciled at `main@sha1:63a2e384`.

### Pipeline topology

```
events.yral.com (HTTPS)
  → Snowplow Scala Stream Collector 3.7.0  (3 pods, namespace: snowplow)
  → Kafka topic: snowplow-raw              (12 partitions, RF3, 2-day retention)
  → Snowplow Enrich 6.8.0                 (2 pods, namespace: snowplow)
  → Kafka topic: snowplow-enriched         (12 partitions, RF3, unlimited retention)
  → Kafka Connect S3 Sink 12.1.0          (2 workers, namespace: kafka-connect)
  → Hetzner S3: yral-events-data-lake     (fsn1.your-objectstorage.com)
       prefix: events/enriched/year=YYYY/month=MM/day=dd/hour=HH/
```

### Pod status (all Running)

| Namespace     | Pod                                   | Status  |
|---------------|---------------------------------------|---------|
| kafka         | kafka-cluster-combined-0/1/2          | Running |
| kafka         | kafka-cluster-entity-operator-*       | Running |
| snowplow      | snowplow-collector-* (×3)             | Running |
| snowplow      | snowplow-enrich-* (×2)                | Running |
| kafka-connect | connect-cluster-connect-0/1           | Running |

KafkaConnector `s3-sink-snowplow-enriched`: **READY: True**

### Flux Kustomizations (all Applied: True)

- `infrastructure-strimzi` — Strimzi 0.50.1 operator
- `infrastructure-kafka` — Kafka 4.1.1 cluster (3 combined broker/controller nodes, KRaft)
- `infrastructure-snowplow` — Collector + Enrich deployments, SOPS-encrypted kafka-credentials
- `infrastructure-kafka-connect` — KafkaConnect cluster + S3 sink connector, SOPS-encrypted connect-credentials

---

## Key Files

| File | Purpose |
|------|---------|
| `kubernetes/infrastructure/strimzi/` | Strimzi operator HelmRelease |
| `kubernetes/infrastructure/kafka/kafka.yaml` | Kafka 4.1.1 KRaft cluster (3 nodes, 250Gi each) |
| `kubernetes/infrastructure/kafka/topics.yaml` | snowplow-raw, snowplow-enriched, snowplow-bad |
| `kubernetes/infrastructure/kafka/users.yaml` | snowplow-collector, snowplow-enrich, kafka-connect users |
| `kubernetes/infrastructure/kafka/user-passwords.sops.yaml` | SCRAM passwords (SOPS encrypted) |
| `kubernetes/infrastructure/snowplow/collector-config.yaml` | Collector 3.7.0 HOCON config |
| `kubernetes/infrastructure/snowplow/enrich-config.yaml` | Enrich 6.8.0 HOCON config |
| `kubernetes/infrastructure/snowplow/enrichments-config.yaml` | Empty enrichments dir (no enrichments applied) |
| `kubernetes/infrastructure/snowplow/kafka-credentials.sops.yaml` | JAAS config strings (SOPS encrypted) |
| `kubernetes/infrastructure/kafka-connect/connect.yaml` | KafkaConnect CR, SCRAM auth via spec.authentication |
| `kubernetes/infrastructure/kafka-connect/connector-s3-sink.yaml` | S3 sink connector CR |
| `kubernetes/infrastructure/kafka-connect/connect-credentials.sops.yaml` | S3 keys + Kafka password (SOPS encrypted) |
| `kubernetes/networking/routes/snowplow-collector.yaml` | HTTPRoute for events.yral.com |
| `docker/kafka-connect-plugins/Dockerfile` | Strimzi base + Confluent S3 Sink 12.1.0 |
| `.github/workflows/build-kafka-connect-plugins.yml` | Builds + pushes ghcr.io/dolr-ai/kafka-connect-plugins:latest |

---

## Pending / Next Steps

### 1. Verify S3 data is landing (immediate)

After a few minutes of traffic (or once the first flush interval fires), check that
objects are appearing in the bucket:

```bash
# Using the AWS CLI configured with Hetzner FSN1 credentials
aws s3 ls s3://yral-events-data-lake/events/enriched/ \
  --endpoint-url https://fsn1.your-objectstorage.com --recursive | head -20
```

If no objects appear after ~10 minutes of collector traffic, check the connector task logs:
```bash
kubectl logs -n kafka-connect connect-cluster-connect-0 | grep -i "error\|s3\|sink" | tail -30
```

### 2. Send a test event to validate end-to-end

```bash
# Snowplow GET pixel — should return 200 + set sp cookie
curl -v "https://events.yral.com/i?e=pv&url=https%3A%2F%2Fyral.com&page=Test"
```

Then verify it flows through to the enriched topic:
```bash
kubectl exec -n kafka kafka-cluster-combined-0 -- \
  bin/kafka-console-consumer.sh \
  --bootstrap-server kafka-cluster-kafka-bootstrap.kafka.svc.cluster.local:9092 \
  --topic snowplow-enriched \
  --from-beginning \
  --max-messages 1 \
  --consumer-property security.protocol=SASL_PLAINTEXT \
  --consumer-property sasl.mechanism=SCRAM-SHA-512 \
  --consumer-property 'sasl.jaas.config=org.apache.kafka.common.security.scram.ScramLoginModule required username="kafka-connect" password="6gEGoE+JAWwRb8UM7fLu5Zi5/RJPctHtR3Eji/gQIFo=";'
```

### 3. Add Snowplow enrichments (optional, when needed)

Currently no enrichments are applied (`enrichments-config.yaml` contains an empty
`enrichments/` directory). To add enrichments (e.g. IP lookup, UA parser):

1. Add enrichment JSON files to the `enrichments-config.yaml` ConfigMap in
   `kubernetes/infrastructure/snowplow/enrichments-config.yaml`
2. Commit and push — Reloader will restart enrich pods automatically

### 4. Downstream connectors (deferred)

BigQuery and Mixpanel connectors were deferred during initial implementation.
When ready, add them as additional `KafkaConnector` CRs in
`kubernetes/infrastructure/kafka-connect/` and reference them in its
`kustomization.yaml`.

---

## Known Issues / Gotchas Discovered During Deployment

- **Strimzi namespaces must pre-exist before HelmRelease**: The `strimzi`
  Kustomization now includes `kafka/namespace.yaml` and
  `kafka-connect/namespace.yaml` to prevent the operator from failing to create
  RoleBindings during initial install.

- **KafkaConnect auth**: Must use `spec.authentication` (Strimzi injects SASL
  config automatically). Setting `security.protocol`/`sasl.jaas.config` manually
  in `spec.config` does NOT work at worker bootstrap time.

- **Confluent S3 Sink 12.x uses AWS SDK v2**: `com.amazonaws.auth.EnvironmentVariableCredentialsProvider` (SDK v1) is gone. The SDK v2 default credentials chain reads `AWS_ACCESS_KEY_ID`/`AWS_SECRET_ACCESS_KEY` automatically — no `s3.credentials.provider.class` needed.

- **EnvVarConfigProvider**: Only the native Kafka provider works
  (`org.apache.kafka.common.config.provider.EnvVarConfigProvider`). Strimzi's
  own provider was removed in 0.46.0.
