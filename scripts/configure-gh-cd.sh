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
KNOWN_HOSTS_FILE="${HOME}/.ssh/known_hosts"


require() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "$1 is required"
        exit 1
    fi
}


require gh
require ssh
require ssh-keygen
require awk


echo "==> Checking GitHub authentication"

gh auth status >/dev/null


echo "==> Resolving GitHub repository"

REPOSITORY="$(
    gh repo view \
        --json nameWithOwner \
        --jq '.nameWithOwner'
)"

echo "Repository: ${REPOSITORY}"


echo "==> Resolving SSH connection"

SSH_CONFIG="$(ssh -G "${SSH_DESTINATION}")"

SSH_HOST="$(
    awk '$1 == "hostname" { print $2; exit }' <<< "${SSH_CONFIG}"
)"

SSH_PORT="$(
    awk '$1 == "port" { print $2; exit }' <<< "${SSH_CONFIG}"
)"

SSH_USER="$(
    awk '$1 == "user" { print $2; exit }' <<< "${SSH_CONFIG}"
)"

if [[ -z "${SSH_HOST}" || -z "${SSH_PORT}" || -z "${SSH_USER}" ]]; then
    echo "Could not resolve SSH connection parameters"
    exit 1
fi

echo "SSH host: ${SSH_HOST}"
echo "SSH port: ${SSH_PORT}"
echo "SSH user: ${SSH_USER}"


echo "==> Reading trusted SSH host key"

if [[ ! -f "${KNOWN_HOSTS_FILE}" ]]; then
    echo "Known hosts file does not exist:"
    echo "  ${KNOWN_HOSTS_FILE}"
    exit 1
fi

if [[ "${SSH_PORT}" == "22" ]]; then
    KNOWN_HOSTS_LOOKUP="${SSH_HOST}"
else
    KNOWN_HOSTS_LOOKUP="[${SSH_HOST}]:${SSH_PORT}"
fi

KNOWN_HOSTS="$(
    ssh-keygen \
        -F "${KNOWN_HOSTS_LOOKUP}" \
        -f "${KNOWN_HOSTS_FILE}" \
        2>/dev/null \
        | grep -v '^#' \
        || true
)"

if [[ -z "${KNOWN_HOSTS}" ]]; then
    echo "No trusted known_hosts entry found for ${KNOWN_HOSTS_LOOKUP}"
    echo "Connect to the server with SSH and verify its host key first."
    exit 1
fi


echo "==> Preparing GitHub Actions deploy key"

install -m 700 -d "${HOME}/.ssh"

if [[ -f "${DEPLOY_KEY}" ]]; then
    if [[ ! -f "${DEPLOY_KEY}.pub" ]]; then
        echo "Public key is missing; deriving it from the existing private key"

        ssh-keygen \
            -y \
            -f "${DEPLOY_KEY}" \
            > "${DEPLOY_KEY}.pub"

        chmod 644 "${DEPLOY_KEY}.pub"
    else
        echo "Deploy key already exists"
    fi
elif [[ -e "${DEPLOY_KEY}.pub" ]]; then
    echo "Deploy key pair is incomplete:"
    echo "  private key is missing: ${DEPLOY_KEY}"
    echo "  public key exists:      ${DEPLOY_KEY}.pub"
    exit 1
else
    ssh-keygen \
        -t ed25519 \
        -N "" \
        -f "${DEPLOY_KEY}" \
        -C "github-actions-yalom"
fi

chmod 600 "${DEPLOY_KEY}"


echo "==> Installing deploy public key on server"

cat "${DEPLOY_KEY}.pub" \
    | ssh \
        -o BatchMode=yes \
        -o StrictHostKeyChecking=yes \
        "${SSH_DESTINATION}" \
        '
            set -Eeuo pipefail

            umask 077

            mkdir -p "${HOME}/.ssh"
            chmod 700 "${HOME}/.ssh"

            touch "${HOME}/.ssh/authorized_keys"
            chmod 600 "${HOME}/.ssh/authorized_keys"

            IFS= read -r public_key

            if ! grep -qxF -- "${public_key}" "${HOME}/.ssh/authorized_keys"; then
                printf "%s\n" "${public_key}" >> "${HOME}/.ssh/authorized_keys"
            fi
        '


echo "==> Verifying deploy key"

TMP_KNOWN_HOSTS="$(mktemp)"

cleanup() {
    rm -f "${TMP_KNOWN_HOSTS}"
}

trap cleanup EXIT

printf '%s\n' "${KNOWN_HOSTS}" > "${TMP_KNOWN_HOSTS}"
chmod 600 "${TMP_KNOWN_HOSTS}"

ssh \
    -F /dev/null \
    -i "${DEPLOY_KEY}" \
    -o IdentitiesOnly=yes \
    -o BatchMode=yes \
    -o StrictHostKeyChecking=yes \
    -o UserKnownHostsFile="${TMP_KNOWN_HOSTS}" \
    -o GlobalKnownHostsFile=/dev/null \
    -p "${SSH_PORT}" \
    "${SSH_USER}@${SSH_HOST}" \
    'true'


echo "==> Configuring GitHub Actions"

gh variable set OCI_SSH_HOST \
    --repo "${REPOSITORY}" \
    --body "${SSH_HOST}"

gh variable set OCI_SSH_PORT \
    --repo "${REPOSITORY}" \
    --body "${SSH_PORT}"

gh variable set OCI_SSH_USER \
    --repo "${REPOSITORY}" \
    --body "${SSH_USER}"

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
echo "  OCI_SSH_PORT"
echo "  OCI_SSH_USER"
echo "  OCI_SSH_PRIVATE_KEY"
echo "  OCI_SSH_KNOWN_HOSTS"