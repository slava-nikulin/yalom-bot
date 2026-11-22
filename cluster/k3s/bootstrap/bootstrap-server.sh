#!/usr/bin/env bash

set -Eeuo pipefail

if [[ "${EUID}" -ne 0 ]]; then
    echo "bootstrap-server.sh must be run as root"
    exit 1
fi

if [[ $# -ne 1 ]]; then
    echo "Usage: bootstrap-server.sh <bundle-dir>"
    exit 1
fi

BUNDLE_DIR="$1"

# Pinned cluster version.
K3S_VERSION="v1.36.3+k3s1"

K3S_CONFIG_DIR="/etc/rancher/k3s"
K3S_CONFIG="${K3S_CONFIG_DIR}/config.yaml"


log() {
    printf '\n==> %s\n' "$1"
}


install_if_changed() {
    local source="$1"
    local destination="$2"
    local mode="${3:-0644}"

    mkdir -p "$(dirname "${destination}")"

    if [[ -f "${destination}" ]] && cmp -s "${source}" "${destination}"; then
        return 1
    fi

    install -m "${mode}" "${source}" "${destination}"
    return 0
}


log "Checking prerequisites"

if ! command -v curl >/dev/null 2>&1; then
    apt-get update
    apt-get install -y curl ca-certificates
fi

log "Configuring host forwarding for k3s"

PERSISTENT_RULES="/etc/iptables/rules.v4"
FORWARD_REJECT='-A FORWARD -j REJECT --reject-with icmp-host-prohibited'

if [[ -f "${PERSISTENT_RULES}" ]] \
    && grep -Fxq -- "${FORWARD_REJECT}" "${PERSISTENT_RULES}"; then

    if [[ ! -f "${PERSISTENT_RULES}.pre-k3s.bak" ]]; then
        cp "${PERSISTENT_RULES}" "${PERSISTENT_RULES}.pre-k3s.bak"
    fi

    sed -i \
        '\|^-A FORWARD -j REJECT --reject-with icmp-host-prohibited$|d' \
        "${PERSISTENT_RULES}"
fi

if iptables -C FORWARD \
    -j REJECT \
    --reject-with icmp-host-prohibited \
    >/dev/null 2>&1; then

    iptables -D FORWARD \
        -j REJECT \
        --reject-with icmp-host-prohibited
fi


log "Installing k3s configuration"

restart_k3s=0

if install_if_changed \
    "${BUNDLE_DIR}/config/server.yaml" \
    "${K3S_CONFIG}"; then

    restart_k3s=1
fi


log "Checking k3s version"

installed_version=""

if command -v k3s >/dev/null 2>&1; then
    installed_version="$(k3s --version | awk 'NR == 1 { print $3 }')"
fi

if [[ "${installed_version}" != "${K3S_VERSION}" ]]; then
    log "Installing k3s ${K3S_VERSION}"

    curl -sfL https://get.k3s.io \
        | INSTALL_K3S_VERSION="${K3S_VERSION}" \
          INSTALL_K3S_SKIP_START=true \
          sh -

    restart_k3s=1
else
    log "k3s ${K3S_VERSION} is already installed"
fi


log "Starting k3s"

systemctl enable k3s >/dev/null

if ! systemctl is-active --quiet k3s; then
    systemctl start k3s
elif [[ "${restart_k3s}" -eq 1 ]]; then
    systemctl restart k3s
fi


log "Waiting for Kubernetes API"

api_ready=0

for _ in $(seq 1 90); do
    if k3s kubectl get --raw='/readyz' >/dev/null 2>&1; then
        api_ready=1
        break
    fi

    sleep 2
done

if [[ "${api_ready}" -ne 1 ]]; then
    echo "Kubernetes API did not become ready"
    exit 1
fi

log "Waiting for node registration"

node_registered=0

for _ in $(seq 1 90); do
    if [[ -n "$(k3s kubectl get nodes -o name 2>/dev/null)" ]]; then
        node_registered=1
        break
    fi

    sleep 2
done

if [[ "${node_registered}" -ne 1 ]]; then
    echo "No Kubernetes nodes registered"
    exit 1
fi


log "Waiting for node readiness"

k3s kubectl wait \
    --for=condition=Ready \
    node \
    --all \
    --timeout=180s

log "Configuring OCI Run Command sudo access"

sudoers_file=/etc/sudoers.d/101-oracle-cloud-agent-run-command
sudoers_tmp="$(mktemp)"

trap 'rm -f "$sudoers_tmp"' EXIT

cat >"$sudoers_tmp" <<'EOF'
ocarun ALL=(root) NOPASSWD: /usr/local/bin/k3s
EOF

chmod 440 "$sudoers_tmp"
visudo -cf "$sudoers_tmp"

install_if_changed "$sudoers_tmp" "$sudoers_file" 0440 || true

log "k3s bootstrap completed"

k3s kubectl get nodes -o wide
