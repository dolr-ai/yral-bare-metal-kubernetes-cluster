I am setting up Hetzner Robot bare metal nodes with Kubernetes. 

- We will do everything as declaratively as possible using the Kubernetes Cluster API.
- Use Hetzner Robot bare metal nodes with Kubernetes using the Cluster API Provider Hetzner (CAPH).
- We will use Cilium as the CNI and Longhorn as the CSI. please help setup the cluster.
- I want to use maximum Kubernetes native features instead of any provider specific feature
- Data replication of 2 with Longhorn CSI
- Auto update to latest stable releases as much as possible
- I already have servers provisioned via Hetzner robot. They are accessible via SSH
- The servers have 2 physical disks of the same size attached as sda and sdb or nvme0n1 and nvme1n1. Should be setup for longhorn to later utilize them
- I want the management cluster to also reside on the bare metal instances. We'll start with 3. Everything on bare metal
- Step by step changes and make 1 change at a time
