- Re-evaluate the deployed services and see if they need more replicas or resizing of their existing replicas as suggested in their default deployment guides and adjust accordingly.
- Evaluate if we should setup affinity rules or taints to have latency sensitive workloads run on nodes near to each other
- Is there a UI for Flux where we can follow deployment status?
- Flux logs via Loki:
    > The official Flux team maintains three dashboards as JSON in their repo: cluster.json, control-plane.json, and logs.json (logs needs Loki — skip that one). Let me check the full HelmRelease values before editing:
    Might as well also get the logs