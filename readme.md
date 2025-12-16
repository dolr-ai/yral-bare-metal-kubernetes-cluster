# Run on Hetzner Rescue Mode

## Control Plane

```bash
curl -s https://raw.githubusercontent.com/dolr-ai/yral-bare-metal-kubernetes-cluster/refs/heads/main/hetzner-rescue/node.sh | bash
```

# Prompt 

If I am setting up Hetzner Robot bare metal nodes for use with the Kubernetes Cluster API bare metal and intend to use Cilium as the CNI and Longhorn as the CSI, what will my setup look like?
 

* I want to use maximum Kubernetes native features instead of any provider specific feature. So preferable to use bare metal cluster api provider instead of caph which is hetzner specific

* Want to have everything as declarative in my git repo as possible

* Data replication of 2 with Longhorn CSI

* Fedora CoreOS as the auto updating OS

* Everything declarative as much as possible in my git repo

* Make changes, commit to Github and have Github CI run and apply changes automatically to my cluster

* Auto update to latest stable releases as much as possible
- I already have servers provisioned via Hetzner robot. They are accessible via SSH
- The servers have 2 physical disks of the same size attached as sda and sdb or nvme0n1 and nvme1n1. How should they be setup for longhorn to later utilize them
- I want the management cluster to also reside on the bare metal instances. We'll start with 3. Everything on bare metal

For the above, I have 3 control plane node machines initialized. What are next steps?
