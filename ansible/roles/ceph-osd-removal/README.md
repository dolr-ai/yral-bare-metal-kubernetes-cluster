---
# ceph-osd-removal — safely remove a node's Ceph OSDs before reprovisioning.
#
# Called from the migrate-node-to-longhorn playbook BEFORE node-drain.
# The node must still be running and its OSDs must be `up` so Ceph can
# rebalance data off them cleanly.
#
# Usage: ansible-playbook ... -e target_host=worker-X