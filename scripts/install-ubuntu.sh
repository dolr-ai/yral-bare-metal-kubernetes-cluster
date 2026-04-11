#!/bin/bash

set -e

# Check if we're in Hetzner rescue system
# installimage is typically an alias pointing to /root/.oldroot/nfs/install/installimage
# We need to check for the actual file since aliases don't work in non-interactive shells
if [ ! -f /root/.oldroot/nfs/install/installimage ]; then
    echo "ERROR: installimage command not found!"
    echo "This script must be run from Hetzner's rescue system."
    echo ""
    echo "To boot into rescue mode:"
    echo "1. Go to Hetzner Robot panel"
    echo "2. Select your server"
    echo "3. Click 'Rescue' tab"
    echo "4. Activate Linux rescue system"
    echo "5. Reset/reboot your server"
    echo "6. SSH into the rescue system and run this script"
    exit 1
fi

# Validate required environment variables
if [ -z "$MACHINE_HOSTNAME" ]; then
    echo "ERROR: MACHINE_HOSTNAME environment variable is not set"
    exit 1
fi

if [ -z "$DRIVE_TO_USE" ]; then
    echo "ERROR: DRIVE_TO_USE environment variable is not set"
    echo "Example: DRIVE_TO_USE='nvme0n1' or DRIVE_TO_USE='sda'"
    exit 1
fi

# Validate drive type
if [[ "$DRIVE_TO_USE" != nvme* ]] && [[ "$DRIVE_TO_USE" != sd* ]]; then
    echo "ERROR: Unsupported drive type: $DRIVE_TO_USE"
    echo "Supported: nvme0n1, nvme1n1, sda, sdb"
    exit 1
fi

echo "Hetzner Bare Metal Installation Script"
echo "Hostname: $MACHINE_HOSTNAME"
echo "Installation Drive: /dev/$DRIVE_TO_USE"

# Stop any existing mdadm arrays if they exist
echo "Stopping existing RAID arrays..."
if ls /dev/md* 2>/dev/null | grep -q .; then
    for md in /dev/md*; do
        if [ -b "$md" ]; then
            echo "Stopping $md"
            mdadm --stop "$md" 2>/dev/null || true
        fi
    done
else
    echo "No mdadm arrays found to stop"
fi

# Wipe filesystems based on drive type
echo "Wiping existing filesystems..."
if [[ "$DRIVE_TO_USE" == nvme* ]]; then
    # For NVMe drives
    if ls /dev/nvme*n1 2>/dev/null | grep -q .; then
        for drive in /dev/nvme*n1; do
            if [ -b "$drive" ]; then
                echo "Wiping $drive"
                wipefs -fa "$drive" 2>/dev/null || true
            fi
        done
    else
        echo "No NVMe drives found"
    fi
else
    # For SATA drives
    if ls /dev/sd* 2>/dev/null | grep -q .; then
        for drive in /dev/sd?; do
            if [ -b "$drive" ]; then
                echo "Wiping $drive"
                wipefs -fa "$drive" 2>/dev/null || true
            fi
        done
    else
        echo "No SATA drives found"
    fi
fi

# Verify the target drive exists
if [ ! -b "/dev/$DRIVE_TO_USE" ]; then
    echo "ERROR: Drive /dev/$DRIVE_TO_USE does not exist!"
    echo "Available drives:"
    lsblk -d -o NAME,SIZE,TYPE | grep disk
    exit 1
fi

echo "Starting installimage..."

# OS_PARTITION_SIZE controls the root filesystem size on the primary drive.
# Defaults to 'all' (use the entire disk), which is correct for control planes.
# Worker nodes set this to '50G' during provisioning so that ~450 GB of the
# primary NVMe remains as raw unallocated space — the storage-setup role then
# creates a GPT partition there for a Rook/Ceph OSD, and the second NVMe is
# left entirely raw for a second Ceph OSD.  This maximises the cluster-wide
# Ceph pool to ~950 GB per worker (50 GB OS + 450 GB partition + 500 GB nvme1).
OS_PARTITION_SIZE="${OS_PARTITION_SIZE:-all}"

echo "OS partition size: ${OS_PARTITION_SIZE}"

# Write installimage autosetup config file.
#
# FORCE_GPT 2 — forces a GPT partition table regardless of whether the rescue
# system is running in UEFI or legacy BIOS mode, and regardless of disk size.
# Without it, installimage defaults to MBR on disks smaller than 2 TiB when
# the rescue system boots in BIOS mode, which prevents sgdisk from later
# adding a GPT partition for the Ceph OSD without the --mbrtogpt workaround.
# With FORCE_GPT 2, installimage automatically inserts a small BIOS boot
# partition (type bios_grub, ~1 MiB) as the first partition so GRUB can still
# boot in legacy-BIOS mode from the GPT disk.
#
# Resulting partition layout (workers):
#   nvme0n1p1  ~1 MiB   BIOS boot (auto, for GRUB stage1.5)
#   nvme0n1p2  50 GiB   btrfs root
#   [free]     ~450 GiB → storage-setup adds nvme0n1p3 (ceph-osd)
#
# Resulting partition layout (control planes, OS_PARTITION_SIZE=all):
#   nvme0n1p1  ~1 MiB   BIOS boot (auto)
#   nvme0n1p2  all      btrfs root
cat > /autosetup << AUTOEOF
DRIVE1 /dev/${DRIVE_TO_USE}
SWRAID 0
FORMATDRIVE1 yes
HOSTNAME ${MACHINE_HOSTNAME}
BOOTLOADER grub
PART / btrfs ${OS_PARTITION_SIZE}
IMAGE /root/images/Ubuntu-2404-noble-amd64-base.tar.gz
FORCE_GPT 2
AUTOEOF

echo "Running installimage..."
# Use full path — the installimage alias is not available in non-interactive shells.
# Do NOT pass -a here.  When /autosetup exists, installimage automatically detects
# it and switches to non-interactive automode.  Passing -a without also providing
# the config and image via -c/-i flags produces:
#   "ERROR: in automatic mode you need to specify an image and a config file!"
# The /autosetup detection route is the correct non-interactive path.
/root/.oldroot/nfs/install/installimage

echo "Installation complete!"
