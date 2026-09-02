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

# Run installimage with full path (alias doesn't work in non-interactive shells)
# Install on single drive - second drive will be added after first boot
# Ubuntu 26.04 (resolute) — all fleet servers run Ubuntu 26.04 per AGENTS.md
/root/.oldroot/nfs/install/installimage -a \
    -n "$MACHINE_HOSTNAME" \
    -r no \
    -l 0 \
    -p /:btrfs:all \
    -d "$DRIVE_TO_USE" \
    -f yes \
    -t yes \
    -i /root/images/Ubuntu-resolute-latest-amd64-base.tar.zst

echo "Installation complete!"