#!/usr/bin/env bash
set -Eeuo pipefail
umask 077

# OCI Bastion and MySQL Studio
REGION="eu-frankfurt-1"
BASTION_ID="ocid1.bastion.oc1.eu-frankfurt-1.amaaaaaahv4cvsqa6wbdf5khzfdciigpoyweymwfnh6naf7qnwlsb7e7fhmq"
TARGET_IP="10.0.2.78"
TARGET_PORT=8443
LOCAL_PORT=18443
SESSION_NAME="yalom-studio-cli"
SESSION_TTL=10800  # 3 hours
DEBUG="${DEBUG:-0}"  # DEBUG=1 ./scripts/connect-studio.sh
SSH_ATTEMPTS=5
SSH_RETRY_DELAY=3

for cmd in oci ssh ssh-keygen curl jq mktemp timeout tail; do
    command -v "$cmd" >/dev/null || { echo "Missing dependency: $cmd" >&2; exit 1; }
done

# Bound API calls as well as OCI work-request waits. Never enable OCI --debug:
# it can log authentication headers; DEBUG below applies only to SSH.
oci() {
    command timeout --foreground 150 oci --connection-timeout 10 \
        --read-timeout 20 --max-retries 2 "$@"
}

SESSION_ID=""
TEMP_DIR=""
SSH_PID=""
stop_ssh() {
    if [[ -n "$SSH_PID" ]]; then
        kill "$SSH_PID" 2>/dev/null || true
        wait "$SSH_PID" 2>/dev/null || true
        SSH_PID=""
    fi
}
cleanup() {
    local status=$?
    trap - EXIT
    stop_ssh
    if [[ "$SESSION_ID" == ocid1.bastionsession.* ]]; then
        echo "Removing OCI Bastion session..."
        if oci bastion session delete --region "$REGION" \
            --session-id "$SESSION_ID" --force \
            --wait-for-state SUCCEEDED --max-wait-seconds 120 \
            --wait-interval-seconds 3 > /dev/null 2>"$TEMP_DIR/cleanup.log"; then
            :
        else
            echo "Warning: failed to delete session; it will expire after TTL." >&2
            [[ "$DEBUG" != 1 ]] || tail -n 30 "$TEMP_DIR/cleanup.log" >&2
        fi
    fi
    if [[ -n "$TEMP_DIR" ]]; then
        rm -rf -- "$TEMP_DIR"
    fi
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
trap 'exit 129' HUP

echo "[1/5] Checking public IP..."
PUBLIC_IP="$(curl -fsS4 --max-time 10 https://api.ipify.org)"
[[ "$PUBLIC_IP" =~ ^([0-9]{1,3}\.){3}[0-9]{1,3}$ ]] || {
    echo "Unexpected public IPv4: $PUBLIC_IP" >&2
    exit 1
}
CIDR="$PUBLIC_IP/32"
[[ "$DEBUG" != 1 ]] || echo "Current public IP: $PUBLIC_IP"

# This Bastion is dedicated to this script: maintain exactly one allowed IP.
echo "[2/5] Checking Bastion configuration..."
BASTION_JSON="$(oci bastion bastion get --region "$REGION" \
    --bastion-id "$BASTION_ID" \
    --output json)"
BASTION_IP="$(jq -er '.data."private-endpoint-ip-address"' <<<"$BASTION_JSON")"
[[ "$(jq -r '.data."lifecycle-state"' <<<"$BASTION_JSON")" == ACTIVE ]] || {
    echo "Bastion is not ACTIVE." >&2
    exit 1
}
[[ "$DEBUG" != 1 ]] || echo "Bastion private endpoint: $BASTION_IP (must be allowed to $TARGET_IP:$TARGET_PORT)"
CURRENT_CIDRS="$(jq '.data."client-cidr-block-allow-list"' <<<"$BASTION_JSON")"
if ! jq -e --arg cidr "$CIDR" 'length == 1 and .[0] == $cidr' \
    >/dev/null <<<"$CURRENT_CIDRS"; then
    echo "Updating Bastion allowlist to $CIDR ..."
    oci bastion bastion update --region "$REGION" \
        --bastion-id "$BASTION_ID" \
        --client-cidr-list "[\"$CIDR\"]" \
        --force --wait-for-state SUCCEEDED --max-wait-seconds 120 \
        --wait-interval-seconds 5 >/dev/null
fi

# Delete only our previous sessions, not all Bastion sessions.
echo "[3/5] Creating SSH session..."
SESSIONS_JSON="$(oci bastion session list --region "$REGION" \
    --bastion-id "$BASTION_ID" --display-name "$SESSION_NAME" \
    --all --output json)"
OLD_IDS="$(jq -r --arg name "$SESSION_NAME" '.data[]? | select(."display-name" == $name) | select(."lifecycle-state" == "ACTIVE" or ."lifecycle-state" == "CREATING") | .id' \
    <<<"$SESSIONS_JSON")"
while IFS= read -r old_id; do
    [[ -z "$old_id" ]] && continue
    [[ "$DEBUG" != 1 ]] || echo "Removing previous session: $old_id"
    oci bastion session delete --region "$REGION" \
        --session-id "$old_id" --force \
        --wait-for-state SUCCEEDED --max-wait-seconds 120 \
        --wait-interval-seconds 5 >/dev/null
done <<<"$OLD_IDS"

TEMP_DIR="$(mktemp -d)"
KEY="$TEMP_DIR/bastion_key"
SSH_LOG="$TEMP_DIR/ssh.log"
CONTROL_SOCKET="$TEMP_DIR/ssh.sock"
ssh-keygen -q -t rsa -b 3072 -f "$KEY" -N ''

SESSION_ID="$(oci bastion session create-port-forwarding \
    --region "$REGION" \
    --bastion-id "$BASTION_ID" \
    --display-name "$SESSION_NAME" \
    --target-private-ip "$TARGET_IP" \
    --target-port "$TARGET_PORT" \
    --ssh-public-key-file "$KEY.pub" \
    --session-ttl "$SESSION_TTL" \
    --query 'data.id' --raw-output)"

if [[ "$SESSION_ID" != ocid1.bastionsession.* ]]; then
    echo "Unable to identify new Bastion session: $SESSION_ID" >&2
    exit 1
fi

# Capture the ID before waiting so activation failures still trigger deletion.
[[ "$DEBUG" != 1 ]] || echo "Waiting for Bastion session to become ACTIVE..."
READY=false
DEADLINE=$((SECONDS + 120))
while ((SECONDS < DEADLINE)); do
    SESSION_JSON="$(oci bastion session get --region "$REGION" \
        --session-id "$SESSION_ID" \
        --output json)" || SESSION_JSON='{}'
    STATE="$(jq -r '.data."lifecycle-state" // "unknown"' <<<"$SESSION_JSON")"
    if [[ "$STATE" == "ACTIVE" ]]; then
        READY=true
        break
    fi
    if [[ "$STATE" == "FAILED" || "$STATE" == "DELETED" || "$STATE" == "DELETING" ]]; then
        echo "Session entered unexpected state: $STATE" >&2
        exit 1
    fi
    sleep 2
done
[[ "$READY" == true ]] || { echo "Session activation timeout (last state: ${STATE:-unknown})" >&2; exit 1; }

SSH_DEST="${SESSION_ID}@host.bastion.${REGION}.oci.oraclecloud.com"

# Compare key material, not filenames or comments. Never print the private key.
read -r LOCAL_KEY_TYPE LOCAL_KEY_BLOB _ < "$KEY.pub"
read -r DERIVED_KEY_TYPE DERIVED_KEY_BLOB _ <<<"$(ssh-keygen -y -f "$KEY")"
read -r OCI_KEY_TYPE OCI_KEY_BLOB _ <<<"$(jq -er '.data."key-details"."public-key-content"' <<<"$SESSION_JSON")"
if [[ "$LOCAL_KEY_TYPE $LOCAL_KEY_BLOB" != "$DERIVED_KEY_TYPE $DERIVED_KEY_BLOB" ||
      "$LOCAL_KEY_TYPE $LOCAL_KEY_BLOB" != "$OCI_KEY_TYPE $OCI_KEY_BLOB" ]]; then
    echo "Bastion session public key does not match the temporary SSH private key. Run the script again to create a fresh session." >&2
    exit 1
fi
SSH_TEMPLATE="$(jq -er '.data."ssh-metadata".command' <<<"$SESSION_JSON")"
if ! jq -e --arg id "$SESSION_ID" --arg bastion "$BASTION_ID" \
    '.data.id == $id and .data."bastion-id" == $bastion' >/dev/null <<<"$SESSION_JSON" ||
    [[ "$SSH_TEMPLATE" != *"-L <localPort>:$TARGET_IP:$TARGET_PORT "* ||
       "$SSH_TEMPLATE" != *"-p 22 $SSH_DEST" ]]; then
    echo "Bastion session ID or SSH metadata does not match the requested tunnel." >&2
    exit 1
fi
[[ "$DEBUG" != 1 ]] || echo "Verified matching local/OCI public keys, session ID, and SSH metadata."

SSH_VERBOSITY=(-o LogLevel=ERROR)
[[ "$DEBUG" != 1 ]] || SSH_VERBOSITY=(-vvv)

ssh_log_contains() {
    local line
    while IFS= read -r line; do
        [[ "$line" != *"$1"* ]] || return 0
    done < "$SSH_LOG"
    return 1
}

show_ssh_failure() {
    if [[ "$DEBUG" == 1 ]]; then
        echo "SSH diagnostics:" >&2
        tail -n 80 "$SSH_LOG" >&2 || true
    else
        local line
        while IFS= read -r line; do
            case "$line" in
                *"Permission denied"*|*"Host key verification failed"*|*"REMOTE HOST IDENTIFICATION HAS CHANGED"*|*"Address already in use"*|*"Could not resolve hostname"*|*"Connection refused"*|*"Connection timed out"*)
                    printf '%s\n' "$line" >&2 ;;
            esac
        done < "$SSH_LOG"
        echo "Use DEBUG=1 ./scripts/connect-studio.sh for detailed SSH diagnostics." >&2
    fi
}

# ACTIVE can precede SSH key availability. Retry only public-key rejection,
# keeping the same verified session/key; host-key and bind failures are fatal.
echo "[4/5] Establishing SSH tunnel..."
READY=false
for ((attempt = 1; attempt <= SSH_ATTEMPTS; attempt++)); do
    SSH_LOG="$TEMP_DIR/ssh-$attempt.log"
    : > "$SSH_LOG"
    [[ "$DEBUG" != 1 ]] || echo "SSH attempt $attempt log (removed on exit): $SSH_LOG"
    # Ignore inherited identities/config for this invocation only. Known-hosts
    # verification is retained; changed host keys remain fatal.
    ssh "${SSH_VERBOSITY[@]}" -F /dev/null -E "$SSH_LOG" -i "$KEY" \
        -N -M -S "$CONTROL_SOCKET" \
        -o IdentitiesOnly=yes -o IdentityAgent=none \
        -o BatchMode=yes -o ControlPersist=no \
        -o StrictHostKeyChecking=accept-new \
        -o ConnectTimeout=10 -o ConnectionAttempts=1 \
        -o ExitOnForwardFailure=yes \
        -o ServerAliveInterval=30 -o ServerAliveCountMax=3 \
        -L "127.0.0.1:$LOCAL_PORT:$TARGET_IP:$TARGET_PORT" \
        -p 22 "$SSH_DEST" 2>>"$SSH_LOG" &
    SSH_PID=$!
    DEADLINE=$((SECONDS + 15))
    while ((SECONDS < DEADLINE)); do
        kill -0 "$SSH_PID" 2>/dev/null || break
        if ssh -F /dev/null -S "$CONTROL_SOCKET" -O check "$SSH_DEST" >/dev/null 2>&1; then
            READY=true
            break
        fi
        sleep 1
    done
    [[ "$READY" != true ]] || break
    stop_ssh
    if ! ssh_log_contains 'Permission denied (publickey)' || ((attempt == SSH_ATTEMPTS)); then
        break
    fi
    echo "Bastion has not accepted the session key yet; retrying in ${SSH_RETRY_DELAY}s ($attempt/$SSH_ATTEMPTS)..."
    rm -f -- "$CONTROL_SOCKET"
    sleep "$SSH_RETRY_DELAY"
done
if [[ "$READY" != true ]]; then
    if ssh_log_contains 'Permission denied (publickey)'; then
        echo "Bastion rejected the verified session key after $SSH_ATTEMPTS attempts. Try again shortly; persistent rejection may require OCI support." >&2
    else
        echo "Unable to establish SSH tunnel. Check the Bastion allowlist, host-key error, or whether port $LOCAL_PORT is already in use." >&2
    fi
    show_ssh_failure
    exit 1
fi

# Ignore certificate trust only for this unauthenticated local readiness probe:
# Studio supports localhost access, but its certificate may not match localhost.
echo "[5/5] Checking MySQL Studio..."
if ! HTTP_CODE="$(curl -fksS --noproxy '*' --connect-timeout 15 --max-time 20 \
    -o /dev/null -w '%{http_code}' "https://127.0.0.1:$LOCAL_PORT/")"; then
    echo "Studio HTTPS probe failed (HTTP ${HTTP_CODE:-000})." >&2
    echo "SSH listening locally does not prove target connectivity." >&2
    echo "Check stateful ingress from $BASTION_IP/32 to TCP $TARGET_PORT on the DB's NSG/security lists, Bastion egress, and Studio status." >&2
    show_ssh_failure
    exit 1
fi
[[ "$DEBUG" != 1 ]] || echo "Verified HTTPS response: HTTP $HTTP_CODE"
printf '%s\n' \
    '+----------------------------------------------------+' \
    '|                                                    |' \
    '|             MySQL Studio is READY                  |' \
    '|                                                    |'
printf '|           %-40s |\n' "https://localhost:$LOCAL_PORT"
printf '%s\n' \
    '|                                                    |' \
    '|   Press Ctrl+C to disconnect                       |' \
    '|                                                    |' \
    '+----------------------------------------------------+'
if ! wait "$SSH_PID"; then
    echo "SSH tunnel closed unexpectedly. Run the script again to reconnect." >&2
    show_ssh_failure
    exit 1
fi
