# Diagnosing Mini App nudges in OCI

The backend emits structured JSON events to stdout/stderr via `tracing-subscriber`.
The deployment sets `RUST_LOG=warn,yalom_bot=info,telegram_admin=info` through the
`yalom-bot-config` ConfigMap. There is **no OCI Logging integration configured**
in this repository: running on a self-managed k3s VM does not automatically
forward container output to OCI Logging.

## First: inspect the existing logs

Run on the OCI k3s VM (SSH, or OCI Compute Instance Agent **Run Command**):

```sh
sudo /usr/local/bin/k3s kubectl get pods -l app=yalom-bot
sudo /usr/local/bin/k3s kubectl logs deployment/yalom-bot \
  -c yalom-bot --since=30m --timestamps --tail=500
```

Reproduce the failure from Telegram Mini App, then rerun the logs command.
For a restarted container, choose the concrete Pod name from `get pods` and use:

```sh
sudo /usr/local/bin/k3s kubectl logs POD_NAME \
  -c yalom-bot --previous --timestamps --tail=500
```

Look for the `miniapp.generate_nudge` span and one of these errors:

- `failed to load user`: user-store/MySQL failure
- `rate limiter failed`: SQL/transaction error in `manual_nudge_rate_limits`
- `failed to generate nudge`: OpenAI API failure or curated-nudge store failure
- `OpenAI returned a non-completed response`: inspect `status`,
  `incomplete_details`, and `response_id`
- `failed to enqueue nudge`: sender queue unavailable (HTTP 503)

An HTTP `internal_error` intentionally contains **no internal details**. Use
the server log, not the API response, to find the underlying failure.

If the logs suggest a missing MySQL table, check the production migration run
before doing anything destructive. The release workflow applies migrations
before updating the deployment.

**Security:** never post raw session cookies, Telegram init data, API keys,
database URLs, generated messages, or sensitive user identifiers in issue
reports. Sanitize logs before sharing them externally.

## Later: centralize logs with OCI Logging

OCI **Logging / Custom Logs** is a managed log destination, but you need an
input agent on this self-managed k3s VM:

1. In OCI, create an appropriate **Log Group** and **Custom Log**.
2. Verify that the VM's **Custom Logs Monitoring** (Unified Monitoring Agent)
   plugin supports the installed OS and ARM architecture, and enable it.
3. Configure an IAM dynamic group for the VM and grant it the minimum needed
   `use log-content` permission for the target scope.
4. Create an **Agent Configuration**, targeting this VM/dynamic group, with
   the container logs as an input. Kubernetes commonly exposes CRI log files
   under `/var/log/containers/`. For a default-namespace backend container
   the pattern is typically
   `/var/log/containers/*_default_yalom-bot-*.log`.
   Confirm the **actual file names, symlinks, agent permissions, and parser**
   on the host before using that pattern in production.
5. In **Observability & Management → Logging → Logs / Search**, verify that
   errors emitted by the Pod arrive and can be filtered.
6. Set a sensible retention period, budget notifications, and avoid collecting
   unrelated containers. OCI Logging may incur charges; check your tenancy's
   current prices and allowances.

For k3s / containerd, the on-disk CRI log line wraps the application's JSON
event in timestamp, stream, and flag fields. Configure the collector parser
accordingly, rather than assuming every physical log line is plain JSON.

OCI documentation:

- [OCI Custom Logs](https://docs.oracle.com/en-us/iaas/Content/Logging/Concepts/custom_logs.htm)
- [OCI Unified Monitoring Agent](https://docs.oracle.com/en-us/iaas/Content/Logging/Concepts/agent_management.htm)
- [OCI Creating a Log](https://docs.oracle.com/en-us/iaas/Content/Logging/Task/create-logging-log.htm)

For immediate debugging, `kubectl logs` is sufficient and does not require
the centralized logging setup.
