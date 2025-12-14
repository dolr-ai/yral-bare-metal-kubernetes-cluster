export DEBIAN_FRONTEND=noninteractive
apt update && apt full-upgrade -y && apt autoremove -y

curl --request GET -sL \
       --url "https://github.com/coreos/butane/releases/download/v0.25.1/butane-$(uname -m)-unknown-linux-gnu"\
       --output "/usr/local/bin/butane"
chmod +x /usr/local/bin/butane

apt install podman -y

# mdadm --stop /dev/md0
# sleep 2 # Give it a moment

# Zero out RAID superblocks on both NVMe drives
# mdadm --zero-superblock /dev/nvme0n1
# mdadm --zero-superblock /dev/nvme1n1
# sleep 2 # Give it a moment

# Confirm no RAID signatures remain
wipefs -a /dev/nvme0n1
wipefs -a /dev/nvme1n1

# Zero out RAID superblocks on both SATA drives
# mdadm --zero-superblock /dev/sda
# mdadm --zero-superblock /dev/sdb
# sleep 2 # Give it a moment

# Confirm no RAID signatures remain
wipefs -a /dev/sda
wipefs -a /dev/sdb

sleep 5

GITHUB_REPO="dolr-ai/yral-bare-metal-kubernetes-cluster"
LATEST_RELEASE=$(curl -s "https://api.github.com/repos/${GITHUB_REPO}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')

echo "Latest release: ${LATEST_RELEASE}"

curl -sLO "https://github.com/${GITHUB_REPO}/releases/download/${LATEST_RELEASE}/control-plane.ign"
curl -sLO "https://github.com/${GITHUB_REPO}/releases/download/${LATEST_RELEASE}/SHA256SUMS"

# Verify the checksum (important!)
sha256sum -c SHA256SUMS --ignore-missing

podman run --pull=always --privileged --rm \
  --network=host \
  -v /dev:/dev -v /run/udev:/run/udev -v $(pwd):/data -w /data \
  quay.io/coreos/coreos-installer:release \
  install /dev/nvme0n1 \
  --ignition-file control-plane.ign \
  --stream stable \
  --architecture x86_64
