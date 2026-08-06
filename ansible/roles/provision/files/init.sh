#!/bin/bash
set -e

# Hetzner bare metal installation script — runs inside the rescue system.
# Called by the provision Ansible role (ansible/roles/provision/tasks/main.yml).
#
# This script is copied to the rescue system via Ansible's copy module (not
# downloaded from GitHub) so provisioning works even if the repo is private or
# the GitHub raw endpoint is unavailable.
#
# Environment variables:
#   MACHINE_HOSTNAME    — hostname for the installed system (required)
#   DRIVE_TO_USE        — target drive, e.g. nvme0n1 or sda (required)
#   OS_PARTITION_SIZE   — root partition size: '50G' for workers, 'all' for CPs (default: all)

# Check if we're in Hetzner rescue system
if [ ! -f /root/.oldroot/nfs/install/installimage ]; then
    echo "ERROR: installimage command not found!"
    echo "This script must be run from Hetzner's rescue system."
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

# Wipe filesystems AND partition tables on all drives.
#   sgdisk --zap-all  — destroys GPT primary + backup partition tables + MBR.
#   wipefs -fa        — removes filesystem/RAID superblock signatures (btrfs, ext4, LVM, mdraid).
echo "Wiping existing partition tables and filesystems..."
if [[ "$DRIVE_TO_USE" == nvme* ]]; then
    if ls /dev/nvme*n1 2>/dev/null | grep -q .; then
        for drive in /dev/nvme*n1; do
            if [ -b "$drive" ]; then
                echo "Wiping $drive"
                sgdisk --zap-all "$drive" 2>/dev/null || true
                wipefs -fa "$drive" 2>/dev/null || true
            fi
        done
    else
        echo "No NVMe drives found"
    fi
else
    if ls /dev/sd* 2>/dev/null | grep -q .; then
        for drive in /dev/sd?; do
            if [ -b "$drive" ]; then
                echo "Wiping $drive"
                sgdisk --zap-all "$drive" 2>/dev/null || true
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

OS_PARTITION_SIZE="${OS_PARTITION_SIZE:-all}"
echo "OS partition size: ${OS_PARTITION_SIZE}"

# Write installimage autosetup config file.
#
# FORCE_GPT 2 — forces a GPT partition table regardless of UEFI/BIOS mode.
# installimage auto-inserts a BIOS boot partition (~1 MiB) for GRUB.
#
# Ubuntu 26.04 (resolute) — all cluster nodes run Ubuntu 26.04.
#
# Resulting partition layout (workers, OS_PARTITION_SIZE=50G):
#   nvme0n1p1  ~1 MiB   BIOS boot (auto, for GRUB stage1.5)
#   nvme0n1p2  50 GiB   btrfs root
#   [free]     ~450 GiB → storage-longhorn role adds to btrfs RAID0
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
IMAGE /root/images/Ubuntu-resolute-latest-amd64-base.tar.zst
FORCE_GPT 2
AUTOEOF

echo "Running installimage..."
# Use full path — the installimage alias is not available in non-interactive shells.
# When /autosetup exists, installimage auto-detects it and runs in non-interactive automode.
/root/.oldroot/nfs/install/installimage

echo "INSTALLATION COMPLETE"