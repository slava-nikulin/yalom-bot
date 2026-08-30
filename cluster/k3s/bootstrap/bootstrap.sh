#!/usr/bin/env bash

set -Eeuo pipefail

if [[ $# -ne 1 ]]; then
    echo "Usage:"
    echo "  $0 <ssh-destination>"
    echo
    echo "Example:"
    echo "  $0 ubuntu@203.0.113.10"
    exit 1
fi

SSH_DESTINATION="$1"

SCRIPT_DIR="$(
    cd -- "$(dirname -- "${BASH_SOURCE[0]}")" >/dev/null 2>&1
    pwd
)"

K3S_DIR="$(
    cd -- "${SCRIPT_DIR}/.." >/dev/null 2>&1
    pwd
)"


if ! command -v ssh >/dev/null 2>&1; then
    echo "ssh is required"
    exit 1
fi

if ! command -v tar >/dev/null 2>&1; then
    echo "tar is required"
    exit 1
fi


echo "Bootstrapping k3s cluster on ${SSH_DESTINATION}"
echo


REMOTE_COMMAND="$(cat <<'EOF'
set -Eeuo pipefail

tmp="$(mktemp -d)"

cleanup() {
    rm -rf "${tmp}"
}

trap cleanup EXIT

tar -xzf - -C "${tmp}"

sudo bash \
    "${tmp}/bootstrap/bootstrap-server.sh" \
    "${tmp}"
EOF
)"


tar \
    -C "${K3S_DIR}" \
    -czf - \
    . \
    | ssh "${SSH_DESTINATION}" "${REMOTE_COMMAND}"