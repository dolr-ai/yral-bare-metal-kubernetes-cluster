# Storage and Workload Scheduling Strategy

## Node Labels

All nodes are labeled with disk type information to enable intelligent scheduling:

### Current Configuration (All NVMe)
- **Label**: `node.yral.com/disk-type=nvme`
- **Workload Preference**: `node.yral.com/workload-preference=stateful`
- **Applied to**: All 16 nodes (3 control planes + 13 workers)

### Future SATA Nodes
When adding SATA disk servers, label them as:
- **Label**: `node.yral.com/disk-type=sata`
- **Workload Preference**: `node.yral.com/workload-preference=stateless`

## Scheduling Strategies

### Stateful Workloads (Databases, StatefulSets)

**Use NVMe nodes with required affinity:**

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: database-pod
spec:
  affinity:
    nodeAffinity:
      requiredDuringSchedulingIgnoredDuringExecution:
        nodeSelectorMatchExpressions:
        - key: node.yral.com/disk-type
          operator: In
          values:
          - nvme
  containers:
  - name: database
    image: postgres:15
```

### Stateless Workloads (Web Apps, API Services)

**Prefer SATA nodes, fall back to NVMe:**

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: web-app-pod
spec:
  affinity:
    nodeAffinity:
      preferredDuringSchedulingIgnoredDuringExecution:
      - weight: 100
        preference:
          matchExpressions:
          - key: node.yral.com/disk-type
            operator: In
            values:
            - sata
      - weight: 50
        preference:
          matchExpressions:
          - key: node.yral.com/workload-preference
            operator: In
            values:
            - stateless
  containers:
  - name: web-app
    image: nginx:latest
```

## Storage Classes

### High-Performance Storage Class (NVMe)

```yaml
apiVersion: storage.k8s.io/v1
kind: StorageClass
metadata:
  name: local-nvme
  annotations:
    storageclass.kubernetes.io/is-default-class: "true"
provisioner: kubernetes.io/no-provisioner
volumeBindingMode: WaitForFirstConsumer
allowedTopologies:
- matchLabelExpressions:
  - key: node.yral.com/disk-type
    values:
    - nvme
```

### Standard Storage Class (SATA) - For future use

```yaml
apiVersion: storage.k8s.io/v1
kind: StorageClass
metadata:
  name: local-sata
provisioner: kubernetes.io/no-provisioner
volumeBindingMode: WaitForFirstConsumer
allowedTopologies:
- matchLabelExpressions:
  - key: node.yral.com/disk-type
    values:
    - sata
```

## Deployment Examples

### StatefulSet with NVMe Requirement

```yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: cassandra
spec:
  serviceName: cassandra
  replicas: 3
  selector:
    matchLabels:
      app: cassandra
  template:
    metadata:
      labels:
        app: cassandra
    spec:
      affinity:
        nodeAffinity:
          requiredDuringSchedulingIgnoredDuringExecution:
            nodeSelectorMatchExpressions:
            - key: node.yral.com/disk-type
              operator: In
              values:
              - nvme
        podAntiAffinity:
          requiredDuringSchedulingIgnoredDuringExecution:
          - labelSelector:
              matchExpressions:
              - key: app
                operator: In
                values:
                - cassandra
            topologyKey: kubernetes.io/hostname
      containers:
      - name: cassandra
        image: cassandra:4.1
        volumeMounts:
        - name: data
          mountPath: /var/lib/cassandra
  volumeClaimTemplates:
  - metadata:
      name: data
    spec:
      accessModes: ["ReadWriteOnce"]
      storageClassName: local-nvme
      resources:
        requests:
          storage: 100Gi
```

### Deployment with SATA Preference

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: web-frontend
spec:
  replicas: 10
  selector:
    matchLabels:
      app: web-frontend
  template:
    metadata:
      labels:
        app: web-frontend
    spec:
      affinity:
        nodeAffinity:
          preferredDuringSchedulingIgnoredDuringExecution:
          - weight: 100
            preference:
              matchExpressions:
              - key: node.yral.com/disk-type
                operator: In
                values:
                - sata
          - weight: 50
            preference:
              matchExpressions:
              - key: node.yral.com/workload-preference
                operator: In
                values:
                - stateless
      containers:
      - name: nginx
        image: nginx:latest
        resources:
          requests:
            cpu: 100m
            memory: 128Mi
```

## Cluster Node Distribution

### Current Setup (16 nodes, all NVMe)

**Control Plane (3 nodes):**
- control-plane-1: HEL1-DC2, Helsinki, NVMe
- control-plane-2: HEL1-DC2, Helsinki, NVMe
- control-plane-3: HEL1-DC2, Helsinki, NVMe

**Worker Nodes (13 nodes):**
- worker-1 to worker-9: FSN1 (Falkenstein), NVMe
- worker-10 to worker-12: HEL1 (Helsinki), NVMe
- worker-13: FSN1-DC11 (Falkenstein), NVMe

**Geographic Distribution:**
- Helsinki (HEL1): 6 nodes (3 control + 3 workers)
- Falkenstein (FSN1): 10 nodes (all workers)

## Best Practices

1. **Stateful workloads**: Always use `requiredDuringSchedulingIgnoredDuringExecution` with NVMe selector
2. **Stateless workloads**: Use `preferredDuringSchedulingIgnoredDuringExecution` to prefer SATA
3. **Anti-affinity**: Spread replicas across availability zones using topology labels
4. **Storage classes**: Use appropriate storage class in PVC definitions
5. **Resource requests**: Always define CPU/memory requests for proper scheduling

## Verification Commands

```bash
# Check node labels
kubectl get nodes -L node.yral.com/disk-type,node.yral.com/workload-preference

# Check pods scheduled on NVMe nodes
kubectl get pods -A -o wide --field-selector spec.nodeName=worker-1

# Check pod affinity rules
kubectl get pod <pod-name> -o yaml | grep -A 20 affinity

# List storage classes
kubectl get storageclass
```
