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
K3S_MANIFEST_DIR="/var/lib/rancher/k3s/server/manifests"


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


log "Installing Traefik cluster configuration"

mkdir -p "${K3S_MANIFEST_DIR}"

install_if_changed \
    "${BUNDLE_DIR}/traefik/helm-chart-config.yaml" \
    "${K3S_MANIFEST_DIR}/10-traefik-config.yaml" \
    || true


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

log "Waiting for Gateway API CRDs"

gateway_api_ready=0

for _ in $(seq 1 90); do
    if k3s kubectl get \
        crd/gateways.gateway.networking.k8s.io \
        >/dev/null 2>&1 \
        && \
       k3s kubectl get \
        crd/httproutes.gateway.networking.k8s.io \
        >/dev/null 2>&1; then

        gateway_api_ready=1
        break
    fi

    sleep 2
done

if [[ "${gateway_api_ready}" -ne 1 ]]; then
    echo "Gateway API CRDs did not become ready"
    exit 1
fi


log "Installing External Secrets Operator"

k3s kubectl apply \
    -f "${BUNDLE_DIR}/external-secrets/helm-chart.yaml"


log "Installing cert-manager"

k3s kubectl apply \
    -f "${BUNDLE_DIR}/cert-manager/helm-chart.yaml"


log "Waiting for Traefik"

for _ in $(seq 1 60); do
    if k3s kubectl \
        -n kube-system \
        get daemonset/traefik \
        >/dev/null 2>&1; then
        break
    fi

    sleep 2
done

k3s kubectl \
    -n kube-system \
    rollout status daemonset/traefik \
    --timeout=300s


log "Waiting for External Secrets Operator"

for _ in $(seq 1 60); do
    if k3s kubectl \
        -n external-secrets \
        get deployment \
        -o name 2>/dev/null \
        | grep -q .; then
        break
    fi

    sleep 2
done

k3s kubectl \
    -n external-secrets \
    wait \
    --for=condition=Available \
    deployment \
    --all \
    --timeout=300s


log "Waiting for cert-manager"

for _ in $(seq 1 60); do
    if k3s kubectl \
        -n cert-manager \
        get deployment \
        -o name 2>/dev/null \
        | grep -q .; then
        break
    fi

    sleep 2
done

k3s kubectl \
    -n cert-manager \
    wait \
    --for=condition=Available \
    deployment \
    --all \
    --timeout=300s


log "Applying Gateway configuration"

k3s kubectl apply \
    -k "${BUNDLE_DIR}/gateway"


log "Waiting for GatewayClass"

k3s kubectl wait \
    --for=condition=Accepted \
    gatewayclass/traefik \
    --timeout=180s


log "Configuring Let's Encrypt issuer"

k3s kubectl apply \
    -f "${BUNDLE_DIR}/cert-manager/cluster-issuer.yaml"

k3s kubectl wait \
    --for=condition=Ready \
    clusterissuer/letsencrypt-shortlived \
    --timeout=180s


log "Requesting public IP certificate"

k3s kubectl apply \
    -f "${BUNDLE_DIR}/cert-manager/certificate.yaml"

k3s kubectl \
    -n gateway-system \
    wait \
    --for=condition=Ready \
    certificate/public-ip \
    --timeout=600s

log "Configuring Traefik default TLS certificate"

k3s kubectl apply \
    -f "${BUNDLE_DIR}/traefik/tls-store.yaml"

log "Waiting for public Gateway"

k3s kubectl \
    -n gateway-system \
    wait \
    --for=condition=Programmed \
    gateway/public-gateway \
    --timeout=180s


log "Cluster bootstrap completed"

k3s kubectl get nodes -o wide

echo

k3s kubectl \
    -n kube-system \
    get daemonset traefik

echo

k3s kubectl get gatewayclass

echo

k3s kubectl \
    -n gateway-system \
    get gateway

echo

k3s kubectl get clusterissuer

echo

k3s kubectl \
    -n gateway-system \
    get certificate