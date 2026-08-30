#!/usr/bin/env bash
set -Eeuo pipefail

if [[ $# -ne 1 ]]; then
    echo "Usage:"
    echo "  $0 <ssh-destination>"
    echo
    echo "Example:"
    echo "  $0 oracle-frankfurt"
    exit 1
fi

SSH_DESTINATION="$1"
DEPLOY_KEY="${HOME}/.ssh/yalom_github_actions"


require() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "$1 is required"
        exit 1
    fi
}


require gh
require ssh
require ssh-keygen


echo "==> Checking GitHub authentication"

gh auth status >/dev/null


echo "==> Resolving GitHub repository"

REPOSITORY="$(
    gh repo view \
        --json nameWithOwner \
        --jq '.nameWithOwner'
)"

echo "Repository: ${REPOSITORY}"


echo "==> Resolving SSH host"

SSH_HOST="$(
    ssh -G "${SSH_DESTINATION}" \
        | awk '$1 == "hostname" { print $2; exit }'
)"

if [[ -z "${SSH_HOST}" ]]; then
    echo "Could not resolve SSH hostname"
    exit 1
fi

echo "SSH host: ${SSH_HOST}"


echo "==> Creating GitHub Actions deploy key"

if [[ ! -f "${DEPLOY_KEY}" ]]; then
    ssh-keygen \
        -t ed25519 \
        -N "" \
        -f "${DEPLOY_KEY}" \
        -C "github-actions-yalom"
else
    echo "Deploy key already exists"
fi


echo "==> Installing deploy public key on server"

PUBLIC_KEY="$(cat "${DEPLOY_KEY}.pub")"

ssh "${SSH_DESTINATION}" "
    set -Eeuo pipefail

    umask 077
    mkdir -p ~/.ssh
    touch ~/.ssh/authorized_keys

    if ! grep -qxF '${PUBLIC_KEY}' ~/.ssh/authorized_keys; then
        printf '%s\n' '${PUBLIC_KEY}' >> ~/.ssh/authorized_keys
    fi
"


echo "==> Verifying deploy key"

ssh \
    -i "${DEPLOY_KEY}" \
    -o IdentitiesOnly=yes \
    "${SSH_DESTINATION}" \
    'true'


echo "==> Reading trusted SSH host key"

KNOWN_HOSTS="$(
    ssh-keygen \
        -F "${SSH_HOST}" \
        -f "${HOME}/.ssh/known_hosts" \
        2>/dev/null \
        | grep -v '^#' \
        || true
)"

if [[ -z "${KNOWN_HOSTS}" ]]; then
    echo "No trusted known_hosts entry found for ${SSH_HOST}"
    echo "Connect to the server with SSH and verify its host key first."
    exit 1
fi


echo "==> Configuring GitHub Actions"

gh variable set OCI_SSH_HOST \
    --repo "${REPOSITORY}" \
    --body "${SSH_HOST}"

gh secret set OCI_SSH_PRIVATE_KEY \
    --repo "${REPOSITORY}" \
    < "${DEPLOY_KEY}"

printf '%s\n' "${KNOWN_HOSTS}" \
    | gh secret set OCI_SSH_KNOWN_HOSTS \
        --repo "${REPOSITORY}"


echo "==> Done"

echo
echo "Configured for ${REPOSITORY}:"
echo "  OCI_SSH_HOST"
echo "  OCI_SSH_PRIVATE_KEY"
echo "  OCI_SSH_KNOWN_HOSTS"