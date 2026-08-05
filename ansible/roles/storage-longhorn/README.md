---
# storage-longhorn — prepare worker disks for Longhorn storage.
#
# This is the Longhorn equivalent of the Ceph storage-setup role. Instead of
# creating Ceph OSD partitions, it expands btrfs to the second NVMe drive and
# creates the Longhorn data directory with nodatacow.
#
# Called from the migrate-node-to-longhorn playbook after ceph-osd-removal
# and node reprovisioning (provision role).
#
# Usage: part of the add-worker or migrate-node-to-longhorn playbook.